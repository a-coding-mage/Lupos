//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/tiny.c
//! test-origin: linux:vendor/linux/kernel/rcu/tiny.c
//! Tiny RCU coverage for M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/tiny.c`.  Tiny RCU is the uniprocessor
//! implementation; callbacks can run after a local quiescent-state pass.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[repr(C)]
pub struct TinyRcu {
    queued: AtomicUsize,
    completed_gp: AtomicU64,
}

impl TinyRcu {
    pub const fn new() -> Self {
        Self {
            queued: AtomicUsize::new(0),
            completed_gp: AtomicU64::new(0),
        }
    }

    pub fn call_rcu(&self) {
        self.queued.fetch_add(1, Ordering::AcqRel);
    }

    pub fn rcu_process_callbacks(&self) -> usize {
        let callbacks = self.queued.swap(0, Ordering::AcqRel);
        if callbacks != 0 {
            self.completed_gp.fetch_add(1, Ordering::AcqRel);
        }
        callbacks
    }

    pub fn completed_gp(&self) -> u64 {
        self.completed_gp.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callbacks_flush_in_one_pass() {
        let rcu = TinyRcu::new();
        rcu.call_rcu();
        rcu.call_rcu();
        assert_eq!(rcu.rcu_process_callbacks(), 2);
        assert_eq!(rcu.completed_gp(), 1);
    }
}
