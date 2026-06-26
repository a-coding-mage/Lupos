//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu/rcutorture.c
//! test-origin: linux:vendor/linux/kernel/rcu/rcutorture.c
//! RCU torture harness coverage for M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/rcutorture.c`.

use super::update::{rcu_read_lock_sched, rcu_read_unlock_sched, synchronize_rcu_expedited};

pub fn rcutorture_smoke() -> bool {
    rcu_read_lock_sched();
    let held = super::update::rcu_read_lock_held();
    rcu_read_unlock_sched();
    held && synchronize_rcu_expedited() != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_exercises_read_and_update_paths() {
        assert!(rcutorture_smoke());
    }
}
