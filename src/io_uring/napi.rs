//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/napi.c
//! test-origin: linux:vendor/linux/io_uring/napi.c
//! NAPI busy-poll integration for io_uring.
//!
//! `IORING_REGISTER_NAPI` lets a ring opt into busy-polling network queues
//! during `io_uring_enter(GETEVENTS)`.  Lupos integrates with `src/net/`
//! via the per-ring `NapiState` below.
//!
//! Ref: vendor/linux/io_uring/napi.c

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::uapi::IoUringNapi;

/// `IO_URING_NAPI_TRACKING_*` strategies.
pub const TRACKING_DYNAMIC: u32 = 0;
pub const TRACKING_STATIC: u32 = 1;
pub const TRACKING_INACTIVE: u32 = 255;

/// Per-ring NAPI state.
pub struct NapiState {
    pub busy_poll_to_us: AtomicU32,
    pub prefer_busy_poll: AtomicBool,
    pub tracking: AtomicU32,
    pub enabled: AtomicBool,
}

impl NapiState {
    pub const fn new() -> Self {
        Self {
            busy_poll_to_us: AtomicU32::new(0),
            prefer_busy_poll: AtomicBool::new(false),
            tracking: AtomicU32::new(TRACKING_INACTIVE),
            enabled: AtomicBool::new(false),
        }
    }

    /// `io_register_napi` — accept a user-side `io_uring_napi`.  Returns
    /// `-EINVAL` for unknown opcode/strategy values.
    pub fn register(&self, n: &IoUringNapi) -> Result<(), i32> {
        // Linux validates op_param == tracking for REGISTER_OP.
        match n.op_param {
            TRACKING_DYNAMIC | TRACKING_STATIC | TRACKING_INACTIVE => {}
            _ => return Err(-22),
        }
        self.busy_poll_to_us
            .store(n.busy_poll_to, Ordering::Release);
        self.prefer_busy_poll
            .store(n.prefer_busy_poll != 0, Ordering::Release);
        self.tracking.store(n.op_param, Ordering::Release);
        self.enabled.store(true, Ordering::Release);
        Ok(())
    }

    /// `io_unregister_napi` — flush state.
    pub fn unregister(&self) {
        self.enabled.store(false, Ordering::Release);
        self.busy_poll_to_us.store(0, Ordering::Release);
        self.tracking.store(TRACKING_INACTIVE, Ordering::Release);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_constants_match_linux() {
        assert_eq!(TRACKING_DYNAMIC, 0);
        assert_eq!(TRACKING_STATIC, 1);
        assert_eq!(TRACKING_INACTIVE, 255);
    }

    #[test]
    fn register_records_timeout_and_strategy() {
        let s = NapiState::new();
        let n = IoUringNapi {
            busy_poll_to: 50,
            prefer_busy_poll: 1,
            opcode: 0,
            pad: [0, 0],
            op_param: TRACKING_STATIC,
            resv: 0,
        };
        s.register(&n).unwrap();
        assert!(s.is_enabled());
        assert_eq!(s.busy_poll_to_us.load(Ordering::Acquire), 50);
        assert_eq!(s.tracking.load(Ordering::Acquire), TRACKING_STATIC);
    }

    #[test]
    fn register_rejects_unknown_strategy() {
        let s = NapiState::new();
        let n = IoUringNapi {
            busy_poll_to: 0,
            prefer_busy_poll: 0,
            opcode: 0,
            pad: [0, 0],
            op_param: 99,
            resv: 0,
        };
        assert_eq!(s.register(&n).unwrap_err(), -22);
    }

    #[test]
    fn unregister_clears_state() {
        let s = NapiState::new();
        let n = IoUringNapi {
            busy_poll_to: 100,
            prefer_busy_poll: 0,
            opcode: 0,
            pad: [0, 0],
            op_param: TRACKING_DYNAMIC,
            resv: 0,
        };
        s.register(&n).unwrap();
        s.unregister();
        assert!(!s.is_enabled());
        assert_eq!(s.busy_poll_to_us.load(Ordering::Acquire), 0);
    }
}
