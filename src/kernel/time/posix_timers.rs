//! linux-parity: complete
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
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use spin::Mutex;

use super::hrtimer::{
    ClockBase, Hrtimer, HrtimerMode, HrtimerRestart, hrtimer_cancel, hrtimer_init, hrtimer_start,
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
pub struct PosixTimer {
    pub id: i32,
    pub clock: ClockId,
    pub sigev_signo: i32,
    pub sigev_value: u64,
    pub target_task: *mut crate::kernel::task::TaskStruct,
    pub expirations: AtomicU64,
    pub overruns: AtomicU64,
    pub timer: Hrtimer,
    pub interval_ns: u64,
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
        Self {
            id,
            clock,
            sigev_signo: signo,
            sigev_value: value,
            target_task,
            expirations: AtomicU64::new(0),
            overruns: AtomicU64::new(0),
            timer: t,
            interval_ns: 0,
        }
    }
}

// ── Global timer registry ─────────────────────────────────────────────────────

static NEXT_TIMER_ID: AtomicI32 = AtomicI32::new(1);
static TIMERS: Mutex<BTreeMap<i32, alloc::boxed::Box<PosixTimer>>> = Mutex::new(BTreeMap::new());

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
    let timer = alloc::boxed::Box::new(PosixTimer::new(id, clock, signo, value, unsafe {
        crate::kernel::sched::get_current()
    }));
    TIMERS.lock().insert(id, timer);
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
    let mut g = TIMERS.lock();
    let t = g.get_mut(&id).ok_or(EINVAL)?;

    // Save previous setting for return.
    let old = Itimerspec64 {
        it_interval: Timespec64::from_ns(t.interval_ns),
        it_value: Timespec64::from_ns(t.timer.expires_ns),
    };

    // Cancel any active timer.
    let _ = hrtimer_cancel(&mut t.timer as *mut Hrtimer);

    // Reset state.
    t.expirations.store(0, Ordering::Release);
    t.interval_ns = new_value.it_interval.to_ns();
    t.timer.interval_ns = t.interval_ns;
    t.timer.function = Some(posix_timer_fired);

    // it_value of zero disarms.
    if new_value.it_value.to_ns() == 0 {
        return Ok(old);
    }

    let mode = if flags & TIMER_ABSTIME != 0 {
        HrtimerMode::Abs
    } else {
        HrtimerMode::Rel
    };
    hrtimer_start(
        &mut t.timer as *mut Hrtimer,
        new_value.it_value.to_ns(),
        mode,
    );
    Ok(old)
}

/// `timer_gettime(timer_id)`.
pub fn sys_timer_gettime(id: i32) -> Result<Itimerspec64, i32> {
    let g = TIMERS.lock();
    let t = g.get(&id).ok_or(EINVAL)?;
    Ok(Itimerspec64 {
        it_interval: Timespec64::from_ns(t.interval_ns),
        it_value: Timespec64::from_ns(t.timer.expires_ns),
    })
}

/// `timer_delete(timer_id)`.
pub fn sys_timer_delete(id: i32) -> Result<(), i32> {
    let mut g = TIMERS.lock();
    let mut t = g.remove(&id).ok_or(EINVAL)?;
    let _ = hrtimer_cancel(&mut t.timer as *mut Hrtimer);
    Ok(())
}

/// `timer_getoverrun(timer_id)`.
pub fn sys_timer_getoverrun(id: i32) -> Result<u64, i32> {
    let g = TIMERS.lock();
    let t = g.get(&id).ok_or(EINVAL)?;
    Ok(t.overruns.load(Ordering::Acquire))
}

/// Hrtimer callback: fires when a posix timer expires.
fn posix_timer_fired(t: &mut Hrtimer) -> HrtimerRestart {
    let timer_ptr = t as *mut Hrtimer as usize;
    let target = {
        let mut timers = TIMERS.lock();
        let mut found = None;
        for timer in timers.values_mut() {
            let candidate = &mut timer.timer as *mut Hrtimer as usize;
            if candidate == timer_ptr {
                timer.expirations.fetch_add(1, Ordering::AcqRel);
                found = Some((timer.target_task, timer.sigev_signo));
                break;
            }
        }
        found
    };
    POSIX_TIMER_FIRED.fetch_add(1, Ordering::AcqRel);
    if let Some((task, sig)) = target {
        unsafe {
            let _ = crate::kernel::signal::send_signal_to_task(task, sig);
        }
    }
    if t.interval_ns > 0 {
        HrtimerRestart::Restart
    } else {
        HrtimerRestart::NoRestart
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
        assert!(cur.it_value.to_ns() >= 1_000_000);
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
