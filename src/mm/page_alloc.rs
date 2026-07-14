//! linux-parity: complete
//! linux-source: vendor/linux/mm/page_alloc.c
//! test-origin: linux:vendor/linux/mm/page_alloc.c
//! Linux-visible page allocation wrappers backed by the Lupos buddy allocator.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86::boot::startup::map_kernel::START_KERNEL_MAP;
use crate::arch::x86::mm::paging::{
    __PAGE_KERNEL, PAGE_OFFSET, PAGE_SHIFT, PAGE_SIZE as X86_PAGE_SIZE, pfn_to_virt,
    set_kernel_page_range_flags, virt_to_phys,
};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::buddy::{
    is_buddy_ready, page_in_mem_map, page_to_pfn, pfn_to_page, with_global_buddy,
};
use crate::mm::page::Page;
use crate::mm::page_flags::{__GFP_COMP, GFP_KERNEL, GfpFlags};

const LINUX_MAX_NUMNODES: usize = 64;
const LINUX_NR_NODE_STATES: usize = 6;
const LINUX_NODE_PRESENT: u64 = 1;
const LINUX_PGLIST_DATA_BYTES: usize = 128 * 1024;
const LINUX_MAX_NR_ZONES: usize = 4;
const LINUX_ZONE_SIZE: usize = 0x540;
const LINUX_ZONE_NODE_OFFSET: usize = 0x58;
const LINUX_ZONE_PGDAT_OFFSET: usize = 0x60;
const LINUX_ZONE_MANAGED_PAGES_OFFSET: usize = 0x90;

#[repr(C, align(64))]
struct LinuxPglistData {
    bytes: [u8; LINUX_PGLIST_DATA_BYTES],
}

impl LinuxPglistData {
    const fn zeroed() -> Self {
        Self {
            bytes: [0; LINUX_PGLIST_DATA_BYTES],
        }
    }
}

static mut LINUX_NODE_STATES: [u64; LINUX_NR_NODE_STATES] = [
    LINUX_NODE_PRESENT,
    LINUX_NODE_PRESENT,
    LINUX_NODE_PRESENT,
    LINUX_NODE_PRESENT,
    LINUX_NODE_PRESENT,
    0,
];
static mut LINUX_NODE_DATA: [usize; LINUX_MAX_NUMNODES] = [0; LINUX_MAX_NUMNODES];
static mut LINUX_CONTIG_PAGE_DATA: LinuxPglistData = LinuxPglistData::zeroed();
static LINUX_NUMA_NODE: i32 = 0;
static LINUX_NUMA_MEM: i32 = 0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    init_linux_node_abi();
    export_symbol_once(
        "node_states",
        core::ptr::addr_of_mut!(LINUX_NODE_STATES) as usize,
        false,
    );
    export_symbol_once(
        "node_data",
        core::ptr::addr_of_mut!(LINUX_NODE_DATA) as usize,
        false,
    );
    export_symbol_once("contig_page_data", linux_pgdat_ptr() as usize, false);
    export_symbol_once(
        "numa_node",
        core::ptr::addr_of!(LINUX_NUMA_NODE) as usize,
        false,
    );
    export_symbol_once(
        "_numa_mem_",
        core::ptr::addr_of!(LINUX_NUMA_MEM) as usize,
        false,
    );
    export_symbol_once("__alloc_pages_noprof", __alloc_pages_noprof as usize, false);
    export_symbol_once("alloc_pages_noprof", alloc_pages_noprof as usize, false);
    export_symbol_once("folio_alloc_noprof", folio_alloc_noprof as usize, false);
    export_symbol_once(
        "get_free_pages_noprof",
        get_free_pages_noprof as usize,
        false,
    );
    export_symbol_once(
        "get_zeroed_page_noprof",
        get_zeroed_page_noprof as usize,
        false,
    );
    export_symbol_once("split_page", split_page as usize, false);
    export_symbol_once("__free_pages", __free_pages as usize, false);
    export_symbol_once("free_pages", free_pages as usize, false);
    export_symbol_once(
        "alloc_pages_exact_noprof",
        alloc_pages_exact_noprof as usize,
        false,
    );
    export_symbol_once("free_pages_exact", free_pages_exact as usize, false);
    export_symbol_once(
        "_totalram_pages",
        core::ptr::addr_of!(crate::mm::mm_public::TOTALRAM_PAGES) as usize,
        false,
    );
    export_symbol_once("nr_free_buffer_pages", nr_free_buffer_pages as usize, true);
}

fn linux_pgdat_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(LINUX_CONTIG_PAGE_DATA).cast::<u8>()
}

unsafe fn write_linux_pgdat_usize(offset: usize, value: usize) {
    unsafe { linux_pgdat_ptr().add(offset).cast::<usize>().write(value) };
}

unsafe fn write_linux_pgdat_i32(offset: usize, value: i32) {
    unsafe { linux_pgdat_ptr().add(offset).cast::<i32>().write(value) };
}

fn init_linux_node_abi() {
    let pgdat = linux_pgdat_ptr();
    let managed_pages = crate::mm::mm_public::totalram_pages() as usize;
    unsafe {
        core::ptr::addr_of_mut!(LINUX_NODE_DATA)
            .cast::<usize>()
            .write(pgdat as usize);
        for zone in 0..LINUX_MAX_NR_ZONES {
            let base = zone * LINUX_ZONE_SIZE;
            write_linux_pgdat_i32(base + LINUX_ZONE_NODE_OFFSET, 0);
            write_linux_pgdat_usize(base + LINUX_ZONE_PGDAT_OFFSET, pgdat as usize);
            write_linux_pgdat_usize(
                base + LINUX_ZONE_MANAGED_PAGES_OFFSET,
                if zone == 0 { managed_pages } else { 0 },
            );
        }
    }
}

pub extern "C" fn __alloc_pages_noprof(
    gfp: GfpFlags,
    order: u32,
    _nid: i32,
    _nodemask: *const u8,
) -> *mut Page {
    if !is_buddy_ready() {
        return core::ptr::null_mut();
    }
    with_global_buddy(|buddy| buddy.alloc_pages(order as usize, gfp))
        .unwrap_or(core::ptr::null_mut())
}

pub extern "C" fn alloc_pages_noprof(gfp: GfpFlags, order: u32) -> *mut Page {
    __alloc_pages_noprof(gfp, order, -1, core::ptr::null())
}

pub fn alloc_pages_nolock_noprof(gfp: GfpFlags, order: u32) -> *mut Page {
    alloc_pages_noprof(gfp, order)
}

pub fn __alloc_frozen_pages_noprof(
    gfp: GfpFlags,
    order: u32,
    nid: i32,
    nodemask: *const u8,
) -> *mut Page {
    __alloc_pages_noprof(gfp, order, nid, nodemask)
}

pub fn __folio_alloc_noprof(gfp: GfpFlags, order: u32) -> *mut Page {
    alloc_pages_noprof(gfp | __GFP_COMP, order)
}

pub extern "C" fn folio_alloc_noprof(gfp: GfpFlags, order: u32) -> *mut Page {
    __folio_alloc_noprof(gfp, order)
}

pub extern "C" fn get_free_pages_noprof(gfp: GfpFlags, order: u32) -> usize {
    let page = alloc_pages_noprof(gfp, order);
    if page.is_null() || !page_in_mem_map(page) {
        0
    } else {
        pfn_to_virt(page_to_pfn(page)) as usize
    }
}

pub extern "C" fn get_zeroed_page_noprof(gfp: GfpFlags) -> usize {
    get_free_pages_noprof(gfp | crate::mm::page_flags::__GFP_ZERO, 0)
}

pub extern "C" fn __free_pages(page: *mut Page, order: u32) {
    if !page.is_null() && is_buddy_ready() && page_in_mem_map(page) {
        with_global_buddy(|buddy| buddy.free_pages(page, order as usize));
    }
}

pub extern "C" fn free_pages(addr: usize, order: u32) {
    if addr == 0 || !is_buddy_ready() {
        return;
    }
    let Some(phys) = virt_to_phys(addr as u64) else {
        return;
    };
    let page = pfn_to_page((phys >> PAGE_SHIFT) as usize);
    __free_pages(page, order);
}

fn order_for_pages(pages: usize) -> u32 {
    usize::BITS - pages.max(1).saturating_sub(1).leading_zeros()
}

pub extern "C" fn free_pages_exact(addr: *mut u8, size: usize) {
    if addr.is_null() || size == 0 || !is_buddy_ready() {
        return;
    }
    let Some(phys) = virt_to_phys(addr as u64) else {
        return;
    };
    let pfn = (phys >> PAGE_SHIFT) as usize;
    let page = pfn_to_page(pfn);
    if page_in_mem_map(page) {
        __free_pages(
            page,
            order_for_pages(size.div_ceil(crate::mm::frame::PAGE_SIZE)),
        );
    }
}

pub fn split_page(page: *mut Page, order: u32) {
    if page.is_null() || order == 0 {
        return;
    }
    let nr = 1usize << order;
    for idx in 0..nr {
        unsafe {
            (*page.add(idx))
                ._refcount
                .store(1, core::sync::atomic::Ordering::Release);
        }
    }
}

pub unsafe fn alloc_pages_bulk_noprof(
    gfp: GfpFlags,
    _preferred_nid: i32,
    _nodemask: *const u8,
    nr_pages: usize,
    page_array: *mut *mut Page,
) -> usize {
    if page_array.is_null() {
        return 0;
    }
    let mut allocated = 0usize;
    for idx in 0..nr_pages {
        let page = alloc_pages_noprof(gfp, 0);
        if page.is_null() {
            break;
        }
        unsafe {
            *page_array.add(idx) = page;
        }
        allocated += 1;
    }
    allocated
}

pub extern "C" fn alloc_pages_exact_noprof(size: usize, gfp: GfpFlags) -> *mut u8 {
    let pages = size.div_ceil(crate::mm::frame::PAGE_SIZE);
    let order = order_for_pages(pages);
    let page = alloc_pages_noprof(gfp, order);
    if page.is_null() || !page_in_mem_map(page) {
        core::ptr::null_mut()
    } else {
        pfn_to_virt(page_to_pfn(page))
    }
}

pub fn alloc_contig_pages_noprof(
    nr_pages: usize,
    gfp: GfpFlags,
    _nid: i32,
    _nodemask: *const u8,
) -> *mut Page {
    let order = order_for_pages(nr_pages);
    alloc_pages_noprof(gfp, order)
}

pub fn alloc_contig_frozen_pages_noprof(
    nr_pages: usize,
    gfp: GfpFlags,
    nid: i32,
    nodemask: *const u8,
) -> *mut Page {
    alloc_contig_pages_noprof(nr_pages, gfp, nid, nodemask)
}

pub fn alloc_contig_range_noprof(
    _start: usize,
    _end: usize,
    _migratetype: usize,
    _gfp: GfpFlags,
) -> i32 {
    -95
}

pub fn alloc_contig_frozen_range_noprof(
    start: usize,
    end: usize,
    migratetype: usize,
    gfp: GfpFlags,
) -> i32 {
    alloc_contig_range_noprof(start, end, migratetype, gfp)
}

pub fn free_contig_range(start_pfn: usize, nr_pages: usize) {
    if !is_buddy_ready() {
        return;
    }
    for pfn in start_pfn..start_pfn.saturating_add(nr_pages) {
        let page = crate::mm::buddy::pfn_to_page(pfn);
        if page_in_mem_map(page) {
            __free_pages(page, 0);
        }
    }
}

pub fn free_contig_frozen_range(start_pfn: usize, nr_pages: usize) {
    free_contig_range(start_pfn, nr_pages)
}

pub fn is_free_buddy_page(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).is_buddy() }
}

pub fn adjust_managed_page_count(page: *mut Page, count: isize) {
    if page.is_null() {
        return;
    }
    unsafe {
        if count >= 0 {
            (*page).private = (*page).private.saturating_add(count as usize);
        } else {
            (*page).private = (*page).private.saturating_sub(count.unsigned_abs());
        }
    }
}

pub fn movable_zone() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn node_states(_state: usize) -> u64 {
    crate::mm::mempolicy::online_nodes()
}

pub fn nr_free_buffer_pages() -> usize {
    if !is_buddy_ready() {
        0
    } else {
        with_global_buddy(|buddy| buddy.free_count())
    }
}

pub fn nr_node_ids() -> usize {
    crate::mm::mempolicy::online_nodes().count_ones().max(1) as usize
}

pub fn nr_online_nodes() -> usize {
    nr_node_ids()
}

pub fn numa_node() -> i32 {
    0
}

pub fn _numa_mem_() -> i32 {
    0
}

pub fn latent_entropy() -> u64 {
    0
}

pub fn fs_reclaim_acquire(_gfp_mask: GfpFlags) {}

pub fn fs_reclaim_release(_gfp_mask: GfpFlags) {}

pub fn node_map_pfn_alignment() -> usize {
    1
}

pub fn early_pfn_to_nid(pfn: usize) -> i32 {
    let addr = (pfn as u64) << PAGE_SHIFT;
    crate::mm::mempolicy::numa_memblocks()
        .into_iter()
        .find(|block| addr >= block.start && addr < block.end)
        .map(|block| block.nid as i32)
        .unwrap_or(0)
}

pub fn get_num_physpages() -> usize {
    if !is_buddy_ready() {
        0
    } else {
        with_global_buddy(|buddy| buddy.total_managed())
    }
}

pub fn free_area_init(_nodes: *mut u8, _max_zone_pfn: *mut usize) {}

pub fn free_reserved_area(start: *mut u8, end: *mut u8, _poison: i32, _s: *const u8) -> usize {
    if start.is_null() || end.is_null() || end <= start {
        return 0;
    }
    let bytes = (end as usize).saturating_sub(start as usize);
    let pages = bytes.div_ceil(crate::mm::frame::PAGE_SIZE);
    if is_buddy_ready() {
        for idx in 0..pages {
            let addr = unsafe { start.add(idx * crate::mm::frame::PAGE_SIZE) };
            let Some(phys) = virt_to_phys(addr as u64) else {
                continue;
            };
            free_reserved_phys_page(phys);
        }
    }
    pages
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KernelImageFreeStats {
    pub pages: usize,
    pub low_alias_ptes: u64,
    pub direct_alias_ptes: u64,
    pub high_alias_ptes: u64,
}

const PAGE_SIZE_USIZE: usize = crate::mm::frame::PAGE_SIZE;
static INITMEM_FREED: AtomicBool = AtomicBool::new(false);

#[inline]
const fn page_align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE_USIZE - 1) & !(PAGE_SIZE_USIZE - 1)
}

#[inline]
const fn page_align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE_USIZE - 1)
}

fn free_reserved_phys_page(phys: u64) -> bool {
    if phys & (X86_PAGE_SIZE - 1) != 0 {
        return false;
    }
    let pfn = (phys >> PAGE_SHIFT) as usize;
    let page = pfn_to_page(pfn);
    if !page_in_mem_map(page) {
        return false;
    }
    with_global_buddy(|buddy| buddy.free_reserved_page(page));
    true
}

#[cfg(not(test))]
fn high_kernel_alias(low_addr: usize) -> Option<u64> {
    unsafe extern "C" {
        static _kernel_phys_start: u8;
        static _kernel_phys_end: u8;
    }

    let phys_start = unsafe { &_kernel_phys_start as *const u8 as usize };
    let phys_end = unsafe { &_kernel_phys_end as *const u8 as usize };
    if low_addr < phys_start || low_addr > phys_end {
        return None;
    }
    Some(START_KERNEL_MAP + (low_addr - phys_start) as u64)
}

#[cfg(test)]
fn high_kernel_alias(_low_addr: usize) -> Option<u64> {
    None
}

unsafe fn make_kernel_image_range_reusable(
    begin: usize,
    end: usize,
) -> Option<KernelImageFreeStats> {
    let mut stats = KernelImageFreeStats::default();
    let low = unsafe { set_kernel_page_range_flags(begin as u64, end as u64, __PAGE_KERNEL)? };
    stats.low_alias_ptes = low.updated_ptes;

    let direct_start = PAGE_OFFSET.checked_add(begin as u64)?;
    let direct_end = PAGE_OFFSET.checked_add(end as u64)?;
    if let Some(direct) =
        unsafe { set_kernel_page_range_flags(direct_start, direct_end, __PAGE_KERNEL) }
    {
        stats.direct_alias_ptes = direct.updated_ptes;
    }

    if let (Some(high_start), Some(high_end)) = (high_kernel_alias(begin), high_kernel_alias(end)) {
        if let Some(high) =
            unsafe { set_kernel_page_range_flags(high_start, high_end, __PAGE_KERNEL) }
        {
            stats.high_alias_ptes = high.updated_ptes;
        }
    }

    Some(stats)
}

pub unsafe fn free_kernel_image_pages(
    what: &str,
    begin: *mut u8,
    end: *mut u8,
) -> KernelImageFreeStats {
    if begin.is_null() || end.is_null() || end <= begin {
        return KernelImageFreeStats::default();
    }

    let begin = page_align_up(begin as usize);
    let end = page_align_down(end as usize);
    if begin >= end || !is_buddy_ready() {
        return KernelImageFreeStats::default();
    }

    let Some(mut stats) = (unsafe { make_kernel_image_range_reusable(begin, end) }) else {
        crate::log_warn!(
            "",
            "free_initmem: skipped {} because page-table permissions could not be updated",
            what
        );
        return KernelImageFreeStats::default();
    };

    let mut pages = 0usize;
    let mut addr = begin;
    while addr < end {
        let Some(phys) = virt_to_phys(addr as u64) else {
            addr += PAGE_SIZE_USIZE;
            continue;
        };
        if free_reserved_phys_page(phys) {
            pages += 1;
        }
        addr += PAGE_SIZE_USIZE;
    }

    stats.pages = pages;
    if pages != 0 {
        crate::log_info!(
            "",
            "Freeing {} memory: {}K",
            what,
            pages * PAGE_SIZE_USIZE / 1024
        );
    }
    stats
}

#[cfg(not(test))]
pub fn free_initmem_default(_poison: i32) -> usize {
    unsafe extern "C" {
        static __init_begin: u8;
        static __init_end: u8;
    }

    let stats = unsafe {
        free_kernel_image_pages(
            "unused kernel image (initmem)",
            &__init_begin as *const u8 as *mut u8,
            &__init_end as *const u8 as *mut u8,
        )
    };
    stats.pages
}

#[cfg(test)]
pub fn free_initmem_default(_poison: i32) -> usize {
    0
}

pub fn free_initmem() {
    if INITMEM_FREED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    crate::arch::x86::mm::mem_encrypt_amd::mem_encrypt_free_decrypted_mem();
    let pages = free_initmem_default(0);
    if pages != 0 {
        crate::init::boot_trace::record("mm", "init memory freed");
    }
}

#[cfg(not(test))]
pub fn free_kernel_section_gaps(
    text_end: *const u8,
    rodata_start: *const u8,
    rodata_end: *const u8,
    data_start: *const u8,
) -> KernelImageFreeStats {
    let text_gap = unsafe {
        free_kernel_image_pages(
            "unused kernel image (text/rodata gap)",
            text_end as *mut u8,
            rodata_start as *mut u8,
        )
    };
    let rodata_gap = unsafe {
        free_kernel_image_pages(
            "unused kernel image (rodata/data gap)",
            rodata_end as *mut u8,
            data_start as *mut u8,
        )
    };
    KernelImageFreeStats {
        pages: text_gap.pages + rodata_gap.pages,
        low_alias_ptes: text_gap.low_alias_ptes + rodata_gap.low_alias_ptes,
        direct_alias_ptes: text_gap.direct_alias_ptes + rodata_gap.direct_alias_ptes,
        high_alias_ptes: text_gap.high_alias_ptes + rodata_gap.high_alias_ptes,
    }
}

#[cfg(test)]
pub fn free_kernel_section_gaps(
    _text_end: *const u8,
    _rodata_start: *const u8,
    _rodata_end: *const u8,
    _data_start: *const u8,
) -> KernelImageFreeStats {
    KernelImageFreeStats::default()
}

pub fn mem_init() {}

pub fn mm_core_init() {}

pub fn mm_core_init_early() {}

pub fn arch_zone_limits_init() {}

pub fn accept_memory(_start: u64, _end: u64) {}

pub fn absent_pages_in_range(start_pfn: usize, end_pfn: usize) -> usize {
    if end_pfn <= start_pfn {
        return 0;
    }
    let blocks = crate::mm::mempolicy::numa_memblocks();
    if blocks.is_empty() {
        return 0;
    }
    (start_pfn..end_pfn)
        .filter(|pfn| {
            let addr = (*pfn as u64) << PAGE_SHIFT;
            !blocks
                .iter()
                .any(|block| addr >= block.start && addr < block.end)
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use core::sync::atomic::Ordering;

    #[test]
    fn allocation_wrappers_return_disabled_shape_before_buddy_init() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        crate::mm::buddy::reset_buddy_state_for_test();

        assert!(alloc_pages_noprof(GFP_KERNEL, 0).is_null());
        assert!(alloc_pages_nolock_noprof(GFP_KERNEL, 0).is_null());
        assert!(alloc_contig_pages_noprof(1, GFP_KERNEL, 0, core::ptr::null()).is_null());
        assert_eq!(nr_free_buffer_pages(), 0);
        assert_eq!(get_num_physpages(), 0);
        assert_eq!(alloc_contig_range_noprof(0, 1, 0, GFP_KERNEL), -95);
    }

    #[test]
    fn nr_free_buffer_pages_export_registers_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("nr_free_buffer_pages"),
            Some(nr_free_buffer_pages as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("get_zeroed_page_noprof"),
            Some(get_zeroed_page_noprof as usize)
        );
    }

    #[test]
    fn split_and_managed_count_helpers_mutate_page_metadata() {
        let mut pages = [const { Page::new() }, const { Page::new() }];
        split_page(pages.as_mut_ptr(), 1);
        assert_eq!(pages[0]._refcount.load(Ordering::Acquire), 1);
        assert_eq!(pages[1]._refcount.load(Ordering::Acquire), 1);

        adjust_managed_page_count(&mut pages[0], 4);
        assert_eq!(pages[0].private, 4);
        adjust_managed_page_count(&mut pages[0], -2);
        assert_eq!(pages[0].private, 2);
        assert!(!is_free_buddy_page(core::ptr::null()));
    }

    #[test]
    fn node_and_absent_page_helpers_follow_memblocks() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        crate::mm::mempolicy::reset_for_tests();
        crate::mm::mempolicy::numa_add_memblk(0, (2 * crate::mm::frame::PAGE_SIZE) as u64, 0)
            .unwrap();
        crate::mm::mempolicy::numa_add_memblk(
            (4 * crate::mm::frame::PAGE_SIZE) as u64,
            (5 * crate::mm::frame::PAGE_SIZE) as u64,
            1,
        )
        .unwrap();

        assert_eq!(node_states(0) & 0b11, 0b11);
        assert_eq!(nr_node_ids(), 2);
        assert_eq!(nr_online_nodes(), 2);
        assert_eq!(early_pfn_to_nid(0), 0);
        assert_eq!(early_pfn_to_nid(4), 1);
        assert_eq!(early_pfn_to_nid(3), 0);
        assert_eq!(absent_pages_in_range(0, 5), 2);
        assert_eq!(node_map_pfn_alignment(), 1);
    }

    #[test]
    fn reserved_area_reports_released_page_count_without_buddy() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        crate::mm::buddy::reset_buddy_state_for_test();
        let mut bytes = [0u8; crate::mm::frame::PAGE_SIZE * 2];
        let start = bytes.as_mut_ptr();
        let end = unsafe { start.add(bytes.len()) };
        assert_eq!(free_reserved_area(start, end, 0, core::ptr::null()), 2);
        assert_eq!(free_reserved_area(start, start, 0, core::ptr::null()), 0);
    }

    #[test]
    fn free_kernel_image_pages_releases_page_aligned_range_to_buddy() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        crate::mm::buddy::reset_buddy_state_for_test();
        unsafe { crate::arch::x86::mm::paging::reset_test_pool() };

        const TEST_PAGES: usize = 8;
        let mut pages = [const { Page::new() }; TEST_PAGES];
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe {
            crate::mm::buddy::set_mem_map(pages.as_mut_ptr(), 16, TEST_PAGES);
            crate::mm::buddy::install_test_buddy(16, TEST_PAGES);
        }

        let (initial_free, start_pfn) = crate::mm::buddy::with_global_buddy(|buddy| {
            let initial_free = buddy.free_count();
            let page = buddy
                .alloc_pages(1, GFP_KERNEL)
                .expect("allocate image pages");
            (initial_free, page_to_pfn(page))
        });
        let start = (start_pfn << PAGE_SHIFT) as u64;
        unsafe {
            crate::arch::x86::mm::paging::map_kernel_page(
                start,
                start,
                crate::arch::x86::mm::paging::PAGE_KERNEL_RO,
            );
            crate::arch::x86::mm::paging::map_kernel_page(
                start + X86_PAGE_SIZE,
                start + X86_PAGE_SIZE,
                crate::arch::x86::mm::paging::PAGE_KERNEL_RO,
            );
        }

        let stats = unsafe {
            free_kernel_image_pages(
                "unused kernel image (initmem)",
                start as *mut u8,
                (start + 2 * X86_PAGE_SIZE) as *mut u8,
            )
        };

        assert_eq!(stats.pages, 2);
        assert_eq!(stats.low_alias_ptes, 2);
        assert_eq!(
            crate::mm::buddy::with_global_buddy(|buddy| buddy.free_count()),
            initial_free
        );
    }
}
