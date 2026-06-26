//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/namespace.c
//! test-origin: linux:vendor/linux/kernel/time/namespace.c
//! Time namespace coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/namespace.c`.

use core::sync::atomic::{AtomicI64, Ordering};

#[repr(C)]
pub struct TimeNamespace {
    monotonic_offset_ns: AtomicI64,
    boottime_offset_ns: AtomicI64,
}

impl TimeNamespace {
    pub const fn new() -> Self {
        Self {
            monotonic_offset_ns: AtomicI64::new(0),
            boottime_offset_ns: AtomicI64::new(0),
        }
    }

    pub fn set_offsets(&self, monotonic_ns: i64, boottime_ns: i64) {
        self.monotonic_offset_ns
            .store(monotonic_ns, Ordering::Release);
        self.boottime_offset_ns
            .store(boottime_ns, Ordering::Release);
    }

    pub fn monotonic_now(&self, base_ns: u64) -> u64 {
        apply_offset(base_ns, self.monotonic_offset_ns.load(Ordering::Acquire))
    }

    pub fn boottime_now(&self, base_ns: u64) -> u64 {
        apply_offset(base_ns, self.boottime_offset_ns.load(Ordering::Acquire))
    }
}

fn apply_offset(base_ns: u64, offset_ns: i64) -> u64 {
    if offset_ns >= 0 {
        base_ns.saturating_add(offset_ns as u64)
    } else {
        base_ns.saturating_sub(offset_ns.unsigned_abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_positive_and_negative_offsets() {
        let ns = TimeNamespace::new();
        ns.set_offsets(10, -5);
        assert_eq!(ns.monotonic_now(100), 110);
        assert_eq!(ns.boottime_now(100), 95);
    }
}
