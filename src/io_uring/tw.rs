//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/tw.c
//! test-origin: linux:vendor/linux/io_uring/tw.c
//! Task-work runner for io_uring.
//!
//! Linux uses `task_work_add()` to deliver callbacks to the task that owns
//! a ring; on return-to-user the callbacks fire in FIFO order.  Lupos backs
//! this with a per-task queue on the `IoUringTask`.
//!
//! Ref: vendor/linux/io_uring/tw.c

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use super::IoRingCtx;
use super::tctx::{IoUringTask, TaskWorkItem};

/// `IOU_F_TWA_*` task_work add flags.  Subset matching the upstream values.
/// Ref: vendor/linux/io_uring/tw.h
pub mod twa {
    /// Normal task_work add — wakes the target task if blocked.
    pub const SIGNAL: u32 = 0;
    /// `TWA_SIGNAL_NO_IPI` — skip IPI on remote task.
    pub const SIGNAL_NO_IPI: u32 = 1;
}

/// Counter exposed for tests / fdinfo — total task_work items processed.
pub static TASK_WORK_RUN_COUNT: AtomicU32 = AtomicU32::new(0);

/// `io_task_work_add` — enqueue a completion callback on `tctx`.
pub fn task_work_add(tctx: &IoUringTask, ctx: Arc<IoRingCtx>, user_data: u64, res: i32) {
    tctx.task_work_add(TaskWorkItem {
        ctx,
        user_data,
        res,
    });
}

/// `tctx_task_work_run` — drain `tctx` and post a CQE for each pending item.
///
/// Returns the number of items run.  Mirrors the FIFO ordering and the
/// "drain to completion" semantics of Linux's tw runner.
pub fn task_work_run(tctx: &IoUringTask) -> u32 {
    let items: Vec<TaskWorkItem> = tctx.task_work_run();
    let n = items.len() as u32;
    for item in items {
        item.ctx.complete(item.user_data, item.res);
    }
    TASK_WORK_RUN_COUNT.fetch_add(n, Ordering::AcqRel);
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_uring::IoRingCtx;
    use alloc::sync::Arc;
    use core::sync::atomic::Ordering;

    #[test]
    fn task_work_add_then_run_posts_cqes() {
        TASK_WORK_RUN_COUNT.store(0, Ordering::Release);
        let tctx = IoUringTask::new();
        let ctx = Arc::new(IoRingCtx::new(8));
        task_work_add(&tctx, ctx.clone(), 0xdead, 7);
        task_work_add(&tctx, ctx.clone(), 0xbeef, -22);
        let n = task_work_run(&tctx);
        assert_eq!(n, 2);
        // CQEs reflect the items in FIFO order.
        assert_eq!(ctx.cqes[0].user_data, 0xdead);
        assert_eq!(ctx.cqes[0].res, 7);
        assert_eq!(ctx.cqes[1].user_data, 0xbeef);
        assert_eq!(ctx.cqes[1].res, -22);
    }

    #[test]
    fn empty_run_returns_zero() {
        let tctx = IoUringTask::new();
        assert_eq!(task_work_run(&tctx), 0);
    }

    #[test]
    fn twa_constants_match_linux() {
        // Mirrors `TWA_SIGNAL` / `TWA_SIGNAL_NO_IPI` token values.
        assert_eq!(twa::SIGNAL, 0);
        assert_eq!(twa::SIGNAL_NO_IPI, 1);
    }
}
