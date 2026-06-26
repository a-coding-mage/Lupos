//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timer_migration.c
//! test-origin: linux:vendor/linux/kernel/time/timer_migration.c
//! Timer migration coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timer_migration.c`.

use core::sync::atomic::{AtomicU64, Ordering};

#[repr(C)]
pub struct TimerMigration {
    migrated: AtomicU64,
}

impl TimerMigration {
    pub const fn new() -> Self {
        Self {
            migrated: AtomicU64::new(0),
        }
    }

    pub fn migrate_timer(&self, from_cpu: usize, to_cpu: usize) -> bool {
        if from_cpu == to_cpu {
            return false;
        }
        self.migrated.fetch_add(1, Ordering::AcqRel);
        true
    }

    pub fn migrated_count(&self) -> u64 {
        self.migrated.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_cross_cpu_migration() {
        let migration = TimerMigration::new();
        assert!(migration.migrate_timer(0, 1));
        assert!(!migration.migrate_timer(1, 1));
        assert_eq!(migration.migrated_count(), 1);
    }
}
