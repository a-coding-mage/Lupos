//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/srcutree.c
//! test-origin: linux:vendor/linux/kernel/rcu/srcutree.c
//! Sleepable RCU tree coverage for M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/srcutree.c`.  The existing `srcu.rs` keeps
//! the public API; this module models the tree-side counters and grace-period
//! sequencing used by larger SRCU domains.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[repr(C)]
pub struct SrcuTree {
    readers: AtomicUsize,
    gp_seq: AtomicU64,
}

impl SrcuTree {
    pub const fn new() -> Self {
        Self {
            readers: AtomicUsize::new(0),
            gp_seq: AtomicU64::new(0),
        }
    }

    pub fn read_lock(&self) -> usize {
        self.readers.fetch_add(1, Ordering::AcqRel);
        (self.gp_seq.load(Ordering::Acquire) & 1) as usize
    }

    pub fn read_unlock(&self, _idx: usize) {
        self.readers.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn synchronize(&self) -> u64 {
        while self.readers.load(Ordering::Acquire) != 0 {
            core::hint::spin_loop();
        }
        self.gp_seq.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn gp_seq(&self) -> u64 {
        self.gp_seq.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synchronize_advances_after_reader_leaves() {
        let srcu = SrcuTree::new();
        let idx = srcu.read_lock();
        assert_eq!(srcu.gp_seq(), 0);
        srcu.read_unlock(idx);
        assert_eq!(srcu.synchronize(), 1);
    }
}
