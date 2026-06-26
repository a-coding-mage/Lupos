//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/posix-clock.c
//! test-origin: linux:vendor/linux/kernel/time/posix-clock.c
//! POSIX clock APIs — `clock_gettime` / `clock_settime` / `clock_getres` /
//! `clock_nanosleep` (M36).
//!
//! Mirrors `vendor/linux/kernel/time/posix-stubs.c` for UAPI parity.

use super::hrtimer::{
    ClockBase, Hrtimer, HrtimerMode, HrtimerRestart, hrtimer_cancel, hrtimer_init,
    hrtimer_run_queues, hrtimer_start,
};
use super::jiffies::NSEC_PER_TICK;
use super::timekeeping::{TK, ktime_get, ktime_get_boottime, ktime_get_real, tk_set_wall_seconds};
use crate::kernel::capability::{CAP_SYS_TIME, capable};

// ── UAPI: CLOCK_* ids ────────────────────────────────────────────────────────

pub const CLOCK_REALTIME: i32 = 0;
pub const CLOCK_MONOTONIC: i32 = 1;
pub const CLOCK_PROCESS_CPUTIME_ID: i32 = 2;
pub const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
pub const CLOCK_MONOTONIC_RAW: i32 = 4;
pub const CLOCK_REALTIME_COARSE: i32 = 5;
pub const CLOCK_MONOTONIC_COARSE: i32 = 6;
pub const CLOCK_BOOTTIME: i32 = 7;
pub const CLOCK_REALTIME_ALARM: i32 = 8;
pub const CLOCK_BOOTTIME_ALARM: i32 = 9;
pub const CLOCK_TAI: i32 = 11;

pub type ClockId = i32;

/// `struct __kernel_timespec64` (UAPI).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timespec64 {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

impl Timespec64 {
    pub const fn new(sec: i64, nsec: i64) -> Self {
        Self {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    pub fn from_ns(ns: u64) -> Self {
        Self {
            tv_sec: (ns / 1_000_000_000) as i64,
            tv_nsec: (ns % 1_000_000_000) as i64,
        }
    }
    pub fn to_ns(self) -> u64 {
        (self.tv_sec as u64)
            .saturating_mul(1_000_000_000)
            .saturating_add(self.tv_nsec as u64)
    }
    pub fn is_valid(self) -> bool {
        self.tv_sec >= 0 && self.tv_nsec >= 0 && self.tv_nsec < 1_000_000_000
    }
}

const _: () = assert!(core::mem::size_of::<Timespec64>() == 16);

// ── errno values ─────────────────────────────────────────────────────────────

pub const EINVAL: i32 = 22;
pub const EFAULT: i32 = 14;
pub const ENOTSUP: i32 = 95;
pub const EPERM: i32 = 1;

// ── sys_clock_gettime ────────────────────────────────────────────────────────

pub fn sys_clock_gettime(clock: ClockId) -> Result<Timespec64, i32> {
    let ns = match clock {
        CLOCK_REALTIME | CLOCK_REALTIME_COARSE => ktime_get_real(),
        CLOCK_MONOTONIC | CLOCK_MONOTONIC_COARSE | CLOCK_MONOTONIC_RAW => ktime_get(),
        CLOCK_BOOTTIME => ktime_get_boottime(),
        CLOCK_TAI => {
            ktime_get_real()
                + TK.tai_offset.load(core::sync::atomic::Ordering::Acquire) * 1_000_000_000
        }
        _ => return Err(EINVAL),
    };
    Ok(Timespec64::from_ns(ns))
}

pub fn sys_clock_settime(clock: ClockId, tp: Timespec64) -> Result<(), i32> {
    if !tp.is_valid() {
        return Err(EINVAL);
    }
    match clock {
        CLOCK_REALTIME => {
            if !capable(CAP_SYS_TIME) {
                return Err(EPERM);
            }
            tk_set_wall_seconds(tp.tv_sec as u64);
            Ok(())
        }
        CLOCK_MONOTONIC | CLOCK_BOOTTIME => Err(EINVAL), // monotonic clocks are read-only
        _ => Err(EINVAL),
    }
}

pub fn sys_clock_getres(clock: ClockId) -> Result<Timespec64, i32> {
    match clock {
        CLOCK_REALTIME
        | CLOCK_MONOTONIC
        | CLOCK_BOOTTIME
        | CLOCK_TAI
        | CLOCK_REALTIME_COARSE
        | CLOCK_MONOTONIC_COARSE
        | CLOCK_MONOTONIC_RAW => Ok(Timespec64::from_ns(NSEC_PER_TICK)),
        _ => Err(EINVAL),
    }
}

fn nanosleep_timer_wake(t: &mut Hrtimer) -> HrtimerRestart {
    let task = t.data as *mut crate::kernel::task::TaskStruct;
    if !task.is_null() {
        unsafe {
            crate::kernel::sched::wake_task(task);
        }
    }
    HrtimerRestart::NoRestart
}

/// `clock_nanosleep(clockid, flags, request, remain)`.
pub fn sys_clock_nanosleep(
    clock: ClockId,
    abs_time: bool,
    request: Timespec64,
    remain: Option<*mut Timespec64>,
) -> Result<(), i32> {
    if !request.is_valid() {
        return Err(EINVAL);
    }
    let now_ns = match clock {
        CLOCK_REALTIME => ktime_get_real(),
        CLOCK_MONOTONIC | CLOCK_BOOTTIME => ktime_get(),
        CLOCK_TAI => ktime_get_real(),
        _ => return Err(EINVAL),
    };
    let requested_ns = request.to_ns();
    let target = if abs_time {
        requested_ns
    } else {
        let duration_ns = if requested_ns > 0 && requested_ns < NSEC_PER_TICK.saturating_mul(10) {
            NSEC_PER_TICK.saturating_mul(10)
        } else {
            requested_ns
        };
        now_ns.saturating_add(duration_ns)
    };
    if target <= now_ns {
        return Ok(());
    }

    // Park on an hrtimer.  In cooperative mode we cycle hrtimer_run_queues
    // until the deadline is reached.
    let mut t = Hrtimer::new();
    hrtimer_init(&mut t, ClockBase::Monotonic, HrtimerMode::Abs);
    let target_mono = if clock == CLOCK_MONOTONIC || clock == CLOCK_BOOTTIME {
        target
    } else {
        // Translate REALTIME to MONOTONIC by removing the wall-clock offset.
        ktime_get().saturating_add(target.saturating_sub(now_ns))
    };
    #[cfg(not(test))]
    {
        t.data = unsafe { crate::kernel::sched::get_current() } as usize;
    }
    t.function = Some(nanosleep_timer_wake);
    hrtimer_start(&mut t as *mut Hrtimer, target_mono, HrtimerMode::Abs);

    while ktime_get() < target_mono {
        #[cfg(not(test))]
        {
            crate::init::rootfs::drain_console_control_bytes();
            let current = unsafe { crate::kernel::sched::get_current() };
            if crate::kernel::signal::has_pending_signals(current) {
                if !current.is_null() {
                    unsafe {
                        (*current).__state.store(
                            crate::kernel::task::task_state::TASK_RUNNING,
                            core::sync::atomic::Ordering::Release,
                        );
                    }
                }
                // Linux writes the remaining time only for interrupted
                // RELATIVE sleeps (hrtimer_nanosleep -> nanosleep_copyout);
                // `remain` is the syscall wrapper's kernel-side scratch
                // buffer, which the wrapper copies out to userspace.
                if !abs_time && let Some(remain) = remain {
                    unsafe {
                        *remain = Timespec64::from_ns(target_mono.saturating_sub(ktime_get()));
                    }
                }
                return finish_nanosleep_timer(
                    &mut t as *mut Hrtimer,
                    Err(crate::include::uapi::errno::EINTR),
                );
            }
            // The current cooperative scheduler only drains timer softirqs from
            // the idle loop. A userspace sleep can run with no idle slice, so
            // advance the periodic tick here before yielding.
            if !current.is_null() {
                unsafe {
                    (*current).__state.store(
                        crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                        core::sync::atomic::Ordering::Release,
                    );
                }
            }
            crate::kernel::time::clockevents::tick_handle_periodic();
            unsafe {
                crate::kernel::sched::schedule_with_irqs_enabled();
            }
        }
        #[cfg(test)]
        {
            // In tests, advance the monotonic clock manually so we exit.
            super::timekeeping::tick_advance_walltime();
            hrtimer_run_queues();
        }
    }
    // A sleep that ran to completion leaves the caller's remain buffer
    // untouched, exactly like Linux (rem is only meaningful after EINTR).
    #[cfg(not(test))]
    {
        let current = unsafe { crate::kernel::sched::get_current() };
        if !current.is_null() {
            unsafe {
                (*current).__state.store(
                    crate::kernel::task::task_state::TASK_RUNNING,
                    core::sync::atomic::Ordering::Release,
                );
            }
        }
    }
    finish_nanosleep_timer(&mut t as *mut Hrtimer, Ok(()))
}

fn finish_nanosleep_timer(t: *mut Hrtimer, result: Result<(), i32>) -> Result<(), i32> {
    let _ = hrtimer_cancel(t);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timespec64_layout_matches_linux() {
        assert_eq!(core::mem::size_of::<Timespec64>(), 16);
    }

    #[test]
    fn completed_relative_sleep_succeeds_and_leaves_remain_untouched() {
        // `remain` is the syscall wrapper's KERNEL-side scratch buffer, not a
        // user pointer — treating it as one made every coreutils `sleep`
        // (clock_nanosleep with non-NULL rem) fail EFAULT via access_ok.
        // Linux semantics: rem is written only when a relative sleep is
        // interrupted (vendor/linux/kernel/time/hrtimer.c::hrtimer_nanosleep
        // -> nanosleep_copyout in the restart path); a completed sleep
        // leaves it untouched.
        let mut remain = Timespec64 {
            tv_sec: 77,
            tv_nsec: 55,
        };
        let result = sys_clock_nanosleep(
            CLOCK_MONOTONIC,
            false,
            Timespec64 {
                tv_sec: 0,
                tv_nsec: 1,
            },
            Some(&raw mut remain),
        );
        assert_eq!(result, Ok(()));
        assert_eq!(
            (remain.tv_sec, remain.tv_nsec),
            (77, 55),
            "completed sleeps must not touch the caller's remain buffer"
        );
    }

    #[test]
    fn completed_realtime_sleep_with_remain_does_not_efault() {
        // Regression shape of trixie coreutils sleep:
        // clock_nanosleep(CLOCK_REALTIME, 0, &req, &rem).
        let mut remain = Timespec64::default();
        assert_eq!(
            sys_clock_nanosleep(
                CLOCK_REALTIME,
                false,
                Timespec64 {
                    tv_sec: 0,
                    tv_nsec: 1,
                },
                Some(&raw mut remain),
            ),
            Ok(())
        );
    }

    #[test]
    fn clock_gettime_monotonic_is_monotonic() {
        let a = sys_clock_gettime(CLOCK_MONOTONIC).unwrap();
        super::super::timekeeping::tick_advance_walltime();
        let b = sys_clock_gettime(CLOCK_MONOTONIC).unwrap();
        assert!(b.to_ns() > a.to_ns());
    }

    #[test]
    fn clock_gettime_invalid_id_returns_einval() {
        assert_eq!(sys_clock_gettime(42), Err(EINVAL));
    }

    #[test]
    fn clock_settime_realtime_requires_cap_sys_time() {
        use alloc::boxed::Box;

        use crate::kernel::{cred::Cred, sched, task::TaskStruct};

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let cred = Box::new(unsafe { core::mem::zeroed::<Cred>() });
        current.cred = &*cred as *const Cred;

        tk_set_wall_seconds(7);
        unsafe { sched::set_current(&mut *current as *mut TaskStruct) };

        assert_eq!(
            sys_clock_settime(CLOCK_REALTIME, Timespec64::new(123_456, 0)),
            Err(EPERM)
        );
        assert_eq!(sys_clock_gettime(CLOCK_REALTIME).unwrap().tv_sec, 7);

        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn interrupted_nanosleep_cancels_stack_hrtimer() {
        let mut timer = Hrtimer::new();
        hrtimer_init(&mut timer, ClockBase::Monotonic, HrtimerMode::Abs);
        timer.function = Some(|_| HrtimerRestart::NoRestart);
        hrtimer_start(&mut timer as *mut Hrtimer, u64::MAX / 2, HrtimerMode::Abs);

        assert_eq!(
            finish_nanosleep_timer(
                &mut timer as *mut Hrtimer,
                Err(crate::include::uapi::errno::EINTR)
            ),
            Err(crate::include::uapi::errno::EINTR)
        );
        assert_eq!(
            timer.state,
            crate::kernel::time::hrtimer::HRTIMER_STATE_INACTIVE
        );
        assert!(!hrtimer_cancel(&mut timer as *mut Hrtimer));
    }

    #[test]
    fn clock_getres_returns_one_tick() {
        let r = sys_clock_getres(CLOCK_MONOTONIC).unwrap();
        assert_eq!(r.to_ns(), NSEC_PER_TICK);
    }

    #[test]
    fn clock_constants_match_linux() {
        assert_eq!(CLOCK_REALTIME, 0);
        assert_eq!(CLOCK_MONOTONIC, 1);
        assert_eq!(CLOCK_BOOTTIME, 7);
        assert_eq!(CLOCK_TAI, 11);
    }

    #[test]
    fn timespec_is_valid_rejects_negative() {
        assert!(!Timespec64::new(-1, 0).is_valid());
        assert!(!Timespec64::new(0, -1).is_valid());
        assert!(!Timespec64::new(0, 1_000_000_000).is_valid());
        assert!(Timespec64::new(0, 999_999_999).is_valid());
    }
}
