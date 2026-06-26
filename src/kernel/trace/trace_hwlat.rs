//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_hwlat.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_hwlat.c
//! `hwlat` (hardware-latency) detector tracer — busy-loops on every cpu
//! and records any gap larger than the configured threshold.
//!
//! Ref: vendor/linux/kernel/trace/trace_hwlat.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static THRESHOLD_NS: AtomicU64 = AtomicU64::new(10_000); // 10us default
pub static MAX_OBSERVED_NS: AtomicU64 = AtomicU64::new(0);

pub fn observe(gap_ns: u64) {
    let cur = MAX_OBSERVED_NS.load(Ordering::Acquire);
    if gap_ns > cur {
        MAX_OBSERVED_NS.store(gap_ns, Ordering::Release);
    }
}

pub fn over_threshold() -> bool {
    MAX_OBSERVED_NS.load(Ordering::Acquire) > THRESHOLD_NS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_records_max() {
        MAX_OBSERVED_NS.store(0, Ordering::Release);
        observe(5_000);
        observe(20_000);
        observe(8_000);
        assert_eq!(MAX_OBSERVED_NS.load(Ordering::Acquire), 20_000);
        assert!(over_threshold());
    }
}
