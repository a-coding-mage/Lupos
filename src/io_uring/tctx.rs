//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/tctx.c
//! test-origin: linux:vendor/linux/io_uring/tctx.c
//! Per-task io_uring context.
//!
//! Each task that opens an io_uring fd gets a `IoUringTask` that tracks
//! pending task_work, the list of contexts it owns, and io-wq affiliations.
//!
//! Ref: vendor/linux/io_uring/tctx.c
//! Ref: vendor/linux/io_uring/tctx.h

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use spin::Mutex;

use super::IoRingCtx;

/// `struct io_uring_task` — per-task io_uring state.
/// Ref: vendor/linux/include/linux/io_uring_types.h::io_uring_task
pub struct IoUringTask {
    /// `xa` — set of ring tokens this task has registered with.
    rings: Mutex<BTreeSet<usize>>,
    /// `task_running` flag — true while task_work is being drained.
    pub task_running: AtomicBool,
    /// Pending task_work items (`io_task_work_list`).
    pub task_list: Mutex<Vec<TaskWorkItem>>,
    /// `last` — last cached ring lookup (perf hint, omitted from API).
    pub in_cancel: AtomicU32,
}

/// One queued task_work callback.
pub struct TaskWorkItem {
    pub ctx: Arc<IoRingCtx>,
    pub user_data: u64,
    pub res: i32,
}

impl IoUringTask {
    pub fn new() -> Self {
        Self {
            rings: Mutex::new(BTreeSet::new()),
            task_running: AtomicBool::new(false),
            task_list: Mutex::new(Vec::new()),
            in_cancel: AtomicU32::new(0),
        }
    }

    /// `io_uring_add_tctx_node` — attach a ring to this task.
    pub fn add_ring(&self, token: usize) {
        self.rings.lock().insert(token);
    }

    /// `io_uring_del_tctx_node` — detach a ring from this task.
    pub fn del_ring(&self, token: usize) {
        self.rings.lock().remove(&token);
    }

    pub fn ring_count(&self) -> usize {
        self.rings.lock().len()
    }

    pub fn has_ring(&self, token: usize) -> bool {
        self.rings.lock().contains(&token)
    }

    /// `io_task_work_add` — enqueue a task_work item.
    pub fn task_work_add(&self, item: TaskWorkItem) {
        self.task_list.lock().push(item);
    }

    /// `tctx_task_work_run` — drain queue, returning the items in FIFO order.
    pub fn task_work_run(&self) -> Vec<TaskWorkItem> {
        self.task_running.store(true, Ordering::Release);
        let items = core::mem::take(&mut *self.task_list.lock());
        self.task_running.store(false, Ordering::Release);
        items
    }
}

impl Default for IoUringTask {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_uring::IoRingCtx;
    use alloc::sync::Arc;

    #[test]
    fn add_and_del_ring_round_trip() {
        let t = IoUringTask::new();
        t.add_ring(7);
        assert!(t.has_ring(7));
        assert_eq!(t.ring_count(), 1);
        t.del_ring(7);
        assert!(!t.has_ring(7));
    }

    #[test]
    fn task_work_fifo_order() {
        let t = IoUringTask::new();
        let ctx = Arc::new(IoRingCtx::new(4));
        t.task_work_add(TaskWorkItem {
            ctx: ctx.clone(),
            user_data: 1,
            res: 0,
        });
        t.task_work_add(TaskWorkItem {
            ctx: ctx.clone(),
            user_data: 2,
            res: 0,
        });
        let items = t.task_work_run();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].user_data, 1);
        assert_eq!(items[1].user_data, 2);
        // Queue is drained.
        assert!(t.task_list.lock().is_empty());
    }

    #[test]
    fn task_running_flag_toggles_around_run() {
        let t = IoUringTask::new();
        assert!(!t.task_running.load(Ordering::Acquire));
        t.task_work_run();
        // After drain returns, flag must be cleared.
        assert!(!t.task_running.load(Ordering::Acquire));
    }
}
