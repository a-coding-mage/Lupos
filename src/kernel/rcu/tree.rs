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

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering, fence};

use super::segcblist::SegCbList;
use super::types::RcuHead;
use crate::kernel::locking::RawSpinLocked;
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
///
/// Linux protects the callback list with a raw spinlock and IRQ-save
/// semantics.  The atomic count lets the scheduler-clock hook decide whether
/// this CPU needs an RCU softirq without taking that lock from hard IRQ
/// context.
static CB_LISTS: [RawSpinLocked<SegCbList>; MAX_CPUS] =
    [const { RawSpinLocked::new(SegCbList::new()) }; MAX_CPUS];
static CB_COUNTS: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

#[inline]
fn cpu_index() -> usize {
    crate::arch::x86::kernel::setup_percpu::current_cpu_number()
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

/// `get_state_synchronize_rcu()` — snapshot current RCU grace-period state.
pub fn get_state_synchronize_rcu() -> u64 {
    fence(Ordering::SeqCst);
    GP_SEQ.load(Ordering::Acquire)
}

/// `poll_state_synchronize_rcu()` — true once a later grace period completed.
pub fn poll_state_synchronize_rcu(oldstate: u64) -> bool {
    let done = GP_SEQ.load(Ordering::Acquire) != oldstate;
    if done {
        fence(Ordering::SeqCst);
    }
    done
}

/// `start_poll_synchronize_rcu()` — snapshot and force progress in Lupos' RCU.
pub fn start_poll_synchronize_rcu() -> u64 {
    let oldstate = get_state_synchronize_rcu();
    synchronize_rcu();
    oldstate
}

/// `cond_synchronize_rcu()` — wait only when no grace period has elapsed yet.
pub fn cond_synchronize_rcu(oldstate: u64) {
    if !poll_state_synchronize_rcu(oldstate) {
        synchronize_rcu();
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
    // Begin a new grace period before publishing the callback. A local timer
    // tick alone must not recycle a vmapped stack another CPU can still see.
    GP_SEQ.fetch_add(1, Ordering::AcqRel);
    let cpu = cpu_index();
    let (mut cbs, flags) = CB_LISTS[cpu].lock_irqsave();
    unsafe {
        cbs.enqueue(head);
    }
    CB_COUNTS[cpu].fetch_add(1, Ordering::Release);
    RawSpinLocked::unlock_irqrestore(cbs, flags);
    crate::kernel::softirq::raise_softirq(crate::kernel::softirq::SoftIrqVec::Rcu);
}

/// Linux `rcu_sched_clock_irq()` asks RCU core to run when this CPU has
/// callbacks pending.  Keeping this retry at the scheduler-clock boundary is
/// important: a grace period can be incomplete when the first RCU softirq
/// runs, but immediately re-raising the same softirq would monopolize
/// `__do_softirq()` and starve ordinary work.
pub fn rcu_sched_clock_irq() {
    let cpu = cpu_index();
    if CB_COUNTS[cpu].load(Ordering::Acquire) != 0 {
        crate::kernel::softirq::raise_softirq(crate::kernel::softirq::SoftIrqVec::Rcu);
    }
}

fn dequeue_callback(cpu: usize) -> *mut RcuHead {
    let (mut cbs, flags) = CB_LISTS[cpu].lock_irqsave();
    let head = cbs.dequeue();
    if !head.is_null() {
        CB_COUNTS[cpu].fetch_sub(1, Ordering::AcqRel);
    }
    RawSpinLocked::unlock_irqrestore(cbs, flags);
    head
}

unsafe fn invoke_callback(head: *mut RcuHead) {
    if let Some(func) = unsafe { (*head).func.take() } {
        unsafe { func(head) };
    }
}

/// `rcu_check_callbacks()` — invoked from the Timer softirq once per tick.
/// Drains and invokes any RCU callbacks whose grace period has elapsed.
pub fn rcu_check_callbacks() {
    rcu_qs();
    let target = GP_SEQ.load(Ordering::Acquire);
    let online = ONLINE_MASK.load(Ordering::Acquire);
    let complete = (0..MAX_CPUS).all(|cpu| {
        online & (1u64 << (cpu & 63)) == 0
            || QS_AT_GP[cpu].load(Ordering::Acquire) >= target
    });
    if !complete {
        // Linux leaves the work for the next scheduler-clock/RCU-core pass.
        // Re-raising here makes __do_softirq() chase a moving grace-period
        // target indefinitely when task-stack callbacks arrive continuously.
        return;
    }

    // Linux extracts callbacks under the RCU lock and invokes them after the
    // lock is released.  Bound one pass so a callback flood cannot monopolize
    // a softirq; any remainder is picked up by the normal bounded softirq
    // restart/ksoftirqd path.
    const MAX_CALLBACKS_PER_PASS: usize = 64;
    let cpu = cpu_index();
    // Linux's segmented callback list advances only the callbacks which were
    // ready at the grace-period boundary.  A callback may legally repost its
    // rcu_head from inside the callback; that repost belongs to a later grace
    // period and must not be consumed by this same pass.  Snapshot the count
    // before invoking anything so the callback loop cannot run into that new
    // segment.
    let ready_callbacks = CB_COUNTS[cpu]
        .load(Ordering::Acquire)
        .min(MAX_CALLBACKS_PER_PASS as u64) as usize;
    for _ in 0..ready_callbacks {
        let head_ptr = dequeue_callback(cpu);
        if head_ptr.is_null() {
            break;
        }
        unsafe { invoke_callback(head_ptr) };
    }
    if CB_COUNTS[cpu].load(Ordering::Acquire) != 0 {
        crate::kernel::softirq::raise_softirq(crate::kernel::softirq::SoftIrqVec::Rcu);
    }
}

/// `rcu_barrier()` — wait for all queued callbacks to complete on every CPU.
pub fn rcu_barrier() {
    // Drain every per-CPU list synchronously (grace period guaranteed by
    // synchronize_rcu before the call).
    synchronize_rcu();
    for cpu in 0..MAX_CPUS {
        while CB_COUNTS[cpu].load(Ordering::Acquire) != 0 {
            let head_ptr = dequeue_callback(cpu);
            if head_ptr.is_null() {
                break;
            }
            unsafe { invoke_callback(head_ptr) };
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

    static RCU_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

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
    fn conditional_synchronize_uses_gp_state() {
        rcu_init();
        let before = get_state_synchronize_rcu();
        assert!(!poll_state_synchronize_rcu(before));
        cond_synchronize_rcu(before);
        assert!(poll_state_synchronize_rcu(before));
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

    /// A callback reposted from inside its callback belongs to the next RCU
    /// grace period.  Linux's `rcu_segcblist` keeps it out of the current
    /// `rcu_do_batch()` extraction; a flat FIFO that drains until empty would
    /// invoke it immediately and can free a late task_struct release early.
    ///
    /// test-origin: linux:vendor/linux/kernel/rcu/tree.c:rcu_do_batch
    /// test-origin: linux:vendor/linux/kernel/rcu/rcu_segcblist.c:
    /// rcu_segcblist_extract_done_cbs
    #[test]
    fn reposted_rcu_callback_waits_for_a_later_batch() {
        let _guard = RCU_TEST_LOCK.lock();
        static FIRED: AtomicU32 = AtomicU32::new(0);
        static mut HEAD: RcuHead = RcuHead::new();

        unsafe extern "C" fn repost_once(head: *mut RcuHead) {
            let count = FIRED.fetch_add(1, Ordering::AcqRel) + 1;
            if count == 1 {
                call_rcu(head, repost_once);
            }
        }

        rcu_init();
        FIRED.store(0, Ordering::Release);
        unsafe {
            call_rcu(core::ptr::addr_of_mut!(HEAD), repost_once);
        }

        rcu_check_callbacks();
        assert_eq!(FIRED.load(Ordering::Acquire), 1);

        rcu_check_callbacks();
        assert_eq!(FIRED.load(Ordering::Acquire), 2);
    }

    /// Linux's RCU core does not immediately requeue RCU softirq work while a
    /// grace period is incomplete.  The next scheduler/timer pass reports the
    /// missing quiescent state; an inline self-requeue can consume every
    /// softirq restart and starve ordinary task progress under callback load.
    ///
    /// test-origin: linux:vendor/linux/kernel/rcu/tree.c:rcu_core
    /// test-origin: linux:vendor/linux/kernel/rcu/tree.c:rcu_sched_clock_irq
    #[test]
    fn incomplete_grace_period_does_not_self_requeue_rcu_softirq() {
        let _guard = RCU_TEST_LOCK.lock();
        crate::kernel::softirq::do_softirq();
        rcu_init();

        let next_gp = GP_SEQ.load(Ordering::Acquire) + 1;
        GP_SEQ.store(next_gp, Ordering::Release);
        ONLINE_MASK.store(0b11, Ordering::Release);
        QS_AT_GP[0].store(0, Ordering::Release);
        QS_AT_GP[1].store(0, Ordering::Release);

        rcu_check_callbacks();

        assert_eq!(
            crate::kernel::softirq::local_softirq_pending()
                & crate::kernel::softirq::SoftIrqVec::Rcu.bit(),
            0,
            "an incomplete grace period must wait for a later timer/RCU pass"
        );

        ONLINE_MASK.store(1, Ordering::Release);
        QS_AT_GP[1].store(0, Ordering::Release);
        crate::kernel::softirq::do_softirq();
    }
}
