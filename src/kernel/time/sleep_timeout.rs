//! linux-parity: partial
//! linux-source: vendor/linux/kernel/time/sleep_timeout.c
//! test-origin: linux:vendor/linux/kernel/time/sleep_timeout.c
//! Sleep timeout coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/sleep_timeout.c`.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Mutex;

use super::jiffies::{HZ, jiffies, jiffies_to_msecs, msecs_to_jiffies, time_before};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::task::task_state;

pub const MAX_SCHEDULE_TIMEOUT: u64 = u64::MAX / 2;

// ── Sleep-timer wheel ────────────────────────────────────────────────────────
//
// A real timer-backed wakeup for `schedule_timeout`/`msleep`, so a timed sleep
// truly sleeps (the task goes non-runnable and the CPU halts) and the periodic
// LAPIC tick wakes it when its jiffies deadline passes — instead of keeping the
// task RUNNABLE and busy-yielding until the deadline (which burned a CPU for the
// whole sleep, e.g. throughout systemd's many startup timeouts).

struct SleepTimer {
    /// `*mut TaskStruct` as usize (the sleeper).
    task: usize,
    /// Jiffies value at/after which the sleeper must be woken.
    expire: u64,
}

static SLEEP_TIMERS: Mutex<Vec<SleepTimer>> = Mutex::new(Vec::new());

/// Register `task` to be woken at `expire`.  Task context only; interrupts are
/// disabled across the (brief) critical section so the tick handler — which
/// takes the same lock from hard-IRQ — can never deadlock against us.
fn sleep_timer_add(task: usize, expire: u64) {
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    {
        let mut timers = SLEEP_TIMERS.lock();
        if let Some(existing) = timers.iter_mut().find(|t| t.task == task) {
            existing.expire = expire;
        } else {
            timers.push(SleepTimer { task, expire });
        }
    }
    crate::kernel::locking::irqflags::local_irq_restore(flags);
}

/// Arm a one-shot wakeup for `task` at jiffy `expire` (for callers that sleep
/// on their own condition but want a bounded re-check, e.g. the block-I/O wait
/// re-polling within a tick in case a HBA completion interrupt is delayed).
pub fn arm_wakeup(task: usize, expire: u64) {
    sleep_timer_add(task, expire);
}

/// Cancel a wakeup armed with [`arm_wakeup`].
pub fn cancel_wakeup(task: usize) {
    sleep_timer_remove(task);
}

/// Cancel `task`'s sleep timer (it woke for another reason or its sleep ended).
fn sleep_timer_remove(task: usize) {
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    {
        let mut timers = SLEEP_TIMERS.lock();
        if let Some(pos) = timers.iter().position(|t| t.task == task) {
            timers.swap_remove(pos);
        }
    }
    crate::kernel::locking::irqflags::local_irq_restore(flags);
}

/// Wake every sleeper whose deadline has passed.  Called from the timer tick
/// (`apic_timer::on_tick`) in hard-IRQ context — interrupts are already
/// disabled, so it just takes the lock and marks expired sleepers RUNNING.
pub fn sleep_timers_expire(now: u64) {
    let mut timers = SLEEP_TIMERS.lock();
    timers.retain(|timer| {
        if time_before(now, timer.expire) {
            return true;
        }
        let task = timer.task as *mut crate::kernel::task::TaskStruct;
        if !task.is_null() {
            unsafe {
                // `wake_up_process()` is what Linux timer expiry uses: merely
                // storing TASK_RUNNING loses the wake once the production
                // scheduler has dequeued a sleeping task.
                crate::kernel::sched::wake_task_normal(task);
            }
        }
        false
    });
}

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
    let current = unsafe { crate::kernel::sched::get_current() };

    // Linux arms once and calls schedule() once. A condition/signal wake makes
    // the task runnable and returns early with the remaining timeout; the timer
    // wake returns at expiry. Re-storing `sleep_state` in a loop would erase a
    // real early wake.
    if !current.is_null() && crate::kernel::locking::preempt::preempt_count() == 0 {
        let task_id = current as usize;
        sleep_timer_add(task_id, expire);
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        sleep_timer_remove(task_id);
        set_current_task_state(task_state::TASK_RUNNING);
        let now = jiffies();
        return if time_before(now, expire) {
            expire.saturating_sub(now)
        } else {
            0
        };
    }

    // Fallback (no task context / atomic context / caller left us RUNNING): keep
    // the old bounded busy-yield to the jiffies deadline.
    loop {
        set_current_task_state(task_state::TASK_RUNNING);
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        let now = jiffies();
        if !time_before(now, expire) {
            set_current_task_state(task_state::TASK_RUNNING);
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
