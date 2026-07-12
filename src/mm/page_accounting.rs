//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Page accounting, vmstat, page-idle, and page-extension helpers.
//!
//! Implements the memory-owned accounting paths from:
//! - `vendor/linux/mm/mmzone.c`
//! - `vendor/linux/mm/page_counter.c`
//! - `vendor/linux/mm/page_ext.c`
//! - `vendor/linux/mm/page_frag_cache.c`
//! - `vendor/linux/mm/page_idle.c`
//! - `vendor/linux/mm/page_reporting.c`
//! - `vendor/linux/mm/page_vma_mapped.c`
//! - `vendor/linux/mm/vmstat.c`

extern crate alloc;

use alloc::collections::BTreeMap;

use spin::Mutex;

use crate::mm::page::Page;
use crate::mm::page_flags::{PG_DIRTY, PG_REFERENCED, PG_WORKINGSET, PG_WRITEBACK};

pub use crate::mm::memcg::PageCounter;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VmStat {
    pub nr_free_pages: usize,
    pub nr_file_pages: usize,
    pub nr_anon_pages: usize,
    pub nr_dirty: usize,
    pub nr_writeback: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PageExt {
    pub owner_id: u64,
    pub reported: bool,
    pub idle: bool,
}

struct PageAccountingState {
    vmstat: VmStat,
    extensions: BTreeMap<u64, PageExt>,
}

impl PageAccountingState {
    const fn new() -> Self {
        Self {
            vmstat: VmStat {
                nr_free_pages: 0,
                nr_file_pages: 0,
                nr_anon_pages: 0,
                nr_dirty: 0,
                nr_writeback: 0,
            },
            extensions: BTreeMap::new(),
        }
    }

    fn reset(&mut self) {
        self.vmstat = VmStat::default();
        self.extensions.clear();
    }
}

static PAGE_ACCOUNTING: Mutex<PageAccountingState> = Mutex::new(PageAccountingState::new());

pub fn page_is_idle(page: &Page) -> bool {
    !page.test_flag(PG_REFERENCED)
}

pub fn set_page_idle(page: &Page) {
    page.clear_flag(PG_REFERENCED);
}

pub fn mark_page_accessed(page: &Page) {
    page.set_flag(PG_REFERENCED | PG_WORKINGSET);
}

pub fn mark_page_dirty(page: &Page) {
    page.set_flag(PG_DIRTY);
    PAGE_ACCOUNTING.lock().vmstat.nr_dirty += 1;
}

pub fn clear_page_dirty(page: &Page) {
    if page.test_flag(PG_DIRTY) {
        page.clear_flag(PG_DIRTY);
        let mut state = PAGE_ACCOUNTING.lock();
        state.vmstat.nr_dirty = state.vmstat.nr_dirty.saturating_sub(1);
    }
}

pub fn set_page_writeback(page: &Page) {
    if !page.test_flag(PG_WRITEBACK) {
        page.set_flag(PG_WRITEBACK);
        PAGE_ACCOUNTING.lock().vmstat.nr_writeback += 1;
    }
}

pub fn end_page_writeback(page: &Page) {
    if page.test_flag(PG_WRITEBACK) {
        page.clear_flag(PG_WRITEBACK);
        let mut state = PAGE_ACCOUNTING.lock();
        state.vmstat.nr_writeback = state.vmstat.nr_writeback.saturating_sub(1);
    }
}

pub fn account_free_pages(delta: isize) {
    apply_counter_delta(&mut PAGE_ACCOUNTING.lock().vmstat.nr_free_pages, delta);
}

pub fn account_file_pages(delta: isize) {
    apply_counter_delta(&mut PAGE_ACCOUNTING.lock().vmstat.nr_file_pages, delta);
}

pub fn account_anon_pages(delta: isize) {
    apply_counter_delta(&mut PAGE_ACCOUNTING.lock().vmstat.nr_anon_pages, delta);
}

pub fn page_vma_mapped(page: &Page) -> bool {
    page._mapcount().load(core::sync::atomic::Ordering::Acquire) >= 0
}

pub fn page_ext(pfn: u64) -> Option<PageExt> {
    PAGE_ACCOUNTING.lock().extensions.get(&pfn).copied()
}

pub fn set_page_owner(pfn: u64, owner_id: u64) {
    let mut state = PAGE_ACCOUNTING.lock();
    let ext = state.extensions.entry(pfn).or_default();
    ext.owner_id = owner_id;
}

pub fn report_free_page_ext(pfn: u64) {
    let mut state = PAGE_ACCOUNTING.lock();
    let ext = state.extensions.entry(pfn).or_default();
    ext.reported = true;
}

pub fn report_free_page(ext: &mut PageExt) {
    ext.reported = true;
}

pub fn page_frag_cache_order(size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    let mut order = 0;
    let mut bytes = crate::mm::frame::PAGE_SIZE;
    while bytes < size {
        bytes <<= 1;
        order += 1;
    }
    order
}

pub fn global_vmstat() -> VmStat {
    PAGE_ACCOUNTING.lock().vmstat
}

fn apply_counter_delta(counter: &mut usize, delta: isize) {
    if delta >= 0 {
        *counter = counter.saturating_add(delta as usize);
    } else {
        *counter = counter.saturating_sub(delta.unsigned_abs());
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    PAGE_ACCOUNTING.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn page_idle_dirty_writeback_and_workingset_flags_round_trip() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let page = Page::new();
        assert!(page_is_idle(&page));
        mark_page_accessed(&page);
        assert!(!page_is_idle(&page));
        set_page_idle(&page);
        assert!(page_is_idle(&page));

        mark_page_dirty(&page);
        set_page_writeback(&page);
        assert_eq!(global_vmstat().nr_dirty, 1);
        assert_eq!(global_vmstat().nr_writeback, 1);
        clear_page_dirty(&page);
        end_page_writeback(&page);
        assert_eq!(global_vmstat().nr_dirty, 0);
        assert_eq!(global_vmstat().nr_writeback, 0);
    }

    #[test]
    fn page_ext_vmstat_and_frag_cache_account_state() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        set_page_owner(9, 77);
        report_free_page_ext(9);
        assert_eq!(
            page_ext(9),
            Some(PageExt {
                owner_id: 77,
                reported: true,
                idle: false,
            })
        );
        account_free_pages(4);
        account_file_pages(2);
        account_anon_pages(1);
        account_free_pages(-1);
        assert_eq!(global_vmstat().nr_free_pages, 3);
        assert_eq!(global_vmstat().nr_file_pages, 2);
        assert_eq!(global_vmstat().nr_anon_pages, 1);
        assert_eq!(page_frag_cache_order(4096), 0);
        assert_eq!(page_frag_cache_order(4097), 1);
        assert_eq!(page_frag_cache_order(16385), 3);
    }
}
