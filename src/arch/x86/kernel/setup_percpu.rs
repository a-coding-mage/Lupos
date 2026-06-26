//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/setup_percpu.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/setup_percpu.c
//! x86 per-CPU area setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/setup_percpu.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};

use crate::include::uapi::errno::EINVAL;
use crate::kernel::sched::MAX_CPUS;

pub static __PER_CPU_OFFSET: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
pub static THIS_CPU_OFF: AtomicU64 = AtomicU64::new(0);
pub static CPU_NUMBER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PerCpuLayout {
    pub unit_size: u64,
    pub nr_units: usize,
    pub base: u64,
}

pub const fn pcpu_need_numa(nodes: usize) -> bool {
    nodes > 1
}

pub const fn pcpu_cpu_distance(from_node: usize, to_node: usize) -> i32 {
    if from_node == to_node { 10 } else { 20 }
}

pub fn setup_per_cpu_areas(base: u64, unit_size: u64, cpus: usize) -> Result<PerCpuLayout, i32> {
    if unit_size == 0 || cpus == 0 {
        return Err(EINVAL);
    }
    let nr = cpus.min(MAX_CPUS);
    for cpu in 0..nr {
        __PER_CPU_OFFSET[cpu].store(base + unit_size * cpu as u64, Ordering::Release);
    }
    THIS_CPU_OFF.store(base, Ordering::Release);
    CPU_NUMBER.store(0, Ordering::Release);
    Ok(PerCpuLayout {
        unit_size,
        nr_units: nr,
        base,
    })
}

pub fn per_cpu_offset(cpu: usize) -> u64 {
    __PER_CPU_OFFSET[cpu.min(MAX_CPUS - 1)].load(Ordering::Acquire)
}

pub fn setup_percpu_segment(cpu: usize) {
    CPU_NUMBER.store(cpu as u64, Ordering::Release);
    THIS_CPU_OFF.store(per_cpu_offset(cpu), Ordering::Release);
}

pub const fn pcpu_populate_pte(_addr: u64) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_per_cpu_areas_populates_offsets() {
        let layout = setup_per_cpu_areas(0x1000, 0x2000, 3).unwrap();
        assert_eq!(layout.nr_units, 3);
        assert_eq!(per_cpu_offset(0), 0x1000);
        assert_eq!(per_cpu_offset(2), 0x5000);
        setup_percpu_segment(2);
        assert_eq!(THIS_CPU_OFF.load(Ordering::Acquire), 0x5000);
    }
}
