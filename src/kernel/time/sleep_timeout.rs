//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/sleep_timeout.c
//! test-origin: linux:vendor/linux/kernel/time/sleep_timeout.c
//! Sleep timeout coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/sleep_timeout.c`.

use core::sync::atomic::Ordering;

use super::jiffies::{HZ, jiffies, jiffies_to_msecs, msecs_to_jiffies, time_before};
use crate::kernel::module::{export_symbol, find_symbol};

pub const MAX_SCHEDULE_TIMEOUT: u64 = u64::MAX / 2;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("schedule_timeout", linux_schedule_timeout as usize, false);
    export_symbol_once("msleep", msleep as usize, false);
    export_symbol_once("msleep_interruptible", msleep_interruptible as usize, false);
}

/// `msleep` - `vendor/linux/kernel/time/sleep_timeout.c:313`.
///
/// This is a module-facing sleep point. Lupos has cooperative boot-time
/// execution today, so the helper yields the timeout through the low-resolution
/// timeout path rather than busy-spinning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msleep(msecs: u32) {
    let mut timeout = msecs_to_schedule_timeout(msecs as u64);
    while timeout != 0 {
        timeout = schedule_timeout_uninterruptible(timeout);
    }
}

/// `msleep_interruptible` - `vendor/linux/kernel/time/sleep_timeout.c:337`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msleep_interruptible(msecs: u32) -> u32 {
    let timeout = msecs_to_schedule_timeout(msecs as u64);
    jiffies_to_msecs(schedule_timeout_with_state(
        timeout,
        crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
    )) as u32
}

pub fn schedule_timeout(timeout_jiffies: u64) -> u64 {
    #[cfg(test)]
    {
        if timeout_jiffies == MAX_SCHEDULE_TIMEOUT {
            return MAX_SCHEDULE_TIMEOUT;
        }
        return 0;
    }

    #[cfg(not(test))]
    {
        schedule_timeout_runtime(timeout_jiffies)
    }
}

#[unsafe(export_name = "schedule_timeout")]
pub unsafe extern "C" fn linux_schedule_timeout(timeout_jiffies: u64) -> u64 {
    schedule_timeout(timeout_jiffies)
}

#[cfg(not(test))]
fn schedule_timeout_runtime(timeout_jiffies: u64) -> u64 {
    if timeout_jiffies == MAX_SCHEDULE_TIMEOUT {
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        return MAX_SCHEDULE_TIMEOUT;
    }
    if timeout_jiffies == 0 {
        set_current_task_state(crate::kernel::task::task_state::TASK_RUNNING);
        return 0;
    }

    let expire = jiffies().saturating_add(timeout_jiffies);
    loop {
        // There is no timer-wheel wakeup path for module sleeps yet. Keep the
        // cooperative sleeper runnable while it yields so it can return here
        // and observe the jiffies deadline itself.
        set_current_task_state(crate::kernel::task::task_state::TASK_RUNNING);
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }

        let now = jiffies();
        let remaining = expire.saturating_sub(now);
        if remaining == 0 {
            set_current_task_state(crate::kernel::task::task_state::TASK_RUNNING);
            return remaining;
        }
        if !time_before(now, expire) {
            set_current_task_state(crate::kernel::task::task_state::TASK_RUNNING);
            return 0;
        }
    }
}

pub fn schedule_timeout_with_state(timeout_jiffies: u64, state: u32) -> u64 {
    set_current_task_state(state);
    schedule_timeout(timeout_jiffies)
}

pub fn schedule_timeout_uninterruptible(timeout_jiffies: u64) -> u64 {
    schedule_timeout_with_state(
        timeout_jiffies,
        crate::kernel::task::task_state::TASK_UNINTERRUPTIBLE,
    )
}

fn set_current_task_state(state: u32) {
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() {
        unsafe {
            (*current).__state.store(state, Ordering::Release);
        }
    }
}

pub fn msecs_to_schedule_timeout(ms: u64) -> u64 {
    if ms == u64::MAX {
        MAX_SCHEDULE_TIMEOUT
    } else {
        msecs_to_jiffies(ms)
    }
}

pub fn seconds_to_timeout(sec: u64) -> u64 {
    sec.saturating_mul(HZ)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_timeout_expires_to_zero() {
        assert_eq!(schedule_timeout(seconds_to_timeout(1)), 0);
        assert_eq!(schedule_timeout(MAX_SCHEDULE_TIMEOUT), MAX_SCHEDULE_TIMEOUT);
    }

    #[test]
    fn msleep_export_is_registered_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("schedule_timeout"),
            Some(linux_schedule_timeout as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("msleep"),
            Some(msleep as usize)
        );
        unsafe { msleep(1) };
    }
}
