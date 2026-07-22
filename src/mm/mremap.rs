//! linux-parity: partial
//! linux-source: vendor/linux/mm/mremap.c
//! test-origin: linux:vendor/linux/mm/mremap.c
/// Memory remapping — `mremap`.
///
/// Implements VMA resize and relocation following `mm/mremap.c` semantics.
///
/// | Lupos function  | Linux equivalent         | Source              |
/// |-----------------|--------------------------|---------------------|
/// | `do_mremap`     | `do_mremap()`            | `mm/mremap.c:1915`  |
/// | `move_vma`      | `move_vma()`             | `mm/mremap.c`       |
/// | `move_ptes`     | `move_ptes()`            | `mm/mremap.c:197`   |
///
/// ## References
///
/// - Linux `mm/mremap.c` — primary reference
/// - Linux `tools/testing/selftests/mm/mremap_dontunmap.c` — parity tests
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::arch::x86::mm::paging::{
    _PAGE_TABLE, PAGE_MASK, PAGE_SHIFT, PAGE_SIZE, flush_tlb_range, p4d_offset, pgd_none,
    pgd_offset_pgd, pgd_t, pmd_alloc, pmd_huge, pmd_none, pmd_offset, pte_alloc, pte_none,
    pte_offset_kernel, pte_t, ptep_get_and_clear, pud_alloc, pud_huge, pud_none, pud_offset,
    set_pte,
};
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::mmap::{
    MAP_ANONYMOUS, MAP_FIXED, MAP_PRIVATE, PROT_READ, PROT_WRITE, SYSCTL_MAX_MAP_COUNT, TASK_SIZE,
    do_mmap, do_munmap, get_unmapped_area, sync_shared_file_range,
};
use crate::mm::vm_flags::{VM_DONTEXPAND, VM_PFNMAP, VmFlags, vm_flags_equal};
use crate::mm::vma::{
    find_vma, insert_vma, remove_vma, vm_area_dup, vm_area_free, vma_file_get, vma_file_put_raw,
    vma_open,
};

// ---------------------------------------------------------------------------
// MREMAP_* flag constants
// ---------------------------------------------------------------------------

/// mremap flag: allow kernel to choose a new address.
/// Ref: Linux `include/uapi/linux/mman.h`
pub const MREMAP_MAYMOVE: u32 = 1;
/// mremap flag: place the mapping at `new_addr` exactly.
pub const MREMAP_FIXED: u32 = 2;
/// mremap flag: move page tables without unmapping the source.
pub const MREMAP_DONTUNMAP: u32 = 4;

#[derive(Clone, Copy)]
struct MremapSegment {
    vma: *mut VmAreaStruct,
    start: u64,
    end: u64,
}

// ---------------------------------------------------------------------------
// move_ptes — relocate PTEs from source to destination
// ---------------------------------------------------------------------------

/// Copy PTEs from `[old_start, old_end)` to `[new_addr, new_addr+len)`.
///
/// For each present source PTE the entry is moved (source cleared, destination
/// set).  A TLB flush is issued over both ranges when done.
///
/// Ref: Linux `mm/mremap.c` — `move_ptes()` line 197
///
/// # Safety
/// Both source and destination address ranges must be page-aligned and must not
/// overlap.  `mm` must be exclusively accessible.
pub unsafe fn move_ptes(mm: &mut MmStruct, old_start: u64, old_end: u64, new_addr: u64) {
    let pgd_base = mm.pgd as *mut pgd_t;
    if pgd_base.is_null() {
        return;
    }

    let len = old_end - old_start;
    let mut offset = 0u64;
    while offset < len {
        let src_addr = old_start + offset;
        let dst_addr = new_addr + offset;

        // ── Source: walk to PTE ──────────────────────────────────────────────
        let src_pgdp = unsafe { pgd_offset_pgd(pgd_base, src_addr) };
        if unsafe { pgd_none(*src_pgdp) } {
            offset += PAGE_SIZE;
            continue;
        }
        let src_p4dp = unsafe { p4d_offset(src_pgdp, src_addr) };
        let src_pudp = unsafe { pud_offset(src_p4dp, src_addr) };
        if unsafe { pud_none(*src_pudp) || pud_huge(*src_pudp) } {
            offset += PAGE_SIZE;
            continue;
        }
        let src_pmdp = unsafe { pmd_offset(src_pudp, src_addr) };
        if unsafe { pmd_none(*src_pmdp) || pmd_huge(*src_pmdp) } {
            offset += PAGE_SIZE;
            continue;
        }
        let src_ptep = unsafe { pte_offset_kernel(src_pmdp, src_addr) };
        let pte: pte_t = unsafe { ptep_get_and_clear(core::ptr::null_mut(), src_addr, src_ptep) };
        if pte_none(pte) {
            offset += PAGE_SIZE;
            continue;
        }

        // ── Destination: allocate page-table levels on demand ────────────────
        let dst_pgdp = unsafe { pgd_offset_pgd(pgd_base, dst_addr) };
        let dst_pudp = match unsafe { pud_alloc(dst_pgdp, dst_addr, _PAGE_TABLE) } {
            Some(p) => p,
            None => {
                offset += PAGE_SIZE;
                continue;
            }
        };
        let dst_pmdp = match unsafe { pmd_alloc(dst_pudp, dst_addr, _PAGE_TABLE) } {
            Some(p) => p,
            None => {
                offset += PAGE_SIZE;
                continue;
            }
        };
        let dst_ptep = match unsafe { pte_alloc(dst_pmdp, dst_addr, _PAGE_TABLE) } {
            Some(p) => p,
            None => {
                offset += PAGE_SIZE;
                continue;
            }
        };

        unsafe {
            set_pte(dst_ptep, pte);
        }
        offset += PAGE_SIZE;
    }

    unsafe {
        flush_tlb_range(old_start, old_end);
    }
    unsafe {
        flush_tlb_range(new_addr, new_addr + len);
    }
}

// ---------------------------------------------------------------------------
// move_vma — relocate a VMA to a new address
// ---------------------------------------------------------------------------

/// Move `[old_start, old_start+old_len)` to `[new_addr, new_addr+old_len)`.
///
/// Creates a new VMA at the destination, moves PTEs, then (unless
/// `MREMAP_DONTUNMAP`) removes the source VMA.
///
/// Ref: Linux `mm/mremap.c` — `move_vma()`
///
/// # Safety
/// `mm` must be exclusively accessible.  `new_addr` must point to a free range.
pub unsafe fn move_vma(
    mm: &mut MmStruct,
    vma: *mut VmAreaStruct,
    old_start: u64,
    old_len: u64,
    new_addr: u64,
    flags: u32,
) -> Result<u64, i32> {
    let old_end = old_start + old_len;

    // Create the destination VMA (may merge with neighbours).
    let vm_flags = unsafe { (*vma).vm_flags };
    let vm_pgoff = unsafe { (*vma).vm_pgoff };
    let vm_file = unsafe { (*vma).vm_file };

    // Build prot bits from vm_flags to pass to do_mmap.
    let prot: u32 = {
        let mut p = 0u32;
        if vm_flags & crate::mm::vm_flags::VM_READ != 0 {
            p |= PROT_READ;
        }
        if vm_flags & crate::mm::vm_flags::VM_WRITE != 0 {
            p |= PROT_WRITE;
        }
        if vm_flags & crate::mm::vm_flags::VM_EXEC != 0 {
            p |= crate::mm::mmap::PROT_EXEC;
        }
        p
    };

    let mmap_flags = if vm_flags & crate::mm::vm_flags::VM_SHARED != 0 {
        crate::mm::mmap::MAP_SHARED | MAP_FIXED
    } else {
        MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED
    };

    // Reserve the destination address range.
    let took_file_ref = unsafe { vma_file_get(vma) };
    if let Err(err) = unsafe { do_mmap(mm, new_addr, old_len, prot, mmap_flags, vm_pgoff, vm_file) }
    {
        if took_file_ref {
            unsafe { vma_file_put_raw(vm_file) };
        }
        return Err(err);
    }

    if (flags & MREMAP_DONTUNMAP) != 0
        && vm_file != 0
        && vm_flags & crate::mm::vm_flags::VM_SHARED != 0
    {
        unsafe { sync_shared_file_range(mm, old_start, old_len)? };
    }

    // Transfer PTEs.
    unsafe {
        move_ptes(mm, old_start, old_end, new_addr);
    }

    // Unmap source unless MREMAP_DONTUNMAP.
    if (flags & MREMAP_DONTUNMAP) == 0 {
        unsafe { do_munmap(mm, old_start, old_len) }?;
    }

    Ok(new_addr)
}

fn collect_mremap_segments(mm: &MmStruct, start: u64, len: u64) -> Vec<MremapSegment> {
    let end = start.saturating_add(len);
    mm.mm_mt
        .collect_entries()
        .into_iter()
        .filter_map(|(vstart, vend_inclusive, ptr)| {
            let vend = vend_inclusive + 1;
            if vstart < end && vend > start {
                Some(MremapSegment {
                    vma: ptr as *mut VmAreaStruct,
                    start: vstart.max(start),
                    end: vend.min(end),
                })
            } else {
                None
            }
        })
        .collect()
}

unsafe fn reserve_mremap_destination_segment(
    mm: &mut MmStruct,
    segment: MremapSegment,
    dest: u64,
) -> Result<(), i32> {
    let src = unsafe { &*segment.vma };
    let len = segment.end - segment.start;
    let pgoff = src.vm_pgoff + ((segment.start - src.vm_start) >> PAGE_SHIFT);

    let dst_vma = unsafe { vm_area_dup(segment.vma) };
    unsafe {
        (*dst_vma).vm_start = dest;
        (*dst_vma).vm_end = dest + len;
        (*dst_vma).vm_pgoff = pgoff;
        match insert_vma(mm, dst_vma) {
            Ok(()) => {
                vma_open(dst_vma);
                Ok(())
            }
            Err(err) => {
                vm_area_free(dst_vma);
                Err(err)
            }
        }
    }
}

unsafe fn move_mremap_span(
    mm: &mut MmStruct,
    old_addr: u64,
    move_len: u64,
    dest: u64,
    flags: u32,
    source_unmap_len: u64,
) -> Result<u64, i32> {
    const EFAULT: i32 = -14;

    let segments = collect_mremap_segments(mm, old_addr, move_len);
    if segments
        .first()
        .is_none_or(|segment| segment.start != old_addr)
    {
        return Err(EFAULT);
    }

    unsafe {
        do_munmap(mm, dest, move_len)?;
    }

    let multi_vma_move = segments.len() > 1;
    for segment in segments {
        if multi_vma_move
            && crate::mm::shmem::userfaultfd_range_registered(
                segment.start,
                segment.end - segment.start,
            )
        {
            return Err(EFAULT);
        }
        let dest_segment = dest + (segment.start - old_addr);
        unsafe {
            let src = &*segment.vma;
            if (flags & MREMAP_DONTUNMAP) != 0
                && src.vm_file != 0
                && src.vm_flags & crate::mm::vm_flags::VM_SHARED != 0
            {
                sync_shared_file_range(mm, segment.start, segment.end - segment.start)?;
            }
            reserve_mremap_destination_segment(mm, segment, dest_segment)?;
            move_ptes(mm, segment.start, segment.end, dest_segment);
        }
    }

    if (flags & MREMAP_DONTUNMAP) == 0 {
        unsafe {
            do_munmap(mm, old_addr, source_unmap_len)?;
        }
    }

    Ok(dest)
}

fn vma_pages(vma: &VmAreaStruct) -> u64 {
    (vma.vm_end - vma.vm_start) >> PAGE_SHIFT
}

unsafe fn mremap_can_merge_adjacent_vmas(
    left: *const VmAreaStruct,
    right: *const VmAreaStruct,
) -> bool {
    let left = unsafe { &*left };
    let right = unsafe { &*right };

    left.vm_end == right.vm_start
        && vm_flags_equal(left.vm_flags, right.vm_flags)
        && left.vm_file == right.vm_file
        && left.vm_ops == right.vm_ops
        && left.vm_private_data == right.vm_private_data
        && left.vm_pgoff + vma_pages(left) == right.vm_pgoff
        && left.anon_vma.is_null()
        && right.anon_vma.is_null()
}

unsafe fn try_merge_mremap_expansion(mm: &mut MmStruct, left: *mut VmAreaStruct) {
    let left_end = unsafe { (*left).vm_end };
    let Some(right) = find_vma(mm, left_end) else {
        return;
    };
    if right == left || unsafe { (*right).vm_start } != left_end {
        return;
    }
    if !unsafe { mremap_can_merge_adjacent_vmas(left, right) } {
        return;
    }

    let right_end = unsafe { (*right).vm_end };
    unsafe {
        remove_vma(mm, left);
        remove_vma(mm, right);
        (*left).vm_end = right_end;
        if insert_vma(mm, left).is_ok() {
            vm_area_free(right);
        } else {
            (*left).vm_end = left_end;
            let _ = insert_vma(mm, left);
            let _ = insert_vma(mm, right);
        }
    }
}

// ---------------------------------------------------------------------------
// do_mremap
// ---------------------------------------------------------------------------

/// Core mremap handler.
///
/// ## Error codes (matching Linux)
/// - `-EINVAL` (-22): unknown flags; `MREMAP_DONTUNMAP` + resize; zero
///   `new_len`; unaligned addresses.
/// - `-EFAULT` (-14): no VMA at `addr`; gap in multi-VMA span.
/// - `-ENOMEM` (-12): `new_len > TASK_SIZE`; map count would overflow; no
///   free gap for expansion + move.
///
/// ## Three cases
/// 1. `new_len == old_len` (+ optional `MREMAP_DONTUNMAP`): copy or no-op.
/// 2. `new_len < old_len`: shrink in-place via `do_munmap`.
/// 3. `new_len > old_len`: try expand in-place; if blocked + `MREMAP_MAYMOVE`,
///    move to a new location.
///
/// Ref: Linux `mm/mremap.c` — `do_mremap()` line 1915
///
/// # Safety
/// `mm` must be exclusively accessible (mmap_lock held for write).
pub unsafe fn do_mremap(
    mm: &mut MmStruct,
    addr: u64,
    old_len: u64,
    new_len: u64,
    flags: u32,
    new_addr: u64,
) -> Result<u64, i32> {
    const EINVAL: i32 = -22;
    const EFAULT: i32 = -14;
    const ENOMEM: i32 = -12;

    // 1. Validate flags.
    let known = MREMAP_MAYMOVE | MREMAP_FIXED | MREMAP_DONTUNMAP;
    if flags & !known != 0 {
        return Err(EINVAL);
    }
    // MREMAP_FIXED implies MREMAP_MAYMOVE.
    if (flags & MREMAP_FIXED) != 0 && (flags & MREMAP_MAYMOVE) == 0 {
        return Err(EINVAL);
    }
    // MREMAP_DONTUNMAP requires MREMAP_MAYMOVE and old_len == new_len.
    if (flags & MREMAP_DONTUNMAP) != 0 {
        if (flags & MREMAP_MAYMOVE) == 0 {
            return Err(EINVAL);
        }
        if old_len != new_len {
            return Err(EINVAL);
        }
    }

    // 2. new_len must be nonzero.
    if new_len == 0 {
        return Err(EINVAL);
    }

    // 3. Page-align lengths.
    let old_len = old_len.wrapping_add(crate::arch::x86::mm::paging::PAGE_SIZE - 1)
        & crate::arch::x86::mm::paging::PAGE_MASK;
    let new_len = new_len.wrapping_add(crate::arch::x86::mm::paging::PAGE_SIZE - 1)
        & crate::arch::x86::mm::paging::PAGE_MASK;

    if new_len > TASK_SIZE {
        return Err(ENOMEM);
    }

    // 4. Addresses must be page-aligned.
    if addr & !PAGE_MASK != 0 {
        return Err(EINVAL);
    }
    if (flags & MREMAP_FIXED) != 0 && new_addr & !PAGE_MASK != 0 {
        return Err(EINVAL);
    }

    // 5. Look up VMA at addr.
    let vma_ptr = find_vma(mm, addr).ok_or(EFAULT)?;
    let vstart = unsafe { (*vma_ptr).vm_start };
    let vend = unsafe { (*vma_ptr).vm_end };
    if vstart > addr {
        return Err(EFAULT); // gap before VMA
    }
    let old_end = addr.checked_add(old_len).ok_or(ENOMEM)?;
    let old_range_single_vma = vend >= old_end;
    let source_flags = unsafe { (*vma_ptr).vm_flags };

    // Linux rejects MREMAP_DONTUNMAP for remap/PFN VMAs: duplicating the VMA
    // would leave two independently managed raw-PFN mappings.
    if (flags & MREMAP_DONTUNMAP) != 0 && source_flags & (VM_DONTEXPAND | VM_PFNMAP) != 0 {
        return Err(EINVAL);
    }

    // ── Case 1: same size ────────────────────────────────────────────────────
    if new_len == old_len {
        if (flags & MREMAP_DONTUNMAP) != 0 || (flags & MREMAP_FIXED) != 0 {
            // Determine destination address.
            let dest = if (flags & MREMAP_FIXED) != 0 {
                new_addr
            } else {
                unsafe { get_unmapped_area(mm, 0, new_len, 0) }?
            };
            if ranges_overlap(dest, new_len, addr, old_len) {
                return Err(EINVAL);
            }
            if !old_range_single_vma {
                return unsafe { move_mremap_span(mm, addr, old_len, dest, flags, old_len) };
            }
            return unsafe { move_vma(mm, vma_ptr, addr, old_len, dest, flags) };
        }
        return Ok(addr); // pure no-op
    }

    // ── Case 2: shrink ───────────────────────────────────────────────────────
    if new_len < old_len {
        if (flags & MREMAP_FIXED) != 0 {
            let dest = new_addr;
            if ranges_overlap(dest, new_len, addr, old_len) {
                return Err(EINVAL);
            }
            return unsafe { move_mremap_span(mm, addr, new_len, dest, flags, old_len) };
        }
        let excess = old_len - new_len;
        unsafe { do_munmap(mm, addr + new_len, excess)? };
        return Ok(addr);
    }

    // ── Case 3: expand ───────────────────────────────────────────────────────
    let expand = new_len - old_len;
    if !old_range_single_vma {
        return Err(EFAULT);
    }
    if source_flags & (VM_DONTEXPAND | VM_PFNMAP) != 0 {
        return Err(EFAULT);
    }

    // Try in-place expansion: is the gap after the VMA large enough?
    let next_start = {
        // Find first VMA whose start is > vend.
        mm.mm_mt
            .collect_entries()
            .into_iter()
            .find(|&(s, _, _)| s >= vend)
            .map(|(s, _, _)| s)
            .unwrap_or(TASK_SIZE)
    };

    if vend + expand <= next_start {
        // Gap is big enough — extend the VMA in-place.
        unsafe {
            remove_vma(mm, vma_ptr);
            (*vma_ptr).vm_end = vend + expand;
            insert_vma(mm, vma_ptr)?;
            try_merge_mremap_expansion(mm, vma_ptr);
        }
        return Ok(addr);
    }

    // In-place expansion failed.
    if (flags & MREMAP_MAYMOVE) == 0 {
        return Err(ENOMEM);
    }

    // Enforce VMA count limit (moving creates an extra VMA momentarily).
    if mm.map_count + 2 >= SYSCTL_MAX_MAP_COUNT {
        return Err(ENOMEM);
    }

    // Find a new location and move.
    let dest = if (flags & MREMAP_FIXED) != 0 {
        new_addr
    } else {
        unsafe { get_unmapped_area(mm, 0, new_len, 0) }?
    };

    // Destination must not overlap source.
    if ranges_overlap(dest, new_len, addr, old_len) {
        return Err(EINVAL);
    }

    // Map the full new_len at destination.
    let vm_flags = unsafe { (*vma_ptr).vm_flags };
    let prot: u32 = {
        let mut p = 0u32;
        if vm_flags & crate::mm::vm_flags::VM_READ != 0 {
            p |= PROT_READ;
        }
        if vm_flags & crate::mm::vm_flags::VM_WRITE != 0 {
            p |= PROT_WRITE;
        }
        if vm_flags & crate::mm::vm_flags::VM_EXEC != 0 {
            p |= crate::mm::mmap::PROT_EXEC;
        }
        p
    };
    unsafe {
        do_mmap(
            mm,
            dest,
            new_len,
            prot,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
            0,
            0,
        )
    }?;

    // Move PTEs from old region.
    unsafe {
        move_ptes(mm, addr, addr + old_len, dest);
    }

    // Remove old VMA (MREMAP_DONTUNMAP check not needed here as it requires old==new).
    unsafe { do_munmap(mm, addr, old_len) }?;

    Ok(dest)
}

fn ranges_overlap(a_start: u64, a_len: u64, b_start: u64, b_len: u64) -> bool {
    let Some(a_end) = a_start.checked_add(a_len) else {
        return true;
    };
    let Some(b_end) = b_start.checked_add(b_len) else {
        return true;
    };
    a_start < b_end && a_end > b_start
}

// ---------------------------------------------------------------------------
// Unit tests — ported from vendor/linux/tools/testing/selftests/mm/mremap_dontunmap.c
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::mm::mm_types::MmStruct;
    use crate::mm::mmap::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, do_mmap, do_munmap};
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;
    use crate::mm::vma::find_vma;

    fn make_mm() -> MmStruct {
        MmStruct::new(0)
    }

    fn addr_is_mapped(mm: &MmStruct, addr: u64) -> bool {
        find_vma(mm, addr)
            .map(|vma| {
                let vma = unsafe { &*vma };
                vma.vm_start <= addr && addr < vma.vm_end
            })
            .unwrap_or(false)
    }

    fn map_sparse_test_range(mm: &mut MmStruct, start: u64, pages: u64) {
        unsafe {
            do_mmap(
                mm,
                start,
                pages * PAGE_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();

        let mut page = 1;
        while page < pages {
            unsafe { do_munmap(mm, start + page * PAGE_SIZE, PAGE_SIZE) }.unwrap();
            page += 2;
        }
    }

    // ── Test 1 ─────────────────────────────────────────────────────────────────
    // Shrinking a VMA truncates its end address.
    #[test]
    fn mremap_shrink_truncates_end() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x20000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        let new_addr = unsafe { do_mremap(&mut mm, 0x10000, 0x20000, 0x10000, 0, 0) }.unwrap();

        assert_eq!(new_addr, 0x10000, "shrink must keep addr");
        let vma = unsafe { &*find_vma(&mm, 0x10000).unwrap() };
        assert_eq!(vma.vm_start, 0x10000);
        assert_eq!(vma.vm_end, 0x20000, "VMA end must be truncated");
        assert!(
            find_vma(&mm, 0x25000).is_none(),
            "excess pages must be unmapped"
        );
    }

    // ── Test 2 ─────────────────────────────────────────────────────────────────
    // Expanding into an adjacent free gap succeeds in-place.
    #[test]
    fn mremap_expand_into_free_gap() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        // Anchor at a fixed address to guarantee free space after it.
        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();

        let new_addr = unsafe { do_mremap(&mut mm, 0x10000, 0x10000, 0x20000, 0, 0) }.unwrap();

        assert_eq!(new_addr, 0x10000, "in-place expand must keep addr");
        let vma = unsafe { &*find_vma(&mm, 0x10000).unwrap() };
        assert_eq!(vma.vm_end, 0x30000, "VMA must extend to new_len");
    }

    #[test]
    fn mremap_expand_merge_coalesces_adjacent_vma() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        let page = PAGE_SIZE;
        let start = 0x10000;

        unsafe {
            do_mmap(
                &mut mm,
                start,
                3 * page,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();
        unsafe { do_munmap(&mut mm, start + page, page) }.unwrap();
        assert_eq!(mm.map_count, 2, "middle unmap must split the VMA");

        let new_addr = unsafe { do_mremap(&mut mm, start, page, 2 * page, 0, 0) }.unwrap();

        assert_eq!(new_addr, start);
        assert_eq!(
            mm.map_count, 1,
            "expanded VMA must merge with its right neighbor"
        );
        let vma = unsafe { &*find_vma(&mm, start).unwrap() };
        assert_eq!(vma.vm_start, start);
        assert_eq!(vma.vm_end, start + 3 * page);
    }

    // ── Test 3 ─────────────────────────────────────────────────────────────────
    // Expanding when blocked by the next VMA, without MREMAP_MAYMOVE → ENOMEM.
    #[test]
    fn mremap_shrink_accepts_sparse_vma_range() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        let page = PAGE_SIZE;
        let start = 0x20000;

        map_sparse_test_range(&mut mm, start, 10);
        assert!(addr_is_mapped(&mm, start));
        assert!(addr_is_mapped(&mm, start + 2 * page));

        let new_addr = unsafe { do_mremap(&mut mm, start, 10 * page, page, 0, 0) }.unwrap();

        assert_eq!(new_addr, start);
        assert!(addr_is_mapped(&mm, start));
        for page_index in 1..10 {
            assert!(
                !addr_is_mapped(&mm, start + page_index * page),
                "page {page_index} must be unmapped after sparse shrink"
            );
        }
    }

    #[test]
    fn mremap_fixed_moves_sparse_vma_range() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        let page = PAGE_SIZE;
        let start = 0x40000;
        let dest = 0x80000;

        map_sparse_test_range(&mut mm, start, 11);
        unsafe {
            do_mmap(
                &mut mm,
                dest,
                22 * page,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();
        unsafe { do_munmap(&mut mm, dest, 22 * page) }.unwrap();

        let new_addr = unsafe {
            do_mremap(
                &mut mm,
                start,
                11 * page,
                11 * page,
                MREMAP_MAYMOVE | MREMAP_FIXED,
                dest,
            )
        }
        .unwrap();

        assert_eq!(new_addr, dest);
        for page_index in [0_u64, 2, 4, 6, 8, 10] {
            assert!(
                addr_is_mapped(&mm, dest + page_index * page),
                "destination page {page_index} must be mapped"
            );
        }
        for page_index in [1_u64, 3, 5, 7, 9] {
            assert!(
                !addr_is_mapped(&mm, dest + page_index * page),
                "destination hole {page_index} must stay unmapped"
            );
        }
        for page_index in 0..11 {
            assert!(
                !addr_is_mapped(&mm, start + page_index * page),
                "source page {page_index} must be unmapped after fixed move"
            );
        }
    }

    #[test]
    fn mremap_expand_fails_if_blocked_no_maymove() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        // Two adjacent VMAs with different flags to prevent vma_merge:
        // [0x10000, 0x20000) PROT_READ and [0x20000, 0x30000) PROT_READ|PROT_WRITE.
        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();
        unsafe {
            do_mmap(
                &mut mm,
                0x20000,
                0x10000,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();

        let r = unsafe {
            do_mremap(
                &mut mm, 0x10000, 0x10000, 0x20000, 0, /* no MAYMOVE */
                0,
            )
        };
        assert_eq!(r, Err(-12)); // ENOMEM
    }

    // ── Test 4 ─────────────────────────────────────────────────────────────────
    // Port of: mremap_dontunmap.c — MREMAP_DONTUNMAP leaves the source VMA.
    #[test]
    fn mremap_fixed_rejects_overlapping_destination() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x4000,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();

        let r = unsafe {
            do_mremap(
                &mut mm,
                0x10000,
                0x4000,
                0x4000,
                MREMAP_MAYMOVE | MREMAP_FIXED,
                0x12000,
            )
        };
        assert_eq!(r, Err(-22));
        let vma = unsafe { &*find_vma(&mm, 0x10000).unwrap() };
        assert_eq!(vma.vm_start, 0x10000);
        assert_eq!(vma.vm_end, 0x14000);
    }

    #[test]
    fn mremap_dontunmap_leaves_source_vma() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        // Source VMA [0x10000, 0x20000).
        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | crate::mm::mmap::MAP_FIXED,
                0,
                0,
            )
        }
        .unwrap();

        let dest = unsafe {
            do_mremap(
                &mut mm,
                0x10000,
                0x10000,
                0x10000,
                MREMAP_MAYMOVE | MREMAP_DONTUNMAP,
                0,
            )
        }
        .unwrap();

        // Source must still be present.
        assert!(
            find_vma(&mm, 0x10000).is_some(),
            "MREMAP_DONTUNMAP: source VMA must remain"
        );

        // Destination must also be present at a different address.
        assert_ne!(dest, 0x10000, "destination must differ from source");
        assert!(
            find_vma(&mm, dest).is_some(),
            "MREMAP_DONTUNMAP: destination VMA must exist"
        );
    }
}
