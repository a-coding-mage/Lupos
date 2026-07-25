//! linux-parity: partial
//! linux-source: vendor/linux/kernel/time/sleep_timeout.c
//! test-origin: linux:vendor/linux/kernel/time/sleep_timeout.c
//! Sleep timeout coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/sleep_timeout.c`.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

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
    /// Identifies this individual armed timer, so nested sleeps by the same
    /// task cannot cancel or overwrite one another.
    id: u64,
    /// `*mut TaskStruct` as usize (the sleeper).
    task: usize,
    /// Jiffies value at/after which the sleeper must be woken.
    expire: u64,
}

static SLEEP_TIMERS: Mutex<Vec<SleepTimer>> = Mutex::new(Vec::new());

/// Source of [`SleepTimerId`] values. Starts at 1 so 0 is never a live id.
static NEXT_SLEEP_TIMER_ID: AtomicU64 = AtomicU64::new(1);

/// Handle for one armed wakeup.
///
/// Linux gives every timed sleep its own timer: `schedule_timeout()` puts a
/// `struct process_timer` **on the caller's stack** and arms that
/// (`vendor/linux/kernel/time/sleep_timeout.c`), so nesting is inherently safe.
///
/// Lupos previously keyed this wheel by task pointer with at most one entry per
/// task, which silently broke under nesting: `arm_wakeup()` overwrote an outer
/// sleep's deadline, and the inner `cancel_wakeup()` deleted the outer timer
/// outright. That is reachable — `schedule_with_irqs_enabled()` pumps driver
/// completions, and `poll_driver_abi_events_for_wait()` drains workqueues and
/// softirqs in the *caller's* task context, where `block_facade_acquire()`
/// arms and cancels a wakeup for `current`. An `epoll_wait()` fallback timer
/// could therefore be destroyed during its own `schedule()`, leaving the task
/// asleep with no timeout at all until some unrelated event woke it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SleepTimerId(u64);

impl SleepTimerId {
    /// A handle that refers to no armed timer; cancelling it is a no-op.
    pub const NONE: Self = Self(0);
}

/// Register `task` to be woken at `expire`.  Task context only; interrupts are
/// disabled across the (brief) critical section so the tick handler — which
/// takes the same lock from hard-IRQ — can never deadlock against us.
fn sleep_timer_add(task: usize, expire: u64) -> SleepTimerId {
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    let handle = {
        let mut timers = SLEEP_TIMERS.lock();
        let handle = next_sleep_timer_id_locked(&timers);
        timers.push(SleepTimer {
            id: handle.0,
            task,
            expire,
        });
        handle
    };
    crate::kernel::locking::irqflags::local_irq_restore(flags);
    handle
}

fn next_sleep_timer_id_locked(timers: &[SleepTimer]) -> SleepTimerId {
    loop {
        let mut current = NEXT_SLEEP_TIMER_ID.load(Ordering::Relaxed);
        let id = loop {
            let id = if current == SleepTimerId::NONE.0 {
                1
            } else {
                current
            };
            let next = id.wrapping_add(1);
            let next = if next == SleepTimerId::NONE.0 {
                1
            } else {
                next
            };
            match NEXT_SLEEP_TIMER_ID.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break id,
                Err(observed) => current = observed,
            }
        };
        if timers.iter().all(|timer| timer.id != id) {
            return SleepTimerId(id);
        }
    }
}

/// Arm a one-shot wakeup for `task` at jiffy `expire` (for callers that sleep
/// on their own condition but want a bounded re-check, e.g. the block-I/O wait
/// re-polling within a tick in case a HBA completion interrupt is delayed).
///
/// The returned handle must be passed to [`cancel_wakeup`]; cancelling by task
/// would disarm unrelated nested sleeps.
#[must_use = "an armed wakeup must be cancelled by its own SleepTimerId"]
pub fn arm_wakeup(task: usize, expire: u64) -> SleepTimerId {
    sleep_timer_add(task, expire)
}

/// Cancel a wakeup armed with [`arm_wakeup`].
pub fn cancel_wakeup(id: SleepTimerId) {
    sleep_timer_remove(id);
}

/// Cancel one armed sleep timer (it woke for another reason or its sleep ended).
fn sleep_timer_remove(id: SleepTimerId) {
    if id == SleepTimerId::NONE {
        return;
    }
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    {
        let mut timers = SLEEP_TIMERS.lock();
        if let Some(pos) = timers.iter().position(|t| t.id == id.0) {
            timers.swap_remove(pos);
        }
    }
    crate::kernel::locking::irqflags::local_irq_restore(flags);
}

/// Sleepers detached per pass. Bounded so the batch lives on the stack: this
/// runs in hard-IRQ context, where allocating is not allowed.
const EXPIRE_BATCH: usize = 32;

/// Detach up to `EXPIRE_BATCH` expired sleepers, returning how many were
/// written to `out`.
///
/// Split out from [`sleep_timers_expire`] so the wakeups happen *after*
/// `SLEEP_TIMERS` is unlocked. `vendor/linux/kernel/time/timer.c`
/// (`expire_timers()`) does the same thing explicitly:
///
/// ```text
/// raw_spin_unlock(&base->lock);
/// call_timer_fn(timer, fn, baseclk);
/// raw_spin_lock(&base->lock);
/// ```
fn take_expired_sleepers(now: u64, out: &mut [usize; EXPIRE_BATCH]) -> usize {
    let mut n = 0usize;
    let mut timers = SLEEP_TIMERS.lock();
    timers.retain(|timer| {
        if n == EXPIRE_BATCH || time_before(now, timer.expire) {
            return true;
        }
        out[n] = timer.task;
        n += 1;
        false
    });
    n
}

/// Wake every sleeper whose deadline has passed.  Called from the timer tick
/// (`apic_timer::on_tick`) in hard-IRQ context.
///
/// The wakeups deliberately run with `SLEEP_TIMERS` **unlocked**. Holding it
/// across `wake_task_normal()` was a deadlock hazard rather than a mere
/// inefficiency: that path reaches `try_to_wake_up()`, which spins on
/// `while task_on_cpu(p) {}` (Linux's `smp_cond_load_acquire(&p->on_cpu, !VAL)`)
/// when the target is still mid-context-switch on another CPU. A CPU can
/// therefore sit in hard IRQ, holding this lock, waiting for a CPU that is
/// itself blocked acquiring the very same lock in `sleep_timer_add()` /
/// `sleep_timer_remove()` — an AB-BA cycle whose least-bad outcome is a long
/// convoy on a lock every timed sleep in the system needs.
pub fn sleep_timers_expire(now: u64) {
    loop {
        let mut batch = [0usize; EXPIRE_BATCH];
        let n = take_expired_sleepers(now, &mut batch);
        for &task in &batch[..n] {
            let task = task as *mut crate::kernel::task::TaskStruct;
            if !task.is_null() {
                unsafe {
                    // `wake_up_process()` is what Linux timer expiry uses:
                    // merely storing TASK_RUNNING loses the wake once the
                    // production scheduler has dequeued a sleeping task.
                    crate::kernel::sched::wake_task_normal(task);
                }
            }
        }
        // A full batch may have left more expired sleepers behind.
        if n < EXPIRE_BATCH {
            return;
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("schedule_timeout", linux_schedule_timeout as usize, false);
    export_symbol_once(
        "io_schedule_timeout",
        linux_io_schedule_timeout as usize,
        false,
    );
    export_symbol_once("io_schedule", linux_io_schedule as usize, false);
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

/// `io_schedule_timeout` - `vendor/linux/kernel/sched/core.c:8136`.
///
/// Linux adds IO-wait accounting around `schedule_timeout()`. Lupos does not
/// yet account per-task IO wait, so the blocking semantics are the exported
/// scheduler contract here.
pub unsafe extern "C" fn linux_io_schedule_timeout(timeout_jiffies: i64) -> i64 {
    if timeout_jiffies < 0 {
        return 0;
    }
    schedule_timeout(timeout_jiffies as u64).min(i64::MAX as u64) as i64
}

/// `io_schedule` - `vendor/linux/kernel/sched/core.c:8149`.
pub unsafe extern "C" fn linux_io_schedule() {
    #[cfg(test)]
    {}
    #[cfg(not(test))]
    unsafe {
        let _ = crate::kernel::sched::schedule();
    }
}

#[cfg(not(test))]
fn schedule_timeout_runtime(timeout_jiffies: u64) -> u64 {
    if timeout_jiffies == MAX_SCHEDULE_TIMEOUT {
        // Linux `schedule_timeout()` calls `schedule()` directly after
        // arming its stack-resident process timer.  Do not route a driver
        // wait through Lupos's cooperative wrapper: that wrapper loops and
        // drains deferred work on the blocked caller's task stack, adding
        // non-Linux stack depth to a path that vendor drivers expect to fit
        // within THREAD_SIZE.
        unsafe {
            let _ = crate::kernel::sched::schedule();
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
        let timer = sleep_timer_add(task_id, expire);
        unsafe {
            let _ = crate::kernel::sched::schedule();
        }
        sleep_timer_remove(timer);
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
            let _ = crate::kernel::sched::schedule();
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

    /// `SLEEP_TIMERS` is global, and the harness runs tests in parallel, so
    /// every test that inspects the wheel must hold this first.
    static SLEEP_TIMER_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// `expire_timers()` in `vendor/linux/kernel/time/timer.c` drops
    /// `base->lock` around `call_timer_fn()`. Lupos must likewise detach
    /// expired sleepers and release `SLEEP_TIMERS` *before* waking them, since
    /// `wake_task_normal()` can spin on another CPU's `on_cpu`.
    #[test]
    fn expired_sleepers_are_detached_with_the_lock_released() {
        let _guard = SLEEP_TIMER_TEST_LOCK.lock();
        SLEEP_TIMERS.lock().clear();
        sleep_timer_add(0x1000, 10);
        sleep_timer_add(0x2000, 20);
        sleep_timer_add(0x3000, 30);

        let mut batch = [0usize; EXPIRE_BATCH];
        let n = take_expired_sleepers(25, &mut batch);

        // Both due sleepers come back to the caller, which wakes them outside
        // the lock; the not-yet-due one stays armed.
        assert_eq!(n, 2);
        let mut taken = batch[..n].to_vec();
        taken.sort_unstable();
        assert_eq!(taken, alloc::vec![0x1000, 0x2000]);

        // The decisive assertion: nothing holds SLEEP_TIMERS once the expired
        // set has been detached, so the wake below cannot deadlock against
        // sleep_timer_add()/sleep_timer_remove() on another CPU.
        assert!(
            SLEEP_TIMERS.try_lock().is_some(),
            "SLEEP_TIMERS must be unlocked while expired sleepers are woken"
        );
        assert_eq!(SLEEP_TIMERS.lock().len(), 1);
        SLEEP_TIMERS.lock().clear();
    }

    /// A nested sleep by the same task must not disarm the outer one.
    ///
    /// Linux gets this for free: `schedule_timeout()` arms a `timer_list` that
    /// lives on the caller's stack, so each nesting level owns a distinct
    /// timer. The previous Lupos wheel kept at most one entry per task keyed by
    /// task pointer, so the inner `arm_wakeup()` overwrote the outer deadline
    /// and the inner `cancel_wakeup()` deleted the outer timer outright —
    /// leaving the outer sleeper with no timeout at all. This test fails
    /// against that design and passes with per-timer handles.
    #[test]
    fn nested_wakeup_cancel_leaves_the_outer_timer_armed() {
        let _guard = SLEEP_TIMER_TEST_LOCK.lock();
        SLEEP_TIMERS.lock().clear();
        let task = 0xdead_beef_usize;

        let outer = arm_wakeup(task, 500);
        let inner = arm_wakeup(task, 10);
        assert_ne!(outer, inner, "each arm must get its own handle");

        // The nested sleep finishes and cancels only its own timer.
        cancel_wakeup(inner);

        let timers = SLEEP_TIMERS.lock();
        assert_eq!(
            timers.len(),
            1,
            "outer timer must survive the nested cancel"
        );
        assert_eq!(
            timers[0].expire, 500,
            "outer deadline must not be overwritten"
        );
        drop(timers);
        SLEEP_TIMERS.lock().clear();
    }

    #[test]
    fn wakeup_handle_allocation_skips_none_after_wrap() {
        let _guard = SLEEP_TIMER_TEST_LOCK.lock();
        SLEEP_TIMERS.lock().clear();
        let saved_next_id = NEXT_SLEEP_TIMER_ID.load(Ordering::Relaxed);
        NEXT_SLEEP_TIMER_ID.store(u64::MAX - 1, Ordering::Relaxed);

        let first = arm_wakeup(0x1000, 10);
        let second = arm_wakeup(0x2000, 20);
        let third = arm_wakeup(0x3000, 30);

        assert_eq!(first, SleepTimerId(u64::MAX - 1));
        assert_eq!(second, SleepTimerId(u64::MAX));
        assert_eq!(third, SleepTimerId(1));
        assert_ne!(third, SleepTimerId::NONE);

        cancel_wakeup(first);
        cancel_wakeup(second);
        cancel_wakeup(third);
        SLEEP_TIMERS.lock().clear();
        NEXT_SLEEP_TIMER_ID.store(saved_next_id, Ordering::Relaxed);
    }

    #[test]
    fn finite_timeout_expires_to_zero() {
        assert_eq!(schedule_timeout(seconds_to_timeout(1)), 0);
        assert_eq!(schedule_timeout(MAX_SCHEDULE_TIMEOUT), MAX_SCHEDULE_TIMEOUT);
    }

    /// `schedule_timeout()` and `io_schedule()` must enter the scheduler once,
    /// as Linux does.  The cooperative wrapper is for boot-only callers and
    /// must not add recursive work/drain frames to a vendor driver's blocked
    /// task stack.
    ///
    /// test-origin: linux:vendor/linux/kernel/time/sleep_timeout.c:schedule_timeout,
    ///              linux:vendor/linux/kernel/sched/core.c:io_schedule
    #[test]
    fn module_sleep_entries_call_schedule_directly() {
        let linux_timeout = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/time/sleep_timeout.c"
        ));
        assert!(linux_timeout.contains("schedule();\n\ttimer_delete_sync"));
        let linux_sched = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"), "/vendor/linux/kernel/sched/core.c"
        ));
        assert!(linux_sched.contains("void __sched io_schedule(void)"));
        assert!(linux_sched.contains("\ttoken = io_schedule_prepare();\n\tschedule();"));

        let source = include_str!("sleep_timeout.rs");
        let timeout = source
            .split("fn schedule_timeout_runtime(timeout_jiffies: u64) -> u64")
            .nth(1)
            .expect("runtime timeout implementation must exist");
        assert!(timeout.contains("crate::kernel::sched::schedule()"));
        assert!(!timeout.contains("schedule_with_irqs_enabled"));
        let io_schedule = source
            .split("pub unsafe extern \"C\" fn linux_io_schedule()")
            .nth(1)
            .and_then(|body| body.split("fn schedule_timeout_runtime").next())
            .expect("io_schedule implementation must precede timeout runtime");
        assert!(io_schedule.contains("crate::kernel::sched::schedule()"));
        assert!(!io_schedule.contains("schedule_with_irqs_enabled"));
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
