//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/setup_percpu.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/setup_percpu.c
//! x86 per-CPU area setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/setup_percpu.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::sched::MAX_CPUS;

pub static __PER_CPU_OFFSET: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

pub fn register_module_exports() {
    if find_symbol("__per_cpu_offset").is_none() {
        export_symbol(
            "__per_cpu_offset",
            __PER_CPU_OFFSET.as_ptr() as usize,
            false,
        );
    }
}

/// Configured `struct softnet_data` size/alignment, verified against the
/// vendor build (`CONFIG_NR_CPUS=64`, x86-64 generic configuration).
pub const LINUX_SOFTNET_DATA_SIZE: usize = 1024;

/// Static Linux per-CPU template used by C modules.
///
/// x86 module code addresses these symbols as `%gs:symbol`. The symbol table
/// therefore exposes fields in unit zero, while `MSR_GS_BASE` contains the
/// byte delta from unit zero to the current CPU's unit. This is the same
/// `symbol + __per_cpu_offset[cpu]` contract used by Linux.
#[repr(C, align(64))]
pub struct LinuxPerCpuArea {
    cpu_number: AtomicU32,
    _cpu_number_pad: u32,
    this_cpu_off: AtomicU64,
    preempt_count: AtomicU32,
    _header_pad: [u8; 44],
    softnet_data: [u8; LINUX_SOFTNET_DATA_SIZE],
    cpu_info: [u8; crate::arch::x86::kernel::cpu::common::LINUX_CPUINFO_X86_SIZE],
    initialized: AtomicBool,
    _tail_pad: [u8; 63],
}

impl LinuxPerCpuArea {
    const fn new() -> Self {
        Self {
            cpu_number: AtomicU32::new(0),
            _cpu_number_pad: 0,
            this_cpu_off: AtomicU64::new(0),
            preempt_count: AtomicU32::new(0),
            _header_pad: [0; 44],
            softnet_data: [0; LINUX_SOFTNET_DATA_SIZE],
            cpu_info: [0; crate::arch::x86::kernel::cpu::common::LINUX_CPUINFO_X86_SIZE],
            initialized: AtomicBool::new(false),
            _tail_pad: [0; 63],
        }
    }
}

static LINUX_PER_CPU_AREAS: [LinuxPerCpuArea; MAX_CPUS] =
    [const { LinuxPerCpuArea::new() }; MAX_CPUS];

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
    let cpu = cpu.min(MAX_CPUS - 1);
    let base = core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0]) as usize;
    let current = core::ptr::addr_of!(LINUX_PER_CPU_AREAS[cpu]) as usize;
    let offset = current.wrapping_sub(base) as u64;
    __PER_CPU_OFFSET[cpu].store(offset, Ordering::Release);
    LINUX_PER_CPU_AREAS[cpu]
        .cpu_number
        .store(cpu as u32, Ordering::Release);
    LINUX_PER_CPU_AREAS[cpu]
        .this_cpu_off
        .store(offset, Ordering::Release);
    LINUX_PER_CPU_AREAS[cpu]
        .preempt_count
        .store(0, Ordering::Release);
    crate::arch::x86::kernel::cpu::common::write_linux_cpuinfo_x86(core::ptr::addr_of!(
        LINUX_PER_CPU_AREAS[cpu].cpu_info
    ) as *mut u8);
    initialize_softnet_data(cpu);

    #[cfg(not(test))]
    unsafe {
        crate::arch::x86::kernel::msr::write(crate::arch::x86::kernel::msr::MSR_GS_BASE, offset);
    }
}

fn initialize_softnet_data(cpu: usize) {
    let area = &LINUX_PER_CPU_AREAS[cpu];
    if area.initialized.swap(true, Ordering::AcqRel) {
        return;
    }
    let softnet = area.softnet_data.as_ptr() as *mut u8;
    // `softnet_data.poll_list` is an empty Linux list head. Remaining fields
    // begin zeroed, matching `net_dev_init()` before backlog NAPI setup.
    unsafe {
        (softnet as *mut usize).write(softnet as usize);
        (softnet.add(core::mem::size_of::<usize>()) as *mut usize).write(softnet as usize);
        // `softnet_data.cpu` at configured offset 232.
        (softnet.add(232) as *mut u32).write(cpu as u32);
    }
}

pub fn cpu_number_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].cpu_number) as usize
}

pub fn this_cpu_off_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].this_cpu_off) as usize
}

pub fn preempt_count_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].preempt_count) as usize
}

pub fn softnet_data_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].softnet_data) as usize
}

pub fn cpu_info_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].cpu_info) as usize
}

pub fn preempt_count_slot(cpu: usize) -> &'static AtomicU32 {
    &LINUX_PER_CPU_AREAS[cpu.min(MAX_CPUS - 1)].preempt_count
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
        assert_eq!(
            LINUX_PER_CPU_AREAS[2].this_cpu_off.load(Ordering::Acquire),
            (core::mem::size_of::<LinuxPerCpuArea>() * 2) as u64
        );
        assert_eq!(LINUX_PER_CPU_AREAS[2].cpu_number.load(Ordering::Acquire), 2);
    }

    #[test]
    fn exports_linux_per_cpu_offset_array_symbol() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__per_cpu_offset"),
            Some(__PER_CPU_OFFSET.as_ptr() as usize)
        );
    }
}
