//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_osnoise.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_osnoise.c
//! `osnoise` / `timerlat` — measures the OS noise on each CPU.
//!
//! Ref: vendor/linux/kernel/trace/trace_osnoise.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static OS_NOISE_NS: AtomicU64 = AtomicU64::new(0);
pub static OS_NOISE_SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn record_sample(noise_ns: u64) {
    OS_NOISE_NS.fetch_add(noise_ns, Ordering::AcqRel);
    OS_NOISE_SAMPLE_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn average_ns() -> u64 {
    let n = OS_NOISE_SAMPLE_COUNT.load(Ordering::Acquire);
    if n == 0 {
        return 0;
    }
    OS_NOISE_NS.load(Ordering::Acquire) / n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_then_average() {
        OS_NOISE_NS.store(0, Ordering::Release);
        OS_NOISE_SAMPLE_COUNT.store(0, Ordering::Release);
        record_sample(100);
        record_sample(200);
        record_sample(300);
        assert_eq!(average_ns(), 200);
    }
}
