//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Dirty tracking and writeback.
///
/// This module ports the writeback subset the Rust page cache needs for
/// Milestone 16:
/// - dirty page accounting
/// - dirty ratio / background ratio thresholds
/// - queued `wb_workfn()` style background flushing
/// - page dirty/writeback flag transitions and XArray tag maintenance
///
/// Ref: Linux `mm/page-writeback.c`
///      Linux `fs/fs-writeback.c`
extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Mutex;

use crate::arch::x86::mm::paging::PAGE_SIZE;

use super::address_space::{AddressSpace, lock_page, unlock_page};
use super::page::Page;
use super::page_flags::{PG_DIRTY, PG_RECLAIM, PG_WRITEBACK};
use super::xarray::XaMark;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum WritebackSyncMode {
    #[default]
    None = 0,
    All = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct WritebackControl {
    pub nr_to_write: isize,
    pub pages_skipped: isize,
    /// Byte offsets, matching Linux `writeback_control`.
    pub range_start: u64,
    pub range_end: u64,
    pub index: u64,
    pub sync_mode: WritebackSyncMode,
    pub tagged_writepages: bool,
    pub for_background: bool,
    pub for_reclaim: bool,
    pub range_cyclic: bool,
}

impl Default for WritebackControl {
    fn default() -> Self {
        Self::new(isize::MAX)
    }
}

impl WritebackControl {
    pub const fn new(nr_to_write: isize) -> Self {
        Self {
            nr_to_write,
            pages_skipped: 0,
            range_start: 0,
            range_end: u64::MAX,
            index: 0,
            sync_mode: WritebackSyncMode::None,
            tagged_writepages: false,
            for_background: false,
            for_reclaim: false,
            range_cyclic: false,
        }
    }

    #[inline]
    fn start_index(&self) -> u64 {
        self.range_start / PAGE_SIZE as u64
    }

    #[inline]
    fn end_index(&self) -> u64 {
        if self.range_end == u64::MAX {
            u64::MAX
        } else {
            self.range_end / PAGE_SIZE as u64
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WritebackWork {
    pub nr_pages: isize,
    pub sync_mode: WritebackSyncMode,
    pub tagged_writepages: bool,
    pub for_background: bool,
    pub range_cyclic: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WritebackStats {
    pub dirty_pages: usize,
    pub writeback_pages: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyThrottleResult {
    pub background_writeback: bool,
    pub throttled: bool,
    pub dirty_pages: usize,
    pub background_thresh: usize,
    pub dirty_thresh: usize,
}

#[derive(Clone, Copy)]
struct WritebackJob {
    mapping: *mut AddressSpace,
    work: WritebackWork,
}

struct WritebackState {
    dirtyable_memory: usize,
    dirty_background_ratio: usize,
    dirty_ratio: usize,
    dirty_pages: usize,
    writeback_pages: usize,
    queued: Vec<WritebackJob>,
    mappings: Vec<usize>,
}

unsafe impl Send for WritebackState {}

impl WritebackState {
    const fn new() -> Self {
        Self {
            dirtyable_memory: 1024,
            dirty_background_ratio: 10,
            dirty_ratio: 20,
            dirty_pages: 0,
            writeback_pages: 0,
            queued: Vec::new(),
            mappings: Vec::new(),
        }
    }

    fn dirty_limits(&self) -> (usize, usize) {
        let dirtyable = self.dirtyable_memory.max(1);
        let background = ((dirtyable * self.dirty_background_ratio) / 100).max(1);
        let dirty = ((dirtyable * self.dirty_ratio) / 100).max(background);
        (background, dirty)
    }

    fn track_mapping(&mut self, mapping: *mut AddressSpace) {
        if mapping.is_null() {
            return;
        }
        let key = mapping as usize;
        if !self.mappings.iter().copied().any(|tracked| tracked == key) {
            self.mappings.push(key);
        }
    }

    fn untrack_mapping(&mut self, mapping: *mut AddressSpace) {
        let key = mapping as usize;
        if let Some(idx) = self.mappings.iter().position(|&tracked| tracked == key) {
            self.mappings.swap_remove(idx);
        }
        self.queued.retain(|job| job.mapping as usize != key);
    }
}

static WRITEBACK_STATE: Mutex<WritebackState> = Mutex::new(WritebackState::new());

pub fn set_dirtyable_memory_pages(pages: usize) {
    WRITEBACK_STATE.lock().dirtyable_memory = pages.max(1);
}

pub fn set_dirty_background_ratio(ratio: usize) {
    WRITEBACK_STATE.lock().dirty_background_ratio = ratio.min(100);
}

pub fn set_dirty_ratio(ratio: usize) {
    WRITEBACK_STATE.lock().dirty_ratio = ratio.min(100);
}

pub fn stats() -> WritebackStats {
    let state = WRITEBACK_STATE.lock();
    WritebackStats {
        dirty_pages: state.dirty_pages,
        writeback_pages: state.writeback_pages,
    }
}

pub fn queued_work_items() -> usize {
    WRITEBACK_STATE.lock().queued.len()
}

pub unsafe fn track_mapping(mapping: *mut AddressSpace) {
    WRITEBACK_STATE.lock().track_mapping(mapping);
}

pub unsafe fn untrack_mapping(mapping: *mut AddressSpace) {
    WRITEBACK_STATE.lock().untrack_mapping(mapping);
}

pub fn snapshot_mappings() -> Vec<*mut AddressSpace> {
    WRITEBACK_STATE
        .lock()
        .mappings
        .iter()
        .copied()
        .map(|mapping| mapping as *mut AddressSpace)
        .collect()
}

pub unsafe fn note_page_dirty(page: *mut Page, newly_dirty: bool) {
    if page.is_null() {
        return;
    }

    unsafe {
        (&*page).flags.fetch_and(!PG_RECLAIM, Ordering::Relaxed);
    }
    let mapping = unsafe { (*page).mapping as *mut AddressSpace };
    if !mapping.is_null() {
        unsafe {
            (*mapping)
                .i_pages
                .xa_set_mark((*page).index as u64, XaMark::Dirty);
        }
        WRITEBACK_STATE.lock().track_mapping(mapping);
    }

    if newly_dirty {
        WRITEBACK_STATE.lock().dirty_pages += 1;
    }
}

pub unsafe fn mark_page_dirty(page: *mut Page) -> bool {
    if page.is_null() {
        return false;
    }

    let old = unsafe { (&*page).flags.fetch_or(PG_DIRTY, Ordering::AcqRel) };
    let newly_dirty = (old & PG_DIRTY) == 0;
    if newly_dirty {
        unsafe { note_page_dirty(page, true) };
    } else {
        unsafe { (&*page).flags.fetch_and(!PG_RECLAIM, Ordering::Relaxed) };
    }
    newly_dirty
}

pub unsafe fn page_cache_remove(page: *mut Page) {
    if page.is_null() {
        return;
    }

    let mapping = unsafe { (*page).mapping as *mut AddressSpace };
    let index = unsafe { (*page).index as u64 };
    let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };

    {
        let mut state = WRITEBACK_STATE.lock();
        if (flags & PG_DIRTY) != 0 {
            state.dirty_pages = state.dirty_pages.saturating_sub(1);
        }
        if (flags & PG_WRITEBACK) != 0 {
            state.writeback_pages = state.writeback_pages.saturating_sub(1);
        }
    }

    if !mapping.is_null() {
        unsafe {
            (*mapping).i_pages.xa_clear_mark(index, XaMark::Dirty);
            (*mapping).i_pages.xa_clear_mark(index, XaMark::Writeback);
            (*mapping).i_pages.xa_clear_mark(index, XaMark::ToWrite);
        }
    }

    unsafe {
        (&*page)
            .flags
            .fetch_and(!(PG_DIRTY | PG_WRITEBACK | PG_RECLAIM), Ordering::Release);
    }
}

pub unsafe fn clear_page_dirty_for_io(page: *mut Page) -> bool {
    if page.is_null() {
        return false;
    }

    let old = unsafe { (&*page).flags.fetch_and(!PG_DIRTY, Ordering::AcqRel) };
    if (old & PG_DIRTY) == 0 {
        return false;
    }

    let mut state = WRITEBACK_STATE.lock();
    state.dirty_pages = state.dirty_pages.saturating_sub(1);
    true
}

pub unsafe fn start_page_writeback(page: *mut Page, keep_write: bool) -> bool {
    if page.is_null() {
        return false;
    }

    let old = unsafe { (&*page).flags.fetch_or(PG_WRITEBACK, Ordering::AcqRel) };
    if (old & PG_WRITEBACK) != 0 {
        return false;
    }

    let mapping = unsafe { (*page).mapping as *mut AddressSpace };
    if !mapping.is_null() {
        let index = unsafe { (*page).index as u64 };
        unsafe {
            (*mapping).i_pages.xa_set_mark(index, XaMark::Writeback);
        }
        if (unsafe { (&*page).flags.load(Ordering::Acquire) } & PG_DIRTY) == 0 {
            unsafe {
                (*mapping).i_pages.xa_clear_mark(index, XaMark::Dirty);
            }
        }
        if !keep_write {
            unsafe {
                (*mapping).i_pages.xa_clear_mark(index, XaMark::ToWrite);
            }
        }
    }

    WRITEBACK_STATE.lock().writeback_pages += 1;
    true
}

pub unsafe fn end_page_writeback(page: *mut Page) {
    if page.is_null() {
        return;
    }

    let old = unsafe { (&*page).flags.fetch_and(!PG_WRITEBACK, Ordering::Release) };
    if (old & PG_WRITEBACK) == 0 {
        return;
    }

    let mapping = unsafe { (*page).mapping as *mut AddressSpace };
    if !mapping.is_null() {
        let index = unsafe { (*page).index as u64 };
        unsafe {
            (*mapping).i_pages.xa_clear_mark(index, XaMark::Writeback);
        }
    }

    let mut state = WRITEBACK_STATE.lock();
    state.writeback_pages = state.writeback_pages.saturating_sub(1);
}

unsafe fn writeback_one_page_locked(page: *mut Page) -> bool {
    if !unsafe { clear_page_dirty_for_io(page) } {
        return false;
    }
    unsafe { start_page_writeback(page, false) };
    unsafe { end_page_writeback(page) };
    true
}

pub unsafe fn writeback_one_page(page: *mut Page) -> bool {
    if page.is_null() {
        return false;
    }

    unsafe { lock_page(page) };
    let wrote = unsafe { writeback_one_page_locked(page) };
    unsafe { unlock_page(page) };
    wrote
}

unsafe fn generic_writepages(mapping: *mut AddressSpace, wbc: *mut WritebackControl) -> isize {
    if mapping.is_null() || wbc.is_null() {
        return 0;
    }

    let wbc_ref = unsafe { &mut *wbc };
    let start = if wbc_ref.range_cyclic {
        unsafe { (&*mapping).writeback_index.load(Ordering::Relaxed) }
    } else {
        wbc_ref.index.max(wbc_ref.start_index())
    };
    let end = wbc_ref.end_index();
    let mark = if wbc_ref.tagged_writepages {
        XaMark::ToWrite
    } else {
        XaMark::Dirty
    };

    let mut phases = [(start, end), (0, 0)];
    let mut nr_phases = 1usize;
    if wbc_ref.range_cyclic && start > 0 {
        phases[1] = (0, start.saturating_sub(1));
        nr_phases = 2;
    }

    let mut written = 0isize;
    let mut next_index = start;

    for (phase_start, phase_end) in phases.into_iter().take(nr_phases) {
        if phase_start > phase_end {
            continue;
        }

        let mut index = phase_start;
        loop {
            if written >= wbc_ref.nr_to_write {
                break;
            }
            let found = unsafe { (&*mapping).i_pages.xa_find(index, phase_end, mark) };
            let (page_idx, page) = match found {
                Some(found) => found,
                None => break,
            };
            let page_ptr = page.as_ptr();
            unsafe { lock_page(page_ptr) };
            if unsafe { writeback_one_page_locked(page_ptr) } {
                written += 1;
            } else {
                wbc_ref.pages_skipped += 1;
            }
            unsafe { unlock_page(page_ptr) };
            next_index = page_idx.saturating_add(1);
            index = next_index;
        }
    }

    unsafe {
        (&*mapping)
            .writeback_index
            .store(next_index, Ordering::Relaxed);
    }
    wbc_ref.nr_to_write -= written;
    written
}

unsafe fn run_writeback_job(job: WritebackJob) -> isize {
    if job.mapping.is_null() {
        return 0;
    }

    let mut wbc = WritebackControl::new(job.work.nr_pages.max(0));
    wbc.sync_mode = job.work.sync_mode;
    wbc.tagged_writepages = job.work.tagged_writepages;
    wbc.for_background = job.work.for_background;
    wbc.range_cyclic = job.work.range_cyclic;

    let ops = unsafe { (*job.mapping).a_ops };
    if !ops.is_null() {
        if let Some(writepages) = unsafe { (*ops).writepages } {
            let requested = wbc.nr_to_write;
            unsafe { writepages(job.mapping, &mut wbc) };
            return requested.saturating_sub(wbc.nr_to_write);
        }
    }

    unsafe { generic_writepages(job.mapping, &mut wbc) }
}

pub unsafe fn wb_queue_work(mapping: *mut AddressSpace, work: WritebackWork) {
    if mapping.is_null() {
        return;
    }

    let mut state = WRITEBACK_STATE.lock();
    state.track_mapping(mapping);
    if let Some(existing) = state
        .queued
        .iter_mut()
        .find(|queued| queued.mapping == mapping)
    {
        if work.nr_pages > existing.work.nr_pages {
            existing.work.nr_pages = work.nr_pages;
        }
        existing.work.for_background |= work.for_background;
        existing.work.range_cyclic |= work.range_cyclic;
        existing.work.tagged_writepages |= work.tagged_writepages;
        if matches!(work.sync_mode, WritebackSyncMode::All) {
            existing.work.sync_mode = WritebackSyncMode::All;
        }
        return;
    }

    state.queued.push(WritebackJob { mapping, work });
}

pub fn wb_workfn() -> isize {
    let jobs = {
        let mut state = WRITEBACK_STATE.lock();
        core::mem::take(&mut state.queued)
    };

    let mut written = 0isize;
    for job in jobs {
        written += unsafe { run_writeback_job(job) };
    }
    written
}

pub fn flush_all_dirty_pages() -> isize {
    let mappings = snapshot_mappings();
    let mut written = 0isize;
    for mapping in mappings {
        let dirty = stats().dirty_pages as isize;
        if dirty <= 0 {
            break;
        }
        unsafe {
            wb_queue_work(
                mapping,
                WritebackWork {
                    nr_pages: dirty,
                    sync_mode: WritebackSyncMode::All,
                    tagged_writepages: false,
                    for_background: false,
                    range_cyclic: true,
                },
            );
        }
    }
    written += wb_workfn();
    written
}

pub unsafe fn balance_dirty_pages(mapping: *mut AddressSpace) -> DirtyThrottleResult {
    let (background_thresh, dirty_thresh, dirty_pages) = {
        let state = WRITEBACK_STATE.lock();
        let (background, dirty) = state.dirty_limits();
        (background, dirty, state.dirty_pages)
    };

    let mut result = DirtyThrottleResult {
        background_writeback: false,
        throttled: false,
        dirty_pages,
        background_thresh,
        dirty_thresh,
    };

    if dirty_pages > background_thresh {
        result.background_writeback = true;
        unsafe {
            wb_queue_work(
                mapping,
                WritebackWork {
                    nr_pages: dirty_pages as isize,
                    sync_mode: WritebackSyncMode::None,
                    tagged_writepages: false,
                    for_background: true,
                    range_cyclic: true,
                },
            );
        }
    }

    if dirty_pages > dirty_thresh {
        result.throttled = true;
        let _ = wb_workfn();
        if stats().dirty_pages > background_thresh {
            let _ = flush_all_dirty_pages();
        }
        result.dirty_pages = stats().dirty_pages;
    }

    result
}

#[cfg(test)]
pub fn reset_writeback_state_for_test() {
    *WRITEBACK_STATE.lock() = WritebackState::new();
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::sync::atomic::Ordering;

    use super::*;
    use crate::mm::address_space::AddressSpace;
    use crate::mm::buddy::reset_buddy_state_for_test;
    use crate::mm::filemap::{filemap_add_folio, filemap_remove_folio, set_page_dirty};
    use crate::mm::lru::{lru_add_drain, reset_lru_state_for_test};
    use crate::mm::page_flags::GFP_KERNEL;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

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
        reset_writeback_state_for_test();
        reset_lru_state_for_test();
        guard
    }

    #[test]
    fn balance_dirty_pages_queues_and_throttles() {
        let _guard = test_guard();
        set_dirtyable_memory_pages(8);
        set_dirty_background_ratio(25);
        set_dirty_ratio(50);

        let mut mapping = Box::new(AddressSpace::new());
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let mut pages = Vec::new();

        for index in 0..5u64 {
            let page = alloc_test_page();
            unsafe { filemap_add_folio(mptr, page, index, GFP_KERNEL) };
            unsafe { set_page_dirty(page) };
            pages.push(page);
        }
        lru_add_drain();

        let result = unsafe { balance_dirty_pages(mptr) };
        assert!(result.background_writeback);
        assert!(result.throttled);
        assert!(stats().dirty_pages <= result.background_thresh);
        assert_eq!(queued_work_items(), 0);

        for page in pages {
            unsafe { filemap_remove_folio(page) };
            unsafe { free_test_page(page) };
        }
    }

    #[test]
    fn wb_workfn_cleans_dirty_marks() {
        let _guard = test_guard();

        let mut mapping = Box::new(AddressSpace::new());
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let page = alloc_test_page();
        unsafe { filemap_add_folio(mptr, page, 3, GFP_KERNEL) };
        unsafe { set_page_dirty(page) };
        lru_add_drain();

        unsafe {
            wb_queue_work(
                mptr,
                WritebackWork {
                    nr_pages: 1,
                    sync_mode: WritebackSyncMode::All,
                    tagged_writepages: false,
                    for_background: false,
                    range_cyclic: false,
                },
            );
        }

        assert_eq!(queued_work_items(), 1);
        assert_eq!(wb_workfn(), 1);
        assert_eq!(stats().dirty_pages, 0);
        assert_eq!(
            unsafe { (&*page).flags.load(Ordering::Relaxed) } & (PG_DIRTY | PG_WRITEBACK),
            0
        );

        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }
}
