//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_sched_wakeup.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_sched_wakeup.c
//! `sched_wakeup` latency tracer.
//!
//! Ref: vendor/linux/kernel/trace/trace_sched_wakeup.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static MAX_WAKEUP_LATENCY_NS: AtomicU64 = AtomicU64::new(0);

pub fn record_latency(ns: u64) {
    let cur = MAX_WAKEUP_LATENCY_NS.load(Ordering::Acquire);
    if ns > cur {
        MAX_WAKEUP_LATENCY_NS.store(ns, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_tracks_maximum() {
        MAX_WAKEUP_LATENCY_NS.store(0, Ordering::Release);
        record_latency(100);
        record_latency(500);
        record_latency(200);
        assert_eq!(MAX_WAKEUP_LATENCY_NS.load(Ordering::Acquire), 500);
    }
}
