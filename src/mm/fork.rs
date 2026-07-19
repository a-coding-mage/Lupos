//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Memory management for fork — `dup_mm()` and `dup_mmap()`.
//
/// This module implements the fork half of the anonymous memory stack.
/// When a process forks, the kernel creates a new `mm_struct` with copies
/// of all the parent's VMAs, establishing COW chains via `anon_vma_fork()`
/// and `copy_page_range()`.
//
/// ## Functions
//
/// | Lupos function | Linux equivalent | Source |
/// |---|---|---|
/// | `dup_mm()` | `dup_mm()` | `kernel/fork.c:1515` |
/// | `dup_mmap()` | `dup_mmap()` | `mm/mmap.c:1732` |
//
/// ## References
///
/// - Linux `kernel/fork.c` — `dup_mm()`
/// - Linux `mm/mmap.c` — `dup_mmap()`
/// - Linux `include/linux/mm_types.h` — `struct mm_struct`
extern crate alloc;

use alloc::boxed::Box;
use core::sync::atomic::Ordering;

use crate::arch::x86::mm::paging::{
    PAGE_OFFSET, PAGE_SHIFT, PAGE_SIZE as X86_PAGE_SIZE, PMD_SIZE, PTE_PFN_MASK, PTRS_PER_PGD,
    PTRS_PER_PMD, PTRS_PER_PUD, PUD_SIZE, dump_pt_page_life_trace, pgd_none, pgd_t, pmd_huge,
    pmd_none, pmd_t, pte_present, pte_t, pud_huge, pud_none, pud_t, record_pt_page_free,
};
use crate::mm::buddy::{pfn_to_page, pfn_valid, with_global_buddy};
use crate::mm::fault::copy_page_range;
use crate::mm::mm_types::{MmStruct, VmAreaStruct, mmf_init_legacy_flags};
use crate::mm::mmap::do_munmap;
use crate::mm::page_flags::{GFP_KERNEL, PGTY_TABLE, decode_page_type};
use crate::mm::pagewalk::{MmWalk, MmWalkOps, walk_page_vma};
use crate::mm::rmap::anon_vma_fork;
use crate::mm::vm_flags::VM_DONTCOPY;
use crate::mm::vma::{insert_vma, vm_area_dup};

const PAGE_SIZE: usize = 4096;

// ---------------------------------------------------------------------------
// dup_mm — duplicate an mm_struct
// ---------------------------------------------------------------------------

/// Allocate and initialize a new `mm_struct` by copying an existing one.
///
/// The new mm has a fresh PGD (page-global-directory) and a duplicate set
/// of VMAs with COW chains set up.  Page tables are copied lazily via
/// `copy_page_range()` during the VMA iteration.
///
/// Returns a pointer to the new mm on success; returns `None` on allocation
/// failure.
///
/// # Safety
/// `old_mm` must be a valid `mm_struct` pointer.  The caller is responsible
/// for calling `mmdrop()` on the returned mm when done.
///
/// Ref: Linux `kernel/fork.c` — `dup_mm()` line 1515
pub unsafe fn dup_mm(old_mm: *mut MmStruct) -> Option<*mut MmStruct> {
    // Short-circuit when the buddy allocator hasn't been initialised yet
    // (e.g. host-side unit tests).  Returning None lets callers (copy_process)
    // report ENOMEM without panicking.
    if !crate::mm::buddy::is_buddy_ready() {
        return None;
    }

    unsafe {
        // Allocate a new mm_struct heap object.
        let new_mm_box = Box::new(MmStruct::new(0));
        let new_mm = Box::into_raw(new_mm_box);

        // Copy scalar fields from the parent.
        // Ref: vendor/linux/kernel/fork.c::dup_mm — all mm_struct scalar fields copied.
        (*new_mm).total_vm = (*old_mm).total_vm;
        (*new_mm).locked_vm = (*old_mm).locked_vm;
        (*new_mm).data_vm = (*old_mm).data_vm;
        (*new_mm).exec_vm = (*old_mm).exec_vm;
        (*new_mm).stack_vm = (*old_mm).stack_vm;
        (*new_mm).hiwater_vm = (*old_mm).hiwater_vm;
        (*new_mm).hiwater_rss = (*old_mm).hiwater_rss;
        (*new_mm).flags = mmf_init_legacy_flags((*old_mm).flags);
        (*new_mm).def_flags = (*old_mm).def_flags;
        // Address space layout fields — needed by /proc/<pid>/stat, cmdline, environ.
        // Without these, children report zeroed boundaries and cannot read their own env.
        (*new_mm).start_code = (*old_mm).start_code;
        (*new_mm).end_code = (*old_mm).end_code;
        (*new_mm).start_data = (*old_mm).start_data;
        (*new_mm).end_data = (*old_mm).end_data;
        (*new_mm).start_brk = (*old_mm).start_brk;
        (*new_mm).brk = (*old_mm).brk;
        (*new_mm).start_stack = (*old_mm).start_stack;
        (*new_mm).arg_start = (*old_mm).arg_start;
        (*new_mm).arg_end = (*old_mm).arg_end;
        (*new_mm).env_start = (*old_mm).env_start;
        (*new_mm).env_end = (*old_mm).env_end;
        if let Some(exe_file) = crate::mm::mm_public::get_mm_exe_file_ref(old_mm) {
            crate::mm::mm_public::set_mm_exe_file_ref(new_mm, exe_file);
        }

        // Allocate a fresh PGD (page table root) from the buddy allocator.
        // The PGD must be page-aligned and zeroed.
        let pgd_page = with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL))?;
        let pgd_pfn = crate::mm::buddy::page_to_pfn(pgd_page);
        let pgd_virt = crate::arch::x86::mm::paging::pfn_to_virt(pgd_pfn as usize);

        // Zero the entire PGD table.
        core::ptr::write_bytes(pgd_virt, 0u8, PAGE_SIZE);
        #[cfg(not(test))]
        {
            let init_pgd = crate::arch::x86::mm::paging::phys_to_virt(
                crate::arch::x86::mm::paging::init_pgd_phys(),
            ) as *const crate::arch::x86::mm::paging::pgd_t;
            if copy_kernel_pgd_entries(
                pgd_virt as *mut crate::arch::x86::mm::paging::pgd_t,
                init_pgd,
            )
            .is_err()
            {
                with_global_buddy(|b| b.free_pages(pgd_page, 0));
                let _ = Box::from_raw(new_mm);
                return None;
            }
        }

        (*new_mm).pgd = pgd_virt as usize;

        // Mm users and counts start at 1.
        (*new_mm).mm_users.store(1, Ordering::Release);
        (*new_mm).mm_count.store(1, Ordering::Release);

        // Duplicate all VMAs from the parent.
        // On error, dup_mmap will clean up and return Err.
        if dup_mmap(new_mm, old_mm).is_err() {
            exit_mmap(new_mm);
            with_global_buddy(|b| unsafe { b.free_pages(pgd_page, 0) });
            let _ = Box::from_raw(new_mm);
            return None;
        }

        // Update high-water marks.
        (*new_mm).hiwater_rss = get_mm_rss(new_mm);
        (*new_mm).hiwater_vm = (*new_mm).total_vm;

        Some(new_mm)
    }
}

#[cfg(not(test))]
unsafe fn copy_kernel_pgd_entries(
    dst: *mut crate::arch::x86::mm::paging::pgd_t,
    init_pgd: *const crate::arch::x86::mm::paging::pgd_t,
) -> Result<(), i32> {
    unsafe {
        core::ptr::copy_nonoverlapping(init_pgd, dst, 512);
        let mut idx = 1usize;
        while idx < 256 {
            *dst.add(idx) = crate::arch::x86::mm::paging::pgd_t(0);
            idx += 1;
        }
        crate::arch::x86::mm::paging::clone_low_identity_pgd_slot_for_user(dst, init_pgd)
            .ok_or(-12)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// dup_mmap — duplicate all VMAs from one mm_struct to another
// ---------------------------------------------------------------------------

/// Copy the VMA list from the source mm to the destination mm, setting up
/// COW chains and duplicating page tables as needed.
///
/// Iterates over all VMAs in `old_mm`, duplicating each (except those with
/// `VM_DONTCOPY`), and inserting it into `new_mm`.  For each VMA, calls
/// `anon_vma_fork()` and `copy_page_range()` to set up COW.
///
/// On error, partially-copied VMAs remain in `new_mm` — the caller should
/// call `exit_mmap()` to clean up.
///
/// # Safety
/// Both `new_mm` and `old_mm` must be valid `mm_struct` pointers.
///
/// Ref: Linux `mm/mmap.c` — `dup_mmap()` line 1732
pub unsafe fn dup_mmap(new_mm: *mut MmStruct, old_mm: *mut MmStruct) -> Result<(), i32> {
    unsafe {
        // Iterate over all VMAs in the source mm.
        // The Maple Tree is accessed directly here.
        let entries = (*old_mm).mm_mt.collect_entries();

        for (_, _, vma_ptr_val) in entries {
            let src_vma = vma_ptr_val as *mut VmAreaStruct;
            let src_flags = (*src_vma).vm_flags;

            // Skip VMAs marked as do-not-copy.
            if src_flags & VM_DONTCOPY != 0 {
                continue;
            }

            // Duplicate the VMA structure.
            let dst_vma = vm_area_dup(src_vma);

            // Update the mm_struct pointer to the new mm.
            (*dst_vma).vm_mm = new_mm;

            // Check for VM_WIPEONFORK — do not copy page tables to the child.
            const VM_WIPEONFORK: u64 = 1 << 18; // Linux value
            if src_flags & VM_WIPEONFORK != 0 {
                // Clear the anon_vma on the child VMA so new pages are faulted on demand.
                (*dst_vma).anon_vma = core::ptr::null_mut();
            } else {
                // Set up COW chains and copy page tables.
                anon_vma_fork(dst_vma, src_vma)?;
                copy_page_range(new_mm, old_mm, src_vma)?;
            }

            // Insert the copied VMA into the new mm.
            insert_vma(&mut *new_mm, dst_vma)?;
        }

        // High-water marks are set by the caller (dup_mm).
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Tear down all VMAs in an mm.
///
/// Ref: Linux `mm/mmap.c` — `exit_mmap()`
///
/// # Safety
/// `mm` must be exclusively owned by the caller and must not be active on a CPU.
pub unsafe fn exit_mmap(mm: *mut MmStruct) {
    if mm.is_null() {
        return;
    }

    let entries = unsafe { (*mm).mm_mt.collect_entries() };
    for (start, end_inclusive, _) in entries {
        let end = end_inclusive.saturating_add(1);
        if end > start {
            let _ = unsafe { do_munmap(&mut *mm, start, end - start) };
        }
    }
}

/// Drop a userspace mm reference and destroy the address space on the last user.
///
/// This is the Lupos equivalent of Linux `mmput()`: when `mm_users` reaches
/// zero it tears down VMAs, page tables, the PGD page, and finally the
/// `mm_struct` heap allocation.
///
/// # Safety
/// `mm` must be a valid `MmStruct` pointer owned by task mm lifetime rules.
pub unsafe fn mmput(mm: *mut MmStruct) {
    if mm.is_null() {
        return;
    }

    if unsafe { (*mm).mmput() } {
        unsafe { destroy_mm(mm) };
    }
}

/// Destroy an mm whose `mm_users` count has already reached zero.
///
/// # Safety
/// The caller must ensure no CPU is still executing with this mm loaded in
/// CR3. `exit_mm()` switches away before calling this for the current task;
/// exec teardown calls it only after loading the replacement mm.
pub unsafe fn destroy_mm(mm: *mut MmStruct) {
    if mm.is_null() {
        return;
    }

    unsafe {
        crate::kernel::futex::futex_private_hash_mm_destroy(mm as u64);
        crate::mm::mm_public::clear_mm_exe_file_ref(mm);
        exit_mmap(mm);
        free_user_page_tables(mm);
        free_mm_pgd(mm);
        let _ = Box::from_raw(mm);
    }
}

unsafe fn free_user_page_tables(mm: *mut MmStruct) {
    let pgd = unsafe { (*mm).pgd as *mut pgd_t };
    if pgd.is_null() {
        return;
    }

    unsafe {
        // Slots 0..255 are the user half for this transitional x86_64 layout.
        // Slot 0 also carries cloned low identity-map tables; the leaf mappings
        // are kernel mappings, but the cloned table pages themselves are owned
        // by this mm and must be released.
        let user_limit = PTRS_PER_PGD / 2;
        let mut pgd_idx = 0usize;
        while pgd_idx < user_limit {
            let pgdp = pgd.add(pgd_idx);
            let pgd_entry = *pgdp;
            if !pgd_none(pgd_entry) {
                if let Some(pud_ptr) =
                    owned_table_ptr_from_entry(pgd_entry.0, pgdp as usize, "pgd", pgd_idx, 0, 0)
                {
                    let pud_base = pud_ptr as *mut pud_t;
                    let mut pud_idx = 0usize;
                    while pud_idx < PTRS_PER_PUD {
                        let pudp = pud_base.add(pud_idx);
                        let pud_entry = *pudp;
                        if !pud_none(pud_entry) && !pud_huge(pud_entry) {
                            if let Some(pmd_ptr) = owned_table_ptr_from_entry(
                                pud_entry.0,
                                pudp as usize,
                                "pud",
                                pgd_idx,
                                pud_idx,
                                0,
                            ) {
                                let pmd_base = pmd_ptr as *mut pmd_t;
                                let mut pmd_idx = 0usize;
                                while pmd_idx < PTRS_PER_PMD {
                                    let pmdp = pmd_base.add(pmd_idx);
                                    let pmd_entry = *pmdp;
                                    if !pmd_none(pmd_entry) && !pmd_huge(pmd_entry) {
                                        if owned_table_ptr_from_entry(
                                            pmd_entry.0,
                                            pmdp as usize,
                                            "pmd",
                                            pgd_idx,
                                            pud_idx,
                                            pmd_idx,
                                        )
                                        .is_some()
                                        {
                                            free_table_page_from_entry(
                                                pmd_entry.0,
                                                "pte",
                                                pgd_idx,
                                                pud_idx,
                                                pmd_idx,
                                            );
                                        }
                                        *pmdp = pmd_t(0);
                                    }
                                    pmd_idx += 1;
                                }
                                free_table_page_from_entry(pud_entry.0, "pmd", pgd_idx, pud_idx, 0);
                            }
                            *pudp = pud_t(0);
                        }
                        pud_idx += 1;
                    }
                    free_table_page_from_entry(pgd_entry.0, "pud", pgd_idx, 0, 0);
                }
                *pgdp = pgd_t(0);
            }
            pgd_idx += 1;
        }
    }
}

unsafe fn free_mm_pgd(mm: *mut MmStruct) {
    let pgd = unsafe { (*mm).pgd };
    if pgd == 0 {
        return;
    }

    if let Some(phys) = direct_map_phys_from_virt(pgd) {
        unsafe { free_table_page_phys(phys, "pgd_root", 0, 0, 0) };
    }
    unsafe {
        (*mm).pgd = 0;
    }
}

fn direct_map_phys_from_virt(virt: usize) -> Option<u64> {
    let virt = virt as u64;
    if virt >= PAGE_OFFSET {
        Some(virt - PAGE_OFFSET)
    } else if virt & ((1u64 << PAGE_SHIFT) - 1) == 0 {
        Some(virt)
    } else {
        None
    }
}

fn direct_map_phys_from_entry_value(entry: u64) -> Option<u64> {
    let aligned = entry & !((1u64 << PAGE_SHIFT) - 1);
    direct_map_phys_from_virt(aligned as usize)
}

#[cfg(not(test))]
unsafe fn owned_table_ptr_from_entry(
    entry: u64,
    slot: usize,
    level: &str,
    pgd_idx: usize,
    pud_idx: usize,
    pmd_idx: usize,
) -> Option<*mut u8> {
    let phys = entry & PTE_PFN_MASK;
    if phys == 0 {
        return None;
    }
    let dm_phys = direct_map_phys_from_entry_value(entry).unwrap_or(u64::MAX);
    let pfn = (phys >> PAGE_SHIFT) as usize;
    if !pfn_valid(pfn) {
        let target_phys = if dm_phys != u64::MAX { dm_phys } else { phys };
        dump_pt_page_life_trace("invalid_table_entry", target_phys);
        // A non-present, non-zero entry that does not point at a tracked frame
        // is not a valid table pointer; skip it (the page, if any, is not ours
        // to free). Rate-limit the diagnostic: emitting one line per such entry
        // floods the serial console and starves the cooperative scheduler into
        // a soft lockup during process teardown. Mirrors Linux
        // `pr_err_ratelimited`.
        crate::log_ratelimited!(
            crate::kernel::printk::log::Level::Error,
            "mm",
            crate::kernel::time::jiffies::HZ,
            3,
            "free_user_page_tables: skip invalid table level={} pgd={} pud={} pmd={} slot={:#018x} entry={:#018x} phys={:#018x} dm_phys={:#018x}",
            level,
            pgd_idx,
            pud_idx,
            pmd_idx,
            slot,
            entry,
            phys,
            dm_phys
        );
        return None;
    }

    let page = pfn_to_page(pfn);
    if page.is_null() {
        return None;
    }
    let page_type = unsafe { (*page).page_type.load(Ordering::Relaxed) };
    if decode_page_type(page_type) != PGTY_TABLE {
        let target_phys = if dm_phys != u64::MAX { dm_phys } else { phys };
        dump_pt_page_life_trace("non_table_entry", target_phys);
        // Rate-limited for the same reason as the invalid-pfn case above.
        crate::log_ratelimited!(
            crate::kernel::printk::log::Level::Error,
            "mm",
            crate::kernel::time::jiffies::HZ,
            3,
            "free_user_page_tables: skip non-table level={} pgd={} pud={} pmd={} slot={:#018x} entry={:#018x} phys={:#018x} dm_phys={:#018x} page_type={:#x}",
            level,
            pgd_idx,
            pud_idx,
            pmd_idx,
            slot,
            entry,
            phys,
            dm_phys,
            page_type
        );
        return None;
    }

    Some(crate::arch::x86::mm::paging::phys_to_virt(phys))
}

#[cfg(test)]
unsafe fn owned_table_ptr_from_entry(
    entry: u64,
    _slot: usize,
    _level: &str,
    _pgd_idx: usize,
    _pud_idx: usize,
    _pmd_idx: usize,
) -> Option<*mut u8> {
    let phys = entry & PTE_PFN_MASK;
    (phys != 0).then_some(phys as *mut u8)
}

unsafe fn free_table_page_from_entry(
    entry: u64,
    level: &'static str,
    pgd_idx: usize,
    pud_idx: usize,
    pmd_idx: usize,
) {
    let phys = entry & PTE_PFN_MASK;
    unsafe { free_table_page_phys(phys, level, pgd_idx, pud_idx, pmd_idx) };
}

unsafe fn free_table_page_phys(
    phys: u64,
    level: &'static str,
    pgd_idx: usize,
    pud_idx: usize,
    pmd_idx: usize,
) {
    let pfn = (phys >> PAGE_SHIFT) as usize;
    if !pfn_valid(pfn) {
        return;
    }
    let page = pfn_to_page(pfn);
    if page.is_null() {
        return;
    }
    let page_type = unsafe { (*page).page_type.load(Ordering::Relaxed) };
    record_pt_page_free(phys, page, page_type, level, pgd_idx, pud_idx, pmd_idx);
    with_global_buddy(|b| unsafe { b.free_pages(page, 0) });
}

// Page-table walk state used by get_mm_rss().
///
// Lupos accounts mapped VMA pages here; per-type RSS buckets are maintained by
// the owning fault/reclaim paths as they grow in.
///
/// Ref: Linux `mm/mmap.c` — `get_mm_rss()`
struct RssWalk {
    pages: u64,
}

impl MmWalkOps for RssWalk {
    fn pte_entry(
        &mut self,
        ptep: *mut pte_t,
        _addr: u64,
        _next: u64,
        _walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        let pte = unsafe { *ptep };
        if pte_present(pte) {
            self.pages = self.pages.saturating_add(1);
        }
        Ok(())
    }

    fn pmd_entry(
        &mut self,
        pmdp: *mut pmd_t,
        _addr: u64,
        _next: u64,
        _walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        let pmd = unsafe { *pmdp };
        if pmd_huge(pmd) {
            self.pages = self.pages.saturating_add(PMD_SIZE / X86_PAGE_SIZE);
        }
        Ok(())
    }

    fn pud_entry(
        &mut self,
        pudp: *mut pud_t,
        _addr: u64,
        _next: u64,
        _walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        let pud = unsafe { *pudp };
        if pud_huge(pud) {
            self.pages = self.pages.saturating_add(PUD_SIZE / X86_PAGE_SIZE);
        }
        Ok(())
    }

    fn has_pte_entry(&self) -> bool {
        true
    }

    fn has_pmd_entry(&self) -> bool {
        true
    }

    fn has_pud_entry(&self) -> bool {
        true
    }
}

/// Compute the current resident set size (RSS) of an mm in pages.
///
/// Ref: Linux `include/linux/mm.h` - `get_mm_rss()`.
pub unsafe fn get_mm_rss(mm: *const MmStruct) -> u64 {
    if mm.is_null() {
        return 0;
    }

    let mut walk = RssWalk { pages: 0 };
    for (_, _, vma_ptr) in unsafe { (*mm).mm_mt.collect_entries() } {
        let vma = vma_ptr as *const VmAreaStruct;
        if vma.is_null() {
            continue;
        }
        let start = unsafe { (*vma).vm_start };
        let end = unsafe { (*vma).vm_end };
        if start >= end {
            continue;
        }
        let _ = unsafe { walk_page_vma(vma, start, end, &mut walk, core::ptr::null_mut()) };
    }
    walk.pages
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use crate::mm::vm_flags::{VM_READ, VM_WRITE, VmFlags};
    use crate::mm::vma::{insert_vma, vm_area_free};

    // ---------------------------------------------------------------------------
    // dup_mmap tests — these exercise VMA duplication without touching the buddy
    // allocator (no real pages are mapped, so copy_page_range is a no-op).
    // ---------------------------------------------------------------------------

    #[test]
    fn dup_mmap_empty_mm() {
        // dup_mmap on an empty mm does not need the buddy allocator.
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let old_mm = Box::into_raw(Box::new(MmStruct::new(0x1000)));
            let new_mm = Box::into_raw(Box::new(MmStruct::new(0x2000)));

            let result = dup_mmap(new_mm, old_mm);
            assert!(result.is_ok());
            assert_eq!((*new_mm).map_count, 0);

            // Clean up.
            let _ = Box::from_raw(old_mm);
            let _ = Box::from_raw(new_mm);
        }
    }

    /// Allocate a heap-resident VMA with an initialized `anon_vma_chain`.
    unsafe fn make_vma(start: u64, end: u64, flags: VmFlags) -> *mut VmAreaStruct {
        use crate::mm::list::ListHead;
        let vma = Box::new(VmAreaStruct::new(start, end, flags));
        let ptr = Box::into_raw(vma);
        ListHead::init(&mut (*ptr).anon_vma_chain);
        ptr
    }

    /// Free all VMAs stored in an mm's Maple Tree and then drop the mm itself.
    unsafe fn cleanup_mm(mm: *mut MmStruct) {
        for (_, _, ptr) in (*mm).mm_mt.collect_entries() {
            vm_area_free(ptr as *mut VmAreaStruct);
        }
        let _ = Box::from_raw(mm);
    }

    #[test]
    fn dup_mmap_creates_equal_vma_count() {
        // Populate the source mm with two VMAs, then dup_mmap and verify
        // that the destination mm ends up with the same count.
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let old_mm = Box::into_raw(Box::new(MmStruct::new(0x1000)));
            let new_mm = Box::into_raw(Box::new(MmStruct::new(0x2000)));

            // Insert two VMAs into old_mm.
            let vma1 = make_vma(0x1000, 0x2000, VM_READ);
            let vma2 = make_vma(0x3000, 0x5000, VM_READ | VM_WRITE);
            insert_vma(&mut *old_mm, vma1).expect("insert vma1");
            insert_vma(&mut *old_mm, vma2).expect("insert vma2");
            assert_eq!((*old_mm).map_count, 2);

            let result = dup_mmap(new_mm, old_mm);
            assert!(result.is_ok(), "dup_mmap failed");
            assert_eq!((*new_mm).map_count, 2, "new mm must have same VMA count");
            assert_eq!((*new_mm).total_vm, (*old_mm).total_vm);

            cleanup_mm(old_mm);
            cleanup_mm(new_mm);
        }
    }

    #[test]
    fn dup_mmap_vmas_are_independent_copies() {
        // After dup_mmap, child VMAs must have matching address ranges and
        // flags but be distinct heap allocations.
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let old_mm = Box::into_raw(Box::new(MmStruct::new(0x1000)));
            let new_mm = Box::into_raw(Box::new(MmStruct::new(0x2000)));

            let vma = make_vma(0x4000, 0x8000, VM_READ | VM_WRITE);
            insert_vma(&mut *old_mm, vma).expect("insert vma");

            dup_mmap(new_mm, old_mm).expect("dup_mmap");

            // Get the source and destination VMA pointers.
            let src_entries = (*old_mm).mm_mt.collect_entries();
            let dst_entries = (*new_mm).mm_mt.collect_entries();
            assert_eq!(dst_entries.len(), 1);

            let src_vma = src_entries[0].2 as *mut VmAreaStruct;
            let dst_vma = dst_entries[0].2 as *mut VmAreaStruct;

            // Different allocations.
            assert_ne!(src_vma, dst_vma, "parent and child VMAs must be distinct");
            // Same range and flags.
            assert_eq!((*dst_vma).vm_start, (*src_vma).vm_start);
            assert_eq!((*dst_vma).vm_end, (*src_vma).vm_end);
            assert_eq!((*dst_vma).vm_flags, (*src_vma).vm_flags);
            // Child's vm_mm points to new_mm.
            assert_eq!((*dst_vma).vm_mm, new_mm);

            cleanup_mm(old_mm);
            cleanup_mm(new_mm);
        }
    }

    #[test]
    fn dup_mmap_skips_vm_dontcopy() {
        // A VMA with VM_DONTCOPY must not appear in the child mm.
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            use crate::mm::vm_flags::VM_DONTCOPY;

            let old_mm = Box::into_raw(Box::new(MmStruct::new(0x1000)));
            let new_mm = Box::into_raw(Box::new(MmStruct::new(0x2000)));

            // One normal VMA and one VM_DONTCOPY VMA.
            let normal = make_vma(0x1000, 0x2000, VM_READ);
            let skip = make_vma(0x3000, 0x4000, VM_READ | VM_DONTCOPY);
            insert_vma(&mut *old_mm, normal).expect("insert normal");
            insert_vma(&mut *old_mm, skip).expect("insert skip");
            assert_eq!((*old_mm).map_count, 2);

            dup_mmap(new_mm, old_mm).expect("dup_mmap");

            // Only the normal VMA should be in the child.
            assert_eq!((*new_mm).map_count, 1);
            let dst = (*new_mm).mm_mt.collect_entries();
            let dst_vma = dst[0].2 as *mut VmAreaStruct;
            assert_eq!((*dst_vma).vm_start, 0x1000);

            cleanup_mm(old_mm);
            cleanup_mm(new_mm);
        }
    }

    #[test]
    fn get_mm_rss_counts_present_pages_not_virtual_size() {
        use crate::arch::x86::mm::paging;
        use crate::mm::buddy;
        use crate::mm::fault::{FAULT_FLAG_USER, FAULT_FLAG_WRITE, handle_mm_fault};
        use crate::mm::page::Page;

        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        const TEST_PAGES: usize = 256;
        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        unsafe {
            let mm = Box::into_raw(Box::new(
                MmStruct::new(paging::init_pgd_for_test() as usize),
            ));
            let start = 0x0040_0000;
            let vma = make_vma(start, start + 4 * PAGE_SIZE as u64, VM_READ | VM_WRITE);
            (*vma).vm_mm = mm;
            insert_vma(&mut *mm, vma).expect("insert rss vma");

            assert_eq!(get_mm_rss(mm), 0);
            assert_eq!(
                handle_mm_fault(vma, start, FAULT_FLAG_USER | FAULT_FLAG_WRITE),
                0
            );
            assert_eq!(
                handle_mm_fault(
                    vma,
                    start + 2 * PAGE_SIZE as u64,
                    FAULT_FLAG_USER | FAULT_FLAG_WRITE
                ),
                0
            );

            assert_eq!(get_mm_rss(mm), 2);
            cleanup_mm(mm);
        }
    }
}
