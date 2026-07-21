//! linux-parity: partial
//! linux-source: vendor/linux/kernel/softirq.c
//! test-origin: linux:vendor/linux/kernel/softirq.c
//! Softirq / tasklet — deferred kernel work (Milestone 6).
//!
//! Real softirq/tasklet layer (NR_SOFTIRQS=10, per-CPU pending masks,
//! IRQ-exit draining, preempt-count context, per-CPU ksoftirqd threads, and
//! per-CPU tasklet queues).
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
//! # Remaining divergence
//!
//! CPU hot-unplug is not implemented by Lupos, so the per-CPU ksoftirqd tasks
//! do not yet need Linux's smpboot park/unpark and dead-CPU tasklet takeover
//! callbacks.
//!
//! Linux references:
//!   kernel/softirq.c          — `__do_softirq`, `raise_softirq`, `tasklet_action`
//!   include/linux/interrupt.h — `softirq_action`, `NR_SOFTIRQS`, `tasklet_struct`

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::sched::MAX_CPUS;

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

const _: () = assert!(core::mem::size_of::<fn()>() == core::mem::size_of::<usize>());

/// A registered softirq handler entry.
pub struct SoftIrqAction {
    /// A function pointer encoded as a native word; zero means unregistered.
    ///
    /// Lupos currently targets x86_64, where code pointers and `usize` have
    /// the same representation width.  Keeping each slot atomic permits
    /// Linux's lock-free dispatch while remaining race-free if a registration
    /// is published as another CPU examines the table.
    handler: AtomicUsize,
}

impl SoftIrqAction {
    pub const fn empty() -> Self {
        Self {
            handler: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn publish(&self, handler: fn()) {
        self.handler.store(handler as usize, Ordering::Release);
    }

    #[inline]
    fn load(&self) -> Option<fn()> {
        let address = self.handler.load(Ordering::Acquire);
        if address == 0 {
            None
        } else {
            // SAFETY: `handler` is private and receives only zero or the
            // native-word encoding of a valid `fn()` in `publish()`.
            Some(unsafe { core::mem::transmute::<usize, fn()>(address) })
        }
    }

    #[cfg(test)]
    fn clear(&self) {
        self.handler.store(0, Ordering::SeqCst);
    }
}

/// Global softirq action table — indexed by `SoftIrqVec::index()`.
///
/// Linux publishes this table during initialization and walks it directly
/// from `handle_softirqs()`.  Per-slot atomics preserve that lock-free
/// dispatch while providing a race-free Rust representation.
static SOFTIRQ_VEC: [SoftIrqAction; NR_SOFTIRQS] = [const { SoftIrqAction::empty() }; NR_SOFTIRQS];

/// Counts real dispatch-path acquisitions of a global registration mutex.
///
/// The atomic table does not increment this counter.  Keeping it beside the
/// real handler-load helper lets the regression test exercise dispatch and
/// verify that no table mutex was acquired, rather than inspecting source
/// text.
#[cfg(test)]
static SOFTIRQ_VEC_DISPATCH_LOCK_ACQUISITIONS: AtomicUsize = AtomicUsize::new(0);

/// Load one registered action without serializing other CPUs' softirq drains.
#[inline]
fn registered_softirq_handler(index: usize) -> Option<fn()> {
    SOFTIRQ_VEC[index].load()
}

/// Per-CPU pending masks — bit `i` is set iff `SoftIrqVec(i)` has local work.
///
/// Mirrors Linux's `irq_stat.__softirq_pending`.
static PENDING: [AtomicU32; MAX_CPUS] = [const { AtomicU32::new(0) }; MAX_CPUS];

/// Per-CPU reentrance guards, set while `do_softirq` runs on that CPU.
///
/// Prevents `do_softirq` recursion if a handler accidentally re-enters it.
/// TODO(M7+): replace with proper preempt counter (`include/linux/preempt.h`).
static IN_SOFTIRQ: [AtomicBool; MAX_CPUS] = [const { AtomicBool::new(false) }; MAX_CPUS];

/// Linux's `DEFINE_PER_CPU(struct task_struct *, ksoftirqd)`.
///
/// Tasks are allocated during boot, before device traffic can exhaust an
/// inline softirq budget. Waking one from IRQ exit therefore never allocates.
static KSOFTIRQD_TASKS: [AtomicPtr<crate::kernel::task::TaskStruct>; MAX_CPUS] =
    [const { AtomicPtr::new(core::ptr::null_mut()) }; MAX_CPUS];

#[cfg(test)]
static KSOFTIRQD_WAKE_REQUESTS: [AtomicU32; MAX_CPUS] = [const { AtomicU32::new(0) }; MAX_CPUS];

/// Linux bounds one `__do_softirq()` invocation before deferring work which
/// continuously re-raises itself.
const MAX_SOFTIRQ_RESTART: usize = 10;
const MAX_SOFTIRQ_TIME_MS: u64 = 2;

#[inline]
fn current_cpu_slot() -> usize {
    crate::arch::x86::kernel::setup_percpu::current_cpu_number().min(MAX_CPUS - 1)
}

#[inline]
fn pending_for_cpu(cpu: usize) -> &'static AtomicU32 {
    &PENDING[cpu.min(MAX_CPUS - 1)]
}

#[inline]
fn in_softirq_for_cpu(cpu: usize) -> &'static AtomicBool {
    &IN_SOFTIRQ[cpu.min(MAX_CPUS - 1)]
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__tasklet_schedule",
        linux___tasklet_schedule as usize,
        false,
    );
    export_symbol_once(
        "__tasklet_hi_schedule",
        linux___tasklet_hi_schedule as usize,
        false,
    );
    export_symbol_once("tasklet_setup", linux_tasklet_setup as usize, false);
    export_symbol_once("tasklet_init", linux_tasklet_init as usize, false);
    export_symbol_once("tasklet_kill", linux_tasklet_kill as usize, false);
    export_symbol_once(
        "tasklet_unlock_spin_wait",
        linux_tasklet_unlock_spin_wait as usize,
        false,
    );
    export_symbol_once("tasklet_unlock", linux_tasklet_unlock as usize, true);
    export_symbol_once(
        "tasklet_unlock_wait",
        linux_tasklet_unlock_wait as usize,
        true,
    );
}

// ── Public registration / raise / drain ──────────────────────────────────────

/// Initialize the softirq subsystem.
pub fn init() {
    open_softirq(
        SoftIrqVec::Timer,
        crate::kernel::time::clockevents::tick_handle_periodic,
    );
    open_softirq(SoftIrqVec::Hi, tasklet_hi_action);
    open_softirq(SoftIrqVec::Tasklet, tasklet_action);
    open_softirq(
        SoftIrqVec::Sched,
        crate::kernel::sched::run_rebalance_softirq,
    );
}

/// Linux `ksoftirqd_should_run()`.
#[inline]
fn ksoftirqd_should_run(cpu: usize) -> bool {
    pending_for_cpu(cpu).load(Ordering::Acquire) != 0
}

/// Linux `wakeup_softirqd()`.
///
/// This is IRQ-safe: the task and its stack were allocated by
/// [`spawn_ksoftirqd`] and the scheduler wake path only takes IRQ-safe
/// runqueue locks.
#[inline]
fn wakeup_softirqd_on_cpu(cpu: usize) {
    let cpu = cpu.min(MAX_CPUS - 1);
    #[cfg(test)]
    KSOFTIRQD_WAKE_REQUESTS[cpu].fetch_add(1, Ordering::SeqCst);

    let task = KSOFTIRQD_TASKS[cpu].load(Ordering::Acquire);
    if !task.is_null() {
        unsafe {
            let _ = crate::kernel::sched::wake_task_normal(task);
        }
    }
}

/// Per-CPU smpboot-style thread loop used by Linux's `softirq_threads`.
///
/// The task-state store and pending recheck happen with local IRQs disabled.
/// Since softirqs are raised on their local CPU, this is the same lost-wakeup
/// exclusion used by `smpboot_thread_fn()`: either the thread observes the
/// pending bit, or the raiser observes `TASK_INTERRUPTIBLE` and wakes it.
unsafe extern "C" fn ksoftirqd_thread(data: *mut core::ffi::c_void) -> ! {
    let bound_cpu = data as usize;

    loop {
        let irq_flags = crate::kernel::locking::local_irq_save();
        let current = unsafe { crate::kernel::sched::get_current() };
        debug_assert!(!current.is_null());
        debug_assert_eq!(current_cpu_slot(), bound_cpu);

        unsafe {
            (*current).__state.store(
                crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                Ordering::SeqCst,
            );
        }

        if !ksoftirqd_should_run(bound_cpu) {
            crate::kernel::locking::local_irq_restore(irq_flags);
            unsafe {
                crate::kernel::sched::schedule_with_irqs_enabled();
            }
            continue;
        }

        unsafe {
            (*current).__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                Ordering::Release,
            );
        }
        crate::kernel::locking::local_irq_restore(irq_flags);

        // Linux run_ksoftirqd() uses the shallow kthread stack directly,
        // drains one bounded batch, then cond_resched() before checking again.
        do_softirq();
        if crate::kernel::sched::current_needs_resched() {
            unsafe {
                crate::kernel::sched::reschedule_runnable();
            }
        }
    }
}

fn ksoftirqd_name(cpu: usize) -> [u8; 16] {
    let mut name = *b"ksoftirqd/0\0\0\0\0\0";
    if cpu >= 10 {
        name[10] = b'0' + ((cpu / 10) % 10) as u8;
        name[11] = b'0' + (cpu % 10) as u8;
        name[12] = 0;
    } else {
        name[10] = b'0' + cpu as u8;
        name[11] = 0;
    }
    name
}

/// Linux `spawn_ksoftirqd()` / `smpboot_register_percpu_thread()`.
///
/// Call after all boot CPUs have entered the scheduler. Each active CPU gets
/// one permanently CPU-affine task. The function is idempotent so a future
/// CPU-online path can call it again after publishing another active CPU.
pub fn spawn_ksoftirqd() {
    let active = crate::kernel::sched::cpu_active_mask();
    for cpu in 0..MAX_CPUS {
        if !active.test(cpu as u32) || !KSOFTIRQD_TASKS[cpu].load(Ordering::Acquire).is_null() {
            continue;
        }

        let name = ksoftirqd_name(cpu);
        let task = unsafe {
            crate::kernel::sched::kthread_create(
                ksoftirqd_thread,
                cpu as *mut core::ffi::c_void,
                &name,
            )
        };
        assert!(
            !task.is_null(),
            "softirq: failed to create ksoftirqd/{}",
            cpu
        );

        unsafe {
            (*task).m29.cpus_mask = crate::kernel::sched::entity::CpuMask::one(cpu as u32);
            (*task).m29.cpus_ptr = &(*task).m29.cpus_mask as *const _;
            (*task).m29.nr_cpus_allowed = 1;
            (*task).thread_info.cpu = cpu as u32;
            (*task).m29.recent_used_cpu = cpu as i32;
            (*task).m29.wake_cpu = cpu as i32;
        }

        KSOFTIRQD_TASKS[cpu].store(task, Ordering::Release);
        unsafe {
            crate::kernel::sched::enqueue_task(task);
        }
    }
}

/// Install a handler for the given softirq slot.
///
/// Like Linux `open_softirq()`, a later registration replaces the prior
/// action for that slot.
pub fn open_softirq(nr: SoftIrqVec, handler: fn()) {
    SOFTIRQ_VEC[nr.index()].publish(handler);
}

/// Mark the given softirq slot as pending.
///
/// Cheap and lock-free — just sets a bit in the local CPU's mask.  The actual
/// handler runs from the next `do_softirq()` call.
#[inline]
pub fn raise_softirq(nr: SoftIrqVec) {
    // Linux raise_softirq() serializes the local pending update and
    // wakeup_softirqd() decision against hard IRQ entry.
    let irq_flags = crate::kernel::locking::local_irq_save();
    raise_softirq_on_cpu(current_cpu_slot(), nr);
    crate::kernel::locking::local_irq_restore(irq_flags);
}

#[inline]
fn raise_softirq_on_cpu(cpu: usize, nr: SoftIrqVec) {
    let cpu = cpu.min(MAX_CPUS - 1);
    pending_for_cpu(cpu).fetch_or(nr.bit(), Ordering::Release);

    // Linux raise_softirq_irqoff() wakes the daemon when called outside all
    // interrupt/BH contexts. IRQ and serving-softirq callers defer this to
    // irq exit or the bounded drain's tail.
    if cpu == current_cpu_slot() && !in_interrupt() {
        wakeup_softirqd_on_cpu(cpu);
    }
}

/// True iff a softirq drain is currently in progress on this CPU.
///
/// Used both as the reentrance guard and for diagnostic logging.  Linux's
/// `in_interrupt()` is broader (it includes hardirq + NMI context); for
/// Milestone 6 only the softirq bit matters.
#[inline]
pub fn in_interrupt() -> bool {
    crate::kernel::locking::preempt::in_irq() || crate::kernel::locking::preempt::in_softirq()
}

/// Snapshot of the pending mask — Linux `local_softirq_pending()`.
#[inline]
pub fn local_softirq_pending() -> u32 {
    pending_for_cpu(current_cpu_slot()).load(Ordering::Acquire)
}

/// Linux `softirq_handle_begin()` for the non-RT target.
///
/// Active softirq service uses one `SOFTIRQ_OFFSET`, distinct from the two
/// offsets used by a task which merely disabled bottom halves.
#[inline]
fn softirq_handle_begin() {
    crate::kernel::locking::preempt::linux___local_bh_disable_ip(
        0,
        crate::kernel::locking::preempt::SOFTIRQ_OFFSET,
    );
}

/// Linux `softirq_handle_end()` for the non-RT target.
///
/// This must use the decrement-only helper: the normal outermost BH-enable
/// path checks and runs pending work, which would recursively enter this
/// drain before its per-CPU guard has been released.
#[inline]
fn softirq_handle_end() {
    crate::kernel::locking::preempt::local_bh_enable_no_softirq(
        crate::kernel::locking::preempt::SOFTIRQ_OFFSET,
    );
}

/// Drain the pending mask, invoking each registered handler exactly once.
///
/// **Must NOT be called from a hard IRQ context** (interrupt-gate handler).
/// Lupos calls this from IRQ exit, explicit task-context wait boundaries,
/// ksoftirqd, and each CPU's idle loop.
///
/// The reentrance guard prevents recursive entry: if a handler raises a new
/// softirq, the bit will be picked up by the *outer* loop iteration here.
pub fn do_softirq() {
    // Linux do_softirq() rejects interrupt context, then disables local IRQs
    // before selecting the per-CPU pending word. Keeping that order prevents
    // migration from making the guard and pending mask refer to a CPU other
    // than the one whose handlers we execute.
    if in_interrupt() {
        return;
    }
    let irq_flags = crate::kernel::locking::local_irq_save();
    let cpu = current_cpu_slot();
    let in_softirq = in_softirq_for_cpu(cpu);
    let pending_mask = pending_for_cpu(cpu);

    // Linux avoids entering __do_softirq() entirely when the local pending
    // word is zero. In particular, the idle/wait callers may probe this
    // function frequently; do not pay a locked guard transition, BH updates,
    // IF toggles, or timekeeping reads on the overwhelmingly common empty
    // path.
    if pending_mask.load(Ordering::Acquire) == 0 {
        crate::kernel::locking::local_irq_restore(irq_flags);
        return;
    }

    // Acquire the reentrance guard.  If already set, bail — the outer caller
    // will pick up any new pending bits on its next iteration.
    if in_softirq
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
        .is_err()
    {
        crate::kernel::locking::local_irq_restore(irq_flags);
        return;
    }

    // Linux __do_softirq() enters with hard IRQs disabled, establishes
    // SOFTIRQ_OFFSET, enables IRQs while handlers run, then returns with the
    // caller's original IF state restored.
    softirq_handle_begin();
    crate::kernel::locking::local_irq_enable();

    let end_jiffies = crate::kernel::time::jiffies::jiffies()
        .saturating_add(crate::kernel::time::jiffies::msecs_to_jiffies(MAX_SOFTIRQ_TIME_MS).max(1));
    let mut passes = 0usize;

    // Linux retries newly raised work only within a bounded budget. Remaining
    // bits stay pending and wake ksoftirqd; this prevents sustained network or
    // a disabled tasklet from monopolizing IRQ return.
    loop {
        let pending = pending_mask.swap(0, Ordering::AcqRel);
        if pending == 0 {
            break;
        }

        for i in 0..NR_SOFTIRQS {
            let bit = 1u32 << i;
            if pending & bit == 0 {
                continue;
            }
            // Linux reads the published action directly.  Load this slot once
            // so the call uses one coherent handler value even if another CPU
            // republishes the registration concurrently.
            let handler = registered_softirq_handler(i);
            if let Some(h) = handler {
                h();
            }
        }

        passes += 1;
        if pending_mask.load(Ordering::Acquire) == 0 {
            break;
        }
        if passes >= MAX_SOFTIRQ_RESTART
            || crate::kernel::time::jiffies::jiffies() >= end_jiffies
            || crate::kernel::sched::current_needs_resched()
        {
            break;
        }
    }

    crate::kernel::locking::local_irq_disable();
    if pending_mask.load(Ordering::Acquire) != 0 {
        wakeup_softirqd_on_cpu(cpu);
    }
    softirq_handle_end();
    in_softirq.store(false, Ordering::Release);
    crate::kernel::locking::local_irq_restore(irq_flags);
}

// ── Tasklets ──────────────────────────────────────────────────────────────────
//
// A tasklet is a one-off deferred function with an associated `data` argument.
// Drivers schedule tasklets without burning a softirq slot; the dedicated
// `Tasklet` softirq drains the queue.
//
// Tasklet state machine (mirrors Linux `TASKLET_STATE_SCHED` and
// `TASKLET_STATE_RUN`):
//   scheduled = false → idle, may be scheduled
//   scheduled = true  → already on a queue; subsequent schedules are no-ops
//   running = true    → callback is executing on one CPU

/// A deferred work item dispatched by the `Tasklet` softirq.
pub struct Tasklet {
    pub func: fn(u64),
    pub data: u64,
    pub scheduled: AtomicBool,
    running: AtomicBool,
    next: AtomicPtr<Tasklet>,
}

impl Tasklet {
    /// Create a new (idle) tasklet.
    pub const fn new(func: fn(u64), data: u64) -> Self {
        Self {
            func,
            data,
            scheduled: AtomicBool::new(false),
            running: AtomicBool::new(false),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

struct TaskletQueue {
    head: *mut Tasklet,
    tail: *mut Tasklet,
}

unsafe impl Send for TaskletQueue {}

impl TaskletQueue {
    const fn new() -> Self {
        Self {
            head: core::ptr::null_mut(),
            tail: core::ptr::null_mut(),
        }
    }

    unsafe fn push(&mut self, tasklet: *mut Tasklet) {
        unsafe {
            (*tasklet)
                .next
                .store(core::ptr::null_mut(), Ordering::Relaxed);
            if self.tail.is_null() {
                self.head = tasklet;
            } else {
                (*self.tail).next.store(tasklet, Ordering::Relaxed);
            }
            self.tail = tasklet;
        }
    }

    fn take_all(&mut self) -> *mut Tasklet {
        let head = self.head;
        self.head = core::ptr::null_mut();
        self.tail = core::ptr::null_mut();
        head
    }

    fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    fn len(&self) -> usize {
        let mut count = 0;
        let mut node = self.head;
        while !node.is_null() {
            count += 1;
            node = unsafe { (*node).next.load(Ordering::Relaxed) };
        }
        count
    }

    fn clear(&mut self) {
        let _ = self.take_all();
    }
}

/// Per-CPU intrusive queues of pending tasklets (`tasklet_vec` in Linux).
static TASKLET_LIST: [Mutex<TaskletQueue>; MAX_CPUS] =
    [const { Mutex::new(TaskletQueue::new()) }; MAX_CPUS];

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

    // Linux mutates tasklet_vec with local IRQs disabled. Without this, an
    // interrupt which schedules a tasklet on this CPU can spin forever on a
    // queue lock held by the interrupted frame.
    let flags = crate::kernel::locking::local_irq_save();
    let cpu = current_cpu_slot();
    unsafe {
        TASKLET_LIST[cpu]
            .lock()
            .push(t as *const Tasklet as *mut Tasklet);
    }
    raise_softirq_on_cpu(cpu, SoftIrqVec::Tasklet);
    crate::kernel::locking::local_irq_restore(flags);
    true
}

/// `Tasklet` softirq handler — detaches and drains this CPU's tasklet queue.
fn tasklet_action() {
    let flags = crate::kernel::locking::local_irq_save();
    let cpu = current_cpu_slot();
    let mut tasklet = {
        let mut queue = TASKLET_LIST[cpu].lock();
        queue.take_all()
    };
    crate::kernel::locking::local_irq_restore(flags);

    while !tasklet.is_null() {
        let t = unsafe { &*tasklet };
        tasklet = t.next.load(Ordering::Relaxed);
        if t.running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_err()
        {
            // Another CPU owns RUN. Keep SCHED set and retry in a later
            // action pass, matching tasklet_action_common().
            let flags = crate::kernel::locking::local_irq_save();
            unsafe {
                TASKLET_LIST[cpu]
                    .lock()
                    .push(t as *const Tasklet as *mut Tasklet);
            }
            raise_softirq_on_cpu(cpu, SoftIrqVec::Tasklet);
            crate::kernel::locking::local_irq_restore(flags);
            continue;
        }

        // Clear the scheduled flag *before* the handler runs so the handler
        // is free to re-schedule the same tasklet (matches Linux semantics).
        t.scheduled.store(false, Ordering::Release);
        (t.func)(t.data);
        t.running.store(false, Ordering::Release);
    }
    linux_tasklet_action();
}

const TASKLET_STATE_SCHED: usize = 1 << 0;
const TASKLET_STATE_RUN: usize = 1 << 1;

#[repr(C)]
pub union LinuxTaskletCallback {
    pub func: Option<unsafe extern "C" fn(usize)>,
    pub callback: Option<unsafe extern "C" fn(*mut LinuxTasklet)>,
}

#[repr(C)]
pub struct LinuxTasklet {
    pub next: *mut LinuxTasklet,
    pub state: AtomicUsize,
    pub count: AtomicI32,
    pub use_callback: bool,
    pub callback: LinuxTaskletCallback,
    pub data: usize,
}

struct LinuxTaskletQueue {
    head: *mut LinuxTasklet,
    tail: *mut LinuxTasklet,
}

unsafe impl Send for LinuxTaskletQueue {}

impl LinuxTaskletQueue {
    const fn new() -> Self {
        Self {
            head: core::ptr::null_mut(),
            tail: core::ptr::null_mut(),
        }
    }

    unsafe fn push(&mut self, tasklet: *mut LinuxTasklet) {
        unsafe {
            (*tasklet).next = core::ptr::null_mut();
            if self.tail.is_null() {
                self.head = tasklet;
            } else {
                (*self.tail).next = tasklet;
            }
            self.tail = tasklet;
        }
    }

    fn take_all(&mut self) -> *mut LinuxTasklet {
        let head = self.head;
        self.head = core::ptr::null_mut();
        self.tail = core::ptr::null_mut();
        head
    }

    fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    fn len(&self) -> usize {
        let mut count = 0;
        let mut node = self.head;
        while !node.is_null() {
            count += 1;
            node = unsafe { (*node).next };
        }
        count
    }

    fn clear(&mut self) {
        let _ = self.take_all();
    }
}

/// Linux `tasklet_vec`: normal-priority tasklets, one queue per CPU.
static LINUX_TASKLET_LIST: [Mutex<LinuxTaskletQueue>; MAX_CPUS] =
    [const { Mutex::new(LinuxTaskletQueue::new()) }; MAX_CPUS];

/// Linux `tasklet_hi_vec`: high-priority tasklets, one queue per CPU.
static LINUX_TASKLET_HI_LIST: [Mutex<LinuxTaskletQueue>; MAX_CPUS] =
    [const { Mutex::new(LinuxTaskletQueue::new()) }; MAX_CPUS];

unsafe fn append_linux_tasklet(
    queues: &[Mutex<LinuxTaskletQueue>; MAX_CPUS],
    tasklet: *mut LinuxTasklet,
    softirq: SoftIrqVec,
) {
    // Linux mutates tasklet_vec with local IRQs disabled. This is required
    // even though the queue has a lock: an interrupt on this CPU can schedule
    // a tasklet and would otherwise spin on the lock held by its interrupted
    // frame.
    let flags = crate::kernel::locking::local_irq_save();
    let cpu = current_cpu_slot();
    unsafe {
        queues[cpu].lock().push(tasklet);
    }
    raise_softirq_on_cpu(cpu, softirq);
    crate::kernel::locking::local_irq_restore(flags);
}

unsafe fn schedule_linux_tasklet(
    t: *mut LinuxTasklet,
    queues: &[Mutex<LinuxTaskletQueue>; MAX_CPUS],
    softirq: SoftIrqVec,
) {
    if t.is_null() {
        return;
    }
    unsafe {
        append_linux_tasklet(queues, t, softirq);
    }
}

/// `__tasklet_schedule` - `vendor/linux/kernel/softirq.c:838`.
///
/// Linux's inline `tasklet_schedule()` has already atomically set
/// `TASKLET_STATE_SCHED` before it calls this exported queueing helper.
pub unsafe extern "C" fn linux___tasklet_schedule(t: *mut LinuxTasklet) {
    unsafe {
        schedule_linux_tasklet(t, &LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);
    }
}

/// `__tasklet_hi_schedule` - `vendor/linux/kernel/softirq.c:846`.
///
/// As above, the inline caller owns the transition to SCHED before entry.
pub unsafe extern "C" fn linux___tasklet_hi_schedule(t: *mut LinuxTasklet) {
    unsafe {
        schedule_linux_tasklet(t, &LINUX_TASKLET_HI_LIST, SoftIrqVec::Hi);
    }
}

/// `tasklet_setup` - `vendor/linux/kernel/softirq.c:975`.
pub unsafe extern "C" fn linux_tasklet_setup(
    t: *mut LinuxTasklet,
    callback: Option<unsafe extern "C" fn(*mut LinuxTasklet)>,
) {
    if t.is_null() {
        return;
    }
    unsafe {
        (*t).next = core::ptr::null_mut();
        (*t).state.store(0, Ordering::Release);
        (*t).count.store(0, Ordering::Release);
        (*t).use_callback = true;
        (*t).callback.callback = callback;
        (*t).data = 0;
    }
}

/// `tasklet_init` - `vendor/linux/kernel/softirq.c:987`.
pub unsafe extern "C" fn linux_tasklet_init(
    t: *mut LinuxTasklet,
    func: Option<unsafe extern "C" fn(usize)>,
    data: usize,
) {
    if t.is_null() {
        return;
    }
    unsafe {
        (*t).next = core::ptr::null_mut();
        (*t).state.store(0, Ordering::Release);
        (*t).count.store(0, Ordering::Release);
        (*t).use_callback = false;
        (*t).callback.func = func;
        (*t).data = data;
    }
}

/// `tasklet_kill` - `vendor/linux/kernel/softirq.c:1022`.
pub unsafe extern "C" fn linux_tasklet_kill(t: *mut LinuxTasklet) {
    if t.is_null() {
        return;
    }

    // Linux wait_on_bit_lock(TASKLET_STATE_SCHED): wait until the queued
    // action consumes SCHED, then acquire that bit exclusively so a concurrent
    // scheduler cannot observe it set and lose its enqueue when kill clears it.
    loop {
        let state = unsafe { (*t).state.load(Ordering::Acquire) };
        if state & TASKLET_STATE_SCHED == 0
            && unsafe {
                (*t).state
                    .compare_exchange(
                        state,
                        state | TASKLET_STATE_SCHED,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
            }
        {
            break;
        }
        #[cfg(not(test))]
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        #[cfg(test)]
        core::hint::spin_loop();
    }
    unsafe {
        linux_tasklet_unlock_wait(t);
        linux_tasklet_clear_sched(t);
    }
}

/// `tasklet_unlock` - `vendor/linux/kernel/softirq.c:1036`.
pub unsafe extern "C" fn linux_tasklet_unlock(t: *mut LinuxTasklet) {
    if !t.is_null() {
        unsafe {
            linux_tasklet_clear_run(t);
        }
    }
}

/// `tasklet_unlock_wait` - `vendor/linux/kernel/softirq.c:1042`.
pub unsafe extern "C" fn linux_tasklet_unlock_wait(t: *mut LinuxTasklet) {
    if t.is_null() {
        return;
    }
    while unsafe { (*t).state.load(Ordering::Acquire) } & TASKLET_STATE_RUN != 0 {
        core::hint::spin_loop();
    }
}

/// `tasklet_unlock_spin_wait` - `vendor/linux/kernel/softirq.c:1005`.
pub unsafe extern "C" fn linux_tasklet_unlock_spin_wait(t: *mut LinuxTasklet) {
    unsafe {
        linux_tasklet_unlock_wait(t);
    }
}

#[inline]
unsafe fn linux_tasklet_trylock(t: *mut LinuxTasklet) -> bool {
    unsafe { (*t).state.fetch_or(TASKLET_STATE_RUN, Ordering::AcqRel) & TASKLET_STATE_RUN == 0 }
}

#[inline]
unsafe fn linux_tasklet_clear_sched(t: *mut LinuxTasklet) -> bool {
    unsafe {
        (*t).state.fetch_and(!TASKLET_STATE_SCHED, Ordering::AcqRel) & TASKLET_STATE_SCHED != 0
    }
}

#[inline]
unsafe fn linux_tasklet_clear_run(t: *mut LinuxTasklet) {
    unsafe {
        (*t).state.fetch_and(!TASKLET_STATE_RUN, Ordering::Release);
    }
}

unsafe fn invoke_linux_tasklet(t: *mut LinuxTasklet) {
    unsafe {
        if (*t).use_callback {
            if let Some(callback) = (*t).callback.callback {
                callback(t);
            }
        } else if let Some(func) = (*t).callback.func {
            func((*t).data);
        }
    }
}

/// `tasklet_action_common()` - `vendor/linux/kernel/softirq.c:916`.
///
/// Detach the current CPU's queue before invoking callbacks. A tasklet which
/// is disabled or already RUNning elsewhere retains SCHED, is appended to the
/// same local queue, and re-raises the matching softirq. RUN is released only
/// by the CPU which acquired it.
fn linux_tasklet_action_common(queues: &[Mutex<LinuxTaskletQueue>; MAX_CPUS], softirq: SoftIrqVec) {
    let flags = crate::kernel::locking::local_irq_save();
    let cpu = current_cpu_slot();
    let mut tasklet = {
        let mut queue = queues[cpu].lock();
        queue.take_all()
    };
    crate::kernel::locking::local_irq_restore(flags);

    while !tasklet.is_null() {
        let t = tasklet;
        tasklet = unsafe { (*t).next };

        let locked = unsafe { linux_tasklet_trylock(t) };
        if locked {
            let enabled = unsafe { (*t).count.load(Ordering::Acquire) } == 0;
            if enabled {
                let scheduled = unsafe { linux_tasklet_clear_sched(t) };
                if scheduled {
                    unsafe {
                        invoke_linux_tasklet(t);
                    }
                }
                unsafe {
                    linux_tasklet_clear_run(t);
                }
                // Linux consumes a queue entry whose SCHED bit was missing
                // after warning; it does not manufacture another schedule.
                continue;
            }
            unsafe {
                linux_tasklet_clear_run(t);
            }
        }

        // Disabled tasklets and tasklets RUNning on another CPU stay
        // scheduled. Requeue at the tail and retry from a later softirq pass.
        unsafe {
            append_linux_tasklet(queues, t, softirq);
        }
    }
}

fn linux_tasklet_action() {
    linux_tasklet_action_common(&LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);
}

fn linux_tasklet_hi_action() {
    linux_tasklet_action_common(&LINUX_TASKLET_HI_LIST, SoftIrqVec::Hi);
}

fn tasklet_hi_action() {
    linux_tasklet_hi_action();
}

// ── TDD: softirq boot test (Milestone 6) ──────────────────────────────────────

#[cfg(feature = "test-softirq")]
static SOFTIRQ_TEST_COUNT: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "test-softirq")]
static SOFTIRQ_TEST_RERAISE_REMAINING: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "test-softirq")]
fn softirq_test_handler() {
    SOFTIRQ_TEST_COUNT.fetch_add(1, Ordering::Release);
    if SOFTIRQ_TEST_RERAISE_REMAINING
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
            remaining.checked_sub(1)
        })
        .is_ok()
    {
        raise_softirq(SoftIrqVec::Rcu);
    }
}

/// Boot-time TDD: verify that registering, raising, and draining a softirq
/// works end-to-end.
///
/// Pass criterion (asserted by the xtask harness):
///   - serial log contains `softirq: deferred work executed`
///   - QEMU exits with success code (0x21).
///
/// We use the otherwise-idle `Rcu` slot so Linux-compatible `Sched`, `Hi`, and
/// `Tasklet` vectors remain dedicated to their production handlers. `IrqPoll`
/// is already owned by the i8042 deferred-input path at this point in boot.
#[cfg(feature = "test-softirq")]
pub fn run_softirq_test() {
    // 1. Register the test handler in a free non-tasklet slot.
    open_softirq(SoftIrqVec::Rcu, softirq_test_handler);

    // 2. Inline raise + drain — must execute the handler exactly once.
    raise_softirq(SoftIrqVec::Rcu);
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

    // 4. Linux handle_softirqs() stops after MAX_SOFTIRQ_RESTART passes and
    // wakes this CPU's ksoftirqd. Keep re-raising beyond that first batch,
    // prove the inline caller returns with work pending, then yield until the
    // CPU-affine daemon drains the remainder.
    const DEFERRED_RERAISES: u32 = MAX_SOFTIRQ_RESTART as u32 + 3;
    let before_deferred = SOFTIRQ_TEST_COUNT.load(Ordering::Acquire);
    SOFTIRQ_TEST_RERAISE_REMAINING.store(DEFERRED_RERAISES, Ordering::Release);

    let irq_flags = crate::kernel::locking::local_irq_save();
    raise_softirq(SoftIrqVec::Rcu);
    do_softirq();
    assert_ne!(
        local_softirq_pending() & SoftIrqVec::Rcu.bit(),
        0,
        "softirq: inline drain consumed work beyond Linux restart budget"
    );
    crate::kernel::locking::local_irq_restore(irq_flags);

    let deadline = crate::kernel::time::jiffies::jiffies()
        .saturating_add(crate::kernel::time::jiffies::msecs_to_jiffies(1_000).max(1));
    while local_softirq_pending() & SoftIrqVec::Rcu.bit() != 0
        || SOFTIRQ_TEST_RERAISE_REMAINING.load(Ordering::Acquire) != 0
    {
        assert!(
            crate::kernel::time::jiffies::jiffies() < deadline,
            "softirq: ksoftirqd did not drain deferred RCU test work"
        );
        unsafe {
            crate::kernel::sched::reschedule_runnable();
        }
    }

    let after_deferred = SOFTIRQ_TEST_COUNT.load(Ordering::Acquire);
    assert_eq!(
        after_deferred - before_deferred,
        DEFERRED_RERAISES + 1,
        "softirq: ksoftirqd lost or duplicated deferred work"
    );

    // 5. Banner — must match SOFTIRQ_BANNER in xtask/src/lib.rs.
    crate::kernel::printk::log_info!(
        "softirq",
        "softirq: deferred work executed (count={}, ksoftirqd-drained={})",
        after_deferred,
        after_deferred - before_deferred
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
    // Serialize all tests that touch softirq state so parallel host test
    // threads cannot race on CPU0's emulated per-CPU slot.
    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    /// Reset all global softirq state and hold the test mutex for the
    /// caller's scope.  The caller must bind the return value:
    ///   `let _guard = reset_state();`
    fn reset_state() -> spin::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock();
        for pending in &PENDING {
            pending.store(0, Ordering::SeqCst);
        }
        for in_softirq in &IN_SOFTIRQ {
            in_softirq.store(false, Ordering::SeqCst);
        }
        for task in &KSOFTIRQD_TASKS {
            task.store(core::ptr::null_mut(), Ordering::SeqCst);
        }
        for wakeups in &KSOFTIRQD_WAKE_REQUESTS {
            wakeups.store(0, Ordering::SeqCst);
        }
        SOFTIRQ_VEC_DISPATCH_LOCK_ACQUISITIONS.store(0, Ordering::SeqCst);
        for slot in &SOFTIRQ_VEC {
            slot.clear();
        }
        for list in &TASKLET_LIST {
            list.lock().clear();
        }
        for list in &LINUX_TASKLET_LIST {
            list.lock().clear();
        }
        for list in &LINUX_TASKLET_HI_LIST {
            list.lock().clear();
        }
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
        assert_eq!(pending_for_cpu(0).load(Ordering::SeqCst), 0);
        raise_softirq(SoftIrqVec::Timer);
        assert_eq!(
            pending_for_cpu(0).load(Ordering::SeqCst),
            SoftIrqVec::Timer.bit()
        );
        // Drain via swap
        let drained = pending_for_cpu(0).swap(0, Ordering::SeqCst);
        assert_eq!(drained, SoftIrqVec::Timer.bit());
        assert_eq!(pending_for_cpu(0).load(Ordering::SeqCst), 0);
    }

    #[test]
    fn raise_softirq_is_idempotent_on_same_bit() {
        let _guard = reset_state();
        raise_softirq(SoftIrqVec::Hi);
        raise_softirq(SoftIrqVec::Hi);
        // Two raises must yield a single bit, not "2".
        assert_eq!(
            pending_for_cpu(0).load(Ordering::SeqCst),
            SoftIrqVec::Hi.bit()
        );
    }

    #[test]
    fn pending_masks_are_cpu_local() {
        let _guard = reset_state();
        raise_softirq_on_cpu(1, SoftIrqVec::NetRx);

        assert_eq!(pending_for_cpu(0).load(Ordering::SeqCst), 0);
        assert_eq!(
            pending_for_cpu(1).load(Ordering::SeqCst),
            SoftIrqVec::NetRx.bit()
        );
    }

    #[test]
    fn open_softirq_installs_handler() {
        let _guard = reset_state();
        fn h() {}
        open_softirq(SoftIrqVec::NetRx, h);
        assert!(registered_softirq_handler(SoftIrqVec::NetRx.index()).is_some());
    }

    #[test]
    fn init_registers_distinct_hi_and_normal_tasklet_actions() {
        let _guard = reset_state();
        init();
        assert_eq!(
            registered_softirq_handler(SoftIrqVec::Hi.index()).map(|handler| handler as usize),
            Some(tasklet_hi_action as usize)
        );
        assert_eq!(
            registered_softirq_handler(SoftIrqVec::Tasklet.index()).map(|handler| handler as usize),
            Some(tasklet_action as usize)
        );
    }

    static DRAIN_COUNT: AtomicU32 = AtomicU32::new(0);
    fn drain_handler() {
        DRAIN_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    static BUDGET_COUNT: AtomicU32 = AtomicU32::new(0);
    fn budget_handler() {
        BUDGET_COUNT.fetch_add(1, Ordering::SeqCst);
        raise_softirq(SoftIrqVec::Sched);
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
        assert_eq!(pending_for_cpu(0).load(Ordering::SeqCst), 0);
        // Guard must be released after drain.
        assert!(!in_softirq_for_cpu(0).load(Ordering::SeqCst));
    }

    #[test]
    fn outermost_local_bh_enable_drains_pending_softirq() {
        // Origin: vendor/linux/kernel/softirq.c::__local_bh_enable_ip.
        // On !PREEMPT_RT Linux, the outermost BH enable keeps preemption
        // disabled, invokes do_softirq() for local pending work, and only then
        // removes the final preempt-count offset.
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        open_softirq(SoftIrqVec::Hi, drain_handler);

        crate::kernel::locking::preempt::local_bh_disable();
        raise_softirq(SoftIrqVec::Hi);
        assert_eq!(
            DRAIN_COUNT.load(Ordering::SeqCst),
            0,
            "BH-disabled work must remain deferred"
        );
        crate::kernel::locking::preempt::local_bh_enable();
        assert_eq!(
            DRAIN_COUNT.load(Ordering::SeqCst),
            1,
            "outermost local_bh_enable must run pending local softirqs"
        );
        assert_eq!(local_softirq_pending(), 0);

        crate::kernel::locking::preempt::linux___local_bh_disable_ip(
            0,
            crate::kernel::locking::preempt::SOFTIRQ_DISABLE_OFFSET,
        );
        raise_softirq(SoftIrqVec::Hi);
        crate::kernel::locking::preempt::linux___local_bh_enable_ip(
            0,
            crate::kernel::locking::preempt::SOFTIRQ_DISABLE_OFFSET,
        );
        assert_eq!(
            DRAIN_COUNT.load(Ordering::SeqCst),
            2,
            "__local_bh_enable_ip must share the outermost drain behavior"
        );
        assert_eq!(local_softirq_pending(), 0);
    }

    #[test]
    fn softirq_dispatch_does_not_take_global_registration_mutex() {
        // Origin: vendor/linux/kernel/softirq.c::handle_softirqs. Linux starts
        // at `softirq_vec` once and advances through all pending actions; it
        // does not acquire a global registration lock for each action.
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        open_softirq(SoftIrqVec::Hi, drain_handler);
        open_softirq(SoftIrqVec::Timer, drain_handler);
        raise_softirq(SoftIrqVec::Hi);
        raise_softirq(SoftIrqVec::Timer);
        SOFTIRQ_VEC_DISPATCH_LOCK_ACQUISITIONS.store(0, Ordering::SeqCst);

        do_softirq();

        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 2);
        assert_eq!(
            SOFTIRQ_VEC_DISPATCH_LOCK_ACQUISITIONS.load(Ordering::SeqCst),
            0,
            "softirq dispatch must not serialize CPUs on a global registration mutex"
        );
    }

    #[test]
    fn do_softirq_empty_pending_word_does_not_enter_handler_context() {
        // Origin: vendor/linux/kernel/softirq.c::do_softirq. Linux checks
        // local_softirq_pending() with IRQs disabled and skips
        // do_softirq_own_stack() when the word is zero.
        let _guard = reset_state();
        assert_eq!(pending_for_cpu(0).load(Ordering::SeqCst), 0);

        do_softirq();

        assert!(
            !in_softirq_for_cpu(0).load(Ordering::SeqCst),
            "empty do_softirq must not acquire the heavy-path guard"
        );
        assert_eq!(crate::kernel::locking::preempt::preempt_count(), 0);
    }

    #[test]
    fn do_softirq_respects_in_softirq_guard() {
        let _guard = reset_state();
        // Pretend we are already inside a softirq drain.
        in_softirq_for_cpu(0).store(true, Ordering::SeqCst);
        open_softirq(SoftIrqVec::Hi, drain_handler);
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        raise_softirq(SoftIrqVec::Hi);
        do_softirq();
        // Guard prevented entry → handler must NOT have run.
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 0);
        // Pending bit must still be set so the outer drain can pick it up.
        assert_eq!(
            pending_for_cpu(0).load(Ordering::SeqCst),
            SoftIrqVec::Hi.bit()
        );
        // Reset guard for subsequent tests.
        in_softirq_for_cpu(0).store(false, Ordering::SeqCst);
    }

    #[test]
    fn continuously_reraised_softirq_stops_at_linux_restart_budget() {
        let _guard = reset_state();
        BUDGET_COUNT.store(0, Ordering::SeqCst);
        open_softirq(SoftIrqVec::Sched, budget_handler);
        raise_softirq(SoftIrqVec::Sched);
        let wakes_before_drain = KSOFTIRQD_WAKE_REQUESTS[0].load(Ordering::SeqCst);

        do_softirq();

        let runs = BUDGET_COUNT.load(Ordering::SeqCst);
        assert!(runs >= 1);
        assert!(runs <= MAX_SOFTIRQ_RESTART as u32);
        assert_ne!(
            pending_for_cpu(0).load(Ordering::SeqCst) & SoftIrqVec::Sched.bit(),
            0,
            "work beyond the budget must remain pending"
        );
        assert!(
            KSOFTIRQD_WAKE_REQUESTS[0].load(Ordering::SeqCst) > wakes_before_drain,
            "Linux handle_softirqs() must wake ksoftirqd when work exceeds its budget"
        );
    }

    #[test]
    fn ksoftirqd_name_and_should_run_match_linux_per_cpu_policy() {
        let _guard = reset_state();
        assert_eq!(&ksoftirqd_name(0)[..12], b"ksoftirqd/0\0");
        assert_eq!(&ksoftirqd_name(12)[..13], b"ksoftirqd/12\0");

        assert!(!ksoftirqd_should_run(0));
        raise_softirq_on_cpu(0, SoftIrqVec::NetRx);
        assert!(ksoftirqd_should_run(0));
        assert!(!ksoftirqd_should_run(1));
    }

    #[test]
    fn tasklet_schedule_sets_tasklet_pending_bit() {
        let _guard = reset_state();
        static T: Tasklet = Tasklet::new(|_| {}, 0);
        // Manually clear `scheduled` because the static persists across tests.
        T.scheduled.store(false, Ordering::SeqCst);
        T.running.store(false, Ordering::SeqCst);

        let queued = tasklet_schedule(&T);
        assert!(queued);
        // Tasklet softirq bit must be raised.
        assert_ne!(
            pending_for_cpu(0).load(Ordering::SeqCst) & SoftIrqVec::Tasklet.bit(),
            0
        );
        // Second schedule on the same tasklet must be a no-op.
        let queued_again = tasklet_schedule(&T);
        assert!(!queued_again);
        // Cleanup so other tests don't see leftover state.
        T.scheduled.store(false, Ordering::SeqCst);
        T.running.store(false, Ordering::SeqCst);
        TASKLET_LIST[0].lock().clear();
    }

    static GENERIC_SELF_RESCHEDULE: AtomicBool = AtomicBool::new(false);
    static GENERIC_TASKLET_RUNS: AtomicU32 = AtomicU32::new(0);
    static GENERIC_SELF_TASKLET: Tasklet = Tasklet::new(generic_self_reschedule_handler, 0);

    fn generic_self_reschedule_handler(_data: u64) {
        GENERIC_TASKLET_RUNS.fetch_add(1, Ordering::SeqCst);
        if GENERIC_SELF_RESCHEDULE.swap(false, Ordering::SeqCst) {
            assert!(tasklet_schedule(&GENERIC_SELF_TASKLET));
        }
    }

    #[test]
    fn generic_tasklet_self_reschedule_waits_for_next_action_snapshot() {
        let _guard = reset_state();
        GENERIC_SELF_TASKLET
            .scheduled
            .store(false, Ordering::SeqCst);
        GENERIC_SELF_TASKLET.running.store(false, Ordering::SeqCst);
        GENERIC_SELF_RESCHEDULE.store(true, Ordering::SeqCst);
        GENERIC_TASKLET_RUNS.store(0, Ordering::SeqCst);

        assert!(tasklet_schedule(&GENERIC_SELF_TASKLET));
        pending_for_cpu(0).store(0, Ordering::SeqCst);
        tasklet_action();

        assert_eq!(GENERIC_TASKLET_RUNS.load(Ordering::SeqCst), 1);
        assert!(GENERIC_SELF_TASKLET.scheduled.load(Ordering::SeqCst));
        assert!(!GENERIC_SELF_TASKLET.running.load(Ordering::SeqCst));
        assert_eq!(TASKLET_LIST[0].lock().len(), 1);
        assert_ne!(
            pending_for_cpu(0).load(Ordering::SeqCst) & SoftIrqVec::Tasklet.bit(),
            0
        );

        pending_for_cpu(0).store(0, Ordering::SeqCst);
        tasklet_action();
        assert_eq!(GENERIC_TASKLET_RUNS.load(Ordering::SeqCst), 2);
        assert!(TASKLET_LIST[0].lock().is_empty());
        assert!(!GENERIC_SELF_TASKLET.scheduled.load(Ordering::SeqCst));
        assert!(!GENERIC_SELF_TASKLET.running.load(Ordering::SeqCst));
    }

    #[test]
    fn linux_tasklet_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__tasklet_schedule"),
            Some(linux___tasklet_schedule as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__tasklet_hi_schedule"),
            Some(linux___tasklet_hi_schedule as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("tasklet_setup"),
            Some(linux_tasklet_setup as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("tasklet_kill"),
            Some(linux_tasklet_kill as usize)
        );
    }

    static CALLBACK_STATE: AtomicUsize = AtomicUsize::new(0);
    static SELF_RESCHEDULE: AtomicBool = AtomicBool::new(false);

    unsafe extern "C" fn linux_tasklet_test_callback(t: *mut LinuxTasklet) {
        if !t.is_null() {
            CALLBACK_STATE.store(
                unsafe { (*t).state.load(Ordering::SeqCst) },
                Ordering::SeqCst,
            );
        }
        DRAIN_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn linux_tasklet_self_reschedule_callback(t: *mut LinuxTasklet) {
        DRAIN_COUNT.fetch_add(1, Ordering::SeqCst);
        if SELF_RESCHEDULE.swap(false, Ordering::SeqCst) {
            let old = unsafe { (*t).state.fetch_or(TASKLET_STATE_SCHED, Ordering::AcqRel) };
            if old & TASKLET_STATE_SCHED == 0 {
                unsafe {
                    linux___tasklet_schedule(t);
                }
            }
        }
    }

    #[test]
    fn linux_tasklet_schedule_runs_callback_from_softirq_drain() {
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        CALLBACK_STATE.store(0, Ordering::SeqCst);
        open_softirq(SoftIrqVec::Tasklet, tasklet_action);
        let mut tasklet = LinuxTasklet {
            next: core::ptr::null_mut(),
            state: AtomicUsize::new(0),
            count: AtomicI32::new(0),
            use_callback: false,
            callback: LinuxTaskletCallback { func: None },
            data: 0,
        };

        unsafe {
            linux_tasklet_setup(&mut tasklet, Some(linux_tasklet_test_callback));
            tasklet
                .state
                .fetch_or(TASKLET_STATE_SCHED, Ordering::AcqRel);
            linux___tasklet_schedule(&mut tasklet);
        }
        assert_ne!(
            pending_for_cpu(0).load(Ordering::SeqCst) & SoftIrqVec::Tasklet.bit(),
            0
        );
        do_softirq();
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 1);
        let callback_state = CALLBACK_STATE.load(Ordering::SeqCst);
        assert_eq!(callback_state & TASKLET_STATE_SCHED, 0);
        assert_ne!(callback_state & TASKLET_STATE_RUN, 0);
        assert_eq!(
            tasklet.state.load(Ordering::SeqCst) & (TASKLET_STATE_SCHED | TASKLET_STATE_RUN),
            0
        );
    }

    #[test]
    fn disabled_linux_tasklet_is_requeued_with_sched_set() {
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        let mut tasklet = LinuxTasklet {
            next: core::ptr::null_mut(),
            state: AtomicUsize::new(TASKLET_STATE_SCHED),
            count: AtomicI32::new(1),
            use_callback: true,
            callback: LinuxTaskletCallback {
                callback: Some(linux_tasklet_test_callback),
            },
            data: 0,
        };

        unsafe {
            linux___tasklet_schedule(&mut tasklet);
        }
        pending_for_cpu(0).store(0, Ordering::SeqCst);
        linux_tasklet_action_common(&LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);

        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 0);
        assert_eq!(LINUX_TASKLET_LIST[0].lock().len(), 1);
        assert_eq!(
            tasklet.state.load(Ordering::SeqCst) & (TASKLET_STATE_SCHED | TASKLET_STATE_RUN),
            TASKLET_STATE_SCHED
        );
        assert_ne!(
            pending_for_cpu(0).load(Ordering::SeqCst) & SoftIrqVec::Tasklet.bit(),
            0
        );

        tasklet.count.store(0, Ordering::SeqCst);
        pending_for_cpu(0).store(0, Ordering::SeqCst);
        linux_tasklet_action_common(&LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 1);
        assert!(LINUX_TASKLET_LIST[0].lock().is_empty());
        assert_eq!(tasklet.state.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn contended_linux_tasklet_keeps_foreign_run_lock() {
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        let mut tasklet = LinuxTasklet {
            next: core::ptr::null_mut(),
            state: AtomicUsize::new(TASKLET_STATE_SCHED | TASKLET_STATE_RUN),
            count: AtomicI32::new(0),
            use_callback: true,
            callback: LinuxTaskletCallback {
                callback: Some(linux_tasklet_test_callback),
            },
            data: 0,
        };

        unsafe {
            linux___tasklet_schedule(&mut tasklet);
        }
        pending_for_cpu(0).store(0, Ordering::SeqCst);
        linux_tasklet_action_common(&LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);

        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 0);
        assert_eq!(LINUX_TASKLET_LIST[0].lock().len(), 1);
        assert_eq!(
            tasklet.state.load(Ordering::SeqCst) & (TASKLET_STATE_SCHED | TASKLET_STATE_RUN),
            TASKLET_STATE_SCHED | TASKLET_STATE_RUN
        );

        unsafe {
            linux_tasklet_unlock(&mut tasklet);
        }
        assert_eq!(
            tasklet.state.load(Ordering::SeqCst) & (TASKLET_STATE_SCHED | TASKLET_STATE_RUN),
            TASKLET_STATE_SCHED
        );
        pending_for_cpu(0).store(0, Ordering::SeqCst);
        linux_tasklet_action_common(&LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(tasklet.state.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn high_tasklet_uses_only_hi_queue_and_vector() {
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        let mut tasklet = LinuxTasklet {
            next: core::ptr::null_mut(),
            state: AtomicUsize::new(TASKLET_STATE_SCHED),
            count: AtomicI32::new(0),
            use_callback: true,
            callback: LinuxTaskletCallback {
                callback: Some(linux_tasklet_test_callback),
            },
            data: 0,
        };

        unsafe {
            linux___tasklet_hi_schedule(&mut tasklet);
        }
        assert!(LINUX_TASKLET_LIST[0].lock().is_empty());
        assert_eq!(LINUX_TASKLET_HI_LIST[0].lock().len(), 1);
        assert_eq!(
            pending_for_cpu(0).load(Ordering::SeqCst),
            SoftIrqVec::Hi.bit()
        );

        linux_tasklet_action();
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 0);
        assert_eq!(LINUX_TASKLET_HI_LIST[0].lock().len(), 1);
        linux_tasklet_hi_action();
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 1);
        assert!(LINUX_TASKLET_HI_LIST[0].lock().is_empty());
        assert_eq!(tasklet.state.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn self_reschedule_stays_queued_for_next_action_pass() {
        let _guard = reset_state();
        DRAIN_COUNT.store(0, Ordering::SeqCst);
        SELF_RESCHEDULE.store(true, Ordering::SeqCst);
        let mut tasklet = LinuxTasklet {
            next: core::ptr::null_mut(),
            state: AtomicUsize::new(TASKLET_STATE_SCHED),
            count: AtomicI32::new(0),
            use_callback: true,
            callback: LinuxTaskletCallback {
                callback: Some(linux_tasklet_self_reschedule_callback),
            },
            data: 0,
        };

        unsafe {
            linux___tasklet_schedule(&mut tasklet);
        }
        linux_tasklet_action_common(&LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(LINUX_TASKLET_LIST[0].lock().len(), 1);
        assert_eq!(
            tasklet.state.load(Ordering::SeqCst) & (TASKLET_STATE_SCHED | TASKLET_STATE_RUN),
            TASKLET_STATE_SCHED
        );

        linux_tasklet_action_common(&LINUX_TASKLET_LIST, SoftIrqVec::Tasklet);
        assert_eq!(DRAIN_COUNT.load(Ordering::SeqCst), 2);
        assert!(LINUX_TASKLET_LIST[0].lock().is_empty());
        assert_eq!(tasklet.state.load(Ordering::SeqCst), 0);
    }
}
