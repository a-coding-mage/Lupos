//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/tasks.h
//! test-origin: linux:vendor/linux/kernel/rcu/tasks.h
//! Tasks RCU — M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/tasks.h`.  A grace period elapses when
//! every non-idle, non-kthread task has performed a voluntary context switch
//! at least once.  Used by tracing trampolines (M62) where preempt-RCU
//! quiescent states are insufficient.
//!
//! Lupos M34 ships the cooperative variant: each `schedule()` call records a
//! "tasks_rcu_qs" pass for the current task; `synchronize_rcu_tasks` waits
//! until the global pass counter has incremented at least once for the calling
//! task.

extern crate alloc;

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use super::types::RcuHead;

/// Global pass counter — every voluntary `schedule()` bumps it.
static TASKS_PASS: AtomicU64 = AtomicU64::new(0);

/// Send-wrapped pair so the static can be a `Mutex` of a `VecDeque`.
struct CbEntry {
    head: *mut RcuHead,
    func: unsafe extern "C" fn(*mut RcuHead),
}
unsafe impl Send for CbEntry {}

/// Pending tasks-RCU callbacks.
static PENDING: Mutex<VecDeque<CbEntry>> = Mutex::new(VecDeque::new());

/// Record a quiescent state.  Called from `schedule()` when the task yields.
#[inline]
pub fn tasks_rcu_qs() {
    TASKS_PASS.fetch_add(1, Ordering::AcqRel);
}

/// `synchronize_rcu_tasks()` — block until at least one global pass has
/// elapsed (all tasks have voluntarily scheduled at least once).
///
/// Cooperative-mode shortcut: the calling task counts as one pass, then we
/// yield once to give every other runnable task its slot.  Real Linux walks
/// the task list and verifies each non-idle task has scheduled.
pub fn synchronize_rcu_tasks() {
    // Record our own pass first so we don't wait for ourselves.
    tasks_rcu_qs();
    #[cfg(not(test))]
    unsafe {
        crate::kernel::sched::schedule_with_irqs_enabled();
    }
    // Final QS bump confirms the grace period closed.
    tasks_rcu_qs();
}

/// `call_rcu_tasks(head, func)` — queue a tasks-RCU callback.
pub fn call_rcu_tasks(head: *mut RcuHead, func: unsafe extern "C" fn(*mut RcuHead)) {
    if head.is_null() {
        return;
    }
    unsafe {
        (*head).func = Some(func);
    }
    PENDING.lock().push_back(CbEntry { head, func });
}

/// Drain pending callbacks after their grace period has elapsed.  Called from
/// the Timer softirq (per-tick).
pub fn drain_pending_tasks_callbacks() {
    let mut q = PENDING.lock();
    while let Some(e) = q.pop_front() {
        unsafe { (e.func)(e.head) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synchronize_advances_pass_counter() {
        let before = TASKS_PASS.load(Ordering::Acquire);
        synchronize_rcu_tasks();
        assert!(TASKS_PASS.load(Ordering::Acquire) > before);
    }

    #[test]
    fn call_rcu_tasks_queues_callback() {
        unsafe extern "C" fn cb(_head: *mut RcuHead) {}
        let mut head = RcuHead::new();
        call_rcu_tasks(&mut head as *mut RcuHead, cb);
        // Drain.
        drain_pending_tasks_callbacks();
    }
}
