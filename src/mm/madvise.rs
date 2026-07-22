//! linux-parity: partial
//! linux-source: vendor/linux/mm/madvise.c
//! test-origin: linux:vendor/linux/mm/madvise.c
use crate::arch::x86::mm::paging::{
    _PAGE_TABLE, PAGE_MASK, PAGE_SHIFT, PAGE_SIZE, flush_tlb_page, p4d_offset, pgd_none,
    pgd_offset_pgd, pgd_t, pmd_alloc, pmd_huge, pmd_none, pmd_offset, pte_alloc, pte_clear,
    pte_offset_kernel, pte_t, ptep_get, pud_alloc, pud_huge, pud_none, pud_offset, set_pte_at,
};
/// Memory advice — `madvise`.
///
/// Implements the subset of `madvise` behaviors relevant for anonymous memory
/// in Milestone 13.  File-backed and swap behaviors are deferred to M15/M17.
///
/// | Lupos function          | Linux equivalent              | Source               |
/// |-------------------------|-------------------------------|----------------------|
/// | `do_madvise`            | `do_madvise()`                | `mm/madvise.c:1345`  |
/// | `madvise_vma_behavior`  | `madvise_vma_behavior()`      | `mm/madvise.c:825`   |
///
/// ## References
///
/// - Linux `mm/madvise.c` — primary reference
/// - Linux `tools/testing/selftests/mm/madv_populate.c` — parity tests
/// - Linux `include/uapi/asm-generic/mman-common.h` — MADV_* values
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::mmap::unmap_page_range;
use crate::mm::swap::{PTE_MARKER_GUARD, is_guard_pte_marker, make_pte_marker};
use crate::mm::vm_flags::{
    VM_DONTCOPY, VM_HUGEPAGE, VM_HUGETLB, VM_LOCKED, VM_NOHUGEPAGE, VM_READ, VM_SPECIAL, VM_WRITE,
};
use crate::mm::vma::{find_vma, vma_split};

// ---------------------------------------------------------------------------
// MADV_* advisory constants — include/uapi/asm-generic/mman-common.h
// ---------------------------------------------------------------------------

pub const MADV_NORMAL: i32 = 0;
pub const MADV_RANDOM: i32 = 1;
pub const MADV_SEQUENTIAL: i32 = 2;
pub const MADV_WILLNEED: i32 = 3;
pub const MADV_DONTNEED: i32 = 4;
pub const MADV_FREE: i32 = 8;
pub const MADV_REMOVE: i32 = 9;
pub const MADV_DONTFORK: i32 = 10;
pub const MADV_DOFORK: i32 = 11;
pub const MADV_HWPOISON: i32 = 100;
pub const MADV_SOFT_OFFLINE: i32 = 101;
pub const MADV_MERGEABLE: i32 = 12;
pub const MADV_UNMERGEABLE: i32 = 13;
pub const MADV_HUGEPAGE: i32 = 14;
pub const MADV_NOHUGEPAGE: i32 = 15;
pub const MADV_DONTDUMP: i32 = 16;
pub const MADV_DODUMP: i32 = 17;
pub const MADV_WIPEONFORK: i32 = 18;
pub const MADV_KEEPONFORK: i32 = 19;
pub const MADV_COLD: i32 = 20;
pub const MADV_PAGEOUT: i32 = 21;
pub const MADV_POPULATE_READ: i32 = 22;
pub const MADV_POPULATE_WRITE: i32 = 23;
pub const MADV_DONTNEED_LOCKED: i32 = 24;
pub const MADV_COLLAPSE: i32 = 25;
pub const MADV_GUARD_INSTALL: i32 = 102;
pub const MADV_GUARD_REMOVE: i32 = 103;

fn is_valid_guard_vma(vma: *const VmAreaStruct, allow_locked: bool) -> bool {
    let mut disallowed = VM_SPECIAL | VM_HUGETLB;
    if !allow_locked {
        disallowed |= VM_LOCKED;
    }
    unsafe { (*vma).vm_flags & disallowed == 0 }
}

unsafe fn guard_pte_alloc(mm: &mut MmStruct, addr: u64) -> Result<*mut pte_t, i32> {
    const ENOMEM: i32 = -12;

    let pgd_base = mm.pgd as *mut pgd_t;
    if pgd_base.is_null() {
        return Err(ENOMEM);
    }
    let pgdp = unsafe { pgd_offset_pgd(pgd_base, addr) };
    let pudp = unsafe { pud_alloc(pgdp, addr, _PAGE_TABLE) }.ok_or(ENOMEM)?;
    let pmdp = unsafe { pmd_alloc(pudp, addr, _PAGE_TABLE) }.ok_or(ENOMEM)?;
    unsafe { pte_alloc(pmdp, addr, _PAGE_TABLE) }.ok_or(ENOMEM)
}

unsafe fn guard_pte_lookup(mm: &MmStruct, addr: u64) -> Option<*mut pte_t> {
    let pgd_base = mm.pgd as *mut pgd_t;
    if pgd_base.is_null() {
        return None;
    }
    let pgdp = unsafe { pgd_offset_pgd(pgd_base, addr) };
    if unsafe { pgd_none(*pgdp) } {
        return None;
    }
    let p4dp = unsafe { p4d_offset(pgdp, addr) };
    let pudp = unsafe { pud_offset(p4dp, addr) };
    if unsafe { pud_none(*pudp) || pud_huge(*pudp) } {
        return None;
    }
    let pmdp = unsafe { pmd_offset(pudp, addr) };
    if unsafe { pmd_none(*pmdp) || pmd_huge(*pmdp) } {
        return None;
    }
    Some(unsafe { pte_offset_kernel(pmdp, addr) })
}

unsafe fn madvise_guard_install(
    mm: &mut MmStruct,
    vma: *mut VmAreaStruct,
    start: u64,
    end: u64,
) -> Result<(), i32> {
    const EINVAL: i32 = -22;

    if !is_valid_guard_vma(vma, false) {
        return Err(EINVAL);
    }
    if crate::mm::fault::vma_is_anonymous(vma) {
        // Linux also sets VMA_MAYBE_GUARD so fork copies guard-marker PTEs.
        // Lupos does not model that VMA flag yet; preparing anon_vma preserves
        // the same fork-copy behavior for the anonymous path exercised here.
        unsafe { crate::mm::rmap::anon_vma_prepare(vma)? };
    }

    unsafe {
        unmap_page_range(mm, start, end);
    }

    let mm_ptr = mm as *mut MmStruct;
    let mut addr = start;
    while addr < end {
        let ptep = unsafe { guard_pte_alloc(mm, addr)? };
        unsafe {
            set_pte_at(
                mm_ptr.cast::<()>(),
                addr,
                ptep,
                make_pte_marker(PTE_MARKER_GUARD),
            );
        }
        flush_tlb_page(addr);
        addr += PAGE_SIZE;
    }
    Ok(())
}

unsafe fn madvise_guard_remove(
    mm: &mut MmStruct,
    vma: *mut VmAreaStruct,
    start: u64,
    end: u64,
) -> Result<(), i32> {
    const EINVAL: i32 = -22;

    if !is_valid_guard_vma(vma, true) {
        return Err(EINVAL);
    }

    let mm_ptr = mm as *mut MmStruct;
    let mut addr = start;
    while addr < end {
        if let Some(ptep) = unsafe { guard_pte_lookup(mm, addr) } {
            let pte = unsafe { ptep_get(ptep) };
            if is_guard_pte_marker(pte) {
                pte_clear(mm_ptr.cast::<()>(), addr, ptep);
                flush_tlb_page(addr);
            }
        }
        addr += PAGE_SIZE;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// madvise_vma_behavior — apply advice to one VMA sub-range
// ---------------------------------------------------------------------------

/// Apply `advice` to the sub-range `[start, end)` of `vma`.
///
/// Supported behaviors for M13:
/// - `MADV_DONTNEED` / `MADV_DONTNEED_LOCKED`: free pages in range so they
///   are faulted in fresh on next access.
/// - `MADV_DONTFORK`: set `VM_DONTCOPY` so this VMA is excluded from `fork`.
/// - `MADV_DOFORK`: clear `VM_DONTCOPY`.
/// - `MADV_POPULATE_READ`: pre-fault each page readable; error if VMA lacks
///   `VM_READ` (`EINVAL`) or a page cannot be allocated (`ENOMEM`).
/// - `MADV_POPULATE_WRITE`: pre-fault each page writable; error if VMA lacks
///   `VM_WRITE` (`EINVAL`).
/// - `MADV_GUARD_INSTALL` / `MADV_GUARD_REMOVE`: install/remove Linux PTE
///   guard markers; later access faults with `VM_FAULT_SIGSEGV`.
/// - `MADV_NORMAL`, `MADV_RANDOM`, `MADV_SEQUENTIAL`, `MADV_WILLNEED`:
///   readahead hints — accepted as no-ops (M16 adds readahead).
/// - All others: `EINVAL`.
///
/// Ref: Linux `mm/madvise.c` — `madvise_vma_behavior()` line 825
///
/// # Safety
/// `mm` must be exclusively accessible.  `vma` must be in `mm`'s tree.
pub unsafe fn madvise_vma_behavior(
    mm: &mut MmStruct,
    vma: *mut VmAreaStruct,
    start: u64,
    end: u64,
    advice: i32,
) -> Result<(), i32> {
    const EINVAL: i32 = -22;
    const ENOMEM: i32 = -12;

    match advice {
        // ── Readahead hints: no-op (M16 adds real readahead) ────────────────
        MADV_NORMAL | MADV_RANDOM | MADV_SEQUENTIAL | MADV_WILLNEED => Ok(()),

        // ── MADV_DONTNEED / MADV_DONTNEED_LOCKED: free pages ────────────────
        //
        // Ref: Linux `madvise_dontneed_free()` — `mm/madvise.c`
        MADV_DONTNEED | MADV_DONTNEED_LOCKED => {
            unsafe {
                unmap_page_range(mm, start, end);
            }
            Ok(())
        }

        // ── MADV_GUARD_INSTALL / MADV_GUARD_REMOVE: PTE guard markers ─────
        //
        // Ref: Linux `madvise_guard_install()` / `madvise_guard_remove()`.
        MADV_GUARD_INSTALL => unsafe { madvise_guard_install(mm, vma, start, end) },
        MADV_GUARD_REMOVE => unsafe { madvise_guard_remove(mm, vma, start, end) },

        // ── MADV_REMOVE: punch a hole in a shared file-backed range ────────
        //
        // Ref: Linux `madvise_remove()` — `mm/madvise.c`
        MADV_REMOVE => {
            let vm_file = unsafe { (*vma).vm_file };
            let vm_flags = unsafe { (*vma).vm_flags };
            if vm_file == 0 || vm_flags & crate::mm::vm_flags::VM_SHARED == 0 {
                return Err(EINVAL);
            }
            let file_offset =
                unsafe { ((*vma).vm_pgoff << PAGE_SHIFT) + (start - (*vma).vm_start) };
            unsafe {
                crate::fs::syscalls::zero_file_range_raw(vm_file, file_offset, end - start, true)
                    .map_err(|errno| -errno)?;
                unmap_page_range(mm, start, end);
            }
            Ok(())
        }

        // ── MADV_DONTFORK: set VM_DONTCOPY ──────────────────────────────────
        // MADV_HWPOISON: inject a synthetic poisoned page for file reads.
        //
        // Ref: Linux `madvise_inject_error()` - `mm/madvise.c`. Lupos records
        // poisoned memfd pages so hugetlbfs reads short-count at the same
        // boundary as Linux's page-cache path.
        MADV_HWPOISON => {
            let vm_file = unsafe { (*vma).vm_file };
            let file_is_clean = crate::mm::huge::take_file_mapping_clean(vm_file);
            let mut file_hwpoison_recorded = false;
            if vm_file != 0 {
                let file_offset =
                    unsafe { ((*vma).vm_pgoff << PAGE_SHIFT) + (start - (*vma).vm_start) };
                unsafe {
                    match crate::fs::syscalls::mark_file_hwpoison_raw(
                        vm_file,
                        file_offset,
                        end - start,
                    ) {
                        Ok(()) => file_hwpoison_recorded = true,
                        Err(errno) if errno == crate::include::uapi::errno::EINVAL => {}
                        Err(errno) => return Err(-errno),
                    }
                }
            }
            if file_hwpoison_recorded {
                return Ok(());
            }
            let hard_signal = vm_file == 0 || (!file_is_clean && !file_hwpoison_recorded);
            let mut addr = start;
            while addr < end {
                let pfn = addr >> PAGE_SHIFT;
                if hard_signal {
                    crate::mm::huge::record_hard_hwpoison_range(addr, addr + PAGE_SIZE, pfn);
                } else {
                    crate::mm::huge::record_soft_offline_range(addr, addr + PAGE_SIZE, pfn);
                }
                addr += PAGE_SIZE;
            }
            if hard_signal {
                unsafe {
                    unmap_page_range(mm, start, end);
                }
            }
            Ok(())
        }

        MADV_SOFT_OFFLINE => {
            if unsafe { (*vma).vm_flags & VM_HUGETLB != 0 } {
                crate::mm::huge::soft_offline_hugetlb_page()?;
                return Ok(());
            }
            let mut addr = start;
            while addr < end {
                let pfn = addr >> PAGE_SHIFT;
                crate::mm::huge::record_soft_offline_range(addr, addr + PAGE_SIZE, pfn);
                addr += PAGE_SIZE;
            }
            Ok(())
        }

        MADV_HUGEPAGE => {
            crate::mm::huge::clear_thp_split_range(start, end);
            unsafe {
                (*vma).vm_flags = ((*vma).vm_flags | VM_HUGEPAGE) & !VM_NOHUGEPAGE;
            }
            Ok(())
        }

        MADV_NOHUGEPAGE => {
            unsafe {
                (*vma).vm_flags = ((*vma).vm_flags | VM_NOHUGEPAGE) & !VM_HUGEPAGE;
            }
            Ok(())
        }

        MADV_DONTFORK => {
            unsafe {
                (*vma).vm_flags |= VM_DONTCOPY;
            }
            Ok(())
        }

        // ── MADV_DOFORK: clear VM_DONTCOPY ──────────────────────────────────
        MADV_DOFORK => {
            unsafe {
                (*vma).vm_flags &= !VM_DONTCOPY;
            }
            Ok(())
        }

        // ── MADV_POPULATE_READ: pre-fault every page for reading ─────────────
        //
        // Ref: Linux `madvise_populate()` — `mm/madvise.c`
        MADV_POPULATE_READ => {
            let vm_flags = unsafe { (*vma).vm_flags };
            if vm_flags & VM_READ == 0 {
                return Err(EINVAL);
            }
            // Trigger demand-page faults over the range.
            let mut addr = start;
            while addr < end {
                use crate::mm::fault::{FAULT_FLAG_USER, handle_mm_fault};
                let ret = handle_mm_fault(vma, addr, FAULT_FLAG_USER);
                if ret & crate::mm::fault::VM_FAULT_ERROR != 0 {
                    return Err(ENOMEM);
                }
                addr += PAGE_SIZE;
            }
            Ok(())
        }

        // ── MADV_POPULATE_WRITE: pre-fault every page for writing ────────────
        MADV_POPULATE_WRITE => {
            let vm_flags = unsafe { (*vma).vm_flags };
            if vm_flags & VM_WRITE == 0 {
                return Err(EINVAL);
            }
            let mut addr = start;
            while addr < end {
                use crate::mm::fault::{FAULT_FLAG_USER, FAULT_FLAG_WRITE, handle_mm_fault};
                let ret = handle_mm_fault(vma, addr, FAULT_FLAG_USER | FAULT_FLAG_WRITE);
                if ret & crate::mm::fault::VM_FAULT_ERROR != 0 {
                    return Err(ENOMEM);
                }
                addr += PAGE_SIZE;
            }
            Ok(())
        }

        // ── All others: EINVAL ───────────────────────────────────────────────
        _ => Err(EINVAL),
    }
}

// ---------------------------------------------------------------------------
// do_madvise — core madvise handler
// ---------------------------------------------------------------------------

/// Core madvise handler.
///
/// ## Error codes (matching Linux)
/// - `-EINVAL` (-22): `start` not page-aligned; `len` overflow; unknown advice.
/// - `-ENOMEM` (-12): range contains a hole (no VMA) for `MADV_POPULATE_*`.
///
/// Ref: Linux `mm/madvise.c` — `do_madvise()` line 1345
///
/// # Safety
/// `mm` must be exclusively accessible (mmap_lock held for write).
pub unsafe fn do_madvise(mm: &mut MmStruct, start: u64, len: u64, advice: i32) -> Result<(), i32> {
    const EINVAL: i32 = -22;
    const ENOMEM: i32 = -12;

    // 1. start must be page-aligned.
    if start & !PAGE_MASK != 0 {
        return Err(EINVAL);
    }

    // 2. Zero-length is a no-op.
    if len == 0 {
        return Ok(());
    }

    // 3. Align end; check for overflow.
    let end = start
        .checked_add(len)
        .map(|e| (e + PAGE_SIZE - 1) & PAGE_MASK)
        .ok_or(EINVAL)?;
    if end <= start {
        return Err(EINVAL);
    }

    // 4. Walk VMAs covering [start, end).
    let mut cur = start;
    while cur < end {
        let vma_ptr = find_vma(mm, cur).ok_or(ENOMEM)?;
        let vma = unsafe { &*vma_ptr };

        if vma.vm_start > cur {
            // Gap — semantics differ by advice:
            // POPULATE_* must error; others skip the gap.
            match advice {
                MADV_POPULATE_READ | MADV_POPULATE_WRITE => return Err(ENOMEM),
                _ => {
                    cur = vma.vm_start;
                    continue;
                }
            }
        }

        let seg_end = vma.vm_end.min(end);
        let mut vma_ptr = vma_ptr;
        if matches!(advice, MADV_HUGEPAGE | MADV_NOHUGEPAGE) {
            if cur > unsafe { (*vma_ptr).vm_start } {
                vma_ptr = unsafe { vma_split(mm, vma_ptr, cur)? };
            }
            if seg_end < unsafe { (*vma_ptr).vm_end } {
                let _ = unsafe { vma_split(mm, vma_ptr, seg_end)? };
            }
        }
        unsafe { madvise_vma_behavior(mm, vma_ptr, cur, seg_end, advice)? };
        cur = seg_end;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests — ported from vendor/linux/tools/testing/selftests/mm/madv_populate.c
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::arch::x86::mm::paging;
    use crate::mm::fault::{FAULT_FLAG_USER, VM_FAULT_SIGSEGV, handle_mm_fault};
    use crate::mm::mm_types::MmStruct;
    use crate::mm::mmap::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, do_mmap};
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;
    use crate::mm::vma::find_vma;

    fn make_mm() -> MmStruct {
        MmStruct::new(0)
    }

    fn make_paged_mm() -> MmStruct {
        MmStruct::new(paging::init_pgd_for_test() as usize)
    }

    unsafe fn pte_for(mm: &MmStruct, addr: u64) -> paging::pte_t {
        let ptep = unsafe { guard_pte_lookup(mm, addr).expect("PTE table must exist") };
        unsafe { paging::ptep_get(ptep) }
    }

    // ── Test 1 ─────────────────────────────────────────────────────────────────
    // MADV_DONTNEED on an anonymous VMA must succeed (no-op in unit tests since
    // pgd is null and unmap_page_range guards against it).
    #[test]
    fn madvise_dontneed_ok_on_anonymous() {
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

        let r = unsafe { do_madvise(&mut mm, 0x10000, 0x10000, MADV_DONTNEED) };
        assert!(r.is_ok(), "MADV_DONTNEED must succeed on anonymous VMA");
    }

    // ── Test 2 ─────────────────────────────────────────────────────────────────
    // Port of: madv_populate.c — MADV_POPULATE_WRITE on a PROT_READ-only VMA
    // must return EINVAL.
    #[test]
    fn madvise_populate_write_requires_write_vma() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ, /* no PROT_WRITE */
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        let r = unsafe { do_madvise(&mut mm, 0x10000, 0x10000, MADV_POPULATE_WRITE) };
        assert_eq!(r, Err(-22)); // EINVAL
    }

    // ── Test 3 ─────────────────────────────────────────────────────────────────
    // Port of: madv_populate.c — MADV_POPULATE_READ/WRITE on a range that
    // contains a hole (no VMA) must return ENOMEM.
    #[test]
    fn madvise_populate_read_on_hole_returns_enomem() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        // No VMA at all — the entire range is a hole.
        let r = unsafe { do_madvise(&mut mm, 0x10000, 0x10000, MADV_POPULATE_READ) };
        assert_eq!(r, Err(-12)); // ENOMEM
    }

    // ── Test 4 ─────────────────────────────────────────────────────────────────
    // Unknown advice value must return EINVAL.
    #[test]
    fn madvise_unknown_advice_returns_einval() {
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

        let r = unsafe { do_madvise(&mut mm, 0x10000, 0x10000, 9999) };
        assert_eq!(r, Err(-22)); // EINVAL
    }

    #[test]
    fn madvise_guard_install_sets_marker_that_faults_sigsegv() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_paged_mm();

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

        let r = unsafe { do_madvise(&mut mm, 0x10000, 0x1000, MADV_GUARD_INSTALL) };
        assert_eq!(r, Ok(()));

        let pte = unsafe { pte_for(&mm, 0x10000) };
        assert!(crate::mm::swap::is_guard_pte_marker(pte));

        let vma = find_vma(&mm, 0x10000).unwrap();
        assert_eq!(
            handle_mm_fault(vma, 0x10000, FAULT_FLAG_USER),
            VM_FAULT_SIGSEGV
        );
    }

    #[test]
    fn madvise_guard_remove_clears_only_guard_marker() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_paged_mm();

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

        unsafe { do_madvise(&mut mm, 0x10000, 0x1000, MADV_GUARD_INSTALL) }.unwrap();
        unsafe { do_madvise(&mut mm, 0x10000, 0x1000, MADV_GUARD_REMOVE) }.unwrap();

        let pte = unsafe { pte_for(&mm, 0x10000) };
        assert!(paging::pte_none(pte), "guard remove must clear the marker");
    }

    #[test]
    fn madvise_guard_install_rejects_locked_vma_like_linux() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_paged_mm();

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

        let vma = find_vma(&mm, 0x10000).unwrap();
        unsafe {
            (*vma).vm_flags |= VM_LOCKED;
        }

        let r = unsafe { do_madvise(&mut mm, 0x10000, 0x1000, MADV_GUARD_INSTALL) };
        assert_eq!(r, Err(-22));
    }
}
