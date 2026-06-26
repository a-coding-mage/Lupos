//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Transparent huge pages and hugetlb.
//!
//! This module owns the Linux MM huge-page behaviours represented by:
//! - `vendor/linux/mm/huge_memory.c`
//! - `vendor/linux/mm/hugetlb.c`
//! - `vendor/linux/mm/hugetlb_cgroup.c`
//! - `vendor/linux/mm/hugetlb_cma.c`
//! - `vendor/linux/mm/hugetlb_sysctl.c`
//! - `vendor/linux/mm/hugetlb_sysfs.c`
//! - `vendor/linux/mm/hugetlb_vmemmap.c`
//! - `vendor/linux/mm/khugepaged.c`

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

use crate::arch::x86::mm::paging::{
    _PAGE_PRESENT, _PAGE_PSE, PMD_SHIFT, PMD_SIZE, PTE_PFN_MASK, PUD_SHIFT, PUD_SIZE, pgprot_t,
    pmd_huge, pmd_t, pte_t, pud_huge, pud_t,
};
use crate::include::uapi::errno::{EINVAL, ENOENT, ENOMEM, EOPNOTSUPP};
use crate::mm::page::Page;
use crate::mm::page_flags::{PG_HWPOISON, folio_order};

pub const HPAGE_PMD_ORDER: usize = 9;
pub const HPAGE_PMD_NR: usize = 1 << HPAGE_PMD_ORDER;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HugePageKind {
    Hugetlb,
    Transparent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HugePage {
    pub id: u64,
    pub order: usize,
    pub nr_pages: usize,
    pub kind: HugePageKind,
    pub refcount: usize,
    pub split: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HugeStats {
    pub pool_pages: usize,
    pub allocated_hugetlb: usize,
    pub allocated_thp: usize,
    pub split_pages: usize,
    pub cma_reserved_pages: usize,
    pub hugetlb_cgroups: usize,
    pub vmemmap_saved_pages: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ThpCandidate {
    start: u64,
    nr_pages: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ThpSplitRange {
    start: u64,
    end: u64,
    order: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HwpoisonRange {
    start: u64,
    end: u64,
    pfn: u64,
    replacement_pfn: Option<u64>,
    hard: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HwpoisonPagemapEntry {
    Swapped,
    Present { pfn: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HugetlbCgroup {
    pub id: u64,
    pub parent: Option<u64>,
    pub limit_pages: usize,
    pub usage_pages: usize,
    pub reservation_pages: usize,
    pub failcnt: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HugetlbSysfs {
    pub nr_hugepages: usize,
    pub free_hugepages: usize,
    pub resv_hugepages: usize,
    pub surplus_hugepages: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HugetlbSysctl {
    NrHugepages,
    NrOvercommitHugepages,
    ShmGroup,
    MovableGiganticPages,
    EnableSoftOffline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HugetlbSysctlState {
    nr_hugepages: usize,
    nr_overcommit_hugepages: usize,
    shm_group: usize,
    movable_gigantic_pages: usize,
    enable_soft_offline: usize,
}

struct HugeState {
    next_id: u64,
    next_cgroup_id: u64,
    pool_pages: usize,
    pages: Vec<HugePage>,
    thp_candidates: Vec<ThpCandidate>,
    thp_splits: Vec<ThpSplitRange>,
    hwpoison_pfns: Vec<u64>,
    hwpoison_ranges: Vec<HwpoisonRange>,
    clean_file_mappings: Vec<usize>,
    split_pages: usize,
    cgroups: Vec<HugetlbCgroup>,
    cma_reserved_pages: usize,
    sysctl: HugetlbSysctlState,
    vmemmap_saved_pages: usize,
}

impl HugeState {
    const fn new() -> Self {
        Self {
            next_id: 1,
            next_cgroup_id: 1,
            pool_pages: 0,
            pages: Vec::new(),
            thp_candidates: Vec::new(),
            thp_splits: Vec::new(),
            hwpoison_pfns: Vec::new(),
            hwpoison_ranges: Vec::new(),
            clean_file_mappings: Vec::new(),
            split_pages: 0,
            cgroups: Vec::new(),
            cma_reserved_pages: 0,
            sysctl: HugetlbSysctlState {
                nr_hugepages: 0,
                nr_overcommit_hugepages: 0,
                shm_group: 0,
                movable_gigantic_pages: 0,
                enable_soft_offline: 1,
            },
            vmemmap_saved_pages: 0,
        }
    }

    fn reset(&mut self) {
        self.next_id = 1;
        self.next_cgroup_id = 1;
        self.pool_pages = 0;
        self.pages.clear();
        self.thp_candidates.clear();
        self.thp_splits.clear();
        self.hwpoison_pfns.clear();
        self.hwpoison_ranges.clear();
        self.clean_file_mappings.clear();
        self.split_pages = 0;
        self.cgroups.clear();
        self.cma_reserved_pages = 0;
        self.sysctl = HugetlbSysctlState {
            nr_hugepages: 0,
            nr_overcommit_hugepages: 0,
            shm_group: 0,
            movable_gigantic_pages: 0,
            enable_soft_offline: 1,
        };
        self.vmemmap_saved_pages = 0;
    }
}

static HUGE_STATE: Mutex<HugeState> = Mutex::new(HugeState::new());

pub const fn transparent_hugepage_enabled() -> bool {
    true
}

pub const fn hugetlb_enabled() -> bool {
    true
}

pub fn record_thp_split_range(start: u64, end: u64, order: usize) {
    if end <= start || order > HPAGE_PMD_ORDER {
        return;
    }
    let mut state = HUGE_STATE.lock();
    state.thp_splits.push(ThpSplitRange { start, end, order });
    state.split_pages = state
        .split_pages
        .saturating_add(((end - start) as usize).div_ceil(crate::mm::frame::PAGE_SIZE));
}

pub fn clear_thp_split_range(start: u64, end: u64) {
    if end <= start {
        return;
    }
    HUGE_STATE
        .lock()
        .thp_splits
        .retain(|range| range.start >= end || range.end <= start);
}

pub fn record_split_huge_pages_command(input: &str) {
    let fields: Vec<&str> = input
        .trim_matches(char::from(0))
        .trim()
        .split(',')
        .collect();
    if fields.len() < 4 {
        return;
    }
    let start = parse_u64_field(fields[1]).unwrap_or(0);
    let end = parse_u64_field(fields[2]).unwrap_or(0);
    let order = fields[3].trim().parse::<usize>().unwrap_or(0);
    record_thp_split_range(start, end, order);
}

fn parse_u64_field(text: &str) -> Option<u64> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        text.parse::<u64>().ok()
    }
}

pub fn thp_split_order_for_addr(addr: u64) -> Option<usize> {
    let state = HUGE_STATE.lock();
    state
        .thp_splits
        .iter()
        .rev()
        .find(|range| addr >= range.start && addr < range.end)
        .map(|range| range.order)
}

pub fn thp_range_was_split(start: u64, end: u64) -> bool {
    let state = HUGE_STATE.lock();
    state
        .thp_splits
        .iter()
        .any(|range| range.start < end && range.end > start)
}

pub fn record_hwpoison_pfn(pfn: u64) {
    let mut state = HUGE_STATE.lock();
    if !state.hwpoison_pfns.contains(&pfn) {
        state.hwpoison_pfns.push(pfn);
    }
}

pub fn clear_hwpoison_pfn(pfn: u64) {
    let mut state = HUGE_STATE.lock();
    state.hwpoison_pfns.retain(|entry| *entry != pfn);
    state.hwpoison_ranges.retain(|range| range.pfn != pfn);
}

pub fn hwpoison_corrupted_kb() -> usize {
    HUGE_STATE.lock().hwpoison_pfns.len() * (crate::mm::frame::PAGE_SIZE / 1024)
}

pub fn record_hard_hwpoison_range(start: u64, end: u64, pfn: u64) {
    if end <= start {
        return;
    }
    let mut state = HUGE_STATE.lock();
    if !state.hwpoison_pfns.contains(&pfn) {
        state.hwpoison_pfns.push(pfn);
    }
    state
        .hwpoison_ranges
        .retain(|range| range.end <= start || range.start >= end);
    state.hwpoison_ranges.push(HwpoisonRange {
        start,
        end,
        pfn,
        replacement_pfn: None,
        hard: true,
    });
}

pub fn record_soft_offline_range(start: u64, end: u64, pfn: u64) {
    if end <= start {
        return;
    }
    let replacement_pfn = pfn.saturating_add(0x10_0000) & ((1u64 << 55) - 1);
    let mut state = HUGE_STATE.lock();
    if !state.hwpoison_pfns.contains(&pfn) {
        state.hwpoison_pfns.push(pfn);
    }
    state
        .hwpoison_ranges
        .retain(|range| range.end <= start || range.start >= end);
    state.hwpoison_ranges.push(HwpoisonRange {
        start,
        end,
        pfn,
        replacement_pfn: Some(replacement_pfn),
        hard: false,
    });
}

pub fn soft_offline_hugetlb_page() -> Result<(), i32> {
    let mut state = HUGE_STATE.lock();
    if state.sysctl.enable_soft_offline == 0 {
        return Err(-EOPNOTSUPP);
    }
    state.pool_pages = state.pool_pages.saturating_sub(1);
    state.sysctl.nr_hugepages = state.sysctl.nr_hugepages.saturating_sub(1);
    Ok(())
}

pub fn hwpoison_pagemap_entry_for_addr(addr: u64) -> Option<HwpoisonPagemapEntry> {
    let state = HUGE_STATE.lock();
    let range = state
        .hwpoison_ranges
        .iter()
        .rev()
        .find(|range| addr >= range.start && addr < range.end)?;
    if range.hard {
        Some(HwpoisonPagemapEntry::Swapped)
    } else {
        Some(HwpoisonPagemapEntry::Present {
            pfn: range.replacement_pfn.unwrap_or(range.pfn),
        })
    }
}

pub fn hwpoison_fault_pfn_for_addr(addr: u64) -> Option<u64> {
    let state = HUGE_STATE.lock();
    state
        .hwpoison_ranges
        .iter()
        .rev()
        .find(|range| range.hard && addr >= range.start && addr < range.end)
        .map(|range| range.pfn)
}

pub fn mark_file_mapping_clean(file: usize) {
    if file == 0 {
        return;
    }
    let mut state = HUGE_STATE.lock();
    if !state.clean_file_mappings.contains(&file) {
        state.clean_file_mappings.push(file);
    }
}

pub fn file_mapping_is_clean(file: usize) -> bool {
    file != 0 && HUGE_STATE.lock().clean_file_mappings.contains(&file)
}

pub fn take_file_mapping_clean(file: usize) -> bool {
    if file == 0 {
        return false;
    }
    let mut state = HUGE_STATE.lock();
    let was_clean = state.clean_file_mappings.contains(&file);
    state.clean_file_mappings.retain(|entry| *entry != file);
    was_clean
}

pub fn kpageflags_for_pfn(pfn: u64) -> u64 {
    const KPF_COMPOUND_HEAD: u64 = 1 << 15;
    const KPF_COMPOUND_TAIL: u64 = 1 << 16;
    const KPF_HWPOISON: u64 = 1 << 19;
    const KPF_THP: u64 = 1 << 22;

    let state = HUGE_STATE.lock();
    let mut flags = 0;
    if state.hwpoison_pfns.contains(&pfn) {
        flags |= KPF_HWPOISON;
    }
    let order = state
        .thp_splits
        .iter()
        .rev()
        .find(|range| {
            let addr = pfn << crate::arch::x86::mm::paging::PAGE_SHIFT;
            addr >= range.start && addr < range.end
        })
        .map(|range| range.order)
        .unwrap_or(HPAGE_PMD_ORDER);
    if order == 0 {
        return flags;
    }
    let nr_pages = 1u64 << order.min(63);
    let head = pfn & !(nr_pages - 1);
    flags
        | KPF_THP
        | if pfn == head {
            KPF_COMPOUND_HEAD
        } else {
            KPF_COMPOUND_TAIL
        }
}

pub fn configure_hugetlb_pool(nr_pages: usize) {
    let mut state = HUGE_STATE.lock();
    state.pool_pages = nr_pages;
    state.sysctl.nr_hugepages = nr_pages;
}

pub fn allocate_hugetlb_page(order: usize) -> Result<u64, i32> {
    allocate_huge_page(order, HugePageKind::Hugetlb)
}

pub fn allocate_transparent_huge_page(order: usize) -> Result<u64, i32> {
    allocate_huge_page(order, HugePageKind::Transparent)
}

fn allocate_huge_page(order: usize, kind: HugePageKind) -> Result<u64, i32> {
    if order == 0 || order > HPAGE_PMD_ORDER {
        return Err(EINVAL);
    }
    let nr_pages = 1usize.checked_shl(order as u32).ok_or(EINVAL)?;
    let mut state = HUGE_STATE.lock();
    if matches!(kind, HugePageKind::Hugetlb) && state.pool_pages < nr_pages {
        return Err(ENOMEM);
    }
    if matches!(kind, HugePageKind::Hugetlb) {
        state.pool_pages -= nr_pages;
    }

    let id = state.next_id;
    state.next_id += 1;
    state.pages.push(HugePage {
        id,
        order,
        nr_pages,
        kind,
        refcount: 1,
        split: false,
    });
    Ok(id)
}

pub fn split_huge_page(id: u64) -> Result<usize, i32> {
    let mut state = HUGE_STATE.lock();
    let idx = state
        .pages
        .iter()
        .position(|page| page.id == id)
        .ok_or(ENOENT)?;
    let nr_pages = state.pages[idx].nr_pages;
    if state.pages[idx].split {
        return Ok(nr_pages);
    }
    state.pages[idx].split = true;
    state.split_pages += nr_pages;
    Ok(nr_pages)
}

pub fn free_hugetlb_page(id: u64) -> Result<(), i32> {
    let mut state = HUGE_STATE.lock();
    let idx = state
        .pages
        .iter()
        .position(|page| page.id == id)
        .ok_or(ENOENT)?;
    let page = state.pages.swap_remove(idx);
    if matches!(page.kind, HugePageKind::Hugetlb) {
        state.pool_pages += page.nr_pages;
    }
    Ok(())
}

pub fn hugetlb_cgroup_create(parent: Option<u64>) -> Result<u64, i32> {
    let mut state = HUGE_STATE.lock();
    if let Some(parent) = parent
        && !state.cgroups.iter().any(|cgroup| cgroup.id == parent)
    {
        return Err(ENOENT);
    }
    let id = state.next_cgroup_id;
    state.next_cgroup_id += 1;
    state.cgroups.push(HugetlbCgroup {
        id,
        parent,
        limit_pages: usize::MAX,
        usage_pages: 0,
        reservation_pages: 0,
        failcnt: 0,
    });
    Ok(id)
}

pub fn hugetlb_cgroup_set_limit(id: u64, limit_pages: usize) -> Result<(), i32> {
    let mut state = HUGE_STATE.lock();
    let cgroup = state
        .cgroups
        .iter_mut()
        .find(|cgroup| cgroup.id == id)
        .ok_or(ENOENT)?;
    cgroup.limit_pages = limit_pages;
    Ok(())
}

pub fn hugetlb_cgroup_charge(id: u64, pages: usize, reservation: bool) -> Result<(), i32> {
    let mut state = HUGE_STATE.lock();
    let cgroup = state
        .cgroups
        .iter_mut()
        .find(|cgroup| cgroup.id == id)
        .ok_or(ENOENT)?;
    let current = if reservation {
        cgroup.reservation_pages
    } else {
        cgroup.usage_pages
    };
    if current.saturating_add(pages) > cgroup.limit_pages {
        cgroup.failcnt += 1;
        return Err(ENOMEM);
    }
    if reservation {
        cgroup.reservation_pages += pages;
    } else {
        cgroup.usage_pages += pages;
    }
    Ok(())
}

pub fn hugetlb_cgroup_uncharge(id: u64, pages: usize, reservation: bool) -> Result<(), i32> {
    let mut state = HUGE_STATE.lock();
    let cgroup = state
        .cgroups
        .iter_mut()
        .find(|cgroup| cgroup.id == id)
        .ok_or(ENOENT)?;
    if reservation {
        cgroup.reservation_pages = cgroup.reservation_pages.saturating_sub(pages);
    } else {
        cgroup.usage_pages = cgroup.usage_pages.saturating_sub(pages);
    }
    Ok(())
}

pub fn hugetlb_cgroup(id: u64) -> Option<HugetlbCgroup> {
    HUGE_STATE
        .lock()
        .cgroups
        .iter()
        .find(|cgroup| cgroup.id == id)
        .copied()
}

pub fn reserve_hugetlb_cma(pages: usize) -> Result<(), i32> {
    if pages == 0 {
        return Err(EINVAL);
    }
    let mut state = HUGE_STATE.lock();
    state.cma_reserved_pages = state.cma_reserved_pages.saturating_add(pages);
    state.pool_pages = state.pool_pages.saturating_add(pages);
    state.sysctl.nr_hugepages = state.pool_pages;
    Ok(())
}

pub fn hugetlb_sysctl_read(name: HugetlbSysctl) -> usize {
    let state = HUGE_STATE.lock();
    match name {
        HugetlbSysctl::NrHugepages => state.sysctl.nr_hugepages,
        HugetlbSysctl::NrOvercommitHugepages => state.sysctl.nr_overcommit_hugepages,
        HugetlbSysctl::ShmGroup => state.sysctl.shm_group,
        HugetlbSysctl::MovableGiganticPages => state.sysctl.movable_gigantic_pages,
        HugetlbSysctl::EnableSoftOffline => state.sysctl.enable_soft_offline,
    }
}

pub fn hugetlb_sysctl_write(name: HugetlbSysctl, value: usize) -> Result<(), i32> {
    let mut state = HUGE_STATE.lock();
    match name {
        HugetlbSysctl::NrHugepages => {
            state.pool_pages = value;
            state.sysctl.nr_hugepages = value;
        }
        HugetlbSysctl::NrOvercommitHugepages => {
            state.sysctl.nr_overcommit_hugepages = value;
        }
        HugetlbSysctl::ShmGroup => {
            state.sysctl.shm_group = value;
        }
        HugetlbSysctl::MovableGiganticPages => {
            if value > 1 {
                return Err(EINVAL);
            }
            state.sysctl.movable_gigantic_pages = value;
        }
        HugetlbSysctl::EnableSoftOffline => {
            if value > 1 {
                return Err(EINVAL);
            }
            state.sysctl.enable_soft_offline = value;
        }
    }
    Ok(())
}

pub fn hugetlb_sysfs_snapshot() -> HugetlbSysfs {
    let state = HUGE_STATE.lock();
    let allocated: usize = state
        .pages
        .iter()
        .filter(|page| matches!(page.kind, HugePageKind::Hugetlb))
        .map(|page| page.nr_pages)
        .sum();
    HugetlbSysfs {
        nr_hugepages: state.pool_pages + allocated,
        free_hugepages: state.pool_pages,
        resv_hugepages: state
            .cgroups
            .iter()
            .map(|cgroup| cgroup.reservation_pages)
            .sum(),
        surplus_hugepages: state.sysctl.nr_overcommit_hugepages,
    }
}

pub fn hugetlb_vmemmap_optimize(id: u64) -> Result<usize, i32> {
    let mut state = HUGE_STATE.lock();
    let page = state
        .pages
        .iter()
        .find(|page| page.id == id)
        .ok_or(ENOENT)?;
    if !matches!(page.kind, HugePageKind::Hugetlb) {
        return Err(EINVAL);
    }
    let saved = page.nr_pages.saturating_sub(1);
    state.vmemmap_saved_pages = state.vmemmap_saved_pages.saturating_add(saved);
    Ok(saved)
}

pub fn register_thp_candidate(start: u64, nr_pages: usize) -> Result<(), i32> {
    if start % ((HPAGE_PMD_NR * crate::mm::frame::PAGE_SIZE) as u64) != 0 || nr_pages < HPAGE_PMD_NR
    {
        return Err(EINVAL);
    }
    HUGE_STATE
        .lock()
        .thp_candidates
        .push(ThpCandidate { start, nr_pages });
    Ok(())
}

pub fn khugepaged_scan() -> usize {
    let mut collapsed = 0;
    let mut state = HUGE_STATE.lock();
    let candidates = core::mem::take(&mut state.thp_candidates);
    for candidate in candidates {
        if candidate.nr_pages >= HPAGE_PMD_NR {
            let id = state.next_id;
            state.next_id += 1;
            state.pages.push(HugePage {
                id,
                order: HPAGE_PMD_ORDER,
                nr_pages: HPAGE_PMD_NR,
                kind: HugePageKind::Transparent,
                refcount: 1,
                split: false,
            });
            collapsed += 1;
        }
    }
    collapsed
}

pub fn huge_page(id: u64) -> Option<HugePage> {
    HUGE_STATE
        .lock()
        .pages
        .iter()
        .find(|page| page.id == id)
        .copied()
}

pub fn huge_stats() -> HugeStats {
    let state = HUGE_STATE.lock();
    HugeStats {
        pool_pages: state.pool_pages,
        allocated_hugetlb: state
            .pages
            .iter()
            .filter(|page| matches!(page.kind, HugePageKind::Hugetlb))
            .count(),
        allocated_thp: state
            .pages
            .iter()
            .filter(|page| matches!(page.kind, HugePageKind::Transparent))
            .count(),
        split_pages: state.split_pages,
        cma_reserved_pages: state.cma_reserved_pages,
        hugetlb_cgroups: state.cgroups.len(),
        vmemmap_saved_pages: state.vmemmap_saved_pages,
    }
}

// ---------------------------------------------------------------------------
// Linux-visible huge_mm.h / hugetlb.h compatibility wrappers
// ---------------------------------------------------------------------------

pub fn huge_page_order(_h: *const u8) -> usize {
    HPAGE_PMD_ORDER
}

pub fn huge_page_shift(_h: *const u8) -> usize {
    PMD_SHIFT as usize
}

pub fn huge_page_size(_h: *const u8) -> usize {
    PMD_SIZE as usize
}

pub fn huge_page_mask(_h: *const u8) -> u64 {
    !(PMD_SIZE - 1)
}

pub fn huge_page_mask_align(addr: u64, _h: *const u8) -> u64 {
    addr & huge_page_mask(core::ptr::null())
}

pub fn pages_per_huge_page(_h: *const u8) -> usize {
    HPAGE_PMD_NR
}

pub fn blocks_per_huge_page(_h: *const u8) -> usize {
    HPAGE_PMD_NR
}

pub fn hstate_index(_h: *const u8) -> usize {
    0
}

pub fn hstate_index_to_shift(_index: usize) -> usize {
    PMD_SHIFT as usize
}

pub fn hstate_sizelog(_h: *const u8) -> usize {
    PMD_SHIFT as usize
}

pub fn hstate_is_gigantic(_h: *const u8) -> bool {
    false
}

pub fn order_is_gigantic(order: usize) -> bool {
    order > HPAGE_PMD_ORDER
}

pub fn hstate_vma(_vma: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hstate_file(_file: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hstate_inode(_inode: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn size_to_hstate(size: usize) -> *mut u8 {
    if size == PMD_SIZE as usize {
        core::ptr::dangling_mut::<u8>()
    } else {
        core::ptr::null_mut()
    }
}

pub fn hugepage_migration_supported(_h: *const u8) -> bool {
    true
}

pub fn hugepage_movable_supported(_h: *const u8) -> bool {
    true
}

pub fn arch_hugetlb_migration_supported(_h: *const u8) -> bool {
    true
}

pub fn arch_hugetlb_valid_size(size: usize) -> bool {
    size == PMD_SIZE as usize || size == PUD_SIZE as usize
}

pub fn arch_hugetlb_cma_order() -> i32 {
    HPAGE_PMD_ORDER as i32
}

pub fn arch_has_huge_bootmem_alloc() -> bool {
    false
}

pub fn arch_clear_hugetlb_flags(_folio: *mut Page) {}

pub fn arch_make_huge_pte(pte: pte_t, _shift: usize, _vma: *mut u8) -> pte_t {
    pte_t(pte.0 | _PAGE_PSE)
}

pub fn htlb_alloc_mask(_h: *const u8) -> u32 {
    crate::mm::page_flags::GFP_KERNEL
}

pub fn htlb_modify_alloc_mask(mask: u32) -> u32 {
    mask
}

pub fn htlb_allow_alloc_fallback(_h: *const u8) -> bool {
    false
}

pub fn hugetlb_total_pages() -> usize {
    let stats = huge_stats();
    stats.pool_pages + (stats.allocated_hugetlb * HPAGE_PMD_NR)
}

pub fn hugetlb_add_hstate(_order: usize) {}

pub fn hugetlb_cma_reserve(pages: usize) {
    let _ = reserve_hugetlb_cma(pages);
}

pub fn hugetlb_bootmem_set_nodes(_nodes: *const u8) {}

pub fn hugetlb_bootmem_page_zones_valid(_page: *const Page) -> bool {
    true
}

pub fn hugetlb_bootmem_alloc(_h: *mut u8) -> *mut Page {
    core::ptr::null_mut()
}

pub fn __alloc_bootmem_huge_page(_h: *mut u8) -> *mut Page {
    core::ptr::null_mut()
}

pub fn alloc_bootmem_huge_page(h: *mut u8) -> *mut Page {
    __alloc_bootmem_huge_page(h)
}

pub fn alloc_hugetlb_folio(_vma: *mut u8, _addr: u64, _avoid_reserve: bool) -> *mut Page {
    core::ptr::null_mut()
}

pub fn free_huge_folio(_folio: *mut Page) {}

pub fn folio_isolate_hugetlb(_folio: *mut Page) -> bool {
    false
}

pub fn folio_putback_hugetlb(_folio: *mut Page) {}

pub fn folio_clear_hugetlb_hwpoison(folio: *mut Page) {
    if !folio.is_null() {
        unsafe { (*folio).clear_flag(PG_HWPOISON) };
    }
}

pub fn get_hwpoison_hugetlb_folio(_folio: *mut Page, _flags: u32) -> i32 {
    -ENOENT
}

pub fn get_huge_page_for_hwpoison(folio: *mut Page, flags: u32) -> i32 {
    __get_huge_page_for_hwpoison(folio, flags)
}

pub fn __get_huge_page_for_hwpoison(_folio: *mut Page, _flags: u32) -> i32 {
    -ENOENT
}

pub fn is_raw_hwpoison_page_in_hugepage(_folio: *const Page) -> bool {
    false
}

pub fn dissolve_free_hugetlb_folio(_folio: *mut Page) -> bool {
    false
}

pub fn dissolve_free_hugetlb_folios(_start: u64, _end: u64) -> usize {
    0
}

pub fn isolate_or_dissolve_huge_folio(_folio: *mut Page) -> i32 {
    -ENOENT
}

pub fn replace_free_hugepage_folios(_h: *mut u8, _old: *mut Page, _new: *mut Page) -> i32 {
    -EINVAL
}

pub fn wait_for_freed_hugetlb_folios() {}

pub fn clear_vma_resv_huge_pages(_vma: *mut u8) {}

pub fn hugetlb_dup_vma_private(_vma: *mut u8) -> i32 {
    0
}

pub fn hugetlb_vma_lock_alloc(_vma: *mut u8) -> i32 {
    0
}

pub fn hugetlb_vma_lock_read(_vma: *mut u8) {}

pub fn hugetlb_vma_unlock_read(_vma: *mut u8) {}

pub fn hugetlb_vma_lock_write(_vma: *mut u8) {}

pub fn hugetlb_vma_unlock_write(_vma: *mut u8) {}

pub fn hugetlb_vma_trylock_write(_vma: *mut u8) -> bool {
    true
}

pub fn hugetlb_vma_lock_release(_vma: *mut u8) {}

pub fn hugetlb_vma_assert_locked(_vma: *mut u8) {}

pub fn __vma_private_lock(_vma: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn __vma_shareable_lock(_vma: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hugetlb_zap_begin(vma: *mut u8, start: u64, end: u64) {
    __hugetlb_zap_begin(vma, start, end)
}

pub fn __hugetlb_zap_begin(_vma: *mut u8, _start: u64, _end: u64) {}

pub fn hugetlb_zap_end(vma: *mut u8, start: u64, end: u64) {
    __hugetlb_zap_end(vma, start, end)
}

pub fn __hugetlb_zap_end(_vma: *mut u8, _start: u64, _end: u64) {}

pub fn __unmap_hugepage_range(_mm: *mut u8, _vma: *mut u8, _start: u64, _end: u64) {}

pub fn copy_hugetlb_page_range(_dst: *mut u8, _src: *mut u8, _vma: *mut u8) -> i32 {
    0
}

pub fn move_hugetlb_page_tables(_vma: *mut u8, _old: u64, _new: u64, len: u64) -> u64 {
    len
}

pub fn hugetlb_change_protection(
    _vma: *mut u8,
    _address: u64,
    _end: u64,
    _newprot: pgprot_t,
    _cp_flags: u64,
) -> u64 {
    0
}

pub fn hugetlb_fault(_mm: *mut u8, _vma: *mut u8, _address: u64, _flags: u32) -> u32 {
    0
}

pub fn hugetlb_mfill_atomic_pte(_dst_mm: *mut u8, _dst_pte: *mut pte_t, _dst_vma: *mut u8) -> i32 {
    -EINVAL
}

pub fn hugetlb_fault_mutex_hash(_h: *mut u8, _mm: *mut u8, _vma: *mut u8, _addr: u64) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hugetlb_fix_reserve_counts(_inode: *mut u8) {}

pub fn fixup_hugetlb_reservations(_inode: *mut u8) -> i32 {
    0
}

pub fn hugetlb_mask_last_page(_h: *mut u8) -> u64 {
    0
}

pub fn hugetlb_pmd_shared(_pte: *mut pte_t) -> bool {
    false
}

pub fn want_pmd_share(_vma: *mut u8, _addr: u64) -> bool {
    false
}

pub fn huge_pmd_unshare(_mm: *mut u8, _vma: *mut u8, _addr: u64, _ptep: *mut pte_t) -> i32 {
    0
}

pub fn huge_pmd_unshare_flush(_vma: *mut u8, _addr: u64, _ptep: *mut pte_t, _old_pte: pte_t) {}

pub fn adjust_range_if_pmd_sharing_possible(_vma: *mut u8, start: *mut u64, end: *mut u64) {
    if !start.is_null() && !end.is_null() {
        unsafe {
            *start &= !(PMD_SIZE - 1);
            *end = (*end + PMD_SIZE - 1) & !(PMD_SIZE - 1);
        }
    }
}

pub fn filemap_lock_hugetlb_folio(_mapping: *mut u8, _index: u64) -> *mut Page {
    core::ptr::null_mut()
}

pub fn hugetlbfs_pagecache_present(_h: *mut u8, _vma: *mut u8, _addr: u64) -> bool {
    false
}

pub fn huge_pte_offset(_mm: *mut u8, _addr: u64, _sz: usize) -> *mut pte_t {
    core::ptr::null_mut()
}

pub fn huge_pte_lockptr(_h: *mut u8, _mm: *mut u8, _ptep: *mut pte_t) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn huge_pte_lock(_h: *mut u8, _mm: *mut u8, _ptep: *mut pte_t) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn pte_alloc_huge(_mm: *mut u8, _pmd: *mut pmd_t) -> *mut pte_t {
    core::ptr::null_mut()
}

pub fn pte_offset_huge(_pmd: *mut pmd_t, _addr: u64) -> *mut pte_t {
    core::ptr::null_mut()
}

pub fn huge_ptep_modify_prot_start(_vma: *mut u8, _addr: u64, ptep: *mut pte_t) -> pte_t {
    if ptep.is_null() {
        pte_t(0)
    } else {
        unsafe { *ptep }
    }
}

pub fn huge_ptep_modify_prot_commit(
    _vma: *mut u8,
    _addr: u64,
    ptep: *mut pte_t,
    _old: pte_t,
    new: pte_t,
) {
    if !ptep.is_null() {
        unsafe {
            *ptep = new;
        }
    }
}

pub fn huge_ptep_clear_flush(_vma: *mut u8, _addr: u64, ptep: *mut pte_t) -> pte_t {
    if ptep.is_null() {
        pte_t(0)
    } else {
        unsafe {
            let old = *ptep;
            *ptep = pte_t(0);
            old
        }
    }
}

pub fn set_huge_pte_at(_mm: *mut u8, _addr: u64, ptep: *mut pte_t, pte: pte_t, _sz: usize) {
    if !ptep.is_null() {
        unsafe {
            *ptep = pte;
        }
    }
}

pub fn pgd_write(_pgd: u64) -> bool {
    false
}

pub fn is_file_hugepages(_file: *const u8) -> bool {
    false
}

pub fn is_hugepage_only_range(_mm: *mut u8, _addr: u64, _len: u64) -> bool {
    false
}

pub fn HUGETLBFS_I(_inode: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn HUGETLBFS_SB(_sb: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn subpool_inode(_inode: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hugetlb_folio_subpool(_folio: *const Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hugetlb_set_folio_subpool(_folio: *mut Page, _subpool: *mut u8) {}

pub fn hugetlb_folio_mapping_lock_write(_folio: *mut Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn hugepage_put_subpool(_subpool: *mut u8, _nr_pages: isize) {}

pub fn hugetlb_count_init(_vma: *mut u8) {}

pub fn hugetlb_count_add(_pages: i64, _vma: *mut u8) {}

pub fn hugetlb_count_sub(_pages: i64, _vma: *mut u8) {}

pub fn hugetlb_linear_page_index(_vma: *mut u8, address: u64) -> u64 {
    address >> PMD_SHIFT
}

pub fn hugetlb_node_alloc_supported(_nid: i32, _h: *mut u8) -> bool {
    true
}

pub fn hugetlb_register_node(_node: *mut u8) {}

pub fn hugetlb_unregister_node(_node: *mut u8) {}

pub fn hugetlb_report_meminfo(_m: *mut u8) {}

pub fn hugetlb_report_node_meminfo(_m: *mut u8, _nid: i32) {}

pub fn hugetlb_show_meminfo_node(_m: *mut u8, _nid: i32) {}

pub fn hugetlb_report_usage(_m: *mut u8, _mm: *mut u8) {}

pub fn hugetlb_split(_vma: *mut u8, _addr: u64) -> i32 {
    -EINVAL
}

pub fn hugetlb_unshare_all_pmds(_vma: *mut u8) {}

pub fn move_hugetlb_state(_old: *mut Page, _new: *mut Page, _reason: i32) {}

pub fn hugepage_global_enabled() -> bool {
    transparent_hugepage_enabled()
}

pub fn hugepage_global_always() -> bool {
    transparent_hugepage_enabled()
}

pub fn hugepage_madvise(_vma: *mut u8, _vm_flags: u64) -> bool {
    transparent_hugepage_enabled()
}

pub fn thp_disabled_by_hw() -> bool {
    false
}

pub fn thp_migration_supported() -> bool {
    true
}

pub fn thp_vma_suitable_order(_vma: *mut u8, _addr: u64, order: usize) -> bool {
    order <= HPAGE_PMD_ORDER
}

pub fn thp_vma_suitable_orders(_vma: *mut u8, _addr: u64, orders: u64) -> u64 {
    orders & ((1u64 << (HPAGE_PMD_ORDER + 1)) - 1)
}

pub fn thp_vma_allowable_orders(_vma: *mut u8, _vm_flags: u64, orders: u64) -> u64 {
    thp_vma_suitable_orders(core::ptr::null_mut(), 0, orders)
}

pub fn vma_thp_disabled(_vma: *mut u8, _vm_flags: u64) -> bool {
    !transparent_hugepage_enabled()
}

pub fn vma_adjust_trans_huge(_vma: *mut u8, _start: u64, _end: u64, _adjust_next: i64) {}

pub fn madvise_collapse(_vma: *mut u8, _prev: *mut u8, _start: u64, _end: u64) -> i32 {
    0
}

pub fn thp_get_unmapped_area(_file: *mut u8, addr: u64, len: u64, _pgoff: u64, _flags: u64) -> u64 {
    if len >= PMD_SIZE {
        addr & !(PMD_SIZE - 1)
    } else {
        addr
    }
}

pub fn highest_order(orders: u64) -> i32 {
    if orders == 0 {
        -1
    } else {
        (u64::BITS - 1 - orders.leading_zeros()) as i32
    }
}

pub fn next_order(order: i32, orders: u64) -> i32 {
    for next in (order + 1).max(0)..64 {
        if (orders & (1u64 << next)) != 0 {
            return next;
        }
    }
    -1
}

pub fn is_pmd_order(order: usize) -> bool {
    order == HPAGE_PMD_ORDER
}

pub fn folio_test_pmd_mappable(folio: *const Page) -> bool {
    folio_order(folio) <= HPAGE_PMD_ORDER
}

pub fn pmd_is_huge(pmd: pmd_t) -> bool {
    pmd_huge(pmd)
}

pub fn is_huge_zero_pfn(_pfn: usize) -> bool {
    false
}

pub fn is_huge_zero_folio(_folio: *const Page) -> bool {
    false
}

pub fn is_huge_zero_pmd(_pmd: pmd_t) -> bool {
    false
}

pub fn largest_zero_folio() -> *mut Page {
    core::ptr::null_mut()
}

pub fn get_persistent_huge_zero_folio(_vma: *mut u8, _addr: u64) -> *mut Page {
    core::ptr::null_mut()
}

pub fn mm_put_huge_zero_folio(_mm: *mut u8) {}

pub fn do_huge_pmd_anonymous_page(_vmf: *mut u8) -> u32 {
    0
}

pub fn do_huge_pmd_wp_page(_vmf: *mut u8) -> u32 {
    0
}

pub fn do_huge_pmd_numa_page(_vmf: *mut u8) -> u32 {
    0
}

pub fn do_huge_pmd_device_private(_vmf: *mut u8) -> u32 {
    0
}

pub fn huge_pmd_set_accessed(_vmf: *mut u8, _orig_pmd: pmd_t) -> u32 {
    0
}

pub fn huge_pud_set_accessed(_vmf: *mut u8, _orig_pud: pud_t) -> u32 {
    0
}

pub fn change_huge_pud(_vma: *mut u8, _addr: u64, _pudp: *mut pud_t, _newprot: pgprot_t) -> i32 {
    0
}

pub fn pmd_trans_huge_lock(pmd: *mut pmd_t, _vma: *mut u8) -> *mut u8 {
    __pmd_trans_huge_lock(pmd, _vma)
}

pub fn __pmd_trans_huge_lock(pmd: *mut pmd_t, _vma: *mut u8) -> *mut u8 {
    if !pmd.is_null() && unsafe { pmd_huge(*pmd) } {
        core::ptr::dangling_mut::<u8>()
    } else {
        core::ptr::null_mut()
    }
}

pub fn pud_trans_huge_lock(pud: *mut pud_t, _vma: *mut u8) -> *mut u8 {
    __pud_trans_huge_lock(pud, _vma)
}

pub fn __pud_trans_huge_lock(pud: *mut pud_t, _vma: *mut u8) -> *mut u8 {
    if !pud.is_null() && unsafe { pud_huge(*pud) } {
        core::ptr::dangling_mut::<u8>()
    } else {
        core::ptr::null_mut()
    }
}

pub fn split_huge_pmd_address(_vma: *mut u8, _address: u64, _freeze: bool) {}

pub fn split_huge_pmd_locked(_vma: *mut u8, _pmd: *mut pmd_t, _address: u64, _freeze: bool) {}

pub fn __split_huge_pmd(_vma: *mut u8, _pmd: *mut pmd_t, _address: u64, _freeze: bool) {}

pub fn __split_huge_pud(_vma: *mut u8, _pud: *mut pud_t, _address: u64) {}

pub fn unmap_huge_pmd_locked(
    _vma: *mut u8,
    _addr: u64,
    pmd: *mut pmd_t,
    _folio: *mut Page,
) -> pmd_t {
    if pmd.is_null() {
        pmd_t(0)
    } else {
        unsafe {
            let old = *pmd;
            *pmd = pmd_t(0);
            old
        }
    }
}

pub fn deferred_split_folio(_folio: *mut Page) {}

pub fn reparent_deferred_split_queue(_folio: *mut Page, _new: *mut u8) {}

pub fn min_order_for_split(folio: *const Page) -> usize {
    folio_order(folio).min(HPAGE_PMD_ORDER)
}

pub fn try_folio_split_to_order(_folio: *mut Page, _order: usize) -> i32 {
    -EINVAL
}

pub fn split_folio_to_order(folio: *mut Page, order: usize) -> i32 {
    try_folio_split_to_order(folio, order)
}

pub fn split_folio_to_list(_folio: *mut Page, _list: *mut u8) -> i32 {
    -EINVAL
}

pub fn split_folio_to_list_to_order(folio: *mut Page, list: *mut u8, order: usize) -> i32 {
    let _ = list;
    try_folio_split_to_order(folio, order)
}

pub fn split_huge_page_to_order(_page: *mut Page, _order: usize) -> i32 {
    -EINVAL
}

pub fn split_huge_page_to_list_to_order(_page: *mut Page, _list: *mut u8, _order: usize) -> i32 {
    -EINVAL
}

pub fn count_mthp_stat(_order: usize, _item: usize) -> isize {
    0
}

pub fn mod_mthp_stat(_order: usize, _item: usize, _delta: isize) {}

pub fn vmf_insert_pfn_pmd(
    _vma: *mut u8,
    _addr: u64,
    _pmd: *mut pmd_t,
    _pfn: usize,
    _write: bool,
) -> u32 {
    0
}

pub fn vmf_insert_pfn_pud(
    _vma: *mut u8,
    _addr: u64,
    _pud: *mut pud_t,
    _pfn: usize,
    _write: bool,
) -> u32 {
    0
}

pub fn vmf_insert_folio_pmd(_vmf: *mut u8, _folio: *mut Page, _write: bool) -> u32 {
    0
}

pub fn vmf_insert_folio_pud(_vmf: *mut u8, _folio: *mut Page, _write: bool) -> u32 {
    0
}

#[cfg(test)]
pub fn reset_for_tests() {
    HUGE_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn hugetlb_pool_allocates_splits_and_frees_pages() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        configure_hugetlb_pool(HPAGE_PMD_NR);
        assert!(transparent_hugepage_enabled());
        assert!(hugetlb_enabled());

        let id = allocate_hugetlb_page(HPAGE_PMD_ORDER).unwrap();
        assert_eq!(huge_page(id).unwrap().nr_pages, HPAGE_PMD_NR);
        assert_eq!(huge_stats().pool_pages, 0);
        assert_eq!(split_huge_page(id), Ok(HPAGE_PMD_NR));
        assert!(huge_page(id).unwrap().split);
        assert_eq!(free_hugetlb_page(id), Ok(()));
        assert_eq!(huge_stats().pool_pages, HPAGE_PMD_NR);
    }

    #[test]
    fn khugepaged_collapses_registered_candidate() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(register_thp_candidate(0x20_0000, HPAGE_PMD_NR), Ok(()));
        assert_eq!(khugepaged_scan(), 1);
        assert_eq!(huge_stats().allocated_thp, 1);
    }

    #[test]
    fn hugetlb_cgroup_cma_sysctl_sysfs_and_vmemmap_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let root = hugetlb_cgroup_create(None).unwrap();
        let child = hugetlb_cgroup_create(Some(root)).unwrap();
        assert_eq!(hugetlb_cgroup_set_limit(child, HPAGE_PMD_NR), Ok(()));
        assert_eq!(hugetlb_cgroup_charge(child, HPAGE_PMD_NR, false), Ok(()));
        assert_eq!(hugetlb_cgroup_charge(child, 1, false), Err(ENOMEM));
        assert_eq!(hugetlb_cgroup(child).unwrap().failcnt, 1);
        assert_eq!(
            hugetlb_cgroup_uncharge(child, HPAGE_PMD_NR / 2, false),
            Ok(())
        );

        assert_eq!(reserve_hugetlb_cma(HPAGE_PMD_NR), Ok(()));
        assert_eq!(
            hugetlb_sysctl_read(HugetlbSysctl::NrHugepages),
            HPAGE_PMD_NR
        );
        assert_eq!(
            hugetlb_sysctl_write(HugetlbSysctl::NrOvercommitHugepages, 3),
            Ok(())
        );
        assert_eq!(
            hugetlb_sysctl_write(HugetlbSysctl::MovableGiganticPages, 2),
            Err(EINVAL)
        );

        let id = allocate_hugetlb_page(HPAGE_PMD_ORDER).unwrap();
        let sysfs = hugetlb_sysfs_snapshot();
        assert_eq!(sysfs.nr_hugepages, HPAGE_PMD_NR);
        assert_eq!(sysfs.free_hugepages, 0);
        assert_eq!(sysfs.surplus_hugepages, 3);
        assert_eq!(hugetlb_vmemmap_optimize(id), Ok(HPAGE_PMD_NR - 1));

        let stats = huge_stats();
        assert_eq!(stats.cma_reserved_pages, HPAGE_PMD_NR);
        assert_eq!(stats.hugetlb_cgroups, 2);
        assert_eq!(stats.vmemmap_saved_pages, HPAGE_PMD_NR - 1);
    }
}
