//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Page reclaim and `drop_caches`-style pressure helpers.
///
/// This module implements the Milestone 16 reclaim subset on top of the
/// Rust page cache and LRU:
/// - shrink active/inactive file LRUs
/// - reclaim clean pages in LRU order
/// - flush dirty pages before forced cache drops
/// - run registered shrinkers alongside page-cache reclaim
///
/// Ref: Linux `mm/vmscan.c`
extern crate alloc;

#[cfg(any(test, feature = "test-zswap-pressure"))]
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::mm::page_flags::GFP_KERNEL;

use super::address_space::{try_lock_page, unlock_page};
use super::filemap::filemap_remove_folio;
use super::lru::{
    LruList, isolate_lru_pages, lru_add_drain, lru_len, putback_page, remove_lru_page,
    total_lru_pages,
};
use super::page::Page;
use super::page_flags::{
    PG_ACTIVE, PG_DIRTY, PG_RECLAIM, PG_REFERENCED, PG_SWAPBACKED, PG_WRITEBACK, folio_mapped,
};
use super::shrinker;
use super::swap::{SwpEntry, add_to_swap, total_swap_pages};
use super::writeback::{flush_all_dirty_pages, writeback_one_page};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanControl {
    pub nr_to_reclaim: usize,
    pub may_writepage: bool,
    pub may_unmap: bool,
    pub may_swap: bool,
    pub priority: u8,
}

impl Default for ScanControl {
    fn default() -> Self {
        Self {
            nr_to_reclaim: 1,
            may_writepage: true,
            may_unmap: true,
            may_swap: false,
            priority: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReclaimStats {
    pub scanned: usize,
    pub reclaimed: usize,
    pub dirty: usize,
    pub writeback: usize,
    pub activated: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DropCachesResult {
    pub written: usize,
    pub reclaimed: usize,
    pub slab_freed: usize,
}

unsafe fn age_active_anon_pages(nr_to_scan: usize) {
    if nr_to_scan == 0 {
        return;
    }

    let mut pages = Vec::new();
    unsafe { isolate_lru_pages(LruList::ActiveAnon, nr_to_scan, &mut pages) };
    for page in pages {
        if page.is_null() {
            continue;
        }
        let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
        if (flags & PG_REFERENCED) != 0 {
            unsafe {
                (&*page).flags.fetch_and(!PG_REFERENCED, Ordering::Relaxed);
                (&*page).flags.fetch_or(PG_ACTIVE, Ordering::Relaxed);
            }
        } else {
            unsafe {
                (&*page).flags.fetch_and(!PG_ACTIVE, Ordering::Relaxed);
            }
        }
        unsafe { putback_page(page) };
    }
}

unsafe fn age_active_file_pages(nr_to_scan: usize) {
    if nr_to_scan == 0 {
        return;
    }

    let mut pages = Vec::new();
    unsafe { isolate_lru_pages(LruList::ActiveFile, nr_to_scan, &mut pages) };
    for page in pages {
        if page.is_null() {
            continue;
        }
        let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
        if (flags & PG_REFERENCED) != 0 {
            unsafe {
                (&*page).flags.fetch_and(!PG_REFERENCED, Ordering::Relaxed);
                (&*page).flags.fetch_or(PG_ACTIVE, Ordering::Relaxed);
            }
        } else {
            unsafe {
                (&*page).flags.fetch_and(!PG_ACTIVE, Ordering::Relaxed);
            }
        }
        unsafe { putback_page(page) };
    }
}

unsafe fn shrink_page_list(pages: Vec<*mut Page>, sc: &ScanControl) -> ReclaimStats {
    let mut stats = ReclaimStats {
        scanned: pages.len(),
        ..ReclaimStats::default()
    };

    for page in pages {
        if page.is_null() {
            continue;
        }

        if !unsafe { try_lock_page(page) } {
            unsafe { putback_page(page) };
            continue;
        }

        let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
        if (flags & PG_REFERENCED) != 0 {
            unsafe {
                (&*page).flags.fetch_and(!PG_REFERENCED, Ordering::Relaxed);
                (&*page).flags.fetch_or(PG_ACTIVE, Ordering::Relaxed);
                unlock_page(page);
                putback_page(page);
            }
            stats.activated += 1;
            continue;
        }

        if (flags & PG_WRITEBACK) != 0 {
            unsafe {
                (&*page).flags.fetch_or(PG_RECLAIM, Ordering::Relaxed);
                unlock_page(page);
                putback_page(page);
            }
            stats.writeback += 1;
            continue;
        }

        // Anonymous/swap-backed page path (M17): swap out before reclaiming
        // the frame.  Fresh anonymous pages store their AnonVma pointer in
        // page.mapping for reverse mapping, so PG_SWAPBACKED is the
        // authoritative discriminator here; requiring mapping == 0 would send
        // normal anonymous pages through file-cache reclaim.
        let is_anon = (flags & PG_SWAPBACKED) != 0;

        if is_anon {
            if !sc.may_swap {
                unsafe {
                    unlock_page(page);
                    putback_page(page);
                }
                stats.dirty += 1;
                continue;
            }
            // Write page to swap backing store.
            if !unsafe { add_to_swap(page) } {
                // No swap space available.
                unsafe {
                    unlock_page(page);
                    putback_page(page);
                }
                stats.dirty += 1;
                continue;
            }
            // Replace all PTEs with swap PTEs.
            let entry = SwpEntry {
                val: unsafe { (*page).index },
            };
            unsafe { crate::mm::rmap::try_to_unmap(page, entry, 0) };
            unsafe { unlock_page(page) };
            // Remove from LRU and return frame to buddy (swap cache holds a ref).
            unsafe { remove_lru_page(page) };
            stats.reclaimed += 1;
            continue;
        }

        // File-backed page path: reclaim can only drop pages that are no
        // longer mapped into userspace.  File-backed rmap/PTE unmapping is not
        // implemented here yet, so keep mapped pages on the LRU instead of
        // removing them from the page cache and potentially freeing a frame
        // that userspace can still reach.
        if folio_mapped(page) {
            unsafe {
                unlock_page(page);
                putback_page(page);
            }
            continue;
        }

        if (flags & PG_DIRTY) != 0 {
            stats.dirty += 1;
            if !sc.may_writepage {
                unsafe {
                    unlock_page(page);
                    putback_page(page);
                }
                continue;
            }

            unsafe {
                unlock_page(page);
            }
            let _ = unsafe { writeback_one_page(page) };
            let after = unsafe { (&*page).flags.load(Ordering::Acquire) };
            if (after & (PG_DIRTY | PG_WRITEBACK)) != 0 {
                unsafe { putback_page(page) };
                continue;
            }
        } else {
            unsafe {
                unlock_page(page);
            }
        }

        unsafe { filemap_remove_folio(page) };
        stats.reclaimed += 1;
    }

    stats
}

pub fn shrink_lruvec(sc: &ScanControl) -> ReclaimStats {
    lru_add_drain();

    // Age active file pages into inactive list if needed.
    if lru_len(LruList::InactiveFile) < sc.nr_to_reclaim && lru_len(LruList::ActiveFile) != 0 {
        unsafe {
            age_active_file_pages(sc.nr_to_reclaim.max(1));
        }
    }

    // Age active anon pages into inactive list if swapping is enabled.
    if sc.may_swap
        && lru_len(LruList::InactiveAnon) < sc.nr_to_reclaim
        && lru_len(LruList::ActiveAnon) != 0
    {
        unsafe { age_active_anon_pages(sc.nr_to_reclaim.max(1)) };
    }

    let mut isolated = Vec::new();
    unsafe {
        isolate_lru_pages(
            LruList::InactiveFile,
            sc.nr_to_reclaim.max(1),
            &mut isolated,
        );
        // Also scan inactive anon pages when swapping is allowed.
        if sc.may_swap {
            isolate_lru_pages(
                LruList::InactiveAnon,
                sc.nr_to_reclaim.max(1),
                &mut isolated,
            );
        }
        shrink_page_list(isolated, sc)
    }
}

pub fn reclaim_pages(nr_to_reclaim: usize) -> ReclaimStats {
    // PSI: account time spent in memory reclaim (Milestone 18).
    let psi_cookie = crate::mm::psi::psi_memstall_enter();
    let sc = ScanControl {
        nr_to_reclaim,
        // Enable swap eviction when at least one swap device is available.
        may_swap: total_swap_pages() > 0,
        ..ScanControl::default()
    };
    let mut stats = shrink_lruvec(&sc);
    stats.reclaimed += shrinker::shrink_slab(GFP_KERNEL, sc.priority as usize);
    crate::mm::psi::psi_memstall_leave(psi_cookie);
    stats
}

pub fn drop_caches(mask: u32) -> DropCachesResult {
    let mut result = DropCachesResult::default();

    if (mask & 1) != 0 {
        result.written = flush_all_dirty_pages() as usize;
        loop {
            let before = total_lru_pages();
            if before == 0 {
                break;
            }

            let pass = shrink_lruvec(&ScanControl {
                nr_to_reclaim: before.max(1),
                may_writepage: false,
                may_unmap: true,
                may_swap: false,
                priority: 0,
            });
            result.reclaimed += pass.reclaimed;

            let after = total_lru_pages();
            if after == before || after == 0 {
                break;
            }
        }
    }

    if (mask & 2) != 0 {
        result.slab_freed = shrinker::shrink_slab(GFP_KERNEL, 0);
    }

    result
}

#[cfg(any(test, feature = "test-zswap-pressure"))]
pub const ZSWAP_PRESSURE_RECLAIM_PAGES: usize = 4;

#[cfg(any(test, feature = "test-zswap-pressure"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZswapPressureSmokeResult {
    pub reclaimed: usize,
    pub stored_pages: usize,
}

#[cfg(any(test, feature = "test-zswap-pressure"))]
fn alloc_pressure_page() -> *mut Page {
    let mut page = Box::new(Page::new());
    unsafe { page.init_lru() };
    Box::into_raw(page)
}

#[cfg(any(test, feature = "test-zswap-pressure"))]
unsafe fn free_pressure_page(page: *mut Page) {
    unsafe {
        drop(Box::from_raw(page));
    }
}

/// Release/QEMU smoke for boot tracker #31.
///
/// Builds a tiny inactive-anonymous working set, enables swap, forces reclaim,
/// verifies the bytes came back from zswap, and then tears the swap slots down.
/// This is intentionally the same path as Linux's frontswap/zswap reclaim flow:
/// `reclaim_pages` -> `add_to_swap` -> `swap_writepage` -> `zswap_store`.
#[cfg(any(test, feature = "test-zswap-pressure"))]
pub fn run_zswap_pressure_smoke() -> ZswapPressureSmokeResult {
    crate::mm::zswap::init();
    let swap_type = crate::mm::swap::swapon(64, 0).expect("zswap-pressure: swapon");

    let mut pages = Vec::new();
    let mut backing = Vec::new();
    for idx in 0..ZSWAP_PRESSURE_RECLAIM_PAGES as u8 {
        let page = alloc_pressure_page();
        let mut bytes = Box::new([0u8; crate::mm::frame::PAGE_SIZE]);
        bytes.fill(idx.wrapping_mul(17).wrapping_add(3));
        unsafe {
            (*page).private = bytes.as_mut_ptr() as usize;
            (&*page)
                .flags
                .fetch_or(crate::mm::page_flags::PG_SWAPBACKED, Ordering::Relaxed);
            crate::mm::lru::lru_cache_add(page);
        }
        pages.push(page);
        backing.push(bytes);
    }
    crate::mm::lru::lru_add_drain();

    let stats = reclaim_pages(ZSWAP_PRESSURE_RECLAIM_PAGES);
    assert_eq!(
        stats.reclaimed, ZSWAP_PRESSURE_RECLAIM_PAGES,
        "zswap-pressure: reclaim did not evict every page"
    );
    let stored_pages = crate::mm::zswap::zswap_total_pages();
    assert_eq!(
        stored_pages, ZSWAP_PRESSURE_RECLAIM_PAGES,
        "zswap-pressure: zswap did not store every reclaimed page"
    );

    for (page, expected) in pages.iter().copied().zip(backing.iter()) {
        let entry = SwpEntry {
            val: unsafe { (*page).index },
        };
        let mut out = Box::new([0u8; crate::mm::frame::PAGE_SIZE]);
        crate::mm::zswap::zswap_load(entry.swp_type(), entry.swp_offset(), out.as_mut_ptr())
            .expect("zswap-pressure: zswap load");
        assert_eq!(
            &out[..],
            &expected[..],
            "zswap-pressure: restored page bytes differ"
        );
        crate::mm::swap::swap_cache_delete(page);
        crate::mm::swap::free_swap_slot(entry);
        unsafe { free_pressure_page(page) };
    }
    crate::mm::swap::swapoff(swap_type).expect("zswap-pressure: swapoff");

    ZswapPressureSmokeResult {
        reclaimed: stats.reclaimed,
        stored_pages,
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use alloc::boxed::Box;
    use alloc::vec::Vec;

    use super::*;
    use crate::mm::address_space::AddressSpace;
    use crate::mm::buddy::reset_buddy_state_for_test;
    use crate::mm::filemap::{filemap_add_folio, filemap_remove_folio, set_page_dirty};
    use crate::mm::list::ListHead;
    use crate::mm::lru::{lru_add_drain, reset_lru_state_for_test};
    use crate::mm::mm_types::VmAreaStruct;
    use crate::mm::page_flags::GFP_KERNEL;
    use crate::mm::rmap::{anon_vma_prepare, anon_vma_unlink};
    use crate::mm::shrinker::reset_shrinker_state_for_test;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use crate::mm::vm_flags::VM_READ;
    use crate::mm::writeback::reset_writeback_state_for_test;

    fn alloc_test_page() -> *mut Page {
        let mut page = Box::new(Page::new());
        unsafe { page.init_lru() };
        Box::into_raw(page)
    }

    unsafe fn free_test_page(page: *mut Page) {
        unsafe {
            drop(Box::from_raw(page));
        }
    }

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        reset_buddy_state_for_test();
        reset_lru_state_for_test();
        reset_writeback_state_for_test();
        reset_shrinker_state_for_test();
        crate::mm::swap::reset_swap_state_for_test();
        crate::mm::zswap::reset_for_tests();
        guard
    }

    #[test]
    fn shrink_lruvec_evicts_clean_file_pages_in_lru_order() {
        let _guard = test_guard();

        let mut mapping = Box::new(AddressSpace::new());
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let pages: Vec<*mut Page> = (0..3u64)
            .map(|index| {
                let page = alloc_test_page();
                unsafe { filemap_add_folio(mptr, page, index, GFP_KERNEL) };
                page
            })
            .collect();
        lru_add_drain();

        let stats = shrink_lruvec(&ScanControl {
            nr_to_reclaim: 2,
            may_writepage: false,
            may_unmap: true,
            may_swap: false,
            priority: 0,
        });
        assert_eq!(stats.reclaimed, 2);
        assert!(unsafe { (&*mptr).i_pages.xa_load(0).is_none() });
        assert!(unsafe { (&*mptr).i_pages.xa_load(1).is_none() });
        assert!(unsafe { (&*mptr).i_pages.xa_load(2).is_some() });

        unsafe { filemap_remove_folio(pages[2]) };
        for page in pages {
            unsafe { free_test_page(page) };
        }
    }

    #[test]
    fn shrink_lruvec_keeps_mapped_file_pages_on_lru() {
        let _guard = test_guard();

        let mut mapping = Box::new(AddressSpace::new());
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let page = alloc_test_page();
        unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };
        unsafe { (*page)._mapcount().store(0, Ordering::Relaxed) };
        lru_add_drain();

        let stats = shrink_lruvec(&ScanControl {
            nr_to_reclaim: 1,
            may_writepage: true,
            may_unmap: false,
            may_swap: false,
            priority: 0,
        });

        assert_eq!(stats.reclaimed, 0);
        assert!(unsafe { (&*mptr).i_pages.xa_load(0).is_some() });
        assert_eq!(unsafe { (*page).refcount() }, 1);

        unsafe { (*page)._mapcount().store(-1, Ordering::Relaxed) };
        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    #[test]
    fn drop_caches_flushes_dirty_pages_before_reclaim() {
        let _guard = test_guard();

        let mut mapping = Box::new(AddressSpace::new());
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let p0 = alloc_test_page();
        let p1 = alloc_test_page();

        unsafe { filemap_add_folio(mptr, p0, 0, GFP_KERNEL) };
        unsafe { filemap_add_folio(mptr, p1, 1, GFP_KERNEL) };
        unsafe { set_page_dirty(p0) };
        lru_add_drain();

        let result = drop_caches(1);
        assert_eq!(result.written, 1);
        assert_eq!(result.reclaimed, 2);
        assert_eq!(unsafe { (&*mptr).nrpages.load(Ordering::Relaxed) }, 0);

        unsafe { free_test_page(p0) };
        unsafe { free_test_page(p1) };
    }

    #[test]
    fn shrink_lruvec_treats_swapbacked_anon_vma_mapping_as_anon() {
        let _guard = test_guard();
        crate::mm::zswap::init();
        let swap_type = crate::mm::swap::swapon(8, 0).expect("anon-vma reclaim: swapon");

        let mut vma = Box::new(VmAreaStruct::new(0x1000, 0x2000, VM_READ));
        let vma_ptr = vma.as_mut() as *mut VmAreaStruct;
        unsafe { ListHead::init(&mut (*vma_ptr).anon_vma_chain) };
        unsafe { anon_vma_prepare(vma_ptr).expect("anon-vma reclaim: prepare anon_vma") };

        let page = alloc_test_page();
        let mut backing = Box::new([0x5au8; crate::mm::frame::PAGE_SIZE]);
        unsafe {
            (*page).private = backing.as_mut_ptr() as usize;
            (*page).mapping = (*vma_ptr).anon_vma as usize;
            (*page).set_flag(PG_SWAPBACKED);
            crate::mm::lru::lru_cache_add(page);
        }
        lru_add_drain();

        let stats = shrink_lruvec(&ScanControl {
            nr_to_reclaim: 1,
            may_writepage: true,
            may_unmap: true,
            may_swap: true,
            priority: 0,
        });

        assert_eq!(stats.reclaimed, 1);
        assert_eq!(crate::mm::zswap::zswap_total_pages(), 1);

        let entry = SwpEntry {
            val: unsafe { (*page).index },
        };
        crate::mm::swap::swap_cache_delete(page);
        crate::mm::swap::free_swap_slot(entry);
        unsafe { free_test_page(page) };
        unsafe { anon_vma_unlink(vma_ptr) };
        crate::mm::swap::swapoff(swap_type).expect("anon-vma reclaim: swapoff");
    }

    #[test]
    fn reclaim_pages_swaps_inactive_anon_pages_into_zswap_under_pressure() {
        let _guard = test_guard();
        let result = run_zswap_pressure_smoke();
        assert_eq!(result.reclaimed, ZSWAP_PRESSURE_RECLAIM_PAGES);
        assert_eq!(result.stored_pages, ZSWAP_PRESSURE_RECLAIM_PAGES);
    }
}
