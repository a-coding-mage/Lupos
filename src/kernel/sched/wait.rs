//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/wait.c
//! test-origin: linux:vendor/linux/kernel/sched/wait.c
//! Scheduler wait queues.
//!
//! Mirrors `vendor/linux/kernel/sched/wait.c`. This is the generic wait queue
//! primitive used by scheduler-owned blocking paths; process wait4/waitid lives
//! separately in `kernel::wait`.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Mutex;

use crate::kernel::task::{TaskStruct, task_state};

pub struct WaitQueueHead {
    waiters: Mutex<Vec<*mut TaskStruct>>,
}

unsafe impl Send for WaitQueueHead {}
unsafe impl Sync for WaitQueueHead {}

impl WaitQueueHead {
    pub const fn new() -> Self {
        Self {
            waiters: Mutex::new(Vec::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.waiters.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub unsafe fn prepare_to_wait(&self, task: *mut TaskStruct, state: u32) {
        if task.is_null() {
            return;
        }
        let mut waiters = self.waiters.lock();
        if !waiters.iter().any(|&queued| queued == task) {
            waiters.push(task);
        }
        unsafe {
            (*task).__state.store(state, Ordering::Release);
        }
    }

    pub unsafe fn finish_wait(&self, task: *mut TaskStruct) {
        if task.is_null() {
            return;
        }
        let mut waiters = self.waiters.lock();
        if let Some(pos) = waiters.iter().position(|&queued| queued == task) {
            waiters.remove(pos);
        }
        unsafe {
            (*task)
                .__state
                .store(task_state::TASK_RUNNING, Ordering::Release);
        }
    }

    pub fn wake_up_one(&self) -> Option<*mut TaskStruct> {
        let task = self.waiters.lock().pop();
        if let Some(task) = task {
            unsafe {
                (*task)
                    .__state
                    .store(task_state::TASK_RUNNING, Ordering::Release);
            }
        }
        task
    }

    pub fn wake_up_all(&self) -> usize {
        let mut waiters = self.waiters.lock();
        let count = waiters.len();
        for &task in waiters.iter() {
            unsafe {
                (*task)
                    .__state
                    .store(task_state::TASK_RUNNING, Ordering::Release);
            }
        }
        waiters.clear();
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
}
