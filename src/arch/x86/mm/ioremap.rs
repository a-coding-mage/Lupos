//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/mm/ioremap.c
//! test-origin: linux:vendor/linux/arch/x86/mm/ioremap.c
//! x86 ioremap helpers.
//!
//! Linux's ioremap path maps device physical ranges into a vmalloc-like kernel
//! virtual window with a cache mode selected by PAT. Lupos keeps the same
//! alignment and offset semantics here and backs it with `map_kernel_page`.
//! It does not yet implement Linux's memtype reservation/alias arbitration or
//! reusable vmalloc-area allocator, so callers must not treat cache-mode
//! conflict handling and long-lived VA reuse as complete.
//!
//! References:
//! - `vendor/linux/arch/x86/mm/ioremap.c`

use core::sync::atomic::{AtomicU64, Ordering};

use super::paging::{
    PAGE_KERNEL, PAGE_MASK, PAGE_SIZE, map_kernel_page, pgprot_t, unmap_kernel_page,
};
use super::pat::{self, PageCacheMode};

pub const IOREMAP_BASE: u64 = 0xffff_fd00_0000_0000;
pub const IOREMAP_END: u64 = 0xffff_fd80_0000_0000;

static NEXT_IOREMAP: AtomicU64 = AtomicU64::new(IOREMAP_BASE);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoremapMapping {
    pub virt: u64,
    pub phys: u64,
    pub size: u64,
    pub prot: pgprot_t,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoremapError {
    Empty,
    Overflow,
    Exhausted,
}

pub const fn prot_for_cachemode(mode: PageCacheMode) -> pgprot_t {
    pat::pgprot_with_cachemode(PAGE_KERNEL, mode)
}

pub unsafe fn ioremap(phys: u64, size: u64) -> Result<IoremapMapping, IoremapError> {
    unsafe { ioremap_cachemode(phys, size, PageCacheMode::UncachedMinus) }
}

pub unsafe fn ioremap_uc(phys: u64, size: u64) -> Result<IoremapMapping, IoremapError> {
    unsafe { ioremap_cachemode(phys, size, PageCacheMode::Uncached) }
}

pub unsafe fn ioremap_wc(phys: u64, size: u64) -> Result<IoremapMapping, IoremapError> {
    unsafe { ioremap_cachemode(phys, size, PageCacheMode::WriteCombining) }
}

pub unsafe fn ioremap_cachemode(
    phys: u64,
    size: u64,
    mode: PageCacheMode,
) -> Result<IoremapMapping, IoremapError> {
    if size == 0 {
        return Err(IoremapError::Empty);
    }
    let offset = phys & (PAGE_SIZE - 1);
    let phys_base = phys & PAGE_MASK;
    let rounded = size
        .checked_add(offset)
        .and_then(|n| n.checked_add(PAGE_SIZE - 1))
        .map(|n| n & PAGE_MASK)
        .ok_or(IoremapError::Overflow)?;
    let virt_base = reserve_ioremap_va(rounded)?;
    let prot = prot_for_cachemode(mode);
    let mut done = 0;
    while done < rounded {
        unsafe { map_kernel_page(virt_base + done, phys_base + done, prot) };
        done += PAGE_SIZE;
    }
    Ok(IoremapMapping {
        virt: virt_base + offset,
        phys,
        size,
        prot,
    })
}

pub unsafe fn iounmap(mapping: IoremapMapping) {
    let offset = mapping.virt & (PAGE_SIZE - 1);
    let virt_base = mapping.virt & PAGE_MASK;
    let rounded = mapping
        .size
        .saturating_add(offset)
        .saturating_add(PAGE_SIZE - 1)
        & PAGE_MASK;
    let mut done = 0;
    while done < rounded {
        unsafe { unmap_kernel_page(virt_base + done) };
        done += PAGE_SIZE;
    }
}

pub fn is_ioremap_addr(addr: *const u8) -> bool {
    let addr = addr as u64;
    (IOREMAP_BASE..IOREMAP_END).contains(&addr)
}

#[cfg(not(test))]
pub unsafe fn ioremap_fault(addr: u64) -> bool {
    if !is_ioremap_addr(addr as *const u8) {
        return false;
    }
    if super::paging::virt_to_phys(addr).is_none() {
        return false;
    }

    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return false;
    }
    let mm = unsafe {
        if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        }
    };
    if mm.is_null() {
        return false;
    }

    unsafe {
        crate::mm::vmalloc::sync_vmalloc_pgd_slot_to_mm(mm, addr & PAGE_MASK, PAGE_SIZE as usize)
    };
    true
}

#[cfg(test)]
pub unsafe fn ioremap_fault(addr: u64) -> bool {
    is_ioremap_addr(addr as *const u8)
}

fn reserve_ioremap_va(size: u64) -> Result<u64, IoremapError> {
    let size = (size + PAGE_SIZE - 1) & PAGE_MASK;
    let base = NEXT_IOREMAP.fetch_add(size, Ordering::AcqRel);
    if base.checked_add(size).ok_or(IoremapError::Overflow)? > IOREMAP_END {
        return Err(IoremapError::Exhausted);
    }
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::{_PAGE_PCD, _PAGE_PWT, pgprot_val, virt_to_phys};

    #[test]
    fn ioremap_preserves_unaligned_offset_and_maps_base_page() {
        let m = unsafe { ioremap(0x12345, 16).expect("ioremap") };
        assert_eq!(m.virt & 0xfff, 0x345);
        assert_eq!(virt_to_phys(m.virt & PAGE_MASK), Some(0x12000));
        unsafe { iounmap(m) };
    }

    #[test]
    fn ioremap_cache_modes_select_pat_bits() {
        let uc = prot_for_cachemode(PageCacheMode::Uncached);
        assert_eq!(
            pgprot_val(uc) & (_PAGE_PCD | _PAGE_PWT),
            _PAGE_PCD | _PAGE_PWT
        );
        let wc = prot_for_cachemode(PageCacheMode::WriteCombining);
        assert_eq!(pgprot_val(wc) & (_PAGE_PCD | _PAGE_PWT), _PAGE_PWT);
    }

    #[test]
    fn ioremap_rejects_empty_mapping() {
        assert_eq!(unsafe { ioremap(0x1000, 0) }, Err(IoremapError::Empty));
    }

    #[test]
    fn ioremap_fault_accepts_ioremap_window_only() {
        assert!(unsafe { ioremap_fault(IOREMAP_BASE) });
        assert!(!unsafe { ioremap_fault(IOREMAP_BASE - PAGE_SIZE) });
        assert!(!unsafe { ioremap_fault(IOREMAP_END) });
    }
}
