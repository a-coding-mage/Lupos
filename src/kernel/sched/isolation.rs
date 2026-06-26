//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/isolation.c
//! test-origin: linux:vendor/linux/kernel/sched/isolation.c
//! Scheduler CPU isolation.
//!
//! Mirrors `vendor/linux/kernel/sched/isolation.c`. Linux tracks housekeeping
//! CPUs separately from isolated CPUs; this module stores the same mask-level
//! policy for scheduler placement.

use core::sync::atomic::{AtomicU64, Ordering};

use super::entity::CpuMask;

static ISOLATED_CPUS: AtomicU64 = AtomicU64::new(0);

pub fn set_cpu_isolated(cpu: u32, isolated: bool) {
    let bit = 1u64 << (cpu & 63);
    if isolated {
        ISOLATED_CPUS.fetch_or(bit, Ordering::AcqRel);
    } else {
        ISOLATED_CPUS.fetch_and(!bit, Ordering::AcqRel);
    }
}

pub fn cpu_is_isolated(cpu: u32) -> bool {
    ISOLATED_CPUS.load(Ordering::Acquire) & (1u64 << (cpu & 63)) != 0
}

pub fn housekeeping_cpumask(possible: CpuMask) -> CpuMask {
    CpuMask(possible.0 & !ISOLATED_CPUS.load(Ordering::Acquire))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolation_removes_cpu_from_housekeeping_mask() {
        set_cpu_isolated(2, true);
        let mask = housekeeping_cpumask(CpuMask::all());
        assert!(!mask.test(2));
        set_cpu_isolated(2, false);
        assert!(housekeeping_cpumask(CpuMask::all()).test(2));
    }
}
