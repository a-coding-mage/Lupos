//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched/wait.c
//! test-origin: linux:vendor/linux/kernel/sched/wait.c
//! Scheduler wait queues.
//!
//! Mirrors `vendor/linux/kernel/sched/wait.c`. This is the generic wait queue
//! primitive used by scheduler-owned blocking paths; process wait4/waitid lives
//! separately in `kernel::wait`.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, Ordering, fence};

use crate::kernel::locking::RawSpinLocked;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::task::{TaskStruct, task_state};

const WQ_FLAG_WOKEN: u32 = 0x02;

#[repr(C)]
struct LinuxListHead {
    next: *mut LinuxListHead,
    prev: *mut LinuxListHead,
}

type LinuxWaitQueueFunc =
    unsafe extern "C" fn(*mut LinuxWaitQueueEntry, u32, i32, *mut c_void) -> i32;

#[repr(C)]
struct LinuxWaitQueueEntry {
    flags: u32,
    private: *mut c_void,
    func: Option<LinuxWaitQueueFunc>,
    entry: LinuxListHead,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "default_wake_function",
        linux_default_wake_function as usize,
        false,
    );
    export_symbol_once(
        "woken_wake_function",
        linux_woken_wake_function as usize,
        false,
    );
}

pub struct WaitQueueHead {
    /// Linux `struct wait_queue_head::lock` is a spinlock, not a sleepable
    /// mutex.  The irqsave acquisition also disables preemption while the
    /// callback/list walk is in progress.
    waiters: RawSpinLocked<Vec<WaitQueueEntry>>,
}

/// Poll wakeups carry Linux's `key_to_poll()` mask to the callback.  A
/// callback which registered interest in EPOLLIN must not be woken by an
/// EPOLLOUT-only event on the same waitqueue (eventfd is the common case).
pub type WaitQueueCallback = fn(usize, usize, u32);

enum WaitQueueEntry {
    Task {
        task: *mut TaskStruct,
        /// `poll_wqueues.triggered` for poll/select registrations.  Generic
        /// wait-event entries leave this unset.
        triggered: Option<Arc<AtomicBool>>,
    },
    Callback {
        id: usize,
        callback: WaitQueueCallback,
        data1: usize,
        data2: usize,
    },
}

unsafe impl Send for WaitQueueHead {}
unsafe impl Sync for WaitQueueHead {}

impl WaitQueueHead {
    pub const fn new() -> Self {
        Self {
            waiters: RawSpinLocked::new(Vec::new()),
        }
    }

    fn with_waiters<R>(&self, f: impl FnOnce(&mut Vec<WaitQueueEntry>) -> R) -> R {
        // Linux `__wake_up_common_lock()` and the waitqueue registration
        // helpers use `spin_lock_irqsave()`.  In addition to masking local
        // IRQs, the raw-spin wrapper disables preemption, so a scheduler
        // transition cannot strand a held waitqueue lock on another task's
        // stack.
        let (mut waiters, flags) = self.waiters.lock_irqsave();
        let result = f(&mut waiters);
        RawSpinLocked::unlock_irqrestore(waiters, flags);
        result
    }

    pub fn len(&self) -> usize {
        self.with_waiters(|waiters| waiters.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub unsafe fn prepare_to_wait(&self, task: *mut TaskStruct, state: u32) {
        if task.is_null() {
            return;
        }
        self.with_waiters(|waiters| {
            if !waiters.iter().any(|queued| {
                matches!(queued, WaitQueueEntry::Task { task: queued, .. } if *queued == task)
            }) {
                waiters.push(WaitQueueEntry::Task {
                    task,
                    triggered: None,
                });
            }
            unsafe {
                (*task).__state.store(state, Ordering::SeqCst);
            }
        });
    }

    /// Install the callback state used by Linux `poll_wait()` without changing
    /// the polling task's scheduler state.  `poll_schedule_timeout()` owns the
    /// later RUNNING -> INTERRUPTIBLE transition and checks this sticky flag to
    /// close the registration/sleep race.
    pub unsafe fn add_poll_wait(&self, task: *mut TaskStruct, triggered: Arc<AtomicBool>) {
        if task.is_null() {
            return;
        }
        self.with_waiters(|waiters| {
            if let Some(entry) = waiters.iter_mut().find(|entry| {
                matches!(entry, WaitQueueEntry::Task { task: queued, .. } if *queued == task)
            }) {
                if let WaitQueueEntry::Task {
                    triggered: entry_triggered,
                    ..
                } = entry
                {
                    *entry_triggered = Some(triggered);
                }
            } else {
                waiters.push(WaitQueueEntry::Task {
                    task,
                    triggered: Some(triggered),
                });
            }
        });
        fence(Ordering::SeqCst);
    }

    /// Install a persistent poll callback, as eventpoll's `ep_ptable_queue_proc`
    /// does for every waitqueue exposed by the watched file.
    pub fn add_callback(&self, id: usize, callback: WaitQueueCallback, data1: usize, data2: usize) {
        self.with_waiters(|waiters| {
            if waiters.iter().any(
                |entry| matches!(entry, WaitQueueEntry::Callback { id: queued, .. } if *queued == id),
            ) {
                return;
            }
            waiters.push(WaitQueueEntry::Callback {
                id,
                callback,
                data1,
                data2,
            });
        });
        fence(Ordering::SeqCst);
    }

    pub fn remove_callback(&self, id: usize) {
        self.with_waiters(|waiters| {
            waiters.retain(|entry| {
                !matches!(entry, WaitQueueEntry::Callback { id: queued, .. } if *queued == id)
            });
        });
    }

    pub unsafe fn finish_wait(&self, task: *mut TaskStruct) {
        if task.is_null() {
            return;
        }

        // Linux `finish_wait()` publishes TASK_RUNNING before it takes the
        // waitqueue lock.  A concurrent `try_to_wake_up()` must therefore see
        // that this task has already resumed and return without waiting for an
        // `on_cpu` handoff while the waiter lock is held elsewhere.
        // Ref: vendor/linux/kernel/sched/wait.c:375-396.
        unsafe {
            (*task)
                .__state
                .store(task_state::TASK_RUNNING, Ordering::Release);
        }
        self.with_waiters(|waiters| {
            if let Some(pos) = waiters.iter().position(|queued| {
                matches!(queued, WaitQueueEntry::Task { task: queued, .. } if *queued == task)
            }) {
                waiters.remove(pos);
            }
        });
    }

    fn wake_callbacks_locked(waiters: &mut Vec<WaitQueueEntry>, key: u32) {
        let mut last_id = None;
        loop {
            let next = waiters
                .iter()
                .filter_map(|entry| match entry {
                    WaitQueueEntry::Callback {
                        id,
                        callback,
                        data1,
                        data2,
                    } if last_id.is_none_or(|last| *id > last) => {
                        Some((*id, *callback, *data1, *data2))
                    }
                    _ => None,
                })
                .min_by_key(|(id, _, _, _)| *id);
            let Some((id, callback, data1, data2)) = next else {
                break;
            };
            last_id = Some(id);
            callback(data1, data2, key);
        }
    }

    /// Remove task entries while the waitqueue lock is held.  Linux's
    /// `autoremove_wake_function()` makes the corresponding list entry empty
    /// before the waiter can run `finish_wait()` and use its careful
    /// lockless-empty check.
    fn take_tasks_locked(
        waiters: &mut Vec<WaitQueueEntry>,
    ) -> Vec<(*mut TaskStruct, Option<Arc<AtomicBool>>)> {
        let mut pending = Vec::new();
        while let Some(pos) = waiters
            .iter()
            .rposition(|entry| matches!(entry, WaitQueueEntry::Task { .. }))
        {
            let entry = waiters.remove(pos);
            if let WaitQueueEntry::Task { task, triggered } = entry {
                pending.push((task, triggered));
            }
        }
        pending
    }

    /// Run scheduler wakeups after releasing the waitqueue lock.
    ///
    /// Linux can call `try_to_wake_up()` while `__wake_up_common()` owns the
    /// queue lock because `finish_wait()` can observe the already-removed
    /// intrusive list entry without taking that lock again.  The Rust queue
    /// stores entries in a locked `Vec`, so it cannot safely reproduce that
    /// lockless list observation.  Deferring the wake until after the locked
    /// removal is the equivalent lifetime/order boundary and prevents the
    /// waker from waiting for `on_cpu` while a resumed waiter waits for this
    /// queue lock.
    fn wake_tasks(pending: Vec<(*mut TaskStruct, Option<Arc<AtomicBool>>)>) -> usize {
        let mut count = 0;
        for (task, triggered) in pending {
            if let Some(triggered) = triggered {
                triggered.store(true, Ordering::SeqCst);
            }
            unsafe {
                crate::kernel::sched::wake_task_normal(task);
            }
            count += 1;
        }
        count
    }

    pub fn wake_up_one(&self) -> Option<*mut TaskStruct> {
        let (mut waiters, flags) = self.waiters.lock_irqsave();
        Self::wake_callbacks_locked(&mut waiters, u32::MAX);
        let entry = waiters
            .iter()
            .rposition(|entry| matches!(entry, WaitQueueEntry::Task { .. }))
            .map(|pos| waiters.remove(pos));
        RawSpinLocked::unlock_irqrestore(waiters, flags);
        if let Some(WaitQueueEntry::Task { task, triggered }) = entry {
            if let Some(triggered) = triggered {
                triggered.store(true, Ordering::SeqCst);
            }
            unsafe {
                crate::kernel::sched::wake_task_normal(task);
            }
            return Some(task);
        }
        None
    }

    /// Wake waiters with a Linux poll/event key.  `u32::MAX` is the local
    /// equivalent of a wakeup without a poll key and preserves the generic
    /// `wake_up_all()` behavior for queues whose producer has no mask.
    pub fn wake_up_poll(&self, key: u32) -> usize {
        let (mut waiters, flags) = self.waiters.lock_irqsave();
        Self::wake_callbacks_locked(&mut waiters, key);
        let pending = Self::take_tasks_locked(&mut waiters);
        // Keep the irqsave state across the deferred scheduler calls, just
        // as Linux keeps IRQs disabled while __wake_up_common() invokes its
        // wake functions.  The queue lock itself must already be released so
        // finish_wait() cannot deadlock behind a wakeup's on_cpu handoff.
        drop(waiters);
        let count = Self::wake_tasks(pending);
        crate::kernel::locking::local_irq_restore(flags);
        count
    }

    /// Apply a producer-side state transition while holding the waitqueue
    /// lock, then publish the matching poll wake before releasing it.
    ///
    /// Linux eventfd keeps `ctx->count` and `wqh` under the same
    /// `spin_lock_irqsave()` critical section. The boolean says whether the
    /// transition changed the state and therefore needs a wakeup; returning
    /// the operation's value keeps this helper useful for both read and write
    /// paths without exposing the waitqueue storage.
    pub fn update_and_wake_poll<R>(&self, key: u32, update: impl FnOnce() -> (R, bool)) -> R {
        let (mut waiters, flags) = self.waiters.lock_irqsave();
        let (result, wake) = update();
        let pending = if wake {
            Self::wake_callbacks_locked(&mut waiters, key);
            Self::take_tasks_locked(&mut waiters)
        } else {
            Vec::new()
        };
        drop(waiters);
        Self::wake_tasks(pending);
        crate::kernel::locking::local_irq_restore(flags);
        result
    }

    pub fn wake_up_all(&self) -> usize {
        self.wake_up_poll(u32::MAX)
    }
}

pub unsafe fn prepare_to_wait(queue: &WaitQueueHead, task: *mut TaskStruct, state: u32) {
    unsafe { queue.prepare_to_wait(task, state) };
}

pub unsafe fn finish_wait(queue: &WaitQueueHead, task: *mut TaskStruct) {
    unsafe { queue.finish_wait(task) };
}

pub fn wake_up(queue: &WaitQueueHead) -> usize {
    queue.wake_up_all()
}

/// `default_wake_function` - `vendor/linux/kernel/sched/core.c:7564`.
unsafe extern "C" fn linux_default_wake_function(
    entry: *mut LinuxWaitQueueEntry,
    mode: u32,
    wake_flags: i32,
    _key: *mut c_void,
) -> i32 {
    if entry.is_null() {
        return 0;
    }
    let task = unsafe { (*entry).private.cast::<TaskStruct>() };
    if task.is_null() {
        return 0;
    }
    let state = unsafe { (*task).__state.load(Ordering::Acquire) };
    if state & mode == 0 {
        return 0;
    }
    unsafe { crate::kernel::sched::wake_task_with_state(task, mode, wake_flags as u32) as i32 }
}

/// `woken_wake_function` - `vendor/linux/kernel/sched/wait.c:457`.
unsafe extern "C" fn linux_woken_wake_function(
    entry: *mut LinuxWaitQueueEntry,
    mode: u32,
    wake_flags: i32,
    key: *mut c_void,
) -> i32 {
    if entry.is_null() {
        return 0;
    }
    fence(Ordering::SeqCst);
    unsafe {
        (*entry).flags |= WQ_FLAG_WOKEN;
        linux_default_wake_function(entry, mode, wake_flags, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use core::sync::atomic::AtomicU32;

    fn task() -> Box<TaskStruct> {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.__state = AtomicU32::new(task_state::TASK_RUNNING);
        task.m29.sched_class = &crate::kernel::sched::fair::FAIR_SCHED_CLASS;
        task
    }

    #[test]
    fn wait_queue_prepare_and_wake_all() {
        let q = WaitQueueHead::new();
        let mut t = task();
        unsafe {
            q.prepare_to_wait(&mut *t, task_state::TASK_UNINTERRUPTIBLE);
        }
        assert_eq!(q.len(), 1);
        assert_eq!(
            t.__state.load(Ordering::Acquire),
            task_state::TASK_UNINTERRUPTIBLE
        );
        assert_eq!(q.wake_up_all(), 1);
        assert_eq!(t.__state.load(Ordering::Acquire), task_state::TASK_RUNNING);
        assert!(q.is_empty());
    }

    #[test]
    fn waitqueue_wake_exports_match_linux_source_contract() {
        let wait_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/sched/wait.c"
        ));
        let core_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/sched/core.c"
        ));
        assert!(core_source.contains("int default_wake_function"));
        assert!(core_source.contains("return try_to_wake_up(curr->private, mode, wake_flags);"));
        assert!(wait_source.contains("wq_entry->flags |= WQ_FLAG_WOKEN;"));
        assert!(wait_source.contains("EXPORT_SYMBOL(woken_wake_function);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("default_wake_function"),
            Some(linux_default_wake_function as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("woken_wake_function"),
            Some(linux_woken_wake_function as usize)
        );
    }

    /// Linux's waitqueue lock is a `spinlock_t` acquired with IRQ-save
    /// semantics in `add_wait_queue()`, `remove_wait_queue()`, and
    /// `__wake_up_common_lock()`.  A sleepable/external mutex here can be
    /// retained across a scheduler transition and stall unrelated CPUs.
    ///
    /// test-origin: linux:vendor/linux/kernel/sched/wait.c:add_wait_queue
    /// test-origin: linux:vendor/linux/kernel/sched/wait.c:__wake_up_common_lock
    #[test]
    fn waitqueue_storage_uses_raw_spin_irqsave_lock() {
        let source = include_str!("wait.rs");
        assert!(source.contains("waiters: RawSpinLocked<Vec<WaitQueueEntry>>"));
        assert!(source.contains("self.waiters.lock_irqsave()"));
        assert!(source.contains("RawSpinLocked::unlock_irqrestore"));
    }

    /// test-origin: linux:vendor/linux/kernel/sched/wait.c:finish_wait
    ///
    /// Linux changes the task state before taking the waitqueue lock.  The
    /// ordering is observable: a concurrent wake must see TASK_RUNNING while
    /// the resumed task is removing its entry, rather than waiting for its
    /// `on_cpu` handoff while another CPU owns the queue lock.
    #[test]
    fn finish_wait_publishes_running_before_locking_the_queue() {
        let source = include_str!("wait.rs");
        let body = source
            .split("pub unsafe fn finish_wait")
            .nth(1)
            .and_then(|body| body.split("fn wake_callbacks_locked").next())
            .expect("finish_wait body must remain present");
        let publish = body
            .find(".store(task_state::TASK_RUNNING, Ordering::Release)")
            .expect("finish_wait must publish TASK_RUNNING");
        let lock = body
            .find("self.with_waiters")
            .expect("finish_wait must remove the wait entry under its lock");
        assert!(
            publish < lock,
            "Linux finish_wait publishes TASK_RUNNING before waitqueue lock acquisition"
        );

        let mut task = task();
        task.__state
            .store(task_state::TASK_INTERRUPTIBLE, Ordering::Release);
        let queue = WaitQueueHead::new();
        unsafe { queue.prepare_to_wait(&mut *task, task_state::TASK_INTERRUPTIBLE) };
        unsafe { queue.finish_wait(&mut *task) };
        assert_eq!(
            task.__state.load(Ordering::Acquire),
            task_state::TASK_RUNNING
        );
        assert!(queue.is_empty());
    }

    /// test-origin: linux:vendor/linux/kernel/sched/wait.c:finish_wait and
    /// __wake_up_common
    ///
    /// Linux's wake entry is removed before a resumed waiter can observe an
    /// empty list and skip the waitqueue lock.  The Rust queue must likewise
    /// finish the list mutation before entering the scheduler wake path; a
    /// wake path that runs while this lock is held can deadlock with
    /// `finish_wait()` during an `on_cpu` handoff.
    #[test]
    fn waitqueue_wake_removes_tasks_before_scheduler_callback() {
        let source = include_str!("wait.rs");
        let marker = ["fn take_", "tasks_locked"].concat();
        let body = source
            .split(&marker)
            .nth(1)
            .and_then(|body| body.split("fn wake_tasks").next())
            .expect("waitqueue task extraction must remain present");
        assert!(
            !body.contains("wake_task_normal"),
            "scheduler wakeups must run after the waitqueue lock is released"
        );

        let queue = WaitQueueHead::new();
        let mut task = task();
        unsafe { queue.prepare_to_wait(&mut *task, task_state::TASK_INTERRUPTIBLE) };
        assert_eq!(queue.wake_up_all(), 1);
        unsafe { queue.finish_wait(&mut *task) };
        assert_eq!(
            task.__state.load(Ordering::Acquire),
            task_state::TASK_RUNNING
        );
    }

    static LAST_POLL_WAKE_KEY: AtomicU32 = AtomicU32::new(0);

    fn record_poll_wake_key(_data1: usize, _data2: usize, key: u32) {
        LAST_POLL_WAKE_KEY.store(key, Ordering::Release);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/wait.c:__wake_up_common
    /// and vendor/linux/fs/select.c:pollwake
    ///
    /// Linux carries the producer's poll mask through a waitqueue callback;
    /// without it an eventfd EPOLLOUT wake can spuriously requeue an EPOLLIN
    /// watcher and turn an event loop into a busy loop.
    #[test]
    fn poll_wakeup_propagates_linux_event_key() {
        let queue = WaitQueueHead::new();
        LAST_POLL_WAKE_KEY.store(0, Ordering::Release);
        queue.add_callback(17, record_poll_wake_key, 0, 0);
        queue.wake_up_poll(crate::fs::select::POLLOUT as u32);
        assert_eq!(
            LAST_POLL_WAKE_KEY.load(Ordering::Acquire),
            crate::fs::select::POLLOUT as u32
        );
    }

    #[test]
    fn woken_wake_function_sets_woken_and_wakes_matching_task_state() {
        let mut task = task();
        task.__state
            .store(task_state::TASK_UNINTERRUPTIBLE, Ordering::Release);
        let mut entry = LinuxWaitQueueEntry {
            flags: 0,
            private: (&mut *task as *mut TaskStruct).cast(),
            func: None,
            entry: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
        };

        let ret = unsafe {
            linux_woken_wake_function(
                &mut entry,
                task_state::TASK_UNINTERRUPTIBLE,
                0,
                core::ptr::null_mut(),
            )
        };

        assert_eq!(ret, 1);
        assert_eq!(entry.flags & WQ_FLAG_WOKEN, WQ_FLAG_WOKEN);
        assert_eq!(
            task.__state.load(Ordering::Acquire),
            task_state::TASK_RUNNING
        );
    }
}
