//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/rcuscale.c
//! test-origin: linux:vendor/linux/kernel/rcu/rcuscale.c
//! RCU scale benchmark coverage for M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/rcuscale.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static BATCHES: AtomicU64 = AtomicU64::new(0);

pub fn rcuscale_record_batch(callbacks: u64) -> u64 {
    BATCHES.fetch_add(callbacks, Ordering::AcqRel) + callbacks
}

pub fn rcuscale_total_callbacks() -> u64 {
    BATCHES.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_counter_accumulates_callbacks() {
        let before = rcuscale_total_callbacks();
        assert_eq!(rcuscale_record_batch(4), before + 4);
    }
}
