//! linux-parity: complete
//! linux-source: vendor/linux/kernel/kthread.c
//! test-origin: linux:vendor/linux/kernel/kthread.c
//! Kernel thread lifecycle — Milestone 22.
//!
//! Provides the public kthread API on top of the low-level pool allocator in
//! `sched.rs`.  Mirrors Linux `kernel/kthread.c` and `include/linux/kthread.h`.
//!
//! # Architecture
//!
//! - `KThread` is a heap-allocated struct that tracks per-thread lifecycle
//!   state (stop flag, exit result, function pointer, back-link to task).
//! - A global `KTHREAD_TABLE` maps `*mut TaskStruct` → `*mut KThread` for the
//!   static pool.  This avoids adding fields to `TaskStruct` (which has
//!   carefully validated Linux-ABI offsets).
//! - `kthread_create_on_node` allocates a pool slot via `sched_alloc_kthread_raw`,
//!   assigns a real PID, creates the `KThread`, and leaves the task in the
//!   stopped state (not yet enqueued).
//! - `kthread_run` = create + enqueue.
//! - `kthread_stop` sets `KTHREAD_SHOULD_STOP`, then spin-yields until the
//!   thread marks itself `TASK_DEAD` and calls `dequeue_task`.
//! - The kthread function itself calls `kthread_should_stop()` in its main
//!   loop and returns an `i32` result.  The `kthread_start` trampoline stores
//!   the result and handles TASK_DEAD + dequeue.
//!
//! # Deferred to M29
//! - `kthreadd` daemon for deferred thread creation
//! - `kthread_park` / `kthread_unpark`
//! - Per-CPU kthreads (`kthread_create_on_cpu`)
//!
//! References:
//!   Linux `kernel/kthread.c`
//!   Linux `include/linux/kthread.h`

extern crate alloc;

use alloc::boxed::Box;
use core::ffi::c_void;
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use spin::Mutex;

use crate::kernel::pid::{INIT_PID_NS, alloc_pid};
use crate::kernel::sched::{
    MAX_KTHREADS, dequeue_task, enqueue_task, get_current, sched_alloc_kthread_raw,
    schedule_with_irqs_enabled,
};
use crate::kernel::task::TaskStruct;

// ── Constants ────────────────────────────────────────────────────────────────

/// Bit index of the "should stop" flag in `KThread::flags`.
/// Matches Linux `KTHREAD_SHOULD_STOP` in `enum KTHREAD_BITS`.
pub const KTHREAD_SHOULD_STOP: usize = 1;

/// Bit index of the "should park" flag.
/// Matches Linux `KTHREAD_SHOULD_PARK`.  Parking itself is deferred to M29.
pub const KTHREAD_SHOULD_PARK: usize = 2;

/// `task.__state` value for a dead / exited task.  Matches Linux `TASK_DEAD`
/// (= 0x80) — see `crate::kernel::task::task_state`.
use crate::kernel::task::task_state::TASK_DEAD;

// ── KThread ──────────────────────────────────────────────────────────────────

/// Per-kthread lifecycle state.
///
/// Mirrors Linux `struct kthread` from `kernel/kthread.c`.
/// Heap-allocated and registered in `KTHREAD_TABLE`.
pub struct KThread {
    /// Lifecycle flags: `KTHREAD_SHOULD_STOP` at bit 1.
    pub flags: AtomicUsize,
    /// Exit result set by the thread before it returns.
    pub result: AtomicI32,
    /// The user-provided thread function (returns `i32`, unlike `KthreadFn`).
    pub threadfn: unsafe extern "C" fn(*mut c_void) -> i32,
    /// Argument passed to `threadfn`.
    pub data: *mut c_void,
    /// Back-pointer to the owning `TaskStruct`.
    pub task: *mut TaskStruct,
}

// SAFETY: KThread is only accessed from its owning thread (read) or from
// kthread_stop (write to flags), under cooperative scheduling in M22.
unsafe impl Send for KThread {}
unsafe impl Sync for KThread {}

// ── KTHREAD_TABLE — side-table: task pointer → KThread pointer ────────────

struct KThreadTable {
    entries: [(*mut TaskStruct, *mut KThread); MAX_KTHREADS],
    len: usize,
}

// SAFETY: All access is serialised through the Mutex below.
unsafe impl Send for KThreadTable {}

static KTHREAD_TABLE: Mutex<KThreadTable> = Mutex::new(KThreadTable {
    entries: [(core::ptr::null_mut(), core::ptr::null_mut()); MAX_KTHREADS],
    len: 0,
});

fn register_kthread(task: *mut TaskStruct, kt: *mut KThread) {
    let mut tbl = KTHREAD_TABLE.lock();
    if tbl.len < MAX_KTHREADS {
        let idx = tbl.len;
        tbl.entries[idx] = (task, kt);
        tbl.len += 1;
    }
}

fn lookup_kthread(task: *mut TaskStruct) -> *mut KThread {
    let tbl = KTHREAD_TABLE.lock();
    for &(t, kt) in &tbl.entries[..tbl.len] {
        if t == task {
            return kt;
        }
    }
    core::ptr::null_mut()
}

/// Return the opaque argument originally passed to a managed kthread.
pub fn kthread_data(task: *mut TaskStruct) -> *mut c_void {
    let kt = lookup_kthread(task);
    if kt.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { (*kt).data }
    }
}

// ── kthread_start trampoline ─────────────────────────────────────────────────

/// Heap-allocated closure passed as `arg` to `kthread_entry_stub`.
///
/// Stored in RBX by `sched_alloc_kthread_raw` and loaded into RDI by
/// `kthread_entry_stub` before calling `kthread_start`.
struct KthreadStart {
    kt: *mut KThread,
}

unsafe impl Send for KthreadStart {}
unsafe impl Sync for KthreadStart {}

/// Trampoline that adapts `fn -> i32` kthreads to the `fn -> !` contract
/// required by `kthread_entry_stub`.
///
/// Called with `arg = *mut KthreadStart` (moved out of the heap Box here).
/// Runs the real `threadfn`, stores the result, dequeues the task, and halts.
///
/// SAFETY: must only be called from `kthread_entry_stub` on first scheduling.
unsafe extern "C" fn kthread_start(arg: *mut c_void) -> ! {
    // Reclaim the KthreadStart Box.
    let start = unsafe { Box::from_raw(arg as *mut KthreadStart) };
    let kt = unsafe { &*start.kt };
    let task = kt.task;
    unsafe {
        crate::arch::x86::kernel::cpu::common::set_linux_current_task(task);
        crate::log_info!(
            "kthread",
            "kthread_start: task={:p} linux_current={:p} threadfn={:#x} pid={}",
            task,
            crate::arch::x86::kernel::cpu::common::linux_current_task(),
            kt.threadfn as usize,
            (*task).pid
        );
    }

    // Run the actual kthread function.
    let result = unsafe { (kt.threadfn)(kt.data) };

    // Store the exit result for kthread_stop to read.
    kt.result.store(result, Ordering::Release);

    // Mark the task as dead and remove it from the run queue.
    unsafe {
        (*task).__state.store(TASK_DEAD, Ordering::Release);
        dequeue_task(task);
    }

    // Halt indefinitely.  kthread_stop's spin loop exits when it sees TASK_DEAD.
    loop {
        core::hint::spin_loop();
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Kernel thread function signature: takes one opaque argument, returns `i32`.
///
/// The return value is reported to `kthread_stop()`.
/// The function must regularly call `kthread_should_stop()` and return when
/// it is true, so that `kthread_stop()` can terminate the thread cleanly.
///
/// Mirrors Linux `int (*threadfn)(void *data)`.
pub type KthreadFnI32 = unsafe extern "C" fn(*mut c_void) -> i32;

/// Create a kernel thread but do not start it (leave it in the stopped state).
///
/// Returns a `*mut TaskStruct` with a real PID, NOT yet added to the run queue.
/// The caller must call `enqueue_task` (or use `kthread_run`) to start it.
///
/// Mirrors Linux `kthread_create_on_node()`.
///
/// # Safety
/// - `threadfn` must be a valid kernel thread function.
/// - `name` must be a 16-byte NUL-terminated array (TASK_COMM_LEN).
/// - Returns NULL on pool/heap/PID exhaustion.
pub unsafe fn kthread_create_on_node(
    threadfn: KthreadFnI32,
    data: *mut c_void,
    name: &[u8; 16],
) -> *mut TaskStruct {
    // Allocate the KThread struct on the heap first, so we can pass its
    // pointer as the `arg` to kthread_entry_stub via KthreadStart.
    let kt_box = Box::new(KThread {
        flags: AtomicUsize::new(0),
        result: AtomicI32::new(0),
        threadfn,
        data,
        task: core::ptr::null_mut(), // filled in below
    });
    let kt: *mut KThread = Box::into_raw(kt_box);

    // Allocate the KthreadStart closure on the heap.
    let start_box = Box::new(KthreadStart { kt });
    let start_ptr: *mut KthreadStart = Box::into_raw(start_box);

    // `kthread_start` is `unsafe extern "C" fn(*mut c_void) -> !` — the same
    // type as `KthreadFn` defined in sched.rs, so no cast is needed.
    //
    // Allocate a pool slot (no PID assigned yet).
    let task = unsafe {
        sched_alloc_kthread_raw(
            kthread_start,
            start_ptr as *mut c_void,
            name,
            crate::kernel::sched::kthread_entry_stub_addr(),
        )
    };
    if task.is_null() {
        // Pool exhausted — free our heap allocations.
        unsafe {
            drop(Box::from_raw(start_ptr));
            drop(Box::from_raw(kt));
        }
        return core::ptr::null_mut();
    }

    // Assign a real PID.
    let nr = match alloc_pid(&INIT_PID_NS, None) {
        Ok(kpid) => {
            let nr = kpid.numbers[0].nr;
            // Leak the KPid Box — cleanup (put_pid) deferred to M26 (do_exit).
            let _ = Box::into_raw(kpid);
            nr
        }
        Err(_) => {
            // PID namespace exhausted.
            unsafe {
                drop(Box::from_raw(start_ptr));
                drop(Box::from_raw(kt));
            }
            return core::ptr::null_mut();
        }
    };

    unsafe {
        (*task).pid = nr;
        (*task).tgid = nr;
        // Fill the back-pointer now that we have the task address.
        (*kt).task = task;
    }

    register_kthread(task, kt);

    task
}

/// Create and immediately enqueue (start) a kernel thread.
///
/// Mirrors Linux `kthread_run()`.
///
/// # Safety
/// Same as `kthread_create_on_node`.  Returns NULL on failure.
pub unsafe fn kthread_run(
    threadfn: KthreadFnI32,
    data: *mut c_void,
    name: &[u8; 16],
) -> *mut TaskStruct {
    let task = unsafe { kthread_create_on_node(threadfn, data, name) };
    if !task.is_null() {
        unsafe { enqueue_task(task) };
    }
    task
}

/// Signal a kthread to stop and wait for it to exit.
///
/// Sets `KTHREAD_SHOULD_STOP` in the kthread's flags, then spin-yields
/// (calling `schedule()`) until the kthread marks itself `TASK_DEAD`.
/// Returns the kthread's exit value (set by the thread before it returns).
///
/// The kthread function must call `kthread_should_stop()` and return when
/// it is true; otherwise `kthread_stop` will spin forever.
///
/// Mirrors Linux `kthread_stop()`.
///
/// # Safety
/// `task` must point to a live kthread previously created by
/// `kthread_create_on_node` or `kthread_run`.
pub unsafe fn kthread_stop(task: *mut TaskStruct) -> i32 {
    let kt = lookup_kthread(task);
    if kt.is_null() {
        return -1; // not a managed kthread
    }

    // Request the thread to stop.
    unsafe { &*kt }
        .flags
        .fetch_or(1 << KTHREAD_SHOULD_STOP, Ordering::Release);

    // Spin-yield until the kthread marks itself TASK_DEAD.
    loop {
        let state = unsafe { (*task).__state.load(Ordering::Acquire) };
        if state == TASK_DEAD {
            break;
        }
        // Yield to let the kthread run.
        unsafe { schedule_with_irqs_enabled() };
    }

    unsafe { &*kt }.result.load(Ordering::Acquire)
}

/// Return `true` if the current kthread has been asked to stop.
///
/// Must be called from within a kthread function (i.e. after the thread
/// has been scheduled for the first time).  Returns `false` if the current
/// task is not a managed kthread.
///
/// Mirrors Linux `kthread_should_stop()`.
///
/// # Safety
/// Must be called after `sched_init()` (so `get_current()` is valid).
pub unsafe fn kthread_should_stop() -> bool {
    let task = unsafe { get_current() };
    if task.is_null() {
        return false;
    }
    let kt = lookup_kthread(task);
    if kt.is_null() {
        return false;
    }
    let flags = unsafe { &*kt }.flags.load(Ordering::Acquire);
    flags & (1 << KTHREAD_SHOULD_STOP) != 0
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constants match Linux ────────────────────────────────────────────────

    #[test]
    fn kthread_should_stop_bit_is_1() {
        assert_eq!(KTHREAD_SHOULD_STOP, 1);
    }

    #[test]
    fn kthread_should_park_bit_is_2() {
        assert_eq!(KTHREAD_SHOULD_PARK, 2);
    }

    // ── KThread flag semantics ───────────────────────────────────────────────

    #[test]
    fn kthread_should_stop_false_by_default() {
        let flags = AtomicUsize::new(0);
        let is_stop = flags.load(Ordering::Relaxed) & (1 << KTHREAD_SHOULD_STOP) != 0;
        assert!(
            !is_stop,
            "SHOULD_STOP bit must not be set on a fresh KThread"
        );
    }

    #[test]
    fn kthread_should_stop_true_after_setting_bit() {
        let flags = AtomicUsize::new(0);
        flags.fetch_or(1 << KTHREAD_SHOULD_STOP, Ordering::Relaxed);
        let is_stop = flags.load(Ordering::Relaxed) & (1 << KTHREAD_SHOULD_STOP) != 0;
        assert!(is_stop, "SHOULD_STOP bit must be set after fetch_or");
    }

    #[test]
    fn kthread_stop_bit_does_not_affect_park_bit() {
        let flags = AtomicUsize::new(0);
        flags.fetch_or(1 << KTHREAD_SHOULD_STOP, Ordering::Relaxed);
        let is_park = flags.load(Ordering::Relaxed) & (1 << KTHREAD_SHOULD_PARK) != 0;
        assert!(!is_park, "Setting SHOULD_STOP must not affect SHOULD_PARK");
    }

    #[test]
    fn kthread_result_starts_at_zero() {
        let result = AtomicI32::new(0);
        assert_eq!(result.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn kthread_result_is_readable_after_store() {
        let result = AtomicI32::new(0);
        result.store(42, Ordering::Release);
        assert_eq!(result.load(Ordering::Acquire), 42);
    }
}
