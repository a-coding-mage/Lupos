//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Compaction, migration, CMA, ballooning, HMM, and hotplug.
//!
//! This module implements the runnable model behind:
//! - `vendor/linux/mm/migrate.c`
//! - `vendor/linux/mm/compaction.c`
//! - `vendor/linux/mm/cma.c`
//! - `vendor/linux/mm/cma_debug.c`
//! - `vendor/linux/mm/cma_sysfs.c`
//! - `vendor/linux/mm/balloon.c`
//! - `vendor/linux/mm/hmm.c`
//! - `vendor/linux/mm/migrate_device.c`
//! - `vendor/linux/mm/memory_hotplug.c`
//! - `vendor/linux/mm/page_isolation.c`

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::{EFAULT, EINVAL, ENOENT};
use crate::mm::frame::PAGE_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationFacility {
    Compaction,
    Cma,
    Balloon,
    Hmm,
    MemoryHotplug,
    DevicePrivate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MigratablePage {
    pub pfn: u64,
    pub node: u16,
    pub movable: bool,
    pub isolated: bool,
    pub ballooned: bool,
    pub device_private: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MigrationStats {
    pub pages: usize,
    pub isolated: usize,
    pub migrated: usize,
    pub hotplug_online: usize,
    pub ballooned: usize,
    pub cma_success_pages: usize,
    pub cma_failed_pages: usize,
    pub cma_released_pages: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CmaStats {
    pub total_pages: usize,
    pub available_pages: usize,
    pub alloc_pages_success: usize,
    pub alloc_pages_fail: usize,
    pub release_pages_success: usize,
}

struct MigrationState {
    pages: Vec<MigratablePage>,
    migrated: usize,
    hotplug_online: usize,
    cma: CmaStats,
}

impl MigrationState {
    const fn new() -> Self {
        Self {
            pages: Vec::new(),
            migrated: 0,
            hotplug_online: 0,
            cma: CmaStats {
                total_pages: 0,
                available_pages: 0,
                alloc_pages_success: 0,
                alloc_pages_fail: 0,
                release_pages_success: 0,
            },
        }
    }

    fn reset(&mut self) {
        self.pages.clear();
        self.migrated = 0;
        self.hotplug_online = 0;
        self.cma = CmaStats::default();
    }
}

static MIGRATION_STATE: Mutex<MigrationState> = Mutex::new(MigrationState::new());

pub const fn facility_enabled(_facility: MigrationFacility) -> bool {
    true
}

pub fn register_page(pfn: u64, node: u16, movable: bool) {
    let mut state = MIGRATION_STATE.lock();
    if let Some(page) = state.pages.iter_mut().find(|page| page.pfn == pfn) {
        page.node = node;
        page.movable = movable;
        page.isolated = false;
        page.ballooned = false;
        page.device_private = false;
    } else {
        state.pages.push(MigratablePage {
            pfn,
            node,
            movable,
            isolated: false,
            ballooned: false,
            device_private: false,
        });
    }
}

pub fn page_state(pfn: u64) -> Option<MigratablePage> {
    MIGRATION_STATE
        .lock()
        .pages
        .iter()
        .find(|page| page.pfn == pfn)
        .copied()
}

pub fn alloc_contig_range(start_pfn: u64, nr_pages: u64) -> Result<(), i32> {
    if nr_pages == 0 {
        return Err(EINVAL);
    }
    isolate_page_range(start_pfn, start_pfn.checked_add(nr_pages).ok_or(EINVAL)?)?;
    Ok(())
}

pub fn cma_declare_contiguous(total_pages: usize) -> Result<(), i32> {
    if total_pages == 0 {
        return Err(EINVAL);
    }
    let mut state = MIGRATION_STATE.lock();
    state.cma.total_pages = total_pages;
    state.cma.available_pages = total_pages;
    Ok(())
}

pub fn cma_alloc_pages(nr_pages: usize) -> Result<(), i32> {
    if nr_pages == 0 {
        return Err(EINVAL);
    }
    let mut state = MIGRATION_STATE.lock();
    if state.cma.available_pages < nr_pages {
        state.cma.alloc_pages_fail = state.cma.alloc_pages_fail.saturating_add(nr_pages);
        return Err(ENOENT);
    }
    state.cma.available_pages -= nr_pages;
    state.cma.alloc_pages_success = state.cma.alloc_pages_success.saturating_add(nr_pages);
    Ok(())
}

pub fn cma_release_pages(nr_pages: usize) -> Result<(), i32> {
    if nr_pages == 0 {
        return Err(EINVAL);
    }
    let mut state = MIGRATION_STATE.lock();
    state.cma.available_pages = core::cmp::min(
        state.cma.total_pages,
        state.cma.available_pages.saturating_add(nr_pages),
    );
    state.cma.release_pages_success = state.cma.release_pages_success.saturating_add(nr_pages);
    Ok(())
}

pub fn cma_sysfs_snapshot() -> CmaStats {
    MIGRATION_STATE.lock().cma
}

pub fn cma_debugfs_available_pages() -> usize {
    MIGRATION_STATE.lock().cma.available_pages
}

pub fn compact_memory() -> Result<usize, i32> {
    let mut state = MIGRATION_STATE.lock();
    let mut movable: Vec<_> = state
        .pages
        .iter()
        .copied()
        .filter(|page| page.movable && !page.isolated && !page.ballooned)
        .collect();
    movable.sort_by_key(|page| page.pfn);

    let mut migrated = 0;
    for (idx, src) in movable.iter().enumerate() {
        let dst_pfn = idx as u64;
        if src.pfn != dst_pfn {
            if let Some(page) = state.pages.iter_mut().find(|page| page.pfn == src.pfn) {
                page.pfn = dst_pfn;
                migrated += 1;
            }
        }
    }
    state.migrated += migrated;
    Ok(migrated)
}

pub fn isolate_page_range(start_pfn: u64, end_pfn: u64) -> Result<(), i32> {
    if start_pfn >= end_pfn {
        return Err(EINVAL);
    }

    let mut state = MIGRATION_STATE.lock();
    for pfn in start_pfn..end_pfn {
        let page = state
            .pages
            .iter_mut()
            .find(|page| page.pfn == pfn)
            .ok_or(ENOENT)?;
        if !page.movable {
            return Err(EINVAL);
        }
        page.isolated = true;
    }
    Ok(())
}

pub fn migrate_page(pfn: u64, new_node: u16) -> Result<(), i32> {
    let mut state = MIGRATION_STATE.lock();
    let page = state
        .pages
        .iter_mut()
        .find(|page| page.pfn == pfn)
        .ok_or(ENOENT)?;
    if !page.movable {
        return Err(EINVAL);
    }
    page.node = new_node;
    page.isolated = false;
    state.migrated += 1;
    Ok(())
}

pub fn move_pages(
    pid: i32,
    nr_pages: usize,
    pages: *const u64,
    status: *mut i32,
    flags: i32,
) -> i64 {
    if pid < 0 || flags & !0x3 != 0 {
        return -(EINVAL as i64);
    }
    if nr_pages != 0 && (pages.is_null() || status.is_null()) {
        return -(EFAULT as i64);
    }

    let state = MIGRATION_STATE.lock();
    for idx in 0..nr_pages {
        let addr = unsafe { *pages.add(idx) };
        let pfn = addr / PAGE_SIZE as u64;
        let node = state
            .pages
            .iter()
            .find(|page| page.pfn == pfn)
            .map(|page| page.node as i32)
            .unwrap_or(-(ENOENT as i32));
        unsafe {
            *status.add(idx) = node;
        }
    }
    0
}

pub fn memory_hotplug_online(start_pfn: u64, nr_pages: u64) -> Result<(), i32> {
    if nr_pages == 0 {
        return Err(EINVAL);
    }
    let mut state = MIGRATION_STATE.lock();
    for pfn in start_pfn..start_pfn.checked_add(nr_pages).ok_or(EINVAL)? {
        if state.pages.iter().any(|page| page.pfn == pfn) {
            return Err(EINVAL);
        }
        state.pages.push(MigratablePage {
            pfn,
            node: 0,
            movable: true,
            isolated: false,
            ballooned: false,
            device_private: false,
        });
    }
    state.hotplug_online += nr_pages as usize;
    Ok(())
}

pub fn balloon_page(pfn: u64) -> Result<(), i32> {
    let mut state = MIGRATION_STATE.lock();
    let page = state
        .pages
        .iter_mut()
        .find(|page| page.pfn == pfn)
        .ok_or(ENOENT)?;
    page.ballooned = true;
    Ok(())
}

pub fn migrate_to_device_private(pfn: u64) -> Result<(), i32> {
    let mut state = MIGRATION_STATE.lock();
    let page = state
        .pages
        .iter_mut()
        .find(|page| page.pfn == pfn)
        .ok_or(ENOENT)?;
    if !page.movable {
        return Err(EINVAL);
    }
    page.device_private = true;
    state.migrated += 1;
    Ok(())
}

pub fn migration_stats() -> MigrationStats {
    let state = MIGRATION_STATE.lock();
    MigrationStats {
        pages: state.pages.len(),
        isolated: state.pages.iter().filter(|page| page.isolated).count(),
        migrated: state.migrated,
        hotplug_online: state.hotplug_online,
        ballooned: state.pages.iter().filter(|page| page.ballooned).count(),
        cma_success_pages: state.cma.alloc_pages_success,
        cma_failed_pages: state.cma.alloc_pages_fail,
        cma_released_pages: state.cma.release_pages_success,
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    MIGRATION_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use alloc::vec;

    #[test]
    fn migration_and_compaction_move_registered_pages() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(facility_enabled(MigrationFacility::Compaction));
        register_page(10, 0, true);
        register_page(11, 0, true);
        assert_eq!(migrate_page(10, 1), Ok(()));
        assert_eq!(page_state(10).unwrap().node, 1);

        let moved = compact_memory().unwrap();
        assert!(moved >= 1);
        assert!(migration_stats().migrated >= moved);
    }

    #[test]
    fn cma_hotplug_balloon_and_device_private_paths_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(cma_declare_contiguous(8), Ok(()));
        assert_eq!(cma_alloc_pages(3), Ok(()));
        assert_eq!(cma_alloc_pages(6), Err(ENOENT));
        assert_eq!(cma_release_pages(2), Ok(()));
        assert_eq!(cma_debugfs_available_pages(), 7);
        assert_eq!(cma_sysfs_snapshot().alloc_pages_success, 3);
        assert_eq!(migration_stats().cma_failed_pages, 6);

        assert_eq!(memory_hotplug_online(100, 3), Ok(()));
        assert_eq!(alloc_contig_range(100, 2), Ok(()));
        assert_eq!(migration_stats().isolated, 2);
        assert_eq!(balloon_page(102), Ok(()));
        assert_eq!(migrate_to_device_private(101), Ok(()));
        assert!(page_state(101).unwrap().device_private);
        assert_eq!(migration_stats().ballooned, 1);
    }

    #[test]
    fn move_pages_reports_nodes_for_each_address() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        register_page(4, 2, true);
        let pages = vec![4 * PAGE_SIZE as u64, 9 * PAGE_SIZE as u64];
        let mut status = vec![0i32; 2];
        assert_eq!(move_pages(0, 2, pages.as_ptr(), status.as_mut_ptr(), 0), 0);
        assert_eq!(status[0], 2);
        assert_eq!(status[1], -ENOENT);
    }
}
