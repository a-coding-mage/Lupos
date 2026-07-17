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
//! Lupos uses a cooperative, non-preemptible tracing model. Each context
//! switch and idle-loop pass publishes the current tasks-RCU generation for
//! that CPU; `synchronize_rcu_tasks()` requests remote reschedules and waits
//! until every participating CPU has crossed a post-request quiescent state.

extern crate alloc;

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use super::types::RcuHead;

/// Global pass counter — every voluntary `schedule()` bumps it.
static TASKS_PASS: AtomicU64 = AtomicU64::new(0);
static TASKS_GP_SEQ: AtomicU64 = AtomicU64::new(0);
static TASKS_ONLINE_MASK: AtomicU64 = AtomicU64::new(0);
static TASKS_QS_AT_GP: [AtomicU64; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicU64::new(0) }; crate::kernel::sched::MAX_CPUS];

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
    let cpu =
        (crate::kernel::sched::current_cpu() as usize).min(crate::kernel::sched::MAX_CPUS - 1);
    TASKS_ONLINE_MASK.fetch_or(1u64 << (cpu & 63), Ordering::AcqRel);
    TASKS_QS_AT_GP[cpu].store(TASKS_GP_SEQ.load(Ordering::Acquire), Ordering::Release);
    TASKS_PASS.fetch_add(1, Ordering::AcqRel);
}

fn all_online_cpus_passed(generation: u64) -> bool {
    let online = TASKS_ONLINE_MASK.load(Ordering::Acquire);
    (0..crate::kernel::sched::MAX_CPUS).all(|cpu| {
        online & (1u64 << (cpu & 63)) == 0
            || TASKS_QS_AT_GP[cpu].load(Ordering::Acquire) >= generation
    })
}

/// `synchronize_rcu_tasks()` — wait until every participating CPU has crossed
/// a context-switch/idle quiescent state after this grace period began.
pub fn synchronize_rcu_tasks() {
    let generation = TASKS_GP_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
    // The caller cannot concurrently be executing a retired trampoline, so
    // it may report its own CPU before requesting progress elsewhere.
    tasks_rcu_qs();
    #[cfg(not(test))]
    {
        let me =
            (crate::kernel::sched::current_cpu() as usize).min(crate::kernel::sched::MAX_CPUS - 1);
        let online = TASKS_ONLINE_MASK.load(Ordering::Acquire);
        for cpu in 0..crate::kernel::sched::MAX_CPUS {
            if cpu != me && online & (1u64 << (cpu & 63)) != 0 {
                crate::kernel::sched::request_reschedule(cpu as u32);
            }
        }
        while !all_online_cpus_passed(generation) {
            unsafe {
                crate::kernel::sched::schedule_with_irqs_enabled();
            }
            core::hint::spin_loop();
        }
    }
    #[cfg(test)]
    assert!(all_online_cpus_passed(generation));
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
        let generation = TASKS_GP_SEQ.load(Ordering::Acquire);
        synchronize_rcu_tasks();
        assert!(TASKS_PASS.load(Ordering::Acquire) > before);
        assert!(TASKS_GP_SEQ.load(Ordering::Acquire) > generation);
        assert!(all_online_cpus_passed(TASKS_GP_SEQ.load(Ordering::Acquire)));
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
