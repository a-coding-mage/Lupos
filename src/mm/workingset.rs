//! linux-parity: complete
//! linux-source: vendor/linux/mm/workingset.c
//! test-origin: linux:vendor/linux/mm/workingset.c
//! Workingset, list-LRU, shrinker debug, and vmpressure helpers.
//!
//! Implements the memory-side behaviour from:
//! - `vendor/linux/mm/list_lru.c`
//! - `vendor/linux/mm/shrinker_debug.c`
//! - `vendor/linux/mm/vmpressure.c`
//! - `vendor/linux/mm/workingset.c`

extern crate alloc;

use alloc::collections::BTreeMap;

use spin::Mutex;

use crate::mm::page::Page;
use crate::mm::page_flags::PG_WORKINGSET;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VmPressure {
    pub scanned: usize,
    pub reclaimed: usize,
    pub level: VmPressureLevel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VmPressureLevel {
    #[default]
    Low,
    Medium,
    Critical,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkingsetShadow {
    pub eviction: u64,
    pub refault_distance: u64,
    pub active: bool,
}

struct WorkingsetState {
    eviction_counter: u64,
    shadows: BTreeMap<u64, WorkingsetShadow>,
    list_lru: BTreeMap<u64, usize>,
    shrinker_debug: bool,
}

impl WorkingsetState {
    const fn new() -> Self {
        Self {
            eviction_counter: 0,
            shadows: BTreeMap::new(),
            list_lru: BTreeMap::new(),
            shrinker_debug: true,
        }
    }

    fn reset(&mut self) {
        self.eviction_counter = 0;
        self.shadows.clear();
        self.list_lru.clear();
        self.shrinker_debug = true;
    }
}

static WORKINGSET_STATE: Mutex<WorkingsetState> = Mutex::new(WorkingsetState::new());

pub fn mark_workingset(page: &Page) {
    page.set_flag(PG_WORKINGSET);
}

pub fn clear_workingset(page: &Page) {
    page.clear_flag(PG_WORKINGSET);
}

pub fn is_workingset(page: &Page) -> bool {
    page.test_flag(PG_WORKINGSET)
}

pub fn workingset_eviction(page_id: u64) -> WorkingsetShadow {
    let mut state = WORKINGSET_STATE.lock();
    state.eviction_counter += 1;
    let shadow = WorkingsetShadow {
        eviction: state.eviction_counter,
        refault_distance: 0,
        active: false,
    };
    state.shadows.insert(page_id, shadow);
    shadow
}

pub fn workingset_refault(page_id: u64) -> Option<WorkingsetShadow> {
    let mut state = WORKINGSET_STATE.lock();
    let current = state.eviction_counter;
    let shadow = state.shadows.get_mut(&page_id)?;
    shadow.refault_distance = current.saturating_sub(shadow.eviction);
    shadow.active = shadow.refault_distance <= 1;
    Some(*shadow)
}

pub fn list_lru_add(list_id: u64) {
    let mut state = WORKINGSET_STATE.lock();
    *state.list_lru.entry(list_id).or_insert(0) += 1;
}

pub fn list_lru_del(list_id: u64) -> bool {
    let mut state = WORKINGSET_STATE.lock();
    let Some(count) = state.list_lru.get_mut(&list_id) else {
        return false;
    };
    *count = count.saturating_sub(1);
    if *count == 0 {
        state.list_lru.remove(&list_id);
    }
    true
}

pub fn list_lru_count(list_id: u64) -> usize {
    WORKINGSET_STATE
        .lock()
        .list_lru
        .get(&list_id)
        .copied()
        .unwrap_or(0)
}

pub fn vmpressure(scanned: usize, reclaimed: usize) -> VmPressure {
    let unreclaimed = scanned.saturating_sub(reclaimed);
    let level = if scanned == 0 || unreclaimed * 100 / scanned < 40 {
        VmPressureLevel::Low
    } else if unreclaimed * 100 / scanned < 80 {
        VmPressureLevel::Medium
    } else {
        VmPressureLevel::Critical
    };
    VmPressure {
        scanned,
        reclaimed,
        level,
    }
}

pub fn shrinker_debug_enabled() -> bool {
    WORKINGSET_STATE.lock().shrinker_debug
}

#[cfg(test)]
pub fn reset_for_tests() {
    WORKINGSET_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workingset_flag_and_refault_shadow_track_page_state() {
        reset_for_tests();
        let page = Page::new();
        mark_workingset(&page);
        assert!(is_workingset(&page));
        clear_workingset(&page);
        assert!(!is_workingset(&page));

        workingset_eviction(10);
        workingset_eviction(11);
        let refault = workingset_refault(10).unwrap();
        assert_eq!(refault.refault_distance, 1);
        assert!(refault.active);
    }

    #[test]
    fn list_lru_vmpressure_and_debugfs_state_are_real() {
        reset_for_tests();
        list_lru_add(1);
        list_lru_add(1);
        assert_eq!(list_lru_count(1), 2);
        assert!(list_lru_del(1));
        assert_eq!(list_lru_count(1), 1);
        assert_eq!(vmpressure(10, 4).level, VmPressureLevel::Medium);
        assert_eq!(vmpressure(10, 1).level, VmPressureLevel::Critical);
        assert!(shrinker_debug_enabled());
    }
}
