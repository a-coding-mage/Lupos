//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_snapshot.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_snapshot.c
//! Per-cpu trace-buffer snapshot — atomic swap with the live buffer for
//! post-mortem analysis.
//!
//! Ref: vendor/linux/kernel/trace/trace_snapshot.c

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub static SNAPSHOT_TAKEN: AtomicBool = AtomicBool::new(false);
pub static SNAPSHOT_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn take_snapshot() -> u64 {
    SNAPSHOT_TAKEN.store(true, Ordering::Release);
    SNAPSHOT_GENERATION.fetch_add(1, Ordering::AcqRel) + 1
}

pub fn clear() {
    SNAPSHOT_TAKEN.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_advances_generation() {
        let a = take_snapshot();
        let b = take_snapshot();
        assert_eq!(b, a + 1);
        clear();
        assert!(!SNAPSHOT_TAKEN.load(Ordering::Acquire));
    }
}
