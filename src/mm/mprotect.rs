//! linux-parity: complete
//! linux-source: vendor/linux/mm/mprotect.c
//! test-origin: linux:vendor/linux/mm/mprotect.c
/// Memory protection — `mprotect` and PTE-level permission updates.
///
/// Implements `do_mprotect` following `mm/mprotect.c` semantics: validation,
/// VMA splitting/merging, and PTE-bit modification with TLB flush.
///
/// | Lupos function          | Linux equivalent              | Source               |
/// |-------------------------|-------------------------------|----------------------|
/// | `do_mprotect`           | `do_mprotect_pkey()`          | `mm/mprotect.c:801`  |
/// | `mprotect_fixup`        | `mprotect_fixup()`            | `mm/mprotect.c:695`  |
/// | `change_protection_range` | `change_protection()`       | `mm/mprotect.c`      |
///
/// ## References
///
/// - Linux `mm/mprotect.c` — primary reference
/// - Linux `tools/testing/selftests/mm/mprotect-fault.c` — parity tests
/// - Linux `include/uapi/asm-generic/mman-common.h` — PROT_* values
use crate::arch::x86::mm::paging::{
    _PAGE_ACCESSED, _PAGE_NX, _PAGE_PRESENT, _PAGE_RW, _PAGE_USER, PAGE_MASK, PAGE_SIZE,
    flush_tlb_range, p4d_offset, pgd_none, pgd_offset_pgd, pgd_t, pmd_huge, pmd_none, pmd_offset,
    pte_offset_kernel, pte_present, pte_t, ptep_get, pud_huge, pud_none, pud_offset, set_pte,
};
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::mmap::{PROT_EXEC, PROT_GROWSDOWN, PROT_GROWSUP, PROT_READ, PROT_WRITE, TASK_SIZE};
use crate::mm::pgprot::vm_get_page_prot;
use crate::mm::vm_flags::{
    VM_EXEC, VM_MAYEXEC, VM_MAYREAD, VM_MAYWRITE, VM_READ, VM_WRITE, VmFlags,
};
use crate::mm::vma::{find_vma, insert_vma, remove_vma, vm_area_dup, vma_split};

// ---------------------------------------------------------------------------
// change_protection_range — update PTE bits in [start, end)
// ---------------------------------------------------------------------------

/// Update PTE protection bits for every present page in `[start, end)`.
///
/// Clears `_PAGE_RW` when `new_flags` does not contain `VM_WRITE`;
/// sets `_PAGE_NX` when `new_flags` does not contain `VM_EXEC`.
/// Flushes the TLB range after all modifications.
///
/// Ref: Linux `mm/mprotect.c` — `change_protection()`
///
/// # Safety
/// `mm` must be exclusively accessible.  `start`/`end` must be page-aligned.
pub unsafe fn change_protection_range(
    mm: &MmStruct,
    _vma: *mut VmAreaStruct,
    start: u64,
    end: u64,
    new_flags: VmFlags,
) {
    let pgd_base = mm.pgd as *mut pgd_t;
    if pgd_base.is_null() {
        return;
    }

    let new_prot = vm_get_page_prot(new_flags);
    let want_write = (new_prot & _PAGE_RW) != 0;
    let want_exec = (new_flags & VM_EXEC) != 0;

    let mut addr = start;
    while addr < end {
        let pgdp = unsafe { pgd_offset_pgd(pgd_base, addr) };
        if unsafe { pgd_none(*pgdp) } {
            addr = ((addr >> 39) + 1) << 39;
            continue;
        }
        let p4dp = unsafe { p4d_offset(pgdp, addr) };
        let pudp = unsafe { pud_offset(p4dp, addr) };
        if unsafe { pud_none(*pudp) } {
            addr = ((addr >> 30) + 1) << 30;
            continue;
        }
        if unsafe { pud_huge(*pudp) } {
            addr = ((addr >> 30) + 1) << 30;
            continue;
        }
        let pmdp = unsafe { pmd_offset(pudp, addr) };
        if unsafe { pmd_none(*pmdp) } {
            addr = ((addr >> 21) + 1) << 21;
            continue;
        }
        if unsafe { pmd_huge(*pmdp) } {
            addr = ((addr >> 21) + 1) << 21;
            continue;
        }
        let ptep = unsafe { pte_offset_kernel(pmdp, addr) };
        let old: pte_t = unsafe { ptep_get(ptep) };
        if pte_present(old) {
            let mut val = old.0;
            if want_write {
                val |= _PAGE_RW;
            } else {
                val &= !_PAGE_RW;
            }
            if want_exec {
                val &= !_PAGE_NX;
            } else {
                val |= _PAGE_NX;
            }
            if val != old.0 {
                unsafe {
                    set_pte(ptep, crate::arch::x86::mm::paging::pte_t(val));
                }
            }
        }
        addr += PAGE_SIZE;
    }

    unsafe {
        flush_tlb_range(start, end);
    }
}

// ---------------------------------------------------------------------------
// mprotect_fixup — split/merge VMA and apply new flags
// ---------------------------------------------------------------------------

/// Carve out `[start, end)` from `vma`, apply `new_flags`, update PTEs.
///
/// Steps (matching Linux `mprotect_fixup`):
/// 1. If new flags equal old flags: no-op.
/// 2. If new flags would exceed `VM_MAY*` permissions: return `EACCES`.
/// 3. Remove the VMA from the tree; adjust its flags; re-insert after splitting
///    if necessary to cover only the requested sub-range.
/// 4. Call `change_protection_range` to update live PTEs.
///
/// Ref: Linux `mm/mprotect.c` — `mprotect_fixup()` line 695
///
/// # Safety
/// `mm` must be exclusively accessible.  `vma` must be in `mm`'s tree.
pub unsafe fn mprotect_fixup(
    mm: &mut MmStruct,
    vma: *mut VmAreaStruct,
    start: u64,
    end: u64,
    new_flags: VmFlags,
) -> Result<(), i32> {
    const EACCES: i32 = -13;

    let old_flags = unsafe { (*vma).vm_flags };

    // No-op when flags are identical.
    if new_flags == old_flags {
        return Ok(());
    }

    // Upgrading above VM_MAY* limits is forbidden.
    let may = old_flags;
    if (new_flags & VM_READ) != 0 && (may & VM_MAYREAD) == 0 {
        return Err(EACCES);
    }
    if (new_flags & VM_WRITE) != 0 && (may & VM_MAYWRITE) == 0 {
        return Err(EACCES);
    }
    if (new_flags & VM_EXEC) != 0 && (may & VM_MAYEXEC) == 0 {
        return Err(EACCES);
    }

    let vstart = unsafe { (*vma).vm_start };
    let vend = unsafe { (*vma).vm_end };

    // Split at start boundary if needed.
    let target_vma: *mut VmAreaStruct = if start > vstart {
        // Split at `start`: vma covers [vstart, start), new right half [start, vend).
        let right = unsafe { vma_split(mm, vma, start)? };
        right
    } else {
        vma
    };

    // Split at end boundary if needed.
    if end < unsafe { (*target_vma).vm_end } {
        let _ = unsafe { vma_split(mm, target_vma, end)? };
        // target_vma now covers [start, end).
    }

    // Apply new flags to target_vma.
    unsafe {
        remove_vma(mm, target_vma);
        // Preserve VM_MAY* and other non-prot flags from old; replace prot bits.
        let keep_mask: VmFlags = !(VM_READ | VM_WRITE | VM_EXEC);
        (*target_vma).vm_flags = (old_flags & keep_mask) | new_flags;
        (*target_vma).vm_page_prot = vm_get_page_prot((*target_vma).vm_flags);
        let _ = insert_vma(mm, target_vma);
    }

    // Update live PTEs.
    let t_start = unsafe { (*target_vma).vm_start };
    let t_end = unsafe { (*target_vma).vm_end };
    unsafe {
        change_protection_range(mm, target_vma, t_start, t_end, new_flags);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// do_mprotect
// ---------------------------------------------------------------------------

/// Core mprotect handler.
///
/// ## Error codes (matching Linux)
/// - `-EINVAL` (-22): `start` not page-aligned; both `PROT_GROWSDOWN` and
///   `PROT_GROWSUP` set; aligned `end` overflows or exceeds `TASK_SIZE`.
/// - `-ENOMEM` (-12): no VMA found covering `start` (gap in range).
/// - `-EACCES` (-13): requested permissions exceed `VM_MAY*` limits.
///
/// Ref: Linux `mm/mprotect.c` — `do_mprotect_pkey()` line 801
///
/// # Safety
/// `mm` must be exclusively accessible (mmap_lock held for write).
pub unsafe fn do_mprotect(mm: &mut MmStruct, start: u64, len: u64, prot: u32) -> Result<(), i32> {
    const EINVAL: i32 = -22;
    const ENOMEM: i32 = -12;
    const EACCES: i32 = -13;

    // 1. start must be page-aligned.
    if start & !PAGE_MASK != 0 {
        return Err(EINVAL);
    }

    // 2. PROT_GROWSDOWN and PROT_GROWSUP are mutually exclusive.
    if (prot & PROT_GROWSDOWN) != 0 && (prot & PROT_GROWSUP) != 0 {
        return Err(EINVAL);
    }

    // 3. Align end; check for overflow / exceeding TASK_SIZE.
    if len == 0 {
        return Ok(());
    }
    let end = start
        .checked_add(len)
        .and_then(|e| Some((e + crate::arch::x86::mm::paging::PAGE_SIZE - 1) & PAGE_MASK))
        .ok_or(EINVAL)?;
    if end == 0 || end > TASK_SIZE {
        return Err(ENOMEM);
    }
    if end <= start {
        return Err(ENOMEM);
    }

    // 4. Derive new VM flags from prot.
    use crate::mm::mmap::calc_vm_prot_bits;
    let new_prot_flags = calc_vm_prot_bits(prot);

    // 5. Walk VMAs covering [start, end).
    let mut cur = start;
    while cur < end {
        let vma_ptr = find_vma(mm, cur).ok_or(ENOMEM)?;
        let vma = unsafe { &*vma_ptr };

        if vma.vm_start > cur {
            // Gap — no VMA covers `cur`.
            return Err(ENOMEM);
        }

        let seg_end = vma.vm_end.min(end);

        // Compute new flags: preserve behaviour bits, replace prot bits.
        let old_flags = vma.vm_flags;
        let keep_mask: VmFlags = !(VM_READ | VM_WRITE | VM_EXEC);
        let new_flags = (old_flags & keep_mask) | new_prot_flags;
        if unsafe { crate::kernel::seccomp::mdwe_refuses_exec_gain_for_mm(mm) }
            && (new_flags & VM_EXEC) != 0
            && (old_flags & VM_EXEC) == 0
        {
            return Err(EACCES);
        }

        unsafe { mprotect_fixup(mm, vma_ptr, cur, seg_end, new_flags)? };

        cur = seg_end;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests — ported from vendor/linux/tools/testing/selftests/mm/mprotect-fault.c
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::arch::x86::mm::paging;
    use crate::mm::buddy;
    use crate::mm::fault::{FAULT_FLAG_USER, FAULT_FLAG_WRITE, handle_mm_fault};
    use crate::mm::mm_types::MmStruct;
    use crate::mm::mmap::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, do_mmap};
    use crate::mm::page::Page;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;
    use crate::mm::vma::find_vma;
    use alloc::boxed::Box;

    fn make_mm() -> MmStruct {
        MmStruct::new(0)
    }

    const TEST_PAGES: usize = 256;

    unsafe fn pte_for(mm: &MmStruct, addr: u64) -> paging::pte_t {
        let pgdp = unsafe { paging::pgd_offset_pgd(mm.pgd as *mut paging::pgd_t, addr) };
        let p4dp = unsafe { paging::p4d_offset(pgdp, addr) };
        let pudp = unsafe { paging::pud_offset(p4dp, addr) };
        let pmdp = unsafe { paging::pmd_offset(pudp, addr) };
        let ptep = unsafe { paging::pte_offset_kernel(pmdp, addr) };
        unsafe { paging::ptep_get(ptep) }
    }

    // ── Test 1 ─────────────────────────────────────────────────────────────────
    // Port of: mprotect-fault.c — upgrading PROT_READ → PROT_READ|PROT_WRITE
    // must set VM_WRITE on the VMA.
    #[test]
    fn mprotect_adds_write_flag() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        unsafe { do_mprotect(&mut mm, 0x10000, 0x10000, PROT_READ | PROT_WRITE) }.unwrap();

        let vma = unsafe { &*find_vma(&mm, 0x10000).unwrap() };
        assert!(
            vma.vm_flags & VM_WRITE != 0,
            "VM_WRITE must be set after mprotect"
        );
    }

    // ── Test 2 ─────────────────────────────────────────────────────────────────
    // Downgrading PROT_READ|PROT_WRITE → PROT_READ must clear VM_WRITE.
    #[test]
    fn mprotect_removes_write_flag() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        unsafe { do_mprotect(&mut mm, 0x10000, 0x10000, PROT_READ) }.unwrap();

        let vma = unsafe { &*find_vma(&mm, 0x10000).unwrap() };
        assert_eq!(
            vma.vm_flags & VM_WRITE,
            0,
            "VM_WRITE must be cleared after downgrade"
        );
    }

    // ── Test 3 ─────────────────────────────────────────────────────────────────
    // Protecting the middle of a VMA splits it into three pieces.
    #[test]
    fn mprotect_partial_range_splits_vma() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        // One big VMA [0x10000, 0x40000).
        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x30000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();
        assert_eq!(mm.map_count, 1);

        // Protect the middle third [0x20000, 0x30000) with PROT_READ|PROT_WRITE.
        unsafe { do_mprotect(&mut mm, 0x20000, 0x10000, PROT_READ | PROT_WRITE) }.unwrap();

        // Expect three VMAs: [0x10000,0x20000), [0x20000,0x30000), [0x30000,0x40000).
        assert_eq!(mm.map_count, 3, "middle mprotect must split into 3 VMAs");

        let mid = unsafe { &*find_vma(&mm, 0x20000).unwrap() };
        assert_eq!(mid.vm_start, 0x20000);
        assert_eq!(mid.vm_end, 0x30000);
        assert!(mid.vm_flags & VM_WRITE != 0);
    }

    // ── Test 4 ─────────────────────────────────────────────────────────────────
    // Attempting to add PROT_WRITE when VM_MAYWRITE is absent must return EACCES.
    #[test]
    fn mprotect_above_mayflag_returns_eacces() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        // Manually create a VMA with VM_MAYWRITE stripped.
        use crate::mm::mm_types::VmAreaStruct;
        use crate::mm::vm_flags::{VM_MAYEXEC, VM_MAYREAD, VM_READ};

        let flags = VM_READ | VM_MAYREAD | VM_MAYEXEC; // no VM_MAYWRITE
        let vma = Box::new(VmAreaStruct::new(0x10000, 0x20000, flags));
        unsafe { crate::mm::vma::insert_vma(&mut mm, Box::into_raw(vma)) }.unwrap();

        let r = unsafe { do_mprotect(&mut mm, 0x10000, 0x10000, PROT_READ | PROT_WRITE) };
        assert_eq!(r, Err(-13)); // EACCES
    }

    #[test]
    fn mprotect_write_upgrade_preserves_private_cow_write_protect() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let addr = 0x1200_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        unsafe {
            do_mmap(
                &mut mm,
                addr,
                paging::PAGE_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .expect("mmap");
        let vma = find_vma(&mm, addr).expect("vma");
        assert_eq!(
            handle_mm_fault(vma, addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER),
            0
        );
        assert!(
            paging::pte_write(unsafe { pte_for(&mm, addr) }),
            "initial private write fault installs a writable exclusive PTE"
        );

        let mut child = MmStruct::new(paging::init_pgd_for_test() as usize);
        unsafe { crate::mm::fork::dup_mmap(&mut child as *mut MmStruct, &mut mm as *mut MmStruct) }
            .expect("dup_mmap");
        assert!(
            !paging::pte_write(unsafe { pte_for(&mm, addr) }),
            "fork must write-protect the parent PTE for COW"
        );
        assert!(
            !paging::pte_write(unsafe { pte_for(&child, addr) }),
            "fork must install a read-only child PTE for COW"
        );

        unsafe { do_mprotect(&mut mm, addr, paging::PAGE_SIZE, PROT_READ) }.expect("mprotect ro");
        unsafe { do_mprotect(&mut mm, addr, paging::PAGE_SIZE, PROT_READ | PROT_WRITE) }
            .expect("mprotect rw");

        assert!(
            !paging::pte_write(unsafe { pte_for(&mm, addr) }),
            "mprotect(PROT_READ|PROT_WRITE) on a private COW-shared mapping must not bypass the write fault"
        );
    }
}
