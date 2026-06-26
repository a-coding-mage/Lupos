//! linux-parity: partial
//! linux-source: vendor/linux/kernel/softirq.c
//! test-origin: linux:vendor/linux/kernel/softirq.c
//! Softirq / tasklet — deferred kernel work (Milestone 6).
//!
//! Real softirq/tasklet layer (NR_SOFTIRQS=10, pending mask, raise/do_softirq,
//! tasklet queue). The documented "Lupos divergences from Linux" below are the
//! remaining work for `complete`: per-CPU `__softirq_pending`/`tasklet_vec`,
//! `__do_softirq` IRQ-enable + ksoftirqd, and preempt-counter reentrancy.
//!
//! Modeled on Linux's softirq layer.  A *softirq* is a low-overhead deferred
//! handler whose number is fixed at compile time (Linux defines 10 of them).
//! Each CPU has a per-CPU pending mask; when a softirq is "raised", its bit is
//! set in the mask, and the next time the CPU runs `do_softirq()` (typically
//! after returning from a hardware interrupt or in the idle loop) the
//! corresponding handler is invoked.
//!
//! Tasklets are softirqs' lighter cousin: they are queued onto a per-CPU list
//! and dispatched by the dedicated `Tasklet` softirq.  This lets driver code
//! defer work without consuming one of the precious 10 softirq slots.
//!
//! # Lupos divergences from Linux
//!
//! 1. **Single global pending mask + tasklet list.**  Linux uses per-CPU
//!    storage (`__softirq_pending`, `tasklet_vec`).  We have no per-CPU
//!    infrastructure yet, so we share state across CPUs behind a single
//!    `AtomicU32` and `Mutex<VecDeque<…>>`.  Acceptable while only the BSP
//!    raises softirqs from its periodic tick.  TODO(M7+): per-CPU storage.
//!
//! 2. **`do_softirq()` is *not* called from inside an ISR.**  Linux's
//!    `__do_softirq` re-enables interrupts (`local_irq_enable`) before
//!    invoking handlers.  Doing that from within an interrupt-gate ISR is
//!    delicate — it requires masking the LAPIC, manually re-enabling IF, and
//!    restoring on return.  For Milestone 6 the timer ISR only *raises* the
//!    Timer softirq; the actual drain happens from task context in
//!    `schedule()` and from `halt_loop_with_softirq` in `main.rs`.  TODO(M7+):
//!    mirror `__do_softirq` once we have proper preempt counters and IRQ
//!    masking helpers.
//!
//! 3. **Reentrance guard is a single AtomicBool**, not Linux's preempt
//!    counter.  Sufficient because Lupos has no nested softirq dispatch yet.
//!
//! Linux references:
//!   kernel/softirq.c          — `__do_softirq`, `raise_softirq`, `tasklet_action`
//!   include/linux/interrupt.h — `softirq_action`, `NR_SOFTIRQS`, `tasklet_struct`

extern crate alloc;

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use alloc::collections::VecDeque;
use spin::Mutex;

// ── Softirq slot enumeration ──────────────────────────────────────────────────
//
// The slot ordering matches Linux `include/linux/interrupt.h` to make future
// porting work easier.  We do not currently use most of these — they exist as
// reserved slots so the indices are stable from day one.

/// Number of softirq slots (matches Linux NR_SOFTIRQS).
pub const NR_SOFTIRQS: usize = 10;

/// Softirq slot identifiers (mirrors Linux ordering, see `include/linux/interrupt.h`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftIrqVec {
    /// High-priority tasklets.
    Hi = 0,
    /// Timer-driven deferred work.
    Timer = 1,
    /// Network transmit completion.
    NetTx = 2,
    /// Network receive processing.
    NetRx = 3,
    /// Block I/O completion.
    Block = 4,
    /// Polled I/O.
    IrqPoll = 5,
    /// Standard tasklet dispatcher.
    Tasklet = 6,
    /// Scheduler load balancing.
    Sched = 7,
    /// High-resolution timers.
    Hrtimer = 8,
    /// RCU grace-period callbacks.
    Rcu = 9,
}

impl SoftIrqVec {
    /// Numeric slot index.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Pending-mask bit corresponding to this slot.
    #[inline]
    pub const fn bit(self) -> u32 {
        1u32 << (self as u32)
    }
}

// ── Softirq action table ──────────────────────────────────────────────────────

/// A registered softirq handler entry.
#[derive(Clone, Copy)]
pub struct SoftIrqAction {
    pub handler: Option<fn()>,
}

impl SoftIrqAction {
    pub const fn empty() -> Self {
        Self { handler: None }
    }
}

/// Global softirq action table — indexed by `SoftIrqVec::index()`.
///
/// Wrapped in a Mutex even though writes only happen at boot time, because
/// `do_softirq` reads it from interrupt context (or the idle loop) and we
/// want a clean lock-based protocol rather than `unsafe` static muts.
static SOFTIRQ_VEC: Mutex<[SoftIrqAction; NR_SOFTIRQS]> =
    Mutex::new([SoftIrqAction::empty(); NR_SOFTIRQS]);

/// Pending mask — bit `i` set iff `SoftIrqVec(i)` has work to do.
///
/// TODO(M7+): per-CPU storage (Linux `__softirq_pending`).
static PENDING: AtomicU32 = AtomicU32::new(0);

/// Reentrance guard — set while `do_softirq` is running on the current CPU.
///
/// Prevents `do_softirq` recursion if a handler accidentally re-enters it.
/// TODO(M7+): replace with proper preempt counter (`include/linux/preempt.h`).
static IN_SOFTIRQ: AtomicBool = AtomicBool::new(false);

// ── Public registration / raise / drain ──────────────────────────────────────

/// Initialize the softirq subsystem.
pub fn init() {
    open_softirq(
        SoftIrqVec::Timer,
        crate::kernel::time::clockevents::tick_handle_periodic,
    );
    open_softirq(SoftIrqVec::Tasklet, tasklet_action);
}

/// Install a handler for the given softirq slot.
///
/// Panics if a handler is already installed for `nr` to catch double
/// registration during boot wiring (Linux silently overwrites; we are stricter).
pub fn open_softirq(nr: SoftIrqVec, handler: fn()) {
    let mut table = SOFTIRQ_VEC.lock();
    let slot = &mut table[nr.index()];
    assert!(
        slot.handler.is_none(),
        "softirq slot {:?} already has a handler installed",
        nr
    );
    slot.handler = Some(handler);
}

/// Mark the given softirq slot as pending.
///
/// Cheap and lock-free — just sets a bit in the global mask.  The actual
/// handler runs from the next `do_softirq()` call.
#[inline]
pub fn raise_softirq(nr: SoftIrqVec) {
    PENDING.fetch_or(nr.bit(), Ordering::Release);
}

/// True iff a softirq drain is currently in progress on this CPU.
///
/// Used both as the reentrance guard and for diagnostic logging.  Linux's
/// `in_interrupt()` is broader (it includes hardirq + NMI context); for
/// Milestone 6 only the softirq bit matters.
#[inline]
pub fn in_interrupt() -> bool {
    IN_SOFTIRQ.load(Ordering::Acquire)
}

/// Drain the pending mask, invoking each registered handler exactly once.
///
/// **Must NOT be called from a hard IRQ context** (interrupt-gate handler).
/// In Lupos M6 this is called from the BSP idle loop only — see
/// `main.rs::halt_loop_with_softirq`.
///
/// The reentrance guard prevents recursive entry: if a handler raises a new
/// softirq, the bit will be picked up by the *outer* loop iteration here.
pub fn do_softirq() {
    // Acquire the reentrance guard.  If already set, bail — the outer caller
    // will pick up any new pending bits on its next iteration.
    if IN_SOFTIRQ
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    // Drain in a loop so handlers that raise additional softirqs (e.g. a
    // tasklet that schedules another tasklet) are processed in the same call.
    loop {
        let pending = PENDING.swap(0, Ordering::AcqRel);
        if pending == 0 {
            break;
        }

        for i in 0..NR_SOFTIRQS {
            let bit = 1u32 << i;
            if pending & bit == 0 {
                continue;
            }
            // Take a snapshot of the handler so we don't hold the lock across
            // the call (a handler must be free to register / drain tasklets).
            let handler = {
                let table = SOFTIRQ_VEC.lock();
                table[i].handler
            };
            if let Some(h) = handler {
                h();
            }
        }
    }

    IN_SOFTIRQ.store(false, Ordering::Release);
}

// ── Tasklets ──────────────────────────────────────────────────────────────────
//
// A tasklet is a one-off deferred function with an associated `data` argument.
// Drivers schedule tasklets without burning a softirq slot; the dedicated
// `Tasklet` softirq drains the queue.
//
// Tasklet state machine (mirrors Linux `TASKLET_STATE_SCHED`):
//   scheduled = false → idle, may be scheduled
//   scheduled = true  → already on the queue; subsequent schedules are no-ops

/// A deferred work item dispatched by the `Tasklet` softirq.
pub struct Tasklet {
    pub func: fn(u64),
    pub data: u64,
    pub scheduled: AtomicBool,
}

impl Tasklet {
    /// Create a new (idle) tasklet.
    pub const fn new(func: fn(u64), data: u64) -> Self {
        Self {
            func,
            data,
            scheduled: AtomicBool::new(false),
        }
    }
}

/// Queue of pending tasklets.
///
/// TODO(M7+): per-CPU `tasklet_vec` (Linux `kernel/softirq.c`).
static TASKLET_LIST: Mutex<VecDeque<&'static Tasklet>> = Mutex::new(VecDeque::new());

/// Schedule `t` for execution by the next `Tasklet` softirq dispatch.
///
/// Idempotent: a tasklet that is already scheduled is left alone.  Returns
/// `true` if this call actually queued the tasklet, `false` if it was already
/// in flight.
pub fn tasklet_schedule(t: &'static Tasklet) -> bool {
    if t.scheduled
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    TASKLET_LIST.lock().push_back(t);
    raise_softirq(SoftIrqVec::Tasklet);
    true
}

/// `Tasklet` softirq handler — drains the global tasklet queue.
fn tasklet_action() {
    loop {
        let next = TASKLET_LIST.lock().pop_front();
        let Some(t) = next else { break };
        // Clear the scheduled flag *before* the handler runs so the handler
        // is free to re-schedule the same tasklet (matches Linux semantics).
        t.scheduled.store(false, Ordering::Release);
        (t.func)(t.data);
    }
}

// ── TDD: softirq boot test (Milestone 6) ──────────────────────────────────────

#[cfg(feature = "test-softirq")]
static SOFTIRQ_TEST_COUNT: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "test-softirq")]
fn softirq_test_handler() {
    SOFTIRQ_TEST_COUNT.fetch_add(1, Ordering::Release);
}

/// Boot-time TDD: verify that registering, raising, and draining a softirq
/// works end-to-end.
///
/// Pass criterion (asserted by the xtask harness):
///   - serial log contains `softirq: deferred work executed`
///   - QEMU exits with success code (0x21).
///
/// We use the `Hi` slot (not Timer) so we don't race with `apic_timer::on_tick`
/// raising the Timer softirq from the LAPIC interrupt.
#[cfg(feature = "test-softirq")]
pub fn run_softirq_test() {
    // 1. Register the test handler in the Hi slot (free for use here).
    open_softirq(SoftIrqVec::Hi, softirq_test_handler);

    // 2. Inline raise + drain — must execute the handler exactly once.
    raise_softirq(SoftIrqVec::Hi);
    do_softirq();

    let after_inline = SOFTIRQ_TEST_COUNT.load(Ordering::Acquire);
    assert!(
        after_inline >= 1,
        "softirq: inline raise/drain did not run handler (count={})",
        after_inline
    );

    // 3. Tasklet path: schedule a tasklet, drain via Tasklet softirq.
    static TEST_TASKLET: Tasklet = Tasklet::new(
        |_| {
            SOFTIRQ_TEST_COUNT.fetch_add(1, Ordering::Release);
        },
        0,
    );
    let queued = tasklet_schedule(&TEST_TASKLET);
    assert!(queued, "tasklet_schedule must queue an idle tasklet");
    do_softirq();

    let after_tasklet = SOFTIRQ_TEST_COUNT.load(Ordering::Acquire);
    assert!(
        after_tasklet >= 2,
        "softirq: tasklet drain did not run handler (count={})",
        after_tasklet
    );

    // 4. Banner — must match SOFTIRQ_BANNER in xtask/src/lib.rs.
    crate::kernel::printk::log_info!(
        "softirq",
        "softirq: deferred work executed (count={})",
        after_tasklet
    );

    #[cfg(feature = "qemu-test")]
    unsafe {
        crate::linux_driver_abi::platform::qemu::exit_success();
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::AtomicU32;
    // Serialize all tests that touch global softirq state (SOFTIRQ_VEC,
    // PENDING, IN_SOFTIRQ) so parallel test threads can't race on shared slots.
    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    /// Reset all global softirq state and hold the test mutex for the
    /// caller's scope.  The caller must bind the return value:
    ///   `let _guard = reset_state();`
    fn reset_state() -> spin::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock();
        PENDING.store(0, Ordering::SeqCst);
        IN_SOFTIRQ.store(false, Ordering::SeqCst);
        {
            let mut table = SOFTIRQ_VEC.lock();
            for slot in table.iter_mut() {
                slot.handler = None;
            }
        }
        TASKLET_LIST.lock().clear();
        guard
    }

    #[test]
    fn nr_softirqs_matches_linux_slot_count() {
        // Linux defines 10 softirq slots (HI, TIMER, NET_TX, NET_RX, BLOCK,
        // IRQ_POLL, TASKLET, SCHED, HRTIMER, RCU).  Keeping our count in sync
        // makes future Linux-parity work easier.
        assert_eq!(NR_SOFTIRQS, 10);
        assert_eq!(SoftIrqVec::Rcu.index() + 1, NR_SOFTIRQS);
    }

    #[test]
    fn softirq_bit_matches_index() {
        assert_eq!(SoftIrqVec::Hi.bit(), 1 << 0);
        assert_eq!(SoftIrqVec::Timer.bit(), 1 << 1);
        assert_eq!(SoftIrqVec::Tasklet.bit(), 1 << 6);
        assert_eq!(SoftIrqVec::Rcu.bit(), 1 << 9);
    }

    #[test]
    fn pending_mask_set_and_clear() {
        let _guard = reset_state();
        assert_eq!(PENDING.load(Ordering::SeqCst), 0);
        raise_softirq(SoftIrqVec::Timer);
        assert_eq!(PENDING.load(Ordering::SeqCst), SoftIrqVec::Timer.bit());
        // Drain via swap
        let drained = PENDING.swap(0, Ordering::SeqCst);
        assert_eq!(drained, SoftIrqVec::Timer.bit());
        assert_eq!(PENDING.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn raise_softirq_is_idempotent_on_same_bit() {
        let _guard = reset_state();
        raise_softirq(SoftIrqVec::Hi);
        raise_softirq(SoftIrqVec::Hi);
        // Two raises must yield a single bit, not "2".
        assert_eq!(PENDING.load(Ordering::SeqCst), SoftIrqVec::Hi.bit());
    }

    #[test]
    fn open_softirq_installs_handler() {
        let _guard = reset_state();
        fn h() {}
        open_softirq(SoftIrqVec::NetRx, h);
        let table = SOFTIRQ_VEC.lock();
        assert!(table[SoftIrqVec::NetRx.index()].handler.is_some());
    }

    static DRAIN_COUNT: AtomicU32 = AtomicU32::new(0);
    fn drain_handler() {
        DRAIN_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn do_softirq_drains_pending() {
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        open_softirq(SoftIrqVec::Hi, drain_handler);
        raise_softirq(SoftIrqVec::Hi);
        do_softirq();
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 1);
        // Pending must be cleared.
        assert_eq!(PENDING.load(Ordering::SeqCst), 0);
        // Guard must be released after drain.
        assert!(!IN_SOFTIRQ.load(Ordering::SeqCst));
    }

    #[test]
    fn do_softirq_respects_in_softirq_guard() {
        let _guard = reset_state();
        // Pretend we are already inside a softirq drain.
        IN_SOFTIRQ.store(true, Ordering::SeqCst);
        open_softirq(SoftIrqVec::Hi, drain_handler);
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        raise_softirq(SoftIrqVec::Hi);
        do_softirq();
        // Guard prevented entry → handler must NOT have run.
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 0);
        // Pending bit must still be set so the outer drain can pick it up.
        assert_eq!(PENDING.load(Ordering::SeqCst), SoftIrqVec::Hi.bit());
        // Reset guard for subsequent tests.
        IN_SOFTIRQ.store(false, Ordering::SeqCst);
    }

    #[test]
    fn tasklet_schedule_sets_tasklet_pending_bit() {
        let _guard = reset_state();
        static T: Tasklet = Tasklet::new(|_| {}, 0);
        // Manually clear `scheduled` because the static persists across tests.
        T.scheduled.store(false, Ordering::SeqCst);

        let queued = tasklet_schedule(&T);
        assert!(queued);
        // Tasklet softirq bit must be raised.
        assert_ne!(
            PENDING.load(Ordering::SeqCst) & SoftIrqVec::Tasklet.bit(),
            0
        );
        // Second schedule on the same tasklet must be a no-op.
        let queued_again = tasklet_schedule(&T);
        assert!(!queued_again);
        // Cleanup so other tests don't see leftover state.
        T.scheduled.store(false, Ordering::SeqCst);
        TASKLET_LIST.lock().clear();
    }
}
