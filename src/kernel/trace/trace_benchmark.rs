//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_benchmark.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_benchmark.c
//! Tracing-benchmark helpers driving a synthetic event stream.
//!
//! Ref: vendor/linux/kernel/trace/trace_benchmark.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static BENCH_EVENTS: AtomicU64 = AtomicU64::new(0);

pub fn fire(n: u64) {
    BENCH_EVENTS.fetch_add(n, Ordering::AcqRel);
}

pub fn count() -> u64 {
    BENCH_EVENTS.load(Ordering::Acquire)
}

pub fn reset() {
    BENCH_EVENTS.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_accumulates_count() {
        reset();
        fire(10);
        fire(7);
        assert_eq!(count(), 17);
    }
}
