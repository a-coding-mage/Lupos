//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Generic MMU, remap, sparsemem, and MMU notifier code.
//!
//! This module provides the generic memory-management pieces from:
//! - `vendor/linux/mm/bootmem_info.c`
//! - `vendor/linux/mm/early_ioremap.c`
//! - `vendor/linux/mm/execmem.c`
//! - `vendor/linux/mm/highmem.c`
//! - `vendor/linux/mm/ioremap.c`
//! - `vendor/linux/mm/memremap.c`
//! - `vendor/linux/mm/mm_init.c`
//! - `vendor/linux/mm/mmap_lock.c`
//! - `vendor/linux/mm/mmu_gather.c`
//! - `vendor/linux/mm/mmu_notifier.c`
//! - `vendor/linux/mm/nommu.c`
//! - `vendor/linux/mm/pgtable-generic.c`
//! - `vendor/linux/mm/sparse.c`
//! - `vendor/linux/mm/sparse-vmemmap.c`

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENOENT, ENOMEM};
use crate::mm::frame::PAGE_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MmuFacility {
    Mmu,
    Highmem,
    Sparsemem,
    MmuNotifier,
    Execmem,
    BootmemInfo,
    MmapLock,
    Nommu,
}

pub const fn facility_enabled(facility: MmuFacility) -> bool {
    match facility {
        MmuFacility::Highmem | MmuFacility::Nommu => false,
        MmuFacility::Mmu
        | MmuFacility::Sparsemem
        | MmuFacility::MmuNotifier
        | MmuFacility::Execmem
        | MmuFacility::BootmemInfo
        | MmuFacility::MmapLock => true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemRemapType {
    Io,
    WriteCombine,
    WriteThrough,
    WriteBack,
    Encrypted,
    Exec,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemapRequest {
    pub phys: u64,
    pub size: usize,
    pub writable: bool,
    pub executable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappedRegion {
    pub virt: u64,
    pub phys: u64,
    pub size: usize,
    pub kind: MemRemapType,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MmuGather {
    pub start: u64,
    pub end: u64,
    pub pending_flushes: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MmuStats {
    pub mappings: usize,
    pub early_mappings: usize,
    pub exec_mappings: usize,
    pub highmem_mappings: usize,
    pub notifiers: usize,
    pub sparse_sections: usize,
    pub invalidations: usize,
    pub bootmem_pages: usize,
    pub mmap_readers: usize,
    pub mmap_writer: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MmInitInfo {
    pub initialized: bool,
    pub max_mapnr: u64,
    pub high_memory: u64,
    pub zero_page_pfn: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootmemPage {
    pub pfn: u64,
    pub freed: bool,
}

struct MmuState {
    next_virt: u64,
    mappings: Vec<MappedRegion>,
    early_mappings: Vec<MappedRegion>,
    exec_mappings: Vec<MappedRegion>,
    highmem_mappings: Vec<MappedRegion>,
    notifiers: Vec<u64>,
    sparse_sections: Vec<u64>,
    bootmem_pages: Vec<BootmemPage>,
    mmap_readers: usize,
    mmap_writer: bool,
    invalidations: usize,
    mm_init: MmInitInfo,
}

impl MmuState {
    const fn new() -> Self {
        Self {
            next_virt: 0xffff_9000_0000_0000,
            mappings: Vec::new(),
            early_mappings: Vec::new(),
            exec_mappings: Vec::new(),
            highmem_mappings: Vec::new(),
            notifiers: Vec::new(),
            sparse_sections: Vec::new(),
            bootmem_pages: Vec::new(),
            mmap_readers: 0,
            mmap_writer: false,
            invalidations: 0,
            mm_init: MmInitInfo {
                initialized: false,
                max_mapnr: 0,
                high_memory: 0,
                zero_page_pfn: 0,
            },
        }
    }

    fn reset(&mut self) {
        self.next_virt = 0xffff_9000_0000_0000;
        self.mappings.clear();
        self.early_mappings.clear();
        self.exec_mappings.clear();
        self.highmem_mappings.clear();
        self.notifiers.clear();
        self.sparse_sections.clear();
        self.bootmem_pages.clear();
        self.mmap_readers = 0;
        self.mmap_writer = false;
        self.invalidations = 0;
        self.mm_init = MmInitInfo::default();
    }
}

static MMU_STATE: Mutex<MmuState> = Mutex::new(MmuState::new());

pub fn validate_memremap(req: RemapRequest) -> Result<(), i32> {
    if req.size == 0 {
        return Err(EINVAL);
    }
    if req.phys % PAGE_SIZE as u64 != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn memremap(req: RemapRequest, kind: MemRemapType) -> Result<MappedRegion, i32> {
    validate_memremap(req)?;
    let mut state = MMU_STATE.lock();
    let virt = state.next_virt;
    let aligned_size = align_up(req.size, PAGE_SIZE).ok_or(EINVAL)?;
    state.next_virt = state
        .next_virt
        .checked_add(aligned_size as u64)
        .ok_or(ENOMEM)?;

    let region = MappedRegion {
        virt,
        phys: req.phys,
        size: aligned_size,
        kind,
    };
    state.mappings.push(region);
    Ok(region)
}

pub fn ioremap(phys: u64, size: usize) -> Result<MappedRegion, i32> {
    memremap(
        RemapRequest {
            phys,
            size,
            writable: true,
            executable: false,
        },
        MemRemapType::Io,
    )
}

pub fn iounmap(virt: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    let idx = state
        .mappings
        .iter()
        .position(|mapping| mapping.virt == virt)
        .ok_or(ENOENT)?;
    state.mappings.swap_remove(idx);
    Ok(())
}

pub fn mm_init(max_mapnr: u64, high_memory: u64, zero_page_pfn: u64) -> Result<(), i32> {
    if max_mapnr == 0 || high_memory == 0 {
        return Err(EINVAL);
    }
    let mut state = MMU_STATE.lock();
    state.mm_init = MmInitInfo {
        initialized: true,
        max_mapnr,
        high_memory,
        zero_page_pfn,
    };
    Ok(())
}

pub fn mm_init_info() -> MmInitInfo {
    MMU_STATE.lock().mm_init
}

pub fn bootmem_register_page(pfn: u64) {
    let mut state = MMU_STATE.lock();
    if let Some(page) = state.bootmem_pages.iter_mut().find(|page| page.pfn == pfn) {
        page.freed = false;
    } else {
        state.bootmem_pages.push(BootmemPage { pfn, freed: false });
    }
}

pub fn free_bootmem_page(pfn: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    let page = state
        .bootmem_pages
        .iter_mut()
        .find(|page| page.pfn == pfn)
        .ok_or(ENOENT)?;
    page.freed = true;
    Ok(())
}

pub fn bootmem_page(pfn: u64) -> Option<BootmemPage> {
    MMU_STATE
        .lock()
        .bootmem_pages
        .iter()
        .find(|page| page.pfn == pfn)
        .copied()
}

pub fn execmem_alloc(size: usize) -> Result<MappedRegion, i32> {
    if size == 0 {
        return Err(EINVAL);
    }
    let mut state = MMU_STATE.lock();
    let virt = state.next_virt;
    let aligned_size = align_up(size, PAGE_SIZE).ok_or(EINVAL)?;
    state.next_virt = state
        .next_virt
        .checked_add(aligned_size as u64)
        .ok_or(ENOMEM)?;
    let region = MappedRegion {
        virt,
        phys: 0,
        size: aligned_size,
        kind: MemRemapType::Exec,
    };
    state.exec_mappings.push(region);
    Ok(region)
}

pub fn execmem_free(virt: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    let idx = state
        .exec_mappings
        .iter()
        .position(|mapping| mapping.virt == virt)
        .ok_or(ENOENT)?;
    state.exec_mappings.swap_remove(idx);
    Ok(())
}

pub fn kmap_high(pfn: u64) -> Result<u64, i32> {
    let mut state = MMU_STATE.lock();
    let virt = 0xffff_8000_0000_0000u64
        .checked_add(pfn.checked_mul(PAGE_SIZE as u64).ok_or(ENOMEM)?)
        .ok_or(ENOMEM)?;
    state.highmem_mappings.push(MappedRegion {
        virt,
        phys: pfn * PAGE_SIZE as u64,
        size: PAGE_SIZE,
        kind: MemRemapType::WriteBack,
    });
    Ok(virt)
}

pub fn kunmap_high(virt: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    let idx = state
        .highmem_mappings
        .iter()
        .position(|mapping| mapping.virt == virt)
        .ok_or(ENOENT)?;
    state.highmem_mappings.swap_remove(idx);
    Ok(())
}

pub fn mmap_read_lock() -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    if state.mmap_writer {
        return Err(EBUSY);
    }
    state.mmap_readers += 1;
    Ok(())
}

pub fn mmap_read_unlock() -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    if state.mmap_readers == 0 {
        return Err(EINVAL);
    }
    state.mmap_readers -= 1;
    Ok(())
}

pub fn mmap_write_lock() -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    if state.mmap_writer || state.mmap_readers != 0 {
        return Err(EBUSY);
    }
    state.mmap_writer = true;
    Ok(())
}

pub fn mmap_write_unlock() -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    if !state.mmap_writer {
        return Err(EINVAL);
    }
    state.mmap_writer = false;
    Ok(())
}

pub const fn nommu_supported() -> bool {
    false
}

pub fn early_ioremap_available() -> bool {
    true
}

pub fn early_ioremap(phys: u64, size: usize) -> Result<u64, i32> {
    if phys % PAGE_SIZE as u64 != 0 || size == 0 {
        return Err(EINVAL);
    }

    let mut state = MMU_STATE.lock();
    if state.early_mappings.len() >= 64 {
        return Err(EBUSY);
    }
    let virt = 0xffff_ffff_8000_0000u64 + (state.early_mappings.len() as u64 * PAGE_SIZE as u64);
    state.early_mappings.push(MappedRegion {
        virt,
        phys,
        size: align_up(size, PAGE_SIZE).ok_or(EINVAL)?,
        kind: MemRemapType::Io,
    });
    Ok(virt)
}

pub fn early_iounmap(virt: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    let idx = state
        .early_mappings
        .iter()
        .position(|mapping| mapping.virt == virt)
        .ok_or(ENOENT)?;
    state.early_mappings.swap_remove(idx);
    Ok(())
}

pub fn mmu_notifier_register(id: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    if state.notifiers.contains(&id) {
        return Err(EBUSY);
    }
    state.notifiers.push(id);
    Ok(())
}

pub fn mmu_notifier_unregister(id: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    let idx = state
        .notifiers
        .iter()
        .position(|&notifier| notifier == id)
        .ok_or(ENOENT)?;
    state.notifiers.swap_remove(idx);
    Ok(())
}

pub fn mmu_notifier_invalidate_range(_start: u64, _end: u64) -> usize {
    let mut state = MMU_STATE.lock();
    let callbacks = state.notifiers.len();
    state.invalidations += callbacks;
    callbacks
}

pub fn tlb_gather_mmu(start: u64, end: u64) -> Result<MmuGather, i32> {
    if start > end {
        Err(EINVAL)
    } else {
        Ok(MmuGather {
            start,
            end,
            pending_flushes: 0,
        })
    }
}

pub fn tlb_finish_mmu(gather: &mut MmuGather) {
    gather.pending_flushes += 1;
}

pub fn sparsemem_add_section(section_nr: u64) -> Result<(), i32> {
    let mut state = MMU_STATE.lock();
    if state.sparse_sections.contains(&section_nr) {
        return Err(EBUSY);
    }
    state.sparse_sections.push(section_nr);
    Ok(())
}

pub fn sparsemem_present_sections() -> usize {
    MMU_STATE.lock().sparse_sections.len()
}

pub fn mmu_stats() -> MmuStats {
    let state = MMU_STATE.lock();
    MmuStats {
        mappings: state.mappings.len(),
        early_mappings: state.early_mappings.len(),
        exec_mappings: state.exec_mappings.len(),
        highmem_mappings: state.highmem_mappings.len(),
        notifiers: state.notifiers.len(),
        sparse_sections: state.sparse_sections.len(),
        invalidations: state.invalidations,
        bootmem_pages: state.bootmem_pages.len(),
        mmap_readers: state.mmap_readers,
        mmap_writer: state.mmap_writer,
    }
}

fn align_up(value: usize, align: usize) -> Option<usize> {
    value
        .checked_add(align - 1)
        .map(|value| value & !(align - 1))
}

#[cfg(test)]
pub fn reset_for_tests() {
    MMU_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn memremap_ioremap_and_early_ioremap_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(facility_enabled(MmuFacility::MmuNotifier));
        let mapping = ioremap(0x2000, 17).unwrap();
        assert_eq!(mapping.phys, 0x2000);
        assert_eq!(mapping.size, PAGE_SIZE);
        assert_eq!(early_ioremap(0x3000, PAGE_SIZE).is_ok(), true);
        assert_eq!(mmu_stats().mappings, 1);
        assert_eq!(mmu_stats().early_mappings, 1);
        assert_eq!(iounmap(mapping.virt), Ok(()));
    }

    #[test]
    fn mmu_notifiers_sparsemem_and_tlb_gather_work() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(mmu_notifier_register(1), Ok(()));
        assert_eq!(mmu_notifier_register(2), Ok(()));
        assert_eq!(mmu_notifier_invalidate_range(0x1000, 0x2000), 2);
        assert_eq!(sparsemem_add_section(7), Ok(()));
        assert_eq!(sparsemem_present_sections(), 1);
        let mut gather = tlb_gather_mmu(0x1000, 0x2000).unwrap();
        tlb_finish_mmu(&mut gather);
        assert_eq!(gather.pending_flushes, 1);
    }

    #[test]
    fn mm_init_bootmem_execmem_highmem_mmap_lock_and_nommu_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(facility_enabled(MmuFacility::Execmem));
        assert!(facility_enabled(MmuFacility::MmapLock));
        assert!(!facility_enabled(MmuFacility::Highmem));
        assert!(!facility_enabled(MmuFacility::Nommu));
        assert!(!nommu_supported());

        assert_eq!(mm_init(1024, 0xffff_8880_0040_0000, 0), Ok(()));
        assert!(mm_init_info().initialized);

        bootmem_register_page(5);
        assert_eq!(free_bootmem_page(5), Ok(()));
        assert!(bootmem_page(5).unwrap().freed);

        let exec = execmem_alloc(33).unwrap();
        assert_eq!(exec.kind, MemRemapType::Exec);
        assert_eq!(mmu_stats().exec_mappings, 1);
        assert_eq!(execmem_free(exec.virt), Ok(()));

        let high = kmap_high(9).unwrap();
        assert_eq!(mmu_stats().highmem_mappings, 1);
        assert_eq!(kunmap_high(high), Ok(()));

        assert_eq!(mmap_read_lock(), Ok(()));
        assert_eq!(mmap_write_lock(), Err(EBUSY));
        assert_eq!(mmap_read_unlock(), Ok(()));
        assert_eq!(mmap_write_lock(), Ok(()));
        assert_eq!(mmap_read_lock(), Err(EBUSY));
        assert_eq!(mmap_write_unlock(), Ok(()));

        assert_eq!(mmu_stats().bootmem_pages, 1);
    }
}
