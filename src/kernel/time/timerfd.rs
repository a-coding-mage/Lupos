//! linux-parity: partial
//! linux-source: vendor/linux/kernel/time
//! test-origin: linux:vendor/linux/kernel/time
//! `timerfd` — file-descriptor-backed timers (M36).
//!
//! Mirrors `vendor/linux/fs/timerfd.c`.  M36 ships the in-kernel object;
//! actual VFS plumbing arrives in M38.

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::kernel::locking::spinlock::SpinLock;
use crate::kernel::sched::wait::WaitQueueHead;

use super::hrtimer::{
    ClockBase, Hrtimer, HrtimerMode, HrtimerRestart, hrtimer_cancel_wait_running,
    hrtimer_forward_now, hrtimer_get_remaining, hrtimer_init, hrtimer_start,
    hrtimer_state_snapshot, hrtimer_try_to_cancel,
};
use super::posix_clock::{
    CLOCK_BOOTTIME, CLOCK_MONOTONIC, CLOCK_REALTIME, ClockId, EINVAL, Timespec64,
};
use super::posix_timers::{Itimerspec64, TIMER_ABSTIME};

pub const TFD_TIMER_ABSTIME: i32 = 1 << 0;
pub const TFD_TIMER_CANCEL_ON_SET: i32 = 1 << 1;
pub const TFD_NONBLOCK: i32 = 0o4000;
pub const TFD_CLOEXEC: i32 = 0o2000000;
const TFD_CREATE_FLAGS: i32 = TFD_NONBLOCK | TFD_CLOEXEC;
const TFD_SETTIME_FLAGS: i32 = TFD_TIMER_ABSTIME | TFD_TIMER_CANCEL_ON_SET;

struct TimerFdState {
    interval_ns: u64,
    expired: bool,
    ticks: u64,
}

pub struct TimerFd {
    pub clock: ClockId,
    pub flags: i32,
    /// Linux `timerfd_ctx::wqh`: readers and poll/epoll waiters sleep here.
    pub(crate) wait: WaitQueueHead,
    /// Linux protects `ticks`, `expired`, `tintv`, and timer programming with
    /// `ctx->wqh.lock`.  Lupos keeps the wait-list lock encapsulated, so this
    /// IRQ-safe spinlock is the equivalent state lock; readiness is published
    /// under it before the waitqueue is woken.
    state: SpinLock<TimerFdState>,
    /// Access is serialized by `state`. Keeping the timer in a separate
    /// UnsafeCell lets raw callback and owner paths operate without borrowing
    /// the whole timerfd object.
    timer: UnsafeCell<Hrtimer>,
}

unsafe impl Send for TimerFd {}
unsafe impl Sync for TimerFd {}

impl TimerFd {
    fn new(clock: ClockId, flags: i32) -> Result<Self, i32> {
        let base = match clock {
            CLOCK_REALTIME => ClockBase::Realtime,
            CLOCK_MONOTONIC => ClockBase::Monotonic,
            CLOCK_BOOTTIME => ClockBase::Boottime,
            _ => return Err(EINVAL),
        };
        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, base, HrtimerMode::Abs);
        Ok(Self {
            clock,
            flags,
            wait: WaitQueueHead::new(),
            state: SpinLock::new(TimerFdState {
                interval_ns: 0,
                expired: false,
                ticks: 0,
            }),
            timer: UnsafeCell::new(t),
        })
    }

    #[inline]
    fn timer_ptr(&self) -> *mut Hrtimer {
        self.timer.get()
    }

    /// Cancel with Linux's `try_to_cancel`/wait/retry lock ordering.  The
    /// state lock must be dropped while a callback is running because the
    /// callback takes the same lock to publish `ticks` and `expired`.
    fn cancel_synchronously(&self) {
        loop {
            let (state, irqflags) = self.state.lock_irqsave();
            let ret = hrtimer_try_to_cancel(self.timer_ptr());
            SpinLock::unlock_irqrestore(state, irqflags);
            if ret >= 0 {
                return;
            }
            hrtimer_cancel_wait_running(self.timer_ptr());
        }
    }

    pub(crate) fn has_ticks(&self) -> bool {
        let (state, irqflags) = self.state.lock_irqsave();
        let ready = state.ticks != 0;
        SpinLock::unlock_irqrestore(state, irqflags);
        ready
    }

    #[cfg(test)]
    pub(crate) fn set_pending_for_test(&self, expired: bool, ticks: u64) {
        let (mut state, irqflags) = self.state.lock_irqsave();
        state.expired = expired;
        state.ticks = ticks;
        SpinLock::unlock_irqrestore(state, irqflags);
    }

    #[cfg(test)]
    fn pending_for_test(&self) -> (bool, u64) {
        let (state, irqflags) = self.state.lock_irqsave();
        let pending = (state.expired, state.ticks);
        SpinLock::unlock_irqrestore(state, irqflags);
        pending
    }

    #[cfg(test)]
    fn timer_state_for_test(&self) -> u8 {
        let (state, irqflags) = self.state.lock_irqsave();
        let timer_state = hrtimer_state_snapshot(self.timer_ptr());
        SpinLock::unlock_irqrestore(state, irqflags);
        timer_state
    }
}

impl Drop for TimerFd {
    fn drop(&mut self) {
        // The object remains allocated throughout Drop.  Synchronous cancel
        // therefore closes the dequeue-to-callback lifetime window before the
        // embedded timer and waitqueue can be freed.
        self.cancel_synchronously();
    }
}

/// `timerfd_create(clockid, flags)`.  Returns an in-kernel handle (real fd
/// allocation lands in M38 VFS).
pub fn sys_timerfd_create(clock: ClockId, flags: i32) -> Result<Arc<TimerFd>, i32> {
    if flags & !TFD_CREATE_FLAGS != 0 {
        return Err(EINVAL);
    }
    TimerFd::new(clock, flags).map(Arc::new)
}

/// `timerfd_settime(tfd, flags, new_value, old_value)`.
pub fn sys_timerfd_settime(
    tfd: &Arc<TimerFd>,
    flags: i32,
    new_value: Itimerspec64,
) -> Result<Itimerspec64, i32> {
    if flags & !TFD_SETTIME_FLAGS != 0 {
        return Err(EINVAL);
    }
    // Linux accepts CANCEL_ON_SET for every timerfd and only arms its
    // realtime-clock cancel list when it is combined with ABSTIME on a
    // realtime clock (timerfd_setup_cancel()).  Keep accepting the ABI flag;
    // the module's partial-parity marker records that clock-step ECANCELED
    // delivery is not wired yet.  Rejecting the flag here breaks systemd's
    // normal manager initialization.
    if !new_value.it_value.is_valid() || !new_value.it_interval.is_valid() {
        return Err(EINVAL);
    }
    let mode = if flags & TFD_TIMER_ABSTIME != 0 || flags & TIMER_ABSTIME != 0 {
        HrtimerMode::Abs
    } else {
        HrtimerMode::Rel
    };
    let expires_ns = new_value.it_value.to_ns();
    loop {
        let (mut state, irqflags) = tfd.state.lock_irqsave();
        let timer_ptr = tfd.timer_ptr();
        let cancel = hrtimer_try_to_cancel(timer_ptr);
        if cancel < 0 {
            SpinLock::unlock_irqrestore(state, irqflags);
            hrtimer_cancel_wait_running(timer_ptr);
            continue;
        }

        let timer = unsafe { &mut *timer_ptr };
        // Linux advances an expired periodic timer before reporting the old
        // expiry, then snapshots both old fields under ctx->wqh.lock.
        if state.expired && state.interval_ns != 0 {
            let _ = hrtimer_forward_now(timer, state.interval_ns);
        }
        let old = Itimerspec64 {
            it_interval: Timespec64::from_ns(state.interval_ns),
            it_value: Timespec64::from_ns(hrtimer_get_remaining(timer_ptr)),
        };

        state.expired = false;
        state.ticks = 0;
        state.interval_ns = new_value.it_interval.to_ns();
        hrtimer_init(timer, clock_base(tfd.clock), mode);
        timer.interval_ns = state.interval_ns;
        timer.function = Some(timerfd_callback);
        timer.data = Arc::as_ptr(tfd) as usize;

        let mut wake = false;
        if expires_ns != 0 {
            if mode == HrtimerMode::Abs && expires_ns <= timer.base_now() {
                // `hrtimer_start_range_ns_user()` reports an already-expired
                // userspace timer to timerfd synchronously.
                timer.expires_ns = expires_ns;
                timerfd_triggered_locked(&mut state);
                wake = true;
            } else {
                hrtimer_start(timer_ptr, expires_ns, mode);
            }
        }
        SpinLock::unlock_irqrestore(state, irqflags);
        if wake {
            tfd.wait.wake_up_all();
        }
        return Ok(old);
    }
}

/// `timerfd_gettime`.
pub fn sys_timerfd_gettime(tfd: &Arc<TimerFd>) -> Itimerspec64 {
    loop {
        let (mut state, irqflags) = tfd.state.lock_irqsave();
        let timer_ptr = tfd.timer_ptr();
        // A remote task can observe `expired` after the callback releases the
        // owner lock but before hrtimer_run_queues clears base.running. Never
        // form a mutable timer reference until that callback has returned.
        if state.expired && state.interval_ns != 0 {
            let cancel = hrtimer_try_to_cancel(timer_ptr);
            if cancel < 0 {
                SpinLock::unlock_irqrestore(state, irqflags);
                hrtimer_cancel_wait_running(timer_ptr);
                continue;
            }
            let timer = unsafe { &mut *timer_ptr };
            state.expired = false;
            let overruns = hrtimer_forward_now(timer, state.interval_ns);
            state.ticks = state.ticks.wrapping_add(overruns.saturating_sub(1));
            hrtimer_start(timer_ptr, timer.expires_ns, HrtimerMode::Abs);
        }
        let current = Itimerspec64 {
            it_interval: Timespec64::from_ns(state.interval_ns),
            it_value: Timespec64::from_ns(hrtimer_get_remaining(timer_ptr)),
        };
        SpinLock::unlock_irqrestore(state, irqflags);
        return current;
    }
}

/// In-kernel `read()` — returns the expiration count and zeros it.
pub fn timerfd_read(tfd: &Arc<TimerFd>) -> u64 {
    loop {
        let (mut state, irqflags) = tfd.state.lock_irqsave();
        let mut ticks = state.ticks;
        if ticks == 0 {
            SpinLock::unlock_irqrestore(state, irqflags);
            return 0;
        }
        let timer_ptr = tfd.timer_ptr();
        if state.expired && state.interval_ns != 0 {
            let cancel = hrtimer_try_to_cancel(timer_ptr);
            if cancel < 0 {
                SpinLock::unlock_irqrestore(state, irqflags);
                hrtimer_cancel_wait_running(timer_ptr);
                continue;
            }
            let timer = unsafe { &mut *timer_ptr };
            let overruns = hrtimer_forward_now(timer, state.interval_ns);
            ticks = ticks.wrapping_add(overruns.saturating_sub(1));
            hrtimer_start(timer_ptr, timer.expires_ns, HrtimerMode::Abs);
        }
        state.expired = false;
        state.ticks = 0;
        SpinLock::unlock_irqrestore(state, irqflags);
        return ticks;
    }
}

fn clock_base(clock: ClockId) -> ClockBase {
    match clock {
        CLOCK_REALTIME => ClockBase::Realtime,
        CLOCK_BOOTTIME => ClockBase::Boottime,
        _ => ClockBase::Monotonic,
    }
}

fn timerfd_callback(t: *mut Hrtimer) -> HrtimerRestart {
    if !t.is_null() && unsafe { (*t).data != 0 } {
        let tfd = unsafe { (*t).data as *const TimerFd };
        timerfd_triggered(tfd);
    } else {
        GLOBAL_TIMERFD_TICKS.fetch_add(1, Ordering::AcqRel);
    }
    HrtimerRestart::NoRestart
}

/// Linux `__timerfd_triggered()`: publish readiness before waking every
/// reader and poll callback registered on `timerfd_ctx::wqh`.
fn timerfd_triggered(tfd: *const TimerFd) {
    if tfd.is_null() {
        return;
    }
    let state_lock = unsafe { &*core::ptr::addr_of!((*tfd).state) };
    let (mut state, irqflags) = state_lock.lock_irqsave();
    timerfd_triggered_locked(&mut state);
    SpinLock::unlock_irqrestore(state, irqflags);
    unsafe { &*core::ptr::addr_of!((*tfd).wait) }.wake_up_all();
}

fn timerfd_triggered_locked(state: &mut TimerFdState) {
    state.expired = true;
    state.ticks = state.ticks.wrapping_add(1);
}

pub static GLOBAL_TIMERFD_TICKS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timerfd_create_supports_monotonic() {
        let tfd = sys_timerfd_create(CLOCK_MONOTONIC, 0).unwrap();
        assert_eq!(tfd.clock, CLOCK_MONOTONIC);
    }

    #[test]
    fn timerfd_create_invalid_clock_returns_einval() {
        assert_eq!(sys_timerfd_create(42, 0).map(|_| ()), Err(EINVAL));
    }

    #[test]
    fn timerfd_create_rejects_unknown_flags() {
        assert_eq!(
            sys_timerfd_create(CLOCK_MONOTONIC, 0x100).map(|_| ()),
            Err(EINVAL)
        );
        assert!(sys_timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK | TFD_CLOEXEC).is_ok());
    }

    #[test]
    fn timerfd_settime_round_trip() {
        let tfd = sys_timerfd_create(CLOCK_MONOTONIC, 0).unwrap();
        let new = Itimerspec64 {
            it_interval: Timespec64::new(0, 0),
            it_value: Timespec64::new(0, 1_000_000),
        };
        let _old = sys_timerfd_settime(&tfd, 0, new).unwrap();
        let cur = sys_timerfd_gettime(&tfd);
        assert!(cur.it_value.to_ns() > 0);
    }

    #[test]
    fn timerfd_settime_clears_pending_expirations() {
        let tfd = sys_timerfd_create(CLOCK_MONOTONIC, 0).unwrap();
        tfd.set_pending_for_test(true, 7);
        let new = Itimerspec64 {
            it_interval: Timespec64::new(0, 0),
            it_value: Timespec64::new(1, 0),
        };
        let _old = sys_timerfd_settime(&tfd, 0, new).unwrap();
        assert_eq!(tfd.pending_for_test(), (false, 0));
        assert_eq!(timerfd_read(&tfd), 0);
    }

    #[test]
    fn timerfd_settime_rejects_unknown_flags_and_bad_timespec() {
        let tfd = sys_timerfd_create(CLOCK_MONOTONIC, 0).unwrap();
        let valid = Itimerspec64 {
            it_interval: Timespec64::new(0, 0),
            it_value: Timespec64::new(0, 1),
        };
        assert_eq!(sys_timerfd_settime(&tfd, 0x100, valid), Err(EINVAL));

        let invalid = Itimerspec64 {
            it_interval: Timespec64::new(0, 1_000_000_000),
            it_value: Timespec64::new(0, 1),
        };
        assert_eq!(sys_timerfd_settime(&tfd, 0, invalid), Err(EINVAL));
    }

    #[test]
    fn flag_constants_match_linux() {
        assert_eq!(TFD_TIMER_ABSTIME, 1);
        assert_eq!(TFD_TIMER_CANCEL_ON_SET, 2);
    }

    #[test]
    fn timerfd_periodic_callback_waits_for_read_to_rearm() {
        let tfd = sys_timerfd_create(CLOCK_MONOTONIC, 0).unwrap();
        let new = Itimerspec64 {
            it_interval: Timespec64::new(3600, 0),
            it_value: Timespec64::new(0, 1),
        };
        let _old = sys_timerfd_settime(&tfd, TFD_TIMER_ABSTIME, new).unwrap();

        crate::kernel::time::timekeeping::tick_advance_walltime();
        crate::kernel::time::hrtimer::hrtimer_run_queues();

        assert_eq!(tfd.pending_for_test(), (true, 1));
        assert_eq!(
            tfd.timer_state_for_test(),
            crate::kernel::time::hrtimer::HRTIMER_STATE_INACTIVE
        );
        assert_eq!(timerfd_read(&tfd), 1);
        assert_eq!(timerfd_read(&tfd), 0);
        assert_eq!(
            tfd.timer_state_for_test(),
            crate::kernel::time::hrtimer::HRTIMER_STATE_ENQUEUED
        );
    }
}
