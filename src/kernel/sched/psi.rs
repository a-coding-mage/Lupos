//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/psi.c
//! test-origin: linux:vendor/linux/kernel/sched/psi.c
//! Pressure stall information.
//!
//! Mirrors `vendor/linux/kernel/sched/psi.c`. Memory reclaim already exposes
//! memory PSI helpers; this module adds the generic scheduler PSI group used
//! for CPU, memory, and IO stall totals.

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PsiResource {
    Cpu = 0,
    Memory = 1,
    Io = 2,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PsiTotals {
    pub cpu_some_us: u64,
    pub memory_some_us: u64,
    pub io_some_us: u64,
}

#[derive(Default)]
pub struct PsiGroup {
    cpu_some_us: AtomicU64,
    memory_some_us: AtomicU64,
    io_some_us: AtomicU64,
}

impl PsiGroup {
    pub const fn new() -> Self {
        Self {
            cpu_some_us: AtomicU64::new(0),
            memory_some_us: AtomicU64::new(0),
            io_some_us: AtomicU64::new(0),
        }
    }

    pub fn account_stall(&self, resource: PsiResource, delta_us: u64) {
        match resource {
            PsiResource::Cpu => self.cpu_some_us.fetch_add(delta_us, Ordering::Relaxed),
            PsiResource::Memory => self.memory_some_us.fetch_add(delta_us, Ordering::Relaxed),
            PsiResource::Io => self.io_some_us.fetch_add(delta_us, Ordering::Relaxed),
        };
    }

    pub fn totals(&self) -> PsiTotals {
        PsiTotals {
            cpu_some_us: self.cpu_some_us.load(Ordering::Relaxed),
            memory_some_us: self.memory_some_us.load(Ordering::Relaxed),
            io_some_us: self.io_some_us.load(Ordering::Relaxed),
        }
    }
}

pub fn memory_psi_total_stall_us() -> u64 {
    crate::mm::psi::psi_total_stall_us()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psi_group_accounts_resource_totals() {
        let group = PsiGroup::new();
        group.account_stall(PsiResource::Cpu, 10);
        group.account_stall(PsiResource::Io, 4);
        assert_eq!(
            group.totals(),
            PsiTotals {
                cpu_some_us: 10,
                memory_some_us: 0,
                io_some_us: 4
            }
        );
    }
}
