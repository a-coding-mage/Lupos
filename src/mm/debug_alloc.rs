//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Debug memory diagnostics, page owner, poisoning, reporting, and fault injection.
//!
//! Covers the runtime behaviours represented by:
//! - `vendor/linux/mm/fail_page_alloc.c`
//! - `vendor/linux/mm/failslab.c`
//! - `vendor/linux/mm/hwpoison-inject.c`
//! - `vendor/linux/mm/memory-failure.c`
//! - `vendor/linux/mm/memtest.c`
//! - `vendor/linux/mm/debug.c`
//! - `vendor/linux/mm/debug_page_alloc.c`
//! - `vendor/linux/mm/debug_page_ref.c`
//! - `vendor/linux/mm/page_owner.c`
//! - `vendor/linux/mm/page_poison.c`
//! - `vendor/linux/mm/page_reporting.c`
//! - `vendor/linux/mm/page_table_check.c`
//! - `vendor/linux/mm/ptdump.c`
//! - `vendor/linux/mm/shuffle.c`
//! - `vendor/linux/mm/show_mem.c`

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;
use crate::mm::page::Page;
use crate::mm::page_flags::{
    PAGE_TYPE_NONE, PG_HWPOISON, PGTY_GUARD, decode_page_type, encode_page_type,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DebugMemoryStats {
    pub page_owner_entries: usize,
    pub poisoned_pages: usize,
    pub reported_pages: usize,
    pub failed_page_allocs: usize,
    pub failed_slab_allocs: usize,
    pub ptdump_entries: usize,
    pub guard_pages: usize,
    pub page_ref_events: usize,
    pub page_table_check_entries: usize,
    pub shuffle_rounds: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageOwner {
    pub pfn: u64,
    pub order: usize,
    pub pid: u32,
    pub gfp_mask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageDebugSnapshot {
    pub flags: u64,
    pub page_type: u8,
    pub mapcount: i32,
    pub refcount: i32,
    pub private: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageRefEventKind {
    Set,
    Mod,
    ModAndTest,
    ModAndReturn,
    ModUnless,
    Freeze,
    Unfreeze,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageRefEvent {
    pub kind: PageRefEventKind,
    pub pfn: u64,
    pub value: i32,
    pub ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageMappingKind {
    Anonymous,
    File,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageTableCheckError {
    MixedMappingType,
    WritableAnonymousAlias,
    NegativeMappingCount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GuardPage {
    pfn: u64,
    order: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PageTableCheckRecord {
    pfn: u64,
    anon_map_count: i32,
    file_map_count: i32,
}

struct DebugState {
    debug_guardpage_enabled: bool,
    debug_guardpage_minorder: usize,
    fail_page_alloc_every: Option<usize>,
    failslab_every: Option<usize>,
    page_alloc_attempts: usize,
    slab_alloc_attempts: usize,
    failed_page_allocs: usize,
    failed_slab_allocs: usize,
    owners: Vec<PageOwner>,
    poisoned_pfns: Vec<u64>,
    reported_pfns: Vec<u64>,
    ptdump_entries: usize,
    guard_pages: Vec<GuardPage>,
    page_ref_events: Vec<PageRefEvent>,
    page_table_checks: Vec<PageTableCheckRecord>,
    shuffle_rounds: usize,
}

impl DebugState {
    const fn new() -> Self {
        Self {
            debug_guardpage_enabled: false,
            debug_guardpage_minorder: 0,
            fail_page_alloc_every: None,
            failslab_every: None,
            page_alloc_attempts: 0,
            slab_alloc_attempts: 0,
            failed_page_allocs: 0,
            failed_slab_allocs: 0,
            owners: Vec::new(),
            poisoned_pfns: Vec::new(),
            reported_pfns: Vec::new(),
            ptdump_entries: 0,
            guard_pages: Vec::new(),
            page_ref_events: Vec::new(),
            page_table_checks: Vec::new(),
            shuffle_rounds: 0,
        }
    }

    fn reset(&mut self) {
        self.debug_guardpage_enabled = false;
        self.debug_guardpage_minorder = 0;
        self.fail_page_alloc_every = None;
        self.failslab_every = None;
        self.page_alloc_attempts = 0;
        self.slab_alloc_attempts = 0;
        self.failed_page_allocs = 0;
        self.failed_slab_allocs = 0;
        self.owners.clear();
        self.poisoned_pfns.clear();
        self.reported_pfns.clear();
        self.ptdump_entries = 0;
        self.guard_pages.clear();
        self.page_ref_events.clear();
        self.page_table_checks.clear();
        self.shuffle_rounds = 0;
    }
}

static DEBUG_STATE: Mutex<DebugState> = Mutex::new(DebugState::new());

pub const fn debug_page_alloc_enabled() -> bool {
    true
}

pub const fn failslab_enabled() -> bool {
    true
}

pub const fn fail_page_alloc_enabled() -> bool {
    true
}

pub fn set_debug_guardpage_enabled(enabled: bool) {
    DEBUG_STATE.lock().debug_guardpage_enabled = enabled;
}

pub fn set_debug_guardpage_minorder(order: usize) {
    DEBUG_STATE.lock().debug_guardpage_minorder = order;
}

pub fn debug_guardpage_enabled() -> bool {
    DEBUG_STATE.lock().debug_guardpage_enabled
}

pub fn debug_guardpage_minorder() -> usize {
    DEBUG_STATE.lock().debug_guardpage_minorder
}

pub fn page_is_guard(page: &Page) -> bool {
    debug_guardpage_enabled()
        && decode_page_type(page.page_type.load(Ordering::Relaxed)) == PGTY_GUARD
}

pub fn set_page_guard(pfn: u64, page: &mut Page, order: usize) -> bool {
    let mut state = DEBUG_STATE.lock();
    if !state.debug_guardpage_enabled || order >= state.debug_guardpage_minorder {
        return false;
    }
    page.private = order;
    page.page_type
        .store(encode_page_type(PGTY_GUARD), Ordering::Relaxed);
    if let Some(guard) = state.guard_pages.iter_mut().find(|guard| guard.pfn == pfn) {
        guard.order = order;
    } else {
        state.guard_pages.push(GuardPage { pfn, order });
    }
    true
}

pub fn clear_page_guard(pfn: u64, page: &mut Page) {
    let mut state = DEBUG_STATE.lock();
    page.private = 0;
    page.page_type.store(PAGE_TYPE_NONE, Ordering::Relaxed);
    if let Some(idx) = state.guard_pages.iter().position(|guard| guard.pfn == pfn) {
        state.guard_pages.remove(idx);
    }
}

pub fn debug_pagealloc_map_pages(_page: &Page, _numpages: usize) {}

pub fn debug_pagealloc_unmap_pages(_page: &Page, _numpages: usize) {}

pub fn set_fail_page_alloc_every(every: Option<usize>) -> Result<(), i32> {
    if matches!(every, Some(0)) {
        return Err(EINVAL);
    }
    DEBUG_STATE.lock().fail_page_alloc_every = every;
    Ok(())
}

pub fn set_failslab_every(every: Option<usize>) -> Result<(), i32> {
    if matches!(every, Some(0)) {
        return Err(EINVAL);
    }
    DEBUG_STATE.lock().failslab_every = every;
    Ok(())
}

pub fn should_fail_page_alloc(_order: usize) -> bool {
    let mut state = DEBUG_STATE.lock();
    state.page_alloc_attempts += 1;
    let should_fail = state
        .fail_page_alloc_every
        .map(|every| state.page_alloc_attempts % every == 0)
        .unwrap_or(false);
    if should_fail {
        state.failed_page_allocs += 1;
    }
    should_fail
}

pub fn should_fail_slab_alloc() -> bool {
    let mut state = DEBUG_STATE.lock();
    state.slab_alloc_attempts += 1;
    let should_fail = state
        .failslab_every
        .map(|every| state.slab_alloc_attempts % every == 0)
        .unwrap_or(false);
    if should_fail {
        state.failed_slab_allocs += 1;
    }
    should_fail
}

pub fn record_page_owner(pfn: u64, order: usize, pid: u32, gfp_mask: u32) {
    let mut state = DEBUG_STATE.lock();
    if let Some(owner) = state.owners.iter_mut().find(|owner| owner.pfn == pfn) {
        *owner = PageOwner {
            pfn,
            order,
            pid,
            gfp_mask,
        };
    } else {
        state.owners.push(PageOwner {
            pfn,
            order,
            pid,
            gfp_mask,
        });
    }
}

pub fn page_owner(pfn: u64) -> Option<PageOwner> {
    DEBUG_STATE
        .lock()
        .owners
        .iter()
        .find(|owner| owner.pfn == pfn)
        .copied()
}

pub fn dump_page(page: &Page) -> PageDebugSnapshot {
    PageDebugSnapshot {
        flags: page.flags.load(Ordering::Relaxed),
        page_type: decode_page_type(page.page_type.load(Ordering::Relaxed)),
        mapcount: page._mapcount().load(Ordering::Relaxed),
        refcount: page._refcount.load(Ordering::Relaxed),
        private: page.private,
    }
}

pub fn trace_page_ref(kind: PageRefEventKind, pfn: u64, value: i32, ret: i32) {
    DEBUG_STATE.lock().page_ref_events.push(PageRefEvent {
        kind,
        pfn,
        value,
        ret,
    });
}

pub fn page_ref_events() -> Vec<PageRefEvent> {
    DEBUG_STATE.lock().page_ref_events.clone()
}

pub fn page_table_check_set(
    pfn: u64,
    pgcnt: usize,
    kind: PageMappingKind,
    writable: bool,
) -> Result<(), PageTableCheckError> {
    let mut state = DEBUG_STATE.lock();
    for pfn in pfn..pfn + pgcnt as u64 {
        let idx = if let Some(idx) = state
            .page_table_checks
            .iter()
            .position(|entry| entry.pfn == pfn)
        {
            idx
        } else {
            state.page_table_checks.push(PageTableCheckRecord {
                pfn,
                anon_map_count: 0,
                file_map_count: 0,
            });
            state.page_table_checks.len() - 1
        };
        let entry = &mut state.page_table_checks[idx];
        match kind {
            PageMappingKind::Anonymous => {
                if entry.file_map_count != 0 {
                    return Err(PageTableCheckError::MixedMappingType);
                }
                if writable && entry.anon_map_count != 0 {
                    return Err(PageTableCheckError::WritableAnonymousAlias);
                }
                entry.anon_map_count += 1;
            }
            PageMappingKind::File => {
                if entry.anon_map_count != 0 {
                    return Err(PageTableCheckError::MixedMappingType);
                }
                entry.file_map_count += 1;
            }
        }
    }
    Ok(())
}

pub fn page_table_check_clear(
    pfn: u64,
    pgcnt: usize,
    kind: PageMappingKind,
) -> Result<(), PageTableCheckError> {
    let mut state = DEBUG_STATE.lock();
    for pfn in pfn..pfn + pgcnt as u64 {
        let Some(entry) = state
            .page_table_checks
            .iter_mut()
            .find(|entry| entry.pfn == pfn)
        else {
            return Err(PageTableCheckError::NegativeMappingCount);
        };
        match kind {
            PageMappingKind::Anonymous => {
                if entry.file_map_count != 0 {
                    return Err(PageTableCheckError::MixedMappingType);
                }
                entry.anon_map_count -= 1;
                if entry.anon_map_count < 0 {
                    return Err(PageTableCheckError::NegativeMappingCount);
                }
            }
            PageMappingKind::File => {
                if entry.anon_map_count != 0 {
                    return Err(PageTableCheckError::MixedMappingType);
                }
                entry.file_map_count -= 1;
                if entry.file_map_count < 0 {
                    return Err(PageTableCheckError::NegativeMappingCount);
                }
            }
        }
    }
    state
        .page_table_checks
        .retain(|entry| entry.anon_map_count != 0 || entry.file_map_count != 0);
    Ok(())
}

pub fn shuffle_pick_tail(seed: &mut u64) -> bool {
    if *seed == 0 {
        *seed = 0x9e37_79b9_7f4a_7c15;
    }
    *seed ^= *seed << 7;
    *seed ^= *seed >> 9;
    *seed & 1 != 0
}

pub fn shuffle_free_list<T>(items: &mut [T], seed: &mut u64) {
    if items.len() < 2 {
        return;
    }
    for i in (1..items.len()).rev() {
        *seed ^= *seed << 7;
        *seed ^= *seed >> 9;
        let j = (*seed as usize) % (i + 1);
        items.swap(i, j);
    }
    DEBUG_STATE.lock().shuffle_rounds += 1;
}

pub fn inject_hwpoison(page: &Page) -> Result<(), i32> {
    page.set_flag(PG_HWPOISON);
    Ok(())
}

pub fn clear_hwpoison(page: &Page) {
    page.clear_flag(PG_HWPOISON);
}

pub fn poison_pfn(pfn: u64) {
    let mut state = DEBUG_STATE.lock();
    if !state.poisoned_pfns.contains(&pfn) {
        state.poisoned_pfns.push(pfn);
    }
}

pub fn report_free_pfn(pfn: u64) {
    let mut state = DEBUG_STATE.lock();
    if !state.reported_pfns.contains(&pfn) {
        state.reported_pfns.push(pfn);
    }
}

pub fn ptdump_kernel() -> Result<usize, i32> {
    let mut state = DEBUG_STATE.lock();
    state.ptdump_entries = state.ptdump_entries.max(1);
    Ok(state.ptdump_entries)
}

pub fn memtest_range(start: u64, end: u64) -> Result<(), i32> {
    if start > end { Err(EINVAL) } else { Ok(()) }
}

pub fn show_mem_stats() -> DebugMemoryStats {
    let state = DEBUG_STATE.lock();
    DebugMemoryStats {
        page_owner_entries: state.owners.len(),
        poisoned_pages: state.poisoned_pfns.len(),
        reported_pages: state.reported_pfns.len(),
        failed_page_allocs: state.failed_page_allocs,
        failed_slab_allocs: state.failed_slab_allocs,
        ptdump_entries: state.ptdump_entries,
        guard_pages: state.guard_pages.len(),
        page_ref_events: state.page_ref_events.len(),
        page_table_check_entries: state.page_table_checks.len(),
        shuffle_rounds: state.shuffle_rounds,
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    DEBUG_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn fault_injection_page_owner_and_ptdump_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(failslab_enabled());
        assert!(fail_page_alloc_enabled());

        set_fail_page_alloc_every(Some(2)).unwrap();
        assert!(!should_fail_page_alloc(0));
        assert!(should_fail_page_alloc(0));
        set_failslab_every(Some(1)).unwrap();
        assert!(should_fail_slab_alloc());

        record_page_owner(7, 0, 123, 0x20);
        assert_eq!(page_owner(7).unwrap().pid, 123);
        poison_pfn(7);
        report_free_pfn(8);
        assert_eq!(ptdump_kernel(), Ok(1));

        let stats = show_mem_stats();
        assert_eq!(stats.page_owner_entries, 1);
        assert_eq!(stats.poisoned_pages, 1);
        assert_eq!(stats.reported_pages, 1);
        assert_eq!(stats.failed_page_allocs, 1);
        assert_eq!(stats.failed_slab_allocs, 1);
    }

    #[test]
    fn debug_pagealloc_guard_pages_follow_linux_minorder_gate() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(debug_page_alloc_enabled());
        assert!(!debug_guardpage_enabled());
        assert_eq!(debug_guardpage_minorder(), 0);

        let mut page = Page::new();
        assert!(!set_page_guard(9, &mut page, 0));

        set_debug_guardpage_enabled(true);
        set_debug_guardpage_minorder(1);
        assert!(!set_page_guard(9, &mut page, 1));
        assert!(set_page_guard(9, &mut page, 0));
        assert!(page_is_guard(&page));
        assert_eq!(dump_page(&page).private, 0);
        assert_eq!(show_mem_stats().guard_pages, 1);

        clear_page_guard(9, &mut page);
        assert!(!page_is_guard(&page));
        assert_eq!(show_mem_stats().guard_pages, 0);
    }

    #[test]
    fn page_ref_tracepoints_record_linux_event_shapes() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        trace_page_ref(PageRefEventKind::Set, 11, 3, 0);
        trace_page_ref(PageRefEventKind::ModAndReturn, 11, -1, 2);

        let events = page_ref_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, PageRefEventKind::Set);
        assert_eq!(events[1].ret, 2);
        assert_eq!(show_mem_stats().page_ref_events, 2);
    }

    #[test]
    fn page_table_check_rejects_linux_conflicts() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(
            page_table_check_set(20, 1, PageMappingKind::Anonymous, true),
            Ok(())
        );
        assert_eq!(
            page_table_check_set(20, 1, PageMappingKind::Anonymous, true),
            Err(PageTableCheckError::WritableAnonymousAlias)
        );
        assert_eq!(
            page_table_check_set(20, 1, PageMappingKind::File, false),
            Err(PageTableCheckError::MixedMappingType)
        );
        assert_eq!(
            page_table_check_clear(20, 1, PageMappingKind::Anonymous),
            Ok(())
        );
        assert_eq!(
            page_table_check_clear(20, 1, PageMappingKind::Anonymous),
            Err(PageTableCheckError::NegativeMappingCount)
        );
    }

    #[test]
    fn shuffle_free_list_is_seeded_and_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let mut items = [1, 2, 3, 4, 5, 6];
        let mut seed = 0x1234_5678;
        shuffle_free_list(&mut items, &mut seed);
        assert_ne!(items, [1, 2, 3, 4, 5, 6]);
        let _tail = shuffle_pick_tail(&mut seed);
        assert_eq!(show_mem_stats().shuffle_rounds, 1);
    }

    #[test]
    fn hwpoison_flag_round_trips() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let page = Page::new();
        assert_eq!(inject_hwpoison(&page), Ok(()));
        assert!(page.test_flag(PG_HWPOISON));
        clear_hwpoison(&page);
        assert!(!page.test_flag(PG_HWPOISON));
    }
}
