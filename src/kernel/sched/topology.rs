//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched/topology.c
//! test-origin: linux:vendor/linux/kernel/sched/topology.c
//! `sched_domain` topology — M31.
//!
//! M29 ships an empty hierarchy; the SMP load balancer wires it in M31.

use super::entity::CpuMask;
use core::sync::atomic::{AtomicU64, Ordering};

/// Linux `SD_*` domain flags.
pub const SD_LOAD_BALANCE: u32 = 0x0001;
pub const SD_BALANCE_NEWIDLE: u32 = 0x0002;
pub const SD_BALANCE_EXEC: u32 = 0x0004;
pub const SD_BALANCE_FORK: u32 = 0x0008;
pub const SD_BALANCE_WAKE: u32 = 0x0010;
pub const SD_WAKE_AFFINE: u32 = 0x0020;
pub const SD_SHARE_CPUCAPACITY: u32 = 0x0080;
pub const SD_SHARE_LLC: u32 = 0x0200;
pub const SD_SHARE_PKG_RESOURCES: u32 = 0x0400;
pub const SD_NUMA: u32 = 0x4000;

/// Linux `struct sched_domain` — one level in the topology hierarchy.
///
/// M29: skeletal; only `cpus` and `flags` are consumed by M31's load balancer.
pub struct SchedDomain {
    pub cpus: CpuMask,
    pub flags: u32,
    pub min_interval: u32,
    pub max_interval: u32,
    pub busy_factor: u32,
    pub level: u8,
}

impl SchedDomain {
    pub const fn empty() -> Self {
        Self {
            cpus: CpuMask::empty(),
            flags: 0,
            min_interval: 1,
            max_interval: 32,
            busy_factor: 16,
            level: 0,
        }
    }
}

/// CPUs covered by the boot-time scheduler domain.
///
/// Lupos currently has one flat domain. Keeping its span synchronized with
/// `cpu_active_mask` preserves Linux's key placement invariant until SMT/MC
/// child domains are represented.
static SCHED_DOMAIN_CPUS: AtomicU64 = AtomicU64::new(1);

/// Initialise the system's sched_domain hierarchy from the active CPU mask.
///
/// Linux builds the final domains in `sched_init_smp()` after AP activation.
/// The flat Lupos domain follows the same publication point.
pub fn init_sched_domains() {
    SCHED_DOMAIN_CPUS.store(super::cpu_active_mask().0, Ordering::Release);
}

pub fn sched_domain_cpus() -> CpuMask {
    CpuMask(SCHED_DOMAIN_CPUS.load(Ordering::Acquire))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_domain_has_no_cpus() {
        let d = SchedDomain::empty();
        assert_eq!(d.cpus.weight(), 0);
        assert_eq!(d.flags, 0);
    }
}
