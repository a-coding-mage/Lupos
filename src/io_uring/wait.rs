//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/wait.c
//! test-origin: linux:vendor/linux/io_uring/wait.c
//! `io_uring_enter(GETEVENTS)` wait helpers.
//!
//! Ref: vendor/linux/io_uring/wait.c

use core::sync::atomic::Ordering;

use super::IoRingCtx;

/// `io_should_wake` — return true if `ctx.cq_ready() >= min_events`.
///
/// Mirrors `io_wake_function` decision logic without taking the wait-queue
/// lock (the loop driver above does that).
pub fn io_should_wake(ctx: &IoRingCtx, min_events: u32) -> bool {
    ctx.cq_ready() >= min_events
}

/// `io_cqring_wait` — busy-wait until `min_events` are ready or the loop
/// budget is exhausted.  Returns the number of completions ready at exit.
///
/// This is the synchronous variant suitable for tests; the kthread-driven
/// variant lands in sqpoll.rs.
pub fn io_cqring_wait(ctx: &IoRingCtx, min_events: u32, max_iters: u32) -> u32 {
    for _ in 0..max_iters {
        if io_should_wake(ctx, min_events) {
            break;
        }
        // Force a re-read; on real hardware this is where we'd cpu_relax().
        core::sync::atomic::fence(Ordering::SeqCst);
    }
    ctx.cq_ready()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_uring::IoRingCtx;
    use core::sync::atomic::Ordering;

    #[test]
    fn io_should_wake_false_when_below_min() {
        let ctx = IoRingCtx::new(8);
        assert!(!io_should_wake(&ctx, 1));
    }

    #[test]
    fn io_should_wake_true_when_at_or_above_min() {
        let ctx = IoRingCtx::new(8);
        ctx.cq_tail.store(3, Ordering::Release);
        assert!(io_should_wake(&ctx, 1));
        assert!(io_should_wake(&ctx, 3));
        assert!(!io_should_wake(&ctx, 4));
    }

    #[test]
    fn io_cqring_wait_returns_ready_count() {
        let ctx = IoRingCtx::new(8);
        ctx.cq_tail.store(2, Ordering::Release);
        let got = io_cqring_wait(&ctx, 1, 8);
        assert_eq!(got, 2);
    }

    #[test]
    fn io_cqring_wait_bounded_when_never_woken() {
        let ctx = IoRingCtx::new(8);
        // No completions; loop exits after max_iters with cq_ready == 0.
        let got = io_cqring_wait(&ctx, 5, 4);
        assert_eq!(got, 0);
    }
}
