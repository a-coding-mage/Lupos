//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/cpuacct.c
//! test-origin: linux:vendor/linux/kernel/sched/cpuacct.c
//! CPU accounting controller helpers.
//!
//! Mirrors `vendor/linux/kernel/sched/cpuacct.c`. This is the v1-style CPU
//! accounting sidecar to the cgroup CPU controller in `kernel::cgroup::cpu`.

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CpuAcctStat {
    pub user_ns: u64,
    pub system_ns: u64,
}

#[derive(Default)]
pub struct CpuAcct {
    user_ns: AtomicU64,
    system_ns: AtomicU64,
}

impl CpuAcct {
    pub const fn new() -> Self {
        Self {
            user_ns: AtomicU64::new(0),
            system_ns: AtomicU64::new(0),
        }
    }

    pub fn charge_user(&self, delta_ns: u64) {
        self.user_ns.fetch_add(delta_ns, Ordering::Relaxed);
    }

    pub fn charge_system(&self, delta_ns: u64) {
        self.system_ns.fetch_add(delta_ns, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CpuAcctStat {
        CpuAcctStat {
            user_ns: self.user_ns.load(Ordering::Relaxed),
            system_ns: self.system_ns.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpuacct_charges_user_and_system_time() {
        let acct = CpuAcct::new();
        acct.charge_user(10);
        acct.charge_system(20);
        assert_eq!(
            acct.snapshot(),
            CpuAcctStat {
                user_ns: 10,
                system_ns: 20
            }
        );
    }
}
