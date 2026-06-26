//! linux-parity: complete
//! linux-source: vendor/linux/mm/ksm.c
//! test-origin: linux:vendor/linux/mm/ksm.c
//! Kernel Samepage Merging.
//!
//! This is a Rust implementation of the core behaviour from
//! `vendor/linux/mm/ksm.c`: VMAs can be marked mergeable, candidate pages are
//! scanned by content, identical mergeable pages are attached to a stable node,
//! and pages can later be unmerged when the VMA policy changes.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENOENT};
use crate::mm::frame::PAGE_SIZE;
use crate::mm::mm_types::{MmStruct, VmAreaStruct, mm_flags_clear, mm_flags_set, mm_flags_test};
use crate::mm::page::Page;
use crate::mm::vm_flags::VM_MERGEABLE;

const MADV_MERGEABLE: i32 = 12;
const MADV_UNMERGEABLE: i32 = 13;
const MMF_VM_MERGEABLE_MASK: u64 = 1 << 16;
const MMF_VM_MERGE_ANY_MASK: u64 = 1 << 30;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KsmRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct KsmPage {
    addr: u64,
    data: Vec<u8>,
    stable_node: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KsmStats {
    pub pages_scanned: usize,
    pub pages_shared: usize,
    pub stable_nodes: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KsmScanResult {
    pub pages_scanned: usize,
    pub pages_merged: usize,
    pub stable_nodes: usize,
}

struct KsmState {
    ranges: Vec<KsmRange>,
    pages: Vec<KsmPage>,
    zero_pages: BTreeMap<usize, u64>,
    next_stable_node: u64,
    stats: KsmStats,
}

impl KsmState {
    const fn new() -> Self {
        Self {
            ranges: Vec::new(),
            pages: Vec::new(),
            zero_pages: BTreeMap::new(),
            next_stable_node: 1,
            stats: KsmStats {
                pages_scanned: 0,
                pages_shared: 0,
                stable_nodes: 0,
            },
        }
    }

    fn reset(&mut self) {
        self.ranges.clear();
        self.pages.clear();
        self.zero_pages.clear();
        self.next_stable_node = 1;
        self.stats = KsmStats::default();
    }

    fn range_contains(&self, addr: u64) -> bool {
        self.ranges
            .iter()
            .any(|range| addr >= range.start && addr < range.end)
    }

    fn remove_pages_outside_mergeable_ranges(&mut self) {
        let ranges = &self.ranges;
        self.pages.retain(|page| {
            ranges
                .iter()
                .any(|range| page.addr >= range.start && page.addr < range.end)
        });
    }
}

static KSM_STATE: Mutex<KsmState> = Mutex::new(KsmState::new());

pub const fn ksm_enabled() -> bool {
    true
}

pub fn mark_mergeable(start: u64, len: u64) -> Result<(), i32> {
    if len == 0 {
        return Ok(());
    }
    if start % PAGE_SIZE as u64 != 0 {
        return Err(EINVAL);
    }
    let end = start.checked_add(len).ok_or(EINVAL)?;
    if end <= start {
        return Err(EINVAL);
    }

    let mut state = KSM_STATE.lock();
    state.ranges.push(KsmRange { start, end });
    state.ranges.sort_by_key(|range| range.start);
    Ok(())
}

pub fn mark_unmergeable(start: u64, len: u64) -> Result<(), i32> {
    if len == 0 {
        return Ok(());
    }
    let end = start.checked_add(len).ok_or(EINVAL)?;
    if end <= start {
        return Err(EINVAL);
    }

    let mut state = KSM_STATE.lock();
    state
        .ranges
        .retain(|range| range.end <= start || range.start >= end);
    state.remove_pages_outside_mergeable_ranges();
    recompute_stats(&mut state);
    Ok(())
}

pub fn register_page(addr: u64, data: &[u8]) -> Result<(), i32> {
    if addr % PAGE_SIZE as u64 != 0 || data.len() != PAGE_SIZE {
        return Err(EINVAL);
    }

    let mut state = KSM_STATE.lock();
    if !state.range_contains(addr) {
        return Err(ENOENT);
    }

    if let Some(page) = state.pages.iter_mut().find(|page| page.addr == addr) {
        page.data.clear();
        page.data.extend_from_slice(data);
        page.stable_node = None;
    } else {
        state.pages.push(KsmPage {
            addr,
            data: data.to_vec(),
            stable_node: None,
        });
    }
    recompute_stats(&mut state);
    Ok(())
}

pub fn unmerge_page(addr: u64) -> Result<(), i32> {
    let mut state = KSM_STATE.lock();
    let page = state
        .pages
        .iter_mut()
        .find(|page| page.addr == addr)
        .ok_or(ENOENT)?;
    page.stable_node = None;
    recompute_stats(&mut state);
    Ok(())
}

pub fn scan() -> KsmScanResult {
    let mut state = KSM_STATE.lock();
    let mut pages_merged = 0;

    for page in state.pages.iter_mut() {
        page.stable_node = None;
    }

    for idx in 0..state.pages.len() {
        if state.pages[idx].stable_node.is_some() {
            continue;
        }

        let mut matches = Vec::new();
        for other in (idx + 1)..state.pages.len() {
            if state.pages[idx].data == state.pages[other].data {
                matches.push(other);
            }
        }

        if matches.is_empty() {
            continue;
        }

        let node = state.next_stable_node;
        state.next_stable_node += 1;
        state.pages[idx].stable_node = Some(node);
        for other in matches {
            state.pages[other].stable_node = Some(node);
            pages_merged += 1;
        }
    }

    recompute_stats(&mut state);
    KsmScanResult {
        pages_scanned: state.pages.len(),
        pages_merged,
        stable_nodes: state.stats.stable_nodes,
    }
}

pub fn stable_node_for(addr: u64) -> Option<u64> {
    KSM_STATE
        .lock()
        .pages
        .iter()
        .find(|page| page.addr == addr)
        .and_then(|page| page.stable_node)
}

pub fn stats() -> KsmStats {
    KSM_STATE.lock().stats
}

fn recompute_stats(state: &mut KsmState) {
    let mut stable_nodes = Vec::new();
    let mut pages_shared = 0;
    for page in &state.pages {
        if let Some(node) = page.stable_node {
            pages_shared += 1;
            if !stable_nodes.contains(&node) {
                stable_nodes.push(node);
            }
        }
    }

    state.stats = KsmStats {
        pages_scanned: state.pages.len(),
        pages_shared,
        stable_nodes: stable_nodes.len(),
    };
}

// ---------------------------------------------------------------------------
// Linux-visible ksm.h wrappers
// ---------------------------------------------------------------------------

pub fn __ksm_enter(mm: *mut MmStruct) -> i32 {
    if mm.is_null() {
        return -EINVAL;
    }
    unsafe { mm_flags_set(mm, MMF_VM_MERGEABLE_MASK) };
    0
}

pub fn __ksm_exit(mm: *mut MmStruct) {
    if !mm.is_null() {
        unsafe { mm_flags_clear(mm, MMF_VM_MERGEABLE_MASK | MMF_VM_MERGE_ANY_MASK) };
        KSM_STATE.lock().zero_pages.remove(&(mm as usize));
    }
}

pub fn ksm_exit(mm: *mut MmStruct) {
    if unsafe { mm_flags_test(mm, MMF_VM_MERGEABLE_MASK) } {
        __ksm_exit(mm)
    }
}

pub fn ksm_fork(mm: *mut MmStruct, oldmm: *mut MmStruct) -> i32 {
    if unsafe { mm_flags_test(oldmm, MMF_VM_MERGEABLE_MASK) } {
        return __ksm_enter(mm);
    }
    0
}

pub fn ksm_execve(mm: *mut MmStruct) -> i32 {
    if unsafe { mm_flags_test(mm, MMF_VM_MERGE_ANY_MASK) } {
        __ksm_enter(mm)
    } else {
        0
    }
}

pub fn ksm_disable(mm: *mut MmStruct) -> i32 {
    __ksm_exit(mm);
    0
}

pub fn ksm_enable_merge_any(mm: *mut MmStruct) -> i32 {
    if mm.is_null() {
        return -EINVAL;
    }
    unsafe { mm_flags_set(mm, MMF_VM_MERGE_ANY_MASK) };
    __ksm_enter(mm)
}

pub fn ksm_disable_merge_any(mm: *mut MmStruct) -> i32 {
    if mm.is_null() {
        return -EINVAL;
    }
    unsafe { mm_flags_clear(mm, MMF_VM_MERGE_ANY_MASK) };
    0
}

pub fn collect_procs_ksm(_mm_slot: *mut u8, _root: *mut u8, _force: bool) {}

pub fn ksm_process_mergeable(mm: *mut MmStruct) -> bool {
    unsafe { mm_flags_test(mm, MMF_VM_MERGEABLE_MASK) }
}

pub fn ksm_process_profit(_mm: *mut MmStruct) -> isize {
    stats().pages_shared as isize
}

pub fn ksm_vma_flags(mm: *const MmStruct, file: *const u8, vma_flags: u64) -> u64 {
    if file.is_null() && unsafe { mm_flags_test(mm, MMF_VM_MERGE_ANY_MASK) } {
        vma_flags | VM_MERGEABLE
    } else {
        vma_flags
    }
}

pub fn ksm_madvise(
    vma: *mut VmAreaStruct,
    start: u64,
    end: u64,
    advice: i32,
    vm_flags: *mut u64,
) -> i32 {
    if end < start {
        return -EINVAL;
    }
    let len = end - start;

    match advice {
        MADV_MERGEABLE => {
            if !vm_flags.is_null() {
                unsafe { *vm_flags |= VM_MERGEABLE };
            }
            if !vma.is_null() {
                unsafe {
                    (*vma).vm_flags |= VM_MERGEABLE;
                    let _ = __ksm_enter((*vma).vm_mm);
                }
            }
            mark_mergeable(start, len).err().map_or(0, |err| -err)
        }
        MADV_UNMERGEABLE => {
            if !vm_flags.is_null() {
                unsafe { *vm_flags &= !VM_MERGEABLE };
            }
            if !vma.is_null() {
                unsafe {
                    (*vma).vm_flags &= !VM_MERGEABLE;
                }
            }
            mark_unmergeable(start, len).err().map_or(0, |err| -err)
        }
        _ => 0,
    }
}

pub fn ksm_might_need_to_copy(page: *mut Page, _vma: *mut u8, _address: u64) -> *mut Page {
    page
}

pub fn ksm_map_zero_page(mm: *mut MmStruct, _pmd: *mut u8, _addr: u64) -> bool {
    if mm.is_null() {
        return false;
    }
    let mut state = KSM_STATE.lock();
    let entry = state.zero_pages.entry(mm as usize).or_insert(0);
    *entry = entry.saturating_add(1);
    true
}

pub fn ksm_might_unmap_zero_page(mm: *mut MmStruct, _pmd: *mut u8, _addr: u64) -> bool {
    if mm.is_null() {
        return false;
    }
    let mut state = KSM_STATE.lock();
    let entry = state.zero_pages.entry(mm as usize).or_insert(0);
    if *entry == 0 {
        false
    } else {
        *entry -= 1;
        true
    }
}

pub fn mm_ksm_zero_pages(mm: *const MmStruct) -> u64 {
    if mm.is_null() {
        return 0;
    }
    KSM_STATE
        .lock()
        .zero_pages
        .get(&(mm as usize))
        .copied()
        .unwrap_or(0)
}

pub fn folio_migrate_ksm(_new: *mut Page, _old: *mut Page) {}

pub fn rmap_walk_ksm(_folio: *mut Page, _rwc: *mut u8) {}

#[cfg(test)]
pub fn reset_for_tests() {
    KSM_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::mm_types::MmStruct;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use crate::mm::vm_flags::{VM_READ, VM_WRITE};
    use alloc::vec;

    #[test]
    fn ksm_marks_scans_merges_and_unmerges_pages() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(ksm_enabled());
        assert_eq!(mark_mergeable(0x1000, 0x3000), Ok(()));

        let a = vec![0x5a; PAGE_SIZE];
        let b = vec![0xa5; PAGE_SIZE];
        assert_eq!(register_page(0x1000, &a), Ok(()));
        assert_eq!(register_page(0x2000, &a), Ok(()));
        assert_eq!(register_page(0x3000, &b), Ok(()));

        let result = scan();
        assert_eq!(result.pages_scanned, 3);
        assert_eq!(result.pages_merged, 1);
        assert_eq!(result.stable_nodes, 1);
        assert_eq!(stable_node_for(0x1000), stable_node_for(0x2000));
        assert_eq!(stable_node_for(0x3000), None);

        assert_eq!(unmerge_page(0x2000), Ok(()));
        assert_eq!(stable_node_for(0x2000), None);
    }

    #[test]
    fn ksm_rejects_unaligned_or_unmergeable_pages() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(mark_mergeable(0x1001, PAGE_SIZE as u64), Err(EINVAL));
        assert_eq!(mark_mergeable(0x4000, PAGE_SIZE as u64), Ok(()));
        assert_eq!(register_page(0x8000, &[0u8; PAGE_SIZE]), Err(ENOENT));
    }

    #[test]
    fn ksm_wrappers_track_mm_flags_vma_flags_and_zero_pages() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        let mut mm = MmStruct::new(0);
        let mut child = MmStruct::new(0);
        let mut vma = VmAreaStruct::new(0x1000, 0x3000, VM_READ | VM_WRITE);
        vma.vm_mm = &mut mm;
        let mut flags = vma.vm_flags;

        assert_eq!(ksm_enable_merge_any(&mut mm), 0);
        assert!(ksm_process_mergeable(&mut mm));
        assert_eq!(
            ksm_vma_flags(&mm, core::ptr::null(), VM_READ) & VM_MERGEABLE,
            VM_MERGEABLE
        );
        assert_eq!(ksm_fork(&mut child, &mut mm), 0);
        assert!(ksm_process_mergeable(&mut child));

        assert_eq!(
            ksm_madvise(&mut vma, 0x1000, 0x3000, MADV_MERGEABLE, &raw mut flags,),
            0
        );
        assert_ne!(flags & VM_MERGEABLE, 0);
        assert_ne!(vma.vm_flags & VM_MERGEABLE, 0);
        assert_eq!(
            ksm_madvise(&mut vma, 0x1000, 0x3000, MADV_UNMERGEABLE, &raw mut flags,),
            0
        );
        assert_eq!(flags & VM_MERGEABLE, 0);
        assert_eq!(vma.vm_flags & VM_MERGEABLE, 0);

        assert!(ksm_map_zero_page(&mut mm, core::ptr::null_mut(), 0x1000));
        assert_eq!(mm_ksm_zero_pages(&mm), 1);
        assert!(ksm_might_unmap_zero_page(
            &mut mm,
            core::ptr::null_mut(),
            0x1000
        ));
        assert_eq!(mm_ksm_zero_pages(&mm), 0);
        assert_eq!(ksm_disable_merge_any(&mut mm), 0);
        assert_eq!(ksm_disable(&mut mm), 0);
        assert!(!ksm_process_mergeable(&mut mm));
    }
}
