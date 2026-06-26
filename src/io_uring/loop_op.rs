//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/loop.c
//! test-origin: linux:vendor/linux/io_uring/loop.c
//! `vendor/linux/io_uring/loop.c` — io_uring task-work loop helper.
//!
//! Walks a list of `struct futex_waitv` entries during the FUTEX_WAITV op.
//! Ported as a small iteration helper here; the SQE-level handler lives in
//! `futex.rs`.
//!
//! Ref: vendor/linux/io_uring/loop.c

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EFAULT, EINTR, EINVAL};

pub const IOU_LOOP_CONTINUE: i32 = 0;
pub const IOU_LOOP_STOP: i32 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IoLoopState {
    pub cq_tail: i32,
    pub cq_wait_idx: i32,
    pub cq_wait_nr: i32,
    pub task_interruptible: bool,
    pub waited: bool,
    pub local_work_ran: bool,
    pub overflow_flushed: bool,
}

pub const fn io_loop_nr_cqes(cq_tail: i32, cq_wait_idx: i32) -> i32 {
    cq_wait_idx - cq_tail
}

pub fn io_loop_wait_start(state: &mut IoLoopState, nr_wait: i32) {
    state.cq_wait_nr = nr_wait;
    state.task_interruptible = true;
}

pub fn io_loop_wait_finish(state: &mut IoLoopState) {
    state.task_interruptible = false;
    state.cq_wait_nr = 0;
}

pub fn io_loop_wait(
    state: &mut IoLoopState,
    local_work_pending: bool,
    check_cq: bool,
    nr_wait: i32,
) {
    io_loop_wait_start(state, nr_wait);
    if local_work_pending || io_loop_nr_cqes(state.cq_tail, state.cq_wait_idx) <= 0 || check_cq {
        io_loop_wait_finish(state);
        return;
    }
    state.waited = true;
    io_loop_wait_finish(state);
}

pub fn run_loop_steps(
    state: &mut IoLoopState,
    steps: &[i32],
    task_sigpending_at: Option<usize>,
    overflow: bool,
) -> Result<(), i32> {
    for (index, step_res) in steps.iter().copied().enumerate() {
        if step_res == -EFAULT {
            return Err(-EFAULT);
        }
        if step_res == IOU_LOOP_STOP {
            break;
        }
        if step_res != IOU_LOOP_CONTINUE {
            return Err(-EINVAL);
        }
        let nr_wait = io_loop_nr_cqes(state.cq_tail, state.cq_wait_idx).max(0);
        if nr_wait > 0 {
            io_loop_wait(state, false, false, nr_wait);
        }
        state.local_work_ran = true;
        if task_sigpending_at == Some(index) {
            return Err(-EINTR);
        }
        if overflow {
            state.overflow_flushed = true;
        }
    }
    Ok(())
}

/// One element of the loop iteration buffer.
#[derive(Clone, Copy, Debug, Default)]
pub struct LoopEntry {
    pub val: u64,
    pub uaddr: u64,
    pub flags: u32,
}

/// `io_loop_walk` — iterate `entries`, calling `f` on each.  Stop on first
/// non-zero result; return that value.  Returns `0` if every entry returned 0.
pub fn loop_walk<F>(entries: &[LoopEntry], mut f: F) -> i32
where
    F: FnMut(&LoopEntry) -> i32,
{
    for e in entries {
        let r = f(e);
        if r != 0 {
            return r;
        }
    }
    0
}

/// Convenience: collect entries where `pred` matches.
pub fn loop_filter<P>(entries: &[LoopEntry], mut pred: P) -> Vec<LoopEntry>
where
    P: FnMut(&LoopEntry) -> bool,
{
    let mut out = Vec::new();
    for e in entries {
        if pred(e) {
            out.push(*e);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> [LoopEntry; 3] {
        [
            LoopEntry {
                val: 1,
                uaddr: 0x1,
                flags: 0,
            },
            LoopEntry {
                val: 2,
                uaddr: 0x2,
                flags: 0,
            },
            LoopEntry {
                val: 3,
                uaddr: 0x3,
                flags: 0,
            },
        ]
    }

    #[test]
    fn run_loop_helpers_match_linux_loop_c_wait_and_error_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/io_uring/loop.c"
        ));
        assert!(source.contains("static inline int io_loop_nr_cqes"));
        assert!(source.contains("return lp->cq_wait_idx - READ_ONCE(ctx->rings->cq.tail);"));
        assert!(source.contains("atomic_set(&ctx->cq_wait_nr, nr_wait);"));
        assert!(source.contains("set_current_state(TASK_INTERRUPTIBLE);"));
        assert!(source.contains("__set_current_state(TASK_RUNNING);"));
        assert!(source.contains("atomic_set(&ctx->cq_wait_nr, IO_CQ_WAKE_INIT);"));
        assert!(source.contains("if (unlikely(!ctx->loop_step))"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("if (step_res == IOU_LOOP_STOP)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("if (unlikely(task_sigpending(current)))"));
        assert!(source.contains("return -EINTR;"));
        assert!(source.contains("io_cqring_overflow_flush_locked(ctx);"));
        assert!(source.contains("if (!io_allowed_run_tw(ctx))"));
        assert!(source.contains("return -EEXIST;"));

        let mut state = IoLoopState {
            cq_tail: 2,
            cq_wait_idx: 5,
            ..IoLoopState::default()
        };
        assert_eq!(io_loop_nr_cqes(2, 5), 3);
        io_loop_wait_start(&mut state, 3);
        assert!(state.task_interruptible);
        assert_eq!(state.cq_wait_nr, 3);
        io_loop_wait_finish(&mut state);
        assert!(!state.task_interruptible);
        assert_eq!(state.cq_wait_nr, 0);
        assert_eq!(
            run_loop_steps(&mut state, &[IOU_LOOP_CONTINUE, IOU_LOOP_STOP], None, true),
            Ok(())
        );
        assert!(state.local_work_ran);
        assert!(state.overflow_flushed);
        assert_eq!(
            run_loop_steps(&mut state, &[IOU_LOOP_CONTINUE], Some(0), false),
            Err(-EINTR)
        );
        assert_eq!(run_loop_steps(&mut state, &[99], None, false), Err(-EINVAL));
    }

    #[test]
    fn loop_walk_returns_zero_on_all_success() {
        let r = loop_walk(&entries(), |_| 0);
        assert_eq!(r, 0);
    }

    #[test]
    fn loop_walk_short_circuits_on_error() {
        let mut seen = 0;
        let r = loop_walk(&entries(), |e| {
            seen += 1;
            if e.val == 2 { -22 } else { 0 }
        });
        assert_eq!(r, -22);
        // We stopped at entry 2.
        assert_eq!(seen, 2);
    }

    #[test]
    fn loop_filter_picks_matching() {
        let r = loop_filter(&entries(), |e| e.val >= 2);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].val, 2);
    }
}
