//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_recursion_record.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_recursion_record.c
//! Records recursion attempts inside the tracer itself so they can be
//! diagnosed without crashing the kernel.
//!
//! Ref: vendor/linux/kernel/trace/trace_recursion_record.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static RECURSION_HITS: AtomicU64 = AtomicU64::new(0);

pub fn record() -> u64 {
    RECURSION_HITS.fetch_add(1, Ordering::AcqRel) + 1
}

pub fn count() -> u64 {
    RECURSION_HITS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_returns_new_count() {
        RECURSION_HITS.store(0, Ordering::Release);
        assert_eq!(record(), 1);
        assert_eq!(record(), 2);
    }
}
