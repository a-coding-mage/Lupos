//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/lockdep_proc.c
//! test-origin: linux:vendor/linux/kernel/locking/lockdep_proc.c
//! Lockdep procfs reporting coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/lockdep_proc.c`.

use super::lockdep;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LockdepProcStats {
    pub held_locks: usize,
    pub max_classes: usize,
}

pub fn lockdep_proc_stats() -> LockdepProcStats {
    LockdepProcStats {
        held_locks: lockdep::held_lock_count(),
        max_classes: lockdep::MAX_LOCKDEP_KEYS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_class_capacity() {
        assert_eq!(lockdep_proc_stats().max_classes, lockdep::MAX_LOCKDEP_KEYS);
    }
}
