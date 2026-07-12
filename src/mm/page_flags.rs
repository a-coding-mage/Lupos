//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Page flags, page types, and GFP (Get Free Pages) allocation flags.
///
/// This module provides Linux-compatible definitions for:
/// - **Page flags** (`PG_*`): per-page status bits stored in `Page::flags`
/// - **Page types** (`PGTY_*`): encoded in `Page::page_type` for non-mapcount pages
/// - **GFP flags**: allocation request modifiers (zone preference, reclaim behavior, etc.)
///
/// All bit positions and values match the Linux kernel exactly so that future
/// milestones (slab, VMA, demand paging) can rely on binary-compatible semantics.
///
/// Ref: Linux include/linux/page-flags.h — enum pageflags, enum pagetype
///      Linux include/linux/gfp_types.h  — GFP bitmask definitions
///      Linux include/linux/mmzone.h     — zone types, migration types
use core::sync::atomic::Ordering;

use crate::mm::page::Page;

// ---------------------------------------------------------------------------
// Page flags  (stored in Page::flags, an AtomicU64)
//
// Bit positions match Linux `enum pageflags` (page-flags.h:93-130).
// The enum is an auto-incrementing C enum starting at 0, so PG_locked = 0,
// PG_writeback = 1, PG_referenced = 2, etc.
// ---------------------------------------------------------------------------

/// Page is locked — do not touch.
pub const PG_LOCKED: u64 = 1 << 0;
/// Page is under writeback to disk.
pub const PG_WRITEBACK: u64 = 1 << 1;
/// Page has been recently accessed (used by LRU reclaim).
pub const PG_REFERENCED: u64 = 1 << 2;
/// Page contents are up to date with backing store.
pub const PG_UPTODATE: u64 = 1 << 3;
/// Page has been modified since last writeback.
pub const PG_DIRTY: u64 = 1 << 4;
/// Page is on an LRU list.
pub const PG_LRU: u64 = 1 << 5;
/// Compound page head — must be bit 6 per Linux convention.
pub const PG_HEAD: u64 = 1 << 6;
/// Page has waiters — must be bit 7, same byte as PG_LOCKED.
pub const PG_WAITERS: u64 = 1 << 7;
/// Page is on the active LRU list.
pub const PG_ACTIVE: u64 = 1 << 8;
/// Page is in the working set.
pub const PG_WORKINGSET: u64 = 1 << 9;
/// Owner-private flag 1 (fs-specific when in page cache).
/// Aliases: PG_swapcache, PG_checked.
pub const PG_OWNER_PRIV_1: u64 = 1 << 10;
/// Owner-private flag 2 (fs-specific when in page cache).
pub const PG_OWNER_2: u64 = 1 << 11;
/// Architecture-specific flag 1.
pub const PG_ARCH_1: u64 = 1 << 12;
/// Page is reserved by the kernel — never reclaim or swap.
pub const PG_RESERVED: u64 = 1 << 13;
/// Page has filesystem-private data.
pub const PG_PRIVATE: u64 = 1 << 14;
/// Page has filesystem auxiliary data.
pub const PG_PRIVATE_2: u64 = 1 << 15;
/// Page should be reclaimed ASAP.
/// Alias: PG_readahead.
pub const PG_RECLAIM: u64 = 1 << 16;
/// Page is backed by RAM/swap (anonymous or shmem).
pub const PG_SWAPBACKED: u64 = 1 << 17;
/// Page is unevictable (pinned, mlocked, etc.).
pub const PG_UNEVICTABLE: u64 = 1 << 18;
/// Drop page on IO completion.
pub const PG_DROPBEHIND: u64 = 1 << 19;
/// Page is VMA-mlocked (CONFIG_MMU).
pub const PG_MLOCKED: u64 = 1 << 20;
/// Hardware-poisoned page — do not touch (CONFIG_MEMORY_FAILURE).
pub const PG_HWPOISON: u64 = 1 << 21;

/// Total number of defined page flags (matches Linux __NR_PAGEFLAGS).
/// Conditional flags (PG_young, PG_idle, PG_arch_2, PG_arch_3) are omitted
/// for now — they depend on CONFIG options not yet relevant.
pub const NR_PAGEFLAGS: u32 = 22;

// Aliases matching Linux (page-flags.h:132-137)
pub const PG_READAHEAD: u64 = PG_RECLAIM;
pub const PG_SWAPCACHE: u64 = PG_OWNER_PRIV_1;
pub const PG_CHECKED: u64 = PG_OWNER_PRIV_1;

// ---------------------------------------------------------------------------
// Page types  (stored in Page::page_type, an AtomicU32)
//
// Linux uses the top 8 bits of page_type as a type tag (the bottom 24 bits
// can be used for per-type data).  A page_type value is "typed" when the
// top-byte is < 0xFF.  The encoding is: `(PGTY_xxx << 24)`.
//
// Ref: Linux include/linux/page-flags.h — enum pagetype (lines 925-939)
// ---------------------------------------------------------------------------

/// Page is free and managed by the buddy allocator.
pub const PGTY_BUDDY: u8 = 0xf0;
/// Page is offline (memory hotplug).
pub const PGTY_OFFLINE: u8 = 0xf1;
/// Page is used as a page table.
pub const PGTY_TABLE: u8 = 0xf2;
/// Guard page (debug, prevents coalescing).
pub const PGTY_GUARD: u8 = 0xf3;
/// HugeTLB page.
pub const PGTY_HUGETLB: u8 = 0xf4;
/// Slab allocator page.
pub const PGTY_SLAB: u8 = 0xf5;
/// ZSmalloc page.
pub const PGTY_ZSMALLOC: u8 = 0xf6;
/// Unaccepted memory (confidential computing).
pub const PGTY_UNACCEPTED: u8 = 0xf7;
/// Large kmalloc page.
pub const PGTY_LARGE_KMALLOC: u8 = 0xf8;
/// Sentinel: mapcount underflow marker.
pub const PGTY_MAPCOUNT_UNDERFLOW: u8 = 0xff;

/// No type assigned — page_type field stores mapcount or is uninitialized.
/// Linux uses `PAGE_TYPE_BASE = 0xFFFF_FFFF` (all bits set) as "no type".
pub const PAGE_TYPE_NONE: u32 = u32::MAX;

/// Encode a page type tag into the `page_type` u32 field.
///
/// Linux stores the type in the top byte: `page_type = (pgty << 24)`.
/// The bottom 24 bits can hold per-type data (unused in M7).
///
/// Ref: `page-flags.h` — `PAGE_TYPE_OPS` macro, `page_type_has_type()`
#[inline]
pub const fn encode_page_type(pgty: u8) -> u32 {
    (pgty as u32) << 24
}

/// Check if a raw `page_type` value represents a typed page.
///
/// A page is "typed" when `page_type < (PGTY_mapcount_underflow << 24)`,
/// i.e., the top byte is less than 0xFF.
#[inline]
pub const fn page_type_has_type(page_type: u32) -> bool {
    page_type < encode_page_type(PGTY_MAPCOUNT_UNDERFLOW)
}

/// Extract the page type tag (top byte) from a raw `page_type` value.
#[inline]
pub const fn decode_page_type(page_type: u32) -> u8 {
    (page_type >> 24) as u8
}

// ---------------------------------------------------------------------------
// GFP flags  (allocation request modifiers)
//
// Bit positions match Linux `enum { ___GFP_DMA_BIT, ... }` (gfp_types.h:26-59).
// The enum auto-increments from 0, so ___GFP_DMA_BIT = 0, etc.
//
// Ref: Linux include/linux/gfp_types.h
// ---------------------------------------------------------------------------

/// Type alias for GFP flags — matches Linux `gfp_t` (a `u32` bitmask).
pub type GfpFlags = u32;

// --- Zone modifiers (bits 0-3) ---

/// Allocate from ZONE_DMA (below 16 MB on x86-64).
pub const __GFP_DMA: GfpFlags = 1 << 0;
/// Allocate from ZONE_HIGHMEM.
pub const __GFP_HIGHMEM: GfpFlags = 1 << 1;
/// Allocate from ZONE_DMA32 (below 4 GB).
pub const __GFP_DMA32: GfpFlags = 1 << 2;
/// Allocate from ZONE_MOVABLE.
pub const __GFP_MOVABLE: GfpFlags = 1 << 3;

// --- Reclaimability hint (bit 4) ---
pub const __GFP_RECLAIMABLE: GfpFlags = 1 << 4;

// --- Watermark modifier (bit 5) ---

/// Access emergency reserves (atomic allocations).
pub const __GFP_HIGH: GfpFlags = 1 << 5;

// --- Reclaim modifiers (bits 6-7) ---

/// Can start physical I/O during reclaim.
pub const __GFP_IO: GfpFlags = 1 << 6;
/// Can call into filesystem during reclaim.
pub const __GFP_FS: GfpFlags = 1 << 7;

// --- Action modifiers (bit 8) ---

/// Return zeroed page(s).
pub const __GFP_ZERO: GfpFlags = 1 << 8;

// Bit 9 is unused in Linux (___GFP_UNUSED_BIT).

// --- Reclaim control (bits 10-11) ---

/// Caller can perform direct reclaim.
pub const __GFP_DIRECT_RECLAIM: GfpFlags = 1 << 10;
/// kswapd can be woken to reclaim.
pub const __GFP_KSWAPD_RECLAIM: GfpFlags = 1 << 11;

/// Shorthand: can reclaim via either direct or kswapd path.
pub const __GFP_RECLAIM: GfpFlags = __GFP_DIRECT_RECLAIM | __GFP_KSWAPD_RECLAIM;

// --- Additional modifiers (bits 12+) ---

/// Indicate IO-bound allocation.
pub const __GFP_WRITE: GfpFlags = 1 << 12;
/// Suppress allocation failure warnings.
pub const __GFP_NOWARN: GfpFlags = 1 << 13;
/// Retry allocation, may fail.
pub const __GFP_RETRY_MAYFAIL: GfpFlags = 1 << 14;
/// Must not fail — retry infinitely.
pub const __GFP_NOFAIL: GfpFlags = 1 << 15;
/// Don't retry after first failure.
pub const __GFP_NORETRY: GfpFlags = 1 << 16;
/// Access all memory reserves (memalloc context).
pub const __GFP_MEMALLOC: GfpFlags = 1 << 17;
/// Return compound (head + tail) page.
pub const __GFP_COMP: GfpFlags = 1 << 18;
/// Don't use emergency reserves.
pub const __GFP_NOMEMALLOC: GfpFlags = 1 << 19;
/// Enforce cpuset memory policy.
pub const __GFP_HARDWALL: GfpFlags = 1 << 20;
/// Allocate from this NUMA node only.
pub const __GFP_THISNODE: GfpFlags = 1 << 21;
/// Account allocation to kmemcg.
pub const __GFP_ACCOUNT: GfpFlags = 1 << 22;
/// Zero memory tags along with content.
pub const __GFP_ZEROTAGS: GfpFlags = 1 << 23;

// ---------------------------------------------------------------------------
// Compound GFP flag combinations
//
// Ref: Linux gfp_types.h:376-389
// ---------------------------------------------------------------------------

/// Atomic context — cannot sleep, access emergency reserves.
pub const GFP_ATOMIC: GfpFlags = __GFP_HIGH | __GFP_KSWAPD_RECLAIM;
/// Normal kernel allocation — can sleep, can reclaim.
pub const GFP_KERNEL: GfpFlags = __GFP_RECLAIM | __GFP_IO | __GFP_FS;
/// Kernel allocation with kmemcg accounting.
pub const GFP_KERNEL_ACCOUNT: GfpFlags = GFP_KERNEL | __GFP_ACCOUNT;
/// Non-blocking, non-sleeping allocation.
pub const GFP_NOWAIT: GfpFlags = __GFP_KSWAPD_RECLAIM | __GFP_NOWARN;
/// Reclaim without starting physical I/O.
pub const GFP_NOIO: GfpFlags = __GFP_RECLAIM;
/// Reclaim without filesystem callbacks.
pub const GFP_NOFS: GfpFlags = __GFP_RECLAIM | __GFP_IO;
/// Userspace allocation (kernel-accessible, cpuset-enforced).
pub const GFP_USER: GfpFlags = __GFP_RECLAIM | __GFP_IO | __GFP_FS | __GFP_HARDWALL;
/// DMA zone allocation.
pub const GFP_DMA: GfpFlags = __GFP_DMA;
/// DMA32 zone allocation.
pub const GFP_DMA32: GfpFlags = __GFP_DMA32;
/// Userspace allocation from ZONE_HIGHMEM.
pub const GFP_HIGHUSER: GfpFlags = GFP_USER | __GFP_HIGHMEM;
/// Movable userspace allocation from ZONE_HIGHMEM.
pub const GFP_HIGHUSER_MOVABLE: GfpFlags = GFP_HIGHUSER | __GFP_MOVABLE;

/// Bitmask for zone-selecting flags.
pub const GFP_ZONEMASK: GfpFlags = __GFP_DMA | __GFP_HIGHMEM | __GFP_DMA32 | __GFP_MOVABLE;

#[allow(non_snake_case)]
pub fn PageUptodate(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).test_flag(PG_UPTODATE) }
}

#[allow(non_snake_case)]
pub fn SetPageUptodate(page: *const Page) {
    if !page.is_null() {
        unsafe { (*page).set_flag(PG_UPTODATE) };
    }
}

#[allow(non_snake_case)]
pub fn __SetPageUptodate(page: *const Page) {
    SetPageUptodate(page)
}

#[allow(non_snake_case)]
pub fn ClearPageUptodate(page: *const Page) {
    if !page.is_null() {
        unsafe { (*page).clear_flag(PG_UPTODATE) };
    }
}

#[allow(non_snake_case)]
pub fn PageHead(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).test_flag(PG_HEAD) }
}

#[allow(non_snake_case)]
pub fn PageTail(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).private & 1 == 1 && !PageHead(page) }
}

#[allow(non_snake_case)]
pub fn PageCompound(page: *const Page) -> bool {
    PageHead(page) || PageTail(page)
}

#[allow(non_snake_case)]
pub fn PageTransCompound(page: *const Page) -> bool {
    PageCompound(page)
}

#[allow(non_snake_case)]
pub fn PageHuge(page: *const Page) -> bool {
    !page.is_null()
        && decode_page_type(unsafe { (*page).page_type.load(Ordering::Relaxed) }) == PGTY_HUGETLB
}

#[allow(non_snake_case)]
pub fn PagePoisoned(_page: *const Page) -> bool {
    false
}

#[allow(non_snake_case)]
pub fn ClearPageCompound(page: *mut Page) {
    if !page.is_null() {
        unsafe {
            (*page).clear_flag(PG_HEAD);
            (*page).private = 0;
        }
    }
}

#[allow(non_snake_case)]
pub fn clear_compound_head(page: *mut Page) {
    ClearPageCompound(page)
}

#[allow(non_snake_case)]
pub fn PageAnon(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).test_flag(PG_SWAPBACKED) && (*page).mapping == 0 }
}

#[allow(non_snake_case)]
pub fn PageAnonExclusive(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).test_flag(PG_OWNER_2) }
}

#[allow(non_snake_case)]
pub fn SetPageAnonExclusive(page: *const Page) {
    if !page.is_null() {
        unsafe { (*page).set_flag(PG_OWNER_2) };
    }
}

#[allow(non_snake_case)]
pub fn ClearPageAnonExclusive(page: *const Page) {
    if !page.is_null() {
        unsafe { (*page).clear_flag(PG_OWNER_2) };
    }
}

#[allow(non_snake_case)]
pub fn __ClearPageAnonExclusive(page: *const Page) {
    ClearPageAnonExclusive(page)
}

#[allow(non_snake_case)]
pub fn PageAnonNotKsm(page: *const Page) -> bool {
    PageAnon(page)
}

pub fn compound_info_has_mask(page: *const Page, mask: usize) -> bool {
    !page.is_null() && unsafe { (*page).private & mask != 0 }
}

pub fn folio_contain_hwpoisoned_page(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).test_flag(PG_HWPOISON) }
}

pub fn folio_has_private(page: *const Page) -> bool {
    !page.is_null() && unsafe { (*page).test_flag(PG_PRIVATE) || (*page).private != 0 }
}

pub fn folio_mark_uptodate(page: *const Page) {
    SetPageUptodate(page)
}

pub fn __folio_mark_uptodate(page: *const Page) {
    SetPageUptodate(page)
}

pub fn _compound_head(page: *mut Page) -> *mut Page {
    if page.is_null() {
        return core::ptr::null_mut();
    }
    if PageTail(page) {
        let head_pfn = unsafe { (*page).private >> 1 };
        crate::mm::buddy::pfn_to_page(head_pfn)
    } else {
        page
    }
}

pub fn compound_order(page: *const Page) -> usize {
    if PageHead(page) {
        unsafe { (*page).private }
    } else {
        0
    }
}

pub fn compound_nr(page: *const Page) -> usize {
    1usize << compound_order(page)
}

pub fn page_ref_count(page: *const Page) -> i32 {
    if page.is_null() {
        0
    } else {
        unsafe { (*page)._refcount.load(Ordering::Relaxed) }
    }
}

pub fn page_count(page: *const Page) -> i32 {
    page_ref_count(page)
}

pub fn set_page_count(page: *const Page, count: i32) {
    __page_ref_set(page, count)
}

pub fn page_ref_add(page: *const Page, value: i32) {
    __page_ref_mod(page, value)
}

pub fn page_ref_sub(page: *const Page, value: i32) {
    __page_ref_mod(page, -value)
}

pub fn page_ref_inc(page: *const Page) {
    page_ref_add(page, 1)
}

pub fn page_ref_dec(page: *const Page) {
    page_ref_sub(page, 1)
}

pub fn page_ref_inc_return(page: *const Page) -> i32 {
    __page_ref_mod_and_return(page, 1)
}

pub fn page_ref_dec_return(page: *const Page) -> i32 {
    __page_ref_mod_and_return(page, -1)
}

pub fn page_ref_dec_and_test(page: *const Page) -> bool {
    __page_ref_mod_and_test(page, -1)
}

pub fn page_ref_sub_and_test(page: *const Page, value: i32) -> bool {
    __page_ref_mod_and_test(page, -value)
}

pub fn page_ref_add_unless_zero(page: *const Page, value: i32) -> bool {
    __page_ref_mod_unless(page, value, 0)
}

pub fn page_ref_freeze(page: *const Page, count: i32) -> bool {
    __page_ref_freeze(page, count)
}

pub fn page_ref_unfreeze(page: *const Page, count: i32) {
    __page_ref_unfreeze(page, count)
}

pub fn __page_ref_set(page: *const Page, value: i32) {
    if !page.is_null() {
        unsafe { (*page)._refcount.store(value, Ordering::Relaxed) };
    }
}

pub fn __page_ref_mod(page: *const Page, value: i32) {
    if !page.is_null() {
        unsafe { (*page)._refcount.fetch_add(value, Ordering::Relaxed) };
    }
}

pub fn __page_ref_mod_and_return(page: *const Page, value: i32) -> i32 {
    if page.is_null() {
        0
    } else {
        unsafe { (*page)._refcount.fetch_add(value, Ordering::Relaxed) + value }
    }
}

pub fn __page_ref_mod_and_test(page: *const Page, value: i32) -> bool {
    __page_ref_mod_and_return(page, value) == 0
}

pub fn __page_ref_mod_unless(page: *const Page, value: i32, unless: i32) -> bool {
    if page.is_null() {
        return false;
    }
    let current = unsafe { (*page)._refcount.load(Ordering::Relaxed) };
    if current == unless {
        false
    } else {
        __page_ref_mod(page, value);
        true
    }
}

pub fn __page_ref_freeze(page: *const Page, count: i32) -> bool {
    if page.is_null() {
        return false;
    }
    unsafe {
        (*page)
            ._refcount
            .compare_exchange(count, 0, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }
}

pub fn __page_ref_unfreeze(page: *const Page, count: i32) {
    __page_ref_set(page, count)
}

pub fn folio_ref_count(folio: *const Page) -> i32 {
    page_ref_count(folio)
}

pub fn folio_set_count(folio: *const Page, count: i32) {
    __page_ref_set(folio, count)
}

pub fn init_page_count(page: *const Page) {
    __page_ref_set(page, 1)
}

pub fn folio_ref_add(folio: *const Page, value: i32) {
    __page_ref_mod(folio, value)
}

pub fn folio_ref_sub(folio: *const Page, value: i32) {
    __page_ref_mod(folio, -value)
}

pub fn folio_ref_inc(folio: *const Page) {
    folio_ref_add(folio, 1)
}

pub fn folio_ref_dec(folio: *const Page) {
    folio_ref_sub(folio, 1)
}

pub fn folio_ref_inc_return(folio: *const Page) -> i32 {
    __page_ref_mod_and_return(folio, 1)
}

pub fn folio_ref_dec_return(folio: *const Page) -> i32 {
    __page_ref_mod_and_return(folio, -1)
}

pub fn folio_ref_sub_return(folio: *const Page, value: i32) -> i32 {
    __page_ref_mod_and_return(folio, -value)
}

pub fn folio_ref_dec_and_test(folio: *const Page) -> bool {
    __page_ref_mod_and_test(folio, -1)
}

pub fn folio_ref_sub_and_test(folio: *const Page, value: i32) -> bool {
    __page_ref_mod_and_test(folio, -value)
}

pub fn folio_ref_add_unless_zero(folio: *const Page, value: i32) -> bool {
    __page_ref_mod_unless(folio, value, 0)
}

pub fn folio_ref_try_add(folio: *const Page, value: i32) -> bool {
    folio_ref_add_unless_zero(folio, value)
}

pub fn folio_try_get(folio: *const Page) -> bool {
    folio_ref_add_unless_zero(folio, 1)
}

pub fn get_page_unless_zero(page: *const Page) -> bool {
    folio_try_get(page)
}

pub fn folio_ref_freeze(folio: *const Page, count: i32) -> bool {
    __page_ref_freeze(folio, count)
}

pub fn folio_ref_unfreeze(folio: *const Page, count: i32) {
    __page_ref_unfreeze(folio, count)
}

pub fn folio_get(folio: *mut Page) -> *mut Page {
    if !folio.is_null() {
        unsafe { (*folio).get_page() };
    }
    folio
}

pub fn folio_get_nontail_page(folio: *mut Page) -> *mut Page {
    if PageTail(folio) {
        core::ptr::null_mut()
    } else {
        folio_get(folio)
    }
}

pub fn __folio_put(folio: *mut Page) {
    let _ = folio_put_testzero(folio);
}

pub fn folio_put(folio: *mut Page) {
    __folio_put(folio)
}

pub fn folio_put_refs(folio: *mut Page, refs: i32) {
    if refs > 0 {
        let _ = __page_ref_mod_and_return(folio, -refs);
    }
}

pub fn folio_put_testzero(folio: *mut Page) -> bool {
    !folio.is_null() && __page_ref_mod_and_test(folio, -1)
}

pub unsafe fn folios_put(folios: *mut *mut Page, nr: usize) {
    if folios.is_null() {
        return;
    }
    for idx in 0..nr {
        let folio = unsafe { *folios.add(idx) };
        folio_put(folio);
    }
}

pub unsafe fn folios_put_refs(folios: *mut *mut Page, refs: *const i32, nr: usize) {
    if folios.is_null() {
        return;
    }
    for idx in 0..nr {
        let folio = unsafe { *folios.add(idx) };
        let count = if refs.is_null() {
            1
        } else {
            unsafe { *refs.add(idx) }
        };
        folio_put_refs(folio, count);
    }
}

pub fn folio_order(folio: *const Page) -> usize {
    compound_order(folio)
}

pub fn folio_large_order(folio: *const Page) -> usize {
    if PageCompound(folio) {
        compound_order(folio)
    } else {
        0
    }
}

pub fn folio_nr_pages(folio: *const Page) -> usize {
    1usize << folio_order(folio)
}

pub fn folio_large_nr_pages(folio: *const Page) -> usize {
    folio_nr_pages(folio)
}

pub fn folio_size(folio: *const Page) -> usize {
    folio_nr_pages(folio) * crate::mm::frame::PAGE_SIZE
}

pub fn folio_shift(folio: *const Page) -> usize {
    crate::arch::x86::mm::paging::PAGE_SHIFT as usize + folio_order(folio)
}

pub fn folio_next(folio: *mut Page) -> *mut Page {
    if folio.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { folio.add(folio_nr_pages(folio)) }
    }
}

pub fn folio_page_idx(folio: *const Page, page: *const Page) -> usize {
    if folio.is_null() || page.is_null() {
        0
    } else {
        unsafe { page.offset_from(folio).max(0) as usize }
    }
}

pub fn folio_pfn(folio: *const Page) -> usize {
    if folio.is_null() || !crate::mm::buddy::page_in_mem_map(folio) {
        0
    } else {
        crate::mm::buddy::page_to_pfn(folio)
    }
}

pub fn folio_address(folio: *const Page) -> *mut u8 {
    let pfn = folio_pfn(folio);
    if pfn == 0 {
        core::ptr::null_mut()
    } else {
        crate::arch::x86::mm::paging::pfn_to_virt(pfn)
    }
}

pub fn lowmem_page_address(page: *const Page) -> *mut u8 {
    folio_address(page)
}

pub fn folio_mapcount(folio: *const Page) -> i32 {
    if folio.is_null() {
        0
    } else {
        unsafe { (*folio)._mapcount().load(Ordering::Relaxed) + 1 }
    }
}

pub fn folio_entire_mapcount(folio: *const Page) -> i32 {
    folio_mapcount(folio)
}

pub fn folio_large_mapcount(folio: *const Page) -> i32 {
    if PageCompound(folio) {
        folio_mapcount(folio)
    } else {
        0
    }
}

pub fn folio_mapped(folio: *const Page) -> bool {
    folio_mapcount(folio) > 0
}

pub fn folio_expected_ref_count(folio: *const Page) -> i32 {
    folio_mapcount(folio).max(1)
}

pub fn folio_has_pincount(_folio: *const Page) -> bool {
    false
}

pub fn folio_maybe_dma_pinned(_folio: *const Page) -> bool {
    false
}

pub fn folio_maybe_mapped_shared(folio: *const Page) -> bool {
    folio_mapcount(folio) > 1
}

pub fn folio_is_longterm_pinnable(_folio: *const Page) -> bool {
    true
}

pub fn folio_is_pfmemalloc(_folio: *const Page) -> bool {
    false
}

pub fn folio_needs_cow_for_dma(_vma: *const u8, folio: *const Page) -> bool {
    folio_maybe_dma_pinned(folio)
}

pub fn folio_nid(_folio: *const Page) -> i32 {
    0
}

pub fn folio_zone(_folio: *const Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn folio_pgdat(_folio: *const Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn folio_last_cpupid(_folio: *const Page) -> i32 {
    -1
}

pub fn folio_xchg_access_time(_folio: *mut Page, value: u64) -> u64 {
    value
}

pub fn folio_use_access_time(_folio: *const Page) -> bool {
    false
}

pub fn folio_reset_order(folio: *mut Page) {
    if !folio.is_null() {
        unsafe {
            (*folio).private = 0;
            (*folio).clear_flag(PG_HEAD);
        }
    }
}

pub fn folio_test_uptodate(folio: *const Page) -> bool {
    PageUptodate(folio)
}

pub fn folio_test_head(folio: *const Page) -> bool {
    PageHead(folio)
}

pub fn folio_test_large(folio: *const Page) -> bool {
    PageCompound(folio)
}

pub fn folio_test_anon(folio: *const Page) -> bool {
    PageAnon(folio)
}

pub fn folio_test_ksm(_folio: *const Page) -> bool {
    false
}

pub fn folio_test_swapcache(folio: *const Page) -> bool {
    !folio.is_null() && unsafe { (*folio).test_flag(PG_SWAPCACHE) }
}

pub fn folio_test_lazyfree(_folio: *const Page) -> bool {
    false
}

pub fn folio_xor_flags_has_waiters(folio: *const Page, mask: u64) -> bool {
    !folio.is_null() && unsafe { ((*folio).flags.load(Ordering::Acquire) ^ mask) & PG_WAITERS != 0 }
}

pub fn set_page_private(page: *mut Page, private: usize) {
    if !page.is_null() {
        unsafe {
            (*page).private = private;
            if private == 0 {
                (*page).clear_flag(PG_PRIVATE);
            } else {
                (*page).set_flag(PG_PRIVATE);
            }
        }
    }
}

pub fn clear_page_pfmemalloc(_page: *mut Page) {}

pub fn is_page_hwpoison(page: *const Page) -> bool {
    folio_contain_hwpoisoned_page(page)
}

pub fn page_has_type(page: *const Page) -> bool {
    !page.is_null() && page_type_has_type(unsafe { (*page).page_type.load(Ordering::Acquire) })
}

pub fn page_mapcount_is_type(page: *const Page, ty: u32) -> bool {
    !page.is_null() && unsafe { (*page).page_type.load(Ordering::Acquire) == ty }
}

pub fn page_has_movable_ops(_page: *const Page) -> bool {
    false
}

pub fn set_compound_head(page: *mut Page, head: *mut Page) {
    if !page.is_null() && !head.is_null() {
        let pfn = if crate::mm::buddy::page_in_mem_map(head) {
            crate::mm::buddy::page_to_pfn(head)
        } else {
            0
        };
        unsafe {
            (*page).private = (pfn << 1) | 1;
        }
    }
}

pub fn page_init_poison(_page: *mut Page, _size: usize) {}

pub fn stable_page_flags(page: *const Page) -> u64 {
    if page.is_null() {
        0
    } else {
        unsafe { (*page).flags.load(Ordering::Acquire) }
    }
}

pub fn page_offline_freeze(page: *const Page) -> bool {
    page_ref_freeze(page, 1)
}

pub fn page_offline_thaw(page: *const Page) {
    page_ref_unfreeze(page, 1)
}

pub fn mark_page_reserved(page: *mut Page) {
    if !page.is_null() {
        unsafe { (*page).set_reserved() };
    }
}

pub fn free_reserved_page(page: *mut Page) {
    if !page.is_null() {
        unsafe {
            (*page).clear_flag(PG_RESERVED);
            (*page)._refcount.store(0, Ordering::Release);
        }
    }
}

pub unsafe fn clear_pages(page: *mut u8, order: u32) {
    if page.is_null() {
        return;
    }
    let bytes = crate::mm::frame::PAGE_SIZE.saturating_mul(1usize << order.min(20));
    unsafe { core::ptr::write_bytes(page, 0, bytes) };
}

// ---------------------------------------------------------------------------
// Zone type  (also defined in zone.rs — re-exported here for gfp_zone())
// ---------------------------------------------------------------------------

/// Memory zone types matching Linux `enum zone_type`.
///
/// Ref: Linux include/linux/mmzone.h — enum zone_type (lines 784-873)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ZoneType {
    /// DMA-capable memory: 0 – 16 MB on x86-64 (ISA DMA limit).
    ZoneDma = 0,
    /// Normal directly-addressable memory: 16 MB+ on x86-64.
    ZoneNormal = 1,
}

/// Number of memory zones.
pub const MAX_NR_ZONES: usize = 2;

// ---------------------------------------------------------------------------
// Migration types
//
// Ref: Linux include/linux/mmzone.h — enum migratetype (lines 64-90)
// ---------------------------------------------------------------------------

/// Page migration type — determines which free list a page sits on.
///
/// For Milestone 7, only `Unmovable` is used; the other variants exist
/// for struct layout compatibility with Linux's `free_list[MIGRATE_TYPES]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MigrateType {
    /// Cannot be moved or reclaimed.
    Unmovable = 0,
    /// Can be moved via page migration.
    Movable = 1,
    /// Can be reclaimed (e.g., page cache).
    Reclaimable = 2,
    /// Sentinel: number of types on per-CPU lists.
    /// Also used as MIGRATE_HIGHATOMIC in Linux.
    PcpTypes = 3,
}

/// Total number of migration types (determines free_list array size).
pub const MIGRATE_TYPES: usize = 4;

// ---------------------------------------------------------------------------
// GFP → zone / migratetype helpers
// ---------------------------------------------------------------------------

/// Select the preferred zone type from GFP flags.
///
/// Mirrors Linux's `gfp_zone()` logic from include/linux/gfp.h.
/// Simplified for our two-zone setup:
/// - `__GFP_DMA` → `ZoneDma`
/// - Everything else → `ZoneNormal`
#[inline]
pub fn gfp_zone(gfp: GfpFlags) -> ZoneType {
    if gfp & __GFP_DMA != 0 {
        ZoneType::ZoneDma
    } else {
        ZoneType::ZoneNormal
    }
}

/// Select the migration type from GFP flags.
///
/// Reclaimable allocations are grouped separately from movable pages; all
/// other allocations use the unmovable free lists.
#[inline]
pub fn gfp_migratetype(gfp: GfpFlags) -> MigrateType {
    if gfp & __GFP_RECLAIMABLE != 0 {
        MigrateType::Reclaimable
    } else if gfp & __GFP_MOVABLE != 0 {
        MigrateType::Movable
    } else {
        MigrateType::Unmovable
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify page flag bit positions match Linux enum pageflags exactly.
    ///
    /// Linux's `enum pageflags` is an auto-incrementing C enum starting at 0:
    ///   PG_locked=0, PG_writeback=1, PG_referenced=2, ...
    ///
    /// Ref: vendor/linux/include/linux/page-flags.h:93-130
    #[test]
    fn page_flag_bit_positions_match_linux() {
        // Each constant should be 1 << N where N is the enum ordinal.
        assert_eq!(PG_LOCKED, 1 << 0, "PG_locked must be bit 0");
        assert_eq!(PG_WRITEBACK, 1 << 1, "PG_writeback must be bit 1");
        assert_eq!(PG_REFERENCED, 1 << 2, "PG_referenced must be bit 2");
        assert_eq!(PG_UPTODATE, 1 << 3, "PG_uptodate must be bit 3");
        assert_eq!(PG_DIRTY, 1 << 4, "PG_dirty must be bit 4");
        assert_eq!(PG_LRU, 1 << 5, "PG_lru must be bit 5");
        assert_eq!(PG_HEAD, 1 << 6, "PG_head must be bit 6 (mandatory)");
        assert_eq!(PG_WAITERS, 1 << 7, "PG_waiters must be bit 7");
        assert_eq!(PG_ACTIVE, 1 << 8, "PG_active must be bit 8");
        assert_eq!(PG_RESERVED, 1 << 13, "PG_reserved must be bit 13");
    }

    /// Verify page type values match Linux enum pagetype.
    ///
    /// Ref: vendor/linux/include/linux/page-flags.h:925-939
    #[test]
    fn page_type_values_match_linux() {
        assert_eq!(PGTY_BUDDY, 0xf0);
        assert_eq!(PGTY_OFFLINE, 0xf1);
        assert_eq!(PGTY_TABLE, 0xf2);
        assert_eq!(PGTY_GUARD, 0xf3);
        assert_eq!(PGTY_HUGETLB, 0xf4);
        assert_eq!(PGTY_SLAB, 0xf5);
        assert_eq!(PGTY_ZSMALLOC, 0xf6);
        assert_eq!(PGTY_UNACCEPTED, 0xf7);
        assert_eq!(PGTY_LARGE_KMALLOC, 0xf8);
        assert_eq!(PGTY_MAPCOUNT_UNDERFLOW, 0xff);
    }

    /// Verify encode/decode round-trip for page types.
    #[test]
    fn page_type_encode_decode_roundtrip() {
        let encoded = encode_page_type(PGTY_BUDDY);
        assert_eq!(encoded, 0xf000_0000);
        assert!(page_type_has_type(encoded));
        assert_eq!(decode_page_type(encoded), PGTY_BUDDY);

        // PAGE_TYPE_NONE (all 1s) should NOT be typed.
        assert!(!page_type_has_type(PAGE_TYPE_NONE));
    }

    /// Verify GFP_KERNEL is composed of __GFP_RECLAIM | __GFP_IO | __GFP_FS.
    ///
    /// Ref: vendor/linux/include/linux/gfp_types.h:377
    #[test]
    fn gfp_kernel_composition() {
        assert_eq!(GFP_KERNEL, __GFP_RECLAIM | __GFP_IO | __GFP_FS);
        // __GFP_RECLAIM itself is DIRECT_RECLAIM | KSWAPD_RECLAIM
        assert_eq!(__GFP_RECLAIM, __GFP_DIRECT_RECLAIM | __GFP_KSWAPD_RECLAIM);
    }

    /// Verify GFP_ATOMIC is composed of __GFP_HIGH | __GFP_KSWAPD_RECLAIM.
    ///
    /// Ref: vendor/linux/include/linux/gfp_types.h:376
    #[test]
    fn gfp_atomic_composition() {
        assert_eq!(GFP_ATOMIC, __GFP_HIGH | __GFP_KSWAPD_RECLAIM);
    }

    /// Verify gfp_zone() selects the correct zone for common flag combos.
    #[test]
    fn gfp_zone_selects_correct_zone() {
        assert_eq!(gfp_zone(GFP_DMA), ZoneType::ZoneDma);
        assert_eq!(gfp_zone(GFP_KERNEL), ZoneType::ZoneNormal);
        assert_eq!(gfp_zone(GFP_ATOMIC), ZoneType::ZoneNormal);
        assert_eq!(gfp_zone(GFP_USER), ZoneType::ZoneNormal);
        // DMA flag combined with KERNEL flags should still pick DMA zone
        assert_eq!(gfp_zone(GFP_KERNEL | __GFP_DMA), ZoneType::ZoneDma);
    }

    /// Verify GFP flag bit positions match Linux's auto-incrementing enum.
    ///
    /// The enum starts at bit 0 and increments; bit 9 is unused.
    /// Ref: vendor/linux/include/linux/gfp_types.h:26-59
    #[test]
    fn gfp_flag_bit_positions_match_linux() {
        assert_eq!(__GFP_DMA, 1 << 0, "___GFP_DMA_BIT = 0");
        assert_eq!(__GFP_HIGHMEM, 1 << 1, "___GFP_HIGHMEM_BIT = 1");
        assert_eq!(__GFP_DMA32, 1 << 2, "___GFP_DMA32_BIT = 2");
        assert_eq!(__GFP_MOVABLE, 1 << 3, "___GFP_MOVABLE_BIT = 3");
        assert_eq!(__GFP_RECLAIMABLE, 1 << 4, "___GFP_RECLAIMABLE_BIT = 4");
        assert_eq!(__GFP_HIGH, 1 << 5, "___GFP_HIGH_BIT = 5");
        assert_eq!(__GFP_IO, 1 << 6, "___GFP_IO_BIT = 6");
        assert_eq!(__GFP_FS, 1 << 7, "___GFP_FS_BIT = 7");
        assert_eq!(__GFP_ZERO, 1 << 8, "___GFP_ZERO_BIT = 8");
        // bit 9 is unused
        assert_eq!(
            __GFP_DIRECT_RECLAIM,
            1 << 10,
            "___GFP_DIRECT_RECLAIM_BIT = 10"
        );
        assert_eq!(
            __GFP_KSWAPD_RECLAIM,
            1 << 11,
            "___GFP_KSWAPD_RECLAIM_BIT = 11"
        );
    }

    /// Verify gfp_migratetype maps GFP flags to Linux migration classes.
    #[test]
    fn gfp_migratetype_decodes_linux_classes() {
        assert_eq!(gfp_migratetype(GFP_KERNEL), MigrateType::Unmovable);
        assert_eq!(gfp_migratetype(GFP_ATOMIC), MigrateType::Unmovable);
        assert_eq!(gfp_migratetype(__GFP_MOVABLE), MigrateType::Movable);
        assert_eq!(gfp_migratetype(__GFP_RECLAIMABLE), MigrateType::Reclaimable);
    }
}
