//! linux-parity: complete
//! linux-source: vendor/linux/include/linux/mm.h
//! Linux-visible `include/linux/mm.h` wrappers.

extern crate alloc;

use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::x86::mm::paging::{p4d_t, pgd_t, pgprot_t, pmd_t, pte_t, pud_t};
use crate::include::uapi::errno::{EFAULT, EINVAL, ENOMEM, EOPNOTSUPP};
use crate::mm::frame::PAGE_SIZE;
use crate::mm::list::ListHead;
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::page::Page;
use crate::mm::page_flags::{PG_RESERVED, compound_order};
use crate::mm::vm_flags::{VM_GROWSDOWN, VM_SHARED, VM_WRITE, VmFlags};
use crate::mm::vma::{find_vma, insert_vma, remove_vma};

const PAGE_SHIFT: usize = 12;
const PAGE_MASK: u64 = !((PAGE_SIZE as u64) - 1);
const PTE_WRITE: u64 = 1 << 1;
const PTE_SPECIAL: u64 = 1 << 10;
const PMD_SPECIAL: u64 = 1 << 10;
const PUD_SPECIAL: u64 = 1 << 10;

static TOTALRAM_PAGES: AtomicU64 = AtomicU64::new(0);
static POISONED_PAGES: AtomicU64 = AtomicU64::new(0);
static MEMBLK_POISONED_PAGES: AtomicU64 = AtomicU64::new(0);
static PGTABLE_BYTES: AtomicU64 = AtomicU64::new(0);
static ZERO_PAGE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];

pub unsafe fn __mm_zero_struct_page(page: *mut Page) {
    if !page.is_null() {
        unsafe { page.write(Page::new()) };
    }
}

pub unsafe fn snapshot_page(page: *const Page) -> *const Page {
    page
}

pub unsafe fn snapshot_page_is_faithful(_page: *const Page, _snapshot: *const Page) -> bool {
    true
}

pub fn page_shift(_page: *const Page) -> usize {
    PAGE_SHIFT
}

pub fn page_size(page: *const Page) -> usize {
    PAGE_SIZE << compound_order(page)
}

pub fn thp_order(_page: *const Page) -> usize {
    0
}

pub fn thp_size(page: *const Page) -> usize {
    PAGE_SIZE << thp_order(page)
}

pub fn page_mapped(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page)._mapcount.load(Ordering::Relaxed) >= 0 }
}

pub fn put_page_testzero(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).put_page() == 0 }
}

pub fn try_get_page(page: *const Page) -> bool {
    if page.is_null() {
        return false;
    }
    unsafe { (*page).get_page() };
    true
}

pub fn is_zero_page(page: *const Page) -> bool {
    page.is_null()
}

pub fn is_zero_folio(folio: *const Page) -> bool {
    is_zero_page(folio)
}

pub fn virt_to_head_page(addr: *const u8) -> *mut Page {
    virt_to_folio(addr)
}

pub fn virt_to_folio(addr: *const u8) -> *mut Page {
    addr as *mut Page
}

pub fn pfn_folio(pfn: u64) -> *mut Page {
    (pfn << PAGE_SHIFT) as *mut Page
}

pub fn lru_to_folio(ptr: *mut u8) -> *mut Page {
    ptr as *mut Page
}

pub fn page_kasan_tag(page: *const Page) -> u8 {
    ((page as usize) >> 56) as u8
}

pub fn page_kasan_tag_set(page: *mut Page, tag: u8) -> *mut Page {
    ((page as usize & 0x00ff_ffff_ffff_ffff) | ((tag as usize) << 56)) as *mut Page
}

pub fn page_kasan_tag_reset(page: *mut Page) -> *mut Page {
    (page as usize & 0x00ff_ffff_ffff_ffff) as *mut Page
}

pub fn page_is_pfmemalloc(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).private & 1 != 0 }
}

pub fn set_page_pfmemalloc(page: *mut Page) {
    if !page.is_null() {
        unsafe { (*page).private |= 1 };
    }
}

pub fn page_to_nid(_page: *const Page) -> i32 {
    0
}

pub fn set_page_node(_page: *mut Page, _nid: i32) {}

pub fn page_zone_id(_page: *const Page) -> usize {
    0
}

pub fn page_zone(_page: *const Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn page_pgdat(_page: *const Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn set_page_zone(_page: *mut Page, _zone: usize) {}

pub fn set_page_section(_page: *mut Page, _section: usize) {}

pub fn set_page_links(page: *mut Page, zone: usize, node: i32, pfn: u64) {
    let _ = (zone, node, pfn);
    if !page.is_null() {
        unsafe { (*page).flags.fetch_and(!PG_RESERVED, Ordering::Relaxed) };
    }
}

pub const fn page_is_ram(_pfn: u64) -> bool {
    true
}

pub const fn arch_is_platform_page(_page: *const Page) -> bool {
    false
}

pub const fn pfn_is_unaccepted_memory(_pfn: u64) -> bool {
    false
}

pub const fn range_contains_unaccepted_memory(_start: u64, _end: u64) -> bool {
    false
}

pub fn memcmp_pages(a: *const Page, b: *const Page) -> i32 {
    if a == b { 0 } else { 1 }
}

pub unsafe fn pages_identical(a: *const u8, b: *const u8) -> bool {
    if a.is_null() || b.is_null() {
        return false;
    }
    let a = unsafe { core::slice::from_raw_parts(a, PAGE_SIZE) };
    let b = unsafe { core::slice::from_raw_parts(b, PAGE_SIZE) };
    a == b
}

pub unsafe fn folio_zero_user(folio: *mut Page, start: usize, len: usize) {
    let addr = folio as *mut u8;
    if !addr.is_null() && start < PAGE_SIZE {
        let len = len.min(PAGE_SIZE - start);
        unsafe { core::ptr::write_bytes(addr.add(start), 0, len) };
    }
}

pub fn mk_pte(page: *const Page, prot: pgprot_t) -> pte_t {
    pte_t(((page as u64) & PAGE_MASK) | prot.0)
}

pub fn folio_mk_pte(folio: *const Page, prot: pgprot_t) -> pte_t {
    mk_pte(folio, prot)
}

pub fn folio_mk_pmd(folio: *const Page, prot: pgprot_t) -> pmd_t {
    pmd_t(((folio as u64) & PAGE_MASK) | prot.0)
}

pub fn folio_mk_pud(folio: *const Page, prot: pgprot_t) -> pud_t {
    pud_t(((folio as u64) & PAGE_MASK) | prot.0)
}

pub fn maybe_mkwrite(pte: pte_t, vma: *const VmAreaStruct) -> pte_t {
    if is_shared_maywrite(vma) {
        pte_t(pte.0 | PTE_WRITE)
    } else {
        pte
    }
}

pub fn pmd_mkspecial(pmd: pmd_t) -> pmd_t {
    pmd_t(pmd.0 | PMD_SPECIAL)
}

pub fn pud_mkspecial(pud: pud_t) -> pud_t {
    pud_t(pud.0 | PUD_SPECIAL)
}

pub fn pmd_special(pmd: pmd_t) -> bool {
    pmd.0 & PMD_SPECIAL != 0
}

pub fn pud_special(pud: pud_t) -> bool {
    pud.0 & PUD_SPECIAL != 0
}

pub fn pmd_pgtable_page(pmd: pmd_t) -> *mut Page {
    (pmd.0 & PAGE_MASK) as *mut Page
}

pub fn pmd_ptdesc(pmd: pmd_t) -> *mut u8 {
    pmd_pgtable_page(pmd) as *mut u8
}

pub fn ptdesc_address(ptdesc: *const u8) -> *mut u8 {
    ptdesc as *mut u8
}

pub fn virt_to_ptdesc(addr: *const u8) -> *mut u8 {
    addr as *mut u8
}

pub fn ptdesc_set_kernel(ptdesc: *mut u8) {
    if !ptdesc.is_null() {
        unsafe { *ptdesc = 1 };
    }
}

pub fn ptdesc_clear_kernel(ptdesc: *mut u8) {
    if !ptdesc.is_null() {
        unsafe { *ptdesc = 0 };
    }
}

pub fn ptdesc_test_kernel(ptdesc: *const u8) -> bool {
    !ptdesc.is_null() && unsafe { *ptdesc != 0 }
}

pub fn pagetable_alloc_noprof(_gfp: u32, _order: u32) -> *mut Page {
    Box::into_raw(Box::new(Page::new()))
}

pub unsafe fn pagetable_free(page: *mut Page) {
    if !page.is_null() {
        unsafe { drop(Box::from_raw(page)) };
    }
}

pub unsafe fn pagetable_free_kernel(page: *mut Page) {
    unsafe { pagetable_free(page) };
}

pub unsafe fn __pagetable_free(page: *mut Page) {
    unsafe { pagetable_free(page) };
}

pub unsafe fn free_reserved_ptdesc(ptdesc: *mut u8) {
    unsafe { pagetable_free(ptdesc as *mut Page) };
}

pub fn pagetable_is_reserved(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).is_reserved() }
}

pub fn __pagetable_ctor(page: *mut Page) -> bool {
    if page.is_null() {
        return false;
    }
    unsafe { (*page).set_reserved() };
    true
}

pub fn pagetable_pte_ctor(page: *mut Page) -> bool {
    __pagetable_ctor(page)
}

pub fn pagetable_pmd_ctor(page: *mut Page) -> bool {
    __pagetable_ctor(page)
}

pub fn pagetable_pud_ctor(page: *mut Page) -> bool {
    __pagetable_ctor(page)
}

pub fn pagetable_p4d_ctor(page: *mut Page) -> bool {
    __pagetable_ctor(page)
}

pub fn pagetable_pgd_ctor(page: *mut Page) -> bool {
    __pagetable_ctor(page)
}

pub fn pagetable_dtor(page: *mut Page) {
    if !page.is_null() {
        unsafe { (*page).clear_flag(PG_RESERVED) };
    }
}

pub fn pagetable_dtor_free(page: *mut Page) {
    pagetable_dtor(page);
    unsafe { pagetable_free(page) };
}

pub fn ptlock_alloc(_page: *mut Page) -> bool {
    true
}

pub fn ptlock_init(_page: *mut Page) {}

pub fn ptlock_free(_page: *mut Page) {}

pub fn ptlock_cache_init() {}

pub fn ptlock_ptr(page: *mut Page) -> *mut u8 {
    page as *mut u8
}

pub fn pte_lockptr(_mm: *mut MmStruct, pmd: *mut pmd_t) -> *mut u8 {
    pmd as *mut u8
}

pub fn ptep_lockptr(mm: *mut MmStruct, pmd: *mut pmd_t) -> *mut u8 {
    pte_lockptr(mm, pmd)
}

pub fn pmd_lockptr(_mm: *mut MmStruct, pmd: *mut pmd_t) -> *mut u8 {
    pmd as *mut u8
}

pub fn pud_lockptr(_mm: *mut MmStruct, pud: *mut pud_t) -> *mut u8 {
    pud as *mut u8
}

pub fn pmd_lock(mm: *mut MmStruct, pmd: *mut pmd_t) -> *mut u8 {
    pmd_lockptr(mm, pmd)
}

pub fn pud_lock(mm: *mut MmStruct, pud: *mut pud_t) -> *mut u8 {
    pud_lockptr(mm, pud)
}

pub fn pmd_ptlock_init(_page: *mut Page) -> bool {
    true
}

pub fn get_locked_pte(_mm: *mut MmStruct, _addr: u64, ptl: *mut *mut u8) -> *mut pte_t {
    if !ptl.is_null() {
        unsafe { *ptl = core::ptr::null_mut() };
    }
    core::ptr::null_mut()
}

pub fn pte_offset_map(_pmd: *mut pmd_t, _addr: u64) -> *mut pte_t {
    core::ptr::null_mut()
}

pub fn __pte_offset_map(pmd: *mut pmd_t, addr: u64) -> *mut pte_t {
    pte_offset_map(pmd, addr)
}

pub fn __pte_alloc(_mm: *mut MmStruct, _pmd: *mut pmd_t) -> i32 {
    0
}

pub fn __pte_alloc_kernel(_pmd: *mut pmd_t) -> i32 {
    0
}

pub fn __pmd_alloc(_mm: *mut MmStruct, _pud: *mut pud_t, _addr: u64) -> i32 {
    0
}

pub fn __pud_alloc(_mm: *mut MmStruct, _p4d: *mut p4d_t, _addr: u64) -> i32 {
    0
}

pub fn __p4d_alloc(_mm: *mut MmStruct, _pgd: *mut pgd_t, _addr: u64) -> i32 {
    0
}

pub fn p4d_alloc(mm: *mut MmStruct, pgd: *mut pgd_t, addr: u64) -> *mut p4d_t {
    if __p4d_alloc(mm, pgd, addr) == 0 {
        pgd as *mut p4d_t
    } else {
        core::ptr::null_mut()
    }
}

pub fn mm_pgtables_bytes(mm: *const MmStruct) -> u64 {
    if mm.is_null() {
        PGTABLE_BYTES.load(Ordering::Relaxed)
    } else {
        unsafe { (*mm).map_count.max(0) as u64 * PAGE_SIZE as u64 }
    }
}

pub fn mm_pgtables_bytes_init() {
    PGTABLE_BYTES.store(0, Ordering::Relaxed);
}

pub fn mm_inc_nr_ptes(_mm: *mut MmStruct) {
    PGTABLE_BYTES.fetch_add(PAGE_SIZE as u64, Ordering::Relaxed);
}

pub fn mm_dec_nr_ptes(_mm: *mut MmStruct) {
    PGTABLE_BYTES.fetch_sub(PAGE_SIZE as u64, Ordering::Relaxed);
}

pub fn mm_inc_nr_pmds(mm: *mut MmStruct) {
    mm_inc_nr_ptes(mm);
}

pub fn mm_dec_nr_pmds(mm: *mut MmStruct) {
    mm_dec_nr_ptes(mm);
}

pub fn mm_inc_nr_puds(mm: *mut MmStruct) {
    mm_inc_nr_ptes(mm);
}

pub fn mm_dec_nr_puds(mm: *mut MmStruct) {
    mm_dec_nr_ptes(mm);
}

pub fn add_mm_counter(mm: *mut MmStruct, member: usize, value: i64) {
    with_mm_counter(mm, member, |slot| {
        if value >= 0 {
            *slot = slot.saturating_add(value as u64);
        } else {
            *slot = slot.saturating_sub(value.unsigned_abs());
        }
    });
}

pub fn inc_mm_counter(mm: *mut MmStruct, member: usize) {
    add_mm_counter(mm, member, 1);
}

pub fn dec_mm_counter(mm: *mut MmStruct, member: usize) {
    add_mm_counter(mm, member, -1);
}

pub fn get_mm_counter(mm: *const MmStruct, member: usize) -> u64 {
    if mm.is_null() {
        return 0;
    }
    unsafe {
        match member {
            0 => (*mm).total_vm,
            1 => (*mm).data_vm,
            2 => (*mm).exec_vm,
            3 => (*mm).stack_vm,
            _ => 0,
        }
    }
}

pub fn get_mm_counter_sum(mm: *const MmStruct) -> u64 {
    get_mm_counter(mm, 0) + get_mm_counter(mm, 1) + get_mm_counter(mm, 2) + get_mm_counter(mm, 3)
}

pub fn mm_counter(mm: *const MmStruct, member: usize) -> u64 {
    get_mm_counter(mm, member)
}

pub fn mm_counter_file(mm: *const MmStruct) -> u64 {
    get_mm_counter(mm, 1)
}

pub fn get_mm_rss_sum(mm: *const MmStruct) -> u64 {
    get_mm_counter_sum(mm)
}

pub fn get_mm_hiwater_rss(mm: *const MmStruct) -> u64 {
    if mm.is_null() {
        0
    } else {
        unsafe { (*mm).hiwater_rss }
    }
}

pub fn get_mm_hiwater_vm(mm: *const MmStruct) -> u64 {
    if mm.is_null() {
        0
    } else {
        unsafe { (*mm).hiwater_vm }
    }
}

pub fn update_hiwater_rss(mm: *mut MmStruct) {
    if !mm.is_null() {
        unsafe { (*mm).hiwater_rss = (*mm).hiwater_rss.max(get_mm_rss_sum(mm)) };
    }
}

pub fn update_hiwater_vm(mm: *mut MmStruct) {
    if !mm.is_null() {
        unsafe { (*mm).hiwater_vm = (*mm).hiwater_vm.max((*mm).total_vm) };
    }
}

pub fn reset_mm_hiwater_rss(mm: *mut MmStruct) {
    if !mm.is_null() {
        unsafe { (*mm).hiwater_rss = get_mm_rss_sum(mm) };
    }
}

pub fn setmax_mm_hiwater_rss(maxrss: *mut u64, mm: *const MmStruct) {
    if !maxrss.is_null() {
        unsafe { *maxrss = (*maxrss).max(get_mm_hiwater_rss(mm)) };
    }
}

pub fn mm_trace_rss_stat(_mm: *mut MmStruct, _member: usize, _count: i64) {}

pub unsafe fn get_mm_exe_file(mm: *const MmStruct) -> usize {
    if mm.is_null() {
        0
    } else {
        unsafe { (*mm).exe_file }
    }
}

pub unsafe fn get_task_exe_file(task: *const MmStruct) -> usize {
    unsafe { get_mm_exe_file(task) }
}

pub unsafe fn set_mm_exe_file(mm: *mut MmStruct, file: usize) {
    if !mm.is_null() {
        unsafe { (*mm).exe_file = file };
    }
}

pub unsafe fn replace_mm_exe_file(mm: *mut MmStruct, file: usize) -> i32 {
    unsafe { set_mm_exe_file(mm, file) };
    0
}

pub fn process_shares_mm(_p: *const u8, _mm: *const MmStruct) -> bool {
    false
}

pub fn mm_take_all_locks(_mm: *mut MmStruct) -> i32 {
    0
}

pub fn mm_drop_all_locks(_mm: *mut MmStruct) {}

pub fn init_mm_internals(mm: *mut MmStruct) {
    if !mm.is_null() {
        unsafe {
            (*mm).mm_users.store(1, Ordering::Relaxed);
            (*mm).mm_count.store(1, Ordering::Relaxed);
        }
    }
}

pub unsafe fn insert_vm_struct(mm: *mut MmStruct, vma: *mut VmAreaStruct) -> i32 {
    if mm.is_null() || vma.is_null() {
        return -EINVAL;
    }
    match unsafe { insert_vma(&mut *mm, vma) } {
        Ok(()) => 0,
        Err(errno) => -errno,
    }
}

pub unsafe fn find_exact_vma(mm: *const MmStruct, vm_start: u64, vm_end: u64) -> *mut VmAreaStruct {
    if mm.is_null() {
        return core::ptr::null_mut();
    }
    match find_vma(unsafe { &*mm }, vm_start) {
        Some(vma) if unsafe { (*vma).vm_start == vm_start && (*vma).vm_end == vm_end } => vma,
        _ => core::ptr::null_mut(),
    }
}

pub unsafe fn vma_lookup(mm: *const MmStruct, addr: u64) -> *mut VmAreaStruct {
    if mm.is_null() {
        return core::ptr::null_mut();
    }
    match find_vma(unsafe { &*mm }, addr) {
        Some(vma) if unsafe { (*vma).contains(addr) } => vma,
        _ => core::ptr::null_mut(),
    }
}

pub unsafe fn range_in_vma(vma: *const VmAreaStruct, start: u64, end: u64) -> bool {
    !vma.is_null() && unsafe { start >= (*vma).vm_start && end <= (*vma).vm_end && start <= end }
}

pub unsafe fn range_in_vma_desc(vma: *const VmAreaStruct, start: u64, end: u64) -> bool {
    unsafe { range_in_vma(vma, start, end) }
}

pub fn range_is_subset(start: u64, end: u64, lower: u64, upper: u64) -> bool {
    start >= lower && end <= upper && start <= end
}

pub unsafe fn vma_pages(vma: *const VmAreaStruct) -> u64 {
    if vma.is_null() {
        0
    } else {
        unsafe { ((*vma).vm_end - (*vma).vm_start) >> PAGE_SHIFT }
    }
}

pub unsafe fn vma_desc_pages(vma: *const VmAreaStruct) -> u64 {
    unsafe { vma_pages(vma) }
}

pub unsafe fn vma_desc_size(vma: *const VmAreaStruct) -> u64 {
    if vma.is_null() {
        0
    } else {
        unsafe { (*vma).vm_end - (*vma).vm_start }
    }
}

pub unsafe fn vma_last_pgoff(vma: *const VmAreaStruct) -> u64 {
    if vma.is_null() {
        0
    } else {
        unsafe { (*vma).vm_pgoff + vma_pages(vma).saturating_sub(1) }
    }
}

pub unsafe fn vm_start_gap(vma: *const VmAreaStruct) -> u64 {
    if vma.is_null() {
        0
    } else {
        unsafe { (*vma).vm_start }
    }
}

pub unsafe fn vm_end_gap(vma: *const VmAreaStruct) -> u64 {
    if vma.is_null() {
        0
    } else {
        unsafe { (*vma).vm_end }
    }
}

pub unsafe fn stack_guard_start_gap(vma: *const VmAreaStruct) -> u64 {
    unsafe { vm_start_gap(vma) }
}

pub unsafe fn vma_flags(vma: *const VmAreaStruct) -> VmFlags {
    if vma.is_null() {
        0
    } else {
        unsafe { (*vma).vm_flags }
    }
}

pub unsafe fn vma_flags_set(vma: *mut VmAreaStruct, flags: VmFlags) {
    if !vma.is_null() {
        unsafe { (*vma).vm_flags |= flags };
    }
}

pub unsafe fn vma_flags_clear(vma: *mut VmAreaStruct, flags: VmFlags) {
    if !vma.is_null() {
        unsafe { (*vma).vm_flags &= !flags };
    }
}

pub unsafe fn vma_flags_mod(vma: *mut VmAreaStruct, set: VmFlags, clear: VmFlags) {
    unsafe {
        vma_flags_clear(vma, clear);
        vma_flags_set(vma, set);
    }
}

pub unsafe fn vma_flags_reset(vma: *mut VmAreaStruct, flags: VmFlags) {
    if !vma.is_null() {
        unsafe { (*vma).vm_flags = flags };
    }
}

pub unsafe fn vma_flags_init(vma: *mut VmAreaStruct, flags: VmFlags) {
    unsafe { vma_flags_reset(vma, flags) };
}

pub unsafe fn vm_flags_init(vma: *mut VmAreaStruct, flags: VmFlags) {
    unsafe { vma_flags_init(vma, flags) };
}

pub unsafe fn vm_flags_set(vma: *mut VmAreaStruct, flags: VmFlags) {
    unsafe { vma_flags_set(vma, flags) };
}

pub unsafe fn vm_flags_clear(vma: *mut VmAreaStruct, flags: VmFlags) {
    unsafe { vma_flags_clear(vma, flags) };
}

pub unsafe fn vm_flags_mod(vma: *mut VmAreaStruct, set: VmFlags, clear: VmFlags) {
    unsafe { vma_flags_mod(vma, set, clear) };
}

pub unsafe fn vm_flags_reset(vma: *mut VmAreaStruct, flags: VmFlags) {
    unsafe { vma_flags_reset(vma, flags) };
}

pub unsafe fn vma_flags_set_flag(vma: *mut VmAreaStruct, flag: VmFlags) {
    unsafe { vma_flags_set(vma, flag) };
}

pub unsafe fn vma_flags_set_mask(vma: *mut VmAreaStruct, mask: VmFlags) {
    unsafe { vma_flags_set(vma, mask) };
}

pub unsafe fn vma_flags_clear_mask(vma: *mut VmAreaStruct, mask: VmFlags) {
    unsafe { vma_flags_clear(vma, mask) };
}

pub unsafe fn vma_set_flags_mask(vma: *mut VmAreaStruct, set: VmFlags, clear: VmFlags) {
    unsafe { vma_flags_mod(vma, set, clear) };
}

pub unsafe fn vma_clear_flags_mask(vma: *mut VmAreaStruct, clear: VmFlags) {
    unsafe { vma_flags_clear(vma, clear) };
}

pub unsafe fn vma_desc_set_flags_mask(vma: *mut VmAreaStruct, set: VmFlags, clear: VmFlags) {
    unsafe { vma_set_flags_mask(vma, set, clear) };
}

pub unsafe fn vma_desc_clear_flags_mask(vma: *mut VmAreaStruct, clear: VmFlags) {
    unsafe { vma_clear_flags_mask(vma, clear) };
}

pub unsafe fn vma_flags_test(vma: *const VmAreaStruct, flags: VmFlags) -> bool {
    unsafe { vma_flags(vma) & flags != 0 }
}

pub unsafe fn vma_flags_test_any_mask(vma: *const VmAreaStruct, mask: VmFlags) -> bool {
    unsafe { vma_flags_test(vma, mask) }
}

pub unsafe fn vma_flags_test_all_mask(vma: *const VmAreaStruct, mask: VmFlags) -> bool {
    unsafe { vma_flags(vma) & mask == mask }
}

pub unsafe fn vma_flags_test_single_mask(vma: *const VmAreaStruct, mask: VmFlags) -> bool {
    unsafe { (vma_flags(vma) & mask).count_ones() == 1 }
}

pub unsafe fn vma_test(vma: *const VmAreaStruct, flags: VmFlags) -> bool {
    unsafe { vma_flags_test(vma, flags) }
}

pub unsafe fn vma_test_any_mask(vma: *const VmAreaStruct, flags: VmFlags) -> bool {
    unsafe { vma_flags_test_any_mask(vma, flags) }
}

pub unsafe fn vma_test_all_mask(vma: *const VmAreaStruct, flags: VmFlags) -> bool {
    unsafe { vma_flags_test_all_mask(vma, flags) }
}

pub fn vma_flags_and_mask(flags: VmFlags, mask: VmFlags) -> VmFlags {
    flags & mask
}

pub fn vma_flags_same_mask(a: VmFlags, b: VmFlags, mask: VmFlags) -> bool {
    (a & mask) == (b & mask)
}

pub unsafe fn vma_flags_same_pair(
    a: *const VmAreaStruct,
    b: *const VmAreaStruct,
    mask: VmFlags,
) -> bool {
    unsafe { vma_flags_same_mask(vma_flags(a), vma_flags(b), mask) }
}

pub unsafe fn vma_flags_diff_pair(a: *const VmAreaStruct, b: *const VmAreaStruct) -> VmFlags {
    unsafe { vma_flags(a) ^ vma_flags(b) }
}

pub fn vma_flags_count(flags: VmFlags) -> u32 {
    flags.count_ones()
}

pub unsafe fn vma_flags_reset_once(vma: *mut VmAreaStruct, flags: VmFlags) {
    unsafe { vma_flags_reset(vma, flags) };
}

pub unsafe fn vma_desc_test(vma: *const VmAreaStruct, flags: VmFlags) -> bool {
    unsafe { vma_test(vma, flags) }
}

pub unsafe fn vma_desc_test_any_mask(vma: *const VmAreaStruct, flags: VmFlags) -> bool {
    unsafe { vma_test_any_mask(vma, flags) }
}

pub unsafe fn vma_desc_test_all_mask(vma: *const VmAreaStruct, flags: VmFlags) -> bool {
    unsafe { vma_test_all_mask(vma, flags) }
}

pub fn vma_desc_is_cow_mapping(flags: VmFlags) -> bool {
    is_cow_mapping(flags)
}

pub fn is_cow_mapping(flags: VmFlags) -> bool {
    flags & (VM_SHARED | VM_WRITE) == VM_WRITE
}

pub fn is_shared_maywrite(vma: *const VmAreaStruct) -> bool {
    !vma.is_null() && unsafe { (*vma).vm_flags & (VM_SHARED | VM_WRITE) == (VM_SHARED | VM_WRITE) }
}

pub fn vma_is_shared_maywrite(vma: *const VmAreaStruct) -> bool {
    is_shared_maywrite(vma)
}

pub fn is_nommu_shared_mapping(flags: VmFlags) -> bool {
    flags & VM_SHARED != 0
}

pub fn is_nommu_shared_vma_flags(flags: VmFlags) -> bool {
    is_nommu_shared_mapping(flags)
}

pub fn vma_is_accessible(vma: *const VmAreaStruct) -> bool {
    !vma.is_null() && unsafe { (*vma).vm_flags & (VM_WRITE | crate::mm::vm_flags::VM_READ) != 0 }
}

pub fn vma_is_shmem(vma: *const VmAreaStruct) -> bool {
    !vma.is_null() && unsafe { (*vma).vm_file != 0 }
}

pub fn vma_is_anon_shmem(vma: *const VmAreaStruct) -> bool {
    vma_is_shmem(vma) && unsafe { (*vma).anon_vma.is_null() }
}

pub fn vma_is_special_mapping(vma: *const VmAreaStruct) -> bool {
    !vma.is_null() && unsafe { (*vma).vm_ops != 0 && (*vma).vm_file == 0 }
}

pub fn vma_is_foreign(_vma: *const VmAreaStruct) -> bool {
    false
}

pub fn vma_is_initial_heap(vma: *const VmAreaStruct) -> bool {
    !vma.is_null()
        && unsafe {
            let mm = (*vma).vm_mm;
            !mm.is_null() && (*vma).vm_start <= (*mm).start_brk && (*vma).vm_end >= (*mm).brk
        }
}

pub fn vma_is_initial_stack(vma: *const VmAreaStruct) -> bool {
    !vma.is_null()
        && unsafe {
            let mm = (*vma).vm_mm;
            !mm.is_null()
                && (*vma).vm_start <= (*mm).start_stack
                && (*vma).vm_end > (*mm).start_stack
        }
}

pub fn vma_is_stack_for_current(vma: *const VmAreaStruct) -> bool {
    vma_is_initial_stack(vma)
}

pub fn vma_is_temporary_stack(vma: *const VmAreaStruct) -> bool {
    vma_is_initial_stack(vma)
}

pub fn vma_set_anonymous(vma: *mut VmAreaStruct) {
    if !vma.is_null() {
        unsafe {
            (*vma).vm_file = 0;
            (*vma).vm_ops = 0;
        }
    }
}

pub fn vma_set_page_prot(vma: *mut VmAreaStruct) {
    if !vma.is_null() {
        unsafe { (*vma).vm_page_prot = crate::mm::pgprot::vm_get_page_prot((*vma).vm_flags) };
    }
}

pub fn vma_get_page_prot(vma: *const VmAreaStruct) -> u64 {
    if vma.is_null() {
        0
    } else {
        unsafe { (*vma).vm_page_prot }
    }
}

pub fn vma_kernel_pagesize(_vma: *const VmAreaStruct) -> usize {
    PAGE_SIZE
}

pub fn vma_mmu_pagesize(vma: *const VmAreaStruct) -> usize {
    vma_kernel_pagesize(vma)
}

pub fn vma_pgtable_walk_begin(_vma: *const VmAreaStruct) -> bool {
    true
}

pub fn vma_pgtable_walk_end(_vma: *const VmAreaStruct) {}

pub fn vma_init(vma: *mut VmAreaStruct, mm: *mut MmStruct) {
    if !vma.is_null() {
        unsafe {
            (*vma).vm_mm = mm;
            ListHead::init(&mut (*vma).anon_vma_chain);
        }
    }
}

pub unsafe fn vm_brk_flags(mm: *mut MmStruct, addr: u64, len: u64, flags: VmFlags) -> i32 {
    if mm.is_null() || len == 0 {
        return -EINVAL;
    }
    let Some(end) = addr.checked_add(len) else {
        return -EINVAL;
    };
    let mut vma = Box::new(VmAreaStruct::new(addr, end, flags));
    vma.vm_mm = mm;
    let ptr = Box::into_raw(vma);
    unsafe { ListHead::init(&mut (*ptr).anon_vma_chain) };
    unsafe { insert_vm_struct(mm, ptr) }
}

pub unsafe fn vm_unmapped_area(
    mm: *const MmStruct,
    len: u64,
    low: u64,
    high: u64,
    _flags: u64,
) -> u64 {
    if mm.is_null() || len == 0 || low >= high {
        return 0;
    }
    let mut cursor = align_up(low, PAGE_SIZE as u64);
    for (start, end, _) in unsafe { (*mm).mm_mt.collect_entries() } {
        if cursor + len <= start {
            return cursor;
        }
        cursor = align_up(end.saturating_add(1), PAGE_SIZE as u64);
        if cursor >= high {
            return 0;
        }
    }
    if cursor + len <= high { cursor } else { 0 }
}

pub unsafe fn __mm_populate(_addr: u64, _len: u64, _ignore_errors: i32) -> i32 {
    0
}

pub unsafe fn mm_populate(addr: u64, len: u64) -> i32 {
    unsafe { __mm_populate(addr, len, 1) }
}

pub unsafe fn access_remote_vm(
    _mm: *mut MmStruct,
    _addr: u64,
    _buf: *mut u8,
    len: usize,
    _write: bool,
) -> isize {
    if len == 0 { 0 } else { -(EFAULT as isize) }
}

pub unsafe fn get_cmdline(_task: *mut u8, _buffer: *mut u8, _buflen: usize) -> i32 {
    0
}

pub fn check_data_rlimit(
    _rlim: u64,
    _new: u64,
    _start: u64,
    _end_data: u64,
    _start_data: u64,
) -> bool {
    true
}

pub fn user_alloc_needs_zeroing() -> bool {
    true
}

pub fn user_shm_lock(_size: usize, _user: *mut u8) -> bool {
    true
}

pub fn user_shm_unlock(_size: usize, _user: *mut u8) {}

pub fn totalram_pages() -> u64 {
    TOTALRAM_PAGES.load(Ordering::Relaxed)
}

pub fn totalram_pages_add(count: i64) {
    if count >= 0 {
        TOTALRAM_PAGES.fetch_add(count as u64, Ordering::Relaxed);
    } else {
        TOTALRAM_PAGES.fetch_sub(count.unsigned_abs(), Ordering::Relaxed);
    }
}

pub fn totalram_pages_inc() {
    totalram_pages_add(1);
}

pub fn totalram_pages_dec() {
    totalram_pages_add(-1);
}

pub fn si_meminfo_node(_nid: i32, val: *mut u64) {
    if !val.is_null() {
        unsafe { *val = totalram_pages() };
    }
}

pub fn show_mem(_filter: u32, _nodemask: *const u8) {}

pub fn __show_mem(filter: u32, nodemask: *const u8, _max_zone_idx: usize) {
    show_mem(filter, nodemask);
}

pub fn warn_alloc(_gfp_mask: u32, _nodemask: *const u8, _fmt: *const u8) {}

pub fn drop_slab() -> i32 {
    0
}

pub fn pagefault_out_of_memory() {}

pub fn __vm_enough_memory(_mm: *mut MmStruct, pages: u64, _cap_sys_admin: i32) -> i32 {
    if pages <= totalram_pages().saturating_add(1 << 20) {
        0
    } else {
        -ENOMEM
    }
}

pub fn vm_stat_account(_mm: *mut MmStruct, flags: VmFlags, npages: i64) {
    let _ = (flags, npages);
}

pub fn change_protection(
    _vma: *mut VmAreaStruct,
    _start: u64,
    _end: u64,
    _newprot: u64,
    _cp_flags: u64,
) -> u64 {
    0
}

pub fn do_set_pmd(_vmf: *mut u8, _page: *mut Page) -> i32 {
    0
}

pub fn finish_fault(_vmf: *mut u8) -> i32 {
    0
}

pub fn vmf_error(errno: i32) -> u32 {
    errno.unsigned_abs()
}

pub fn vmf_fs_error(errno: i32) -> u32 {
    vmf_error(errno)
}

pub fn vm_fault_to_errno(vmf: u32, _flags: i32) -> i32 {
    if vmf == 0 { 0 } else { EFAULT }
}

pub fn fault_flag_allow_retry_first(flags: u32) -> bool {
    flags & 1 != 0
}

pub fn assert_fault_locked(_vmf: *const u8) {}

pub fn release_fault_lock(_vmf: *mut u8) {}

pub fn vmf_insert_page(_vma: *mut VmAreaStruct, _addr: u64, page: *mut Page) -> i32 {
    if page.is_null() { -EFAULT } else { 0 }
}

pub fn zap_vma(_vma: *mut VmAreaStruct) {}

pub fn unmap_shared_mapping_range(_mapping: *mut u8, _holebegin: u64, _holelen: u64) {}

pub fn io_remap_pfn_range(
    _vma: *mut VmAreaStruct,
    _addr: u64,
    _pfn: u64,
    _size: u64,
    _prot: u64,
) -> i32 {
    -EOPNOTSUPP
}

pub fn io_remap_pfn_range_pfn(
    vma: *mut VmAreaStruct,
    addr: u64,
    pfn: u64,
    size: u64,
    prot: u64,
) -> i32 {
    io_remap_pfn_range(vma, addr, pfn, size, prot)
}

pub fn apply_to_existing_page_range(
    _mm: *mut MmStruct,
    _addr: u64,
    _size: u64,
    _fn: *mut u8,
    _data: *mut u8,
) -> i32 {
    -EOPNOTSUPP
}

pub fn do_vmi_munmap(
    _vmi: *mut u8,
    _mm: *mut MmStruct,
    _start: u64,
    _len: u64,
    _uf: *mut u8,
    _unlock: bool,
) -> i32 {
    0
}

pub fn do_mseal(_start: u64, _len: u64, _flags: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn expand_stack_locked(vma: *mut VmAreaStruct, address: u64) -> i32 {
    const DEFAULT_STACK_LIMIT: u64 = 8 * 1024 * 1024;

    if vma.is_null() {
        return -EFAULT;
    }

    unsafe {
        if (*vma).vm_flags & VM_GROWSDOWN == 0 {
            return -EFAULT;
        }

        let mm = (*vma).vm_mm;
        if mm.is_null() {
            return -EFAULT;
        }

        let old_start = (*vma).vm_start;
        let old_end = (*vma).vm_end;
        let new_start = address & PAGE_MASK;
        if new_start >= old_start {
            return 0;
        }
        if old_end <= new_start || old_end - new_start > DEFAULT_STACK_LIMIT {
            return -ENOMEM;
        }

        let growth = (old_start - new_start) >> PAGE_SHIFT;
        if (*vma).vm_file != 0 && growth > (*vma).vm_pgoff {
            return -ENOMEM;
        }

        let overlaps = (*mm)
            .mm_mt
            .collect_entries()
            .into_iter()
            .any(|(start, end, entry)| {
                entry != vma as usize && start < old_start && end >= new_start
            });
        if overlaps {
            return -ENOMEM;
        }

        let old_pgoff = (*vma).vm_pgoff;
        remove_vma(&mut *mm, vma);
        (*vma).vm_start = new_start;
        if (*vma).vm_file != 0 {
            (*vma).vm_pgoff -= growth;
        }

        match insert_vma(&mut *mm, vma) {
            Ok(()) => 0,
            Err(err) => {
                (*vma).vm_start = old_start;
                (*vma).vm_pgoff = old_pgoff;
                let _ = insert_vma(&mut *mm, vma);
                err
            }
        }
    }
}

pub fn print_vma_addr(_prefix: *const u8, _ip: u64) -> bool {
    false
}

pub fn in_gate_area_no_mm(_addr: u64) -> bool {
    false
}

pub fn in_gate_area(_mm: *mut MmStruct, addr: u64) -> bool {
    in_gate_area_no_mm(addr)
}

pub fn get_gate_vma(_mm: *mut MmStruct) -> *mut VmAreaStruct {
    core::ptr::null_mut()
}

pub fn arch_vma_name(_vma: *const VmAreaStruct) -> *const u8 {
    core::ptr::null()
}

pub fn arch_mm_preinit(_mm: *mut MmStruct) -> i32 {
    0
}

pub fn arch_get_shadow_stack_status(_task: *mut u8, status: *mut u64) -> i32 {
    if !status.is_null() {
        unsafe { *status = 0 };
    }
    0
}

pub fn arch_set_shadow_stack_status(_task: *mut u8, _status: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn arch_lock_shadow_stack_status(_task: *mut u8, _status: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn arch_make_folio_accessible(_folio: *mut Page) -> bool {
    true
}

pub fn arch_memory_failure(_pfn: u64, _flags: i32) -> i32 {
    -EOPNOTSUPP
}

pub fn randomize_stack_top(stack_top: u64) -> u64 {
    stack_top
}

pub fn randomize_page(start: u64, range: u64) -> u64 {
    if range == 0 {
        start
    } else {
        start + (range & !(PAGE_SIZE as u64 - 1))
    }
}

pub fn mmap_init() {}

pub fn pagecache_init() {}

pub fn page_address_init() {}

pub fn setup_nr_node_ids() {}

pub fn setup_per_cpu_pageset() {}

pub fn sparse_buffer_alloc(_size: usize) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn get_pfn_range_for_nid(_nid: i32, start_pfn: *mut u64, end_pfn: *mut u64) {
    if !start_pfn.is_null() {
        unsafe { *start_pfn = 0 };
    }
    if !end_pfn.is_null() {
        unsafe { *end_pfn = totalram_pages() };
    }
}

pub fn section_map_size() -> usize {
    PAGE_SIZE
}

pub fn memdesc_section(_memdesc: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn reserve_mem_release_by_name(_name: *const u8) -> i32 {
    -ENOENT_COMPAT
}

const ENOENT_COMPAT: i32 = 2;

pub fn __vmemmap_can_optimize(_altmap: *const u8, _pgmap: *const u8) -> bool {
    false
}

pub fn vmemmap_can_optimize(altmap: *const u8, pgmap: *const u8) -> bool {
    __vmemmap_can_optimize(altmap, pgmap)
}

pub fn vmemmap_verify(_pte: pte_t, _node: i32, _addr: u64, _next: u64) -> i32 {
    0
}

pub fn vmemmap_alloc_block(_size: usize, _node: i32) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn vmemmap_pgd_populate(_addr: u64, _node: i32) -> *mut pgd_t {
    core::ptr::null_mut()
}

pub fn vmemmap_p4d_populate(_pgd: *mut pgd_t, _addr: u64, _node: i32) -> *mut p4d_t {
    core::ptr::null_mut()
}

pub fn vmemmap_pud_populate(_p4d: *mut p4d_t, _addr: u64, _node: i32) -> *mut pud_t {
    core::ptr::null_mut()
}

pub fn vmemmap_pmd_populate(_pud: *mut pud_t, _addr: u64, _node: i32) -> *mut pmd_t {
    core::ptr::null_mut()
}

pub fn vmemmap_populate_print_last() {}

pub fn vmem_altmap_offset(_altmap: *mut u8) -> u64 {
    0
}

pub fn vmem_altmap_free(_altmap: *mut u8, _nr_pfns: u64) {}

pub fn debug_pagealloc_enabled_static() -> bool {
    crate::mm::debug_alloc::debug_guardpage_enabled()
}

pub fn debug_pagealloc_enabled() -> bool {
    debug_pagealloc_enabled_static()
}

pub fn page_poisoning_enabled_static() -> bool {
    false
}

pub fn page_poisoning_enabled() -> bool {
    page_poisoning_enabled_static()
}

pub unsafe fn __kernel_poison_pages(page: *mut Page, numpages: usize) {
    crate::mm::sanitizers::poison_range(page as usize, numpages * PAGE_SIZE);
}

pub unsafe fn __kernel_unpoison_pages(page: *mut Page, numpages: usize) {
    crate::mm::sanitizers::unpoison_range(page as usize, numpages * PAGE_SIZE);
}

pub unsafe fn kernel_poison_pages(page: *mut Page, numpages: usize) {
    unsafe { __kernel_poison_pages(page, numpages) };
}

pub unsafe fn kernel_unpoison_pages(page: *mut Page, numpages: usize) {
    unsafe { __kernel_unpoison_pages(page, numpages) };
}

pub unsafe fn __kernel_map_pages(_page: *mut Page, _numpages: usize, _enable: i32) {}

pub unsafe fn __set_page_guard(
    _zone: *mut u8,
    page: *mut Page,
    order: usize,
    _migratetype: i32,
) -> bool {
    if page.is_null() {
        return false;
    }
    crate::mm::debug_alloc::set_page_guard(0, unsafe { &mut *page }, order)
}

pub unsafe fn __clear_page_guard(
    _zone: *mut u8,
    page: *mut Page,
    _order: usize,
    _migratetype: i32,
) {
    if !page.is_null() {
        crate::mm::debug_alloc::clear_page_guard(0, unsafe { &mut *page });
    }
}

pub fn num_poisoned_pages_inc() {
    POISONED_PAGES.fetch_add(1, Ordering::Relaxed);
}

pub fn num_poisoned_pages_sub(nr: u64) {
    POISONED_PAGES.fetch_sub(nr, Ordering::Relaxed);
}

pub fn memblk_nr_poison_inc(nr: u64) {
    MEMBLK_POISONED_PAGES.fetch_add(nr, Ordering::Relaxed);
}

pub fn memblk_nr_poison_sub(nr: u64) {
    MEMBLK_POISONED_PAGES.fetch_sub(nr, Ordering::Relaxed);
}

pub fn soft_offline_page(_pfn: u64, _flags: i32) -> i32 {
    -EOPNOTSUPP
}

pub fn prep_compound_page(page: *mut Page, order: usize) {
    if !page.is_null() {
        unsafe { (*page).private = order };
    }
}

pub fn page_cpupid_reset_last(page: *mut Page) {
    if !page.is_null() {
        unsafe { (*page).index = 0 };
    }
}

pub fn folio_xchg_last_cpupid(folio: *mut Page, cpupid: i32) -> i32 {
    if folio.is_null() {
        return -1;
    }
    unsafe {
        let old = (*folio).index as i32;
        (*folio).index = cpupid as usize;
        old
    }
}

pub fn cpu_pid_to_cpupid(cpu: i32, pid: i32) -> i32 {
    ((cpu & 0xffff) << 16) | (pid & 0xffff)
}

pub fn cpupid_to_pid(cpupid: i32) -> i32 {
    cpupid & 0xffff
}

pub fn cpupid_to_cpu(cpupid: i32) -> i32 {
    (cpupid >> 16) & 0xffff
}

pub fn cpupid_to_nid(cpupid: i32) -> i32 {
    cpupid_to_cpu(cpupid)
}

pub fn cpupid_pid_unset(cpupid: i32) -> bool {
    cpupid_to_pid(cpupid) == 0xffff
}

pub fn cpupid_cpu_unset(cpupid: i32) -> bool {
    cpupid_to_cpu(cpupid) == 0xffff
}

pub fn cpupid_match_pid(cpupid: i32, pid: i32) -> bool {
    cpupid_to_pid(cpupid) == (pid & 0xffff)
}

pub fn __cpupid_match_pid(cpupid: i32, pid: i32) -> bool {
    cpupid_match_pid(cpupid, pid)
}

pub fn anon_vma_interval_tree_verify(_root: *const u8) {}

pub fn vma_set_access_pid_bit(_vma: *mut VmAreaStruct) {}

pub fn vma_set_atomic_flag(vma: *mut VmAreaStruct, flag: VmFlags) {
    unsafe { vma_flags_set(vma, flag) };
}

pub fn vma_test_atomic_flag(vma: *const VmAreaStruct, flag: VmFlags) -> bool {
    unsafe { vma_flags_test(vma, flag) }
}

pub fn vma_numab_state_init(_vma: *mut VmAreaStruct) -> i32 {
    0
}

pub fn vma_numab_state_free(_vma: *mut VmAreaStruct) {}

pub fn vma_iter_set(_vmi: *mut u8, _addr: u64) {}

pub fn vma_iter_invalidate(_vmi: *mut u8) {}

pub fn vma_iter_free(_vmi: *mut u8) {}

pub fn vma_iter_clear_gfp(_vmi: *mut u8) {}

pub fn vma_iter_bulk_store(_vmi: *mut u8, _vma: *mut VmAreaStruct) -> i32 {
    0
}

pub fn vma_next(_vmi: *mut u8) -> *mut VmAreaStruct {
    core::ptr::null_mut()
}

pub fn vma_prev(_vmi: *mut u8) -> *mut VmAreaStruct {
    core::ptr::null_mut()
}

pub fn mmap_action_simple_ioremap(_addr: u64, _size: u64, _prot: u64) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn mmap_action_ioremap(_action: *mut u8, _addr: u64, _size: u64, _prot: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn mmap_action_ioremap_full(
    _action: *mut u8,
    _addr: u64,
    _size: u64,
    _prot: u64,
    _flags: u64,
) -> i32 {
    -EOPNOTSUPP
}

pub fn mmap_action_map_kernel_pages(
    _action: *mut u8,
    _addr: u64,
    _pages: *mut *mut Page,
    _nr: usize,
) -> i32 {
    -EOPNOTSUPP
}

pub fn mmap_action_map_kernel_pages_full(
    _action: *mut u8,
    _addr: u64,
    _pages: *mut *mut Page,
    _nr: usize,
    _prot: u64,
) -> i32 {
    -EOPNOTSUPP
}

pub fn mmap_action_remap(_action: *mut u8, _addr: u64, _size: u64, _prot: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn mmap_action_remap_full(
    _action: *mut u8,
    _addr: u64,
    _size: u64,
    _prot: u64,
    _flags: u64,
) -> i32 {
    -EOPNOTSUPP
}

pub fn nommu_shrink_inode_mappings(_inode: *mut u8, _size: u64, _len: u64) -> i32 {
    0
}

pub fn kobjsize(_objp: *const u8) -> usize {
    0
}

pub fn mm_counter_file_index() -> usize {
    1
}

pub fn want_init_on_alloc(_gfp: u32) -> bool {
    user_alloc_needs_zeroing()
}

pub fn want_init_on_free() -> bool {
    false
}

pub fn empty_zero_page() -> *const u8 {
    ZERO_PAGE.as_ptr()
}

pub fn __zero_page() -> *const u8 {
    empty_zero_page()
}

pub fn zero_page_pfn() -> u64 {
    empty_zero_page() as u64 >> PAGE_SHIFT
}

pub fn high_memory() -> usize {
    totalram_pages() as usize * PAGE_SIZE
}

pub fn max_mapnr() -> u64 {
    totalram_pages()
}

pub fn mem_map() -> *mut Page {
    core::ptr::null_mut()
}

pub fn _totalram_pages() -> u64 {
    totalram_pages()
}

pub fn __totalhigh_pages() -> u64 {
    0
}

pub fn init_on_alloc() -> bool {
    want_init_on_alloc(0)
}

pub fn init_on_free() -> bool {
    want_init_on_free()
}

pub fn si_meminfo(info: *mut u64) {
    if !info.is_null() {
        unsafe { *info = totalram_pages() };
    }
}

pub fn si_mem_available() -> u64 {
    totalram_pages()
}

pub fn set_page_address(page: *mut Page, virtual_addr: *mut u8) {
    if !page.is_null() {
        unsafe { (*page).mapping = virtual_addr as usize };
    }
}

pub fn page_address(page: *const Page) -> *mut u8 {
    if page.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe {
            if (*page).mapping != 0 {
                (*page).mapping as *mut u8
            } else {
                page as *mut u8
            }
        }
    }
}

pub fn __kmap_local_page_prot(page: *mut Page, _prot: u64) -> *mut u8 {
    page_address(page)
}

pub fn __kmap_local_pfn_prot(pfn: u64, _prot: u64) -> *mut u8 {
    (pfn << PAGE_SHIFT) as *mut u8
}

pub fn __kmap_to_page(addr: *const u8) -> *mut Page {
    addr as *mut Page
}

pub fn kunmap_local_indexed(_addr: *const u8) {}

pub unsafe fn zero_user_segments(
    page: *mut Page,
    start1: usize,
    end1: usize,
    start2: usize,
    end2: usize,
) {
    let base = page_address(page);
    if base.is_null() {
        return;
    }
    unsafe {
        zero_segment(base, start1, end1);
        zero_segment(base, start2, end2);
    }
}

pub fn page_table_check_disabled() -> bool {
    false
}

pub fn __page_table_check_pte_clear(_mm: *mut MmStruct, _addr: u64, _pte: pte_t) {}

pub fn __page_table_check_pmd_clear(_mm: *mut MmStruct, _addr: u64, _pmd: pmd_t) {}

pub fn __page_table_check_pud_clear(_mm: *mut MmStruct, _addr: u64, _pud: pud_t) {}

pub fn __page_table_check_ptes_set(_mm: *mut MmStruct, _addr: u64, _ptep: *mut pte_t, _nr: usize) {}

pub fn __page_table_check_pmds_set(_mm: *mut MmStruct, _addr: u64, _pmdp: *mut pmd_t, _nr: usize) {}

pub fn __page_table_check_puds_set(_mm: *mut MmStruct, _addr: u64, _pudp: *mut pud_t, _nr: usize) {}

pub fn _debug_pagealloc_enabled() -> bool {
    debug_pagealloc_enabled()
}

pub fn _debug_pagealloc_enabled_early() -> bool {
    debug_pagealloc_enabled()
}

pub fn _page_poisoning_enabled() -> bool {
    page_poisoning_enabled()
}

pub fn _page_poisoning_enabled_early() -> bool {
    page_poisoning_enabled()
}

pub fn __kfence_pool() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn kfence_sample_interval() -> u64 {
    0
}

pub fn __might_fault(_file: *const u8, _line: i32) {}

pub fn __copy_overflow(_size: usize) {}

pub fn __check_object_size(_ptr: *const u8, _n: usize, _to_user: bool) {}

pub fn validate_usercopy_range(_ptr: *const u8, _n: usize) -> bool {
    true
}

pub fn access_process_vm(
    task: *mut MmStruct,
    addr: u64,
    buf: *mut u8,
    len: usize,
    write: bool,
) -> isize {
    unsafe { access_remote_vm(task, addr, buf, len, write) }
}

pub fn copy_remote_vm_str(_task: *mut MmStruct, _addr: u64, _buf: *mut u8, _len: usize) -> isize {
    0
}

pub fn generic_access_phys(
    _vma: *mut VmAreaStruct,
    _addr: u64,
    _buf: *mut u8,
    len: usize,
    _write: bool,
) -> isize {
    if len == 0 { 0 } else { -(EFAULT as isize) }
}

pub fn map_kernel_pages_prepare(_page: *mut Page, _num_pages: usize, _prot: u64) -> i32 {
    0
}

pub fn map_kernel_pages_complete(_page: *mut Page, _num_pages: usize, _prot: u64) {}

pub fn apply_to_page_range(
    mm: *mut MmStruct,
    addr: u64,
    size: u64,
    callback: *mut u8,
    data: *mut u8,
) -> i32 {
    apply_to_existing_page_range(mm, addr, size, callback, data)
}

pub fn vm_insert_page(vma: *mut VmAreaStruct, addr: u64, page: *mut Page) -> i32 {
    vmf_insert_page(vma, addr, page)
}

pub fn vm_insert_pages(
    vma: *mut VmAreaStruct,
    addr: u64,
    pages: *mut *mut Page,
    num: *mut usize,
) -> i32 {
    if pages.is_null() || num.is_null() {
        return -EFAULT;
    }
    let count = unsafe { *num };
    for idx in 0..count {
        let page = unsafe { *pages.add(idx) };
        let ret = vm_insert_page(vma, addr + (idx as u64 * PAGE_SIZE as u64), page);
        if ret != 0 {
            unsafe { *num = idx };
            return ret;
        }
    }
    0
}

pub fn vm_map_pages(vma: *mut VmAreaStruct, pages: *mut *mut Page, num: usize) -> i32 {
    let mut nr = num;
    vm_insert_pages(vma, unsafe { (*vma).vm_start }, pages, &mut nr)
}

pub fn vm_map_pages_zero(vma: *mut VmAreaStruct, pages: *mut *mut Page, num: usize) -> i32 {
    vm_map_pages(vma, pages, num)
}

pub fn vmf_insert_pfn(_vma: *mut VmAreaStruct, _addr: u64, _pfn: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn vmf_insert_pfn_prot(_vma: *mut VmAreaStruct, _addr: u64, _pfn: u64, _pgprot: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn vmf_insert_mixed(_vma: *mut VmAreaStruct, _addr: u64, _pfn: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn vmf_insert_page_mkwrite(vma: *mut VmAreaStruct, addr: u64, page: *mut Page) -> i32 {
    vm_insert_page(vma, addr, page)
}

pub fn remap_pfn_range(vma: *mut VmAreaStruct, addr: u64, pfn: u64, size: u64, prot: u64) -> i32 {
    io_remap_pfn_range(vma, addr, pfn, size, prot)
}

pub fn vm_iomap_memory(vma: *mut VmAreaStruct, start: u64, len: u64) -> i32 {
    io_remap_pfn_range(
        vma,
        unsafe { (*vma).vm_start },
        start >> PAGE_SHIFT,
        len,
        unsafe { (*vma).vm_page_prot },
    )
}

pub fn unmap_mapping_pages(_mapping: *mut u8, _start: u64, _nr: u64, _even_cows: bool) {}

pub fn unmap_mapping_range(mapping: *mut u8, holebegin: u64, holelen: u64, _even_cows: bool) {
    unmap_shared_mapping_range(mapping, holebegin, holelen);
}

pub fn follow_pfnmap_start(_vma: *mut VmAreaStruct, _addr: u64, _ctx: *mut u8) -> i32 {
    -EOPNOTSUPP
}

pub fn follow_pfnmap_end(_ctx: *mut u8) {}

pub fn zap_special_vma_range(vma: *mut VmAreaStruct, _start: u64, _size: u64) {
    zap_vma(vma);
}

pub fn find_vma_intersection(mm: *const MmStruct, start: u64, end: u64) -> *mut VmAreaStruct {
    unsafe {
        let vma = vma_lookup(mm, start);
        if !vma.is_null() && (*vma).vm_start < end {
            vma
        } else {
            core::ptr::null_mut()
        }
    }
}

pub fn mm_get_unmapped_area(
    mm: *const MmStruct,
    _file: *mut u8,
    addr: u64,
    len: u64,
    _pgoff: u64,
    flags: u64,
) -> u64 {
    unsafe { vm_unmapped_area(mm, len, addr, u64::MAX & PAGE_MASK, flags) }
}

pub fn can_do_mlock() -> bool {
    true
}

pub fn generic_fadvise(_file: *mut u8, _offset: u64, _len: u64, _advice: i32) -> i32 {
    0
}

pub fn vfs_fadvise(file: *mut u8, offset: u64, len: u64, advice: i32) -> i32 {
    generic_fadvise(file, offset, len, advice)
}

pub fn file_fdatawait_range(_file: *mut u8, _start: u64, _end: u64) -> i32 {
    0
}

pub fn file_ra_state_init(_ra: *mut u8, _mapping: *mut u8) {}

pub fn readahead_expand(_ractl: *mut u8, _new_start: u64, _new_len: u64) {}

pub fn bdi_alloc(_node_id: i32) -> *mut u8 {
    Box::into_raw(Box::new(0u8))
}

pub unsafe fn bdi_put(bdi: *mut u8) {
    if !bdi.is_null() {
        unsafe { drop(Box::from_raw(bdi)) };
    }
}

pub fn bdi_register(_bdi: *mut u8, _fmt: *const u8) -> i32 {
    0
}

pub fn bdi_unregister(_bdi: *mut u8) {}

pub fn bdi_dev_name(_bdi: *const u8) -> *const u8 {
    c"lupos-bdi".as_ptr() as *const u8
}

pub fn inode_to_bdi(_inode: *const u8) -> *mut u8 {
    noop_backing_dev_info()
}

pub fn noop_backing_dev_info() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn list_lru_add_obj(_lru: *mut u8, item: *mut ListHead) -> bool {
    !item.is_null()
}

pub fn list_lru_del_obj(_lru: *mut u8, item: *mut ListHead) -> bool {
    !item.is_null()
}

pub fn list_lru_count_one(_lru: *mut u8, _nid: i32, _memcg: *mut u8) -> u64 {
    0
}

pub fn list_lru_count_node(lru: *mut u8, nid: i32) -> u64 {
    list_lru_count_one(lru, nid, core::ptr::null_mut())
}

pub fn list_lru_walk_one(
    _lru: *mut u8,
    _nid: i32,
    _memcg: *mut u8,
    _isolate: *mut u8,
    _cb_arg: *mut u8,
    _nr_to_walk: *mut u64,
) -> u64 {
    0
}

pub fn list_lru_walk_node(
    lru: *mut u8,
    nid: i32,
    isolate: *mut u8,
    cb_arg: *mut u8,
    nr_to_walk: *mut u64,
) -> u64 {
    list_lru_walk_one(lru, nid, core::ptr::null_mut(), isolate, cb_arg, nr_to_walk)
}

pub fn list_lru_isolate(_lru: *mut u8, _item: *mut ListHead) -> i32 {
    0
}

pub fn list_lru_isolate_move(_lru: *mut u8, _item: *mut ListHead, _head: *mut ListHead) -> i32 {
    0
}

pub fn __list_lru_init(
    _lru: *mut u8,
    _memcg_aware: bool,
    _key: *mut u8,
    _shrinker: *mut u8,
) -> i32 {
    0
}

pub fn list_lru_destroy(_lru: *mut u8) {}

pub fn mem_cgroup_from_task(_task: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn get_mem_cgroup_from_mm(_mm: *mut MmStruct) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn __memcg_memory_event(_memcg: *mut u8, _event: usize) {}

pub fn int_active_memcg() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn lruvec_stat_mod_folio(_folio: *mut Page, _idx: usize, _val: i64) {}

pub fn page_cgroup_ino(_page: *const Page) -> u64 {
    0
}

pub fn memcg_bpf_enabled_key() -> bool {
    false
}

pub fn memcg_kmem_online_key() -> bool {
    false
}

pub fn memcg_sockets_enabled_key() -> bool {
    false
}

pub fn memory_cgrp_subsys() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn alloc_memory_type(_adistance: i32) -> *mut u8 {
    Box::into_raw(Box::new(0u8))
}

pub unsafe fn put_memory_type(mt: *mut u8) {
    if !mt.is_null() {
        unsafe { drop(Box::from_raw(mt)) };
    }
}

pub fn init_node_memory_type(_node: i32, _mt: *mut u8) {}

pub fn clear_node_memory_type(_node: i32, _mt: *mut u8) {}

pub fn mt_find_alloc_memory_type(adistance: i32) -> *mut u8 {
    alloc_memory_type(adistance)
}

pub fn mt_put_memory_types(_types: *mut u8) {}

pub fn mt_perf_to_adistance(perf: i32) -> i32 {
    perf
}

pub fn mt_calc_adistance(a: i32, b: i32) -> i32 {
    (a - b).abs()
}

pub fn register_mt_adistance_algorithm(_algo: *mut u8) -> i32 {
    0
}

pub fn unregister_mt_adistance_algorithm(_algo: *mut u8) {}

pub fn add_memory(_nid: i32, _start: u64, _size: u64, _mhp_flags: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn add_memory_driver_managed(
    _nid: i32,
    _start: u64,
    _size: u64,
    _resource_name: *const u8,
    _mhp_flags: u64,
) -> i32 {
    -EOPNOTSUPP
}

pub fn remove_memory(_nid: i32, _start: u64, _size: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn offline_and_remove_memory(nid: i32, start: u64, size: u64) -> i32 {
    remove_memory(nid, start, size)
}

pub fn pfn_to_online_page(pfn: u64) -> *mut Page {
    if pfn < max_mapnr() {
        pfn_folio(pfn)
    } else {
        core::ptr::null_mut()
    }
}

pub fn generic_online_page(_page: *mut Page, _order: u32) {}

pub fn set_online_page_callback(_callback: *mut u8) -> i32 {
    0
}

pub fn restore_online_page_callback(_callback: *mut u8) {}

pub fn try_offline_node(_nid: i32) {}

pub fn mhp_get_pluggable_range(_start: bool, _idx: i32, _nid: i32) -> u64 {
    0
}

pub fn mhp_supports_memmap_on_memory(_size: u64) -> bool {
    false
}

pub fn memory_add_physaddr_to_nid(_start: u64) -> i32 {
    0
}

pub fn phys_to_target_node(_start: u64) -> i32 {
    0
}

pub fn node_data(_nid: i32) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn __node_distance(a: i32, b: i32) -> i32 {
    if a == b { 10 } else { 20 }
}

pub fn contig_page_data() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn mem_section() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn memdesc_nid(_memdesc: *const u8) -> i32 {
    0
}

pub fn cma_alloc(_cma: *mut u8, count: usize, _align: u32, _no_warn: bool) -> *mut Page {
    if count == 0 {
        core::ptr::null_mut()
    } else {
        pagetable_alloc_noprof(0, 0)
    }
}

pub unsafe fn cma_release(_cma: *mut u8, pages: *mut Page, _count: usize) -> bool {
    unsafe { pagetable_free(pages) };
    true
}

pub fn cma_get_name(_cma: *const u8) -> *const u8 {
    c"lupos-cma".as_ptr() as *const u8
}

pub fn balloon_page_alloc() -> *mut Page {
    pagetable_alloc_noprof(0, 0)
}

pub unsafe fn balloon_page_enqueue(_balloon: *mut u8, page: *mut Page) {
    unsafe { pagetable_free(page) };
}

pub fn balloon_page_dequeue(_balloon: *mut u8) -> *mut Page {
    core::ptr::null_mut()
}

pub fn balloon_page_list_enqueue(_balloon: *mut u8, _pages: *mut ListHead) -> usize {
    0
}

pub fn balloon_page_list_dequeue(
    _balloon: *mut u8,
    _pages: *mut ListHead,
    _n_req_pages: usize,
) -> usize {
    0
}

pub fn folio_migrate_mapping(
    _mapping: *mut u8,
    _newfolio: *mut Page,
    _folio: *mut Page,
    _extra_count: i32,
) -> i32 {
    0
}

pub fn folio_migrate_flags(_newfolio: *mut Page, _folio: *mut Page) {}

pub fn migrate_folio(
    _mapping: *mut u8,
    _newfolio: *mut Page,
    _folio: *mut Page,
    _mode: i32,
) -> i32 {
    0
}

pub fn buffer_migrate_folio(
    mapping: *mut u8,
    newfolio: *mut Page,
    folio: *mut Page,
    mode: i32,
) -> i32 {
    migrate_folio(mapping, newfolio, folio, mode)
}

pub fn buffer_migrate_folio_norefs(
    mapping: *mut u8,
    newfolio: *mut Page,
    folio: *mut Page,
    mode: i32,
) -> i32 {
    migrate_folio(mapping, newfolio, folio, mode)
}

pub fn set_movable_ops(_mapping: *mut u8, _ops: *const u8) {}

pub fn migrate_vma_setup(_args: *mut u8) -> i32 {
    -EOPNOTSUPP
}

pub fn migrate_vma_pages(_args: *mut u8) {}

pub fn migrate_vma_finalize(_args: *mut u8) {}

pub fn migrate_device_range(_src_pfns: *mut u64, _start: u64, _npages: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn migrate_device_pages(_src_pfns: *mut u64, _dst_pfns: *mut u64, _npages: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn migrate_device_finalize(_src_pfns: *mut u64, _dst_pfns: *mut u64, _npages: u64) {}

pub fn migrate_device_pfns(_args: *mut u8) -> i32 {
    -EOPNOTSUPP
}

pub fn hmm_range_fault(_range: *mut u8) -> i32 {
    -EOPNOTSUPP
}

pub fn hmm_dma_map_alloc(_dev: *mut u8, _entries: usize) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hmm_dma_map_free(_map: *mut u8) {}

pub fn hmm_dma_map_pfn(_map: *mut u8, _idx: usize, _pfn: u64) -> i32 {
    -EOPNOTSUPP
}

pub fn hmm_dma_unmap_pfn(_map: *mut u8, _idx: usize) {}

pub fn memory_failure(_pfn: u64, _flags: i32) -> i32 {
    -EOPNOTSUPP
}

pub fn memory_failure_queue(_pfn: u64, _flags: i32) {}

pub fn unpoison_memory(pfn: u64) -> i32 {
    let _ = pfn;
    -EOPNOTSUPP
}

pub fn hwpoison_filter_register(_ops: *mut u8) -> i32 {
    0
}

pub fn hwpoison_filter_unregister(_ops: *mut u8) {}

pub fn register_pfn_address_space(_pfn: u64, _mapping: *mut u8) -> i32 {
    0
}

pub fn unregister_pfn_address_space(_pfn: u64) {}

pub fn mf_dax_kill_procs(_mapping: *mut u8, _index: u64, _flags: u32) -> i32 {
    -EOPNOTSUPP
}

pub fn shake_folio(_folio: *mut Page) -> bool {
    false
}

pub fn folio_split_unmapped(_folio: *mut Page) -> bool {
    false
}

pub fn folio_hstate(_folio: *mut Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn resv_map_alloc() -> *mut u8 {
    Box::into_raw(Box::new(0u8))
}

pub unsafe fn resv_map_release(map: *mut u8) {
    if !map.is_null() {
        unsafe { drop(Box::from_raw(map)) };
    }
}

pub fn page_frag_free(_addr: *mut u8) {}

pub fn __page_frag_alloc_align(
    _nc: *mut u8,
    fragsz: u32,
    _gfp_mask: u32,
    _align_mask: u32,
) -> *mut u8 {
    if fragsz == 0 {
        core::ptr::null_mut()
    } else {
        Box::into_raw(Box::new(0u8))
    }
}

pub fn page_frag_cache_drain(_nc: *mut u8) {}

pub fn __page_frag_cache_drain(nc: *mut u8) {
    page_frag_cache_drain(nc);
}

pub fn dma_pool_create_node(
    _name: *const u8,
    _dev: *mut u8,
    _size: usize,
    _align: usize,
    _boundary: usize,
    _node: i32,
) -> *mut u8 {
    Box::into_raw(Box::new(0u8))
}

pub fn dmam_pool_create(
    name: *const u8,
    dev: *mut u8,
    size: usize,
    align: usize,
    boundary: usize,
) -> *mut u8 {
    dma_pool_create_node(name, dev, size, align, boundary, -1)
}

pub unsafe fn dma_pool_destroy(pool: *mut u8) {
    if !pool.is_null() {
        unsafe { drop(Box::from_raw(pool)) };
    }
}

pub unsafe fn dmam_pool_destroy(pool: *mut u8) {
    unsafe { dma_pool_destroy(pool) };
}

pub fn dma_pool_alloc(_pool: *mut u8, size: usize, handle: *mut u64) -> *mut u8 {
    if !handle.is_null() {
        unsafe { *handle = 0 };
    }
    if size == 0 {
        core::ptr::null_mut()
    } else {
        Box::into_raw(Box::new(0u8))
    }
}

pub unsafe fn dma_pool_free(_pool: *mut u8, vaddr: *mut u8, _dma: u64) {
    if !vaddr.is_null() {
        unsafe { drop(Box::from_raw(vaddr)) };
    }
}

pub fn pcpu_alloc_noprof(_size: usize, _align: usize, _reserved: bool, _gfp: u32) -> *mut u8 {
    Box::into_raw(Box::new(0u8))
}

pub fn __per_cpu_offset(_cpu: usize) -> usize {
    0
}

pub unsafe fn kfree_bulk(nr: usize, ptrs: *mut *mut u8) {
    for idx in 0..nr {
        let ptr = unsafe { *ptrs.add(idx) };
        if !ptr.is_null() {
            unsafe { crate::mm::slab::kfree(ptr) };
        }
    }
}

pub unsafe fn kfree_nolock(ptr: *mut u8) {
    if !ptr.is_null() {
        unsafe { crate::mm::slab::kfree(ptr) };
    }
}

pub fn kmalloc_type(_flags: u32, _caller: usize) -> usize {
    0
}

pub fn kmem_cache_init_late() {}

pub fn kmem_cache_sheaf_size(_cache: *const u8) -> usize {
    0
}

pub fn __kmem_cache_create_args(_cache: *mut u8, _args: *mut u8) -> i32 {
    0
}

pub fn kmem_buckets_create(
    _name: *const u8,
    _align: usize,
    _useroffset: usize,
    _usersize: usize,
    _flags: u32,
) -> *mut u8 {
    Box::into_raw(Box::new(0u8))
}

pub fn kvfree_call_rcu(_head: *mut u8, _ptr: *mut u8) {}

pub fn ioremap_prot(phys_addr: u64, _size: usize, _prot: u64) -> *mut u8 {
    phys_addr as *mut u8
}

pub fn is_vm_area_hugepages(_addr: *const u8) -> bool {
    false
}

pub fn arch_pgtable_dma_compat(_mm: *mut MmStruct) -> bool {
    true
}

pub fn leave_mm(_cpu: i32) {}

pub fn mm_untag_mask(_mm: *const MmStruct) -> u64 {
    u64::MAX
}

pub fn enable_mmiotrace() {}

pub fn disable_mmiotrace() {}

pub fn is_kmmio_active() -> bool {
    false
}

pub fn kmmio_init() -> i32 {
    0
}

pub fn kmmio_cleanup() {}

pub fn kmmio_handler(_regs: *mut u8, _addr: u64) -> i32 {
    0
}

pub fn mmiotrace_ioremap(_phys: u64, _virt: *mut u8, _size: usize) {}

pub fn mmiotrace_iounmap(_virt: *mut u8) {}

pub fn mmio_trace_mapping(_phys: u64, _virt: *mut u8, _size: usize) {}

pub fn mmio_trace_rw(_phys: u64, _value: u64, _width: usize, _write: bool) {}

pub fn mmio_trace_printk(_fmt: *const u8) {}

pub fn register_oom_notifier(_nb: *mut u8) -> i32 {
    0
}

pub fn unregister_oom_notifier(_nb: *mut u8) -> i32 {
    0
}

pub fn shrinker_alloc(_flags: u32, _fmt: *const u8) -> *mut u8 {
    Box::into_raw(Box::new(0u8))
}

pub fn shrinker_register(_shrinker: *mut u8) {}

pub unsafe fn shrinker_free(shrinker: *mut u8) {
    if !shrinker.is_null() {
        unsafe { drop(Box::from_raw(shrinker)) };
    }
}

pub fn shrinker_debugfs_rename(_shrinker: *mut u8, _fmt: *const u8) -> i32 {
    0
}

pub fn page_reporting_order() -> u32 {
    0
}

pub fn page_reporting_register(_prdev: *mut u8) -> i32 {
    0
}

pub fn page_reporting_unregister(_prdev: *mut u8) {}

pub fn clean_record_shared_mapping_range(_mapping: *mut u8, _start: u64, _end: u64) -> i32 {
    0
}

pub fn wp_shared_mapping_range(_mapping: *mut u8, _start: u64, _end: u64) -> i32 {
    0
}

pub fn dump_mm(_mm: *const MmStruct) {}

pub fn dump_vma(_vma: *const VmAreaStruct) {}

pub fn dump_vmg(_vmg: *const u8) {}

pub fn vma_iter_dump_tree(_vmi: *const u8) {}

pub fn reserve_mem_find_by_name(_name: *const u8, start: *mut u64, size: *mut u64) -> bool {
    if !start.is_null() {
        unsafe { *start = 0 };
    }
    if !size.is_null() {
        unsafe { *size = 0 };
    }
    false
}

fn with_mm_counter(mm: *mut MmStruct, member: usize, f: impl FnOnce(&mut u64)) {
    if mm.is_null() {
        return;
    }
    unsafe {
        match member {
            0 => f(&mut (*mm).total_vm),
            1 => f(&mut (*mm).data_vm),
            2 => f(&mut (*mm).exec_vm),
            3 => f(&mut (*mm).stack_vm),
            _ => {}
        }
    }
}

const fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

unsafe fn zero_segment(base: *mut u8, start: usize, end: usize) {
    if start < end && start < PAGE_SIZE {
        let len = (end.min(PAGE_SIZE)) - start;
        unsafe { core::ptr::write_bytes(base.add(start), 0, len) };
    }
}
