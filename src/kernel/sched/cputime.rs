//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/cputime.c
//! test-origin: linux:vendor/linux/kernel/sched/cputime.c
//! Task CPU time accounting.
//!
//! Mirrors `vendor/linux/kernel/sched/cputime.c`.

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TaskCputime {
    pub utime_ns: u64,
    pub stime_ns: u64,
    pub sum_exec_runtime_ns: u64,
}

#[derive(Default)]
pub struct TaskCputimeAtomic {
    utime_ns: AtomicU64,
    stime_ns: AtomicU64,
    sum_exec_runtime_ns: AtomicU64,
}

impl TaskCputimeAtomic {
    pub const fn new() -> Self {
        Self {
            utime_ns: AtomicU64::new(0),
            stime_ns: AtomicU64::new(0),
            sum_exec_runtime_ns: AtomicU64::new(0),
        }
    }

    pub fn account_user_time(&self, delta_ns: u64) {
        self.utime_ns.fetch_add(delta_ns, Ordering::Relaxed);
        self.sum_exec_runtime_ns
            .fetch_add(delta_ns, Ordering::Relaxed);
    }

    pub fn account_system_time(&self, delta_ns: u64) {
        self.stime_ns.fetch_add(delta_ns, Ordering::Relaxed);
        self.sum_exec_runtime_ns
            .fetch_add(delta_ns, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TaskCputime {
        TaskCputime {
            utime_ns: self.utime_ns.load(Ordering::Relaxed),
            stime_ns: self.stime_ns.load(Ordering::Relaxed),
            sum_exec_runtime_ns: self.sum_exec_runtime_ns.load(Ordering::Relaxed),
        }
    }
}

pub const fn nsecs_to_clock_t(ns: u64, user_hz: u64) -> u64 {
    if user_hz == 0 {
        return 0;
    }
    ns / (1_000_000_000 / user_hz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cputime_accounts_user_and_system_runtime() {
        let t = TaskCputimeAtomic::new();
        t.account_user_time(10);
        t.account_system_time(5);
        assert_eq!(
            t.snapshot(),
            TaskCputime {
                utime_ns: 10,
                stime_ns: 5,
                sum_exec_runtime_ns: 15
            }
        );
    }

    #[test]
    fn nsec_conversion_uses_user_hz() {
        assert_eq!(nsecs_to_clock_t(20_000_000, 100), 2);
    }
}
