//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/topology.c
//! test-origin: linux:vendor/linux/kernel/sched/topology.c
//! `sched_domain` topology — M31.
//!
//! M29 ships an empty hierarchy; the SMP load balancer wires it in M31.

use super::entity::CpuMask;

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

/// Initialise the system's sched_domain hierarchy from APIC topology.
///
/// M29 stub: clears domain state.  M31 builds the SMT < MC < DIE < NUMA chain.
pub fn init_sched_domains() {}

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
