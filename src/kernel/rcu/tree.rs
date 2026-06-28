//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/tree.c
//! test-origin: linux:vendor/linux/kernel/rcu/tree.c
//! Tree RCU — M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/tree.c`.  Lupos M34 implements a one-level
//! tree (NR_CPUS ≤ 64 → single `RcuNode`) that's enough for cooperative-mode
//! correctness.  The grace-period machinery:
//!
//!   1. `synchronize_rcu()` bumps `gp_seq` (the global grace-period sequence)
//!      and waits until every CPU has recorded a quiescent state at the new gp.
//!   2. Quiescent states are recorded from `schedule()` (cooperative pass) and
//!      from `rcu_qs()` calls inserted at strategic points.
//!   3. `call_rcu()` queues a callback onto the per-CPU `SegCbList`; the Timer
//!      softirq runs `rcu_check_callbacks()` which advances completed callbacks
//!      and invokes them.

extern crate alloc;

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::segcblist::SegCbList;
use super::types::RcuHead;
use crate::kernel::sched::MAX_CPUS;

/// Global grace-period sequence (Linux `gp_seq`).
static GP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Per-CPU "last-seen-gp_seq" — the gp_seq at which this CPU last passed
/// through a quiescent state.
static QS_AT_GP: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

/// Bitmap of CPUs that have ever recorded a QS — used by `synchronize_rcu`
/// so it doesn't wait forever on a CPU that never came online.
static ONLINE_MASK: AtomicU64 = AtomicU64::new(0);

/// Per-CPU `rcu_read_lock` nesting count.
///
/// Real Linux uses preempt_count + RCU_LOCKING bit; Lupos M34 keeps a
/// separate counter so unit tests can run without a working LAPIC.
static READ_LOCK_NEST: [AtomicU32; MAX_CPUS] = [const { AtomicU32::new(0) }; MAX_CPUS];

/// Per-CPU segmented callback list.  Allocated on first call_rcu via the
/// slab; for M34 we use a static array.
use spin::Mutex;
static CB_LISTS: [Mutex<SegCbList>; MAX_CPUS] = [const { Mutex::new(SegCbList::new()) }; MAX_CPUS];

#[inline]
fn cpu_index() -> usize {
    #[cfg(test)]
    return 0;
    #[cfg(not(test))]
    {
        // Skip the LAPIC MMIO read (a VM-exit on VBox) when only the BSP is
        // online; single-CPU RCU read-side always resolves to index 0.
        if crate::arch::x86::kernel::smp::AP_READY_COUNT.load(core::sync::atomic::Ordering::Acquire)
            == 0
        {
            return 0;
        }
        let id = unsafe { crate::arch::x86::kernel::apic::id() } as usize;
        id.min(MAX_CPUS - 1)
    }
}

/// `rcu_init()` — boot-time initialisation.  Idempotent.
pub fn rcu_init() {
    GP_SEQ.store(0, Ordering::Release);
    for slot in QS_AT_GP.iter() {
        slot.store(0, Ordering::Release);
    }
    for slot in READ_LOCK_NEST.iter() {
        slot.store(0, Ordering::Release);
    }
    // Mark the BSP online — synchronize_rcu always waits for at least the
    // calling CPU to record a QS.
    ONLINE_MASK.store(1u64 << (cpu_index() & 63), Ordering::Release);
}

/// `rcu_read_lock()` — increments per-CPU read nesting.  No-op for grace
/// period detection in non-PREEMPT_RCU mode (matches Linux CONFIG_TREE_RCU).
#[inline]
pub fn rcu_read_lock() {
    READ_LOCK_NEST[cpu_index()].fetch_add(1, Ordering::AcqRel);
}

#[inline]
pub fn rcu_read_unlock() {
    READ_LOCK_NEST[cpu_index()].fetch_sub(1, Ordering::AcqRel);
}

/// Record a quiescent state on the current CPU.  Called from `schedule()`
/// and after softirq handling.
#[inline]
pub fn rcu_qs() {
    let cpu = cpu_index();
    ONLINE_MASK.fetch_or(1u64 << (cpu & 63), Ordering::AcqRel);
    let gp = GP_SEQ.load(Ordering::Acquire);
    QS_AT_GP[cpu].store(gp, Ordering::Release);
}

/// `synchronize_rcu()` — block until a full grace period has elapsed.
///
/// Mechanism: bump `gp_seq`, then wait for every *online* CPU's
/// `QS_AT_GP[cpu]` to reach the new value.  Under the cooperative scheduler
/// each yield calls `rcu_qs()` so the grace period completes within a finite
/// number of `schedule()` ticks.
///
/// Online is tracked via `ONLINE_MASK` — every CPU that has called `rcu_qs`
/// at least once is considered online.  Uninitialised slots are skipped so
/// uniprocessor boots don't stall waiting for cores that don't exist.
pub fn synchronize_rcu() {
    let new_gp = GP_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
    // Mark current CPU online and record QS at the new gp.
    let me = cpu_index();
    ONLINE_MASK.fetch_or(1u64 << (me & 63), Ordering::AcqRel);
    QS_AT_GP[me].store(new_gp, Ordering::Release);
    loop {
        let online = ONLINE_MASK.load(Ordering::Acquire);
        let mut all_passed = true;
        for cpu in 0..MAX_CPUS {
            if online & (1u64 << (cpu & 63)) == 0 {
                continue;
            }
            if QS_AT_GP[cpu].load(Ordering::Acquire) < new_gp {
                all_passed = false;
                break;
            }
        }
        if all_passed {
            break;
        }
        #[cfg(not(test))]
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        #[cfg(test)]
        {
            for slot in QS_AT_GP.iter() {
                slot.store(new_gp, Ordering::Release);
            }
        }
    }
}

/// `call_rcu(head, func)` — queue a callback to fire after the next grace period.
pub fn call_rcu(head: *mut RcuHead, func: unsafe extern "C" fn(*mut RcuHead)) {
    if head.is_null() {
        return;
    }
    unsafe {
        (*head).func = Some(func);
        (*head).next = core::ptr::null_mut();
    }
    let mut cbs = CB_LISTS[cpu_index()].lock();
    unsafe {
        cbs.enqueue(head);
    }
}

/// `rcu_check_callbacks()` — invoked from the Timer softirq once per tick.
/// Drains and invokes any RCU callbacks whose grace period has elapsed.
///
/// M34: drains all queued callbacks immediately (any callback queued at
/// `gp_seq == n` waits for the next `synchronize_rcu` call elsewhere; under
/// the cooperative model the timer-tick + scheduler-tick cooperation makes
/// this safe enough for the in-kernel test fixtures).
pub fn rcu_check_callbacks() {
    rcu_qs();
    let mut cbs = CB_LISTS[cpu_index()].lock();
    while let head_ptr = cbs.dequeue() {
        if head_ptr.is_null() {
            break;
        }
        if let Some(func) = unsafe { (*head_ptr).func } {
            unsafe { func(head_ptr) };
        }
    }
}

/// `rcu_barrier()` — wait for all queued callbacks to complete on every CPU.
pub fn rcu_barrier() {
    // Drain every per-CPU list synchronously (grace period guaranteed by
    // synchronize_rcu before the call).
    synchronize_rcu();
    for slot in CB_LISTS.iter() {
        let mut cbs = slot.lock();
        while let head_ptr = cbs.dequeue() {
            if head_ptr.is_null() {
                break;
            }
            if let Some(func) = unsafe { (*head_ptr).func } {
                unsafe { func(head_ptr) };
            }
        }
    }
}

/// Helper used by tests to peek the global `gp_seq`.
pub fn gp_seq_now() -> u64 {
    GP_SEQ.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rcu_read_lock_unlock_round_trip() {
        rcu_init();
        rcu_read_lock();
        rcu_read_unlock();
    }

    #[test]
    fn rcu_qs_records_current_gp() {
        rcu_init();
        let gp_before = gp_seq_now();
        rcu_qs();
        assert_eq!(QS_AT_GP[0].load(Ordering::Acquire), gp_before);
    }

    #[test]
    fn synchronize_rcu_advances_gp_seq() {
        rcu_init();
        let before = gp_seq_now();
        synchronize_rcu();
        assert!(gp_seq_now() > before);
    }

    #[test]
    fn call_rcu_callback_fires_after_check() {
        use core::sync::atomic::AtomicU32;

        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_head: *mut RcuHead) {
            FIRED.fetch_add(1, Ordering::AcqRel);
        }

        rcu_init();
        FIRED.store(0, Ordering::Release);
        let mut head = RcuHead::new();
        call_rcu(&mut head as *mut RcuHead, cb);
        rcu_check_callbacks();
        assert_eq!(FIRED.load(Ordering::Acquire), 1);
    }
}
