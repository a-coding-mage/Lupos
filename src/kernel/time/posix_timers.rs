//! linux-parity: partial
//! linux-deviation: fixed signal slots currently retain one POSIX timer per signo and synthesize SI_TIMER without the full siginfo payload.
//! linux-source: vendor/linux/kernel/time/posix-timers.c
//! test-origin: linux:vendor/linux/kernel/time/posix-timers.c
//! POSIX timers — `timer_create`/`timer_settime`/`timer_gettime`/`timer_delete`
//! (M36).
//!
//! Mirrors `vendor/linux/kernel/time/posix-timers.c`.  A `posix_timer` wraps
//! an `hrtimer` plus an expiry signal target.  M36 ships the in-kernel
//! object; signal delivery integrates with M25's `send_sig_info`.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicI32, AtomicU64, AtomicUsize, Ordering};

use spin::Mutex;

use super::hrtimer::{
    ClockBase, Hrtimer, HrtimerMode, HrtimerRestart, hrtimer_cancel_wait_running,
    hrtimer_forward_now, hrtimer_get_remaining, hrtimer_init, hrtimer_restart, hrtimer_start,
    hrtimer_try_to_cancel,
};
use super::posix_clock::{
    CLOCK_BOOTTIME, CLOCK_MONOTONIC, CLOCK_REALTIME, CLOCK_TAI, ClockId, EINVAL, Timespec64,
};

/// Linux `struct itimerspec64`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Itimerspec64 {
    pub it_interval: Timespec64,
    pub it_value: Timespec64,
}

const _: () = assert!(core::mem::size_of::<Itimerspec64>() == 32);

pub const TIMER_ABSTIME: i32 = 0x01;

/// `struct k_itimer` — kernel POSIX timer record.
struct PosixTimerExpiryState {
    sigev_signo: i32,
    _sigev_value: u64,
    target_pid: i32,
    target_tgid: i32,
    interval_ns: u64,
}

pub struct PosixTimer {
    pub id: i32,
    pub clock: ClockId,
    pub expirations: AtomicU64,
    pub overruns: AtomicU64,
    /// Linux `k_itimer::it_lock`: expiry-visible state is embedded in the
    /// stable timer object and every accessor masks local IRQs.
    expiry: crate::kernel::locking::SpinLock<PosixTimerExpiryState>,
    /// Access is serialized by `expiry`. Keeping the hrtimer in UnsafeCell
    /// allows raw callback and owner-side access without borrowing the whole
    /// enclosing timer, as in timerfd_ctx.
    timer: UnsafeCell<Hrtimer>,
}

unsafe impl Send for PosixTimer {}
unsafe impl Sync for PosixTimer {}

impl PosixTimer {
    pub fn new(
        id: i32,
        clock: ClockId,
        signo: i32,
        value: u64,
        target_task: *mut crate::kernel::task::TaskStruct,
    ) -> Self {
        let mut t = Hrtimer::new();
        let base = match clock {
            CLOCK_REALTIME | CLOCK_TAI => ClockBase::Realtime,
            CLOCK_MONOTONIC => ClockBase::Monotonic,
            CLOCK_BOOTTIME => ClockBase::Boottime,
            _ => ClockBase::Monotonic,
        };
        hrtimer_init(&mut t, base, HrtimerMode::Abs);
        let (target_pid, target_tgid) = if target_task.is_null() {
            (0, 0)
        } else {
            unsafe { ((*target_task).pid, (*target_task).tgid) }
        };
        Self {
            id,
            clock,
            expirations: AtomicU64::new(0),
            overruns: AtomicU64::new(0),
            expiry: crate::kernel::locking::SpinLock::new(PosixTimerExpiryState {
                sigev_signo: signo,
                _sigev_value: value,
                target_pid,
                target_tgid,
                interval_ns: 0,
            }),
            timer: UnsafeCell::new(t),
        }
    }

    #[inline]
    fn timer_ptr(&self) -> *mut Hrtimer {
        self.timer.get()
    }

    fn with_expiry<R>(&self, f: impl FnOnce(&PosixTimerExpiryState) -> R) -> R {
        let (state, flags) = self.expiry.lock_irqsave();
        let result = f(&state);
        crate::kernel::locking::SpinLock::unlock_irqrestore(state, flags);
        result
    }

    fn cancel_synchronously(&self) {
        loop {
            let (state, flags) = self.expiry.lock_irqsave();
            let ret = hrtimer_try_to_cancel(self.timer_ptr());
            crate::kernel::locking::SpinLock::unlock_irqrestore(state, flags);
            if ret >= 0 {
                return;
            }
            hrtimer_cancel_wait_running(self.timer_ptr());
        }
    }
}

impl Drop for PosixTimer {
    fn drop(&mut self) {
        self.cancel_synchronously();
    }
}

// ── Global timer registry ─────────────────────────────────────────────────────

static NEXT_TIMER_ID: AtomicI32 = AtomicI32::new(1);
static TIMERS: Mutex<BTreeMap<i32, alloc::boxed::Box<PosixTimer>>> = Mutex::new(BTreeMap::new());
static TIMER_COUNT: AtomicUsize = AtomicUsize::new(0);

/// `timer_create(clockid, sigev_signo, sigev_value)`.
pub fn sys_timer_create(clock: ClockId, signo: i32, value: u64) -> Result<i32, i32> {
    if !matches!(
        clock,
        CLOCK_REALTIME | CLOCK_MONOTONIC | CLOCK_BOOTTIME | CLOCK_TAI
    ) {
        return Err(EINVAL);
    }
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::AcqRel);
    let signo = if signo == 0 {
        crate::kernel::signal::SIGALRM
    } else {
        signo
    };
    let target = unsafe { crate::kernel::sched::get_current() };
    if !target.is_null() {
        let _ = unsafe { crate::kernel::signal::prepare_timer_signal_target(target) };
    }
    let timer = alloc::boxed::Box::new(PosixTimer::new(id, clock, signo, value, target));
    let owner = (&*timer as *const PosixTimer) as usize;
    unsafe {
        (*timer.timer_ptr()).data = owner;
    }
    TIMERS.lock().insert(id, timer);
    TIMER_COUNT.fetch_add(1, Ordering::AcqRel);
    Ok(id)
}

/// `timer_settime(timer_id, flags, new_value, old_value)`.
pub fn sys_timer_settime(
    id: i32,
    flags: i32,
    new_value: Itimerspec64,
) -> Result<Itimerspec64, i32> {
    if !new_value.it_value.is_valid() || !new_value.it_interval.is_valid() {
        return Err(EINVAL);
    }
    let g = TIMERS.lock();
    let t = g.get(&id).ok_or(EINVAL)?;
    loop {
        let (mut state, irqflags) = t.expiry.lock_irqsave();
        let timer_ptr = t.timer_ptr();
        let cancel = hrtimer_try_to_cancel(timer_ptr);
        if cancel < 0 {
            crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
            hrtimer_cancel_wait_running(timer_ptr);
            continue;
        }

        let timer = unsafe { &mut *timer_ptr };
        let old = Itimerspec64 {
            it_interval: Timespec64::from_ns(state.interval_ns),
            it_value: Timespec64::from_ns(hrtimer_get_remaining(timer_ptr)),
        };

        t.expirations.store(0, Ordering::Release);
        state.interval_ns = new_value.it_interval.to_ns();
        timer.interval_ns = state.interval_ns;
        timer.function = Some(posix_timer_fired);

        let expires_ns = new_value.it_value.to_ns();
        if expires_ns != 0 {
            let mode = if flags & TIMER_ABSTIME != 0 {
                HrtimerMode::Abs
            } else {
                HrtimerMode::Rel
            };
            hrtimer_start(timer_ptr, expires_ns, mode);
        }
        crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
        return Ok(old);
    }
}

/// `timer_gettime(timer_id)`.
pub fn sys_timer_gettime(id: i32) -> Result<Itimerspec64, i32> {
    let g = TIMERS.lock();
    let t = g.get(&id).ok_or(EINVAL)?;
    let (state, irqflags) = t.expiry.lock_irqsave();
    let current = Itimerspec64 {
        it_interval: Timespec64::from_ns(state.interval_ns),
        it_value: Timespec64::from_ns(hrtimer_get_remaining(t.timer_ptr())),
    };
    crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
    Ok(current)
}

/// `timer_delete(timer_id)`.
pub fn sys_timer_delete(id: i32) -> Result<(), i32> {
    let mut g = TIMERS.lock();
    let t = g.remove(&id).ok_or(EINVAL)?;
    TIMER_COUNT.fetch_sub(1, Ordering::AcqRel);
    drop(g);
    t.cancel_synchronously();
    Ok(())
}

/// `timer_getoverrun(timer_id)`.
pub fn sys_timer_getoverrun(id: i32) -> Result<u64, i32> {
    let g = TIMERS.lock();
    let t = g.get(&id).ok_or(EINVAL)?;
    Ok(t.overruns.load(Ordering::Acquire))
}

/// Hrtimer callback: fires when a posix timer expires.
fn posix_timer_fired(t: *mut Hrtimer) -> HrtimerRestart {
    let owner = if t.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { (*t).data as *mut PosixTimer }
    };
    let target = if owner.is_null() {
        None
    } else {
        unsafe { &*core::ptr::addr_of!((*owner).expirations) }.fetch_add(1, Ordering::AcqRel);
        let expiry = unsafe { &*core::ptr::addr_of!((*owner).expiry) };
        let (state, irqflags) = expiry.lock_irqsave();
        let target = (
            state.target_pid,
            state.target_tgid,
            state.sigev_signo,
            unsafe { core::ptr::addr_of!((*owner).id).read() },
        );
        crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
        Some(target)
    };
    POSIX_TIMER_FIRED.fetch_add(1, Ordering::AcqRel);
    if let Some((pid, tgid, sig, timer_id)) = target {
        let _ = crate::kernel::signal::queue_posix_timer_signal_noalloc(pid, tgid, sig, timer_id);
    }
    // Linux re-arms periodic POSIX timers only when their queued signal is
    // dequeued, preventing tiny intervals from monopolizing one hard IRQ.
    HrtimerRestart::NoRestart
}

/// `common_hrtimer_rearm()` from the POSIX-timer signal-delivery path.
/// Called only after SIGNAL_TABLE has been unlocked.
pub(crate) fn rearm_posix_timer_after_signal(id: i32) {
    if id <= 0 {
        return;
    }
    let timers = TIMERS.lock();
    let Some(timer) = timers.get(&id) else {
        return;
    };
    loop {
        let (state, irqflags) = timer.expiry.lock_irqsave();
        let timer_ptr = timer.timer_ptr();
        let cancel = hrtimer_try_to_cancel(timer_ptr);
        if cancel < 0 {
            crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
            hrtimer_cancel_wait_running(timer_ptr);
            continue;
        }
        if cancel > 0 {
            hrtimer_restart(timer_ptr);
            crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
            return;
        }
        if state.interval_ns == 0 {
            crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
            return;
        }
        unsafe {
            let _ = hrtimer_forward_now(&mut *timer_ptr, state.interval_ns);
        }
        hrtimer_restart(timer_ptr);
        crate::kernel::locking::SpinLock::unlock_irqrestore(state, irqflags);
        return;
    }
}

/// Linux `exit_itimers()` lifetime rule for the current task-owned timer
/// model: remove and synchronously cancel every timer before TaskStruct can be
/// freed or its signal binding cleared.
pub(crate) fn release_task_posix_timers(pid: i32) {
    if pid <= 0 {
        return;
    }
    if TIMER_COUNT.load(Ordering::Acquire) == 0 {
        return;
    }
    loop {
        let timer = {
            let mut timers = TIMERS.lock();
            let id = timers.iter().find_map(|(&id, timer)| {
                timer
                    .with_expiry(|state| state.target_pid == pid)
                    .then_some(id)
            });
            id.and_then(|id| timers.remove(&id))
        };
        let Some(timer) = timer else {
            return;
        };
        TIMER_COUNT.fetch_sub(1, Ordering::AcqRel);
        timer.cancel_synchronously();
    }
}

pub static POSIX_TIMER_FIRED: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itimerspec64_layout_matches_linux() {
        assert_eq!(core::mem::size_of::<Itimerspec64>(), 32);
    }

    #[test]
    fn timer_create_valid_clock_returns_id() {
        let id = sys_timer_create(CLOCK_MONOTONIC, 14, 0).unwrap();
        assert!(id > 0);
        sys_timer_delete(id).unwrap();
    }

    #[test]
    fn timer_create_invalid_clock_returns_einval() {
        assert_eq!(sys_timer_create(42, 14, 0), Err(EINVAL));
    }

    #[test]
    fn timer_settime_then_gettime_round_trip() {
        let id = sys_timer_create(CLOCK_MONOTONIC, 14, 0).unwrap();
        let new = Itimerspec64 {
            it_interval: Timespec64::new(0, 0),
            it_value: Timespec64::new(0, 1_000_000), // 1 ms
        };
        let _old = sys_timer_settime(id, 0, new).unwrap();
        let cur = sys_timer_gettime(id).unwrap();
        assert!(cur.it_value.to_ns() <= 1_000_000);
        sys_timer_delete(id).unwrap();
    }

    #[test]
    fn timer_delete_unknown_id_returns_einval() {
        assert_eq!(sys_timer_delete(999_999), Err(EINVAL));
    }

    #[test]
    fn timer_abstime_constant_matches_linux() {
        assert_eq!(TIMER_ABSTIME, 1);
    }
}
