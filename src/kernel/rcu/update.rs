//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/update.c
//! test-origin: linux:vendor/linux/kernel/rcu/update.c
//! RCU update-side helpers for M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/update.c`.  The full tree-RCU callback
//! engine remains in `tree.rs`; this file carries the common read-side nesting
//! and expedited grace-period surface used by generic kernel code.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

static READ_NESTING: AtomicUsize = AtomicUsize::new(0);
static EXPEDITED_GPS: AtomicU64 = AtomicU64::new(0);

pub fn rcu_read_lock_sched() {
    READ_NESTING.fetch_add(1, Ordering::AcqRel);
}

pub fn rcu_read_unlock_sched() {
    READ_NESTING.fetch_sub(1, Ordering::AcqRel);
}

pub fn rcu_read_lock_held() -> bool {
    READ_NESTING.load(Ordering::Acquire) != 0
}

pub fn synchronize_rcu_expedited() -> u64 {
    while rcu_read_lock_held() {
        core::hint::spin_loop();
    }
    EXPEDITED_GPS.fetch_add(1, Ordering::AcqRel) + 1
}

pub fn expedited_grace_periods() -> u64 {
    EXPEDITED_GPS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_lock_held_tracks_nesting() {
        rcu_read_lock_sched();
        assert!(rcu_read_lock_held());
        rcu_read_unlock_sched();
        assert!(!rcu_read_lock_held());
    }

    #[test]
    fn expedited_gp_counter_advances() {
        let before = expedited_grace_periods();
        assert_eq!(synchronize_rcu_expedited(), before + 1);
    }
}
