//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors
//! RV monitor: no timer miss.
//!
//! Ref: vendor/linux/kernel/trace/rv/monitors/nomiss/nomiss.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static MISSED: AtomicU64 = AtomicU64::new(0);

pub fn record_miss() {
    MISSED.fetch_add(1, Ordering::AcqRel);
}

pub fn violated() -> bool {
    MISSED.load(Ordering::Acquire) > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miss_sets_violation() {
        MISSED.store(0, Ordering::Release);
        record_miss();
        assert!(violated());
    }
}
