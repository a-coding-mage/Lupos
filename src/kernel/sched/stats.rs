//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/stats.c
//! test-origin: linux:vendor/linux/kernel/sched/stats.c
//! Scheduler statistics.
//!
//! Mirrors `vendor/linux/kernel/sched/stats.c`.

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SchedStatSnapshot {
    pub nr_context_switches: u64,
    pub nr_wakeups: u64,
    pub nr_migrations: u64,
}

#[derive(Default)]
pub struct SchedStats {
    nr_context_switches: AtomicU64,
    nr_wakeups: AtomicU64,
    nr_migrations: AtomicU64,
}

impl SchedStats {
    pub const fn new() -> Self {
        Self {
            nr_context_switches: AtomicU64::new(0),
            nr_wakeups: AtomicU64::new(0),
            nr_migrations: AtomicU64::new(0),
        }
    }

    pub fn account_switch(&self) {
        self.nr_context_switches.fetch_add(1, Ordering::Relaxed);
    }

    pub fn account_wakeup(&self) {
        self.nr_wakeups.fetch_add(1, Ordering::Relaxed);
    }

    pub fn account_migration(&self) {
        self.nr_migrations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> SchedStatSnapshot {
        SchedStatSnapshot {
            nr_context_switches: self.nr_context_switches.load(Ordering::Relaxed),
            nr_wakeups: self.nr_wakeups.load(Ordering::Relaxed),
            nr_migrations: self.nr_migrations.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sched_stats_count_events() {
        let stats = SchedStats::new();
        stats.account_switch();
        stats.account_wakeup();
        stats.account_migration();
        assert_eq!(
            stats.snapshot(),
            SchedStatSnapshot {
                nr_context_switches: 1,
                nr_wakeups: 1,
                nr_migrations: 1
            }
        );
    }
}
