//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/mm/init.c
//! test-origin: linux:vendor/linux/arch/x86/mm/init.c
//! x86 memory initialization policy.
//!
//! Mirrors the direct-map and cache-mode policy pieces from
//! `vendor/linux/arch/x86/mm/init.c`. Boot-time page-table construction stays
//! in the existing x86 paging code; this module exposes the decisions that are
//! safe to test outside early boot.

#[cfg(not(test))]
extern crate alloc;

#[cfg(not(test))]
use alloc::vec::Vec;
#[cfg(not(test))]
use lazy_static::lazy_static;
#[cfg(not(test))]
use spin::Mutex;

use crate::arch::x86::mm::paging::{__pgprot, PAGE_MASK, PAGE_SIZE, PMD_SIZE, PUD_SIZE};
#[cfg(not(test))]
use crate::arch::x86::mm::paging::{
    _PAGE_ACCESSED, _PAGE_DIRTY, _PAGE_GLOBAL, _PAGE_NX, _PAGE_PRESENT, _PAGE_RW, map_kernel_page,
    unmap_kernel_page, virt_to_phys,
};
use crate::arch::x86::mm::pat::{PageCacheMode, cachemode_to_pte_flags};
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
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
#[derive(Clone, Copy)]
struct ExecmemRange {
    start: u64,
    size: usize,
}

#[cfg(not(test))]
struct ExecmemState {
    ready: bool,
    free: Vec<ExecmemRange>,
    live: Vec<ExecmemRange>,
}

#[cfg(not(test))]
impl ExecmemState {
    fn new() -> Self {
        Self {
            ready: false,
            free: Vec::new(),
            live: Vec::new(),
        }
    }

    fn ensure_ready(&mut self) -> bool {
        if self.ready {
            return true;
        }
        if self.free.try_reserve(1).is_err() {
            return false;
        }
        self.free.push(ExecmemRange {
            start: MODULES_VADDR,
            size: (MODULES_END - MODULES_VADDR) as usize,
        });
        self.ready = true;
        true
    }
}

#[cfg(not(test))]
lazy_static! {
    static ref EXECMEM_STATE: Mutex<ExecmemState> = Mutex::new(ExecmemState::new());
}

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

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("cachemode2protval", linux_cachemode2protval as usize, false);
}

pub fn cachemode2protval(mode: PageCacheMode) -> u64 {
    cachemode_to_pte_flags(mode)
}

fn linux_page_cache_mode(mode: u32) -> Option<PageCacheMode> {
    match mode {
        0 => Some(PageCacheMode::WriteBack),
        1 => Some(PageCacheMode::WriteCombining),
        2 => Some(PageCacheMode::UncachedMinus),
        3 => Some(PageCacheMode::Uncached),
        4 => Some(PageCacheMode::WriteThrough),
        5 => Some(PageCacheMode::WriteProtected),
        _ => None,
    }
}

/// `cachemode2protval` - `vendor/linux/arch/x86/mm/init.c:64`.
pub extern "C" fn linux_cachemode2protval(mode: u32) -> u64 {
    linux_page_cache_mode(mode)
        .map(cachemode2protval)
        .unwrap_or(0)
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
    for (idx, range) in state.free.iter().enumerate() {
        if range.size < size {
            continue;
        }
        match best_idx {
            None => best_idx = Some(idx),
            Some(best) if range.size < state.free[best].size => best_idx = Some(idx),
            _ => {}
        }
    }

    let idx = best_idx?;
    let range = state.free.swap_remove(idx);
    if range.size > size {
        state.free.push(ExecmemRange {
            start: range.start + size as u64,
            size: range.size - size,
        });
    }
    Some(range.start)
}

#[cfg(not(test))]
fn free_execmem_va(state: &mut ExecmemState, start: u64, size: usize) {
    let mut merged = ExecmemRange { start, size };
    let mut index = 0;
    while index < state.free.len() {
        let range = state.free[index];
        if range.start + range.size as u64 == merged.start {
            merged.start = range.start;
            merged.size += range.size;
            state.free.swap_remove(index);
            index = 0;
        } else if merged.start + merged.size as u64 == range.start {
            merged.size += range.size;
            state.free.swap_remove(index);
            index = 0;
        } else {
            index += 1;
        }
    }
    if state.free.try_reserve(1).is_ok() {
        state.free.push(merged);
    }
}

#[cfg(not(test))]
fn remember_execmem_live(state: &mut ExecmemState, start: u64, size: usize) -> bool {
    if state.live.try_reserve(1).is_err() {
        false
    } else {
        state.live.push(ExecmemRange { start, size });
        true
    }
}

#[cfg(not(test))]
fn take_execmem_live(state: &mut ExecmemState, start: u64) -> Option<ExecmemRange> {
    let index = state.live.iter().position(|range| range.start == start)?;
    Some(state.live.swap_remove(index))
}

#[cfg(not(test))]
fn unmap_execmem_pages(start: u64, size: usize) {
    for off in (0..size).step_by(PAGE_SIZE as usize) {
        let va = start + off as u64;
        if let Some(phys) = virt_to_phys(va) {
            unsafe { unmap_kernel_page(va) };
            let page = pfn_to_page((phys >> 12) as usize);
            with_global_buddy(|buddy| buddy.free_pages(page, 0));
        } else {
            unsafe { unmap_kernel_page(va) };
        }
    }
}

/// Allocate writable, non-executable memory in Linux's x86 module range.
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
        if !state.ensure_ready() {
            return core::ptr::null_mut();
        }
        let Some(start) = reserve_execmem_va(&mut state, size) else {
            return core::ptr::null_mut();
        };
        if !remember_execmem_live(&mut state, start, size) {
            free_execmem_va(&mut state, start, size);
            return core::ptr::null_mut();
        }
        start
    };

    // `execmem_alloc_rw()` first makes module memory non-executable, then
    // writable.  Text becomes ROX only after relocation and architecture
    // finalization; data remains NX.  Lupos does not yet implement that final
    // per-memory-type transition, so the module loader gates real modules
    // before allocation.
    let execmem_rw =
        __pgprot(_PAGE_PRESENT | _PAGE_RW | _PAGE_NX | _PAGE_ACCESSED | _PAGE_DIRTY | _PAGE_GLOBAL);

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

/// Apply the final Linux module permission for one page-aligned execmem
/// allocation after relocation and architecture finalization.
///
/// `module_enable_rodata_ro()`, `module_enable_data_nx()`, and
/// `module_enable_text_rox()` in `vendor/linux/kernel/module/strict_rwx.c`
/// produce exactly three useful states: ROX text, RO+NX rodata, and RW+NX
/// data.  Lupos currently allocates each SHF_ALLOC section separately, so the
/// same transition is applied to the retained section allocation rather than
/// to Linux's grouped memory class.
#[cfg(not(test))]
pub fn execmem_set_final_permissions(
    ptr: *mut u8,
    size: usize,
    writable: bool,
    executable: bool,
) -> Result<(), i32> {
    if ptr.is_null() || size == 0 || (writable && executable) {
        return Err(EINVAL);
    }
    let start = ptr as u64;
    let allocation = {
        let state = EXECMEM_STATE.lock();
        state
            .live
            .iter()
            .find(|range| range.start == start)
            .copied()
    }
    .ok_or(EINVAL)?;
    if size > allocation.size {
        return Err(EINVAL);
    }

    let mut flags = _PAGE_PRESENT | _PAGE_ACCESSED | _PAGE_DIRTY | _PAGE_GLOBAL;
    if writable {
        flags |= _PAGE_RW;
    }
    if !executable {
        flags |= _PAGE_NX;
    }
    let prot = __pgprot(flags);

    for off in (0..allocation.size).step_by(PAGE_SIZE as usize) {
        let virt = allocation.start + off as u64;
        let phys = virt_to_phys(virt).ok_or(EINVAL)?;
        unsafe { map_kernel_page(virt, phys, prot) };
    }
    Ok(())
}

#[cfg(test)]
pub fn execmem_set_final_permissions(
    ptr: *mut u8,
    size: usize,
    writable: bool,
    executable: bool,
) -> Result<(), i32> {
    if ptr.is_null() || size == 0 || (writable && executable) {
        Err(EINVAL)
    } else {
        Ok(())
    }
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
    fn cachemode2protval_returns_linux_cache_bits_only() {
        assert_eq!(
            cachemode2protval(PageCacheMode::Uncached),
            _PAGE_PCD | _PAGE_PWT
        );
        assert_eq!(
            cachemode2protval(PageCacheMode::WriteBack) & (_PAGE_PCD | _PAGE_PWT),
            0
        );
    }

    #[test]
    fn module_exports_include_cachemode2protval() {
        register_module_exports();
        assert_eq!(
            find_symbol("cachemode2protval"),
            Some(linux_cachemode2protval as usize)
        );
        assert_eq!(
            linux_cachemode2protval(PageCacheMode::Uncached as u32),
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
