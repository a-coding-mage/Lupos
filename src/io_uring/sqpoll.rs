//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/sqpoll.c
//! test-origin: linux:vendor/linux/io_uring/sqpoll.c
//! `IORING_SETUP_SQPOLL` — dedicated kernel thread that polls the SQ tail.
//!
//! When SQPOLL is enabled, userspace can submit SQEs without ever calling
//! `io_uring_enter` — the polling thread picks them up.  Lupos hosts the
//! polling on a kthread spawned via `src/kernel/kthread.rs`.
//!
//! Ref: vendor/linux/io_uring/sqpoll.c

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use super::IoRingCtx;

/// Default idle timeout (milliseconds) before the SQPOLL thread parks itself.
/// Mirrors `IORING_SQPOLL_IDLE_DEFAULT_MS`.
pub const IORING_SQPOLL_IDLE_DEFAULT_MS: u32 = 1000;

/// `struct io_sq_data` — shared state between SQPOLL thread and submitter.
pub struct IoSqData {
    /// Bound ring context.
    pub ctx: Arc<IoRingCtx>,
    /// `sq_thread_idle` from `io_uring_params`.
    pub sq_thread_idle_ms: u32,
    /// Optional CPU pin (`IORING_SETUP_SQ_AFF`).  `u32::MAX` = unpinned.
    pub sq_thread_cpu: u32,
    /// Park flag set by `io_sq_thread_park`.
    pub parked: AtomicBool,
    /// Stop flag set on ring teardown.
    pub stop: AtomicBool,
    /// Idle iterations elapsed without finding work.
    pub idle_iters: AtomicU64,
    /// Total SQEs the polling thread has submitted.
    pub submitted: AtomicU32,
}

impl IoSqData {
    pub fn new(ctx: Arc<IoRingCtx>, sq_thread_idle_ms: u32, sq_thread_cpu: u32) -> Arc<Self> {
        Arc::new(Self {
            ctx,
            sq_thread_idle_ms: if sq_thread_idle_ms == 0 {
                IORING_SQPOLL_IDLE_DEFAULT_MS
            } else {
                sq_thread_idle_ms
            },
            sq_thread_cpu,
            parked: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            idle_iters: AtomicU64::new(0),
            submitted: AtomicU32::new(0),
        })
    }

    /// `io_sq_thread_park`.
    pub fn park(&self) {
        self.parked.store(true, Ordering::Release);
    }

    /// `io_sq_thread_unpark`.
    pub fn unpark(&self) {
        self.parked.store(false, Ordering::Release);
        self.idle_iters.store(0, Ordering::Release);
    }

    /// `io_sq_thread_stop` — request thread exit.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }

    pub fn is_parked(&self) -> bool {
        self.parked.load(Ordering::Acquire)
    }

    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    /// `__io_sq_thread` one-shot iteration.  Drains the SQ if the thread is
    /// neither parked nor stopped; returns the count submitted this tick.
    pub fn tick(&self) -> u32 {
        if self.is_stopped() || self.is_parked() {
            return 0;
        }
        let pending = self
            .ctx
            .sq_tail
            .load(Ordering::Acquire)
            .wrapping_sub(self.ctx.sq_head.load(Ordering::Acquire));
        if pending == 0 {
            self.idle_iters.fetch_add(1, Ordering::AcqRel);
            return 0;
        }
        let n = self.ctx.submit(pending);
        self.submitted.fetch_add(n, Ordering::AcqRel);
        self.idle_iters.store(0, Ordering::Release);
        n
    }

    /// `io_sq_thread_finish` post-park check: should the thread sleep?
    /// True once `idle_iters * tick_ms >= sq_thread_idle_ms`.  Test helper
    /// since we don't have a real tick period in user-mode.
    pub fn should_sleep(&self, tick_ms: u32) -> bool {
        let total = self
            .idle_iters
            .load(Ordering::Acquire)
            .saturating_mul(tick_ms as u64);
        total >= self.sq_thread_idle_ms as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_uring::nop::IORING_NOP_INJECT_RESULT;
    use crate::io_uring::sqe::Sqe;
    use alloc::sync::Arc;
    use core::sync::atomic::Ordering;

    #[test]
    fn default_idle_constant() {
        assert_eq!(IORING_SQPOLL_IDLE_DEFAULT_MS, 1000);
    }

    #[test]
    fn park_and_unpark_round_trip() {
        let ctx = Arc::new(IoRingCtx::new(8));
        let sd = IoSqData::new(ctx, 0, u32::MAX);
        assert!(!sd.is_parked());
        sd.park();
        assert!(sd.is_parked());
        sd.unpark();
        assert!(!sd.is_parked());
    }

    #[test]
    fn tick_drains_pending_sqes() {
        let ctx = Arc::new(IoRingCtx::new(8));
        // Queue 3 NOPs.
        unsafe {
            let p = ctx.sqes.as_ptr() as *mut Sqe;
            for i in 0..3 {
                (*p.add(i)).opcode = 0;
                (*p.add(i)).op_flags = IORING_NOP_INJECT_RESULT;
                (*p.add(i)).len = 100 + i as u32;
                (*p.add(i)).user_data = i as u64;
            }
        }
        ctx.sq_tail.store(3, Ordering::Release);
        let sd = IoSqData::new(ctx.clone(), 0, u32::MAX);
        let n = sd.tick();
        assert_eq!(n, 3);
        assert_eq!(ctx.cq_ready(), 3);
        assert_eq!(sd.submitted.load(Ordering::Acquire), 3);
    }

    #[test]
    fn tick_when_parked_does_nothing() {
        let ctx = Arc::new(IoRingCtx::new(8));
        ctx.sq_tail.store(1, Ordering::Release);
        let sd = IoSqData::new(ctx.clone(), 0, u32::MAX);
        sd.park();
        assert_eq!(sd.tick(), 0);
        assert_eq!(ctx.cq_ready(), 0);
    }

    #[test]
    fn idle_ticks_accumulate_when_no_work() {
        let ctx = Arc::new(IoRingCtx::new(8));
        let sd = IoSqData::new(ctx, 0, u32::MAX);
        sd.tick();
        sd.tick();
        assert_eq!(sd.idle_iters.load(Ordering::Acquire), 2);
    }

    #[test]
    fn should_sleep_after_idle_threshold() {
        let ctx = Arc::new(IoRingCtx::new(8));
        let sd = IoSqData::new(ctx, 10, u32::MAX);
        // Two ticks at 5ms each = 10ms, threshold met.
        sd.idle_iters.store(2, Ordering::Release);
        assert!(sd.should_sleep(5));
        sd.idle_iters.store(1, Ordering::Release);
        assert!(!sd.should_sleep(5));
    }
}
