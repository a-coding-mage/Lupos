//! linux-parity: partial
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
    PTRS_PER_PMD, PTRS_PER_PUD, PUD_SIZE, dump_pt_page_life_trace, pgd_none_or_clear_bad, pgd_t,
    pmd_huge, pmd_none_or_clear_bad, pmd_t, pte_present, pte_t, pud_huge, pud_none_or_clear_bad,
    pud_t, record_pt_page_free,
};
use crate::arch::x86::mm::tlb::flush_tlb_mm_range;
use crate::mm::buddy::{pfn_to_page, pfn_valid, with_global_buddy};
use crate::mm::fault::copy_page_range;
use crate::mm::mm_types::{MmStruct, VmAreaStruct, mmf_init_legacy_flags};
use crate::mm::mmap::{TASK_SIZE, do_munmap};
use crate::mm::page_flags::{GFP_KERNEL, PGTY_TABLE, decode_page_type};
use crate::mm::pagewalk::{MmWalk, MmWalkOps, walk_page_vma};
use crate::mm::rmap::anon_vma_fork;
use crate::mm::vm_flags::{VM_DONTCOPY, VM_WIPEONFORK};
use crate::mm::vma::{insert_vma, vm_area_free, vm_area_try_dup, vma_open};

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
/// `old_mm` must be a valid `mm_struct` pointer. The returned mm owns one
/// `mm_users` reference; the caller must release it with [`mmput`].
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
    // Linux takes both mmap locks for the complete VMA/PTE duplication.  The
    // parent lock is write-held because COW setup downgrades source PTEs; the
    // child is not yet published, so nesting cannot deadlock.
    let old_guard = unsafe { crate::mm::mmap_lock::MmapWriteGuard::lock(old_mm) };
    let new_guard = unsafe { crate::mm::mmap_lock::MmapWriteGuard::lock(new_mm) };
    let result = unsafe { dup_mmap_inner(new_mm, old_mm) };

    // Linux converges successful duplication and every failure after taking
    // oldmm's mmap lock on `out:`, where it invalidates all parent translations
    // exactly once. `copy_page_range()` may have write-protected an arbitrary
    // prefix before returning an error, so the flush cannot be success-only.
    drop(new_guard);
    let result = finish_dup_mmap(old_mm, result, |mm, start, end| {
        let _ = unsafe { flush_tlb_mm_range(mm, start, end) };
    });
    drop(old_guard);
    result
}

/// Duplicate the VMA/page-table state without publishing the result or
/// invalidating parent translations.
///
/// All early errors stay inside this helper so [`dup_mmap`] has one common
/// flush-and-return tail, matching Linux's `loop_out -> out` control flow.
unsafe fn dup_mmap_inner(new_mm: *mut MmStruct, old_mm: *mut MmStruct) -> Result<(), i32> {
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
            let dst_vma = vm_area_try_dup(src_vma)?;

            // Update the mm_struct pointer to the new mm.
            (*dst_vma).vm_mm = new_mm;

            // Check for VM_WIPEONFORK — do not copy page tables to the child.
            if src_flags & VM_WIPEONFORK != 0 {
                // Clear the anon_vma on the child VMA so new pages are faulted on demand.
                (*dst_vma).anon_vma = core::ptr::null_mut();
            } else if let Err(err) = anon_vma_fork(dst_vma, src_vma) {
                vm_area_free(dst_vma);
                return Err(err);
            }

            // Insert and open the copied VMA before copying page tables, matching
            // Linux dup_mmap(): vma_iter_bulk_store(), map_count++, ->open(),
            // then copy_page_range().
            if let Err(err) = insert_vma(&mut *new_mm, dst_vma) {
                vm_area_free(dst_vma);
                return Err(err);
            }
            vma_open(dst_vma);

            if src_flags & VM_WIPEONFORK == 0 {
                copy_page_range(dst_vma, src_vma)?;
            }
        }

        // High-water marks are set by the caller (dup_mm).
        Ok(())
    }
}

/// Run the one full parent-mm TLB flush and then propagate the duplication
/// result unchanged.
///
/// `flush_tlb_mm_range()` represents a full invalidation with `end <= start`;
/// zero/zero makes that contract explicit and lets host tests verify the
/// common success/error tail without executing privileged invalidation.
#[inline]
fn finish_dup_mmap<T>(
    old_mm: *mut MmStruct,
    result: Result<T, i32>,
    flush_old_mm: impl FnOnce(*mut MmStruct, u64, u64),
) -> Result<T, i32> {
    flush_old_mm(old_mm, 0, 0);
    result
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Tear down all VMAs in an mm.
///
/// Ref: Linux `mm/mmap.c` — `exit_mmap()`
///
/// # Safety
/// The caller must exclusively own teardown of this address space and no task
/// may access its userspace mappings. CPUs may retain it as a lazy active_mm.
pub unsafe fn exit_mmap(mm: *mut MmStruct) {
    if mm.is_null() {
        return;
    }

    // Linux's exit_mmap() walks and detaches the whole VMA tree in one pass.
    // Use the full userspace range here so do_munmap() snapshots the tree once.
    // Calling do_munmap() separately for every entry repeatedly rebuilt that
    // snapshot, making teardown O(VMA²); a Firefox process with hundreds of
    // mappings could therefore keep its final thread alive long after a
    // session shutdown request.
    let _ = unsafe { do_munmap(&mut *mm, 0, TASK_SIZE) };
}

/// Drop a userspace mm reference and tear down the address space on the last
/// real user.
///
/// This is the Lupos equivalent of Linux `mmput()`/`__mmput()`: when
/// `mm_users` reaches zero it tears down user VMAs and clears their leaf
/// mappings, then drops the owning `mm_count` reference. Lazy-TLB users keep
/// the page-table hierarchy, PGD, and `mm_struct` alive until their matching
/// [`mmdrop`] after switching CR3.
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

/// Tear down an mm whose `mm_users` count has already reached zero.
///
/// # Safety
/// `mm_users` must be zero. CPUs may still carry the mm as a lazy active_mm,
/// but none may access its userspace mappings.
pub unsafe fn destroy_mm(mm: *mut MmStruct) {
    if mm.is_null() {
        return;
    }

    unsafe {
        exit_mmap(mm);
        // Linux's mmu_gather marks `freed_tables` and sends the TLB flush IPI
        // even to CPUs in lazy-TLB mode before reusing hierarchy pages
        // (`arch/x86/mm/tlb.c::native_flush_tlb_multi`). Lupos does not yet
        // have that freed_tables/lazy-CPU protocol. exit_mmap() has cleared the
        // leaf mappings and freed their backing pages; retain every user-half
        // hierarchy page until final mmdrop(), when no CPU can retain this CR3.
        crate::mm::mm_public::clear_mm_exe_file_ref(mm);
        crate::kernel::futex::futex_private_hash_mm_destroy(mm as u64);
        mmdrop(mm);
    }
}

/// Drop a structural/lazy-TLB reference and free the PGD plus `mm_struct` on
/// the last reference.
///
/// Ref: Linux `include/linux/sched/mm.h::mmdrop()` and
/// `kernel/fork.c::__mmdrop()`.
///
/// # Safety
/// `mm` must point to a live `MmStruct` carrying one reference owned by the
/// caller. The CPU performing the final drop must no longer have this PGD in
/// CR3.
pub unsafe fn mmdrop(mm: *mut MmStruct) {
    if mm.is_null() {
        return;
    }

    if unsafe { (*mm).mmdrop() } {
        unsafe {
            free_mm_pgd(mm);
            let _ = Box::from_raw(mm);
        }
    }
}

/// Free page-table hierarchy pages owned by `pgd[first..end]`.
///
/// Leaf mappings have already been unmapped by [`exit_mmap`]. Huge identity
/// mappings and any remaining leaf entries describe backing pages that this
/// helper does not own; only the page-table hierarchy itself is released.
///
/// # Safety
/// Every non-leaf table reachable from the selected slots must either be
/// exclusively owned by this mm or rejected by [`owned_table_ptr_from_entry`].
/// None of the selected slots may be required by a CPU still using this PGD.
unsafe fn free_owned_pgd_slots(pgd: *mut pgd_t, first: usize, end: usize) {
    debug_assert!(!pgd.is_null());
    debug_assert!(first <= end);
    debug_assert!(end <= PTRS_PER_PGD);

    unsafe {
        let mut pgd_idx = first;
        while pgd_idx < end {
            let pgdp = pgd.add(pgd_idx);
            if !pgd_none_or_clear_bad(&mut *pgdp) {
                let pgd_entry = *pgdp;
                if let Some(pud_ptr) =
                    owned_table_ptr_from_entry(pgd_entry.0, pgdp as usize, "pgd", pgd_idx, 0, 0)
                {
                    let pud_base = pud_ptr as *mut pud_t;
                    let mut pud_idx = 0usize;
                    while pud_idx < PTRS_PER_PUD {
                        let pudp = pud_base.add(pud_idx);
                        if !pud_huge(*pudp) && !pud_none_or_clear_bad(&mut *pudp) {
                            let pud_entry = *pudp;
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
                                    if !pmd_huge(*pmdp) && !pmd_none_or_clear_bad(&mut *pmdp) {
                                        let pmd_entry = *pmdp;
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
                                            *pmdp = pmd_t(0);
                                            free_table_page_from_entry(
                                                pmd_entry.0,
                                                "pte",
                                                pgd_idx,
                                                pud_idx,
                                                pmd_idx,
                                            );
                                        } else {
                                            *pmdp = pmd_t(0);
                                        }
                                    }
                                    pmd_idx += 1;
                                }
                                *pudp = pud_t(0);
                                free_table_page_from_entry(pud_entry.0, "pmd", pgd_idx, pud_idx, 0);
                            } else {
                                *pudp = pud_t(0);
                            }
                        }
                        pud_idx += 1;
                    }
                    *pgdp = pgd_t(0);
                    free_table_page_from_entry(pgd_entry.0, "pud", pgd_idx, 0, 0);
                } else {
                    *pgdp = pgd_t(0);
                }
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

    // All user-half hierarchy pages are deliberately retained while any CPU
    // may hold this mm as active_mm. This includes slot 0's low kernel identity
    // mappings and slots 1..255: without Linux's `freed_tables` shootdown, a
    // lazy CPU could speculatively walk a freed and reused hierarchy page.
    // Final mmdrop means every lazy structural reference is gone and the caller
    // has switched CR3 away, so the hierarchy and root can now be released.
    unsafe {
        free_owned_pgd_slots(pgd as *mut pgd_t, 0, PTRS_PER_PGD / 2);
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
            "free_owned_pgd_slots: skip invalid table level={} pgd={} pud={} pmd={} slot={:#018x} entry={:#018x} phys={:#018x} dm_phys={:#018x}",
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
            "free_owned_pgd_slots: skip non-table level={} pgd={} pud={} pmd={} slot={:#018x} entry={:#018x} phys={:#018x} dm_phys={:#018x} page_type={:#x}",
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
    use crate::mm::vm_flags::{VM_MAYWRITE, VM_READ, VM_WRITE, VmFlags};
    use crate::mm::vma::{insert_vma, vm_area_free, vma_split};

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

    #[test]
    fn dup_mmap_common_tail_flushes_full_old_mm_once_before_success_return() {
        // Linux mm/mmap.c::dup_mmap reaches `out:` before returning success,
        // and `flush_tlb_mm(oldmm)` is the only parent invalidation there.
        let old_mm = 0x1234usize as *mut MmStruct;
        let mut flushes = 0usize;

        let result = finish_dup_mmap(old_mm, Ok(7usize), |mm, start, end| {
            assert_eq!(mm, old_mm);
            assert_eq!((start, end), (0, 0), "zero/zero denotes a full flush");
            flushes += 1;
        });

        assert_eq!(flushes, 1);
        assert_eq!(result, Ok(7));
    }

    #[test]
    fn dup_mmap_common_tail_flushes_full_old_mm_once_before_error_return() {
        // Linux takes the same `loop_out -> out` tail when copy_page_range()
        // fails after write-protecting a prefix of the parent page tables.
        let old_mm = 0x5678usize as *mut MmStruct;
        let mut flushes = 0usize;

        let result: Result<(), i32> = finish_dup_mmap(old_mm, Err(-12), |mm, start, end| {
            assert_eq!(mm, old_mm);
            assert_eq!((start, end), (0, 0), "zero/zero denotes a full flush");
            flushes += 1;
        });

        assert_eq!(flushes, 1);
        assert_eq!(result, Err(-12));
    }

    #[test]
    fn exit_mmap_detaches_the_complete_vma_tree() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let mm = Box::into_raw(Box::new(MmStruct::new(0)));
            for (start, end) in [
                (0x1000, 0x3000),
                (0x10_0000, 0x14_0000),
                (TASK_SIZE - 0x4000, TASK_SIZE),
            ] {
                let vma = make_vma(start, end, VM_READ | VM_WRITE);
                insert_vma(&mut *mm, vma).expect("insert exit VMA");
            }
            assert_eq!((*mm).map_count, 3);

            exit_mmap(mm);

            assert_eq!((*mm).map_count, 0);
            assert!((*mm).mm_mt.collect_entries().is_empty());
            let _ = Box::from_raw(mm);
        }
    }

    #[test]
    fn lazy_tlb_mm_count_defers_structural_free_after_last_mm_user() {
        // Linux __mmput() releases userspace resources at mm_users == 0 but
        // leaves the PGD/mm allocation alive for a lazy active_mm reference.
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let mm = Box::into_raw(Box::new(MmStruct::new(0)));
            (*mm).mmdrop_get(); // lazy active_mm reference: mm_count 1 -> 2

            mmput(mm);

            assert_eq!((*mm).mm_users.load(Ordering::Acquire), 0);
            assert_eq!((*mm).mm_count.load(Ordering::Acquire), 1);
            assert_eq!((*mm).map_count, 0);

            // Final lazy drop frees `mm`; do not dereference it afterwards.
            mmdrop(mm);
        }
    }

    #[test]
    fn user_pgd_hierarchies_survive_mmput_until_final_mmdrop() {
        // Linux `exit_mmap()` may run while the mm remains active, whereas
        // its `freed_tables` TLB flush protects hierarchy reuse from lazy CPUs.
        // Lupos-specific adaptation: without that lazy-CPU protocol, every
        // user-half hierarchy follows PGD lifetime and is freed by __mmdrop().
        use crate::arch::x86::mm::paging::{
            _PAGE_TABLE, init_pgd_for_test, reset_test_pool, test_pool,
        };

        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            reset_test_pool();
            let pgd = init_pgd_for_test();
            let low_identity_pud = test_pool::alloc().expect("low identity PUD");
            let ordinary_user_pud = test_pool::alloc().expect("ordinary user PUD");
            let low_entry = low_identity_pud | _PAGE_TABLE;
            *pgd.add(0) = pgd_t(low_entry);
            *pgd.add(1) = pgd_t(ordinary_user_pud | _PAGE_TABLE);

            let mm = Box::into_raw(Box::new(MmStruct::new(pgd as usize)));
            (*mm).mmdrop_get(); // current/lazy active_mm reference
            mmput(mm);

            assert_eq!(
                (*pgd.add(0)).0,
                low_entry,
                "mm_users teardown must preserve the active low identity map"
            );
            assert_eq!(
                (*pgd.add(1)).0,
                ordinary_user_pud | _PAGE_TABLE,
                "mmput must preserve ordinary hierarchy pages from lazy CPUs"
            );
            assert_eq!((*mm).mm_count.load(Ordering::Acquire), 1);

            // Drops the final active_mm reference and frees the mm. The host
            // page-table pool is static, so its cleared entry remains
            // observable after the MmStruct itself is gone.
            mmdrop(mm);

            assert_eq!(
                (*pgd.add(0)).0,
                0,
                "final mmdrop should release the deferred slot-0 hierarchy"
            );
            assert_eq!(
                (*pgd.add(1)).0,
                0,
                "final mmdrop should release ordinary user hierarchy pages"
            );
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

    /// Linux `__split_vma()` preserves anon-vma metadata, so a present
    /// anonymous PTE in the split half remains visible to `dup_mmap()` and is
    /// copied into the child as a COW mapping.
    ///
    /// test-origin: linux:vendor/linux/mm/vma.c:__split_vma
    #[test]
    fn dup_mmap_copies_present_pte_after_faulted_anon_vma_split() {
        use crate::arch::x86::mm::paging;
        use crate::mm::buddy;
        use crate::mm::fault::{FAULT_FLAG_USER, FAULT_FLAG_WRITE, handle_mm_fault};
        use crate::mm::page::Page;

        unsafe fn pte_for_addr(mm: *const MmStruct, addr: u64) -> paging::pte_t {
            let pgd = (*mm).pgd as *mut paging::pgd_t;
            if pgd.is_null() {
                return paging::__pte(0);
            }
            let pgdp = paging::pgd_offset_pgd(pgd, addr);
            if paging::pgd_none(*pgdp) {
                return paging::__pte(0);
            }
            let p4dp = paging::p4d_offset(pgdp, addr);
            let pudp = paging::pud_offset(p4dp, addr);
            if paging::pud_none(*pudp) || paging::pud_huge(*pudp) {
                return paging::__pte(0);
            }
            let pmdp = paging::pmd_offset(pudp, addr);
            if paging::pmd_none(*pmdp) || paging::pmd_huge(*pmdp) {
                return paging::__pte(0);
            }
            let ptep = paging::pte_offset_kernel(pmdp, addr);
            paging::ptep_get(ptep)
        }

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
            let old_mm = Box::into_raw(Box::new(MmStruct::new(
                paging::init_pgd_for_test() as usize
            )));
            let new_mm = Box::into_raw(Box::new(MmStruct::new(
                paging::init_pgd_for_test() as usize
            )));

            let start = 0x0080_0000;
            let split = start + PAGE_SIZE as u64;
            let flags = VM_READ | VM_WRITE | VM_MAYWRITE;
            let vma = make_vma(start, start + 2 * PAGE_SIZE as u64, flags);
            insert_vma(&mut *old_mm, vma).expect("insert split source VMA");

            assert_eq!(
                handle_mm_fault(vma, split, FAULT_FLAG_USER | FAULT_FLAG_WRITE),
                0,
                "fault second page before split"
            );
            let source_pte = pte_for_addr(old_mm, split);
            assert!(paging::pte_present(source_pte));
            assert!(paging::pte_write(source_pte));
            assert!(!(*vma).anon_vma.is_null());

            let right = vma_split(&mut *old_mm, vma, split).expect("split faulted VMA");
            assert_eq!((*right).vm_start, split);
            assert!(
                !(*right).anon_vma.is_null(),
                "split half with present PTEs must keep anon_vma"
            );

            dup_mmap(new_mm, old_mm).expect("dup_mmap");

            let parent_after = pte_for_addr(old_mm, split);
            let child_pte = pte_for_addr(new_mm, split);
            assert!(paging::pte_present(child_pte));
            assert!(
                !paging::pte_write(parent_after),
                "fork must write-protect parent PTE after split"
            );
            assert!(
                !paging::pte_write(child_pte),
                "fork must install read-only child COW PTE after split"
            );
            assert_eq!(
                paging::pte_pfn(child_pte),
                paging::pte_pfn(source_pte),
                "child must inherit the populated anonymous page, not fault a zero page later"
            );

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
