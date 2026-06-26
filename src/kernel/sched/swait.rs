//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/swait.c
//! test-origin: linux:vendor/linux/kernel/sched/swait.c
//! Simple wait queues.
//!
//! Mirrors `vendor/linux/kernel/sched/swait.c`. Linux `swait_queue_head` is a
//! lighter wait queue for exclusive simple sleepers; Lupos builds it over the
//! scheduler wait queue primitive while keeping the smaller API.

use core::ffi::{c_char, c_void};

use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::task::TaskStruct;

use super::wait::WaitQueueHead;

fn export_symbol_once(name: &str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__init_swait_queue_head",
        linux_init_swait_queue_head as usize,
        false,
    );
}

pub struct SwaitQueueHead {
    inner: WaitQueueHead,
}

unsafe impl Send for SwaitQueueHead {}
unsafe impl Sync for SwaitQueueHead {}

impl SwaitQueueHead {
    pub const fn new() -> Self {
        Self {
            inner: WaitQueueHead::new(),
        }
    }

    pub fn active(&self) -> bool {
        !self.inner.is_empty()
    }

    pub unsafe fn prepare(&self, task: *mut TaskStruct, state: u32) {
        unsafe {
            self.inner.prepare_to_wait(task, state);
        }
    }

    pub unsafe fn finish(&self, task: *mut TaskStruct) {
        unsafe {
            self.inner.finish_wait(task);
        }
    }

    pub fn wake_one(&self) -> bool {
        self.inner.wake_up_one().is_some()
    }

    pub fn wake_all(&self) -> usize {
        self.inner.wake_up_all()
    }
}

pub fn swake_up_one(queue: &SwaitQueueHead) -> bool {
    queue.wake_one()
}

pub fn swake_up_all(queue: &SwaitQueueHead) -> usize {
    queue.wake_all()
}

#[repr(C)]
struct LinuxListHead {
    next: *mut c_void,
    prev: *mut c_void,
}

#[repr(C)]
struct LinuxSwaitQueueHead {
    task_list: LinuxListHead,
}

/// `__init_swait_queue_head` - `vendor/linux/kernel/sched/swait.c:7`.
#[unsafe(export_name = "__init_swait_queue_head")]
pub unsafe extern "C" fn linux_init_swait_queue_head(
    queue: *mut c_void,
    _name: *const c_char,
    _key: *mut c_void,
) {
    if queue.is_null() {
        return;
    }
    let queue = queue.cast::<LinuxSwaitQueueHead>();
    unsafe {
        let list = core::ptr::addr_of_mut!((*queue).task_list).cast::<c_void>();
        (*queue).task_list.next = list;
        (*queue).task_list.prev = list;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::task::task_state;
    use alloc::boxed::Box;
    use core::mem::{offset_of, size_of};
    use core::sync::atomic::{AtomicU32, Ordering};

    fn task() -> Box<TaskStruct> {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.__state = AtomicU32::new(task_state::TASK_RUNNING);
        task
    }

    #[test]
    fn swait_wakes_one_waiter() {
        let q = SwaitQueueHead::new();
        let mut t = task();
        unsafe {
            q.prepare(&mut *t, task_state::TASK_INTERRUPTIBLE);
        }
        assert!(q.active());
        assert!(swake_up_one(&q));
        assert_eq!(t.__state.load(Ordering::Acquire), task_state::TASK_RUNNING);
        assert!(!q.active());
    }

    #[test]
    fn linux_swait_module_exports_register() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("__init_swait_queue_head"),
            Some(linux_init_swait_queue_head as usize)
        );
    }

    #[test]
    fn linux_swait_queue_head_layout_matches_configured_vendor() {
        assert_eq!(offset_of!(LinuxListHead, next), 0);
        assert_eq!(offset_of!(LinuxListHead, prev), 8);
        assert_eq!(size_of::<LinuxListHead>(), 0x10);

        assert_eq!(offset_of!(LinuxSwaitQueueHead, task_list), 0);
        assert_eq!(size_of::<LinuxSwaitQueueHead>(), 0x10);
    }

    #[test]
    fn linux_init_swait_queue_head_self_initializes_list() {
        let mut queue = LinuxSwaitQueueHead {
            task_list: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
        };
        unsafe {
            linux_init_swait_queue_head(
                (&mut queue as *mut LinuxSwaitQueueHead).cast(),
                core::ptr::null(),
                core::ptr::null_mut(),
            );
        }
        let list = core::ptr::addr_of_mut!(queue.task_list).cast::<c_void>();
        assert_eq!(queue.task_list.next, list);
        assert_eq!(queue.task_list.prev, list);
    }
}
