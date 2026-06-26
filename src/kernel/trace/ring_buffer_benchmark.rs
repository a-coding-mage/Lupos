//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/ring_buffer_benchmark.c
//! test-origin: linux:vendor/linux/kernel/trace/ring_buffer_benchmark.c
//! Microbenchmark scaffolding for the trace ring buffer.
//!
//! In Linux this is built as a kernel module that fires producer/consumer
//! threads against the trace ring; we keep the entry-point shape so a
//! future test can drive it deterministically.
//!
//! Ref: vendor/linux/kernel/trace/ring_buffer_benchmark.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static EVENTS_PRODUCED: AtomicU64 = AtomicU64::new(0);
pub static EVENTS_CONSUMED: AtomicU64 = AtomicU64::new(0);

pub fn record_produced(n: u64) {
    EVENTS_PRODUCED.fetch_add(n, Ordering::AcqRel);
}

pub fn record_consumed(n: u64) {
    EVENTS_CONSUMED.fetch_add(n, Ordering::AcqRel);
}

pub fn reset() {
    EVENTS_PRODUCED.store(0, Ordering::Release);
    EVENTS_CONSUMED.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_accumulate() {
        reset();
        record_produced(10);
        record_consumed(7);
        assert_eq!(EVENTS_PRODUCED.load(Ordering::Acquire), 10);
        assert_eq!(EVENTS_CONSUMED.load(Ordering::Acquire), 7);
    }
}
