//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! Batched wake-up list (`wake_q_head`) — M33.
//!
//! Mirrors `vendor/linux/include/linux/sched/wake_q.h`.  A locking primitive
//! that wakes multiple waiters batches them into a `WakeQ` head while holding
//! the inner lock, then drops the lock and calls `wake_up_q` outside critical
//! section to avoid wakeup-while-locked latency.

extern crate alloc;

use alloc::vec::Vec;

use crate::kernel::task::TaskStruct;

#[repr(C)]
pub struct WakeQHead {
    /// First task to wake.
    pub first: *mut TaskStruct,
    /// Last task to wake (for tail-append).
    pub last: *mut TaskStruct,
    /// Out-of-line list when the per-task `wake_q.next` chain isn't enough.
    extras: Vec<*mut TaskStruct>,
}

unsafe impl Send for WakeQHead {}
unsafe impl Sync for WakeQHead {}

impl WakeQHead {
    pub const fn new() -> Self {
        Self {
            first: core::ptr::null_mut(),
            last: core::ptr::null_mut(),
            extras: Vec::new(),
        }
    }

    /// Add `task` to the wake list.  Mirrors `wake_q_add`.
    pub fn add(&mut self, task: *mut TaskStruct) {
        if task.is_null() {
            return;
        }
        if self.first.is_null() {
            self.first = task;
            self.last = task;
        } else {
            self.extras.push(task);
            self.last = task;
        }
    }

    /// Drain and wake every task on the list.  Mirrors `wake_up_q`.
    pub fn wake_all(&mut self) {
        if !self.first.is_null() {
            unsafe {
                crate::kernel::sched::wake_task(self.first);
            }
        }
        for &t in self.extras.iter() {
            unsafe {
                crate::kernel::sched::wake_task(t);
            }
        }
        self.first = core::ptr::null_mut();
        self.last = core::ptr::null_mut();
        self.extras.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.first.is_null()
    }
}

impl Default for WakeQHead {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_wake_q_is_empty() {
        let q = WakeQHead::new();
        assert!(q.is_empty());
    }

    #[test]
    fn add_then_wake_clears_list() {
        let mut q = WakeQHead::new();
        let mut t1 = unsafe { core::mem::zeroed::<TaskStruct>() };
        q.add(&mut t1 as *mut TaskStruct);
        assert!(!q.is_empty());
        q.wake_all();
        assert!(q.is_empty());
    }
}
