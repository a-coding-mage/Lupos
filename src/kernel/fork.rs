//! linux-parity: complete
//! linux-source: vendor/linux/kernel/fork.c
//! test-origin: linux:vendor/linux/kernel/fork.c
//! Process forking — `copy_process` and `kernel_clone` — Milestone 23.
//!
//! Implements the kernel side of `fork`, `clone`, and `clone3` by porting
//! `copy_process()` and `kernel_clone()` from Linux `kernel/fork.c`.
//!
//! # What M23 implements
//!
//! - `KernelCloneArgs` — internal clone-arguments struct (mirrors
//!   `struct kernel_clone_args` from `include/linux/sched/task.h`).
//! - `copy_process()` — allocates and fully initialises a child `TaskStruct`,
//!   with the same flag-validation logic as Linux.
//! - `kernel_clone()` — calls `copy_process`, enqueues the child, and returns
//!   the child PID.
//! - `kernel_fork_child_entry` — naked assembly trampoline: the return address
//!   placed in the child's initial stack frame.
//!
//! # Duplication scope in M23
//!
//! | Field     | With flag   | Without flag          | Full impl |
//! |-----------|-------------|-----------------------|-----------|
//! | `mm`      | `CLONE_VM` → share pointer | `dup_mm()` (COW) | M14 ✓ |
//! | `files`   | `CLONE_FILES` → share ptr | share ptr (M39) | M39 |
//! | `signal`  | `CLONE_SIGHAND` → share ptr | share ptr (M25) | M25 |
//! | `cred`    | always copy ptr for now | (M27) | M27 |
//!
//! # Deferred
//! - `do_exit` / `release_task` (M26): heap tasks are tracked but not freed.
//! - `CLONE_VFORK` completion (M26): the parent is not put to sleep.
//! - Namespace flag handling (M28).
//! - User-mode thread-info restoration (M24 / M59).
//!
//! References:
//!   Linux `kernel/fork.c`
//!   Linux `include/linux/sched/task.h`

extern crate alloc;

use alloc::alloc::{Layout, alloc_zeroed, dealloc};
use alloc::boxed::Box;
use core::sync::atomic::Ordering;
use spin::Mutex;

use crate::arch::x86::entry::syscall::load_current_user_tls_base;
use crate::arch::x86::kernel::ptrace::PtRegs;
use crate::arch::x86::kernel::uaccess;
use crate::kernel::clone::{
    CLONE_EMPTY_MNTNS, CLONE_FILES, CLONE_INTO_CGROUP, CLONE_NEWNS, CLONE_NEWUSER, CLONE_NNP,
    CLONE_PIDFD, CLONE_SETTLS, CLONE_SIGHAND, CLONE_THREAD, CLONE_VFORK, CLONE_VM,
};
use crate::kernel::pid::{INIT_PID_NS, alloc_pid};
use crate::kernel::sched::{
    KTHREAD_STACK_SIZE, enqueue_task, get_current, schedule_with_irqs_enabled,
};
use crate::kernel::task::{TaskStruct, ThreadInfo};
use crate::kernel::thread::ThreadStruct;
use crate::mm::mm_types::MmStruct;

// ── KernelCloneArgs ──────────────────────────────────────────────────────────

/// Internal clone arguments passed through the kernel.
///
/// Mirrors Linux `struct kernel_clone_args` from
/// `include/linux/sched/task.h`.  Fields not yet meaningful in M23
/// (io_thread, user_worker, no_files) are present for ABI completeness
/// but not inspected.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KernelCloneArgs {
    /// `CLONE_*` flag bitmask.
    pub flags: u64,
    /// User pointer for pidfd output (CLONE_PIDFD).  NULL in M22 path.
    pub pidfd: *mut i32,
    /// User pointer for CLONE_CHILD_SETTID / CHILD_CLEARTID.
    pub child_tid: *mut i32,
    /// User pointer for CLONE_PARENT_SETTID.
    pub parent_tid: *mut i32,
    /// Signal sent to parent on child exit.
    pub exit_signal: i32,
    /// 1 = this is a kernel thread (no user mm).
    pub kthread: u32,
    /// Child stack pointer (0 = inherit parent RSP).
    pub stack: u64,
    /// Child stack size.
    pub stack_size: u64,
    /// TLS base value for CLONE_SETTLS.
    pub tls: u64,
    /// Desired PID at namespace level 0 (from clone3 `set_tid[0]`).
    pub set_tid: Option<i32>,
    /// Cgroup directory fd for clone3 `CLONE_INTO_CGROUP`.
    pub cgroup: i32,
    /// Thread function for the kthread path (set by `kthread_create_on_node`).
    pub fn_ptr: Option<unsafe extern "C" fn(*mut core::ffi::c_void) -> i32>,
    /// Argument for `fn_ptr`.
    pub fn_arg: *mut core::ffi::c_void,
    /// Saved syscall register frame for a user-mode fork/clone child.
    pub user_regs: Option<PtRegs>,
}

// SAFETY: KernelCloneArgs is short-lived and only accessed from one CPU
// under the cooperative scheduler.
unsafe impl Send for KernelCloneArgs {}
unsafe impl Sync for KernelCloneArgs {}

impl Default for KernelCloneArgs {
    fn default() -> Self {
        KernelCloneArgs {
            flags: 0,
            pidfd: core::ptr::null_mut(),
            child_tid: core::ptr::null_mut(),
            parent_tid: core::ptr::null_mut(),
            exit_signal: 0,
            kthread: 0,
            stack: 0,
            stack_size: 0,
            tls: 0,
            set_tid: None,
            cgroup: -1,
            fn_ptr: None,
            fn_arg: core::ptr::null_mut(),
            user_regs: None,
        }
    }
}

// ── Heap task tracker ─────────────────────────────────────────────────────────
//
// Heap-allocated tasks and their kernel stacks must outlive the task.
// We register them here; cleanup is deferred to `do_exit` / `release_task`
// in M26.

/// Maximum number of concurrently-live heap-allocated tasks.
pub const MAX_HEAP_TASKS: usize = crate::kernel::sched::MAX_RUN_QUEUE;

struct HeapTaskEntry {
    task: *mut TaskStruct,
    stack: *mut u8,
}

// SAFETY: Protected by HEAP_TASKS mutex.
unsafe impl Send for HeapTaskEntry {}

struct HeapTaskTracker {
    entries: [Option<HeapTaskEntry>; MAX_HEAP_TASKS],
    len: usize,
}

// SAFETY: Protected by Mutex.
unsafe impl Send for HeapTaskTracker {}

static HEAP_TASKS: Mutex<HeapTaskTracker> = Mutex::new(HeapTaskTracker {
    entries: [const { None }; MAX_HEAP_TASKS],
    len: 0,
});

fn track_heap_task(task: *mut TaskStruct, stack: *mut u8) {
    let mut tracker = HEAP_TASKS.lock();
    for i in 0..MAX_HEAP_TASKS {
        if tracker.entries[i].is_none() {
            tracker.entries[i] = Some(HeapTaskEntry { task, stack });
            tracker.len += 1;
            return;
        }
    }
}

/// Number of heap-allocated tasks currently tracked.
///
/// Used by tests to verify that `release_task` drains the tracker.
pub fn heap_task_count() -> usize {
    HEAP_TASKS.lock().len
}

fn untrack_heap_task(task: *mut TaskStruct) -> Option<*mut u8> {
    let mut tracker = HEAP_TASKS.lock();
    for i in 0..MAX_HEAP_TASKS {
        if let Some(entry) = &tracker.entries[i] {
            if entry.task == task {
                let stack = entry.stack;
                tracker.entries[i] = None;
                tracker.len -= 1;
                return Some(stack);
            }
        }
    }
    None
}

/// Locate a heap-allocated task by PID.
///
/// Returns NULL when no match is found.  Used by `ptrace::sys_ptrace` and any
/// future PID-keyed kernel lookup before the M28 IDR-backed PID hash lands.
pub fn find_heap_task_by_pid(pid: i32) -> *mut TaskStruct {
    let tracker = HEAP_TASKS.lock();
    for entry in tracker.entries.iter().flatten() {
        unsafe {
            if !entry.task.is_null() && (*entry.task).pid == pid {
                return entry.task;
            }
        }
    }
    core::ptr::null_mut()
}

/// Visit every heap-allocated task currently tracked by the early process
/// table. Used by TTY job-control signal fanout until PID hashes grow
/// process-group indexes.
pub fn for_each_heap_task(mut f: impl FnMut(*mut TaskStruct)) {
    let tracker = HEAP_TASKS.lock();
    for entry in tracker.entries.iter().flatten() {
        if !entry.task.is_null() {
            f(entry.task);
        }
    }
}

/// Locate `task` in the heap-task tracker, remove it, and free both the
/// `Box<TaskStruct>` and the `Box<[u8; KTHREAD_STACK_SIZE]>` it owns.
///
/// Called by `crate::kernel::exit::release_task` once the parent has reaped a
/// zombie child.  After this call returns, `task` is dangling and must not be
/// dereferenced.
///
/// # Safety
/// `task` must be a pointer previously returned by `copy_process` and not
/// yet released.  No other CPU may hold a reference to the task; the caller
/// (`release_task`) ensures this by removing the task from its parent's
/// children list and from the run queue first.
pub unsafe fn heap_task_release(task: *mut TaskStruct) {
    if let Some(stack) = untrack_heap_task(task) {
        unsafe {
            free_kernel_stack(stack);
            // Reclaim the TaskStruct allocation last — `task` becomes invalid.
            drop(Box::from_raw(task));
        }
    }
}

// ── kernel_fork_child_entry ───────────────────────────────────────────────────

fn kernel_stack_layout() -> Layout {
    Layout::from_size_align(KTHREAD_STACK_SIZE, 16).expect("valid kernel stack layout")
}

fn alloc_kernel_stack() -> *mut u8 {
    unsafe { alloc_zeroed(kernel_stack_layout()) }
}

unsafe fn free_kernel_stack(stack: *mut u8) {
    if !stack.is_null() {
        unsafe { dealloc(stack, kernel_stack_layout()) };
    }
}

fn alloc_task_struct_zeroed() -> *mut TaskStruct {
    unsafe { alloc_zeroed(Layout::new::<TaskStruct>()) as *mut TaskStruct }
}

unsafe fn write_user_tid(ptr: *mut i32, tid: i32) -> Result<(), i32> {
    if ptr.is_null() {
        return Ok(());
    }
    unsafe { uaccess::put_user_u32(ptr as *mut u32, tid as u32) }
}

/// Entry point for all tasks created by `copy_process`.
///
/// After `__switch_to_asm` pops six callee-saved registers and
/// `__switch_to` returns:
///   - R12 = `fn_ptr` (from `KernelCloneArgs`)
///   - RBX = `fn_arg`
///
/// Moves `fn_arg` into RDI (first System V AMD64 argument register)
/// and calls `fn_ptr`.  If `fn_ptr` is null, halts immediately.
///
/// Identical in structure to `kthread_entry_stub` but lives in `fork.rs`
/// with a distinct symbol for semantic clarity.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_fork_child_entry() -> ! {
    core::arch::naked_asm!("mov rdi, rbx", "call r12", "2: hlt", "jmp 2b",);
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe extern "C" fn kernel_fork_child_entry() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// Return the address of `kernel_fork_child_entry` for stack frame setup.
pub fn kernel_fork_child_entry_addr() -> u64 {
    kernel_fork_child_entry as u64
}

/// First return path for user-mode fork/clone children.
///
/// The scheduler switches to a newly built child kernel stack, returns here,
/// and finds a copied `PtRegs` frame at `rsp`. The frame mirrors Linux's
/// syscall-exit layout; the only semantic difference from the parent is
/// `rax = 0`, which is the user-visible child return value from `fork()`.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn user_fork_child_return() -> ! {
    core::arch::naked_asm!(
        "sub rsp, 8",
        "call {load_tls}",
        "add rsp, 8",
        // Close the interrupt window before switching RSP to the user stack.
        "cli",
        "mov r15, [rsp + 0]",
        "mov r14, [rsp + 8]",
        "mov r13, [rsp + 16]",
        "mov r12, [rsp + 24]",
        "mov rbp, [rsp + 32]",
        "mov rbx, [rsp + 40]",
        "mov r11, [rsp + 48]",
        "mov r10, [rsp + 56]",
        "mov r9,  [rsp + 64]",
        "mov r8,  [rsp + 72]",
        "mov rax, [rsp + 80]",
        "mov rcx, [rsp + 128]",
        "mov r11, [rsp + 144]",
        "mov rdx, [rsp + 96]",
        "mov rsi, [rsp + 104]",
        "mov rdi, [rsp + 112]",
        "mov rsp, [rsp + 152]",
        "swapgs",
        "sysretq",
        load_tls = sym load_current_user_tls_base,
    );
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe extern "C" fn user_fork_child_return() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

pub fn user_fork_child_return_addr() -> u64 {
    user_fork_child_return as u64
}

// ── copy_process ─────────────────────────────────────────────────────────────

/// Allocate and initialise a new child `TaskStruct`.
///
/// Ported from Linux `copy_process()` in `kernel/fork.c` (lines ~1994–2600).
/// On success returns a raw `*mut TaskStruct`; on failure returns a negative
/// Linux `errno` as `i32`.
///
/// # Validation (matching Linux fork.c lines 1994-2056)
///
/// - `CLONE_THREAD` requires `CLONE_SIGHAND`
/// - `CLONE_SIGHAND` requires `CLONE_VM`
/// - `exit_signal` must be 0 or a valid signal number (0–63 in M23)
///
/// # Memory duplication (M23 scope)
///
/// | `mm` field | With `CLONE_VM` | Without `CLONE_VM` |
/// |------------|-----------------|-------------------|
/// | action     | share pointer   | `dup_mm` or null for kthreads |
///
/// `files`, `signal`, and `cred` are pointer-copied until M25/M27/M39.
///
/// # Safety
///
/// `parent` must be a valid, non-null `TaskStruct` pointer.
pub unsafe fn copy_process(
    parent: *mut TaskStruct,
    args: &KernelCloneArgs,
) -> Result<*mut TaskStruct, i32> {
    // ── 1. Validate flags ───────────────────────────────────────────────────
    if args.flags & CLONE_THREAD != 0 && args.flags & CLONE_SIGHAND == 0 {
        return Err(-22); // EINVAL: CLONE_THREAD requires CLONE_SIGHAND
    }
    if args.flags & CLONE_SIGHAND != 0 && args.flags & CLONE_VM == 0 {
        return Err(-22); // EINVAL: CLONE_SIGHAND requires CLONE_VM
    }
    if args.exit_signal < 0 || args.exit_signal > 64 {
        return Err(-22); // EINVAL: invalid signal number
    }

    // ── 2. Allocate the child TaskStruct on the heap ────────────────────────
    let child: *mut TaskStruct = alloc_task_struct_zeroed();
    if child.is_null() {
        return Err(-12);
    }

    // ── 3. Allocate a kernel stack ──────────────────────────────────────────
    let stack_ptr: *mut u8 = alloc_kernel_stack();
    if stack_ptr.is_null() {
        unsafe {
            drop(Box::from_raw(child));
        }
        return Err(-12);
    }
    let stack_top = unsafe { stack_ptr.add(KTHREAD_STACK_SIZE) };

    track_heap_task(child, stack_ptr);
    let mut task_allocated = false;

    unsafe {
        // ── 4. Copy thread_info from parent; clear TIF_SIGPENDING (bit 2) ──
        (*child).thread_info = ThreadInfo {
            flags: (*parent).thread_info.flags & !(1u64 << 2), // TIF_SIGPENDING
            syscall_work: (*parent).thread_info.syscall_work,
            status: 0,
            cpu: (*parent).thread_info.cpu,
        };

        // ── 5. Set initial task state: TASK_RUNNING ─────────────────────────
        (*child).__state = core::sync::atomic::AtomicU32::new(0);
        (*child).saved_state = 0;
        (*child).stack = stack_top as *mut _;
        crate::kernel::sched::init_task_sched_from_parent(parent, child);

        // ── 6. Allocate a PID ───────────────────────────────────────────────
        let kpid = match alloc_pid(&INIT_PID_NS, args.set_tid) {
            Ok(p) => p,
            Err(e) => {
                cleanup_failed_child(parent, child, stack_ptr, task_allocated);
                return Err(e);
            }
        };
        let nr = kpid.numbers[0].nr;
        // M26: keep a raw pointer to the KPid in the child so `release_task`
        // can drop the refcount via `put_pid`.
        let kpid_raw = Box::into_raw(kpid);
        (*child).m26.thread_pid = kpid_raw;

        (*child).pid = nr;
        // tgid: join parent's thread group for CLONE_THREAD; own group otherwise.
        (*child).tgid = if args.flags & CLONE_THREAD != 0 {
            (*parent).tgid
        } else {
            nr
        };
        crate::kernel::syscalls::inherit_process_rlimits(parent, child);
        crate::kernel::session::inherit_from_parent((*parent).pid, nr);
        let task_alloc_err = crate::security::security_task_alloc(nr as u32, args.flags);
        if task_alloc_err != 0 {
            cleanup_failed_child(parent, child, stack_ptr, task_allocated);
            return Err(task_alloc_err);
        }
        task_allocated = true;

        // ── 6b. M26: link parent → child ─────────────────────────────────────
        (*child).m26.real_parent = parent;
        (*child).m26.parent = parent;
        (*child).m26.group_leader = if args.flags & CLONE_THREAD != 0 {
            // Inherit the parent's group leader for thread-group siblings.
            (*parent).m26.group_leader
        } else {
            child // own group leader
        };
        (*child).m26.exit_signal = args.exit_signal;
        if (*parent).m27.mdwe_flags
            & (crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER
                | crate::kernel::task::TASK_CTRL_CHILD_SUBREAPER)
            != 0
        {
            (*child).m27.mdwe_flags |= crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER;
        }
        if args.flags & crate::kernel::clone::CLONE_CHILD_CLEARTID != 0 {
            (*child).m26.clear_child_tid = args.child_tid;
        }

        if args.flags & crate::kernel::clone::CLONE_PARENT_SETTID != 0 {
            if let Err(e) = unsafe { write_user_tid(args.parent_tid, nr) } {
                cleanup_failed_child(parent, child, stack_ptr, task_allocated);
                return Err(e);
            }
        }
        if args.flags & crate::kernel::clone::CLONE_CHILD_SETTID != 0 {
            if let Err(e) = unsafe { write_user_tid(args.child_tid, nr) } {
                cleanup_failed_child(parent, child, stack_ptr, task_allocated);
                return Err(e);
            }
        }
        // Insert into parent's children array (best-effort; no error if full).

        // ── 7. Build initial stack frame ────────────────────────────────────
        // Layout (7 × 8 bytes = 56 bytes, stack grows downward):
        //   [sp-1] = entry point (kernel_fork_child_entry)
        //   [sp-2] = saved RBP = 0
        //   [sp-3] = saved RBX = fn_arg  (loaded into RDI by entry stub)
        //   [sp-4] = saved R12 = fn_ptr  (called by entry stub)
        //   [sp-5..7] = R13/R14/R15 = 0
        let initial_sp = if args.kthread == 0 {
            if let Some(mut regs) = args.user_regs {
                regs.rax = 0;
                if args.stack != 0 {
                    regs.rsp = if args.stack_size != 0 {
                        args.stack.saturating_add(args.stack_size)
                    } else {
                        args.stack
                    };
                }
                let regs_ptr =
                    (stack_top as *mut u8).sub(core::mem::size_of::<PtRegs>()) as *mut PtRegs;
                regs_ptr.write(regs);
                let ret_slot = (regs_ptr as *mut u64).sub(1);
                ret_slot.write(user_fork_child_return_addr());
                ret_slot.sub(1).write(0); // RBP
                ret_slot.sub(2).write(0); // RBX
                ret_slot.sub(3).write(0); // R12
                ret_slot.sub(4).write(0); // R13
                ret_slot.sub(5).write(0); // R14
                ret_slot.sub(6).write(0); // R15
                ret_slot.sub(6) as u64
            } else {
                let sp = stack_top as *mut u64;
                sp.sub(1).write(kernel_fork_child_entry_addr());
                sp.sub(2).write(0); // RBP
                sp.sub(3).write(args.fn_arg as u64); // RBX = fn_arg
                sp.sub(4).write(args.fn_ptr.map(|f| f as u64).unwrap_or(0));
                sp.sub(5).write(0); // R13
                sp.sub(6).write(0); // R14
                sp.sub(7).write(0); // R15
                sp.sub(7) as u64
            }
        } else {
            let sp = stack_top as *mut u64;
            sp.sub(1).write(kernel_fork_child_entry_addr());
            sp.sub(2).write(0); // RBP
            sp.sub(3).write(args.fn_arg as u64); // RBX = fn_arg
            sp.sub(4).write(args.fn_ptr.map(|f| f as u64).unwrap_or(0));
            sp.sub(5).write(0); // R13
            sp.sub(6).write(0); // R14
            sp.sub(7).write(0); // R15
            sp.sub(7) as u64
        };

        let mut child_thread = ThreadStruct {
            tls_array: (*parent).thread.tls_array,
            sp: initial_sp,
            es: (*parent).thread.es,
            ds: (*parent).thread.ds,
            fsindex: (*parent).thread.fsindex,
            gsindex: (*parent).thread.gsindex,
            _pad0: 0,
            fsbase: (*parent).thread.fsbase,
            gsbase: (*parent).thread.gsbase,
            pkru: (*parent).thread.pkru,
            _pad1: 0,
        };
        // CLONE_SETTLS: override FS base with the requested TLS value.
        if args.flags & CLONE_SETTLS != 0 {
            child_thread.fsbase = args.tls;
        }
        (*child).thread = child_thread;

        // ── 8. copy_mm ──────────────────────────────────────────────────────
        (*child).mm = if args.flags & CLONE_VM != 0 {
            // Share the parent's mm.
            let mm = (*parent).mm;
            if !mm.is_null() {
                (*mm).mmget();
            }
            mm
        } else if args.kthread != 0 || (*parent).mm.is_null() {
            // Kernel threads and children of kernel threads have no mm.
            core::ptr::null_mut()
        } else {
            // Duplicate the mm (COW setup).
            match crate::mm::fork::dup_mm((*parent).mm) {
                Some(mm) => mm,
                None => {
                    cleanup_failed_child(parent, child, stack_ptr, task_allocated);
                    return Err(-12); // ENOMEM
                }
            }
        };

        // ── 9. copy_files (M39) — share on CLONE_FILES, dup otherwise ───────
        (*child).active_mm = if !(*child).mm.is_null() {
            (*child).mm
        } else {
            (*parent).active_mm
        };
        if args.flags & CLONE_VM != 0 && !(*child).active_mm.is_null() {
            crate::kernel::futex::futex_private_hash_note_clone((*child).active_mm as u64);
        }

        crate::kernel::files::copy_files(child, parent, args.flags & CLONE_FILES != 0);

        // ── 9b. copy_fs (fs_struct.c) — share on CLONE_FS, private copy otherwise ──
        crate::fs::fs_struct::copy_fs(
            child,
            parent,
            args.flags & crate::kernel::clone::CLONE_FS != 0,
        );

        // ── 10. copy_signal / copy_sighand (pointer copy; full dup in M25) ──
        (*child).signal = if args.flags & CLONE_SIGHAND != 0 {
            (*parent).signal
        } else {
            (*parent).signal // shallow copy — real dup in M25
        };

        // ── 11. copy_creds (M27) — real COW credential dup ──────────────────
        if let Err(e) = crate::kernel::cred::copy_creds(child, parent, args.flags) {
            cleanup_failed_child(parent, child, stack_ptr, task_allocated);
            return Err(e);
        }

        // ── 11b. seccomp (M27) — inherit mode + filter chain head ────────────
        let parent_mode = (*parent)
            .m27_seccomp
            .mode
            .load(core::sync::atomic::Ordering::Acquire);
        (*child)
            .m27_seccomp
            .mode
            .store(parent_mode, core::sync::atomic::Ordering::Release);
        let parent_filter = (*parent)
            .m27_seccomp
            .filter
            .load(core::sync::atomic::Ordering::Acquire);
        if !parent_filter.is_null() {
            (*parent_filter)
                .usage
                .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        }
        (*child)
            .m27_seccomp
            .filter
            .store(parent_filter, core::sync::atomic::Ordering::Release);
        (*child).m27.no_new_privs = (*parent).m27.no_new_privs;
        // ── 11c. copy_namespaces (M28) ───────────────────────────────────────
        if let Err(e) = crate::kernel::nsproxy::copy_namespaces(args.flags, parent, child) {
            cleanup_failed_child(parent, child, stack_ptr, task_allocated);
            return Err(e);
        }

        // ── 12. Copy comm from parent ────────────────────────────────────────
        (*child).comm = (*parent).comm;
        let parent_count = (*parent).m26.children_count as usize;
        if parent_count < crate::kernel::task::MAX_CHILDREN {
            (*parent).m26.children[parent_count] = child;
            (*parent).m26.children_count = (parent_count + 1) as u32;
        }
    }

    Ok(child)
}

// ── kernel_clone ─────────────────────────────────────────────────────────────

/// Drop task-owned security and namespace state that is not reclaimed by the
/// plain `Box<TaskStruct>` release.
///
/// # Safety
/// `task` must be a valid task pointer that is no longer reachable by normal
/// execution paths.
pub(crate) unsafe fn cleanup_task_shared_state(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }

    unsafe {
        crate::fs::fs_struct::exit_fs(task);

        let seccomp_filter = (*task)
            .m27_seccomp
            .filter
            .swap(core::ptr::null_mut(), Ordering::AcqRel);
        crate::kernel::seccomp::SeccompFilter::put(seccomp_filter);

        if !(*task).m28_nsproxy.nsproxy.is_null() {
            crate::kernel::nsproxy::put_nsproxy((*task).m28_nsproxy.nsproxy);
            (*task).m28_nsproxy.nsproxy = core::ptr::null_mut();
            (*task).m28_nsproxy.thread_pid_ns_for_children = core::ptr::null_mut();
        }

        let cred = (*task).cred;
        let real_cred = (*task).m27.real_cred;
        (*task).cred = core::ptr::null();
        (*task).m27.real_cred = core::ptr::null();
        if !cred.is_null() {
            crate::kernel::cred::Cred::put(cred);
        }
        if !real_cred.is_null() {
            crate::kernel::cred::Cred::put(real_cred);
        }
    }
}

unsafe fn cleanup_failed_child(
    parent: *mut TaskStruct,
    child: *mut TaskStruct,
    stack_ptr: *mut u8,
    task_allocated: bool,
) {
    if child.is_null() {
        return;
    }

    unsafe {
        if !parent.is_null() {
            let count = (*parent).m26.children_count as usize;
            let mut found_at: Option<usize> = None;
            for i in 0..count.min(crate::kernel::task::MAX_CHILDREN) {
                if (*parent).m26.children[i] == child {
                    found_at = Some(i);
                    break;
                }
            }
            if let Some(i) = found_at {
                let last = count - 1;
                if i != last {
                    (*parent).m26.children[i] = (*parent).m26.children[last];
                }
                (*parent).m26.children[last] = core::ptr::null_mut();
                (*parent).m26.children_count = last as u32;
            }
        }

        if task_allocated {
            crate::security::security_task_free((*child).pid as u32);
        }

        crate::kernel::syscalls::release_process_rlimits(child);
        crate::kernel::syscalls::release_task_rseq_registration(child);

        cleanup_task_shared_state(child);

        if !(*child).mm.is_null() {
            let mm = (*child).mm;
            (*child).mm = core::ptr::null_mut();
            (*child).active_mm = core::ptr::null_mut();
            crate::mm::fork::mmput(mm);
        }

        let thread_pid = (*child).m26.thread_pid;
        (*child).m26.thread_pid = core::ptr::null_mut();
        if !thread_pid.is_null() {
            crate::kernel::pid::put_pid(thread_pid);
        }

        let tracked_stack = untrack_heap_task(child);
        let stack_to_free = tracked_stack.unwrap_or(stack_ptr);
        drop(Box::from_raw(child));
        free_kernel_stack(stack_to_free);
    }
}

/// Entry point for all fork/clone/clone3 syscalls.
///
/// Calls `copy_process`, enqueues the child, and returns the child's PID.
/// Returns a negative errno on failure.
///
/// Mirrors Linux `kernel_clone()` in `kernel/fork.c` (line ~2672).
///
/// # Safety
/// Must be called from a valid task context (after `sched_init()`).
pub unsafe fn kernel_clone(args: &KernelCloneArgs) -> i64 {
    let parent = unsafe { get_current() };
    if parent.is_null() {
        return -22; // EINVAL — no current task
    }

    let mut effective_args = *args;
    if effective_args.flags & CLONE_EMPTY_MNTNS != 0 {
        effective_args.flags |= CLONE_NEWNS;
    }
    if effective_args.flags & CLONE_NEWUSER != 0 {
        // User namespaces are only modeled as static init credentials today.
        // Failing the clone keeps callers such as systemd's ID-mapping probe on
        // their non-userns fallback instead of exposing incomplete proc/ns state.
        return -1; // EPERM
    }
    if effective_args.flags & CLONE_NEWNS != 0 {
        // Mount namespaces are identity-only (copy_mnt_ns clones no mount
        // tree); a child mutating its "private" tree would corrupt the
        // global one and freeze the system under systemd's per-service
        // sandboxing.  EPERM keeps systemd on its containerized fallback.
        // See kernel::nsproxy::sys_unshare and the ROADMAP "Per-namespace
        // mount trees" prerequisite.
        return -1; // EPERM
    }

    let child = match unsafe { copy_process(parent, &effective_args) } {
        Ok(c) => c,
        Err(e) => return e as i64,
    };

    let child_pid = unsafe { (*child).pid } as i64;

    if effective_args.flags & CLONE_NNP != 0 {
        unsafe {
            (*child).m27.no_new_privs = 1;
        }
    }

    if effective_args.flags & CLONE_INTO_CGROUP != 0 {
        if let Err(errno) =
            crate::kernel::cgroup::assign_pid_to_cgroup_fd(child_pid as i32, effective_args.cgroup)
        {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::cgroup_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-cgroup-clone-into-error child={} fd={} errno={}",
                    child_pid,
                    effective_args.cgroup,
                    errno
                );
            }
            unsafe {
                cleanup_failed_child(parent, child, core::ptr::null_mut(), true);
            }
            return -(errno as i64);
        }
    }

    if effective_args.flags & CLONE_PIDFD != 0 {
        if effective_args.pidfd.is_null() {
            unsafe {
                cleanup_failed_child(parent, child, core::ptr::null_mut(), true);
            }
            return -14; // EFAULT
        }
        let pidfd = match crate::fs::pidfd::install_pidfd(child, true) {
            Ok(fd) => fd,
            Err(errno) => {
                unsafe {
                    cleanup_failed_child(parent, child, core::ptr::null_mut(), true);
                }
                return -(errno as i64);
            }
        };
        #[cfg(not(test))]
        if crate::kernel::debug_trace::proc_enabled() {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-proc-pidfd child={} fd={} ptr={:#x}",
                child_pid,
                pidfd,
                effective_args.pidfd as usize
            );
        }
        if let Err(errno) = unsafe { write_user_tid(effective_args.pidfd, pidfd) } {
            if let Some(files) = unsafe { crate::kernel::files::get_task_files(parent) } {
                let _ = files.close(pidfd);
            }
            unsafe {
                cleanup_failed_child(parent, child, core::ptr::null_mut(), true);
            }
            return errno as i64;
        }
    }

    // Enqueue the child so the scheduler can run it.
    unsafe { enqueue_task(child) };

    if effective_args.flags & CLONE_VFORK != 0 {
        for _ in 0..1024 {
            let child_done = unsafe {
                (*child).__state.load(Ordering::Acquire)
                    & crate::kernel::task::task_state::EXIT_ZOMBIE
                    != 0
                    || (*child).mm != (*parent).mm
            };
            if child_done {
                break;
            }
            unsafe { schedule_with_irqs_enabled() };
        }
    }

    child_pid
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::vec::Vec;
    use std::sync::Mutex;

    use crate::include::uapi::errno::EACCES;
    use crate::kernel::clone::{CLONE_SIGHAND, CLONE_THREAD, CLONE_VM};
    use crate::kernel::pid::RESERVED_PIDS;
    use crate::security::hooks::{LsmHooks, NOOP_HOOKS};
    use crate::security::lsm_list::{TEST_LSM_LOCK, reset_for_test};
    use crate::security::register_lsm;

    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static TASK_HOOK_LOG: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

    macro_rules! clean_lsm_hooks {
        () => {
            let _lsm_guard = TEST_LSM_LOCK.lock();
            reset_for_test();
        };
    }

    /// Build a zeroed parent TaskStruct on the heap.
    fn make_parent() -> Box<TaskStruct> {
        let mut p = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        p.pid = 1000;
        p.tgid = 1000;
        p.comm = *b"test-parent\0\0\0\0\0";
        p
    }

    /// Free a child's PID without going through do_exit (deferred to M26).
    unsafe fn cleanup_child(child: *mut TaskStruct) {
        let parent = unsafe { (*child).m26.real_parent };
        unsafe { cleanup_failed_child(parent, child, core::ptr::null_mut(), true) };
        // Don't drop — the child is in HEAP_TASKS; proper cleanup in M26.
    }

    // ── Flag validation ──────────────────────────────────────────────────────

    unsafe fn cleanup_child_fully(child: *mut TaskStruct) {
        let parent = unsafe { (*child).m26.real_parent };
        unsafe { cleanup_failed_child(parent, child, core::ptr::null_mut(), true) };
    }

    fn test_task_alloc_allow(_task_id: u32, _clone_flags: u64) -> i32 {
        TASK_HOOK_LOG.lock().unwrap().push("alloc");
        0
    }

    fn test_task_alloc_deny(_task_id: u32, _clone_flags: u64) -> i32 {
        TASK_HOOK_LOG.lock().unwrap().push("alloc");
        -EACCES
    }

    fn test_task_free(_task_id: u32) {
        TASK_HOOK_LOG.lock().unwrap().push("free");
    }

    #[test]
    fn copy_process_rejects_thread_without_sighand() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let args = KernelCloneArgs {
            flags: CLONE_VM | CLONE_THREAD, // missing CLONE_SIGHAND
            ..KernelCloneArgs::default()
        };
        let result = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) };
        assert_eq!(
            result,
            Err(-22),
            "CLONE_THREAD without CLONE_SIGHAND must return EINVAL"
        );
    }

    #[test]
    fn copy_process_rejects_sighand_without_vm() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let args = KernelCloneArgs {
            flags: CLONE_SIGHAND, // missing CLONE_VM
            ..KernelCloneArgs::default()
        };
        let result = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) };
        assert_eq!(
            result,
            Err(-22),
            "CLONE_SIGHAND without CLONE_VM must return EINVAL"
        );
    }

    #[test]
    fn copy_process_rejects_invalid_exit_signal() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let args = KernelCloneArgs {
            exit_signal: -1,
            ..KernelCloneArgs::default()
        };
        let result = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) };
        assert_eq!(result, Err(-22));
    }

    // ── PID assignment ────────────────────────────────────────────────────────

    #[test]
    fn copy_process_inherits_child_subreaper_hint() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        parent.m27.mdwe_flags |= crate::kernel::task::TASK_CTRL_CHILD_SUBREAPER;

        let child =
            unsafe { copy_process(&mut *parent as *mut TaskStruct, &KernelCloneArgs::default()) }
                .expect("child");

        unsafe {
            assert_ne!(
                (*child).m27.mdwe_flags & crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER,
                0
            );
            cleanup_child_fully(child);
        }
    }

    #[test]
    fn copy_process_child_gets_unique_pid() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let args = KernelCloneArgs {
            flags: CLONE_VM,
            kthread: 0,
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should succeed");
        let child_pid = unsafe { (*child).pid };
        assert_ne!(child_pid, parent.pid, "Child PID must differ from parent");
        assert!(
            child_pid >= RESERVED_PIDS,
            "Child PID must be >= RESERVED_PIDS"
        );
        unsafe { cleanup_child(child) };
    }

    #[test]
    fn copy_process_user_fork_builds_sysret_child_frame() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let regs = PtRegs {
            r15: 15,
            r14: 14,
            r13: 13,
            r12: 12,
            rbp: 0x5555,
            rbx: 0x4444,
            r11: 0x202,
            r10: 10,
            r9: 9,
            r8: 8,
            rax: 57,
            rcx: 0x401000,
            rdx: 2,
            rsi: 1,
            rdi: 0,
            orig_rax: 57,
            rip: 0x401000,
            cs: 0x23,
            eflags: 0x202,
            rsp: 0x7fff_ffff_f000,
            ss: 0x1b,
        };
        let args = KernelCloneArgs {
            flags: CLONE_VM,
            user_regs: Some(regs),
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should build a user child");
        let frame = unsafe { (*child).thread.sp as *const u64 };
        assert_eq!(unsafe { *frame.add(6) }, user_fork_child_return_addr());
        let child_regs = unsafe { &*(frame.add(7) as *const PtRegs) };
        assert_eq!(child_regs.rax, 0);
        assert_eq!(child_regs.rip, regs.rip);
        assert_eq!(child_regs.rsp, regs.rsp);
        unsafe { cleanup_child(child) };
    }

    #[test]
    fn user_fork_child_return_disables_irqs_before_user_rsp_switch() {
        let source = include_str!("fork.rs");
        let trampoline = source
            .split("pub unsafe extern \"C\" fn user_fork_child_return() -> !")
            .nth(1)
            .expect("user fork child return trampoline must exist");
        let irq_disable = trampoline
            .find("\"cli\"")
            .expect("fork child return must disable IRQs before SYSRET restore");
        let user_rsp_load = trampoline
            .find("\"mov rsp, [rsp + 152]\"")
            .expect("fork child return must load user RSP before SYSRET");
        let swapgs = trampoline
            .find("\"swapgs\"")
            .expect("fork child return must swap to user GS before SYSRET");
        let sysret = trampoline
            .find("\"sysretq\"")
            .expect("fork child return must return through SYSRET");

        assert!(irq_disable < user_rsp_load);
        assert!(user_rsp_load < swapgs);
        assert!(swapgs < sysret);
    }

    #[test]
    fn copy_process_fork_sets_tgid_to_own_pid() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let args = KernelCloneArgs {
            flags: 0,
            kthread: 1, // use kthread path to avoid dup_mm
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should succeed");
        let pid = unsafe { (*child).pid };
        let tgid = unsafe { (*child).tgid };
        assert_eq!(
            pid, tgid,
            "Without CLONE_THREAD, child.tgid must equal child.pid"
        );
        unsafe { cleanup_child(child) };
    }

    #[test]
    fn copy_process_clone_thread_sets_tgid_to_parent_tgid() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        parent.tgid = 2000;
        let args = KernelCloneArgs {
            flags: CLONE_VM | CLONE_SIGHAND | CLONE_THREAD,
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should succeed");
        assert_eq!(
            unsafe { (*child).tgid },
            parent.tgid,
            "CLONE_THREAD child.tgid must equal parent.tgid"
        );
        unsafe { cleanup_child(child) };
    }

    // ── mm handling ───────────────────────────────────────────────────────────

    #[test]
    fn copy_process_clone_vm_shares_mm() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        // Use a sentinel non-null pointer — we don't dereference it in the
        // CLONE_VM path (no dup_mm call).
        let parent_mm = Box::into_raw(Box::new(MmStruct::new(0)));
        parent.mm = parent_mm;

        let args = KernelCloneArgs {
            flags: CLONE_VM,
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should succeed");
        assert_eq!(
            unsafe { (*child).mm },
            parent_mm,
            "With CLONE_VM, child.mm must equal parent.mm"
        );
        unsafe { cleanup_child(child) };
        unsafe {
            let _ = Box::from_raw(parent_mm);
        }
    }

    #[test]
    fn copy_process_kthread_has_null_mm() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        // parent.mm stays null (kthread)
        let args = KernelCloneArgs {
            flags: 0,
            kthread: 1,
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should succeed");
        assert!(
            unsafe { (*child).mm.is_null() },
            "Kernel threads must have null mm"
        );
        unsafe { cleanup_child(child) };
    }

    #[test]
    fn copy_process_dup_mm_fails_gracefully() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        // Allocate a real zeroed MmStruct so dup_mm can dereference it safely.
        // The buddy allocator is not initialised in the test environment,
        // so dup_mm returns None → copy_process returns ENOMEM.
        let real_mm = Box::into_raw(Box::new(unsafe { core::mem::zeroed::<MmStruct>() }));
        parent.mm = real_mm;

        let args = KernelCloneArgs {
            flags: 0,
            kthread: 0,
            ..KernelCloneArgs::default()
        };
        let result = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) };
        // Leak real_mm — deferred cleanup (M26 do_exit).
        assert_eq!(result, Err(-12), "dup_mm failure should return ENOMEM");
    }

    // ── comm copy ─────────────────────────────────────────────────────────────

    #[test]
    fn copy_process_copies_comm_from_parent() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let args = KernelCloneArgs {
            flags: CLONE_VM,
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should succeed");
        assert_eq!(
            unsafe { (*child).comm },
            parent.comm,
            "Child comm must be copied from parent"
        );
        unsafe { cleanup_child(child) };
    }

    // ── CLONE_SETTLS ──────────────────────────────────────────────────────────

    #[test]
    fn copy_process_clone_settls_sets_fsbase() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let tls_addr = 0xdead_0000_u64;
        let args = KernelCloneArgs {
            flags: CLONE_VM | CLONE_SETTLS,
            tls: tls_addr,
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should succeed");
        assert_eq!(
            unsafe { (*child).thread.fsbase },
            tls_addr,
            "CLONE_SETTLS must set child.thread.fsbase"
        );
        unsafe { cleanup_child(child) };
    }

    #[test]
    fn copy_process_clone_tid_flags_write_expected_slots() {
        clean_lsm_hooks!();
        let mut parent = make_parent();
        let mut parent_tid = -1i32;
        let mut child_tid = -1i32;
        let args = KernelCloneArgs {
            flags: crate::kernel::clone::CLONE_PARENT_SETTID
                | crate::kernel::clone::CLONE_CHILD_SETTID
                | crate::kernel::clone::CLONE_CHILD_CLEARTID,
            parent_tid: &mut parent_tid,
            child_tid: &mut child_tid,
            kthread: 1,
            ..KernelCloneArgs::default()
        };
        let child = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }
            .expect("copy_process should write clone tid pointers");
        let pid = unsafe { (*child).pid };
        assert_eq!(parent_tid, pid, "CLONE_PARENT_SETTID must write child pid");
        assert_eq!(child_tid, pid, "CLONE_CHILD_SETTID must write child pid");
        assert_eq!(
            unsafe { (*child).m26.clear_child_tid },
            &mut child_tid as *mut i32,
            "CLONE_CHILD_CLEARTID must store the child clear-tid pointer"
        );
        unsafe { cleanup_child(child) };
    }

    // ── KernelCloneArgs layout ────────────────────────────────────────────────

    #[test]
    fn kernel_clone_args_default_is_sane() {
        let args = KernelCloneArgs::default();
        assert_eq!(args.flags, 0);
        assert!(args.pidfd.is_null());
        assert!(args.child_tid.is_null());
        assert_eq!(args.exit_signal, 0);
        assert_eq!(args.kthread, 0);
        assert_eq!(args.cgroup, -1);
        assert!(args.fn_ptr.is_none());
    }

    #[test]
    fn copy_process_lsm_denial_cleans_tracked_child_state() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        TASK_HOOK_LOG.lock().unwrap().clear();

        let baseline = heap_task_count();
        let mut parent = make_parent();
        register_lsm(LsmHooks {
            name: "fork_task_alloc_deny",
            task_alloc: Some(test_task_alloc_deny),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        let args = KernelCloneArgs {
            kthread: 1,
            ..KernelCloneArgs::default()
        };
        let result = unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) };

        assert_eq!(result, Err(-EACCES));
        assert_eq!(heap_task_count(), baseline);
        assert_eq!(&*TASK_HOOK_LOG.lock().unwrap(), &["alloc"]);
    }

    #[test]
    fn copy_process_task_hooks_match_linux_lifecycle() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        TASK_HOOK_LOG.lock().unwrap().clear();

        let baseline = heap_task_count();
        let mut parent = make_parent();
        register_lsm(LsmHooks {
            name: "fork_task_lifecycle",
            task_alloc: Some(test_task_alloc_allow),
            task_free: Some(test_task_free),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        let args = KernelCloneArgs {
            kthread: 1,
            ..KernelCloneArgs::default()
        };
        let child =
            unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }.expect("copy_process");
        unsafe { cleanup_child_fully(child) };

        assert_eq!(heap_task_count(), baseline);
        assert_eq!(&*TASK_HOOK_LOG.lock().unwrap(), &["alloc", "free"]);
        assert_eq!(parent.m26.children_count, 0);
    }
}
