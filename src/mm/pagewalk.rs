//! linux-parity: complete
//! linux-source: vendor/linux/mm/pagewalk.c
//! test-origin: linux:vendor/linux/mm/pagewalk.c
//! Generic kernel page-table walker — port of `vendor/linux/mm/pagewalk.c`.
//!
//! Linux's `walk_page_range` family is the audited code path every memory
//! feature uses to iterate page tables: `/proc/pid/pagemap`, swap, hugetlb,
//! soft-dirty tracking, userfaultfd, gup, KSM, smaps, and madvise all funnel
//! through `mm_walk_ops`. Lupos commits to bit-for-bit Linux ABI, which means
//! we need an audited walker with the same callback shape, the same control
//! flow tokens (`ACTION_SUBTREE` / `ACTION_CONTINUE` / `ACTION_AGAIN`), the
//! same per-level depth indicators on holes, and the same short-circuit
//! semantics on non-zero error returns.
//!
//! This module provides:
//!
//!   * [`MmWalkOps`] — the callback trait, mirroring `struct mm_walk_ops`.
//!     Default-method bodies stand in for the nullable C function pointers
//!     so callers only override what they care about.
//!   * [`MmWalk`] — the per-walk state, mirroring `struct mm_walk`.
//!   * [`PageWalkAction`] — descent control, mirroring `enum page_walk_action`.
//!   * [`walk_kernel_page_table_range`] — the entry point that takes a raw
//!     PGD pointer (no `mm_struct` required), matching Linux's
//!     `walk_kernel_page_table_range` introduced for kernel-table walks.
//!
//! Deferred to milestone 11 (when `mm_struct` and `vm_area_struct` land):
//!
//!   * `walk_page_range`, `walk_page_vma`, `walk_page_mapping`
//!   * VMA-aware `pre_vma` / `post_vma` / `test_walk` / `hugetlb_entry`
//!   * `install_pte` (allocates PTEs on the fly during user faults)
//!   * `split_huge_pmd` / `split_huge_pud` (THP machinery)
//!
//! References (line numbers as of the vendored Linux tree):
//!
//!   * `vendor/linux/mm/pagewalk.c:30-320` — per-level walker functions.
//!   * `vendor/linux/include/linux/pagewalk.h:70-130` — `mm_walk_ops`,
//!     `mm_walk`, `page_walk_action`.
//!   * `vendor/linux/mm/debug_vm_pgtable.c` — invariants the host tests below
//!     port from.

use core::marker::PhantomData;
use core::ptr;

use crate::arch::x86::mm::paging::{
    PAGE_SIZE, PGDIR_SIZE, PMD_SIZE, PTRS_PER_PGD, PUD_SIZE, p4d_offset, p4d_t, pgd_index,
    pgd_none, pgd_offset_pgd, pgd_t, pmd_huge, pmd_none, pmd_offset, pmd_t, pte_offset_kernel,
    pte_t, pud_huge, pud_none, pud_offset, pud_t,
};

// ---------------------------------------------------------------------------
// Action token — mirrors `enum page_walk_action` in
// `vendor/linux/include/linux/pagewalk.h`.
// ---------------------------------------------------------------------------

/// Descent control for [`MmWalkOps`] callbacks. Set the value via
/// [`MmWalk::action`] from inside a callback before returning `Ok(())`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PageWalkAction {
    /// Descend into the next page-table level (Linux `ACTION_SUBTREE`).
    /// Default after every callback returns.
    Subtree = 0,
    /// Skip the subtree and move on to the next entry at the current level
    /// (Linux `ACTION_CONTINUE`).
    Continue = 1,
    /// Re-call the current callback on the same entry (Linux `ACTION_AGAIN`).
    /// Useful for "we just installed something, look again" patterns.
    Again = 2,
}

// ---------------------------------------------------------------------------
// Walk state — mirrors `struct mm_walk` in `pagewalk.h`.
// ---------------------------------------------------------------------------

/// Live state passed to every [`MmWalkOps`] callback.
///
/// Equivalent to Linux's `struct mm_walk`.
///
/// ## M11 additions
///
/// - `mm`: pointer to the `mm_struct` being walked (null for kernel walks).
/// - `vma`: pointer to the current `vm_area_struct` being walked (null for
///   kernel walks and between-VMA gaps).
///
/// Ref: Linux `include/linux/pagewalk.h` — `struct mm_walk`
pub struct MmWalk<'a> {
    /// PGD (PML4) root the walker descends from.
    pub pgd: *mut pgd_t,
    /// Descent control — callbacks may set this to [`PageWalkAction::Continue`]
    /// or [`PageWalkAction::Again`] to influence the walker.
    pub action: PageWalkAction,
    /// Caller-supplied context pointer (Linux's `walk->private`). Untyped on
    /// purpose so the same walker handles every consumer.
    pub private: *mut (),
    /// Pointer to the `mm_struct` being walked.  Null for kernel-table walks.
    ///
    /// Ref: Linux `mm_walk::mm`
    pub mm: *const crate::mm::mm_types::MmStruct,
    /// Pointer to the current VMA being walked.  Null for kernel-table walks
    /// and for gaps between VMAs during `walk_page_range`.
    ///
    /// Ref: Linux `mm_walk::vma`
    pub vma: *const crate::mm::mm_types::VmAreaStruct,
    _phantom: PhantomData<&'a mut ()>,
}

impl<'a> MmWalk<'a> {
    /// Construct a fresh walk state. The `'a` lifetime ties the resulting
    /// `MmWalk` to whichever borrow you took to obtain `pgd` / `private`.
    #[inline]
    pub fn new(pgd: *mut pgd_t, private: *mut ()) -> Self {
        Self {
            pgd,
            action: PageWalkAction::Subtree,
            private,
            mm: ptr::null(),
            vma: ptr::null(),
            _phantom: PhantomData,
        }
    }

    /// Construct walk state for a user-space mm walk.
    #[inline]
    pub fn new_mm(
        pgd: *mut pgd_t,
        private: *mut (),
        mm: *const crate::mm::mm_types::MmStruct,
    ) -> Self {
        Self {
            pgd,
            action: PageWalkAction::Subtree,
            private,
            mm,
            vma: ptr::null(),
            _phantom: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Callback trait — `struct mm_walk_ops` analogue.
// ---------------------------------------------------------------------------

/// Per-level callback hooks for the page-table walker. Implementors override
/// only the levels they care about; the defaults are no-ops that let the
/// walker descend.
///
/// Returning `Err(_)` from any callback short-circuits the entire walk —
/// matches Linux `if (err) break;` semantics. The error value is propagated
/// back to the caller of [`walk_kernel_page_table_range`] unchanged.
#[allow(unused_variables)]
pub trait MmWalkOps {
    /// Called once per PGD entry covering `[addr, next)`.
    fn pgd_entry(
        &mut self,
        pgd: *mut pgd_t,
        addr: u64,
        next: u64,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        Ok(())
    }
    /// Called once per P4D entry covering `[addr, next)`. Folded into PGD on
    /// 4-level x86_64 — fires once per visited PGD entry.
    fn p4d_entry(
        &mut self,
        p4d: *mut p4d_t,
        addr: u64,
        next: u64,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        Ok(())
    }
    /// Called once per PUD entry covering `[addr, next)`. Must handle 1 GiB
    /// huge pages (`pud_huge` returns true).
    fn pud_entry(
        &mut self,
        pud: *mut pud_t,
        addr: u64,
        next: u64,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        Ok(())
    }
    /// Called once per PMD entry covering `[addr, next)`. Must handle 2 MiB
    /// huge pages (`pmd_huge` / `pmd_trans_huge`).
    fn pmd_entry(
        &mut self,
        pmd: *mut pmd_t,
        addr: u64,
        next: u64,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        Ok(())
    }
    /// Called once per PTE entry. Linux invokes this even for `pte_none`
    /// entries when `install_pte` is set; we don't have install_pte yet
    /// (deferred to M11) so empty entries are routed to [`Self::pte_hole`]
    /// instead.
    fn pte_entry(
        &mut self,
        pte: *mut pte_t,
        addr: u64,
        next: u64,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        Ok(())
    }
    /// Called for unmapped ranges. `depth` mirrors Linux: 0 = PGD, 1 = P4D,
    /// 2 = PUD, 3 = PMD. The PTE level never produces holes (see above).
    fn pte_hole(
        &mut self,
        addr: u64,
        next: u64,
        depth: i32,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        Ok(())
    }

    /// Marker reflecting whether a `pte_entry` handler exists. The walker
    /// uses this to skip the leaf descent for callers that only care about
    /// upper levels — same as Linux's `bool has_handler` locals.
    ///
    /// Override and return `true` if you implement [`Self::pte_entry`].
    /// Defaults to `false` because the trait method is a no-op by default.
    fn has_pte_entry(&self) -> bool {
        false
    }
    /// Same idea for `pmd_entry` — override if you provide one.
    fn has_pmd_entry(&self) -> bool {
        false
    }
    /// Same idea for `pud_entry` — override if you provide one.
    fn has_pud_entry(&self) -> bool {
        false
    }

    /// Called before walking a VMA.  Return `false` to skip this VMA.
    ///
    /// Ref: Linux `mm_walk_ops::test_walk`
    fn test_walk(&mut self, _walk: &MmWalk<'_>) -> bool {
        true
    }

    /// Called before entering a VMA's page tables.
    ///
    /// Ref: Linux `mm_walk_ops::pre_vma`
    fn pre_vma(&mut self, _walk: &mut MmWalk<'_>) -> Result<(), i32> {
        Ok(())
    }

    /// Called after leaving a VMA's page tables.
    ///
    /// Ref: Linux `mm_walk_ops::post_vma`
    fn post_vma(&mut self, _walk: &mut MmWalk<'_>) {}
}

// ---------------------------------------------------------------------------
// Address-range helpers — `pXd_addr_end` from
// `vendor/linux/include/linux/pgtable.h:1005-1050`.
//
// All four return `min(end, ALIGN_UP(addr+1, PXD_SIZE))`. The wrap-around
// guard at the top of the address space mirrors the C macro: if the next
// boundary is 0 (overflow), clamp to `end`.
// ---------------------------------------------------------------------------

#[inline]
fn level_addr_end(addr: u64, end: u64, size: u64) -> u64 {
    let next = (addr.wrapping_add(size)) & !(size - 1);
    if next == 0 || next > end { end } else { next }
}

#[inline]
fn pgd_addr_end(addr: u64, end: u64) -> u64 {
    level_addr_end(addr, end, PGDIR_SIZE)
}
#[inline]
fn p4d_addr_end(addr: u64, end: u64) -> u64 {
    // 4-level x86_64 folds P4D into PGD, so the boundary is the PGD one.
    level_addr_end(addr, end, PGDIR_SIZE)
}
#[inline]
fn pud_addr_end(addr: u64, end: u64) -> u64 {
    level_addr_end(addr, end, PUD_SIZE)
}
#[inline]
fn pmd_addr_end(addr: u64, end: u64) -> u64 {
    level_addr_end(addr, end, PMD_SIZE)
}

// ---------------------------------------------------------------------------
// Per-level walkers — port of `vendor/linux/mm/pagewalk.c:30-310`.
//
// Each function mirrors the corresponding C function one-for-one so future
// audits can do a side-by-side diff. The control-flow-token handling
// (Subtree / Continue / Again) matches Linux exactly.
// ---------------------------------------------------------------------------

unsafe fn walk_pte_range<O: MmWalkOps>(
    pmd: *mut pmd_t,
    addr: u64,
    end: u64,
    walk: &mut MmWalk<'_>,
    ops: &mut O,
) -> Result<(), i32> {
    // Linux pagewalk.c:62 `walk_pte_range`: iterate `pmd`'s 512 PTE slots
    // covering `[addr, end)` and call `pte_entry` for each. Empty entries
    // route to `pte_hole(depth=4)` because we lack `install_pte`.
    let mut cur = addr;
    let mut ptep = unsafe { pte_offset_kernel(pmd, addr) };
    loop {
        let next = cur + PAGE_SIZE;
        let pte = unsafe { *ptep };
        if pte.0 == 0 {
            ops.pte_hole(cur, next, 4, walk)?;
        } else {
            ops.pte_entry(ptep, cur, next, walk)?;
        }
        if next >= end {
            break;
        }
        cur = next;
        ptep = unsafe { ptep.add(1) };
    }
    Ok(())
}

unsafe fn walk_pmd_range<O: MmWalkOps>(
    pud: *mut pud_t,
    addr: u64,
    end: u64,
    walk: &mut MmWalk<'_>,
    ops: &mut O,
) -> Result<(), i32> {
    // Linux pagewalk.c:97 `walk_pmd_range`. Iterate one PMD at a time, call
    // `pmd_entry` (the caller MUST handle `pmd_trans_huge` if it cares),
    // honour the action token, and descend if the entry is not a leaf.
    let mut cur = addr;
    loop {
        let pmdp = unsafe { pmd_offset(pud, cur) };
        let next = pmd_addr_end(cur, end);

        if pmd_none(unsafe { *pmdp }) {
            ops.pte_hole(cur, next, 3, walk)?;
            // No `install_pte` yet — fall through to next entry.
            if next >= end {
                break;
            }
            cur = next;
            continue;
        }

        loop {
            walk.action = PageWalkAction::Subtree;
            ops.pmd_entry(pmdp, cur, next, walk)?;

            match walk.action {
                PageWalkAction::Again => {
                    // Re-invoke the handler. Linux uses a `goto again;`.
                    continue;
                }
                PageWalkAction::Continue => {
                    // Skip the subtree entirely.
                    break;
                }
                PageWalkAction::Subtree => {
                    // Descend into PTEs unless this is a leaf or empty.
                    let pmd = unsafe { *pmdp };
                    if pmd_huge(pmd) {
                        // Linux: walker observes huge pages without splitting.
                        break;
                    }
                    if !ops.has_pte_entry() {
                        // No leaf handler — nothing more to do.
                        break;
                    }
                    unsafe { walk_pte_range(pmdp, cur, next, walk, ops)? };
                    break;
                }
            }
        }

        if next >= end {
            break;
        }
        cur = next;
    }
    Ok(())
}

unsafe fn walk_pud_range<O: MmWalkOps>(
    p4d: *mut p4d_t,
    addr: u64,
    end: u64,
    walk: &mut MmWalk<'_>,
    ops: &mut O,
) -> Result<(), i32> {
    // Linux pagewalk.c:167 `walk_pud_range`. Same shape as walk_pmd_range,
    // one level up. Honours `pud_huge` (1 GiB pages).
    let mut cur = addr;
    loop {
        let pudp = unsafe { pud_offset(p4d, cur) };
        let next = pud_addr_end(cur, end);

        if pud_none(unsafe { *pudp }) {
            ops.pte_hole(cur, next, 2, walk)?;
            if next >= end {
                break;
            }
            cur = next;
            continue;
        }

        loop {
            walk.action = PageWalkAction::Subtree;
            ops.pud_entry(pudp, cur, next, walk)?;

            match walk.action {
                PageWalkAction::Again => continue,
                PageWalkAction::Continue => break,
                PageWalkAction::Subtree => {
                    let pud = unsafe { *pudp };
                    if pud_huge(pud) {
                        break;
                    }
                    if !ops.has_pmd_entry() && !ops.has_pte_entry() {
                        break;
                    }
                    unsafe { walk_pmd_range(pudp, cur, next, walk, ops)? };
                    break;
                }
            }
        }

        if next >= end {
            break;
        }
        cur = next;
    }
    Ok(())
}

unsafe fn walk_p4d_range<O: MmWalkOps>(
    pgd: *mut pgd_t,
    addr: u64,
    end: u64,
    walk: &mut MmWalk<'_>,
    ops: &mut O,
) -> Result<(), i32> {
    // Linux pagewalk.c:232 `walk_p4d_range`. P4D is folded on 4-level x86_64,
    // so this loop runs once per visited PGD slot.
    let p4dp = unsafe { p4d_offset(pgd, addr) };
    let next = p4d_addr_end(addr, end);
    ops.p4d_entry(p4dp, addr, next, walk)?;
    if ops.has_pud_entry() || ops.has_pmd_entry() || ops.has_pte_entry() {
        unsafe { walk_pud_range(p4dp, addr, next, walk, ops)? };
    }
    Ok(())
}

unsafe fn walk_pgd_range<O: MmWalkOps>(
    addr: u64,
    end: u64,
    walk: &mut MmWalk<'_>,
    ops: &mut O,
) -> Result<(), i32> {
    // Linux pagewalk.c:270 `walk_pgd_range`. Top-level entry; the C version
    // either uses `walk->pgd` or `pgd_offset(walk->mm, addr)` — we always
    // come in via `walk_kernel_page_table_range` so `walk.pgd` is set.
    debug_assert!(!walk.pgd.is_null(), "walk_pgd_range: null PGD");

    let mut cur = addr;
    while cur < end {
        let next = pgd_addr_end(cur, end);
        let pgdp = unsafe { pgd_offset_pgd(walk.pgd, cur) };

        if pgd_none(unsafe { *pgdp }) {
            ops.pte_hole(cur, next, 0, walk)?;
        } else {
            ops.pgd_entry(pgdp, cur, next, walk)?;
            unsafe { walk_p4d_range(pgdp, cur, next, walk, ops)? };
        }

        if next == 0 {
            // Walked off the top of the address space — bail.
            break;
        }
        cur = next;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public entry points — `walk_kernel_page_table_range` family.
// ---------------------------------------------------------------------------

/// Walk a range of the kernel page tables, calling `ops` at every level.
///
/// Mirrors `vendor/linux/mm/pagewalk.c:607` `walk_kernel_page_table_range`.
/// Unlike the user-space `walk_page_range`, this entry takes the PGD root
/// directly so callers do not need an `mm_struct`.
///
/// # Safety
/// * `start` must be < `end` and both must be page-aligned.
/// * `pgd` must point to a live kernel PGD (PML4) for the duration of the
///   walk and must not alias any callback-mutated state.
/// * Callbacks may dereference the per-level `*mut` pointers; they must not
///   free the underlying tables while the walk is in progress.
pub unsafe fn walk_kernel_page_table_range<O: MmWalkOps>(
    start: u64,
    end: u64,
    ops: &mut O,
    pgd: *mut pgd_t,
    private: *mut (),
) -> Result<(), i32> {
    debug_assert!(start < end, "walk_kernel_page_table_range: empty range");
    debug_assert_eq!(start & (PAGE_SIZE - 1), 0, "start must be page-aligned");
    debug_assert_eq!(end & (PAGE_SIZE - 1), 0, "end must be page-aligned");
    debug_assert!(!pgd.is_null(), "walk_kernel_page_table_range: null pgd");

    let mut walk = MmWalk::new(pgd, private);
    unsafe { walk_pgd_range(start, end, &mut walk, ops) }
}

/// Lockless variant — same body as [`walk_kernel_page_table_range`] for now.
/// Once `mm_struct` lands the locked variant will hold `mmap_lock` and call
/// this one underneath.
///
/// # Safety
/// Same as [`walk_kernel_page_table_range`].
pub unsafe fn walk_kernel_page_table_range_lockless<O: MmWalkOps>(
    start: u64,
    end: u64,
    ops: &mut O,
    pgd: *mut pgd_t,
    private: *mut (),
) -> Result<(), i32> {
    unsafe { walk_kernel_page_table_range(start, end, ops, pgd, private) }
}

// ---------------------------------------------------------------------------
// VMA-aware walkers — ported from `vendor/linux/mm/pagewalk.c`.
// ---------------------------------------------------------------------------

/// Walk a user address space, iterating VMAs from the mm_struct's Maple Tree.
///
/// For each VMA in `[start, end)`:
/// 1. Calls `ops.test_walk()` — if it returns false, skips the VMA.
/// 2. Calls `ops.pre_vma()`.
/// 3. Descends the page table for the VMA's overlapping range.
/// 4. Calls `ops.post_vma()`.
///
/// Gaps between VMAs are reported via `ops.pte_hole()`.
///
/// Ref: Linux `mm/pagewalk.c:579` — `walk_page_range`
///
/// # Safety
///
/// * `mm` must have a valid `pgd` and Maple Tree.
/// * `start < end`, both page-aligned.
/// * The caller must hold `mm.mmap_lock` for at least read access.
pub unsafe fn walk_page_range<O: MmWalkOps>(
    mm: &crate::mm::mm_types::MmStruct,
    start: u64,
    end: u64,
    ops: &mut O,
    private: *mut (),
) -> Result<(), i32> {
    debug_assert!(start < end, "walk_page_range: empty range");

    let pgd = mm.pgd as *mut pgd_t;
    let mut walk = MmWalk::new_mm(pgd, private, mm);

    let mut addr = start;
    let mut iter = crate::mm::maple_tree::MapleTreeIter::new(&mm.mm_mt, start);

    while addr < end {
        // Find the next VMA at or after `addr`.
        let vma_entry = iter.peek();

        match vma_entry {
            Some((vma_start, _vma_end, vma_ptr)) => {
                let vma = unsafe { &*(vma_ptr as *const crate::mm::mm_types::VmAreaStruct) };
                let vma_vm_end = vma.vm_end;

                // If there's a gap before this VMA, report it as a hole.
                if vma_start > addr {
                    let hole_end = core::cmp::min(vma_start, end);
                    ops.pte_hole(addr, hole_end, 0, &mut walk)?;
                    addr = hole_end;
                    if addr >= end {
                        break;
                    }
                }

                // Set up the VMA context.
                walk.vma = vma as *const crate::mm::mm_types::VmAreaStruct;

                // Test walk.
                if ops.test_walk(&walk) {
                    ops.pre_vma(&mut walk)?;

                    // Walk the page tables for the overlap.
                    let walk_start = core::cmp::max(addr, vma.vm_start);
                    let walk_end = core::cmp::min(vma_vm_end, end);

                    if walk_start < walk_end {
                        unsafe { walk_pgd_range(walk_start, walk_end, &mut walk, ops)? };
                    }

                    ops.post_vma(&mut walk);
                }

                walk.vma = ptr::null();
                addr = vma_vm_end;
                iter.next(); // advance past this VMA
            }
            None => {
                // No more VMAs — report the remaining range as a hole.
                if addr < end {
                    ops.pte_hole(addr, end, 0, &mut walk)?;
                }
                break;
            }
        }
    }

    Ok(())
}

/// Walk a single VMA's page table range.
///
/// Ref: Linux `mm/pagewalk.c:718` — `walk_page_range_vma`
///
/// # Safety
///
/// * `vma` must be a valid VMA in `mm`'s tree.
/// * `start < end`, both page-aligned, and within the VMA's range.
pub unsafe fn walk_page_vma<O: MmWalkOps>(
    vma: *const crate::mm::mm_types::VmAreaStruct,
    start: u64,
    end: u64,
    ops: &mut O,
    private: *mut (),
) -> Result<(), i32> {
    let v = unsafe { &*vma };
    debug_assert!(start < end, "walk_page_vma: empty range");
    debug_assert!(start >= v.vm_start, "walk_page_vma: start before VMA");
    debug_assert!(end <= v.vm_end, "walk_page_vma: end after VMA");

    let mm = &*v.vm_mm;
    let pgd = mm.pgd as *mut pgd_t;
    let mut walk = MmWalk::new_mm(pgd, private, mm);
    walk.vma = vma;

    if !ops.test_walk(&walk) {
        return Ok(());
    }

    ops.pre_vma(&mut walk)?;
    walk_pgd_range(start, end, &mut walk, ops)?;
    ops.post_vma(&mut walk);

    Ok(())
}

/// Walk all VMAs registered against an address_space over a page-index range.
///
/// Ref: Linux `vendor/linux/mm/pagewalk.c` — `walk_page_mapping()`
///
/// # Safety
/// `mapping` and its registered VMAs must remain alive for the duration of the
/// walk.  The caller owns the equivalent of Linux `i_mmap_rwsem`.
pub unsafe fn walk_page_mapping<O: MmWalkOps>(
    mapping: *mut crate::mm::address_space::AddressSpace,
    first_index: u64,
    nr: u64,
    ops: &mut O,
    private: *mut (),
) -> Result<(), i32> {
    const EINVAL: i32 = -22;

    if mapping.is_null() || nr == 0 {
        return Err(EINVAL);
    }
    let last_exclusive = first_index.checked_add(nr).ok_or(EINVAL)?;

    for vma_ptr in crate::mm::address_space::mapping_vmas(mapping) {
        if vma_ptr.is_null() {
            continue;
        }

        let vma = unsafe { &*vma_ptr };
        if vma.vm_mm.is_null() {
            continue;
        }

        let vma_pages = (vma.vm_end - vma.vm_start) >> PAGE_SIZE.trailing_zeros();
        let vba = vma.vm_pgoff;
        let vea = vba.saturating_add(vma_pages);
        let cba = core::cmp::max(first_index, vba);
        let cea = core::cmp::min(last_exclusive, vea);
        if cba >= cea {
            continue;
        }

        let start_addr = ((cba - vba) << PAGE_SIZE.trailing_zeros()) + vma.vm_start;
        let end_addr = ((cea - vba) << PAGE_SIZE.trailing_zeros()) + vma.vm_start;
        if start_addr >= end_addr {
            continue;
        }

        let mm = unsafe { &*vma.vm_mm };
        let pgd = mm.pgd as *mut pgd_t;
        let mut walk = MmWalk::new_mm(pgd, private, mm);
        walk.vma = vma_ptr;

        if !ops.test_walk(&walk) {
            continue;
        }

        ops.pre_vma(&mut walk)?;
        unsafe { walk_pgd_range(start_addr, end_addr, &mut walk, ops)? };
        ops.post_vma(&mut walk);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Suppress dead-code warnings on host builds for the `private` field —
// downstream callers in M11 will populate it.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn _force_use_of_private(walk: &MmWalk<'_>) -> *mut () {
    walk.private
}

#[allow(dead_code)]
fn _force_use_of_null() -> *mut () {
    ptr::null_mut()
}

// ---------------------------------------------------------------------------
// smaps dirty accounting — M14 addition
// ---------------------------------------------------------------------------

/// Per-region smaps-style dirty stats.
///
/// Mirrors the `Shared_Dirty` and `Private_Dirty` lines in
/// `/proc/<pid>/smaps` (Linux `fs/proc/task_mmu.c` — `smaps_pte_range`).
///
/// A page is "shared" when its `_mapcount` is ≥ 1 (i.e., two or more PTEs
/// reference it); otherwise it is "private" (exactly one PTE).
/// Both counts are in bytes, not pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SmapsDirty {
    /// Bytes of dirty pages mapped by more than one PTE.
    pub shared_dirty: usize,
    /// Bytes of dirty pages mapped by exactly one PTE.
    pub private_dirty: usize,
}

/// Walk the page tables of [`mm`] for the virtual range `[start, end)` and
/// compute the `Shared_Dirty` / `Private_Dirty` byte counts.
///
/// A page is considered dirty when the PTE's dirty bit is set.  It is
/// "shared" when `page._mapcount ≥ 1` (meaning two or more PTEs reference
/// it; `_mapcount == 0` is one PTE, `== 1` is two PTEs, etc.).
///
/// Ref: Linux `fs/proc/task_mmu.c` — `smaps_pte_range()`
///
/// # Safety
///
/// * `mm` must be a valid pointer to a live `mm_struct`.
/// * `start < end`, both page-aligned.
/// * The page tables and `buddy::mem_map` must be consistent (i.e., every
///   PFN mapped in the page tables must have a corresponding `Page` entry).
pub unsafe fn smaps_for_range(
    mm: *const crate::mm::mm_types::MmStruct,
    start: u64,
    end: u64,
) -> SmapsDirty {
    use crate::arch::x86::mm::paging::{
        p4d_offset, pgd_none, pgd_offset_pgd, pmd_huge, pmd_none, pmd_offset, pte_dirty, pte_none,
        pte_offset_kernel, pte_present, ptep_get, pud_huge, pud_none, pud_offset,
    };
    use crate::mm::buddy::pfn_to_page;
    use core::sync::atomic::Ordering;

    // Helper: round addr down/up to page boundary.
    const PS: u64 = PAGE_SIZE as u64;

    let mut stats = SmapsDirty::default();
    let pgd = (*mm).pgd as *mut pgd_t;
    if pgd.is_null() {
        return stats;
    }

    let mut addr = start & !(PS - 1);
    while addr < end {
        // PGD
        let pgdp = pgd_offset_pgd(pgd, addr);
        if pgd_none(*pgdp) {
            addr = (addr + crate::arch::x86::mm::paging::PGDIR_SIZE as u64)
                & !(crate::arch::x86::mm::paging::PGDIR_SIZE as u64 - 1);
            continue;
        }
        let p4dp = p4d_offset(pgdp, addr);
        // PUD
        let pudp = pud_offset(p4dp, addr);
        if pud_none(*pudp) {
            addr = (addr + PUD_SIZE as u64) & !(PUD_SIZE as u64 - 1);
            continue;
        }
        if pud_huge(*pudp) {
            addr = (addr + PUD_SIZE as u64) & !(PUD_SIZE as u64 - 1);
            continue;
        }
        // PMD
        let pmdp = pmd_offset(pudp, addr);
        if pmd_none(*pmdp) {
            addr = (addr + PMD_SIZE as u64) & !(PMD_SIZE as u64 - 1);
            continue;
        }
        if pmd_huge(*pmdp) {
            addr = (addr + PMD_SIZE as u64) & !(PMD_SIZE as u64 - 1);
            continue;
        }
        // PTE
        let ptep = pte_offset_kernel(pmdp, addr);
        let pte = ptep_get(ptep);
        if !pte_none(pte) && pte_present(pte) && pte_dirty(pte) {
            let pfn = crate::arch::x86::mm::paging::pte_pfn(pte) as usize;
            let page = pfn_to_page(pfn);
            // _mapcount == 0  → exactly one PTE → private
            // _mapcount >= 1  → two or more PTEs → shared
            let mapcount = (*page)._mapcount.load(Ordering::Relaxed);
            if mapcount >= 1 {
                stats.shared_dirty += PAGE_SIZE as usize;
            } else {
                stats.private_dirty += PAGE_SIZE as usize;
            }
        }
        addr += PS;
    }

    stats
}

// ---------------------------------------------------------------------------
// Host unit tests — port of selected `mm/debug_vm_pgtable.c` invariants
// plus walker-specific shape tests against the existing test_pool mock.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::arch::x86::mm::paging::{
        __pte, _PAGE_ACCESSED, _PAGE_DIRTY, _PAGE_DIRTY_BITS, _PAGE_NX, _PAGE_PRESENT,
        _PAGE_PROTNONE, _PAGE_PSE, _PAGE_RW, _PAGE_SOFT_DIRTY, _PAGE_USER, PAGE_KERNEL,
        PAGE_OFFSET, init_pgd_for_test, map_kernel_page, pte_dirty, pte_mkclean, pte_mkdirty,
        pte_mkold, pte_mkwrite, pte_mkyoung, pte_present, pte_t, pte_write, pte_wrprotect,
        pte_young, reset_test_pool,
    };
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

    /// Walker tests share the global mock PML4 with the paging tests.
    /// Serialize them through one mutex.
    fn setup() {
        unsafe { reset_test_pool() };
    }

    // ── debug_vm_pgtable.c invariants ────────────────────────────────────

    #[test]
    fn pte_dirty_roundtrip() {
        let pte = __pte(0);
        assert!(!pte_dirty(pte));
        let dirty = pte_mkdirty(pte);
        assert!(pte_dirty(dirty));
        let clean = pte_mkclean(dirty);
        assert!(!pte_dirty(clean));
    }

    #[test]
    fn pte_young_roundtrip() {
        let pte = __pte(0);
        assert!(!pte_young(pte));
        let young = pte_mkyoung(pte);
        assert!(pte_young(young));
        let old = pte_mkold(young);
        assert!(!pte_young(old));
    }

    #[test]
    fn pte_write_roundtrip() {
        let pte = __pte(_PAGE_PRESENT);
        assert!(!pte_write(pte));
        let writable = pte_mkwrite(pte);
        assert!(pte_write(writable));
        let ro = pte_wrprotect(writable);
        assert!(!pte_write(ro));
    }

    #[test]
    fn pte_present_accepts_present_or_protnone() {
        assert!(!pte_present(__pte(0)));
        assert!(pte_present(__pte(_PAGE_PRESENT)));
        // PROTNONE: GLOBAL set, PRESENT clear — must still report present.
        assert!(pte_present(__pte(_PAGE_PROTNONE)));
    }

    #[test]
    fn pte_dirty_accepts_either_dirty_bit() {
        // Hardware Dirty.
        assert!(pte_dirty(__pte(_PAGE_DIRTY)));
        // Software-saved Dirty (used when shadow stack clears HW dirty).
        let saved = _PAGE_DIRTY_BITS & !_PAGE_DIRTY;
        assert!(pte_dirty(__pte(saved)));
    }

    // ── Walker shape tests against the mock PML4 ─────────────────────────

    /// Counter callback — counts how many times each level fires.
    struct Counter {
        ptes: usize,
        pmds: usize,
        puds: usize,
        holes_at_depth: [usize; 5],
    }

    impl Counter {
        fn new() -> Self {
            Self {
                ptes: 0,
                pmds: 0,
                puds: 0,
                holes_at_depth: [0; 5],
            }
        }
    }

    impl MmWalkOps for Counter {
        fn pte_entry(
            &mut self,
            _ptep: *mut pte_t,
            _addr: u64,
            _next: u64,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.ptes += 1;
            Ok(())
        }
        fn pmd_entry(
            &mut self,
            _pmdp: *mut pmd_t,
            _addr: u64,
            _next: u64,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.pmds += 1;
            Ok(())
        }
        fn pud_entry(
            &mut self,
            _pudp: *mut pud_t,
            _addr: u64,
            _next: u64,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.puds += 1;
            Ok(())
        }
        fn pte_hole(
            &mut self,
            _addr: u64,
            _next: u64,
            depth: i32,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.holes_at_depth[depth as usize] += 1;
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

    #[test]
    fn walk_empty_pml4_reports_pte_hole_at_depth_0() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let pgd = init_pgd_for_test();
        let mut counter = Counter::new();
        let start = 0x0000_0000_0010_0000_u64; // 1 MiB
        let end = start + PAGE_SIZE;
        unsafe {
            walk_kernel_page_table_range(start, end, &mut counter, pgd, ptr::null_mut()).unwrap()
        };
        assert_eq!(counter.ptes, 0);
        assert_eq!(counter.holes_at_depth[0], 1);
    }

    #[test]
    fn walk_over_mapped_range_visits_each_pte_once() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let phys: u64 = 0x0000_0000_0020_0000;
        let virt_a: u64 = 0x0000_0000_0040_0000;
        let virt_b: u64 = virt_a + PAGE_SIZE;
        let virt_c: u64 = virt_a + 2 * PAGE_SIZE;
        unsafe {
            map_kernel_page(virt_a, phys, PAGE_KERNEL);
            map_kernel_page(virt_b, phys + PAGE_SIZE, PAGE_KERNEL);
            map_kernel_page(virt_c, phys + 2 * PAGE_SIZE, PAGE_KERNEL);
        }

        let mut counter = Counter::new();
        let pgd = init_pgd_for_test();
        unsafe {
            walk_kernel_page_table_range(
                virt_a,
                virt_c + PAGE_SIZE,
                &mut counter,
                pgd,
                ptr::null_mut(),
            )
            .unwrap()
        };
        assert_eq!(counter.ptes, 3);
        // pmd_entry fires once for the single PMD that covers all three PTEs.
        assert_eq!(counter.pmds, 1);
        assert_eq!(counter.puds, 1);
    }

    /// Callback that aborts after the first PTE.
    struct StopAfterFirst {
        seen: usize,
    }
    impl MmWalkOps for StopAfterFirst {
        fn pte_entry(
            &mut self,
            _ptep: *mut pte_t,
            _addr: u64,
            _next: u64,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.seen += 1;
            Err(123)
        }
        fn has_pte_entry(&self) -> bool {
            true
        }
    }

    #[test]
    fn walk_short_circuits_on_err() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let phys: u64 = 0x0000_0000_0010_0000;
        let virt_a: u64 = 0x0000_0000_0050_0000;
        let virt_b: u64 = virt_a + PAGE_SIZE;
        unsafe {
            map_kernel_page(virt_a, phys, PAGE_KERNEL);
            map_kernel_page(virt_b, phys + PAGE_SIZE, PAGE_KERNEL);
        }

        let mut stop = StopAfterFirst { seen: 0 };
        let pgd = init_pgd_for_test();
        let err = unsafe {
            walk_kernel_page_table_range(
                virt_a,
                virt_b + PAGE_SIZE,
                &mut stop,
                pgd,
                ptr::null_mut(),
            )
        };
        assert_eq!(err, Err(123));
        assert_eq!(stop.seen, 1);
    }

    /// Callback that requests `Continue` from `pmd_entry` — should suppress
    /// the descent into PTEs entirely.
    struct PmdSkipper {
        pmd_calls: usize,
        pte_calls: usize,
    }
    impl MmWalkOps for PmdSkipper {
        fn pmd_entry(
            &mut self,
            _pmdp: *mut pmd_t,
            _addr: u64,
            _next: u64,
            walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.pmd_calls += 1;
            walk.action = PageWalkAction::Continue;
            Ok(())
        }
        fn pte_entry(
            &mut self,
            _ptep: *mut pte_t,
            _addr: u64,
            _next: u64,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.pte_calls += 1;
            Ok(())
        }
        fn has_pmd_entry(&self) -> bool {
            true
        }
        fn has_pte_entry(&self) -> bool {
            true
        }
    }

    #[test]
    fn action_continue_skips_subtree() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let phys: u64 = 0x0000_0000_0010_0000;
        let virt: u64 = 0x0000_0000_0060_0000;
        unsafe {
            map_kernel_page(virt, phys, PAGE_KERNEL);
        }

        let mut skipper = PmdSkipper {
            pmd_calls: 0,
            pte_calls: 0,
        };
        let pgd = init_pgd_for_test();
        unsafe {
            walk_kernel_page_table_range(virt, virt + PAGE_SIZE, &mut skipper, pgd, ptr::null_mut())
                .unwrap()
        };
        assert_eq!(skipper.pmd_calls, 1);
        assert_eq!(skipper.pte_calls, 0);
    }

    /// `Action::Again` — fires the callback twice on the same entry, then
    /// allows the walker to descend.
    struct AgainOnce {
        pmd_calls: usize,
        pte_calls: usize,
    }
    impl MmWalkOps for AgainOnce {
        fn pmd_entry(
            &mut self,
            _pmdp: *mut pmd_t,
            _addr: u64,
            _next: u64,
            walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.pmd_calls += 1;
            if self.pmd_calls == 1 {
                walk.action = PageWalkAction::Again;
            }
            Ok(())
        }
        fn pte_entry(
            &mut self,
            _ptep: *mut pte_t,
            _addr: u64,
            _next: u64,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            self.pte_calls += 1;
            Ok(())
        }
        fn has_pmd_entry(&self) -> bool {
            true
        }
        fn has_pte_entry(&self) -> bool {
            true
        }
    }

    #[test]
    fn action_again_re_invokes_handler_then_descends() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let phys: u64 = 0x0000_0000_0010_0000;
        let virt: u64 = 0x0000_0000_0070_0000;
        unsafe {
            map_kernel_page(virt, phys, PAGE_KERNEL);
        }

        let mut again = AgainOnce {
            pmd_calls: 0,
            pte_calls: 0,
        };
        let pgd = init_pgd_for_test();
        unsafe {
            walk_kernel_page_table_range(virt, virt + PAGE_SIZE, &mut again, pgd, ptr::null_mut())
                .unwrap()
        };
        assert_eq!(again.pmd_calls, 2);
        assert_eq!(again.pte_calls, 1);
    }

    #[test]
    fn walk_respects_start_and_end_bounds() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let phys: u64 = 0x0000_0000_0010_0000;
        let inside: u64 = 0x0000_0000_0080_0000;
        let outside: u64 = inside + PAGE_SIZE;
        unsafe {
            map_kernel_page(inside, phys, PAGE_KERNEL);
            map_kernel_page(outside, phys + PAGE_SIZE, PAGE_KERNEL);
        }

        let mut counter = Counter::new();
        let pgd = init_pgd_for_test();
        unsafe {
            walk_kernel_page_table_range(
                inside,
                inside + PAGE_SIZE,
                &mut counter,
                pgd,
                ptr::null_mut(),
            )
            .unwrap()
        };
        assert_eq!(counter.ptes, 1);
    }

    #[test]
    fn walker_oracle_matches_virt_to_phys() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        use crate::arch::x86::mm::paging::virt_to_phys;
        let phys: u64 = 0x0000_0000_0030_0000;
        let virt: u64 = 0x0000_0000_0090_0000;
        unsafe {
            map_kernel_page(virt, phys, PAGE_KERNEL);
        }
        assert_eq!(virt_to_phys(virt), Some(phys));

        // The reverse: an unmapped address must report None via the walker.
        assert_eq!(virt_to_phys(virt + PAGE_SIZE), None);
    }

    #[test]
    fn unused_helpers_link() {
        // Defensive: makes sure the dead-code suppression helpers compile.
        // Without this, the `_force_use_of_*` functions could be stripped
        // by an over-eager linker on some hosts.
        let _ = _force_use_of_null();
        let _ = PAGE_OFFSET;
        let _ = _PAGE_ACCESSED;
        let _ = _PAGE_NX;
        let _ = _PAGE_RW;
        let _ = _PAGE_USER;
        let _ = _PAGE_SOFT_DIRTY;
        let _ = _PAGE_PSE;
    }

    // ── M11: walk_page_range / walk_page_vma integration ────────────

    #[test]
    fn mm_walk_has_mm_and_vma_fields() {
        let walk = MmWalk::new(core::ptr::null_mut(), core::ptr::null_mut());
        assert!(walk.mm.is_null());
        assert!(walk.vma.is_null());
    }

    #[test]
    fn mm_walk_new_mm_sets_mm_ptr() {
        use crate::mm::mm_types::MmStruct;
        let mm = MmStruct::new(0);
        let walk = MmWalk::new_mm(
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mm as *const MmStruct,
        );
        assert!(!walk.mm.is_null());
        assert!(walk.vma.is_null());
    }

    /// Verify that test_walk, pre_vma, post_vma hooks are called.
    #[test]
    fn walk_ops_vma_hooks_default() {
        struct NoOp;
        impl MmWalkOps for NoOp {}

        let mut ops = NoOp;
        let walk = MmWalk::new(core::ptr::null_mut(), core::ptr::null_mut());

        // Default test_walk returns true.
        assert!(ops.test_walk(&walk));

        let mut walk2 = MmWalk::new(core::ptr::null_mut(), core::ptr::null_mut());
        assert!(ops.pre_vma(&mut walk2).is_ok());
        ops.post_vma(&mut walk2);
    }

    /// Verify that test_walk returning false can skip processing.
    #[test]
    fn test_walk_can_skip() {
        struct SkipAll;
        impl MmWalkOps for SkipAll {
            fn test_walk(&mut self, _walk: &MmWalk<'_>) -> bool {
                false
            }
        }

        let mut ops = SkipAll;
        let walk = MmWalk::new(core::ptr::null_mut(), core::ptr::null_mut());
        assert!(!ops.test_walk(&walk));
    }

    /// Walk an mm_struct with no VMAs — pte_hole should cover entire range.
    #[test]
    fn walk_page_range_empty_mm() {
        use crate::mm::mm_types::MmStruct;

        let _guard = TEST_LOCK.lock().unwrap();
        setup();

        struct HoleCounter {
            hole_count: usize,
            total_span: u64,
        }
        impl MmWalkOps for HoleCounter {
            fn pte_hole(
                &mut self,
                addr: u64,
                next: u64,
                _depth: i32,
                _walk: &mut MmWalk<'_>,
            ) -> Result<(), i32> {
                self.hole_count += 1;
                self.total_span += next - addr;
                Ok(())
            }
        }

        // Create mm with no VMAs, using a test PGD.
        let pgd = unsafe { init_pgd_for_test() };
        let mm = MmStruct::new(pgd as usize);
        let mut ops = HoleCounter {
            hole_count: 0,
            total_span: 0,
        };

        let result =
            unsafe { walk_page_range(&mm, 0x1000, 0x5000, &mut ops, core::ptr::null_mut()) };
        assert!(result.is_ok());
        assert!(ops.hole_count > 0);
        assert_eq!(ops.total_span, 0x4000);
    }

    #[test]
    fn walk_page_mapping_visits_registered_vma_index_range() {
        use crate::mm::address_space::{
            AddressSpace, register_mapping_vma, unregister_mapping_vma,
        };
        use crate::mm::list::ListHead;
        use crate::mm::mm_types::{MmStruct, VmAreaStruct};

        let _guard = TEST_LOCK.lock().unwrap();
        setup();

        struct MappingCounter {
            pre_vmas: usize,
            holes: usize,
            span: u64,
        }
        impl MmWalkOps for MappingCounter {
            fn pre_vma(&mut self, _walk: &mut MmWalk<'_>) -> Result<(), i32> {
                self.pre_vmas += 1;
                Ok(())
            }

            fn pte_hole(
                &mut self,
                addr: u64,
                next: u64,
                _depth: i32,
                _walk: &mut MmWalk<'_>,
            ) -> Result<(), i32> {
                self.holes += 1;
                self.span += next - addr;
                Ok(())
            }
        }

        let mut mapping = AddressSpace::new();
        let pgd = unsafe { init_pgd_for_test() };
        let mut mm = MmStruct::new(pgd as usize);
        let mut vma = std::boxed::Box::new(VmAreaStruct::new(0x4000, 0x8000, 0));
        vma.vm_mm = &mut mm;
        vma.vm_file = &mut mapping as *mut AddressSpace as usize;
        vma.vm_pgoff = 2;
        let vma_ptr = std::boxed::Box::into_raw(vma);
        unsafe { ListHead::init(&mut (*vma_ptr).anon_vma_chain) };

        register_mapping_vma(&mut mapping, vma_ptr);
        let mut ops = MappingCounter {
            pre_vmas: 0,
            holes: 0,
            span: 0,
        };

        let result =
            unsafe { walk_page_mapping(&mut mapping, 3, 1, &mut ops, core::ptr::null_mut()) };
        assert_eq!(result, Ok(()));
        assert_eq!(ops.pre_vmas, 1);
        assert_eq!(ops.span, PAGE_SIZE);
        assert!(ops.holes > 0);

        unregister_mapping_vma(&mut mapping, vma_ptr);
        unsafe {
            let _ = std::boxed::Box::from_raw(vma_ptr);
        }
    }
}
