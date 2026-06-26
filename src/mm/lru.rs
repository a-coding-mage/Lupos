//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Page-cache LRU management — active/inactive file/anon lists.
///
/// This is the Rust rewrite of the Linux LRU subset we need in Milestone 16:
/// queued LRU insertion, `mark_page_accessed()`, `lru_add_drain()`, and
/// the isolate/putback hooks reclaim uses.
///
/// Ref: Linux `mm/swap.c` — `folio_mark_accessed()`, `folio_add_lru()`
///      Linux `mm/vmscan.c` — `shrink_active_list()`, `folio_putback_lru()`
extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Mutex;

use super::list::ListHead;
use super::page::Page;
use super::page_flags::{
    PG_ACTIVE, PG_LRU, PG_RECLAIM, PG_REFERENCED, PG_SWAPBACKED, PG_UNEVICTABLE, PG_WORKINGSET,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LruList {
    InactiveAnon = 0,
    ActiveAnon = 1,
    InactiveFile = 2,
    ActiveFile = 3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LruStats {
    pub inactive_anon: usize,
    pub active_anon: usize,
    pub inactive_file: usize,
    pub active_file: usize,
}

struct LruState {
    initialized: bool,
    pending: Vec<*mut Page>,
    inactive_anon: ListHead,
    active_anon: ListHead,
    inactive_file: ListHead,
    active_file: ListHead,
    stats: [usize; 4],
}

unsafe impl Send for LruState {}

impl LruState {
    const fn new() -> Self {
        Self {
            initialized: false,
            pending: Vec::new(),
            inactive_anon: ListHead::uninit(),
            active_anon: ListHead::uninit(),
            inactive_file: ListHead::uninit(),
            active_file: ListHead::uninit(),
            stats: [0; 4],
        }
    }

    unsafe fn ensure_init(&mut self) {
        if self.initialized {
            return;
        }
        unsafe {
            ListHead::init(&mut self.inactive_anon);
            ListHead::init(&mut self.active_anon);
            ListHead::init(&mut self.inactive_file);
            ListHead::init(&mut self.active_file);
        }
        self.initialized = true;
    }

    fn pending_contains(&self, page: *mut Page) -> bool {
        self.pending.iter().copied().any(|queued| queued == page)
    }

    fn remove_pending(&mut self, page: *mut Page) -> bool {
        if let Some(idx) = self.pending.iter().position(|&queued| queued == page) {
            self.pending.swap_remove(idx);
            true
        } else {
            false
        }
    }

    fn list_head_mut(&mut self, list: LruList) -> *mut ListHead {
        match list {
            LruList::InactiveAnon => &mut self.inactive_anon,
            LruList::ActiveAnon => &mut self.active_anon,
            LruList::InactiveFile => &mut self.inactive_file,
            LruList::ActiveFile => &mut self.active_file,
        }
    }

    unsafe fn detach(&mut self, page: *mut Page) -> Option<LruList> {
        if page.is_null() {
            return None;
        }

        let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
        if (flags & PG_LRU) == 0 {
            return None;
        }

        let current = classify_page(page);
        unsafe {
            ListHead::list_del(&mut (*page).lru);
            (&*page).flags.fetch_and(!PG_LRU, Ordering::Release);
        }
        self.stats[current as usize] = self.stats[current as usize].saturating_sub(1);
        Some(current)
    }

    unsafe fn attach(&mut self, page: *mut Page, target: LruList) {
        if page.is_null() {
            return;
        }

        let active = matches!(target, LruList::ActiveAnon | LruList::ActiveFile);
        unsafe {
            if active {
                (&*page).flags.fetch_or(PG_ACTIVE, Ordering::Relaxed);
            } else {
                (&*page).flags.fetch_and(!PG_ACTIVE, Ordering::Relaxed);
            }
            ListHead::list_add_tail(&mut (*page).lru, self.list_head_mut(target));
            (&*page).flags.fetch_or(PG_LRU, Ordering::Release);
        }
        self.stats[target as usize] += 1;
    }
}

static LRU_STATE: Mutex<LruState> = Mutex::new(LruState::new());

#[inline]
fn page_is_file(page: *mut Page) -> bool {
    if page.is_null() {
        return false;
    }
    let flags = unsafe { (&*page).flags.load(Ordering::Relaxed) };
    (unsafe { (*page).mapping != 0 }) && (flags & PG_SWAPBACKED == 0)
}

#[inline]
fn classify_page(page: *mut Page) -> LruList {
    let flags = unsafe { (&*page).flags.load(Ordering::Relaxed) };
    let active = (flags & PG_ACTIVE) != 0;
    match (page_is_file(page), active) {
        (true, false) => LruList::InactiveFile,
        (true, true) => LruList::ActiveFile,
        (false, false) => LruList::InactiveAnon,
        (false, true) => LruList::ActiveAnon,
    }
}

pub unsafe fn lru_cache_add(page: *mut Page) {
    if page.is_null() {
        return;
    }

    let mut state = LRU_STATE.lock();
    unsafe { state.ensure_init() };
    let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
    if (flags & PG_LRU) != 0 || state.pending_contains(page) {
        return;
    }
    state.pending.push(page);
}

pub fn lru_add_drain() {
    let mut state = LRU_STATE.lock();
    unsafe { state.ensure_init() };
    let pending = core::mem::take(&mut state.pending);
    for page in pending {
        if page.is_null() {
            continue;
        }
        let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
        if (flags & PG_LRU) != 0 {
            continue;
        }
        let has_mapping = unsafe { (*page).mapping != 0 };
        let swapbacked = (flags & PG_SWAPBACKED) != 0;
        if !has_mapping && !swapbacked {
            continue;
        }
        unsafe { state.attach(page, classify_page(page)) };
    }
}

pub unsafe fn mark_page_accessed(page: *mut Page) {
    if page.is_null() {
        return;
    }

    let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
    if (flags & PG_REFERENCED) == 0 {
        unsafe {
            (&*page).flags.fetch_or(PG_REFERENCED, Ordering::Relaxed);
        }
        return;
    }

    if (flags & PG_UNEVICTABLE) != 0 || (flags & PG_ACTIVE) != 0 {
        return;
    }

    let mut state = LRU_STATE.lock();
    unsafe { state.ensure_init() };

    if (unsafe { (&*page).flags.load(Ordering::Acquire) } & PG_LRU) != 0 {
        unsafe {
            state.detach(page);
        }
    }

    unsafe {
        (&*page)
            .flags
            .fetch_or(PG_ACTIVE | PG_WORKINGSET, Ordering::Relaxed);
        (&*page).flags.fetch_and(!PG_REFERENCED, Ordering::Relaxed);
    }

    if !state.pending_contains(page) {
        unsafe {
            state.attach(page, classify_page(page));
        }
    }
}

pub unsafe fn remove_lru_page(page: *mut Page) {
    if page.is_null() {
        return;
    }

    let mut state = LRU_STATE.lock();
    unsafe { state.ensure_init() };
    unsafe {
        state.detach(page);
    }
    state.remove_pending(page);
    unsafe {
        (&*page).flags.fetch_and(
            !(PG_LRU | PG_ACTIVE | PG_REFERENCED | PG_WORKINGSET | PG_RECLAIM),
            Ordering::Relaxed,
        );
    }
}

pub unsafe fn isolate_lru_pages(
    list: LruList,
    nr_to_scan: usize,
    out: &mut Vec<*mut Page>,
) -> usize {
    if nr_to_scan == 0 {
        return 0;
    }

    let mut state = LRU_STATE.lock();
    unsafe { state.ensure_init() };
    let head = state.list_head_mut(list);
    let mut taken = 0usize;

    while taken < nr_to_scan {
        let empty = unsafe { ListHead::is_empty(head) };
        if empty {
            break;
        }

        let entry = unsafe { (*head).next };
        unsafe {
            ListHead::list_del(entry);
        }
        let page = crate::container_of!(entry, Page, lru);
        unsafe {
            (&*page).flags.fetch_and(!PG_LRU, Ordering::Release);
        }
        state.stats[list as usize] = state.stats[list as usize].saturating_sub(1);
        out.push(page);
        taken += 1;
    }

    taken
}

pub unsafe fn putback_page(page: *mut Page) {
    if page.is_null() {
        return;
    }

    let mut state = LRU_STATE.lock();
    unsafe { state.ensure_init() };
    let flags = unsafe { (&*page).flags.load(Ordering::Acquire) };
    if state.pending_contains(page) || (flags & PG_LRU) != 0 {
        return;
    }
    unsafe {
        state.attach(page, classify_page(page));
    }
}

pub fn lru_len(list: LruList) -> usize {
    LRU_STATE.lock().stats[list as usize]
}

pub fn total_lru_pages() -> usize {
    let state = LRU_STATE.lock();
    state.stats.iter().sum()
}

pub fn lru_stats() -> LruStats {
    let state = LRU_STATE.lock();
    LruStats {
        inactive_anon: state.stats[LruList::InactiveAnon as usize],
        active_anon: state.stats[LruList::ActiveAnon as usize],
        inactive_file: state.stats[LruList::InactiveFile as usize],
        active_file: state.stats[LruList::ActiveFile as usize],
    }
}

#[cfg(test)]
pub fn reset_lru_state_for_test() {
    *LRU_STATE.lock() = LruState::new();
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use alloc::boxed::Box;
    use core::sync::atomic::Ordering;

    use super::*;
    use crate::mm::address_space::AddressSpace;
    use crate::mm::buddy::reset_buddy_state_for_test;
    use crate::mm::filemap::{filemap_add_folio, filemap_remove_folio};
    use crate::mm::page_flags::GFP_KERNEL;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
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
        guard
    }

    #[test]
    fn mark_page_accessed_promotes_inactive_file_page() {
        let _guard = test_guard();

        let mut mapping = Box::new(AddressSpace::new());
        let page = alloc_test_page();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };
        lru_add_drain();

        let stats = lru_stats();
        assert_eq!(stats.inactive_file, 1);
        assert_eq!(stats.active_file, 0);

        unsafe { mark_page_accessed(page) };
        assert_ne!(
            unsafe { (&*page).flags.load(Ordering::Relaxed) } & PG_REFERENCED,
            0
        );

        unsafe { mark_page_accessed(page) };
        let stats = lru_stats();
        assert_eq!(stats.inactive_file, 0);
        assert_eq!(stats.active_file, 1);
        assert_eq!(
            unsafe { (&*page).flags.load(Ordering::Relaxed) } & (PG_ACTIVE | PG_REFERENCED),
            PG_ACTIVE
        );

        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    #[test]
    fn lru_add_drain_honors_pre_drain_activation() {
        let _guard = test_guard();

        let mut mapping = Box::new(AddressSpace::new());
        let page = alloc_test_page();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };

        unsafe { mark_page_accessed(page) };
        unsafe { mark_page_accessed(page) };
        lru_add_drain();

        let stats = lru_stats();
        assert_eq!(stats.inactive_file, 0);
        assert_eq!(stats.active_file, 1);

        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }
}
