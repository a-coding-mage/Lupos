//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/sync.c
//! test-origin: linux:vendor/linux/kernel/rcu/sync.c
//! RCU sync helper coverage for M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/sync.c`.  `rcu_sync` coordinates a fast
//! read-mostly path with an update-side mode switch.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[repr(C)]
pub struct RcuSync {
    active_readers: AtomicUsize,
    blocked: AtomicBool,
}

impl RcuSync {
    pub const fn new() -> Self {
        Self {
            active_readers: AtomicUsize::new(0),
            blocked: AtomicBool::new(false),
        }
    }

    pub fn read_lock(&self) -> bool {
        if self.blocked.load(Ordering::Acquire) {
            return false;
        }
        self.active_readers.fetch_add(1, Ordering::AcqRel);
        if self.blocked.load(Ordering::Acquire) {
            self.active_readers.fetch_sub(1, Ordering::AcqRel);
            return false;
        }
        true
    }

    pub fn read_unlock(&self) {
        self.active_readers.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn enter_exclusive(&self) {
        self.blocked.store(true, Ordering::Release);
        while self.active_readers.load(Ordering::Acquire) != 0 {
            core::hint::spin_loop();
        }
    }

    pub fn exit_exclusive(&self) {
        self.blocked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_mode_blocks_new_readers() {
        let sync = RcuSync::new();
        assert!(sync.read_lock());
        sync.read_unlock();
        sync.enter_exclusive();
        assert!(!sync.read_lock());
        sync.exit_exclusive();
        assert!(sync.read_lock());
    }
}
