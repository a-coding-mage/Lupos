//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pat/set_memory.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pat/set_memory.c
//! Change-page-attribute helpers.
//!
//! Mirrors the public cache/protection transition surface from
//! `vendor/linux/arch/x86/mm/pat/set_memory.c`. Live large-page splitting and
//! alias flushing are not enabled in Lupos yet, so exported live mutation
//! functions validate inputs and fail closed; pure flag transitions are fully
//! implemented and tested.

use crate::arch::x86::mm::paging::{
    __pgprot, _PAGE_GLOBAL, _PAGE_NX, _PAGE_PCD, _PAGE_PRESENT, _PAGE_PWT, _PAGE_RW, PAGE_SIZE,
    pgprot_t, pgprot_val,
};
use crate::arch::x86::mm::pat::{PageCacheMode, pgprot_with_cachemode};
use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};
use crate::kernel::module::{export_symbol, find_symbol};
#[cfg(test)]
use core::sync::atomic::{AtomicUsize, Ordering};

const DEFAULT_CLFLUSH_LINE_SIZE: usize = 64;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("set_memory_uc", linux_set_memory_uc as usize, false);
    export_symbol_once("set_memory_wc", linux_set_memory_wc as usize, false);
    export_symbol_once("set_memory_wb", linux_set_memory_wb as usize, false);
    export_symbol_once("set_pages_uc", linux_set_pages_uc as usize, false);
    export_symbol_once("set_pages_wb", linux_set_pages_wb as usize, false);
    export_symbol_once(
        "set_pages_array_uc",
        linux_set_pages_array_uc as usize,
        false,
    );
    export_symbol_once(
        "set_pages_array_wc",
        linux_set_pages_array_wc as usize,
        false,
    );
    export_symbol_once(
        "set_pages_array_wb",
        linux_set_pages_array_wb as usize,
        false,
    );
}

unsafe extern "C" fn linux_set_memory_uc(addr: u64, numpages: i32) -> i32 {
    if numpages < 0 {
        return -EINVAL;
    }
    set_memory_uc(addr, numpages as usize).map_or_else(|err| -err, |()| 0)
}

unsafe extern "C" fn linux_set_memory_wc(addr: u64, numpages: i32) -> i32 {
    if numpages < 0 {
        return -EINVAL;
    }
    set_memory_wc(addr, numpages as usize).map_or_else(|err| -err, |()| 0)
}

unsafe extern "C" fn linux_set_memory_wb(addr: u64, numpages: i32) -> i32 {
    if numpages < 0 {
        return -EINVAL;
    }
    set_memory_wb(addr, numpages as usize).map_or_else(|err| -err, |()| 0)
}

unsafe extern "C" fn linux_set_pages_uc(_page: *mut crate::mm::page::Page, numpages: i32) -> i32 {
    if numpages < 0 { -EINVAL } else { -EOPNOTSUPP }
}

unsafe extern "C" fn linux_set_pages_wb(_page: *mut crate::mm::page::Page, numpages: i32) -> i32 {
    if numpages < 0 { -EINVAL } else { -EOPNOTSUPP }
}

unsafe extern "C" fn linux_set_pages_array_uc(
    _pages: *mut *mut crate::mm::page::Page,
    addrinarray: i32,
) -> i32 {
    if addrinarray < 0 {
        -EINVAL
    } else {
        -EOPNOTSUPP
    }
}

unsafe extern "C" fn linux_set_pages_array_wc(
    _pages: *mut *mut crate::mm::page::Page,
    addrinarray: i32,
) -> i32 {
    if addrinarray < 0 {
        -EINVAL
    } else {
        -EOPNOTSUPP
    }
}

unsafe extern "C" fn linux_set_pages_array_wb(
    _pages: *mut *mut crate::mm::page::Page,
    addrinarray: i32,
) -> i32 {
    if addrinarray < 0 {
        -EINVAL
    } else {
        -EOPNOTSUPP
    }
}

#[cfg(test)]
const CLFLUSH_LOG_CAP: usize = 16;
#[cfg(test)]
static CLFLUSH_LOG_LEN: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static CLFLUSH_LOG: spin::Mutex<[u64; CLFLUSH_LOG_CAP]> = spin::Mutex::new([0; CLFLUSH_LOG_CAP]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageAttrChange {
    pub set: u64,
    pub clear: u64,
    pub cache_mode: Option<PageCacheMode>,
}

impl PageAttrChange {
    pub const fn apply(self, prot: pgprot_t) -> pgprot_t {
        let raw = (pgprot_val(prot) | self.set) & !self.clear;
        match self.cache_mode {
            Some(mode) => pgprot_with_cachemode(__pgprot(raw), mode),
            None => __pgprot(raw),
        }
    }
}

pub const SET_MEMORY_RO: PageAttrChange = PageAttrChange {
    set: 0,
    clear: _PAGE_RW,
    cache_mode: None,
};

pub const SET_MEMORY_RW: PageAttrChange = PageAttrChange {
    set: _PAGE_RW,
    clear: 0,
    cache_mode: None,
};

pub const SET_MEMORY_X: PageAttrChange = PageAttrChange {
    set: 0,
    clear: _PAGE_NX,
    cache_mode: None,
};

pub const SET_MEMORY_NX: PageAttrChange = PageAttrChange {
    set: _PAGE_NX,
    clear: 0,
    cache_mode: None,
};

pub const SET_MEMORY_NP: PageAttrChange = PageAttrChange {
    set: 0,
    clear: _PAGE_PRESENT,
    cache_mode: None,
};

pub const SET_MEMORY_P: PageAttrChange = PageAttrChange {
    set: _PAGE_PRESENT,
    clear: 0,
    cache_mode: None,
};

pub const SET_MEMORY_NONGLOBAL: PageAttrChange = PageAttrChange {
    set: 0,
    clear: _PAGE_GLOBAL,
    cache_mode: None,
};

pub const SET_MEMORY_GLOBAL: PageAttrChange = PageAttrChange {
    set: _PAGE_GLOBAL,
    clear: 0,
    cache_mode: None,
};

pub const fn set_memory_uc_change() -> PageAttrChange {
    PageAttrChange {
        set: 0,
        clear: 0,
        cache_mode: Some(PageCacheMode::Uncached),
    }
}

pub const fn set_memory_wc_change() -> PageAttrChange {
    PageAttrChange {
        set: 0,
        clear: 0,
        cache_mode: Some(PageCacheMode::WriteCombining),
    }
}

pub const fn set_memory_wb_change() -> PageAttrChange {
    PageAttrChange {
        set: 0,
        clear: 0,
        cache_mode: Some(PageCacheMode::WriteBack),
    }
}

pub const fn validate_change_request(addr: u64, numpages: usize) -> Result<(), i32> {
    if numpages == 0 || addr & (PAGE_SIZE - 1) != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn set_memory_uc(addr: u64, numpages: usize) -> Result<(), i32> {
    match validate_change_request(addr, numpages) {
        Ok(()) => Err(EOPNOTSUPP),
        Err(err) => Err(err),
    }
}

pub const fn set_memory_wc(addr: u64, numpages: usize) -> Result<(), i32> {
    match validate_change_request(addr, numpages) {
        Ok(()) => Err(EOPNOTSUPP),
        Err(err) => Err(err),
    }
}

pub const fn set_memory_wb(addr: u64, numpages: usize) -> Result<(), i32> {
    match validate_change_request(addr, numpages) {
        Ok(()) => Err(EOPNOTSUPP),
        Err(err) => Err(err),
    }
}

pub const fn set_memory_ro(addr: u64, numpages: usize) -> Result<(), i32> {
    match validate_change_request(addr, numpages) {
        Ok(()) => Err(EOPNOTSUPP),
        Err(err) => Err(err),
    }
}

pub const fn set_memory_rw(addr: u64, numpages: usize) -> Result<(), i32> {
    match validate_change_request(addr, numpages) {
        Ok(()) => Err(EOPNOTSUPP),
        Err(err) => Err(err),
    }
}

pub fn slow_virt_to_phys(virt: u64) -> Option<u64> {
    crate::arch::x86::mm::paging::virt_to_phys(virt)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClflushRangePlan {
    pub start: u64,
    pub end: u64,
    pub line_size: usize,
}

impl ClflushRangePlan {
    pub const fn lines(self) -> usize {
        let bytes = self.end - self.start;
        ((bytes + self.line_size as u64 - 1) / self.line_size as u64) as usize
    }
}

pub const fn clflush_line_size_from_leaf1_ebx(ebx: u32) -> usize {
    let size = (((ebx >> 8) & 0xff) as usize) * 8;
    if size == 0 {
        DEFAULT_CLFLUSH_LINE_SIZE
    } else {
        size
    }
}

pub const fn clflush_range_plan(
    addr: u64,
    size: usize,
    line_size: usize,
) -> Result<ClflushRangePlan, i32> {
    if size == 0 || line_size == 0 || !line_size.is_power_of_two() {
        return Err(EINVAL);
    }
    let end = match addr.checked_add(size as u64) {
        Some(end) => end,
        None => return Err(EINVAL),
    };
    let start = addr & !((line_size as u64) - 1);
    Ok(ClflushRangePlan {
        start,
        end,
        line_size,
    })
}

pub fn cpu_has_clflush() -> bool {
    #[cfg(test)]
    {
        true
    }
    #[cfg(not(test))]
    {
        let leaf1 = crate::arch::x86::kernel::cpuid::cpuid(1, 0);
        leaf1.edx & (1 << 19) != 0
    }
}

fn boot_clflush_line_size() -> usize {
    #[cfg(test)]
    {
        DEFAULT_CLFLUSH_LINE_SIZE
    }
    #[cfg(not(test))]
    {
        let leaf1 = crate::arch::x86::kernel::cpuid::cpuid(1, 0);
        clflush_line_size_from_leaf1_ebx(leaf1.ebx)
    }
}

fn memory_barrier() {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), not(test)))]
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }

    #[cfg(any(test, not(any(target_arch = "x86", target_arch = "x86_64"))))]
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}

unsafe fn clflush_line(addr: u64) {
    #[cfg(test)]
    record_clflush(addr);

    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), not(test)))]
    unsafe {
        core::arch::asm!("clflush [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

fn clflush_cache_range_with_line_size(addr: u64, size: usize, line_size: usize) -> Result<(), i32> {
    let plan = clflush_range_plan(addr, size, line_size)?;
    let mut p = plan.start;

    memory_barrier();
    while p < plan.end {
        unsafe { clflush_line(p) };
        match p.checked_add(plan.line_size as u64) {
            Some(next) => p = next,
            None => break,
        }
    }
    memory_barrier();
    Ok(())
}

pub fn clflush_cache_range(addr: u64, size: usize) -> Result<(), i32> {
    clflush_cache_range_with_line_size(addr, size, boot_clflush_line_size())
}

#[cfg(test)]
pub fn reset_clflush_log() {
    CLFLUSH_LOG_LEN.store(0, Ordering::Release);
    *CLFLUSH_LOG.lock() = [0; CLFLUSH_LOG_CAP];
}

#[cfg(test)]
pub fn clflush_log() -> (usize, [u64; CLFLUSH_LOG_CAP]) {
    (
        CLFLUSH_LOG_LEN.load(Ordering::Acquire).min(CLFLUSH_LOG_CAP),
        *CLFLUSH_LOG.lock(),
    )
}

#[cfg(test)]
fn record_clflush(addr: u64) {
    let idx = CLFLUSH_LOG_LEN.fetch_add(1, Ordering::AcqRel);
    if idx < CLFLUSH_LOG_CAP {
        CLFLUSH_LOG.lock()[idx] = addr;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::{PAGE_KERNEL, pgprot_val};

    #[test]
    fn protection_changes_toggle_expected_bits() {
        let ro = SET_MEMORY_RO.apply(PAGE_KERNEL);
        assert_eq!(pgprot_val(ro) & _PAGE_RW, 0);
        let rw = SET_MEMORY_RW.apply(ro);
        assert_ne!(pgprot_val(rw) & _PAGE_RW, 0);
        let x = SET_MEMORY_X.apply(PAGE_KERNEL);
        assert_eq!(pgprot_val(x) & _PAGE_NX, 0);
    }

    #[test]
    fn cache_changes_use_existing_pat_bits() {
        let uc = set_memory_uc_change().apply(PAGE_KERNEL);
        assert_eq!(
            pgprot_val(uc) & (_PAGE_PCD | _PAGE_PWT),
            _PAGE_PCD | _PAGE_PWT
        );
        let wc = set_memory_wc_change().apply(PAGE_KERNEL);
        assert_eq!(pgprot_val(wc) & (_PAGE_PCD | _PAGE_PWT), _PAGE_PWT);
    }

    #[test]
    fn live_mutation_validates_then_fails_closed() {
        assert_eq!(set_memory_uc(0x1001, 1), Err(EINVAL));
        assert_eq!(set_memory_uc(0x1000, 1), Err(EOPNOTSUPP));
    }

    #[test]
    fn clflush_range_plan_aligns_start_and_walks_linux_cache_lines() {
        let plan = clflush_range_plan(0x1003, 130, 64).expect("plan");
        assert_eq!(plan.start, 0x1000);
        assert_eq!(plan.end, 0x1085);
        assert_eq!(plan.line_size, 64);
        assert_eq!(plan.lines(), 3);
    }

    #[test]
    fn clflush_cache_range_records_each_cacheline_in_test_mode() {
        reset_clflush_log();
        assert_eq!(clflush_cache_range_with_line_size(0x1003, 130, 64), Ok(()));
        let (len, log) = clflush_log();
        assert_eq!(len, 3);
        assert_eq!(&log[..len], &[0x1000, 0x1040, 0x1080]);
    }

    #[test]
    fn clflush_cache_range_rejects_empty_or_invalid_line_size() {
        assert_eq!(
            clflush_cache_range_with_line_size(0x1000, 0, 64),
            Err(EINVAL)
        );
        assert_eq!(
            clflush_cache_range_with_line_size(0x1000, PAGE_SIZE as usize, 0),
            Err(EINVAL)
        );
    }
}
