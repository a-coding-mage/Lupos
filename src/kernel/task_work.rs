//! linux-parity: complete
//! linux-source: vendor/linux/kernel/task_work.c
//! test-origin: linux:vendor/linux/kernel/task_work.c
//! Per-task deferred work — M28a.
//!
//! Implements the `task_work_add` / `task_work_run` mechanism Linux uses to
//! defer work to a task's return-to-userspace path.  Common callers are the
//! credential commit path (`commit_creds` → `key_change_session_keyring`),
//! the io_uring completion path, the signal subsystem (signal-frame teardown
//! after coredump), and the fput path (file close after the last reference).
//!
//! Reference: vendor/linux/kernel/task_work.c
//!            vendor/linux/include/linux/task_work.h
//!
//! # Port notes
//!
//! Linux stores the head of the work list inline on `task_struct` as
//! `task_works`.  Our `TaskStruct` layout is currently locked to a Linux 7.0
//! pahole snapshot and the `task_works` slot has not yet been carved out, so
//! this implementation keeps a side table indexed by `pid`.  The observable
//! API matches Linux: callbacks run in LIFO order (Linux task_work.c line 53
//! "the task_work list is LIFO"), `task_work_add` returns `-ESRCH` if the task
//! has already begun exiting, and `task_work_cancel_*` returns the removed
//! callback (or `None`/`false`) without invoking it.
//!
//! Notification modes (`TWA_RESUME`, `TWA_SIGNAL`, `TWA_SIGNAL_NO_IPI`,
//! `TWA_NMI_CURRENT`) preserve their Linux side effects. `TWA_SIGNAL` also
//! kicks a remote target CPU so its return path observes the pending work.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

use crate::kernel::task::TaskStruct;

// ── Linux ABI constants ──────────────────────────────────────────────────────

/// `enum task_work_notify_mode` from `include/linux/task_work.h`.
///
/// Layout matches Linux: TWA_NONE=0, TWA_RESUME=1, TWA_SIGNAL=2,
/// TWA_SIGNAL_NO_IPI=3, TWA_NMI_CURRENT=4.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskWorkNotify {
    None = 0,
    Resume = 1,
    Signal = 2,
    SignalNoIpi = 3,
    NmiCurrent = 4,
}

impl TaskWorkNotify {
    /// Map a raw integer from userspace/syscall callers to the enum.
    pub fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Resume),
            2 => Some(Self::Signal),
            3 => Some(Self::SignalNoIpi),
            4 => Some(Self::NmiCurrent),
            _ => None,
        }
    }
}

// errno values matching Linux uapi
const ESRCH: i32 = -3;
const EINVAL: i32 = -22;

// ── CallbackHead ─────────────────────────────────────────────────────────────

/// `task_work_func_t` from `include/linux/task_work.h`.
pub type TaskWorkFunc = unsafe extern "C" fn(work: *mut CallbackHead);

/// Linux `struct callback_head` from `include/linux/types.h`.
///
/// Embedded in larger callback structures via the C "container_of" idiom.
/// Public so callers can construct their own pinned instances; the runtime
/// keeps these alive as long as they are queued.
#[repr(C)]
pub struct CallbackHead {
    /// Next pointer (LIFO list).  Owned by the queue while pending.
    pub next: *mut CallbackHead,
    /// Callback executed by `task_work_run`.
    pub func: Option<TaskWorkFunc>,
}

impl CallbackHead {
    /// Construct a zeroed head with `func` set.  Mirrors
    /// `init_task_work(twork, func)` in `include/linux/task_work.h:11`.
    pub const fn new(func: TaskWorkFunc) -> Self {
        Self {
            next: core::ptr::null_mut(),
            func: Some(func),
        }
    }
}

// SAFETY: CallbackHead carries a raw next pointer; the queue locks below
// serialize access.
unsafe impl Send for CallbackHead {}
unsafe impl Sync for CallbackHead {}

// ── Per-task list state ──────────────────────────────────────────────────────

struct TaskEntry {
    pid: i32,
    /// LIFO head pointer of pending callbacks.  Null when no work pending.
    head: *mut CallbackHead,
    /// Mirrors Linux `PF_EXITING`: once set, further `task_work_add` calls
    /// observe `&WORK_EXITED` and return `-ESRCH`.
    exiting: bool,
    /// Last notification mode requested by a caller.  Recorded for parity
    /// with Linux's `set_notify_resume` / `set_notify_signal` side effects.
    last_notify: TaskWorkNotify,
}

// SAFETY: TaskEntry holds raw pointers serialized by the TABLE lock.
unsafe impl Send for TaskEntry {}

struct Table {
    entries: Vec<TaskEntry>,
}

impl Table {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn slot_mut(&mut self, pid: i32) -> &mut TaskEntry {
        if let Some(pos) = self.entries.iter().position(|e| e.pid == pid) {
            return &mut self.entries[pos];
        }
        self.entries.push(TaskEntry {
            pid,
            head: core::ptr::null_mut(),
            exiting: false,
            last_notify: TaskWorkNotify::None,
        });
        self.entries.last_mut().expect("just pushed")
    }

    fn try_slot_mut(&mut self, pid: i32) -> Option<&mut TaskEntry> {
        let pos = self.entries.iter().position(|e| e.pid == pid)?;
        Some(&mut self.entries[pos])
    }
}

static TABLE: Mutex<Table> = Mutex::new(Table::new());

/// True after `task_work_run` has been entered on at least one task — used
/// only by tests to detect re-entrancy.
static IN_RUN: AtomicBool = AtomicBool::new(false);

// ── Public API ───────────────────────────────────────────────────────────────

/// True when `task` has at least one callback queued.
///
/// Linux: `task_work_pending` from `include/linux/task_work.h:24`.
///
/// # Safety
/// `task` must be a valid pointer or null.
pub unsafe fn task_work_pending(task: *const TaskStruct) -> bool {
    if task.is_null() {
        return false;
    }
    let pid = unsafe { (*task).pid };
    let table = TABLE.lock();
    table
        .entries
        .iter()
        .find(|e| e.pid == pid)
        .map(|e| !e.head.is_null())
        .unwrap_or(false)
}

/// Queue `work` on `task` and request notification per `notify`.
///
/// Linux: `task_work_add` from `kernel/task_work.c:59`.  Returns 0 on success
/// or `-ESRCH` if the task is exiting.  Returns `-EINVAL` for
/// `TWA_NMI_CURRENT` when the caller is not the current task (matching the
/// Linux WARN_ON_ONCE branch).
///
/// # Safety
/// `work` must outlive its callback invocation — the caller (or the callback
/// itself) is responsible for freeing the backing storage.
pub unsafe fn task_work_add(
    task: *mut TaskStruct,
    work: *mut CallbackHead,
    notify: TaskWorkNotify,
) -> i32 {
    if task.is_null() || work.is_null() {
        return EINVAL;
    }
    let pid = unsafe { (*task).pid };

    if notify == TaskWorkNotify::NmiCurrent {
        let current = unsafe { crate::kernel::sched::get_current() };
        if current != task {
            return EINVAL;
        }
    }

    let mut table = TABLE.lock();
    let slot = table.slot_mut(pid);
    if slot.exiting {
        return ESRCH;
    }
    // LIFO push.
    unsafe { (*work).next = slot.head };
    slot.head = work;
    slot.last_notify = notify;
    drop(table);

    // Notification side effects.  Under the cooperative scheduler the target
    // will pick the work up on its next schedule; the Resume/Signal modes
    // additionally set TIF_SIGPENDING so the syscall-exit slow path notices.
    //
    // Linux: set_notify_resume() raises TIF_NOTIFY_RESUME, set_notify_signal()
    // raises TIF_NOTIFY_SIGNAL.  Our arch glue folds both onto TIF_SIGPENDING.
    match notify {
        TaskWorkNotify::None => {}
        TaskWorkNotify::Resume | TaskWorkNotify::NmiCurrent => {
            crate::kernel::signal::set_tif_sigpending(task);
        }
        TaskWorkNotify::Signal => {
            crate::kernel::signal::set_tif_sigpending(task);
            // Linux: kick_process / smp_send_reschedule on the remote CPU so
            // the target re-enters the kernel and runs task_work_run from
            // its syscall-exit / interrupt-exit slow path.
            unsafe { kick_remote_task(task) };
        }
        TaskWorkNotify::SignalNoIpi => {
            crate::kernel::signal::set_tif_sigpending(task);
            // Linux variant: skip the IPI; rely on the next kernel entry.
        }
    }
    0
}

/// Send a reschedule IPI to the CPU `task` is currently running on so it
/// drops out of userspace and runs task_work on the next exit.  Linux:
/// `kick_process` / `smp_send_reschedule`.
///
/// # Safety
/// `task` must be a valid TaskStruct pointer.
unsafe fn kick_remote_task(task: *mut TaskStruct) {
    #[cfg(not(test))]
    unsafe {
        if task.is_null() {
            return;
        }
        // Linux's kick_process() pins the caller, reads task_cpu(), and sends
        // only while the target is currently running. `on_cpu` is a boolean
        // handoff flag; `wake_cpu` carries the dense logical CPU number.
        crate::kernel::locking::preempt::preempt_disable();
        let target_cpu = (*task).m29.wake_cpu;
        let current_cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number() as i32;
        if (0..crate::kernel::sched::MAX_CPUS as i32).contains(&target_cpu)
            && target_cpu != current_cpu
            && crate::kernel::sched::task_on_cpu(task)
        {
            crate::arch::x86::kernel::idt::send_reschedule_ipi(target_cpu as u8);
        }
        crate::kernel::locking::preempt::preempt_enable();
    }
    #[cfg(test)]
    {
        // Host tests cannot send real IPIs; record the request for
        // assertion-based verification.
        let _ = task;
        IPI_TEST_HOOK
            .lock()
            .fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    }
}

#[cfg(test)]
pub static IPI_TEST_HOOK: spin::Mutex<core::sync::atomic::AtomicI64> =
    spin::Mutex::new(core::sync::atomic::AtomicI64::new(0));

/// Remove the first callback for which `pred(cb)` returns true.
///
/// Linux: `task_work_cancel_match` from `kernel/task_work.c:115`.
pub unsafe fn task_work_cancel_match<F>(task: *mut TaskStruct, mut pred: F) -> *mut CallbackHead
where
    F: FnMut(*mut CallbackHead) -> bool,
{
    if task.is_null() {
        return core::ptr::null_mut();
    }
    let pid = unsafe { (*task).pid };
    let mut table = TABLE.lock();
    let Some(slot) = table.try_slot_mut(pid) else {
        return core::ptr::null_mut();
    };
    if slot.head.is_null() {
        return core::ptr::null_mut();
    }
    // Walk the singly-linked list with a trailing pointer.
    let mut pprev: *mut *mut CallbackHead = &mut slot.head;
    let mut work = slot.head;
    while !work.is_null() {
        if pred(work) {
            unsafe { *pprev = (*work).next };
            return work;
        }
        unsafe {
            pprev = &mut (*work).next;
            work = (*work).next;
        }
    }
    core::ptr::null_mut()
}

/// Remove the queued instance of `cb` from `task`'s list if present.
///
/// Linux: `task_work_cancel` from `kernel/task_work.c:183`.
pub unsafe fn task_work_cancel(task: *mut TaskStruct, cb: *mut CallbackHead) -> bool {
    let target = cb;
    let removed = unsafe { task_work_cancel_match(task, |w| w == target) };
    !removed.is_null() && removed == cb
}

/// Remove the last callback whose `func` matches.
///
/// Linux: `task_work_cancel_func` from `kernel/task_work.c:162`.
pub unsafe fn task_work_cancel_func(
    task: *mut TaskStruct,
    func: TaskWorkFunc,
) -> *mut CallbackHead {
    unsafe { task_work_cancel_match(task, |w| (*w).func == Some(func)) }
}

/// Drain and invoke every pending callback for the current task.
///
/// Linux: `task_work_run` from `kernel/task_work.c:200`.  Callbacks may queue
/// additional work; we repeat until the list is empty.  Once
/// `mark_current_exiting` has been called the drained list is sealed with
/// the exit sentinel so further `task_work_add` calls return `-ESRCH`.
pub fn task_work_run() {
    IN_RUN.store(true, Ordering::Release);
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        IN_RUN.store(false, Ordering::Release);
        return;
    }
    let pid = unsafe { (*task).pid };

    loop {
        // Detach the head atomically under the lock.
        let mut work = {
            let mut table = TABLE.lock();
            let Some(slot) = table.try_slot_mut(pid) else {
                break;
            };
            let head = slot.head;
            slot.head = core::ptr::null_mut();
            head
        };
        if work.is_null() {
            break;
        }
        // Invoke in LIFO order — Linux drains the list in the order it pops.
        while !work.is_null() {
            let next = unsafe { (*work).next };
            if let Some(func) = unsafe { (*work).func } {
                unsafe { func(work) };
            }
            work = next;
        }
    }

    IN_RUN.store(false, Ordering::Release);
}

/// Mark `task` as exiting so subsequent `task_work_add` calls fail with
/// `-ESRCH`.  Linux: assigning `&work_exited` to `task->task_works` near
/// `do_exit`.  Called by `exit_task_work` and by `do_exit` in the port.
///
/// # Safety
/// `task` must be valid.
pub unsafe fn mark_exiting(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    let pid = unsafe { (*task).pid };
    let mut table = TABLE.lock();
    let slot = table.slot_mut(pid);
    slot.exiting = true;
}

/// Drain remaining work and mark the task exiting.  Linux:
/// `exit_task_work` in `include/linux/task_work.h:38`.
pub fn exit_task_work() {
    task_work_run();
    let task = unsafe { crate::kernel::sched::get_current() };
    if !task.is_null() {
        unsafe { mark_exiting(task) };
    }
}

// ── Test helpers ─────────────────────────────────────────────────────────────

#[cfg(test)]
pub fn reset_for_tests() {
    TABLE.lock().entries.clear();
    IN_RUN.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::boxed::Box;
    use core::sync::atomic::{AtomicU32, Ordering as Ord};

    use super::*;
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    static ORDER: spin::Mutex<alloc::vec::Vec<u32>> = spin::Mutex::new(alloc::vec::Vec::new());

    #[repr(C)]
    struct TaggedWork {
        head: CallbackHead,
        tag: u32,
    }

    unsafe extern "C" fn record_cb(work: *mut CallbackHead) {
        let tagged = work as *mut TaggedWork;
        let tag = unsafe { (*tagged).tag };
        COUNTER.fetch_add(1, Ord::Release);
        ORDER.lock().push(tag);
    }

    fn make_current_task(pid: i32) -> Box<TaskStruct> {
        let mut t = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        t.pid = pid;
        t.tgid = pid;
        t.cred = &raw const INIT_CRED;
        t
    }

    #[test]
    fn add_then_run_invokes_callback_lifo() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        COUNTER.store(0, Ord::Release);
        ORDER.lock().clear();

        let mut task = make_current_task(101);
        let prev = unsafe { sched::get_current() };
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        let mut w1 = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 1,
        });
        let mut w2 = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 2,
        });
        let mut w3 = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 3,
        });

        unsafe {
            assert_eq!(
                task_work_add(
                    &mut *task,
                    &mut w1.head as *mut CallbackHead,
                    TaskWorkNotify::None,
                ),
                0,
            );
            assert_eq!(
                task_work_add(
                    &mut *task,
                    &mut w2.head as *mut CallbackHead,
                    TaskWorkNotify::None,
                ),
                0,
            );
            assert_eq!(
                task_work_add(
                    &mut *task,
                    &mut w3.head as *mut CallbackHead,
                    TaskWorkNotify::None,
                ),
                0,
            );
            assert!(task_work_pending(&*task));
        }

        task_work_run();
        assert_eq!(COUNTER.load(Ord::Acquire), 3);
        // LIFO: w3 pushed last → invoked first.
        assert_eq!(*ORDER.lock(), alloc::vec![3, 2, 1]);
        assert!(!unsafe { task_work_pending(&*task) });

        unsafe { sched::set_current(prev) };
    }

    #[test]
    fn cancel_removes_callback_before_run() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        COUNTER.store(0, Ord::Release);

        let mut task = make_current_task(202);
        let prev = unsafe { sched::get_current() };
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        let mut w1 = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 1,
        });
        let mut w2 = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 2,
        });

        unsafe {
            task_work_add(
                &mut *task,
                &mut w1.head as *mut CallbackHead,
                TaskWorkNotify::None,
            );
            task_work_add(
                &mut *task,
                &mut w2.head as *mut CallbackHead,
                TaskWorkNotify::None,
            );
            assert!(task_work_cancel(
                &mut *task,
                &mut w1.head as *mut CallbackHead
            ));
            // Second cancel must report false (already removed).
            assert!(!task_work_cancel(
                &mut *task,
                &mut w1.head as *mut CallbackHead
            ));
        }

        task_work_run();
        assert_eq!(COUNTER.load(Ord::Acquire), 1);

        unsafe { sched::set_current(prev) };
    }

    #[test]
    fn add_fails_after_exit_marker() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();

        let mut task = make_current_task(303);
        let prev = unsafe { sched::get_current() };
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        unsafe { mark_exiting(&mut *task) };
        let mut w = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 9,
        });
        let ret = unsafe {
            task_work_add(
                &mut *task,
                &mut w.head as *mut CallbackHead,
                TaskWorkNotify::None,
            )
        };
        assert_eq!(ret, ESRCH);

        unsafe { sched::set_current(prev) };
    }

    #[test]
    fn cancel_func_finds_matching_callback() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();

        let mut task = make_current_task(404);
        let prev = unsafe { sched::get_current() };
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        let mut w = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 7,
        });
        unsafe {
            task_work_add(
                &mut *task,
                &mut w.head as *mut CallbackHead,
                TaskWorkNotify::Resume,
            );
            let found = task_work_cancel_func(&mut *task, record_cb);
            assert!(!found.is_null());
            assert_eq!(found, &mut w.head as *mut CallbackHead);
        }

        unsafe { sched::set_current(prev) };
    }

    #[test]
    fn signal_notify_fires_ipi_hook() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        IPI_TEST_HOOK
            .lock()
            .store(0, core::sync::atomic::Ordering::Release);

        let mut task = make_current_task(606);
        let prev = unsafe { sched::get_current() };
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        let mut w = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 11,
        });
        unsafe {
            assert_eq!(
                task_work_add(
                    &mut *task,
                    &mut w.head as *mut CallbackHead,
                    TaskWorkNotify::Signal,
                ),
                0,
            );
        }
        // Host-side hook bumps once per Signal-mode call.
        assert_eq!(
            IPI_TEST_HOOK
                .lock()
                .load(core::sync::atomic::Ordering::Acquire),
            1,
        );

        // SignalNoIpi must NOT bump.
        let mut w2 = Box::new(TaggedWork {
            head: CallbackHead::new(record_cb),
            tag: 12,
        });
        unsafe {
            task_work_add(
                &mut *task,
                &mut w2.head as *mut CallbackHead,
                TaskWorkNotify::SignalNoIpi,
            );
        }
        assert_eq!(
            IPI_TEST_HOOK
                .lock()
                .load(core::sync::atomic::Ordering::Acquire),
            1,
        );

        unsafe { sched::set_current(prev) };
    }

    #[test]
    fn notify_mode_from_raw_round_trip() {
        for (raw, expected) in [
            (0, TaskWorkNotify::None),
            (1, TaskWorkNotify::Resume),
            (2, TaskWorkNotify::Signal),
            (3, TaskWorkNotify::SignalNoIpi),
            (4, TaskWorkNotify::NmiCurrent),
        ] {
            assert_eq!(TaskWorkNotify::from_raw(raw), Some(expected));
        }
        assert_eq!(TaskWorkNotify::from_raw(99), None);
    }
}
