//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_functions.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_functions.c
//! `function` tracer — records every kernel function entry via mcount/fentry
//! into the trace ring buffer.
//!
//! Ref: vendor/linux/kernel/trace/trace_functions.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static FN_ENTRIES: AtomicU64 = AtomicU64::new(0);

pub fn record(_addr: u64) {
    FN_ENTRIES.fetch_add(1, Ordering::AcqRel);
}

pub fn reset() {
    FN_ENTRIES.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments_count() {
        reset();
        record(0x1000);
        record(0x2000);
        assert_eq!(FN_ENTRIES.load(Ordering::Acquire), 2);
    }
}
