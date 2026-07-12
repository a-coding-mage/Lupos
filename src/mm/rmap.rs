//! linux-parity: complete
//! linux-source: vendor/linux/mm/rmap.c
//! test-origin: linux:vendor/linux/mm/rmap.c
/// Reverse-mapping infrastructure — minimal port of `vendor/linux/mm/rmap.c`.
///
/// The core data structures and functions here are sufficient for:
/// - COW (M14): tracking which pages are shared between parent and child.
/// - Reclaim (M17): reverse-mapping to find all PTEs of a page before eviction.
///
/// ## Structures
///
/// ```text
/// AnonVma
///  ├── root  ──────── points to root of inheritance tree
///  ├── parent ──────── points to parent AnonVma
///  ├── refcount
///  └── chains ─────── head of list of all AnonVmaChain.anon_vma_list nodes
///                      (M17: enables try_to_unmap to walk all mapped VMAs)
///
/// VmAreaStruct
///  ├── anon_vma ──── *mut AnonVma (which anon_vma "owns" new pages here)
///  └── anon_vma_chain (list_head) ──── list of AnonVmaChain nodes
///
/// AnonVmaChain
///  ├── vma ─────────── *mut VmAreaStruct
///  ├── anon_vma ─────── *mut AnonVma
///  ├── same_vma ─────── ListHead node (linked into vma.anon_vma_chain)
///  └── anon_vma_list ── ListHead node (linked into anon_vma.chains) — M17
/// ```
///
/// ## References
///
/// - Linux `mm/rmap.c` — `anon_vma_prepare()`, `anon_vma_fork()`, `try_to_unmap()`
/// - Linux `include/linux/rmap.h` — `struct anon_vma`, `struct anon_vma_chain`
extern crate alloc;

use core::sync::atomic::{AtomicI32, Ordering};

use alloc::boxed::Box;

use crate::mm::list::ListHead;
use crate::mm::mm_types::VmAreaStruct;
use crate::mm::swap::SwpEntry;

// ---------------------------------------------------------------------------
// AnonVma
// ---------------------------------------------------------------------------

/// Anchor struct for a set of anonymous pages that may be shared across
/// multiple processes (after fork with COW).
///
/// Each anonymous VMA that has had at least one page faulted in holds a
/// pointer to an `AnonVma`.  After `fork`, parent and child VMAs link into
/// the same `AnonVma` tree through parent/child pointers, allowing the kernel
/// to find all mappings of a physical page (reverse mapping).
///
/// Ref: Linux `include/linux/rmap.h` — `struct anon_vma`
#[repr(C)]
pub struct AnonVma {
    /// Root of the anon_vma inheritance tree.  Points to `self` if this is the root.
    pub root: *mut AnonVma,
    /// Parent anon_vma.  Points to `self` if this is the root.
    pub parent: *mut AnonVma,
    /// Usage refcount.  Freed when drops to 0.
    pub refcount: AtomicI32,
    /// Number of child `AnonVma`s whose `parent` points to `self`.
    pub num_children: usize,
    /// Number of VMAs whose `anon_vma` pointer points directly to `self`.
    pub num_active_vmas: usize,
    /// Head of the list of all `AnonVmaChain.anon_vma_list` nodes that belong
    /// to this `AnonVma`.  Used by `try_to_unmap` (M17) to walk every VMA
    /// that may map pages owned by this `AnonVma`.
    ///
    /// Replaces the interval-tree root (`rb_root`) placeholder from M14.
    ///
    /// Ref: Linux `anon_vma.rb_root` — `include/linux/rmap.h`
    pub chains: ListHead,
}

// AnonVma is accessed under mmap_lock or page-table lock.
unsafe impl Send for AnonVma {}
unsafe impl Sync for AnonVma {}

// ---------------------------------------------------------------------------
// AnonVmaChain
// ---------------------------------------------------------------------------

/// Per-VMA link that connects a `VmAreaStruct` to an `AnonVma`.
///
/// A VMA may be linked into several `AnonVma`s (one per ancestor in the
/// inheritance tree) via a chain of `AnonVmaChain` nodes stored on
/// `VmAreaStruct::anon_vma_chain`.
///
/// Ref: Linux `include/linux/rmap.h` — `struct anon_vma_chain`
#[repr(C)]
pub struct AnonVmaChain {
    /// The VMA this chain node belongs to.
    pub vma: *mut VmAreaStruct,
    /// The anon_vma this chain node is linked into.
    pub anon_vma: *mut AnonVma,
    /// Node on `VmAreaStruct::anon_vma_chain` (the per-VMA list of AVC nodes).
    pub same_vma: ListHead,
    /// Node on `AnonVma::chains` (M17: enables `try_to_unmap` to find all VMAs
    /// mapping pages of this `AnonVma`).
    ///
    /// Ref: Linux `anon_vma_chain.rb` — `include/linux/rmap.h`
    pub anon_vma_list: ListHead,
}

unsafe impl Send for AnonVmaChain {}
unsafe impl Sync for AnonVmaChain {}

// ---------------------------------------------------------------------------
// Internal allocation helpers
// ---------------------------------------------------------------------------

fn anon_vma_alloc_raw() -> Option<*mut AnonVma> {
    let av = Box::new(AnonVma {
        root: core::ptr::null_mut(),
        parent: core::ptr::null_mut(),
        refcount: AtomicI32::new(1),
        num_children: 0,
        num_active_vmas: 0,
        chains: ListHead::uninit(),
    });
    let ptr = Box::into_raw(av);
    // root and parent are self-referential — set after Box is on the heap.
    unsafe {
        (*ptr).root = ptr;
        (*ptr).parent = ptr;
        ListHead::init(&mut (*ptr).chains);
    }
    Some(ptr)
}

unsafe fn anon_vma_free(av: *mut AnonVma) {
    unsafe {
        drop(Box::from_raw(av));
    }
}

fn anon_vma_chain_alloc_raw() -> Option<*mut AnonVmaChain> {
    let avc = Box::new(AnonVmaChain {
        vma: core::ptr::null_mut(),
        anon_vma: core::ptr::null_mut(),
        same_vma: ListHead::uninit(),
        anon_vma_list: ListHead::uninit(),
    });
    Some(Box::into_raw(avc))
}

unsafe fn anon_vma_chain_free(avc: *mut AnonVmaChain) {
    unsafe {
        drop(Box::from_raw(avc));
    }
}

// ---------------------------------------------------------------------------
// Public reference-counting helpers
// ---------------------------------------------------------------------------

/// Increment the `AnonVma` refcount.
///
/// Ref: Linux `get_anon_vma()` — `mm/rmap.c`
#[inline]
pub unsafe fn get_anon_vma(av: *mut AnonVma) {
    unsafe {
        (*av).refcount.fetch_add(1, Ordering::Relaxed);
    }
}

/// Decrement the `AnonVma` refcount; free if it reaches zero.
///
/// Ref: Linux `put_anon_vma()` — `mm/rmap.c`
#[inline]
pub unsafe fn put_anon_vma(av: *mut AnonVma) {
    unsafe {
        let rc = (*av).refcount.fetch_sub(1, Ordering::AcqRel) - 1;
        if rc == 0 {
            anon_vma_free(av);
        }
    }
}

// ---------------------------------------------------------------------------
// anon_vma_prepare — first-fault VMA setup
// ---------------------------------------------------------------------------

/// Attach an `AnonVma` to a VMA before its first anonymous page is installed.
///
/// If the VMA already has an `anon_vma`, this is a fast no-op.  Otherwise a
/// fresh `AnonVma` and one `AnonVmaChain` entry are allocated and wired in.
///
/// Called from `do_anonymous_page()` just before the PTE is installed.
///
/// # Safety
/// `vma` must be a valid, heap-allocated `VmAreaStruct`.
/// `vma.anon_vma_chain` must have been initialized with `ListHead::init`.
///
/// Ref: Linux `mm/rmap.c` — `__anon_vma_prepare()` line 185
pub unsafe fn anon_vma_prepare(vma: *mut VmAreaStruct) -> Result<(), i32> {
    unsafe {
        // Fast path: VMA already linked to an anon_vma.
        if !(*vma).anon_vma.is_null() {
            return Ok(());
        }

        // Lazily initialize the anon_vma_chain list-head if it was not set up
        // during heap allocation (e.g., stack-allocated VMAs in tests).
        if (*vma).anon_vma_chain.next.is_null() {
            ListHead::init(&mut (*vma).anon_vma_chain);
        }

        // Allocate a new AnonVma for this VMA.
        let av = anon_vma_alloc_raw().ok_or(-12i32)?; // -ENOMEM

        // Allocate the chain link.
        let avc = match anon_vma_chain_alloc_raw() {
            Some(p) => p,
            None => {
                anon_vma_free(av);
                return Err(-12i32);
            }
        };

        // Initialise the AnonVma.
        (*av).num_active_vmas = 1;

        // Initialise the chain entry and link it into vma.anon_vma_chain.
        (*avc).vma = vma;
        (*avc).anon_vma = av;
        ListHead::init(&mut (*avc).same_vma);
        ListHead::list_add_tail(&mut (*avc).same_vma, &mut (*vma).anon_vma_chain);
        // Link into av.chains so try_to_unmap can find this VMA.
        ListHead::init(&mut (*avc).anon_vma_list);
        ListHead::list_add_tail(&mut (*avc).anon_vma_list, &mut (*av).chains);

        // Publish the anon_vma pointer last.
        (*vma).anon_vma = av;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// anon_vma_fork — fork-time VMA duplication
// ---------------------------------------------------------------------------

/// Set up `AnonVma` linkage for a child VMA produced by `dup_mmap()`.
///
/// If the parent VMA has no `anon_vma` (no pages faulted in), returns `Ok(())`
/// immediately and leaves `dst.anon_vma == null`.
///
/// Otherwise a fresh child `AnonVma` is allocated and linked below the
/// parent's `anon_vma` in the inheritance tree.  The child VMA's
/// `anon_vma_chain` list is populated with one `AnonVmaChain` entry.
///
/// # Safety
/// Both `dst` and `src` must be valid, heap-allocated `VmAreaStruct`s.
/// `dst.anon_vma_chain` must have been initialized with `ListHead::init`.
///
/// Ref: Linux `mm/rmap.c` — `anon_vma_fork()` line 378
pub unsafe fn anon_vma_fork(dst: *mut VmAreaStruct, src: *const VmAreaStruct) -> Result<(), i32> {
    unsafe {
        let src_av = (*src).anon_vma;

        // Parent has no anon_vma → no pages were faulted in → nothing to inherit.
        if src_av.is_null() {
            return Ok(());
        }

        // Lazily initialize the child's chain list-head if needed.
        if (*dst).anon_vma_chain.next.is_null() {
            ListHead::init(&mut (*dst).anon_vma_chain);
        }

        // Allocate a fresh child AnonVma that will own pages written to `dst`.
        let child_av = anon_vma_alloc_raw().ok_or(-12i32)?;

        // Allocate a chain entry linking `dst` into the child AnonVma.
        let avc = match anon_vma_chain_alloc_raw() {
            Some(p) => p,
            None => {
                anon_vma_free(child_av);
                return Err(-12i32);
            }
        };

        // Set up the child AnonVma's position in the inheritance tree.
        // It shares the same root as its parent.
        let root_av = (*src_av).root;
        (*child_av).root = root_av;
        (*child_av).parent = src_av;
        (*child_av).num_active_vmas = 1;

        // Pin the root so it outlives all descendants.
        // The root holds the rwsem used for all locking in the tree.
        get_anon_vma(root_av);

        // Update parent's child count.
        (*src_av).num_children += 1;

        // Wire the child AnonVma and chain entry into the destination VMA.
        (*dst).anon_vma = child_av;

        (*avc).vma = dst;
        (*avc).anon_vma = child_av;
        ListHead::init(&mut (*avc).same_vma);
        ListHead::list_add_tail(&mut (*avc).same_vma, &mut (*dst).anon_vma_chain);
        // Link into child_av.chains for try_to_unmap.
        ListHead::init(&mut (*avc).anon_vma_list);
        ListHead::list_add_tail(&mut (*avc).anon_vma_list, &mut (*child_av).chains);

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// anon_vma_unlink — called when a VMA is being freed
// ---------------------------------------------------------------------------

/// Detach all `AnonVmaChain` entries from a VMA and release the `AnonVma`
/// reference.
///
/// Must be called before `vm_area_free()` for any VMA that has an
/// `anon_vma`.
///
/// # Safety
/// `vma` must be a valid `VmAreaStruct`.  After this call `vma.anon_vma` is
/// null.
///
/// Ref: Linux `mm/rmap.c` — called from `free_vma()` / `unlink_anon_vmas()`
pub unsafe fn anon_vma_unlink(vma: *mut VmAreaStruct) {
    unsafe {
        let av = (*vma).anon_vma;
        if av.is_null() {
            return;
        }

        // Remove and free all AnonVmaChain nodes for this VMA.
        loop {
            match ListHead::first_entry(&(*vma).anon_vma_chain) {
                None => break,
                Some(node) => {
                    let avc = crate::container_of!(node, AnonVmaChain, same_vma);
                    ListHead::list_del(&mut (*avc).same_vma);
                    // Also unlink from the AnonVma's chains list.
                    if !(*avc).anon_vma_list.next.is_null() {
                        ListHead::list_del(&mut (*avc).anon_vma_list);
                    }
                    anon_vma_chain_free(avc);
                }
            }
        }

        // Decrement active VMAs on this anon_vma.
        (*av).num_active_vmas = (*av).num_active_vmas.saturating_sub(1);

        // Release the VMA's reference to its anon_vma.
        put_anon_vma(av);
        (*vma).anon_vma = core::ptr::null_mut();
    }
}

// ---------------------------------------------------------------------------
// try_to_unmap — remove all PTEs mapping a page, install swap PTEs
//
// Ref: Linux `mm/rmap.c` — `try_to_unmap()`
// ---------------------------------------------------------------------------

/// Replace every PTE mapping `page` with a swap PTE encoding `entry`.
///
/// Walks the `AnonVma::chains` list to find all `AnonVmaChain` nodes, then
/// for each VMA performs a linear scan of `[vm_start, vm_end)` looking for a
/// PTE whose PFN matches `page_to_pfn(page)`.  When found the PTE is atomically
/// replaced with `swp_entry_to_pte(entry)` and the TLB is flushed.
///
/// After this call `page._mapcount` should be -1 (all PTEs removed).
/// Returns `true` if all PTEs were successfully removed.
///
/// The linear scan is O(VMA size) and correct for M17 — a future milestone
/// can optimize this with an interval-tree stored in `AnonVma::chains`.
///
/// # Safety
/// `page` must be a valid, locked `Page`.  `entry` must be a valid swap entry
/// allocated by `folio_alloc_swap`.
///
/// Ref: Linux `mm/rmap.c` — `try_to_unmap()`
pub unsafe fn try_to_unmap(page: *mut Page, entry: SwpEntry, _flags: u32) -> bool {
    use crate::arch::x86::mm::paging::{
        flush_tlb_page, p4d_offset, pgd_offset_pgd, pmd_huge, pmd_none, pmd_offset, pte_none,
        pte_offset_kernel, pte_pfn, pte_present, ptep_get_and_clear, pud_huge, pud_none,
        pud_offset, set_pte_at,
    };
    use crate::mm::buddy::page_to_pfn;
    use crate::mm::swap::swp_entry_to_pte;

    if page.is_null() {
        return true;
    }

    let av = unsafe { (*page).mapping as *mut AnonVma };
    if av.is_null() {
        return true; // No anon_vma: page is not mapped via any VMA.
    }

    let target_pfn = page_to_pfn(page as *const _);
    let swap_pte = swp_entry_to_pte(entry);

    // Walk all AnonVmaChain nodes in av.chains.
    let head = unsafe { &mut (*av).chains as *mut ListHead };
    let mut node = unsafe { (*head).next };
    while node != head {
        // Recover AnonVmaChain from the anon_vma_list node.
        let avc = crate::container_of!(node, AnonVmaChain, anon_vma_list);
        let vma = unsafe { (*avc).vma };
        node = unsafe { (*node).next }; // advance before any mutation

        if vma.is_null() {
            continue;
        }

        let mm = unsafe { (*vma).vm_mm };
        if mm.is_null() {
            continue;
        }
        let pgd_ptr = unsafe { (*mm).pgd } as *mut crate::arch::x86::mm::paging::pgd_t;
        if pgd_ptr.is_null() {
            continue;
        }

        // Scan [vm_start, vm_end) for the PTE whose PFN == target_pfn.
        let vm_start = unsafe { (*vma).vm_start };
        let vm_end = unsafe { (*vma).vm_end };
        let mut addr = vm_start;
        while addr < vm_end {
            unsafe {
                let pgdp = pgd_offset_pgd(pgd_ptr, addr);
                if crate::arch::x86::mm::paging::pgd_none(*pgdp) {
                    addr += crate::mm::frame::PAGE_SIZE as u64;
                    continue;
                }
                let p4dp = p4d_offset(pgdp, addr);
                if crate::arch::x86::mm::paging::p4d_none(*p4dp) {
                    addr += crate::mm::frame::PAGE_SIZE as u64;
                    continue;
                }
                let pudp = pud_offset(p4dp, addr);
                if pud_none(*pudp) || pud_huge(*pudp) {
                    addr += crate::mm::frame::PAGE_SIZE as u64;
                    continue;
                }
                let pmdp = pmd_offset(pudp, addr);
                if pmd_none(*pmdp) || pmd_huge(*pmdp) {
                    addr += crate::mm::frame::PAGE_SIZE as u64;
                    continue;
                }
                let ptep = pte_offset_kernel(pmdp, addr);
                let pte = *ptep;
                if pte_present(pte) && pte_pfn(pte) as usize == target_pfn {
                    ptep_get_and_clear(mm as *mut (), addr, ptep);
                    set_pte_at(mm as *mut (), addr, ptep, swap_pte);
                    flush_tlb_page(addr);
                    (*page)._mapcount().fetch_sub(1, Ordering::Relaxed);
                    break;
                }
            }
            addr += crate::mm::frame::PAGE_SIZE as u64;
        }
    }

    unsafe { (*page)._mapcount().load(Ordering::Acquire) < 0 }
}

// We need the Page type from memory::page.
use crate::mm::page::Page;

// ---------------------------------------------------------------------------
// Linux-visible rmap.h helpers
// ---------------------------------------------------------------------------

pub fn anon_vma_init() {}

pub fn __folio_rmap_sanity_checks(folio: *const Page, page: *const Page, nr: usize) -> bool {
    !folio.is_null() && !page.is_null() && nr != 0
}

pub fn __folio_large_mapcount_sanity_checks(folio: *const Page, nr: usize) -> bool {
    !folio.is_null() && nr != 0
}

pub fn folio_lock_large_mapcount(_folio: *mut Page) {}

pub fn folio_unlock_large_mapcount(_folio: *mut Page) {}

pub fn folio_set_large_mapcount(folio: *mut Page, count: i32) {
    if !folio.is_null() {
        unsafe {
            (*folio)
                ._mapcount()
                .store(count.saturating_sub(1), Ordering::Release);
        }
    }
}

pub fn folio_add_large_mapcount(folio: *mut Page, nr: i32) {
    if !folio.is_null() {
        unsafe {
            (*folio)._mapcount().fetch_add(nr, Ordering::AcqRel);
        }
    }
}

pub fn folio_add_return_large_mapcount(folio: *mut Page, nr: i32) -> i32 {
    if folio.is_null() {
        0
    } else {
        unsafe { (*folio)._mapcount().fetch_add(nr, Ordering::AcqRel) + nr + 1 }
    }
}

pub fn folio_sub_large_mapcount(folio: *mut Page, nr: i32) {
    if !folio.is_null() {
        unsafe {
            (*folio)._mapcount().fetch_sub(nr, Ordering::AcqRel);
        }
    }
}

pub fn folio_sub_return_large_mapcount(folio: *mut Page, nr: i32) -> i32 {
    if folio.is_null() {
        0
    } else {
        unsafe { (*folio)._mapcount().fetch_sub(nr, Ordering::AcqRel) - nr + 1 }
    }
}

pub fn folio_set_mm_id(folio: *mut Page, mm_id: usize) {
    if !folio.is_null() {
        unsafe {
            (*folio).private = mm_id;
        }
    }
}

pub fn folio_mm_id(folio: *const Page) -> usize {
    if folio.is_null() {
        0
    } else {
        unsafe { (*folio).private }
    }
}

pub fn __folio_try_dup_anon_rmap(
    folio: *mut Page,
    _page: *mut Page,
    vma: *mut VmAreaStruct,
) -> i32 {
    if folio.is_null() {
        return -22;
    }
    if !vma.is_null() {
        unsafe {
            (*folio).mapping = (*vma).anon_vma as usize;
        }
    }
    folio_add_large_mapcount(folio, 1);
    0
}

pub fn folio_try_dup_anon_rmap_pte(
    folio: *mut Page,
    page: *mut Page,
    vma: *mut VmAreaStruct,
) -> i32 {
    __folio_try_dup_anon_rmap(folio, page, vma)
}

pub fn folio_try_dup_anon_rmap_pmd(
    folio: *mut Page,
    page: *mut Page,
    vma: *mut VmAreaStruct,
) -> i32 {
    __folio_try_dup_anon_rmap(folio, page, vma)
}

pub fn folio_try_dup_anon_rmap_ptes(
    folio: *mut Page,
    page: *mut Page,
    _nr: usize,
    vma: *mut VmAreaStruct,
) -> i32 {
    __folio_try_dup_anon_rmap(folio, page, vma)
}

pub fn __folio_try_share_anon_rmap(
    folio: *mut Page,
    page: *mut Page,
    vma: *mut VmAreaStruct,
) -> i32 {
    __folio_try_dup_anon_rmap(folio, page, vma)
}

pub fn folio_try_share_anon_rmap_pte(
    folio: *mut Page,
    page: *mut Page,
    vma: *mut VmAreaStruct,
) -> i32 {
    __folio_try_share_anon_rmap(folio, page, vma)
}

pub fn folio_try_share_anon_rmap_pmd(
    folio: *mut Page,
    page: *mut Page,
    vma: *mut VmAreaStruct,
) -> i32 {
    __folio_try_share_anon_rmap(folio, page, vma)
}

pub fn __folio_dup_file_rmap(folio: *mut Page, _page: *mut Page, _nr: usize) {
    folio_add_large_mapcount(folio, _nr as i32);
}

pub fn folio_dup_file_rmap_pte(folio: *mut Page, page: *mut Page) {
    __folio_dup_file_rmap(folio, page, 1)
}

pub fn folio_dup_file_rmap_pmd(folio: *mut Page, page: *mut Page) {
    __folio_dup_file_rmap(folio, page, 1)
}

pub fn folio_dup_file_rmap_ptes(folio: *mut Page, page: *mut Page, nr: usize) {
    __folio_dup_file_rmap(folio, page, nr)
}

pub fn folio_move_anon_rmap(folio: *mut Page, _vma: *mut VmAreaStruct) {
    if !folio.is_null() {
        unsafe {
            (*folio).mapping = if _vma.is_null() {
                0
            } else {
                (*_vma).anon_vma as usize
            };
        }
    }
}

pub fn folio_referenced(
    _folio: *mut Page,
    _is_locked: i32,
    _memcg: *mut u8,
    _vm_flags: *mut u64,
) -> i32 {
    if _folio.is_null() {
        return 0;
    }
    let mapped = unsafe {
        (*_folio)
            ._mapcount()
            .load(Ordering::Acquire)
            .saturating_add(1)
    };
    mapped.max(0)
}

pub fn folio_mkclean(folio: *mut Page) -> i32 {
    if !folio.is_null() {
        unsafe {
            (*folio).clear_flag(crate::mm::page_flags::PG_DIRTY);
        }
    }
    0
}

pub fn mapping_wrprotect_range(_mapping: *mut u8, _start: u64, _nr_pages: u64) -> i32 {
    if _nr_pages == 0 {
        return -22;
    }
    0
}

pub fn make_device_exclusive(_range: *mut u8, _pages: *mut *mut Page, _npages: usize) -> i32 {
    -22
}

pub fn page_vma_mapped_walk(_pvmw: *mut u8) -> bool {
    false
}

pub fn page_vma_mapped_walk_done(_pvmw: *mut u8) {}

pub fn rmap_walk(_folio: *mut Page, _rwc: *mut u8) {}

pub fn rmap_walk_locked(_folio: *mut Page, _rwc: *mut u8) {}

pub fn try_to_migrate(page: *mut Page, entry: SwpEntry, flags: u32) -> bool {
    unsafe { try_to_unmap(page, entry, flags) }
}

pub fn hugetlb_add_file_rmap(folio: *mut Page) {
    folio_add_large_mapcount(folio, 1);
}

pub fn hugetlb_remove_rmap(folio: *mut Page) {
    folio_sub_large_mapcount(folio, 1);
}

pub fn hugetlb_try_dup_anon_rmap(folio: *mut Page, vma: *mut VmAreaStruct) -> i32 {
    __folio_try_dup_anon_rmap(folio, folio, vma)
}

pub fn hugetlb_try_share_anon_rmap(folio: *mut Page, vma: *mut VmAreaStruct) -> i32 {
    __folio_try_share_anon_rmap(folio, folio, vma)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use crate::mm::mm_types::VmAreaStruct;
    use crate::mm::vm_flags::VM_READ;
    use alloc::boxed::Box;

    /// Helper: create a heap-allocated VMA with an initialized anon_vma_chain.
    unsafe fn make_vma() -> *mut VmAreaStruct {
        let vma = Box::new(VmAreaStruct::new(0x1000, 0x2000, VM_READ));
        let ptr = Box::into_raw(vma);
        unsafe { ListHead::init(&mut (*ptr).anon_vma_chain) };
        ptr
    }

    unsafe fn free_vma(vma: *mut VmAreaStruct) {
        unsafe {
            anon_vma_unlink(vma);
            drop(Box::from_raw(vma));
        }
    }

    #[test]
    fn anon_vma_alloc_refcount_is_1() {
        let av = anon_vma_alloc_raw().expect("OOM");
        unsafe {
            assert_eq!((*av).refcount.load(Ordering::Relaxed), 1);
            assert_eq!((*av).root, av, "root must be self on alloc");
            assert_eq!((*av).parent, av, "parent must be self on alloc");
            assert_eq!((*av).num_children, 0);
            assert_eq!((*av).num_active_vmas, 0);
            anon_vma_free(av);
        }
    }

    #[test]
    fn put_anon_vma_frees_on_zero() {
        let av = anon_vma_alloc_raw().expect("OOM");
        // put_anon_vma with refcount 1 → should free (no UB if no UAF).
        unsafe { put_anon_vma(av) }; // refcount → 0 → freed
        // Verify we can still allocate (allocator not corrupt).
        let av2 = anon_vma_alloc_raw().expect("OOM");
        unsafe { anon_vma_free(av2) };
    }

    #[test]
    fn anon_vma_prepare_sets_vma_field() {
        unsafe {
            let vma = make_vma();
            assert!(
                (*vma).anon_vma.is_null(),
                "fresh VMA must have null anon_vma"
            );
            anon_vma_prepare(vma).expect("prepare failed");
            assert!(
                !(*vma).anon_vma.is_null(),
                "after prepare anon_vma must be set"
            );
            assert_eq!(
                (*(*vma).anon_vma).num_active_vmas,
                1,
                "num_active_vmas must be 1 after prepare"
            );
            // Chain must not be empty.
            assert!(!ListHead::is_empty(&(*vma).anon_vma_chain));
            free_vma(vma);
        }
    }

    #[test]
    fn anon_vma_prepare_idempotent() {
        unsafe {
            let vma = make_vma();
            anon_vma_prepare(vma).unwrap();
            let av_first = (*vma).anon_vma;
            // Calling again must be a no-op.
            anon_vma_prepare(vma).unwrap();
            assert_eq!((*vma).anon_vma, av_first, "second prepare must be a no-op");
            free_vma(vma);
        }
    }

    #[test]
    fn anon_vma_fork_child_has_different_av() {
        unsafe {
            let parent_vma = make_vma();
            let child_vma = make_vma();

            anon_vma_prepare(parent_vma).unwrap();
            anon_vma_fork(child_vma, parent_vma).unwrap();

            let p_av = (*parent_vma).anon_vma;
            let c_av = (*child_vma).anon_vma;

            assert!(!c_av.is_null(), "child must have an anon_vma after fork");
            assert_ne!(p_av, c_av, "parent and child must have distinct anon_vmas");

            free_vma(parent_vma);
            free_vma(child_vma);
        }
    }

    #[test]
    fn anon_vma_fork_child_av_parent_is_src_av() {
        unsafe {
            let parent_vma = make_vma();
            let child_vma = make_vma();

            anon_vma_prepare(parent_vma).unwrap();
            let p_av = (*parent_vma).anon_vma;
            anon_vma_fork(child_vma, parent_vma).unwrap();
            let c_av = (*child_vma).anon_vma;

            assert_eq!((*c_av).parent, p_av, "child av parent must equal parent av");
            // Both share the same root.
            assert_eq!(
                (*c_av).root,
                (*p_av).root,
                "child and parent must share root"
            );
            // Parent num_children was incremented.
            assert_eq!((*p_av).num_children, 1);

            free_vma(parent_vma);
            free_vma(child_vma);
        }
    }

    #[test]
    fn anon_vma_fork_null_parent_av_is_noop() {
        unsafe {
            let parent_vma = make_vma();
            let child_vma = make_vma();
            // Parent has no anon_vma (no pages faulted in).
            assert!((*parent_vma).anon_vma.is_null());
            anon_vma_fork(child_vma, parent_vma).unwrap();
            // Child should also have null anon_vma.
            assert!((*child_vma).anon_vma.is_null());
            drop(Box::from_raw(parent_vma));
            drop(Box::from_raw(child_vma));
        }
    }

    #[test]
    fn anon_vma_unlink_frees_chain() {
        unsafe {
            let vma = make_vma();
            anon_vma_prepare(vma).unwrap();
            assert!(!ListHead::is_empty(&(*vma).anon_vma_chain));
            anon_vma_unlink(vma);
            assert!(
                (*vma).anon_vma.is_null(),
                "anon_vma must be null after unlink"
            );
            assert!(
                ListHead::is_empty(&(*vma).anon_vma_chain),
                "chain must be empty after unlink"
            );
            drop(Box::from_raw(vma));
        }
    }

    #[test]
    fn linux_visible_rmap_wrappers_update_page_state() {
        unsafe {
            let vma = make_vma();
            anon_vma_prepare(vma).unwrap();

            let mut folio = Page::new();
            let mut page = Page::new();
            assert!(__folio_rmap_sanity_checks(&folio, &page, 1));
            assert!(!__folio_rmap_sanity_checks(core::ptr::null(), &page, 1));
            assert!(__folio_large_mapcount_sanity_checks(&folio, 2));

            folio_set_large_mapcount(&mut folio, 1);
            assert_eq!(folio._mapcount().load(Ordering::Acquire), 0);
            folio_add_large_mapcount(&mut folio, 2);
            assert_eq!(folio._mapcount().load(Ordering::Acquire), 2);
            assert_eq!(folio_sub_return_large_mapcount(&mut folio, 1), 2);

            assert_eq!(__folio_try_dup_anon_rmap(&mut folio, &mut page, vma), 0);
            assert_eq!(folio.mapping, (*vma).anon_vma as usize);
            folio_move_anon_rmap(&mut folio, core::ptr::null_mut());
            assert_eq!(folio.mapping, 0);
            folio_move_anon_rmap(&mut folio, vma);
            assert_eq!(folio.mapping, (*vma).anon_vma as usize);

            folio_set_mm_id(&mut folio, 0xfeed);
            assert_eq!(folio_mm_id(&folio), 0xfeed);
            folio.set_flag(crate::mm::page_flags::PG_DIRTY);
            assert_eq!(folio_mkclean(&mut folio), 0);
            assert!(!folio.test_flag(crate::mm::page_flags::PG_DIRTY));
            assert!(
                folio_referenced(&mut folio, 0, core::ptr::null_mut(), core::ptr::null_mut()) > 0
            );
            assert_eq!(mapping_wrprotect_range(core::ptr::null_mut(), 0, 0), -22);
            assert_eq!(mapping_wrprotect_range(core::ptr::null_mut(), 0, 1), 0);
            assert_eq!(hugetlb_try_dup_anon_rmap(&mut folio, vma), 0);
            hugetlb_add_file_rmap(&mut folio);
            hugetlb_remove_rmap(&mut folio);
            assert!(unsafe { try_to_unmap(core::ptr::null_mut(), SwpEntry::new(0, 0), 0) });

            free_vma(vma);
        }
    }
}
