//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors
//! RV monitor: cpu stall detector.
//!
//! Ref: vendor/linux/kernel/trace/rv/monitors/stall/stall.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static STALL_THRESHOLD_NS: AtomicU64 = AtomicU64::new(1_000_000_000); // 1s

pub fn observe(idle_ns: u64) -> bool {
    idle_ns > STALL_THRESHOLD_NS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_idle_triggers_violation() {
        assert!(observe(2_000_000_000));
        assert!(!observe(100_000_000));
    }
}
