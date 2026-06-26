//! linux-parity: partial
//! linux-source: vendor/linux/kernel/time
//! Time subsystem — M36.
//!
//! Mirrors `vendor/linux/kernel/time/`.  Lupos M36 provides:
//!
//! | Module        | Linux source              | Description                     |
//! |---------------|---------------------------|---------------------------------|
//! | `jiffies`     | jiffies.c                 | tick counter, HZ, conversions   |
//! | `clocksource` | clocksource.c             | TSC + LAPIC clocksources        |
//! | `clockevents` | clockevents.c             | tick handlers                   |
//! | `timekeeping` | timekeeping.c             | wall-clock accumulator          |
//! | `hrtimer`     | hrtimer.c                 | high-resolution timers          |
//! | `posix_clock` | posix-clock.c             | clock_gettime/settime/getres    |
//! | `posix_timers`| posix-timers.c            | timer_create/settime/gettime    |
//! | `timerfd`     | timerfd.c                 | fd-backed timers                |
//! | `alarmtimer`  | alarmtimer.c              | RTC-backed alarms               |
//! | `syscalls`    | (multiple)                | sys_clock_*, sys_timer_*        |

pub mod alarmtimer;
pub mod clockevents;
pub mod clocksource;
pub mod clocksource_wdtest;
pub mod hrtimer;
pub mod itimer;
pub mod jiffies;
pub mod namespace;
pub mod namespace_vdso;
pub mod ntp;
pub mod posix_clock;
pub mod posix_cpu_timers;
pub mod posix_timers;
pub mod sched_clock;
pub mod sleep_timeout;
pub mod syscalls;
pub mod test_udelay;
pub mod tick_broadcast;
pub mod tick_broadcast_hrtimer;
pub mod tick_common;
pub mod tick_legacy;
pub mod tick_oneshot;
pub mod tick_sched;
pub mod time;
pub mod time_test;
pub mod timeconv;
pub mod timecounter;
pub mod timekeeping;
pub mod timekeeping_debug;
pub mod timer;
pub mod timer_list;
pub mod timer_migration;
pub mod timerfd;
pub mod vsyscall;

pub use clockevents::{Clockevents, tick_handle_periodic};
pub use clocksource::{Clocksource, clocksource_register};
pub use hrtimer::{
    ClockBase, Hrtimer, HrtimerMode, hrtimer_cancel, hrtimer_init, hrtimer_run_queues,
    hrtimer_start,
};
pub use jiffies::{
    HZ, jiffies, jiffies_to_msecs, jiffies_to_usecs, msecs_to_jiffies, time_after, time_before,
};
pub use posix_clock::{
    CLOCK_BOOTTIME, CLOCK_MONOTONIC, CLOCK_MONOTONIC_COARSE, CLOCK_MONOTONIC_RAW,
    CLOCK_PROCESS_CPUTIME_ID, CLOCK_REALTIME, CLOCK_REALTIME_COARSE, CLOCK_TAI,
    CLOCK_THREAD_CPUTIME_ID, ClockId, Timespec64, sys_clock_getres, sys_clock_gettime,
    sys_clock_nanosleep, sys_clock_settime,
};
pub use posix_timers::{
    Itimerspec64, PosixTimer, sys_timer_create, sys_timer_delete, sys_timer_gettime,
    sys_timer_settime,
};
pub use timekeeping::{Timekeeper, ktime_get, ktime_get_boottime, ktime_get_real};
pub use timerfd::{TimerFd, sys_timerfd_create, sys_timerfd_gettime, sys_timerfd_settime};
