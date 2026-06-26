//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/init.c
//! test-origin: linux:vendor/linux/arch/x86/mm/init.c
//! x86 memory initialization policy.
//!
//! Mirrors the direct-map and cache-mode policy pieces from
//! `vendor/linux/arch/x86/mm/init.c`. Boot-time page-table construction stays
//! in the existing x86 paging code; this module exposes the decisions that are
//! safe to test outside early boot.

#[cfg(not(test))]
use spin::Mutex;

use crate::arch::x86::mm::paging::{
    __pgprot, PAGE_MASK, PAGE_SIZE, PMD_SIZE, PUD_SIZE, pgprot_val,
};
#[cfg(not(test))]
use crate::arch::x86::mm::paging::{
    _PAGE_ACCESSED, _PAGE_DIRTY, _PAGE_GLOBAL, _PAGE_PRESENT, _PAGE_RW, map_kernel_page,
    unmap_kernel_page, virt_to_phys,
};
use crate::arch::x86::mm::pat::{PageCacheMode, pgprot_with_cachemode};
use crate::include::uapi::errno::EINVAL;
#[cfg(not(test))]
use crate::mm::buddy::{is_buddy_ready, page_to_pfn, pfn_to_page, with_global_buddy};
#[cfg(not(test))]
use crate::mm::page_flags::GFP_KERNEL;

/// Linux `__START_KERNEL_map`.
///
/// Source: `vendor/linux/arch/x86/include/asm/page_64_types.h`.
pub const START_KERNEL_MAP: u64 = 0xffff_ffff_8000_0000;

/// Linux `KERNEL_IMAGE_SIZE`; modules start after this kernel-image window.
///
/// Source: `vendor/linux/arch/x86/include/asm/page_64_types.h`.
pub const KERNEL_IMAGE_SIZE: u64 = 0x4000_0000;

/// Linux `MODULES_VADDR`.
///
/// Source: `vendor/linux/arch/x86/include/asm/pgtable_64_types.h`.
pub const MODULES_VADDR: u64 = START_KERNEL_MAP + KERNEL_IMAGE_SIZE;

/// Linux `MODULES_END`, exclusive in Lupos range checks.
///
/// Source: `vendor/linux/arch/x86/include/asm/pgtable_64_types.h`.
pub const MODULES_END: u64 = 0xffff_ffff_ff00_0000;

/// Linux `MODULE_ALIGN` without KASAN shadow scaling.
///
/// Source: `vendor/linux/include/linux/execmem.h`.
pub const MODULE_ALIGN: u64 = PAGE_SIZE;

#[cfg(not(test))]
const EXECMEM_ALLOCS: usize = 128;

#[cfg(not(test))]
#[derive(Clone, Copy)]
struct ExecmemRange {
    start: u64,
    size: usize,
}

#[cfg(not(test))]
struct ExecmemState {
    ready: bool,
    free: [Option<ExecmemRange>; EXECMEM_ALLOCS],
    live: [Option<ExecmemRange>; EXECMEM_ALLOCS],
}

#[cfg(not(test))]
impl ExecmemState {
    const fn new() -> Self {
        Self {
            ready: false,
            free: [None; EXECMEM_ALLOCS],
            live: [None; EXECMEM_ALLOCS],
        }
    }

    fn ensure_ready(&mut self) {
        if self.ready {
            return;
        }
        self.free[0] = Some(ExecmemRange {
            start: MODULES_VADDR,
            size: (MODULES_END - MODULES_VADDR) as usize,
        });
        self.ready = true;
    }
}

#[cfg(not(test))]
static EXECMEM_STATE: Mutex<ExecmemState> = Mutex::new(ExecmemState::new());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectMapRange {
    pub start: u64,
    pub end: u64,
    pub use_gbpage: bool,
}

pub const fn direct_gbpages_enabled(cmdline_disabled: bool, cpu_has_gbpages: bool) -> bool {
    !cmdline_disabled && cpu_has_gbpages
}

pub const fn align_direct_map_range(start: u64, size: u64) -> Result<(u64, u64), i32> {
    if size == 0 {
        return Err(EINVAL);
    }
    let end = match start.checked_add(size) {
        Some(end) => end,
        None => return Err(EINVAL),
    };
    Ok((start & PAGE_MASK, (end + PAGE_SIZE - 1) & PAGE_MASK))
}

pub const fn can_use_gbpage(start: u64, end: u64, enabled: bool) -> bool {
    enabled && start & (PUD_SIZE - 1) == 0 && end & (PUD_SIZE - 1) == 0 && end > start
}

pub const fn kernel_physical_mapping_init(
    start: u64,
    size: u64,
    gbpages_enabled: bool,
) -> Result<DirectMapRange, i32> {
    let (start, end) = match align_direct_map_range(start, size) {
        Ok(range) => range,
        Err(err) => return Err(err),
    };
    Ok(DirectMapRange {
        start,
        end,
        use_gbpage: can_use_gbpage(start, end, gbpages_enabled),
    })
}

pub fn cachemode2protval(mode: PageCacheMode) -> u64 {
    pgprot_val(pgprot_with_cachemode(
        crate::arch::x86::mm::paging::PAGE_KERNEL,
        mode,
    ))
}

pub const fn min_mapping_granularity(use_gbpage: bool) -> u64 {
    if use_gbpage { PUD_SIZE } else { PMD_SIZE }
}

pub const fn is_module_addr_value(addr: u64) -> bool {
    addr >= MODULES_VADDR && addr < MODULES_END
}

pub fn is_module_addr(addr: *const u8) -> bool {
    is_module_addr_value(addr as u64)
}

pub const fn fits_x86_64_reloc_32s(value: u64) -> bool {
    value as i64 == value as i32 as i64
}

pub const fn execmem_module_range() -> (u64, u64) {
    (MODULES_VADDR, MODULES_END)
}

#[cfg(not(test))]
fn align_up(value: usize, align: usize) -> Option<usize> {
    value.checked_add(align - 1).map(|v| v & !(align - 1))
}

#[cfg(not(test))]
fn reserve_execmem_va(state: &mut ExecmemState, size: usize) -> Option<u64> {
    let mut best_idx = None;
    for (idx, slot) in state.free.iter().enumerate() {
        if let Some(range) = slot {
            if range.size < size {
                continue;
            }
            match best_idx {
                None => best_idx = Some(idx),
                Some(best) if range.size < state.free[best].unwrap().size => best_idx = Some(idx),
                _ => {}
            }
        }
    }

    let idx = best_idx?;
    let range = state.free[idx].take().unwrap();
    if range.size > size {
        state.free[idx] = Some(ExecmemRange {
            start: range.start + size as u64,
            size: range.size - size,
        });
    }
    Some(range.start)
}

#[cfg(not(test))]
fn free_execmem_va(state: &mut ExecmemState, start: u64, size: usize) {
    let mut merged = ExecmemRange { start, size };
    for slot in state.free.iter_mut() {
        let Some(range) = *slot else {
            continue;
        };
        if range.start + range.size as u64 == merged.start {
            merged.start = range.start;
            merged.size += range.size;
            *slot = None;
        } else if merged.start + merged.size as u64 == range.start {
            merged.size += range.size;
            *slot = None;
        }
    }
    if let Some(slot) = state.free.iter_mut().find(|slot| slot.is_none()) {
        *slot = Some(merged);
    }
}

#[cfg(not(test))]
fn remember_execmem_live(state: &mut ExecmemState, start: u64, size: usize) -> bool {
    if let Some(slot) = state.live.iter_mut().find(|slot| slot.is_none()) {
        *slot = Some(ExecmemRange { start, size });
        true
    } else {
        false
    }
}

#[cfg(not(test))]
fn take_execmem_live(state: &mut ExecmemState, start: u64) -> Option<ExecmemRange> {
    for slot in state.live.iter_mut() {
        if slot.map(|range| range.start == start).unwrap_or(false) {
            return slot.take();
        }
    }
    None
}

#[cfg(not(test))]
fn unmap_execmem_pages(start: u64, size: usize) {
    for off in (0..size).step_by(PAGE_SIZE as usize) {
        let va = start + off as u64;
        if let Some(phys) = virt_to_phys(va) {
            let page = pfn_to_page((phys >> 12) as usize);
            with_global_buddy(|buddy| buddy.free_pages(page, 0));
        }
        unsafe { unmap_kernel_page(va) };
    }
}

/// Allocate writable executable memory in Linux's x86 module range.
///
/// Linux routes module text/data through `execmem_alloc_rw()` with range
/// parameters from `execmem_arch_setup()` in this file's vendor counterpart.
/// Lupos keeps the same address contract because x86 module relocations depend
/// on module addresses being representable by signed 32-bit immediates.
#[cfg(not(test))]
pub fn execmem_alloc_rw(size: usize) -> *mut u8 {
    if size == 0 || !is_buddy_ready() {
        return core::ptr::null_mut();
    }
    let Some(size) = align_up(size, MODULE_ALIGN as usize) else {
        return core::ptr::null_mut();
    };

    let start = {
        let mut state = EXECMEM_STATE.lock();
        state.ensure_ready();
        let Some(start) = reserve_execmem_va(&mut state, size) else {
            return core::ptr::null_mut();
        };
        if !remember_execmem_live(&mut state, start, size) {
            free_execmem_va(&mut state, start, size);
            return core::ptr::null_mut();
        }
        start
    };

    // Writable and executable while the module loader copies sections and
    // applies relocations. Linux later restores ROX where configured; that is
    // a protection upgrade, not part of the driver ABI handoff itself.
    let execmem_rw =
        __pgprot(_PAGE_PRESENT | _PAGE_RW | _PAGE_ACCESSED | _PAGE_DIRTY | _PAGE_GLOBAL);

    let mut mapped = 0usize;
    for off in (0..size).step_by(PAGE_SIZE as usize) {
        let Some(page) = with_global_buddy(|buddy| buddy.alloc_pages(0, GFP_KERNEL)) else {
            break;
        };
        let phys = (page_to_pfn(page) as u64) << 12;
        unsafe { map_kernel_page(start + off as u64, phys, execmem_rw) };
        mapped += PAGE_SIZE as usize;
    }

    if mapped != size {
        unmap_execmem_pages(start, mapped);
        let mut state = EXECMEM_STATE.lock();
        let _ = take_execmem_live(&mut state, start);
        free_execmem_va(&mut state, start, size);
        return core::ptr::null_mut();
    }

    unsafe { core::ptr::write_bytes(start as *mut u8, 0, size) };
    start as *mut u8
}

#[cfg(not(test))]
pub fn execmem_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let start = ptr as u64;
    let range = {
        let mut state = EXECMEM_STATE.lock();
        let Some(range) = take_execmem_live(&mut state, start) else {
            return;
        };
        range
    };
    unmap_execmem_pages(range.start, range.size);
    let mut state = EXECMEM_STATE.lock();
    free_execmem_va(&mut state, range.start, range.size);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::{_PAGE_PCD, _PAGE_PWT};

    #[test]
    fn direct_map_range_aligns_to_pages() {
        assert_eq!(
            kernel_physical_mapping_init(0x1234, 0x1000, false).unwrap(),
            DirectMapRange {
                start: 0x1000,
                end: 0x3000,
                use_gbpage: false
            }
        );
    }

    #[test]
    fn cachemode2protval_uses_existing_pat_mapping() {
        assert_eq!(
            cachemode2protval(PageCacheMode::Uncached) & (_PAGE_PCD | _PAGE_PWT),
            _PAGE_PCD | _PAGE_PWT
        );
    }

    #[test]
    fn x86_module_range_matches_linux_constants() {
        assert_eq!(MODULES_VADDR, 0xffff_ffff_c000_0000);
        assert_eq!(MODULES_END, 0xffff_ffff_ff00_0000);
        assert!(is_module_addr_value(MODULES_VADDR));
        assert!(!is_module_addr_value(MODULES_END));
    }

    #[test]
    fn module_range_fits_x86_64_32s_relocations() {
        assert!(fits_x86_64_reloc_32s(MODULES_VADDR));
        assert!(fits_x86_64_reloc_32s(MODULES_END - MODULE_ALIGN));
        assert!(!fits_x86_64_reloc_32s(0xffff_8880_0000_0000));
    }
}
