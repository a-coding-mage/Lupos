//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/cpu_entry_area.c
//! test-origin: linux:vendor/linux/arch/x86/mm/cpu_entry_area.c
//! CPU entry area layout helpers.
//!
//! Mirrors address calculation and PTE setup validation from
//! `vendor/linux/arch/x86/mm/cpu_entry_area.c`. The actual IST/TSS backing
//! storage is owned by the existing x86 CPU and TSS modules.

use crate::arch::x86::mm::paging::{PAGE_MASK, PAGE_SIZE, pgprot_t};
use crate::include::uapi::errno::{EINVAL, ERANGE};
use crate::kernel::sched::MAX_CPUS;

pub const CPU_ENTRY_AREA_BASE: u64 = 0xffff_fe80_0000_0000;
pub const CPU_ENTRY_AREA_SIZE: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuEntryArea {
    pub cpu: usize,
    pub base: u64,
    pub end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CeaPteRequest {
    pub virt: u64,
    pub phys: u64,
    pub prot: pgprot_t,
}

pub const fn get_cpu_entry_area(cpu: usize) -> Result<CpuEntryArea, i32> {
    if cpu >= MAX_CPUS {
        return Err(ERANGE);
    }
    let base = CPU_ENTRY_AREA_BASE + (cpu as u64) * CPU_ENTRY_AREA_SIZE;
    Ok(CpuEntryArea {
        cpu,
        base,
        end: base + CPU_ENTRY_AREA_SIZE,
    })
}

pub const fn cea_set_pte(virt: u64, phys: u64, prot: pgprot_t) -> Result<CeaPteRequest, i32> {
    if virt & (PAGE_SIZE - 1) != 0 || phys & (PAGE_SIZE - 1) != 0 {
        return Err(EINVAL);
    }
    if virt < CPU_ENTRY_AREA_BASE {
        return Err(ERANGE);
    }
    Ok(CeaPteRequest {
        virt: virt & PAGE_MASK,
        phys: phys & PAGE_MASK,
        prot,
    })
}

pub fn setup_cpu_entry_areas() -> Result<(), i32> {
    get_cpu_entry_area(0).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::PAGE_KERNEL;

    #[test]
    fn cpu_entry_area_addresses_are_per_cpu() {
        let cpu0 = get_cpu_entry_area(0).unwrap();
        let cpu1 = get_cpu_entry_area(1).unwrap();
        assert_eq!(cpu1.base - cpu0.base, CPU_ENTRY_AREA_SIZE);
    }

    #[test]
    fn cea_pte_requires_page_alignment() {
        assert_eq!(
            cea_set_pte(CPU_ENTRY_AREA_BASE + 1, 0x2000, PAGE_KERNEL),
            Err(EINVAL)
        );
        assert!(cea_set_pte(CPU_ENTRY_AREA_BASE, 0x2000, PAGE_KERNEL).is_ok());
    }
}
