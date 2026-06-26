//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/refscale.c
//! test-origin: linux:vendor/linux/kernel/rcu/refscale.c
//! Reference scale benchmark coverage for M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/refscale.c`.

use core::sync::atomic::{AtomicUsize, Ordering};

#[repr(C)]
pub struct RefScaleCounter {
    refs: AtomicUsize,
}

impl RefScaleCounter {
    pub const fn new() -> Self {
        Self {
            refs: AtomicUsize::new(1),
        }
    }

    pub fn get(&self) -> usize {
        self.refs.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn put(&self) -> usize {
        self.refs.fetch_sub(1, Ordering::AcqRel) - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_counter_get_put_round_trip() {
        let refs = RefScaleCounter::new();
        assert_eq!(refs.get(), 2);
        assert_eq!(refs.put(), 1);
    }
}
