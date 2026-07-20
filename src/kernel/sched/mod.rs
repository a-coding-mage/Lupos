//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched
//! test-origin: linux:vendor/linux/kernel/sched
//! Scheduler core — M21 → M32.
//!
//! Layered architecture:
//!
//! | Layer  | Module       | Milestone | Description                                        |
//! |--------|--------------|-----------|----------------------------------------------------|
//! | UAPI   | `prio`       | M29       | `SCHED_*` constants, nice-to-weight tables         |
//! | UAPI   | `entity`     | M29       | `sched_entity`, `sched_rt_entity`, `sched_dl_entity` |
//! | Core   | `class`      | M29       | `struct sched_class` dispatch table                |
//! | Core   | `rq`         | M29/M31   | Per-CPU runqueue (`cfs`, `rt`, `dl` sub-rqs)       |
//! | Class  | `fair`       | M29       | CFS — `update_curr`, `pick_next_entity`, vruntime  |
//! | Class  | `rt`         | M30       | `SCHED_FIFO` / `SCHED_RR`                          |
//! | Class  | `deadline`   | M30       | `SCHED_DEADLINE` (EDF + admission control)         |
//! | Class  | `idle`       | M29       | Idle class                                         |
//! | Class  | `stop`       | M31       | Stop class for migration kthreads                  |
//! | SMP    | `topology`   | M31       | `sched_domain` hierarchy                           |
//! | SMP    | `balance`    | M31       | `load_balance`                                     |
//! | SMP    | `nohz`       | M31       | NOHZ idle bookkeeping                              |
//! | UAPI   | `syscalls`   | M30       | `sched_setattr` / `getattr` / `setscheduler`       |
//! | PELT   | `pelt`       | M31       | Per-entity load tracking (stub in M29)             |
//!
//! Provides legacy cooperative API (used by every M21–M28 boot test):
//!   * Per-CPU `current_task` array (indexed by LAPIC ID).
//!   * Simple global run queue (static array of task pointers).
//!   * `schedule()` — picks the next task and calls `__switch_to_asm`.
//!   * `kthread_create()`, `sched_alloc_kthread_raw()`, `enqueue_task`,
//!     `dequeue_task`, `sched_init`.
//!
//! The cooperative `schedule()` is preserved end-to-end so the M21 → M28 test
//! suite stays green.  M29's `sched_class` infrastructure layers on top of it:
//! tasks carry a `sched_class` pointer, an `Rq` exists per CPU, and CFS-style
//! virtual-runtime accounting runs in parallel — but the actual `pick_next`
//! still falls through to round-robin over the runnable mask.  As CFS / RT /
//! DL classes mature (M29–M30) and load balancing is wired up (M31), the
//! cooperative path is shrunk; full removal is gated on M33 locking and M34
//! per-CPU storage.
//!
//! # Per-CPU current_task
//!
//! Linux uses GS-relative per-CPU variables (`this_cpu_read(current_task)`).
//! Full GS-based per-CPU isn't wired up yet, so we use a simple array indexed
//! by `apic::id()`.  This is replaced by GS-relative access in M34.

pub mod class;
pub mod entity;
pub mod fair;
pub mod idle;
pub mod pelt;
pub mod prio;
pub mod rq;
pub mod stop;
pub mod topology;
// M30
pub mod deadline;
pub mod rt;
pub mod syscalls;
// M31
pub mod balance;
pub mod nohz;
// Phase 4 closure: Linux scheduler utility surfaces.
pub mod autogroup;
pub mod build_policy;
pub mod build_utility;
pub mod clock;
pub mod completion;
pub mod core_sched;
pub mod cpuacct;
pub mod cpudeadline;
pub mod cpufreq;
pub mod cpufreq_schedutil;
pub mod cpupri;
pub mod cputime;
pub mod debug;
pub mod ext;
pub mod ext_idle;
pub mod isolation;
pub mod loadavg;
pub mod membarrier;
pub mod psi;
pub mod rq_offsets;
pub mod stats;
pub mod stop_task;
pub mod swait;
pub mod wait;
pub mod wait_bit;

pub use entity::{SCHED_CLOCK_NS, sched_clock_ns};

#[cfg(test)]
extern crate std;

extern crate alloc;

pub fn register_module_exports() {
    clock::register_module_exports();
    completion::register_module_exports();
    swait::register_module_exports();
    wait::register_module_exports();
    wait_bit::register_module_exports();
    use crate::kernel::module::{export_symbol, find_symbol};
    if find_symbol("yield").is_none() {
        export_symbol("yield", linux_yield as usize, false);
    }
    if find_symbol("sched_set_fifo").is_none() {
        export_symbol("sched_set_fifo", linux_sched_set_fifo as usize, true);
    }
}

use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use spin::Mutex;

use crate::arch::x86::kernel::apic;
use crate::arch::x86::kernel::switch::{
    __switch_to_asm, prepare_switch_to_task, record_switch_attempt,
};
use crate::kernel::pid::{INIT_PID_NS, alloc_pid};
use crate::kernel::task::{LINUX_OFFSET_THREAD, M29SchedFields, TaskStruct, ThreadInfo};
use crate::kernel::thread::{DescStruct, ThreadStruct};

unsafe extern "C" fn linux_yield() {
    let _ = unsafe { crate::kernel::sched::syscalls::sys_sched_yield() };
}

/// `sched_set_fifo` - `vendor/linux/kernel/sched/syscalls.c`.
unsafe extern "C" fn linux_sched_set_fifo(task: *mut TaskStruct) {
    let _ = unsafe {
        syscalls::sys_sched_setscheduler(task, prio::SCHED_FIFO, (prio::MAX_RT_PRIO / 2) as u32)
    };
}

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum CPUs tracked by the per-CPU `CURRENT_TASK` array.
/// Matches `MAX_APS + 1 BSP` from `smp.rs`.
pub const MAX_CPUS: usize = 9;

/// Maximum kernel threads in the static pool (BSP + created kthreads).
// Keep the static task + stack pools below the early 16 MiB boot mapping while
// still leaving enough threads for AHCI/SCSI per-port workers.
pub const MAX_KTHREADS: usize = 30;

/// Kernel stack size per thread.
///
/// Lupos runs a Rust debug-profile kernel in boot gates.  The module loader's
/// debug frame plus nested finalizers exceeds 32 KiB even though an optimized
/// Linux build normally fits its architecture default.  Heap-created task
/// stacks also carry an unmapped leading guard page (see `vmalloc_stack`).
pub const KTHREAD_STACK_SIZE: usize = 64 * 1024;

/// Maximum tasks in the legacy cooperative run queue.
///
/// This queue is shared by early kernel tasks and heap-allocated user tasks
/// until the production per-CPU scheduler takes over.  It must therefore be
/// sized for normal userspace fan-out, not just the static kthread pool.
pub const MAX_RUN_QUEUE: usize = 1024;
const SWITCH_FRAME_BYTES: usize = 7 * core::mem::size_of::<u64>();

#[inline]
fn stack_top_for_sp(sp: usize) -> usize {
    if sp == 0 {
        return 0;
    }
    (sp + KTHREAD_STACK_SIZE - 1) & !(KTHREAD_STACK_SIZE - 1)
}

#[cfg(all(target_arch = "x86_64", not(test)))]
#[inline]
fn current_stack_pointer() -> usize {
    let sp: usize;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) sp, options(nomem, preserves_flags));
    }
    sp
}

#[inline]
unsafe fn seed_current_task_stack_from_sp(task: *mut TaskStruct, sp: usize) {
    if task.is_null() || unsafe { !(*task).stack.is_null() } {
        return;
    }
    let stack_top = stack_top_for_sp(sp);
    assert_ne!(
        stack_top, 0,
        "scheduler current task stack pointer must be nonzero"
    );
    unsafe {
        (*task).stack = stack_top as *mut core::ffi::c_void;
    }
}

#[cfg(all(target_arch = "x86_64", not(test)))]
#[inline]
unsafe fn seed_current_task_stack(task: *mut TaskStruct) {
    unsafe {
        seed_current_task_stack_from_sp(task, current_stack_pointer());
    }
}

#[cfg(any(not(target_arch = "x86_64"), test))]
#[inline]
unsafe fn seed_current_task_stack(_task: *mut TaskStruct) {}

// ── Static storage ───────────────────────────────────────────────────────────
//
// Using static pools avoids any heap dependency during early boot.
// The BSP task occupies slot 0; kthreads start at slot 1.

/// Per-CPU current task pointer.  Index == LAPIC ID (typically 0 for BSP).
///
/// SAFETY: Each slot is written only by the CPU whose LAPIC ID equals the
/// index, and only inside `set_current()` which is called from `__switch_to`.
/// Cross-CPU reads are racy but acceptable for the cooperative scheduler.
#[cfg(not(test))]
static mut CURRENT_TASK: [*mut TaskStruct; MAX_CPUS] = [core::ptr::null_mut(); MAX_CPUS];

/// Autoreaped current tasks cannot drop their own kernel stack. Linux keeps a
/// final current-task reference until finish_task_switch(); this per-CPU slot
/// carries the equivalent ownership across Lupos's stack switch.
static DEFERRED_TASK_RELEASE: [AtomicPtr<TaskStruct>; MAX_CPUS] =
    [const { AtomicPtr::new(core::ptr::null_mut()) }; MAX_CPUS];

#[cfg(test)]
std::thread_local! {
    static TEST_CURRENT_TASK: core::cell::Cell<*mut TaskStruct> =
        const { core::cell::Cell::new(core::ptr::null_mut()) };
}

/// Static task pool (BSP task at index 0, kthreads at 1..MAX_KTHREADS).
static mut TASK_POOL: [TaskStruct; MAX_KTHREADS] =
    [const { unsafe { core::mem::zeroed() } }; MAX_KTHREADS];

/// Per-CPU idle tasks for APs in the production SMP scheduler.
///
/// CPU 0 keeps using `TASK_POOL[0]` so the legacy BSP path remains unchanged.
static mut AP_IDLE_TASKS: [TaskStruct; MAX_CPUS] =
    [const { unsafe { core::mem::zeroed() } }; MAX_CPUS];

/// Kernel stacks for kthreads (index 0 is unused; BSP uses the boot stack).
#[repr(align(16))]
struct KthreadStack([u8; KTHREAD_STACK_SIZE]);

static mut KTHREAD_STACKS: [KthreadStack; MAX_KTHREADS] =
    [const { KthreadStack([0u8; KTHREAD_STACK_SIZE]) }; MAX_KTHREADS];

/// Next free slot index in TASK_POOL / KTHREAD_STACKS.
/// Starts at 0; sched_init() sets it to 1 after placing the BSP task at slot 0,
/// so kthread_create() always allocates from slot 1 onwards.
static KTHREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Production SMP scheduler gate.
///
/// The legacy global queue remains active until at least one AP has joined
/// normal scheduling.  Once enabled, `schedule()` dispatches entirely through
/// per-CPU runqueues.
static PRODUCTION_SCHED_ENABLED: AtomicBool = AtomicBool::new(false);

/// Number of CPUs that have joined the production scheduler.
static SCHED_ONLINE_CPUS: AtomicU32 = AtomicU32::new(0);

/// Logical CPUs which are currently active for task placement.
///
/// This is Lupos's `cpu_active_mask`.  Keep it distinct from the compile-time
/// `NR_CPUS` width: Linux's `sched_getaffinity()` intersects a task's allowed
/// mask with `cpu_active_mask`, so userspace must never infer 64 runnable CPUs
/// merely because `cpumask_t` contains 64 bits.
static SCHED_ACTIVE_CPUS: AtomicU64 = AtomicU64::new(1);

/// Snapshot Linux's `cpu_active_mask` in the task cpumask representation.
pub fn cpu_active_mask() -> entity::CpuMask {
    entity::CpuMask(SCHED_ACTIVE_CPUS.load(Ordering::Acquire))
}

// ── Run queue ────────────────────────────────────────────────────────────────

struct RunQueue {
    tasks: [*mut TaskStruct; MAX_RUN_QUEUE],
    len: usize,
    /// Index of the task that is currently running on this CPU.
    current_idx: usize,
}

// SAFETY: The run queue is only accessed under the spinlock below.
unsafe impl Send for RunQueue {}

impl RunQueue {
    fn normalize_legacy(&mut self) {
        if self.len > MAX_RUN_QUEUE {
            self.len = MAX_RUN_QUEUE;
        }
        if self.len == 0 {
            self.current_idx = 0;
        } else if self.current_idx >= self.len {
            self.current_idx = self.len - 1;
        }
    }

    fn active_tasks(&self) -> &[*mut TaskStruct] {
        &self.tasks[..self.len.min(MAX_RUN_QUEUE)]
    }
}

static RUN_QUEUE: Mutex<RunQueue> = Mutex::new(RunQueue {
    tasks: [core::ptr::null_mut(); MAX_RUN_QUEUE],
    len: 0,
    current_idx: 0,
});

/// Run a legacy runqueue critical section with local interrupts disabled.
///
/// The periodic LAPIC interrupt expires hrtimers, and an hrtimer callback may
/// wake a task through `legacy_place_after_current()`. If the interrupt lands
/// while task context owns `RUN_QUEUE`, the callback would otherwise spin on a
/// lock held by the interrupted frame. This is the legacy equivalent of
/// `rq::with_rq()`'s `rq_lock_irqsave()` discipline.
fn with_legacy_run_queue<R>(f: impl FnOnce(&mut RunQueue) -> R) -> R {
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    let result = {
        let mut rq = RUN_QUEUE.lock();
        f(&mut rq)
    };
    crate::kernel::locking::irqflags::local_irq_restore(flags);
    result
}

#[cfg(test)]
fn clear_legacy_run_queue_for_tests() {
    let mut rq = RUN_QUEUE.lock();
    rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
    rq.len = 0;
    rq.current_idx = 0;
}

fn legacy_place_after_current(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    let current = unsafe { get_current() };
    with_legacy_run_queue(|rq| {
        rq.normalize_legacy();
        if rq.len == 0 {
            rq.tasks[0] = task;
            rq.len = 1;
            rq.current_idx = 0;
            return;
        }
        if task == current {
            return;
        }

        if let Some(pos) = rq.active_tasks().iter().position(|&t| t == task) {
            for i in pos..rq.len - 1 {
                rq.tasks[i] = rq.tasks[i + 1];
            }
            rq.len -= 1;
            let len = rq.len;
            rq.tasks[len] = core::ptr::null_mut();
        } else if rq.len >= MAX_RUN_QUEUE {
            return;
        }

        let current_pos = rq
            .active_tasks()
            .iter()
            .position(|&t| !current.is_null() && t == current)
            .unwrap_or_else(|| rq.current_idx.min(rq.len.saturating_sub(1)));
        let idx = (current_pos + 1).min(rq.len);
        for i in (idx..rq.len).rev() {
            rq.tasks[i + 1] = rq.tasks[i];
        }
        rq.tasks[idx] = task;
        rq.len += 1;
        rq.current_idx = rq
            .active_tasks()
            .iter()
            .position(|&t| !current.is_null() && t == current)
            .unwrap_or(0);
    });
}

#[cfg(test)]
fn current_cpu_index() -> usize {
    0
}

#[cfg(not(test))]
fn current_cpu_index() -> usize {
    // Skip the LAPIC MMIO read (a VM-exit on VBox) when only the BSP is online.
    if crate::arch::x86::kernel::smp::AP_READY_COUNT.load(Ordering::Acquire) == 0 {
        return 0;
    }
    let cpu = unsafe { apic::id() } as usize;
    cpu.min(MAX_CPUS - 1)
}

#[inline]
pub fn current_cpu() -> u32 {
    current_cpu_index() as u32
}

#[inline]
pub fn production_smp_scheduler_enabled() -> bool {
    PRODUCTION_SCHED_ENABLED.load(Ordering::Acquire)
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Return the task currently running on this CPU.
///
/// # Safety
/// Must be called after `sched_init()`.  The returned pointer is valid as long
/// as no context switch occurs while the pointer is held (safe in cooperative
/// scheduling because only explicit `schedule()` calls switch tasks).
pub unsafe fn get_current() -> *mut TaskStruct {
    #[cfg(test)]
    {
        return TEST_CURRENT_TASK.with(|current| current.get());
    }
    #[cfg(not(test))]
    {
        let cpu = current_cpu_index();
        unsafe { CURRENT_TASK[cpu] }
    }
}

/// Return whether the task running on this CPU has been asked to reschedule.
///
/// The BSP idle loop uses this with local IRQs disabled for the final
/// check-before-halt, matching Linux's `do_idle()` contract: a wakeup observed
/// before `sti; hlt` must cause another scheduler pass instead of entering
/// idle with runnable work queued.
#[inline]
pub fn current_needs_resched() -> bool {
    let current = unsafe { get_current() };
    !current.is_null()
        && unsafe { (*current).thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED != 0 }
}

/// Set the current task for this CPU.
///
/// Called from `__switch_to` after the stack swap — at that point we are
/// already executing on `next`'s stack.
///
/// # Safety
/// Must only be called from `__switch_to` while interrupts are disabled.
pub unsafe fn set_current(task: *mut TaskStruct) {
    #[cfg(test)]
    {
        clear_legacy_run_queue_for_tests();
        TEST_CURRENT_TASK.with(|current| current.set(task));
    }
    #[cfg(not(test))]
    {
        let cpu = current_cpu_index();
        unsafe { set_current_on_cpu(cpu, task) };
    }
}

/// Install the current task in an explicitly identified logical-CPU slot.
///
/// Linux's AP bring-up writes its per-CPU `current_task` before marking that
/// CPU online.  It cannot discover the slot through the online mask at that
/// point, so `sched_init_ap()` passes the already-known logical CPU directly.
#[cfg(not(test))]
unsafe fn set_current_on_cpu(cpu: usize, task: *mut TaskStruct) {
    let cpu = cpu.min(MAX_CPUS - 1);
    unsafe {
        CURRENT_TASK[cpu] = task;
    }
    if cpu == 0 {
        crate::arch::x86::kernel::cpu::common::set_linux_current_task(task);
    }
}

#[inline]
unsafe fn clear_need_resched(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    unsafe {
        (*task).thread_info.flags &= !crate::kernel::task::TIF_NEED_RESCHED;
    }
}

#[inline]
unsafe fn set_need_resched(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    unsafe {
        (*task).thread_info.flags |= crate::kernel::task::TIF_NEED_RESCHED;
    }
}

/// Record an autoreaped outgoing task for post-switch reclamation.
///
/// Requiring both Linux states keeps unrelated kthreads which use TASK_DEAD
/// but have no EXIT_DEAD lifecycle out of the process-release path.
unsafe fn prepare_task_switch_release(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    let task_state = unsafe { (*task).__state.load(Ordering::Acquire) };
    let exit_state = unsafe { (*task).m26.exit_state };
    if task_state != crate::kernel::task::task_state::TASK_DEAD
        || exit_state != crate::kernel::task::task_state::EXIT_DEAD
    {
        return;
    }

    let slot = &DEFERRED_TASK_RELEASE[current_cpu_index()];
    let installed = slot.compare_exchange(
        core::ptr::null_mut(),
        task,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
    debug_assert!(
        installed.is_ok(),
        "post-switch task release slot must be empty"
    );
}

/// Linux `finish_task_switch()`'s final TASK_DEAD ownership drop.
///
/// This runs on the incoming task's kernel stack. `prev` is the actual task
/// returned by `__switch_to`, not the stale local restored with that stack.
/// Only CLONE_THREAD autoreap places a pointer in this slot.
pub unsafe fn finish_task_switch(prev: *mut TaskStruct) {
    let slot = &DEFERRED_TASK_RELEASE[current_cpu_index()];
    let deferred = slot.swap(core::ptr::null_mut(), Ordering::AcqRel);
    if !deferred.is_null() && deferred != prev {
        // Never free an ownership token that does not match the architecture
        // switch result. Preserve it for diagnosis rather than guessing.
        slot.store(deferred, Ordering::Release);
    }

    // A remote exit_group can publish TASK_DEAD after the outgoing CPU passed
    // prepare_task_switch_release() but before the physical stack switch. Read
    // the state again here, exactly where Linux finish_task_switch() samples
    // prev->__state. Keep on_cpu set until release finishes so another CPU can
    // never mistake this still-owned stack for an immediately freeable task.
    let autoreap = !prev.is_null()
        && unsafe {
            (*prev).__state.load(Ordering::Acquire) == crate::kernel::task::task_state::TASK_DEAD
                && (*prev).m26.exit_state == crate::kernel::task::task_state::EXIT_DEAD
        };
    if deferred == prev || autoreap {
        unsafe {
            crate::kernel::exit::release_task(prev);
        }
    } else if !prev.is_null() {
        unsafe {
            (*prev).m29.on_cpu = 0;
        }
    }
}

/// Linux `schedule_tail(prev)`: first operation on a newly-forked task's
/// synthetic return path.
pub unsafe extern "C" fn schedule_tail(prev: *mut TaskStruct) {
    unsafe {
        finish_task_switch(prev);
    }
}

#[inline]
fn task_allowed_on_cpu(task: *mut TaskStruct, cpu: u32) -> bool {
    if task.is_null() {
        return false;
    }
    unsafe { (*task).m29.cpus_mask.test(cpu) }
}

fn allowed_cpu_count(task: *mut TaskStruct) -> u32 {
    if task.is_null() {
        return 0;
    }
    unsafe { (*task).m29.cpus_mask.weight() }
}

fn idlest_allowed_cpu(mask: crate::kernel::sched::entity::CpuMask) -> u32 {
    let mut best_cpu = 0;
    let mut best_load = u32::MAX;
    for cpu in 0..MAX_CPUS as u32 {
        if !mask.test(cpu) {
            continue;
        }
        let load = rq::rq_nr_running(cpu).unwrap_or(u32::MAX);
        if load < best_load {
            best_cpu = cpu;
            best_load = load;
        }
    }
    best_cpu
}

unsafe fn task_class(task: *mut TaskStruct) -> *const class::SchedClass {
    if task.is_null() {
        return core::ptr::null();
    }
    unsafe { (*task).m29.sched_class }
}

#[inline]
unsafe fn task_runnable(task: *mut TaskStruct) -> bool {
    if task.is_null() {
        return false;
    }
    let state = unsafe { (*task).__state.load(Ordering::Acquire) };
    state & crate::kernel::task::task_state::NON_RUNNABLE_MASK == 0
}

#[inline]
unsafe fn task_has_switch_frame(task: *mut TaskStruct) -> bool {
    if task.is_null() {
        return false;
    }
    let sp = unsafe { (*task).thread.sp };
    if sp == 0 || sp & 0x7 != 0 {
        return false;
    }

    let stack_top = unsafe { (*task).stack as u64 };
    if stack_top == 0 {
        return false;
    }

    let Some(stack_bottom) = stack_top.checked_sub(KTHREAD_STACK_SIZE as u64) else {
        return false;
    };
    let Some(frame_end) = sp.checked_add(SWITCH_FRAME_BYTES as u64) else {
        return false;
    };
    sp >= stack_bottom && frame_end <= stack_top
}

#[inline]
unsafe fn task_can_switch_to(task: *mut TaskStruct) -> bool {
    unsafe { task_runnable(task) && task_has_switch_frame(task) }
}

/// Initialise scheduler-owned fields for a newly forked task.
///
/// This is the Lupos equivalent of Linux `sched_fork()`: inherit scheduling
/// policy from the parent, reset runqueue-owned state, and default to CFS when
/// the parent was an early boot task without a class yet.
pub unsafe fn init_task_sched_from_parent(parent: *mut TaskStruct, child: *mut TaskStruct) {
    if child.is_null() {
        return;
    }

    let mut sched = M29SchedFields::zeroed();
    if !parent.is_null() {
        unsafe {
            sched.prio = (*parent).m29.prio;
            sched.static_prio = (*parent).m29.static_prio;
            sched.normal_prio = (*parent).m29.normal_prio;
            sched.rt_priority = (*parent).m29.rt_priority;
            sched.sched_class = (*parent).m29.sched_class;
            sched.policy = (*parent).m29.policy;
            sched.cpus_mask = (*parent).m29.cpus_mask;
        }
    }
    if sched.sched_class.is_null()
        || sched.sched_class == &idle::IDLE_SCHED_CLASS as *const class::SchedClass
    {
        sched.sched_class = &fair::FAIR_SCHED_CLASS as *const class::SchedClass;
    }
    if sched.cpus_mask.weight() == 0 {
        sched.cpus_mask = entity::CpuMask::all();
    }
    sched.nr_cpus_allowed = sched.cpus_mask.weight() as i32;
    let nice = prio::prio_to_nice(sched.static_prio);
    sched.se.load.weight = prio::nice_to_weight(nice);
    sched.se.load.inv_weight = prio::nice_to_wmult(nice);

    unsafe {
        (*child).m29 = sched;
        (*child).m29.cpus_ptr = &(*child).m29.cpus_mask as *const _;
        (*child).m29.on_cpu = 0;
        (*child).m29.on_rq = 0;
        (*child).m29.se.on_rq = 0;
        (*child).m29.rt.on_rq = 0;
    }
}

pub(crate) unsafe fn enqueue_on_rq(cpu: u32, task: *mut TaskStruct, flags: u32) {
    if task.is_null() {
        return;
    }
    let class = unsafe { task_class(task) };
    if class.is_null() {
        return;
    }
    let _ = rq::with_rq(cpu, |rq| unsafe {
        if let Some(enqueue) = (*class).enqueue_task {
            enqueue(rq, task, flags);
        }
    });
}

pub(crate) unsafe fn dequeue_from_rq(cpu: u32, task: *mut TaskStruct, flags: u32) -> bool {
    if task.is_null() {
        return false;
    }
    let class = unsafe { task_class(task) };
    if class.is_null() {
        return false;
    }
    rq::with_rq(cpu, |rq| unsafe {
        if let Some(dequeue) = (*class).dequeue_task {
            dequeue(rq, task, flags)
        } else {
            false
        }
    })
    .unwrap_or(false)
}

/// Locate a pool-allocated task (BSP / static kthread) by PID.
///
/// Returns NULL when no match is found.  Used by `ptrace::sys_ptrace` to
/// resolve PIDs that don't live in the heap-task tracker.
pub fn find_pool_task_by_pid(pid: i32) -> *mut TaskStruct {
    let count = KTHREAD_COUNT.load(Ordering::Relaxed).min(MAX_KTHREADS);
    for i in 0..count {
        let task = unsafe { &raw mut TASK_POOL[i] };
        unsafe {
            if (*task).pid == pid {
                return task;
            }
        }
    }
    core::ptr::null_mut()
}

/// Visit every task in the static scheduler pool plus per-CPU idle tasks.
pub fn for_each_pool_task(mut f: impl FnMut(*mut TaskStruct)) {
    let count = KTHREAD_COUNT.load(Ordering::Relaxed).min(MAX_KTHREADS);
    for i in 0..count {
        let task = unsafe { &raw mut TASK_POOL[i] };
        f(task);
    }
    for cpu in 1..MAX_CPUS {
        let task = unsafe { &raw mut AP_IDLE_TASKS[cpu] };
        f(task);
    }
}

pub fn find_pool_task_by_tgid(tgid: i32) -> *mut TaskStruct {
    let mut found: *mut TaskStruct = core::ptr::null_mut();
    for_each_pool_task(|task| unsafe {
        if found.is_null() && !task.is_null() && (*task).tgid == tgid {
            found = task;
        }
    });
    found
}

pub fn request_reschedule(cpu: u32) {
    let current = rq::with_rq(cpu, |rq| rq.current).unwrap_or(core::ptr::null_mut());
    if current.is_null() {
        return;
    }
    unsafe {
        set_need_resched(current);
    }
    #[cfg(not(test))]
    if cpu != current_cpu() {
        unsafe {
            crate::arch::x86::kernel::idt::send_reschedule_ipi(cpu as u8);
        }
    }
}

/// Add a task to the global run queue.
///
/// # Safety
/// `task` must point to a fully-initialised `TaskStruct` that lives at least
/// as long as the scheduler runs.
pub unsafe fn enqueue_task(task: *mut TaskStruct) {
    if production_smp_scheduler_enabled() {
        let cpu = select_task_rq(task, current_cpu(), class::ENQUEUE_WAKEUP);
        unsafe {
            enqueue_on_rq(cpu, task, class::ENQUEUE_WAKEUP);
        }
        request_reschedule(cpu);
        return;
    }
    legacy_place_after_current(task);
    unsafe {
        set_need_resched(get_current());
    }
}

/// Remove a task from the run queue.
///
/// Called from `kthread_stop` once the kthread has marked itself TASK_DEAD,
/// so that the round-robin scheduler does not try to reschedule a halted task.
///
/// # Safety
/// `task` must have been previously passed to `enqueue_task`.
pub unsafe fn dequeue_task(task: *mut TaskStruct) {
    if production_smp_scheduler_enabled() {
        let cpu = unsafe { (*task).thread_info.cpu };
        let _ = unsafe { dequeue_from_rq(cpu, task, class::DEQUEUE_SLEEP) };
        return;
    }
    with_legacy_run_queue(|rq| {
        rq.normalize_legacy();
        let pos = rq.active_tasks().iter().position(|&t| t == task);
        if let Some(pos) = pos {
            // Compact the array by shifting entries down.
            for i in pos..rq.len - 1 {
                rq.tasks[i] = rq.tasks[i + 1];
            }
            rq.len -= 1;
            let new_len = rq.len;
            rq.tasks[new_len] = core::ptr::null_mut();
            // Adjust current_idx if needed.
            if rq.current_idx > pos && rq.current_idx > 0 {
                rq.current_idx -= 1;
            } else if rq.current_idx >= rq.len && rq.len > 0 {
                rq.current_idx = 0;
            }
        }
    });
}

/// Cooperative legacy scheduler used by early uniprocessor milestones.
///
/// Returns `true` when no other task was runnable and no switch happened --
/// i.e. the whole single-CPU cooperative system is idle right now. The
/// scheduler itself never halts the CPU; it only reports idleness so a
/// caller that knows IRQs are enabled (`schedule_with_irqs_enabled()`) can
/// decide to halt. See that function for why the halt must not live here
/// or in any individual caller's own poll loop.
pub unsafe fn legacy_schedule() -> bool {
    let current = unsafe { get_current() };
    if current.is_null() {
        return false;
    }

    enum Pick {
        Idle,
        CurrentNotQueued,
        Switch(*mut TaskStruct, *mut TaskStruct),
    }

    let pick = with_legacy_run_queue(|rq| {
        rq.normalize_legacy();
        if rq.len <= 1 {
            unsafe {
                clear_need_resched(current);
            }
            return Pick::Idle;
        }

        // Find the current task's position.
        let Some(pos) = rq.active_tasks().iter().position(|&t| t == current) else {
            unsafe {
                clear_need_resched(current);
            }
            return Pick::CurrentNotQueued;
        };

        // Advance to the next runnable task in round-robin order.
        // Skip tasks whose `__state` matches the non-runnable mask
        // (sleeping, stopped, traced, zombie, dead, parked, new).
        // M26: this is what makes wait4/exit_state-based blocking work
        // under the cooperative scheduler.
        let mut next_pos = (pos + 1) % rq.len;
        let mut found = false;
        for _ in 0..rq.len {
            let cand = rq.tasks[next_pos];
            if !cand.is_null() && unsafe { task_can_switch_to(cand) } {
                found = true;
                break;
            }
            next_pos = (next_pos + 1) % rq.len;
        }
        if !found {
            unsafe {
                if task_runnable(current) {
                    clear_need_resched(current);
                }
            }
            return Pick::Idle;
        }
        rq.current_idx = next_pos;
        Pick::Switch(rq.tasks[pos], rq.tasks[next_pos])
    });
    let (prev, next) = match pick {
        Pick::Idle => return true,
        Pick::CurrentNotQueued => return false,
        Pick::Switch(prev, next) => (prev, next),
    };

    if prev == next {
        unsafe {
            clear_need_resched(current);
        }
        return true; // round-robin landed back on ourselves; idle
    }

    // Match Linux __schedule(): the reschedule request belongs to the task
    // that entered the scheduler.  Clearing it on `next` leaves `prev` with a
    // stale TIF_NEED_RESCHED, so it schedules again at every return-to-user
    // boundary after it next runs.
    unsafe {
        clear_need_resched(prev);
    }
    crate::kernel::rcu::tasks_rcu_qs();
    crate::kernel::rcu::rcu_qs();

    // Perform the context switch.  This call will not return until `prev` is
    // scheduled again (another call to schedule() or the initial stack setup
    // for a new thread).
    unsafe {
        seed_current_task_stack(prev);
        record_switch_attempt(prev, next);
        prepare_switch_to_task(next);
        prepare_task_switch_release(prev);
        let last = __switch_to_asm(prev, next);
        finish_task_switch(last);
    }
    false
}

unsafe fn pick_next_task(rq: &mut rq::Rq) -> *mut TaskStruct {
    let class_order = [
        &stop::STOP_SCHED_CLASS as *const class::SchedClass,
        &deadline::DL_SCHED_CLASS as *const class::SchedClass,
        &rt::RT_SCHED_CLASS as *const class::SchedClass,
        &fair::FAIR_SCHED_CLASS as *const class::SchedClass,
        &idle::IDLE_SCHED_CLASS as *const class::SchedClass,
    ];
    for class in class_order {
        if let Some(pick) = unsafe { (*class).pick_next_task } {
            let next = unsafe { pick(rq) };
            if !next.is_null() {
                return next;
            }
        }
    }
    rq.idle
}

/// Production per-CPU scheduler path used once APs join normal scheduling.
pub unsafe fn __schedule() {
    let cpu = current_cpu();
    let current = unsafe { get_current() };
    if current.is_null() {
        return;
    }

    let (prev, next) = rq::with_rq(cpu, |this_rq| unsafe {
        this_rq.update_rq_clock();

        let prev = current;
        let prev_class = task_class(prev);
        if !prev_class.is_null() {
            if let Some(put_prev) = (*prev_class).put_prev_task {
                put_prev(this_rq, prev);
            }
        }

        let picked = pick_next_task(this_rq);
        let next = if !picked.is_null() && task_can_switch_to(picked) {
            picked
        } else {
            prev
        };
        clear_need_resched(prev);
        if !next.is_null() && next != prev {
            (*next).m29.on_cpu = 1;
            (*next).thread_info.cpu = cpu;
            (*next).m29.recent_used_cpu = cpu as i32;
            (*next).m29.wake_cpu = cpu as i32;
        }
        this_rq.current = next;
        (prev, next)
    })
    .unwrap_or((current, current));

    if prev == next || next.is_null() {
        unsafe {
            clear_need_resched(current);
            (*current).m29.on_cpu = 1;
        }
        return;
    }

    crate::kernel::rcu::tasks_rcu_qs();
    crate::kernel::rcu::rcu_qs();
    unsafe {
        seed_current_task_stack(prev);
        record_switch_attempt(prev, next);
        prepare_switch_to_task(next);
        prepare_task_switch_release(prev);
        let last = __switch_to_asm(prev, next);
        finish_task_switch(last);
    }
}

/// Main scheduler entry point.
///
/// The legacy global queue remains active until the production SMP scheduler
/// has at least one AP online.  From that point on, all runnable placement and
/// switching happens via per-CPU runqueues.
///
/// Returns `true` when the cooperative (legacy) path found the system idle
/// (see `legacy_schedule()`). Always `false` under the production SMP
/// scheduler, which has its own idle scheduling class.
pub unsafe fn schedule() -> bool {
    crate::kernel::watchdog::touch_softlockup_watchdog_sched();
    #[cfg(not(test))]
    crate::kernel::softirq::do_softirq();
    if production_smp_scheduler_enabled() {
        unsafe { __schedule() };
        false
    } else {
        unsafe { legacy_schedule() }
    }
}

/// Schedule from normal blocking syscall context with IRQs open.
///
/// Linux enters `schedule()` for interruptible syscall sleeps with local IRQs
/// enabled. Lupos' current x86 switch path saves callee-saved registers, but
/// not RFLAGS, so a task that yields with IF clear can resume another task with
/// timer interrupts masked. Keep normal user wait paths explicitly IRQ-open on
/// both sides of the context switch.
///
/// If `schedule()` reports the system idle (no other task runnable), halt
/// the CPU until the next interrupt instead of returning straight back to a
/// busy-polling caller. This is the single chokepoint for halting: every
/// `sys_epoll_wait`/console/wait-loop caller funnels through here, and the
/// scheduler has *just* confirmed, under the runqueue lock, that nothing
/// else on this single CPU is runnable -- so halting cannot delay other
/// work the way halting inside one caller's own poll loop did (that
/// stalled the whole cooperative system for up to a full tick per idle
/// service, since nothing preempts a halted task here). Halting only when
/// truly idle is also what lets the host (under TCG) run QEMU's I/O thread
/// promptly; a tight `spin_loop()` with no halt anywhere starves it and
/// drops bytes on bursty serial input.
/// Reentrancy guard for `pump_driver_abi_events_for_wait`.
///
/// A driver poller's completion callback (e.g. libata's `ahci_handle_port_intr`
/// completing an internal IDENTIFY qc) can itself call back into a cooperative
/// wait. Without this guard that path would re-enter `poll_driver_abi_events`
/// and deadlock on its (non-recursive) poller-list lock.
static DRIVER_POLL_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Surface pending Linux-driver hardware completions when the CPU is otherwise
/// idle, returning whether any event was handled.
///
/// Lupos lacks a native IRQ wakeup path for some Linux-built bus/core
/// completions (notably AHCI/libata during async port probe: the IDENTIFY qc
/// completes via `ahci_handle_port_intr`, but the HBA interrupt may not be
/// wired). The block-queue and root-device wait loops already pump these
/// pollers, but a driver thread that blocks on `wait_for_completion` /
/// `schedule_timeout` before any block request exists would sleep forever.
///
/// Pumping happens ONLY from the `schedule()`-idle path on purpose: that point
/// is reached precisely when every runnable task has already yielded, so the
/// driver-completion callbacks never run nested on another task's mid-syscall
/// kernel stack (e.g. pid 1 inside `execve`, or a `scsi_eh` thread mid error
/// handling). Returns `true` if progress was made so the caller re-runs the
/// scheduler instead of halting.
#[cfg(not(test))]
fn pump_driver_abi_events_on_idle() -> bool {
    if DRIVER_POLL_ACTIVE.swap(true, Ordering::AcqRel) {
        return false;
    }
    let handled = crate::linux_driver_abi::poll_driver_abi_events();
    DRIVER_POLL_ACTIVE.store(false, Ordering::Release);
    handled > 0
}

/// Reschedule a *runnable* task that is merely yielding for fairness (e.g.
/// `TIF_NEED_RESCHED` set on syscall return), giving any peer a turn — but never
/// halting. Unlike [`schedule_with_irqs_enabled`], if no other task is runnable
/// this returns immediately so the current task keeps running, rather than
/// halting the CPU for a tick. Halting a still-runnable task wastes ~a tick per
/// reschedule; with a slow per-syscall transport (VirtualBox's VM-exit-heavy
/// AHCI/port I/O) and a single busy task (systemd scanning units at boot) that
/// is half the CPU. The genuine-sleep paths still use `schedule_with_irqs_enabled`
/// and halt correctly. Only meaningful under the cooperative (single-CPU)
/// scheduler; the production SMP path has its own idle class.
pub unsafe fn reschedule_runnable() {
    crate::kernel::locking::local_irq_enable();
    let _ = unsafe { schedule() };
    crate::kernel::locking::local_irq_enable();
}

pub unsafe fn schedule_with_irqs_enabled() {
    #[cfg(test)]
    {
        return;
    }
    #[cfg(not(test))]
    {
        loop {
            crate::kernel::locking::local_irq_enable();
            let idle = unsafe { schedule() };
            crate::kernel::locking::local_irq_enable();
            let current = unsafe { get_current() };
            if current.is_null() || unsafe { task_runnable(current) } {
                return;
            }

            if idle && pump_driver_abi_events_on_idle() {
                continue;
            }

            // Softirqs raised by hard IRQs (e.g. a NAPI NET_RX from a device
            // interrupt that arrived while every task slept) run at irq_exit
            // on Linux. Lupos interrupt gates keep IRQs disabled in the ISR,
            // so this task-context chokepoint is the deferred execution point
            // — without it, a single blocked task waits on a wakeup that only
            // the undrained softirq could deliver.
            if crate::kernel::softirq::local_softirq_pending() != 0 {
                crate::kernel::softirq::do_softirq();
                continue;
            }

            // Drop the tick's stale reschedule request before closing the
            // check/halt race. Any real wake or newly-runnable peer arriving
            // after this point sets it again.
            unsafe {
                clear_need_resched(current);
            }
            crate::kernel::locking::local_irq_disable();
            let woke = unsafe { task_runnable(current) };
            let need_resched = unsafe {
                (*current).thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED != 0
            };
            if woke || need_resched {
                crate::kernel::locking::local_irq_enable();
                continue;
            }

            unsafe {
                // Linux's idle loop disables IRQs, performs the final
                // need-resched check, then uses sti;hlt so a pending interrupt
                // cannot be lost in between. Return from HLT is only a reason
                // to re-run schedule; the sleeping syscall does not resume
                // until its task is genuinely TASK_RUNNING.
                core::arch::asm!("sti; hlt", options(nostack));
            }
        }
    }
}

// ── Kernel thread creation ───────────────────────────────────────────────────

/// Kernel thread entry point signature.
///
/// Matches Linux `kthread_fn_t`: takes one opaque argument and never returns.
pub type KthreadFn = unsafe extern "C" fn(arg: *mut core::ffi::c_void) -> !;

/// Internal: allocate a pool slot and set up the stack frame.
///
/// Does NOT assign a PID — the caller (`kthread_create` or
/// `kthread::kthread_create_on_node`) assigns the PID after this returns.
///
/// # Initial stack layout
///
/// ```text
/// [stack_top - 8]   entry_fn            ← __switch_to's ret lands here
/// [stack_top - 16]  rbp = 0
/// [stack_top - 24]  rbx = arg           ← entry_fn reads this
/// [stack_top - 32]  r12 = func          ← entry_fn calls this via R12
/// [stack_top - 40]  r13 = 0
/// [stack_top - 48]  r14 = 0
/// [stack_top - 56]  r15 = 0             ← task->thread.sp points here
/// ```
///
/// # Safety
/// - `func` is stored in R12; called by `entry_fn` when first scheduled.
/// - `name` must be a 16-byte array (TASK_COMM_LEN).
/// - Returns NULL if the static pool is exhausted.
pub(crate) unsafe fn sched_alloc_kthread_raw(
    func: KthreadFn,
    arg: *mut core::ffi::c_void,
    name: &[u8; 16],
    entry_fn: u64,
) -> *mut TaskStruct {
    let idx = KTHREAD_COUNT.fetch_add(1, Ordering::Relaxed);
    if idx >= MAX_KTHREADS {
        KTHREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
        return core::ptr::null_mut();
    }

    let task = unsafe { &raw mut TASK_POOL[idx] };
    let stack_base = unsafe { KTHREAD_STACKS[idx].0.as_mut_ptr() };
    let stack_top = unsafe { stack_base.add(KTHREAD_STACK_SIZE) };

    unsafe {
        (*task).thread_info = ThreadInfo {
            flags: 0,
            syscall_work: 0,
            status: 0,
            cpu: 0,
        };
        (*task).__state = core::sync::atomic::AtomicU32::new(0); // TASK_RUNNING
        (*task).saved_state = 0;
        (*task).stack = stack_top as *mut _;
        // PID is 0 here; caller sets the real PID after this function returns.
        (*task).pid = 0;
        (*task).tgid = 0;
        (*task).comm.copy_from_slice(name);

        // Initialise the M29 scheduler block (CFS by default, nice 0).
        (*task).m29 = crate::kernel::task::M29SchedFields::zeroed();
        (*task).m29.sched_class = &fair::FAIR_SCHED_CLASS as *const class::SchedClass;
        (*task).m29.se.load.weight = prio::NICE_0_LOAD;
        (*task).m29.se.load.inv_weight = prio::SCHED_PRIO_TO_WMULT[20]; // nice 0

        let sp = stack_top as *mut u64;
        sp.sub(1).write(entry_fn); // return address → entry_fn
        sp.sub(2).write(0); // saved RBP
        sp.sub(3).write(arg as u64); // saved RBX = arg
        sp.sub(4).write(func as u64); // saved R12 = func
        sp.sub(5).write(0); // saved R13
        sp.sub(6).write(0); // saved R14
        sp.sub(7).write(0); // saved R15

        (*task).thread = ThreadStruct {
            tls_array: [DescStruct(0); 3],
            sp: sp.sub(7) as u64,
            es: 0,
            ds: 0,
            fsindex: 0,
            gsindex: 0,
            _pad0: 0,
            fsbase: 0,
            gsbase: 0,
            pkru: 0,
            _pad1: 0,
        };
    }

    task
}

/// Create a kernel thread from the static pool.
///
/// M21-compatible public API: takes a `fn -> !` thread function and returns
/// a `*mut TaskStruct` (NOT yet enqueued).  Since M22, the task receives a
/// real PID from `alloc_pid` instead of the former `(idx + 2)` placeholder.
///
/// # Safety
/// - `func` must never return (it is a `fn -> !` function).
/// - `name` must be a 16-byte array (TASK_COMM_LEN).
/// - Returns NULL if the pool is exhausted.
pub unsafe fn kthread_create(
    func: KthreadFn,
    arg: *mut core::ffi::c_void,
    name: &[u8; 16],
) -> *mut TaskStruct {
    let task = unsafe { sched_alloc_kthread_raw(func, arg, name, kthread_entry_stub as u64) };
    if task.is_null() {
        return core::ptr::null_mut();
    }
    // Assign a real PID from the init namespace.
    match alloc_pid(&INIT_PID_NS, None) {
        Ok(kpid) => {
            let nr = kpid.numbers[0].nr;
            // Leak the KPid box — cleanup deferred to M26 (do_exit / put_pid).
            let _ = alloc::boxed::Box::into_raw(kpid);
            unsafe {
                (*task).pid = nr;
                (*task).tgid = nr;
            }
        }
        Err(_) => {
            // PID namespace exhausted — give back the pool slot.
            KTHREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
            return core::ptr::null_mut();
        }
    }
    task
}

/// Return the address of `kthread_entry_stub` as a raw `u64`.
///
/// Used by `kthread.rs` and `fork.rs` to build initial stack frames without
/// taking a direct function pointer (which would require them to name the
/// type `KthreadFn`).
pub fn kthread_entry_stub_addr() -> u64 {
    kthread_entry_stub as u64
}

/// Naked stub invoked when a kernel thread is first scheduled.
///
/// At entry (after `__switch_to_asm` pops six callee-saved registers and
/// `__switch_to` returns):
///   - R12 = `func` pointer (set by `kthread_create`)
///   - RBX = `arg` value     (set by `kthread_create`)
///
/// We move `arg` into RDI (first System V argument register) then call `func`.
/// `func` must never return; if it does we halt.
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn kthread_entry_stub() -> ! {
    core::arch::naked_asm!(
        // R12 = func, RBX = arg (both are callee-saved, set by kthread_create)
        // RAX = prev from __switch_to. Every freshly-forked task, including a
        // kernel thread, must finish the outgoing switch before entering its
        // body.
        "mov rdi, rax",
        "call {schedule_tail}",
        "mov rdi, rbx", // arg → first argument register
        "call r12",     // func(arg)
        // Should never reach here; halt if the thread returns.
        "2: hlt",
        "jmp 2b",
        schedule_tail = sym schedule_tail,
    );
}

// ── BSP initialisation ───────────────────────────────────────────────────────

/// Initialise the scheduler for the BSP.
///
/// Creates a minimal `TaskStruct` for the BSP (the currently-running context)
/// and sets it as `current` for the BSP's CPU.  The BSP task's `thread.sp`
/// is written by `__switch_to_asm` the first time the BSP calls `schedule()`.
///
/// Must be called after LAPIC is initialised (so `apic::id()` returns a valid
/// CPU ID) and before any call to `schedule()` or `kthread_create()`.
///
/// # Safety
/// Must be called exactly once, from `kernel_main`, before interrupts are
/// re-enabled.
pub unsafe fn sched_init() {
    // BSP task lives at slot 0 of the pool.
    let bsp_task = unsafe { &raw mut TASK_POOL[0] };

    unsafe {
        (*bsp_task).thread_info = ThreadInfo {
            flags: 0,
            syscall_work: 0,
            status: 0,
            cpu: 0,
        };
        (*bsp_task).__state = core::sync::atomic::AtomicU32::new(0);
        (*bsp_task).pid = 0; // swapper / idle
        (*bsp_task).tgid = 0;
        (*bsp_task).comm = *b"swapper/0\0\0\0\0\0\0\0";

        // M29 scheduler block — BSP idle/swapper task runs under the idle class.
        (*bsp_task).m29 = crate::kernel::task::M29SchedFields::zeroed();
        (*bsp_task).m29.sched_class = &idle::IDLE_SCHED_CLASS as *const class::SchedClass;
        (*bsp_task).m29.policy = prio::SCHED_NORMAL;
        (*bsp_task).m29.cpus_ptr = &(*bsp_task).m29.cpus_mask as *const _;
        (*bsp_task).thread_info.cpu = 0;

        // Linux's idle task is a real task and must eventually have a real
        // `task_top_of_stack()` for `update_task_stack()` in
        // `arch/x86/kernel/process_64.c`. The BSP starts on the boot stack, so
        // we seed `stack` from the live RSP immediately before the first switch
        // away instead of inventing a separate Rust idle stack.
        (*bsp_task).stack = core::ptr::null_mut();

        // thread.sp is intentionally left at 0; it will be filled by
        // __switch_to_asm the first time schedule() is called from the BSP.
        (*bsp_task).thread = ThreadStruct {
            tls_array: [DescStruct(0); 3],
            sp: 0, // filled on first switch away
            es: 0,
            ds: 0,
            fsindex: 0,
            gsindex: 0,
            _pad0: 0,
            fsbase: 0,
            gsbase: 0,
            pkru: 0,
            _pad1: 0,
        };
    }

    // Set BSP task as current.
    unsafe {
        set_current(bsp_task);
    }

    // Enqueue the BSP task so it participates in round-robin scheduling.
    unsafe {
        enqueue_task(bsp_task);
    }

    // Reserve slot 0 for the BSP task so kthread_create() starts at slot 1.
    KTHREAD_COUNT.store(1, Ordering::Relaxed);

    // M29: bring up per-CPU runqueues and the sched_domain hierarchy.
    rq::init_rqs();
    topology::init_sched_domains();

    // Wire BSP runqueue: idle = swapper/0, current = swapper/0.
    let _ = rq::with_rq(0, |rq0| {
        rq0.idle = bsp_task;
        rq0.current = bsp_task;
    });
    SCHED_ACTIVE_CPUS.store(1, Ordering::Release);
    SCHED_ONLINE_CPUS.store(1, Ordering::Release);
    crate::kernel::cpuhotplug::reset_cpu_maps();
}

/// Bring an AP into the production scheduler as that CPU's idle task.
///
/// The AP continues executing on its trampoline-provided kernel stack until it
/// first switches away; from that point forward `__switch_to_asm` owns the
/// saved `thread.sp` exactly like the BSP path.
pub unsafe fn sched_init_ap(cpu: u32) -> *mut TaskStruct {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    let idle_task = unsafe { &raw mut AP_IDLE_TASKS[cpu] };

    unsafe {
        (*idle_task).thread_info = ThreadInfo {
            flags: 0,
            syscall_work: 0,
            status: 0,
            cpu: cpu as u32,
        };
        (*idle_task).__state = core::sync::atomic::AtomicU32::new(0);
        (*idle_task).pid = 0;
        (*idle_task).tgid = 0;
        (*idle_task).comm = *b"swapper/ap\0\0\0\0\0\0";
        (*idle_task).m26 = crate::kernel::task::M26Fields::zeroed();
        (*idle_task).m29 = crate::kernel::task::M29SchedFields::zeroed();
        (*idle_task).m29.sched_class = &idle::IDLE_SCHED_CLASS as *const class::SchedClass;
        (*idle_task).m29.policy = prio::SCHED_NORMAL;
        (*idle_task).m29.cpus_mask = crate::kernel::sched::entity::CpuMask::one(cpu as u32);
        (*idle_task).m29.cpus_ptr = &(*idle_task).m29.cpus_mask as *const _;
        (*idle_task).m29.nr_cpus_allowed = 1;
        (*idle_task).m29.on_cpu = 1;
        // Like Linux `init_idle()`, AP idle tasks are real per-CPU tasks. The
        // trampoline stack becomes their saved task stack on first switch-away.
        (*idle_task).stack = core::ptr::null_mut();
        (*idle_task).thread = ThreadStruct {
            tls_array: [DescStruct(0); 3],
            sp: 0,
            es: 0,
            ds: 0,
            fsindex: 0,
            gsindex: 0,
            _pad0: 0,
            fsbase: 0,
            gsbase: 0,
            pkru: 0,
            _pad1: 0,
        };
    }

    #[cfg(not(test))]
    unsafe {
        // AP_READY_COUNT is intentionally published only after the AP is fully
        // initialized, so select the per-CPU current slot from the explicit
        // logical CPU rather than current_cpu_index()'s pre-online fast path.
        set_current_on_cpu(cpu, idle_task);
    }
    #[cfg(test)]
    unsafe {
        set_current(idle_task);
    }
    let _ = rq::with_rq(cpu as u32, |this_rq| {
        this_rq.idle = idle_task;
        this_rq.current = idle_task;
    });
    SCHED_ACTIVE_CPUS.fetch_or(1u64 << cpu, Ordering::AcqRel);
    SCHED_ONLINE_CPUS.fetch_add(1, Ordering::AcqRel);
    crate::kernel::cpuhotplug::set_cpu_online(cpu as u32, true);
    PRODUCTION_SCHED_ENABLED.store(true, Ordering::Release);
    idle_task
}

// ── M29: scheduler_tick() — invoked from the LAPIC timer ISR ─────────────────

/// Periodic scheduler tick.  Mirrors `vendor/linux/kernel/sched/core.c::scheduler_tick`.
///
/// Called from `apic_timer::on_tick()` after the global tick counter has been
/// incremented.  Updates the current task's class accounting and triggers
/// load balancing when due.  Under the cooperative scheduler this only sets
/// `TIF_NEED_RESCHED` — the actual context switch happens at the next explicit
/// `schedule()` call.
pub fn scheduler_tick() {
    let cur = unsafe { get_current() };
    if cur.is_null() {
        return;
    }
    if !production_smp_scheduler_enabled() {
        unsafe {
            set_need_resched(cur);
        }
        return;
    }

    let cpu = unsafe { apic::id() } as u32;
    let mut run_balance = false;
    let _ = rq::with_rq(cpu, |this_rq| {
        this_rq.update_rq_clock();
        // Dispatch through the task's sched_class.
        unsafe {
            let class = (*cur).m29.sched_class;
            if !class.is_null() {
                if let Some(tick) = (*class).task_tick {
                    tick(this_rq, cur, true);
                }
            }
        }
        // Periodic load balance (M31).
        if this_rq.clock.saturating_sub(this_rq.last_balance_tick)
            >= balance::DEFAULT_BALANCE_INTERVAL_TICKS * 25_000_000
        {
            this_rq.last_balance_tick = this_rq.clock;
            run_balance = production_smp_scheduler_enabled();
        }
    });
    if run_balance {
        balance::run_periodic_balance(cpu);
    }
}

/// Wake-up a freshly-forked task — installs `sched_class` defaults and
/// inserts the entity into the appropriate per-CPU runqueue.
///
/// Mirrors Linux `wake_up_new_task(p)`.
pub unsafe fn wake_up_new_task(p: *mut TaskStruct) {
    if p.is_null() {
        return;
    }
    unsafe {
        // If the parent didn't pre-set a class, default to CFS.
        if (*p).m29.sched_class.is_null() {
            (*p).m29.sched_class = &fair::FAIR_SCHED_CLASS as *const class::SchedClass;
        }
        let class = (*p).m29.sched_class;
        if let Some(fork) = (*class).task_fork {
            fork(p);
        }
        let cpu = select_task_rq(p, current_cpu(), class::ENQUEUE_INITIAL);
        (*p).thread_info.cpu = cpu;
        (*p).m29.recent_used_cpu = cpu as i32;
        (*p).m29.wake_cpu = cpu as i32;
        enqueue_on_rq(cpu, p, class::ENQUEUE_INITIAL);
        request_reschedule(cpu);
    }
}

pub fn select_task_rq(p: *mut TaskStruct, prev_cpu: u32, flags: u32) -> u32 {
    if p.is_null() {
        return prev_cpu;
    }
    let allowed = unsafe { (*p).m29.cpus_mask };
    if allowed.weight() == 0 {
        return prev_cpu;
    }
    // Lupos's APs do not run the scheduler: `ap_main` brings each AP up and then
    // spins in a `hlt` idle loop (it never calls `schedule()`), so only the BSP
    // (CPU 0) ever executes tasks. Load-balancing a waking task onto an "idlest"
    // AP runqueue (which always looks idlest because APs never run anything)
    // would leave that task permanently un-scheduled — the multi-CPU boot hang.
    // Until APs actually run the scheduler, keep every runnable task on the BSP.
    if allowed.test(SCHEDULING_CPU) {
        return SCHEDULING_CPU;
    }
    let class = unsafe { task_class(p) };
    if !class.is_null() {
        if let Some(select) = unsafe { (*class).select_task_rq } {
            let cpu = unsafe { select(p, prev_cpu, flags) };
            if allowed.test(cpu) {
                return cpu;
            }
        }
    }
    if allowed.test(prev_cpu) && rq::rq_nr_running(prev_cpu).is_some() {
        return prev_cpu;
    }
    idlest_allowed_cpu(allowed)
}

/// The only CPU that actually runs the task scheduler today: the BSP. APs are
/// brought up but idle in `ap_main` (see the note in `select_task_rq`).
pub(crate) const SCHEDULING_CPU: u32 = 0;

/// Wake a blocked task onto an allowed CPU and request reschedule there.
pub unsafe fn try_to_wake_up(p: *mut TaskStruct, wake_flags: u32) -> bool {
    if p.is_null() {
        return false;
    }
    let exit_mask =
        crate::kernel::task::task_state::EXIT_ZOMBIE | crate::kernel::task::task_state::EXIT_DEAD;
    unsafe {
        // Linux never makes an exiting task runnable again. In particular, a
        // late SIGKILL may still resolve a zombie PID; blindly storing
        // TASK_RUNNING here would resurrect that zombie and leave its parent
        // spinning forever between exit_state and __state publication.
        let state = (*p).__state.load(Ordering::Acquire);
        if (*p).m26.exit_state & exit_mask != 0
            || state & (exit_mask | crate::kernel::task::task_state::TASK_DEAD) != 0
        {
            return false;
        }
        (*p).__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
    }
    if !production_smp_scheduler_enabled() {
        legacy_place_after_current(p);
        unsafe {
            set_need_resched(get_current());
        }
        return true;
    }
    if unsafe { (*p).m29.on_rq } != 0 {
        request_reschedule(unsafe { (*p).thread_info.cpu });
        return true;
    }
    let target_cpu = select_task_rq(p, unsafe { (*p).thread_info.cpu }, wake_flags);
    unsafe {
        enqueue_on_rq(target_cpu, p, class::ENQUEUE_WAKEUP | wake_flags);
        (*p).thread_info.cpu = target_cpu;
    }
    request_reschedule(target_cpu);
    true
}

pub unsafe fn wake_task(p: *mut TaskStruct) -> bool {
    unsafe { try_to_wake_up(p, class::ENQUEUE_WAKEUP) }
}

/// Normal waitqueue/timer wake, equivalent to Linux
/// `try_to_wake_up(p, TASK_NORMAL, WF_*)`. Unlike signal/ptrace-specific wake
/// paths this must never resume a stopped or traced task.
pub unsafe fn wake_task_normal(p: *mut TaskStruct) -> bool {
    if p.is_null() {
        return false;
    }
    let state = unsafe { (*p).__state.load(Ordering::Acquire) };
    if state & crate::kernel::task::task_state::TASK_NORMAL == 0 {
        return false;
    }
    unsafe { try_to_wake_up(p, class::ENQUEUE_WAKEUP) }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use std::sync::{Mutex as StdMutex, MutexGuard as StdMutexGuard};

    static LEGACY_SCHED_TEST_LOCK: StdMutex<()> = StdMutex::new(());

    struct LegacySchedTestGuard {
        _lock: StdMutexGuard<'static, ()>,
        previous_current: *mut TaskStruct,
        previous_production: bool,
    }

    impl Drop for LegacySchedTestGuard {
        fn drop(&mut self) {
            clear_legacy_run_queue();
            PRODUCTION_SCHED_ENABLED.store(self.previous_production, Ordering::Release);
            unsafe {
                set_current(self.previous_current);
            }
        }
    }

    fn legacy_sched_test_guard() -> LegacySchedTestGuard {
        let lock = LEGACY_SCHED_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_current = unsafe { get_current() };
        let previous_production = PRODUCTION_SCHED_ENABLED.load(Ordering::Acquire);
        clear_legacy_run_queue();
        PRODUCTION_SCHED_ENABLED.store(false, Ordering::Release);
        LegacySchedTestGuard {
            _lock: lock,
            previous_current,
            previous_production,
        }
    }

    fn clear_legacy_run_queue() {
        let mut rq = RUN_QUEUE.lock();
        rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
        rq.len = 0;
        rq.current_idx = 0;
    }

    #[test]
    fn sched_set_fifo_sets_mid_rt_priority() {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let task_ptr = &mut *task as *mut TaskStruct;

        unsafe {
            linux_sched_set_fifo(task_ptr);
        }

        assert_eq!(task.m29.policy, prio::SCHED_FIFO);
        assert_eq!(task.m29.rt_priority, (prio::MAX_RT_PRIO / 2) as u32);
        assert_eq!(
            task.m29.prio,
            prio::MAX_RT_PRIO - 1 - (prio::MAX_RT_PRIO / 2)
        );
        assert_eq!(task.m29.sched_class, &rt::RT_SCHED_CLASS as *const _);
    }

    #[test]
    fn kthread_stack_frame_is_56_bytes() {
        // 7 slots × 8 bytes = 56 bytes total initial frame.
        let slots: usize = 7; // r15, r14, r13, r12, rbx, rbp, return_addr
        assert_eq!(slots * 8, 56);
    }

    #[test]
    fn max_cpus_covers_smp_max() {
        // Must be >= 1 BSP + 8 APs (the default SMP ceiling in smp.rs).
        assert!(MAX_CPUS >= 9);
    }

    #[test]
    fn task_pool_size_matches_kthread_limit() {
        assert_eq!(
            core::mem::size_of::<[TaskStruct; MAX_KTHREADS]>() / core::mem::size_of::<TaskStruct>(),
            MAX_KTHREADS
        );
    }

    #[test]
    fn kthread_stack_is_64k() {
        assert_eq!(KTHREAD_STACK_SIZE, 64 * 1024);
    }

    #[test]
    fn stack_top_for_sp_uses_kernel_thread_size_window() {
        assert_eq!(stack_top_for_sp(0), 0);
        assert_eq!(stack_top_for_sp(0xffff), 0x10000);
        assert_eq!(stack_top_for_sp(0x10000), 0x10000);
        assert_eq!(stack_top_for_sp(0x10001), 0x20000);

        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let task_ptr = &mut *task as *mut TaskStruct;
        unsafe {
            seed_current_task_stack_from_sp(task_ptr, 0xfff0);
        }
        assert_eq!(task.stack as usize, 0x10000);
        unsafe {
            seed_current_task_stack_from_sp(task_ptr, 0x1fff0);
        }
        assert_eq!(
            task.stack as usize, 0x10000,
            "once seeded, the saved task stack top must be owned by switch code"
        );
    }

    #[test]
    fn linux_offset_thread_is_used_for_task_threadsp() {
        // Verify TASK_THREADSP uses the correct struct fields.
        use crate::arch::x86::kernel::switch::TASK_THREADSP;
        use core::mem::offset_of;
        let expected = LINUX_OFFSET_THREAD + offset_of!(ThreadStruct, sp);
        assert_eq!(TASK_THREADSP, expected);
    }

    #[test]
    fn blocking_schedule_helper_keeps_irqs_enabled_around_switch() {
        let source = include_str!("mod.rs");
        let body = source
            .split("pub unsafe fn schedule_with_irqs_enabled()")
            .nth(1)
            .expect("blocking scheduler helper must exist");
        let enable_before = body
            .find("crate::kernel::locking::local_irq_enable();")
            .expect("blocking schedule must enable IRQs before switching");
        let schedule = body[enable_before..]
            .find("unsafe { schedule() };")
            .map(|off| enable_before + off)
            .expect("blocking schedule helper must call schedule");
        let enable_after = body[schedule..]
            .find("crate::kernel::locking::local_irq_enable();")
            .map(|off| schedule + off)
            .expect("blocking schedule must leave IRQs enabled after resume");

        assert!(enable_before < schedule);
        assert!(schedule < enable_after);
    }

    #[test]
    fn bsp_idle_loop_schedules_and_closes_the_check_halt_race() {
        let source = include_str!("../../init/main.rs");
        let body = source
            .split("fn halt_loop_with_softirq() -> !")
            .nth(1)
            .expect("BSP idle loop must exist")
            .split("/// Panic handler")
            .next()
            .expect("BSP idle loop must end before panic handler");

        let schedule = body
            .find("kernel::sched::schedule()")
            .expect("BSP idle task must run the scheduler");
        let disable = body[schedule..]
            .find("kernel::locking::local_irq_disable()")
            .map(|offset| schedule + offset)
            .expect("BSP idle task must disable IRQs before its final checks");
        let need_resched = body[disable..]
            .find("kernel::sched::current_needs_resched()")
            .map(|offset| disable + offset)
            .expect("BSP idle task must recheck need_resched with IRQs disabled");
        let halt = body[need_resched..]
            .find("core::arch::asm!(\"sti; hlt\"")
            .map(|offset| need_resched + offset)
            .expect("BSP idle task must atomically enable IRQs and halt");

        assert!(schedule < disable);
        assert!(disable < need_resched);
        assert!(need_resched < halt);
    }

    #[test]
    fn legacy_run_queue_lock_is_held_with_interrupts_disabled() {
        let source = include_str!("mod.rs");
        let helper = source
            .split("fn with_legacy_run_queue")
            .nth(1)
            .expect("legacy runqueue irq-save helper must exist")
            .split("#[cfg(test)]")
            .next()
            .expect("legacy runqueue helper body end");
        let save = helper
            .find("local_irq_save()")
            .expect("legacy runqueue helper must save and disable IRQs");
        let lock = helper
            .find("RUN_QUEUE.lock()")
            .expect("legacy runqueue helper must acquire RUN_QUEUE");
        let restore = helper
            .find("local_irq_restore(flags)")
            .expect("legacy runqueue helper must restore IRQs");
        assert!(save < lock);
        assert!(lock < restore);

        for (start, end) in [
            (
                "fn legacy_place_after_current",
                "#[cfg(test)]\nfn current_cpu_index",
            ),
            (
                "pub unsafe fn dequeue_task",
                "/// Cooperative legacy scheduler",
            ),
            ("pub unsafe fn legacy_schedule", "unsafe fn pick_next_task"),
        ] {
            let body = source
                .split(start)
                .nth(1)
                .unwrap_or_else(|| panic!("missing legacy runqueue caller {start}"))
                .split(end)
                .next()
                .unwrap_or_else(|| panic!("missing end marker for {start}"));
            assert!(
                body.contains("with_legacy_run_queue"),
                "{start} must use the IRQ-safe legacy runqueue helper"
            );
            assert!(
                !body.contains("RUN_QUEUE.lock()"),
                "{start} must not acquire RUN_QUEUE directly"
            );
        }
    }

    #[test]
    fn legacy_scheduler_tick_only_requests_resched() {
        let _guard = legacy_sched_test_guard();
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let current_ptr = &mut *current as *mut TaskStruct;
        unsafe {
            set_current(current_ptr);
        }

        scheduler_tick();

        assert_ne!(
            current.thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED,
            0,
            "legacy timer tick must request reschedule without touching CFS runqueue state"
        );
    }

    #[test]
    fn legacy_enqueue_places_new_task_after_current() {
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut older = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut child = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let _guard = legacy_sched_test_guard();
        let current_ptr = &mut *current as *mut TaskStruct;
        let older_ptr = &mut *older as *mut TaskStruct;
        let child_ptr = &mut *child as *mut TaskStruct;

        unsafe {
            set_current(current_ptr);
        }
        {
            let mut rq = RUN_QUEUE.lock();
            rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
            rq.tasks[0] = current_ptr;
            rq.tasks[1] = older_ptr;
            rq.len = 2;
            rq.current_idx = 0;
        }

        unsafe {
            enqueue_task(child_ptr);
        }

        let rq = RUN_QUEUE.lock();
        assert_eq!(rq.len, 3);
        assert_eq!(rq.tasks[0], current_ptr);
        assert_eq!(rq.tasks[1], child_ptr);
        assert_eq!(rq.tasks[2], older_ptr);
        assert_ne!(
            current.thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED,
            0,
            "legacy fork placement must request a syscall-exit reschedule"
        );
        drop(rq);
    }

    #[test]
    fn legacy_wake_moves_waiter_after_current() {
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut older = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut waiter = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let _guard = legacy_sched_test_guard();
        let current_ptr = &mut *current as *mut TaskStruct;
        let older_ptr = &mut *older as *mut TaskStruct;
        let waiter_ptr = &mut *waiter as *mut TaskStruct;
        waiter.__state.store(
            crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
            Ordering::Release,
        );

        unsafe {
            set_current(current_ptr);
        }
        {
            let mut rq = RUN_QUEUE.lock();
            rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
            rq.tasks[0] = current_ptr;
            rq.tasks[1] = older_ptr;
            rq.tasks[2] = waiter_ptr;
            rq.len = 3;
            rq.current_idx = 0;
        }

        assert!(unsafe { wake_task(waiter_ptr) });

        let rq = RUN_QUEUE.lock();
        assert_eq!(rq.len, 3);
        assert_eq!(rq.tasks[0], current_ptr);
        assert_eq!(rq.tasks[1], waiter_ptr);
        assert_eq!(rq.tasks[2], older_ptr);
        assert_eq!(
            waiter.__state.load(Ordering::Acquire),
            crate::kernel::task::task_state::TASK_RUNNING
        );
        assert_ne!(
            current.thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED,
            0,
            "legacy wakeup must request a syscall-exit reschedule"
        );
        drop(rq);
    }

    #[test]
    fn wake_rejects_exiting_tasks_without_resurrecting_them() {
        let _guard = legacy_sched_test_guard();
        for (exit_state, task_state) in [
            (
                crate::kernel::task::task_state::EXIT_ZOMBIE,
                crate::kernel::task::task_state::TASK_RUNNING,
            ),
            (
                crate::kernel::task::task_state::EXIT_ZOMBIE,
                crate::kernel::task::task_state::EXIT_ZOMBIE,
            ),
            (
                crate::kernel::task::task_state::EXIT_DEAD,
                crate::kernel::task::task_state::EXIT_DEAD,
            ),
            (0, crate::kernel::task::task_state::TASK_DEAD),
        ] {
            let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
            let task_ptr = &mut *task as *mut TaskStruct;
            task.m26.exit_state = exit_state;
            task.__state.store(task_state, Ordering::Release);

            assert!(!unsafe { wake_task(task_ptr) });
            assert_eq!(task.__state.load(Ordering::Acquire), task_state);
        }
        assert_eq!(RUN_QUEUE.lock().len, 0);
    }

    #[test]
    fn legacy_schedule_ignores_stale_queue_when_current_is_absent() {
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut stale = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let _guard = legacy_sched_test_guard();
        let current_ptr = &mut *current as *mut TaskStruct;
        let stale_ptr = &mut *stale as *mut TaskStruct;

        current.thread_info.flags |= crate::kernel::task::TIF_NEED_RESCHED;
        unsafe {
            set_current(current_ptr);
        }
        {
            let mut rq = RUN_QUEUE.lock();
            rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
            rq.tasks[0] = stale_ptr;
            rq.len = 1;
            rq.current_idx = 0;
        }

        unsafe {
            legacy_schedule();
        }

        assert_eq!(unsafe { get_current() }, current_ptr);
        assert_eq!(
            current.thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED,
            0,
            "absent current task should not switch through a stale queue entry"
        );
    }

    #[test]
    fn legacy_schedule_does_not_switch_to_only_sleeping_candidate() {
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut sleeper = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let _guard = legacy_sched_test_guard();
        let current_ptr = &mut *current as *mut TaskStruct;
        let sleeper_ptr = &mut *sleeper as *mut TaskStruct;

        current.thread_info.flags |= crate::kernel::task::TIF_NEED_RESCHED;
        current.thread.sp = 0x1000;
        sleeper.thread.sp = 0x2000;
        sleeper.__state.store(
            crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
            Ordering::Release,
        );
        unsafe {
            set_current(current_ptr);
        }
        {
            let mut rq = RUN_QUEUE.lock();
            rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
            rq.tasks[0] = current_ptr;
            rq.tasks[1] = sleeper_ptr;
            rq.len = 2;
            rq.current_idx = 0;
        }

        unsafe {
            legacy_schedule();
        }

        assert_eq!(unsafe { get_current() }, current_ptr);
        assert_eq!(
            current.thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED,
            0,
            "scheduler should clear resched when no runnable switch target exists"
        );
        assert_eq!(
            sleeper.__state.load(Ordering::Acquire),
            crate::kernel::task::task_state::TASK_INTERRUPTIBLE
        );
    }

    #[test]
    fn legacy_schedule_skips_stackless_runnable_candidate() {
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut stackless = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let _guard = legacy_sched_test_guard();
        let current_ptr = &mut *current as *mut TaskStruct;
        let stackless_ptr = &mut *stackless as *mut TaskStruct;

        current.thread_info.flags |= crate::kernel::task::TIF_NEED_RESCHED;
        current.thread.sp = 0x1000;
        stackless.__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
        stackless.thread.sp = 0;
        unsafe {
            set_current(current_ptr);
        }
        {
            let mut rq = RUN_QUEUE.lock();
            rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
            rq.tasks[0] = current_ptr;
            rq.tasks[1] = stackless_ptr;
            rq.len = 2;
            rq.current_idx = 0;
        }

        unsafe {
            legacy_schedule();
        }

        assert_eq!(unsafe { get_current() }, current_ptr);
        assert_eq!(
            current.thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED,
            0,
            "stackless runnable tasks must not reach __switch_to_asm"
        );
    }

    #[test]
    fn task_switch_frame_must_stay_inside_kernel_stack() {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let task_ptr = &mut *task as *mut TaskStruct;
        let stack_top = (KTHREAD_STACK_SIZE * 2) as u64;
        let stack_bottom = stack_top - KTHREAD_STACK_SIZE as u64;
        task.__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
        task.stack = stack_top as *mut core::ffi::c_void;

        task.thread.sp = stack_top - SWITCH_FRAME_BYTES as u64;
        assert!(unsafe { task_can_switch_to(task_ptr) });

        task.thread.sp = stack_bottom - 8;
        assert!(!unsafe { task_can_switch_to(task_ptr) });

        task.thread.sp = stack_top - (SWITCH_FRAME_BYTES as u64 - 8);
        assert!(!unsafe { task_can_switch_to(task_ptr) });

        task.thread.sp = stack_top - SWITCH_FRAME_BYTES as u64 + 1;
        assert!(!unsafe { task_can_switch_to(task_ptr) });

        task.pid = 371;
        task.stack = core::ptr::null_mut();
        task.thread.sp = stack_top - SWITCH_FRAME_BYTES as u64;
        assert!(!unsafe { task_can_switch_to(task_ptr) });

        task.pid = 0;
        assert!(
            !unsafe { task_can_switch_to(task_ptr) },
            "Linux idle tasks still need a real task stack before switch-in"
        );
    }

    #[test]
    fn sched_fork_defaults_idle_parent_child_to_cfs() {
        let mut parent = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut child = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });

        parent.m29 = M29SchedFields::zeroed();
        parent.m29.sched_class = &idle::IDLE_SCHED_CLASS as *const class::SchedClass;
        parent.m29.cpus_mask = entity::CpuMask::one(0);
        unsafe {
            init_task_sched_from_parent(
                &mut *parent as *mut TaskStruct,
                &mut *child as *mut TaskStruct,
            );
        }

        assert_eq!(
            child.m29.sched_class,
            &fair::FAIR_SCHED_CLASS as *const class::SchedClass
        );
        assert_eq!(child.m29.nr_cpus_allowed, 1);
        assert_eq!(
            child.m29.cpus_ptr,
            &child.m29.cpus_mask as *const entity::CpuMask
        );
        assert_eq!(child.m29.on_rq, 0);
        assert_eq!(child.m29.on_cpu, 0);
    }

    #[test]
    fn legacy_run_queue_normalizes_stale_len_before_slicing() {
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut child = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let _guard = legacy_sched_test_guard();
        let current_ptr = &mut *current as *mut TaskStruct;
        let child_ptr = &mut *child as *mut TaskStruct;

        unsafe {
            set_current(current_ptr);
        }
        {
            let mut rq = RUN_QUEUE.lock();
            rq.tasks = [core::ptr::null_mut(); MAX_RUN_QUEUE];
            rq.tasks[0] = current_ptr;
            rq.len = MAX_RUN_QUEUE + 7;
            rq.current_idx = MAX_RUN_QUEUE + 3;
        }

        unsafe {
            enqueue_task(child_ptr);
        }

        let rq = RUN_QUEUE.lock();
        assert_eq!(rq.len, MAX_RUN_QUEUE);
        assert!(rq.current_idx < rq.len);
    }
}
