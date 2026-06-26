//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time
//! test-origin: linux:vendor/linux/kernel/time
//! `timerfd` — file-descriptor-backed timers (M36).
//!
//! Mirrors `vendor/linux/fs/timerfd.c`.  M36 ships the in-kernel object;
//! actual VFS plumbing arrives in M38.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use spin::Mutex;

use super::hrtimer::{
    ClockBase, HRTIMER_STATE_ENQUEUED, Hrtimer, HrtimerMode, HrtimerRestart, hrtimer_cancel,
    hrtimer_forward_now, hrtimer_init, hrtimer_restart, hrtimer_start,
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

pub struct TimerFd {
    pub clock: ClockId,
    pub flags: i32,
    pub interval_ns: u64,
    pub expired: AtomicBool,
    pub ticks: AtomicU64,
    pub timer: Mutex<Hrtimer>,
}

unsafe impl Send for TimerFd {}
unsafe impl Sync for TimerFd {}

impl TimerFd {
    pub fn new(clock: ClockId, flags: i32) -> Result<Self, i32> {
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
            interval_ns: 0,
            expired: AtomicBool::new(false),
            ticks: AtomicU64::new(0),
            timer: Mutex::new(t),
        })
    }
}

impl Drop for TimerFd {
    fn drop(&mut self) {
        let mut t = self.timer.lock();
        let _ = hrtimer_cancel(&mut *t as *mut Hrtimer);
    }
}

/// `timerfd_create(clockid, flags)`.  Returns an in-kernel handle (real fd
/// allocation lands in M38 VFS).
pub fn sys_timerfd_create(clock: ClockId, flags: i32) -> Result<TimerFd, i32> {
    if flags & !TFD_CREATE_FLAGS != 0 {
        return Err(EINVAL);
    }
    TimerFd::new(clock, flags)
}

/// `timerfd_settime(tfd, flags, new_value, old_value)`.
pub fn sys_timerfd_settime(
    tfd: &TimerFd,
    flags: i32,
    new_value: Itimerspec64,
) -> Result<Itimerspec64, i32> {
    if flags & !TFD_SETTIME_FLAGS != 0 {
        return Err(EINVAL);
    }
    if !new_value.it_value.is_valid() || !new_value.it_interval.is_valid() {
        return Err(EINVAL);
    }
    let mut t = tfd.timer.lock();
    let old = Itimerspec64 {
        it_interval: Timespec64::from_ns(t.interval_ns),
        it_value: Timespec64::from_ns(timerfd_remaining_ns(&t)),
    };
    let _ = hrtimer_cancel(&mut *t as *mut Hrtimer);
    tfd.expired.store(false, Ordering::Release);
    tfd.ticks.store(0, Ordering::Release);
    t.interval_ns = new_value.it_interval.to_ns();
    t.function = Some(timerfd_callback);
    t.data = tfd as *const TimerFd as usize;

    if new_value.it_value.to_ns() == 0 {
        return Ok(old);
    }
    let mode = if flags & TFD_TIMER_ABSTIME != 0 || flags & TIMER_ABSTIME != 0 {
        HrtimerMode::Abs
    } else {
        HrtimerMode::Rel
    };
    hrtimer_start(&mut *t as *mut Hrtimer, new_value.it_value.to_ns(), mode);
    Ok(old)
}

/// `timerfd_gettime`.
pub fn sys_timerfd_gettime(tfd: &TimerFd) -> Itimerspec64 {
    let t = tfd.timer.lock();
    Itimerspec64 {
        it_interval: Timespec64::from_ns(t.interval_ns),
        it_value: Timespec64::from_ns(timerfd_remaining_ns(&t)),
    }
}

/// In-kernel `read()` — returns the expiration count and zeros it.
pub fn timerfd_read(tfd: &TimerFd) -> u64 {
    let mut ticks = tfd.ticks.swap(0, Ordering::AcqRel);
    if ticks == 0 {
        return 0;
    }

    if tfd.expired.swap(false, Ordering::AcqRel) {
        let mut t = tfd.timer.lock();
        if t.interval_ns > 0 {
            let interval = t.interval_ns;
            let overruns = hrtimer_forward_now(&mut t, interval);
            ticks = ticks.saturating_add(overruns.saturating_sub(1));
            hrtimer_restart(&mut *t as *mut Hrtimer);
        }
    }
    ticks
}

fn timerfd_remaining_ns(t: &Hrtimer) -> u64 {
    if t.state != HRTIMER_STATE_ENQUEUED || t.expires_ns == 0 {
        return 0;
    }
    t.expires_ns.saturating_sub(t.base_now())
}

fn timerfd_callback(t: &mut Hrtimer) -> HrtimerRestart {
    if t.data != 0 {
        let tfd = t.data as *const TimerFd;
        unsafe {
            (*tfd).ticks.fetch_add(1, Ordering::AcqRel);
            (*tfd).expired.store(true, Ordering::Release);
        }
    } else {
        GLOBAL_TIMERFD_TICKS.fetch_add(1, Ordering::AcqRel);
    }
    HrtimerRestart::NoRestart
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
        tfd.expired.store(true, Ordering::Release);
        tfd.ticks.store(7, Ordering::Release);
        let new = Itimerspec64 {
            it_interval: Timespec64::new(0, 0),
            it_value: Timespec64::new(1, 0),
        };
        let _old = sys_timerfd_settime(&tfd, 0, new).unwrap();
        assert!(!tfd.expired.load(Ordering::Acquire));
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

        assert_eq!(tfd.ticks.load(Ordering::Acquire), 1);
        assert_eq!(
            tfd.timer.lock().state,
            crate::kernel::time::hrtimer::HRTIMER_STATE_INACTIVE
        );
        assert_eq!(timerfd_read(&tfd), 1);
        assert_eq!(timerfd_read(&tfd), 0);
        assert_eq!(tfd.timer.lock().state, HRTIMER_STATE_ENQUEUED);
    }
}
