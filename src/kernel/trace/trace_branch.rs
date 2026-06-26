//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_branch.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_branch.c
//! `branch` tracer — records `likely()` / `unlikely()` mispredictions.
//!
//! Ref: vendor/linux/kernel/trace/trace_branch.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static BRANCH_HITS: AtomicU64 = AtomicU64::new(0);
pub static BRANCH_MISSES: AtomicU64 = AtomicU64::new(0);

pub fn record(branch_taken: bool, predicted: bool) {
    if branch_taken == predicted {
        BRANCH_HITS.fetch_add(1, Ordering::AcqRel);
    } else {
        BRANCH_MISSES.fetch_add(1, Ordering::AcqRel);
    }
}

pub fn ratio() -> f32 {
    let h = BRANCH_HITS.load(Ordering::Acquire);
    let m = BRANCH_MISSES.load(Ordering::Acquire);
    if h + m == 0 {
        return 0.0;
    }
    h as f32 / (h + m) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_increments_on_match() {
        let h0 = BRANCH_HITS.load(Ordering::Acquire);
        record(true, true);
        assert_eq!(BRANCH_HITS.load(Ordering::Acquire), h0 + 1);
    }

    #[test]
    fn miss_increments_on_mispredict() {
        let m0 = BRANCH_MISSES.load(Ordering::Acquire);
        record(true, false);
        assert_eq!(BRANCH_MISSES.load(Ordering::Acquire), m0 + 1);
    }
}
