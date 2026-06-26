//! linux-parity: partial
//! linux-source: vendor/linux/kernel/time
//! Time-related syscalls — re-exports of the per-module entry points.

pub use super::posix_clock::{
    sys_clock_getres, sys_clock_gettime, sys_clock_nanosleep, sys_clock_settime,
};
pub use super::posix_timers::{
    sys_timer_create, sys_timer_delete, sys_timer_getoverrun, sys_timer_gettime, sys_timer_settime,
};
pub use super::timerfd::{sys_timerfd_create, sys_timerfd_gettime, sys_timerfd_settime};

use super::posix_clock::{EINVAL, Timespec64};

/// `nanosleep(rqtp, rmtp)` — wrapper that calls `clock_nanosleep(CLOCK_MONOTONIC, 0, ...)`.
pub fn sys_nanosleep(request: Timespec64, remain: Option<*mut Timespec64>) -> Result<(), i32> {
    super::posix_clock::sys_clock_nanosleep(
        super::posix_clock::CLOCK_MONOTONIC,
        false,
        request,
        remain,
    )
}

/// `adjtimex(buf)` — stub returning EINVAL until M59 lands the userspace
/// `struct timex` plumbing.
pub fn sys_adjtimex() -> Result<(), i32> {
    Err(EINVAL)
}
