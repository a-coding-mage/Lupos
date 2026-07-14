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

use spin::Mutex;

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
    waiters: Mutex<Vec<WaitQueueEntry>>,
}

pub type WaitQueueCallback = fn(usize, usize);

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
            waiters: Mutex::new(Vec::new()),
        }
    }

    fn with_waiters<R>(&self, f: impl FnOnce(&mut Vec<WaitQueueEntry>) -> R) -> R {
        // Linux waitqueue locks are irqsave spinlocks. Every access must mask
        // local IRQs so an interrupt-side wake cannot spin on a lock held by
        // the task frame it interrupted.
        let flags = crate::kernel::locking::irqflags::local_irq_save();
        let result = {
            let mut waiters = self.waiters.lock();
            f(&mut waiters)
        };
        crate::kernel::locking::irqflags::local_irq_restore(flags);
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
        self.with_waiters(|waiters| {
            if let Some(pos) = waiters.iter().position(|queued| {
                matches!(queued, WaitQueueEntry::Task { task: queued, .. } if *queued == task)
            }) {
                waiters.remove(pos);
            }
            unsafe {
                (*task)
                    .__state
                    .store(task_state::TASK_RUNNING, Ordering::Release);
            }
        });
    }

    fn wake_callbacks(&self) {
        let mut last_id = None;
        loop {
            let next = self.with_waiters(|waiters| {
                waiters
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
                    .min_by_key(|(id, _, _, _)| *id)
            });
            let Some((id, callback, data1, data2)) = next else {
                break;
            };
            last_id = Some(id);
            callback(data1, data2);
        }
    }

    fn take_one_task(&self) -> Option<WaitQueueEntry> {
        self.with_waiters(|waiters| {
            waiters
                .iter()
                .rposition(|entry| matches!(entry, WaitQueueEntry::Task { .. }))
                .map(|pos| waiters.remove(pos))
        })
    }

    pub fn wake_up_one(&self) -> Option<*mut TaskStruct> {
        self.wake_callbacks();
        let entry = self.take_one_task();
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

    pub fn wake_up_all(&self) -> usize {
        self.wake_callbacks();
        let mut count = 0;
        while let Some(entry) = self.take_one_task() {
            if let WaitQueueEntry::Task { task, triggered } = entry {
                if let Some(triggered) = triggered {
                    triggered.store(true, Ordering::SeqCst);
                }
                unsafe {
                    crate::kernel::sched::wake_task_normal(task);
                }
                count += 1;
            }
        }
        count
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
    unsafe { crate::kernel::sched::try_to_wake_up(task, wake_flags as u32) as i32 }
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
