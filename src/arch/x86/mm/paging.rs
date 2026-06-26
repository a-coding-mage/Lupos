//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm
//! test-origin: linux:vendor/linux/arch/x86/mm
/// x86-64 four-level page table types and operations.
///
/// Matches Linux's `arch/x86/include/asm/pgtable_64_types.h`,
/// `arch/x86/include/asm/pgtable_types.h`, and `include/linux/pgtable.h`
/// for the 4-level (PGD→PUD→PMD→PTE) paging mode.
///
/// All mappings created here live in the kernel's initial page tables
/// (`init_pgd` / CR3), which the boot stub already sets up with:
///   - a temporary 1:1 identity map for early boot
///   - the Linux-style direct map (`PAGE_OFFSET`)
///   - the higher-half kernel image mapping at `__START_KERNEL_map`
///
/// `map_kernel_page` adds new leaf PTE entries (and intermediate tables on
/// demand) for vmalloc/kmap-style windows without disturbing the bootstrap
/// mappings.
///
/// ## References
///
/// - Linux `arch/x86/include/asm/pgtable_64_types.h` — types / shifts
/// - Linux `arch/x86/include/asm/pgtable_types.h` — flag bits
/// - Linux `include/linux/pgtable.h` — index / offset / alloc helpers
/// - Linux `mm/vmalloc.c` — `map_kernel_range_noflush`, `vmap_pte_range`
/// - Intel SDM Vol. 3A §4.5 — 4-level paging

// ---------------------------------------------------------------------------
// Scalar types — Linux arch/x86/include/asm/pgtable_64_types.h
// ---------------------------------------------------------------------------

#[cfg(not(test))]
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(test))]
use spin::Mutex;

/// Raw value type for a PTE (page table entry, level 1).
pub type pteval_t = u64;
/// Raw value type for a PMD entry (level 2).
pub type pmdval_t = u64;
/// Raw value type for a PUD entry (level 3).
pub type pudval_t = u64;
/// Raw value type for a PGD entry (level 4, PML4).
pub type pgdval_t = u64;
/// Raw value type for a protection mask.
pub type pgprotval_t = u64;

// ---------------------------------------------------------------------------
// Typed page-table entry wrappers — matches Linux struct { val_t xxx; };
// ---------------------------------------------------------------------------

/// Level-1 page table entry (4 KiB leaf).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct pte_t(pub pteval_t);

/// Level-2 page middle directory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct pmd_t(pub pmdval_t);

/// Level-3 page upper directory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct pud_t(pub pudval_t);

/// Level-4 page global directory entry (PML4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct pgd_t(pub pgdval_t);

/// Raw value type for a P4D entry (level 4 in 5-level paging).
///
/// On x86_64 with 4-level paging the P4D level is folded into the PGD —
/// `PTRS_PER_P4D == 1` and `p4d_offset(pgdp, addr) == (p4d_t*)pgdp`. We keep
/// the type so the generic walker can run unchanged when CPUs gain LA57.
///
/// Ref: `vendor/linux/include/asm-generic/pgtable-nop4d.h`
pub type p4dval_t = u64;

/// Folded P4D entry — same shape as PGD on 4-level x86_64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct p4d_t(pub p4dval_t);

/// Page protection bits (user-visible encoding, same bit positions as PTEs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct pgprot_t(pub pgprotval_t);

// ---------------------------------------------------------------------------
// CamelCase aliases requested by Milestone 10 — these are exactly the
// `pXd_t` types under Rust-style names so the rest of the kernel and the
// generic walker can refer to them without dragging in the lowercase Linux
// spelling. Both spellings refer to the same `#[repr(transparent)]` type
// so any function that takes a `*mut PteEntry` can also accept a `*mut pte_t`.
// ---------------------------------------------------------------------------

/// Rust-cased alias for [`pte_t`].
pub type PteEntry = pte_t;
/// Rust-cased alias for [`pmd_t`].
pub type PmdEntry = pmd_t;
/// Rust-cased alias for [`pud_t`].
pub type PudEntry = pud_t;
/// Rust-cased alias for [`p4d_t`] (folded into PGD on 4-level x86_64).
pub type P4dEntry = p4d_t;
/// Rust-cased alias for [`pgd_t`] (PML4).
pub type PgdEntry = pgd_t;

/// 4 KiB page-table page — exactly 512 entries of any one of the per-level
/// types above.  Layout matches the hardware:
///
/// - `#[repr(C)]` so the field order is stable;
/// - `align(4096)` so a `PageTable<T>` instance is itself a valid page-table
///   page when allocated as static or test storage;
/// - generic over `T` so we get a single named wrapper for all five levels.
///
/// Production code does not allocate `PageTable` from the heap — table pages
/// come from the buddy allocator (`alloc_pt_page()`). The wrapper is provided
/// so safer code paths can borrow a typed `&[T; 512]` slice from a raw
/// `*mut T` table base.
#[repr(C, align(4096))]
pub struct PageTable<T> {
    pub entries: [T; 512],
}

/// Construct a `pgprot_t` from a raw `pgprotval_t`.
///
/// Ref: Linux `__pgprot(x)` macro — `arch/x86/include/asm/pgtable_types.h:202`
#[inline]
pub const fn __pgprot(val: pgprotval_t) -> pgprot_t {
    pgprot_t(val)
}

/// Construct a `pte_t` from a raw `pteval_t`.
///
/// Ref: Linux `__pte(x)` — `arch/x86/include/asm/pgtable.h`
#[inline]
pub const fn __pte(val: pteval_t) -> pte_t {
    pte_t(val)
}

/// Construct a `pmd_t` from a raw `pmdval_t`.
#[inline]
pub const fn __pmd(val: pmdval_t) -> pmd_t {
    pmd_t(val)
}

/// Construct a `pud_t` from a raw `pudval_t`.
#[inline]
pub const fn __pud(val: pudval_t) -> pud_t {
    pud_t(val)
}

/// Construct a `pgd_t` from a raw `pgdval_t`.
#[inline]
pub const fn __pgd(val: pgdval_t) -> pgd_t {
    pgd_t(val)
}

/// Construct a `p4d_t` from a raw `p4dval_t`.  Folded into PGD on x86_64.
#[inline]
pub const fn __p4d(val: p4dval_t) -> p4d_t {
    p4d_t(val)
}

// `pXd_val` extractors — the C macros that drivers call by name.
// Linux: `pte_val(pte) == pte.pte`, etc.

#[inline]
pub const fn pte_val(pte: pte_t) -> pteval_t {
    pte.0
}
#[inline]
pub const fn pmd_val(pmd: pmd_t) -> pmdval_t {
    pmd.0
}
#[inline]
pub const fn pud_val(pud: pud_t) -> pudval_t {
    pud.0
}
#[inline]
pub const fn p4d_val(p4d: p4d_t) -> p4dval_t {
    p4d.0
}
#[inline]
pub const fn pgd_val(pgd: pgd_t) -> pgdval_t {
    pgd.0
}
#[inline]
pub const fn pgprot_val(prot: pgprot_t) -> pgprotval_t {
    prot.0
}

// ---------------------------------------------------------------------------
// Virtual-address shift constants — pgtable_64_types.h
// ---------------------------------------------------------------------------

/// Top-level (PGD / PML4) shift: bits 47:39 index the PGD.
///
/// Ref: Linux `arch/x86/include/asm/pgtable_64_types.h:50` — `PGDIR_SHIFT`
pub const PGDIR_SHIFT: u32 = 39;

/// P4D shift — equal to `PGDIR_SHIFT` because the P4D level is folded into
/// the PGD on 4-level x86_64.  Promoting to LA57 will set this to 48.
///
/// Ref: `vendor/linux/include/asm-generic/pgtable-nop4d.h`
pub const P4D_SHIFT: u32 = PGDIR_SHIFT;

/// PUD shift: bits 38:30 index the PUD (level 3).
///
/// Ref: Linux `arch/x86/include/asm/pgtable_64_types.h:67` — `PUD_SHIFT`
pub const PUD_SHIFT: u32 = 30;

/// PMD shift: bits 29:21 index the PMD (level 2).
///
/// Ref: Linux `arch/x86/include/asm/pgtable_64_types.h:73` — `PMD_SHIFT`
pub const PMD_SHIFT: u32 = 21;

/// Page shift: bits 20:12 index the PTE (level 1).
pub const PAGE_SHIFT: u32 = 12;

/// Bytes per 4 KiB base page.
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
/// Page-aligned address mask (`PAGE_SIZE - 1` is the offset; this is its complement).
pub const PAGE_MASK: u64 = !(PAGE_SIZE - 1);

/// Bytes covered by one PMD entry (2 MiB).
pub const PMD_SIZE: u64 = 1 << PMD_SHIFT;
/// Bytes covered by one PUD entry (1 GiB).
pub const PUD_SIZE: u64 = 1 << PUD_SHIFT;
/// Bytes covered by one P4D entry (512 GiB on 4-level == one PGD slot).
pub const P4D_SIZE: u64 = 1 << P4D_SHIFT;
/// Bytes covered by one PGD entry (512 GiB on 4-level x86_64).
pub const PGDIR_SIZE: u64 = 1 << PGDIR_SHIFT;

/// Number of entries per page table (all levels are 512-entry = 4 KiB).
pub const PTRS_PER_PGD: usize = 512;
/// Number of P4D entries per table — folded to 1 on 4-level x86_64.
pub const PTRS_PER_P4D: usize = 1;
/// Number of PUD entries per table.
pub const PTRS_PER_PUD: usize = 512;
/// Number of PMD entries per table.
pub const PTRS_PER_PMD: usize = 512;
/// Number of PTE entries per table.
pub const PTRS_PER_PTE: usize = 512;

// ---------------------------------------------------------------------------
// Page-flag bit positions — pgtable_types.h §10-30
// ---------------------------------------------------------------------------

pub const _PAGE_BIT_PRESENT: u32 = 0;
pub const _PAGE_BIT_RW: u32 = 1;
pub const _PAGE_BIT_USER: u32 = 2;
pub const _PAGE_BIT_PWT: u32 = 3;
pub const _PAGE_BIT_PCD: u32 = 4;
pub const _PAGE_BIT_ACCESSED: u32 = 5;
pub const _PAGE_BIT_DIRTY: u32 = 6;
pub const _PAGE_BIT_PSE: u32 = 7; // huge page (2 MB or 1 GB)
pub const _PAGE_BIT_PAT: u32 = 7; // PAT bit on 4 KiB pages (alias of PSE)
pub const _PAGE_BIT_GLOBAL: u32 = 8;
pub const _PAGE_BIT_SOFTW1: u32 = 9;
pub const _PAGE_BIT_SOFTW2: u32 = 10;
pub const _PAGE_BIT_SOFTW3: u32 = 11;
pub const _PAGE_BIT_PAT_LARGE: u32 = 12; // PAT bit on 2 MB / 1 GB pages
pub const _PAGE_BIT_SOFTW4: u32 = 57;
pub const _PAGE_BIT_SOFTW5: u32 = 58;
pub const _PAGE_BIT_PKEY_BIT0: u32 = 59;
pub const _PAGE_BIT_PKEY_BIT1: u32 = 60;
pub const _PAGE_BIT_PKEY_BIT2: u32 = 61;
pub const _PAGE_BIT_PKEY_BIT3: u32 = 62;
pub const _PAGE_BIT_NX: u32 = 63;

// Software-bit aliases — pgtable_types.h:32-44
// These match Linux exactly: SPECIAL/CPA_TEST share SOFTW1, UFFD_WP shares
// SOFTW2, SOFT_DIRTY shares SOFTW3 (or SOFTW2 on 32-bit; we are 64-bit only),
// SAVED_DIRTY shares SOFTW5.
pub const _PAGE_BIT_SPECIAL: u32 = _PAGE_BIT_SOFTW1;
pub const _PAGE_BIT_CPA_TEST: u32 = _PAGE_BIT_SOFTW1;
pub const _PAGE_BIT_UFFD_WP: u32 = _PAGE_BIT_SOFTW2;
pub const _PAGE_BIT_SOFT_DIRTY: u32 = _PAGE_BIT_SOFTW3;
pub const _PAGE_BIT_SAVED_DIRTY: u32 = _PAGE_BIT_SOFTW5;

// When _PAGE_PRESENT is clear, Linux reuses the GLOBAL bit as PROTNONE marker.
// Ref: pgtable_types.h:49 — `#define _PAGE_BIT_PROTNONE _PAGE_BIT_GLOBAL`
pub const _PAGE_BIT_PROTNONE: u32 = _PAGE_BIT_GLOBAL;

// ---------------------------------------------------------------------------
// Page flags — pgtable_types.h §51-160
// ---------------------------------------------------------------------------

pub const _PAGE_PRESENT: u64 = 1 << _PAGE_BIT_PRESENT;
pub const _PAGE_RW: u64 = 1 << _PAGE_BIT_RW;
pub const _PAGE_USER: u64 = 1 << _PAGE_BIT_USER;
pub const _PAGE_PWT: u64 = 1 << _PAGE_BIT_PWT;
pub const _PAGE_PCD: u64 = 1 << _PAGE_BIT_PCD;
pub const _PAGE_ACCESSED: u64 = 1 << _PAGE_BIT_ACCESSED;
pub const _PAGE_DIRTY: u64 = 1 << _PAGE_BIT_DIRTY;
pub const _PAGE_PSE: u64 = 1 << _PAGE_BIT_PSE;
pub const _PAGE_PAT: u64 = 1 << _PAGE_BIT_PAT;
pub const _PAGE_GLOBAL: u64 = 1 << _PAGE_BIT_GLOBAL;
pub const _PAGE_SPECIAL: u64 = 1 << _PAGE_BIT_SPECIAL;
pub const _PAGE_CPA_TEST: u64 = 1 << _PAGE_BIT_CPA_TEST;
pub const _PAGE_PAT_LARGE: u64 = 1 << _PAGE_BIT_PAT_LARGE;
pub const _PAGE_PKEY_BIT0: u64 = 1 << _PAGE_BIT_PKEY_BIT0;
pub const _PAGE_PKEY_BIT1: u64 = 1 << _PAGE_BIT_PKEY_BIT1;
pub const _PAGE_PKEY_BIT2: u64 = 1 << _PAGE_BIT_PKEY_BIT2;
pub const _PAGE_PKEY_BIT3: u64 = 1 << _PAGE_BIT_PKEY_BIT3;
pub const _PAGE_NX: u64 = 1u64 << _PAGE_BIT_NX;
pub const _PAGE_SOFT_DIRTY: u64 = 1u64 << _PAGE_BIT_SOFT_DIRTY;
pub const _PAGE_UFFD_WP: u64 = 1u64 << _PAGE_BIT_UFFD_WP;
pub const _PAGE_SAVED_DIRTY: u64 = 1u64 << _PAGE_BIT_SAVED_DIRTY;

/// PROTNONE marker — Linux uses GLOBAL when PRESENT is clear so a fault on
/// such a page can be distinguished from a true absence.
///
/// Ref: pgtable_types.h:141
pub const _PAGE_PROTNONE: u64 = 1u64 << _PAGE_BIT_PROTNONE;

/// Combined "is dirty" mask used by `pte_dirty()` — accepts either the
/// hardware Dirty bit or the software-saved-dirty bit (used when shadow
/// stack pages clear hw Dirty to encode WSS).
///
/// Ref: pgtable_types.h:139 — `_PAGE_DIRTY_BITS`
pub const _PAGE_DIRTY_BITS: u64 = _PAGE_DIRTY | _PAGE_SAVED_DIRTY;

/// Mask of every flag bit (everything that is *not* the page frame number).
/// Linux: `~PTE_PFN_MASK`.
pub const PTE_FLAGS_MASK: u64 = !PTE_PFN_MASK;

/// Bits that may be modified across `pte_modify()` calls on a 4 KiB PTE.
///
/// Ref: pgtable_types.h:151-155 — `_COMMON_PAGE_CHG_MASK` + `_PAGE_PAT`.
/// The AMD SME/SEV encryption bit is dynamic (`sme_me_mask`) and sits in the
/// PFN field on x86, so this mask preserves it. Dedicated C-bit flips go
/// through `set_kernel_page_encryption_mask()`.
pub const _COMMON_PAGE_CHG_MASK: u64 = PTE_PFN_MASK
    | _PAGE_PCD
    | _PAGE_PWT
    | _PAGE_SPECIAL
    | _PAGE_ACCESSED
    | _PAGE_DIRTY_BITS
    | _PAGE_SOFT_DIRTY
    | _PAGE_UFFD_WP;
pub const _PAGE_CHG_MASK: u64 = _COMMON_PAGE_CHG_MASK | _PAGE_PAT;
/// Same as `_PAGE_CHG_MASK` for huge (PSE) pages.
pub const _HPAGE_CHG_MASK: u64 = _COMMON_PAGE_CHG_MASK | _PAGE_PSE | _PAGE_PAT_LARGE;

/// Linux-style direct-map base for physical memory.
pub const PAGE_OFFSET: u64 = 0xffff_8880_0000_0000;

/// Reserved kernel temporary mapping slot used by `kmap` / `kunmap`.
pub const KMAP_START: u64 = 0xffff_fe00_0000_0000;

/// Mask that extracts the physical page-frame number from an entry.
///
/// Bits 51:12 hold the PFN; the rest are flags.
///
/// Ref: Linux `PTE_PFN_MASK` — `arch/x86/include/asm/pgtable_types.h`
pub const PTE_PFN_MASK: u64 = 0x000F_FFFF_FFFF_F000;

// ---------------------------------------------------------------------------
// Kernel protection constants — pgtable_types.h §218-231
// ---------------------------------------------------------------------------

/// Flags for intermediate kernel page-table pages (PGD/PUD/PMD entries).
/// No USER bit — these tables are kernel-only.
///
/// Ref: Linux `_KERNPG_TABLE` (no encryption variant):
///      `__PP | __RW | 0 | ___A | 0 | ___D | 0 | 0`
pub const _KERNPG_TABLE: u64 = _PAGE_PRESENT | _PAGE_RW | _PAGE_ACCESSED | _PAGE_DIRTY;

/// Flags for intermediate page-table pages that user-space can traverse.
///
/// Equivalent to Linux's `_PAGE_TABLE` — `_KERNPG_TABLE | _PAGE_USER`.
/// Used by the fault handler when building page tables for user VMAs.
///
/// Ref: Linux `arch/x86/include/asm/pgtable_types.h` — `_PAGE_TABLE`
pub const _PAGE_TABLE: u64 = _KERNPG_TABLE | _PAGE_USER;

/// Flags for a standard writable, non-executable kernel data page.
///
/// Ref: Linux `__PAGE_KERNEL`:
///      `__PP | __RW | 0 | ___A | __NX | ___D | 0 | ___G`
pub const __PAGE_KERNEL: u64 =
    _PAGE_PRESENT | _PAGE_RW | _PAGE_ACCESSED | _PAGE_DIRTY | _PAGE_NX | _PAGE_GLOBAL;

/// Standard kernel read-only page protection.
///
/// Ref: Linux `__PAGE_KERNEL_RO`:
///      `__PP | 0 | 0 | ___A | __NX | 0 | 0 | ___G`
pub const __PAGE_KERNEL_RO: u64 = _PAGE_PRESENT | _PAGE_ACCESSED | _PAGE_NX | _PAGE_GLOBAL;

/// Standard kernel read-only executable page protection.
///
/// Ref: Linux `PAGE_KERNEL_ROX` on x86_64: present, supervisor, accessed,
/// global, and executable because `_PAGE_NX` is clear.
pub const __PAGE_KERNEL_ROX: u64 = _PAGE_PRESENT | _PAGE_ACCESSED | _PAGE_GLOBAL;

/// `pgprot_t` for writable, non-executable kernel pages.
///
/// Equivalent to Linux `PAGE_KERNEL` (without SME encryption bit).
pub const PAGE_KERNEL: pgprot_t = __pgprot(__PAGE_KERNEL);

/// `pgprot_t` for read-only, non-executable kernel pages.
pub const PAGE_KERNEL_RO: pgprot_t = __pgprot(__PAGE_KERNEL_RO);

/// `pgprot_t` for read-only, executable kernel text pages.
pub const PAGE_KERNEL_ROX: pgprot_t = __pgprot(__PAGE_KERNEL_ROX);

/// Translate a physical address into the kernel direct-map window.
#[inline]
pub fn phys_to_virt(phys: u64) -> *mut u8 {
    if phys > u64::MAX - PAGE_OFFSET {
        crate::kernel::printk::log_error!(
            "mm",
            "phys_to_virt: overflow phys={:#018x} page_offset={:#018x}",
            phys,
            PAGE_OFFSET
        );
    }
    PAGE_OFFSET.wrapping_add(phys) as *mut u8
}

/// Translate a page frame number (PFN) to its kernel virtual address.
///
/// Equivalent to Linux's `page_to_virt()` / `pfn_to_kaddr()`:
/// `phys = pfn << PAGE_SHIFT`, then apply the direct-map offset.
///
/// In the host-side test runner, physical == virtual (identity map),
/// matching the `entry_base_ptr` convention used throughout the test harness.
///
/// Ref: Linux `include/asm-generic/memory_model.h` — `page_to_virt()`
#[cfg(not(test))]
#[inline]
pub fn pfn_to_virt(pfn: usize) -> *mut u8 {
    phys_to_virt((pfn as u64) << PAGE_SHIFT)
}

#[cfg(test)]
#[inline]
pub fn pfn_to_virt(pfn: usize) -> *mut u8 {
    ((pfn << PAGE_SHIFT as usize) as u64) as *mut u8
}

#[cfg(not(test))]
#[inline]
fn entry_base_ptr(entry: u64) -> *mut u8 {
    phys_to_virt(entry & PTE_PFN_MASK)
}

#[cfg(test)]
#[inline]
fn entry_base_ptr(entry: u64) -> *mut u8 {
    (entry & PTE_PFN_MASK) as *mut u8
}

// ---------------------------------------------------------------------------
// Entry constructors / extractors
// ---------------------------------------------------------------------------

/// Build a PTE from a PFN and protection flags.
///
/// Ref: Linux `pfn_pte()` — `arch/x86/include/asm/pgtable.h`
#[inline]
pub fn pfn_pte(pfn: u64, prot: pgprot_t) -> pte_t {
    pte_t((pfn << PAGE_SHIFT) | prot.0)
}

/// Extract the PFN from a PTE.
///
/// Ref: Linux `pte_pfn()` — `arch/x86/include/asm/pgtable.h`
#[inline]
pub fn pte_pfn(pte: pte_t) -> u64 {
    (pte.0 & PTE_PFN_MASK) >> PAGE_SHIFT
}

/// Return the physical address encoded in a PTE (page-aligned).
#[inline]
pub fn pte_phys(pte: pte_t) -> u64 {
    pte.0 & PTE_PFN_MASK
}

// ---------------------------------------------------------------------------
// None predicates — an entry is "none" (absent) when its raw value is zero.
//
// Ref: Linux `pgd_none`, `pud_none`, `pmd_none`, `pte_none`
//      arch/x86/include/asm/pgtable.h
// ---------------------------------------------------------------------------

#[inline]
pub fn pgd_none(pgd: pgd_t) -> bool {
    pgd.0 == 0
}
#[inline]
pub fn p4d_none(p4d: p4d_t) -> bool {
    p4d.0 == 0
}
#[inline]
pub fn pud_none(pud: pud_t) -> bool {
    pud.0 == 0
}
#[inline]
pub fn pmd_none(pmd: pmd_t) -> bool {
    pmd.0 == 0
}
#[inline]
pub fn pte_none(pte: pte_t) -> bool {
    pte.0 == 0
}

// ---------------------------------------------------------------------------
// Presence predicates — `pte_present` accepts either a hardware-present
// entry or one that has been marked PROTNONE (PRESENT clear, GLOBAL set).
//
// Ref: arch/x86/include/asm/pgtable.h — `pte_present`, `pmd_present`,
// `pud_present`, `pgd_present`, `p4d_present`.
// ---------------------------------------------------------------------------

#[inline]
pub fn pte_present(pte: pte_t) -> bool {
    (pte.0 & (_PAGE_PRESENT | _PAGE_PROTNONE)) != 0
}

#[inline]
pub fn pmd_present(pmd: pmd_t) -> bool {
    // Linux also accepts a PSE huge page as "present" — we mirror that.
    (pmd.0 & (_PAGE_PRESENT | _PAGE_PROTNONE | _PAGE_PSE)) != 0
}

#[inline]
pub fn pud_present(pud: pud_t) -> bool {
    (pud.0 & _PAGE_PRESENT) != 0
}

#[inline]
pub fn p4d_present(p4d: p4d_t) -> bool {
    (p4d.0 & _PAGE_PRESENT) != 0
}

#[inline]
pub fn pgd_present(pgd: pgd_t) -> bool {
    (pgd.0 & _PAGE_PRESENT) != 0
}

// ---------------------------------------------------------------------------
// Leaf / huge predicates — Linux `pmd_huge`, `pud_huge`, `pmd_leaf`,
// `pud_leaf`, `pmd_trans_huge`.  All inspect _PAGE_PSE on a level-2 or
// level-3 entry.
// ---------------------------------------------------------------------------

#[inline]
pub fn pmd_huge(pmd: pmd_t) -> bool {
    pmd_present(pmd) && (pmd.0 & _PAGE_PSE) != 0
}
#[inline]
pub fn pud_huge(pud: pud_t) -> bool {
    pud_present(pud) && (pud.0 & _PAGE_PSE) != 0
}
#[inline]
pub fn pmd_leaf(pmd: pmd_t) -> bool {
    pmd_huge(pmd)
}
#[inline]
pub fn pud_leaf(pud: pud_t) -> bool {
    pud_huge(pud)
}
#[inline]
pub fn pmd_trans_huge(pmd: pmd_t) -> bool {
    pmd_huge(pmd)
}

// ---------------------------------------------------------------------------
// PTE bit-test accessors — pgtable.h:156-220
// ---------------------------------------------------------------------------

#[inline]
pub fn pte_dirty(pte: pte_t) -> bool {
    (pte.0 & _PAGE_DIRTY_BITS) != 0
}
#[inline]
pub fn pte_young(pte: pte_t) -> bool {
    (pte.0 & _PAGE_ACCESSED) != 0
}
#[inline]
pub fn pte_write(pte: pte_t) -> bool {
    (pte.0 & _PAGE_RW) != 0
}
#[inline]
pub fn pte_exec(pte: pte_t) -> bool {
    (pte.0 & _PAGE_NX) == 0
}
#[inline]
pub fn pte_special(pte: pte_t) -> bool {
    (pte.0 & _PAGE_SPECIAL) != 0
}
#[inline]
pub fn pte_soft_dirty(pte: pte_t) -> bool {
    (pte.0 & _PAGE_SOFT_DIRTY) != 0
}
#[inline]
pub fn pte_uffd_wp(pte: pte_t) -> bool {
    (pte.0 & _PAGE_UFFD_WP) != 0
}
#[inline]
pub fn pte_global(pte: pte_t) -> bool {
    (pte.0 & _PAGE_GLOBAL) != 0
}

// ---------------------------------------------------------------------------
// PTE bit-set / bit-clear constructors — pgtable.h:230-300.
// Each returns a new `pte_t`; Linux uses `pte_set_flags` / `pte_clear_flags`
// internally — we open-code them inline since they are trivial.
// ---------------------------------------------------------------------------

#[inline]
pub fn pte_mkdirty(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_DIRTY | _PAGE_SOFT_DIRTY)
}
#[inline]
pub fn pte_mkclean(pte: pte_t) -> pte_t {
    pte_t(pte.0 & !_PAGE_DIRTY_BITS)
}
#[inline]
pub fn pte_mkyoung(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_ACCESSED)
}
#[inline]
pub fn pte_mkold(pte: pte_t) -> pte_t {
    pte_t(pte.0 & !_PAGE_ACCESSED)
}
#[inline]
pub fn pte_mkwrite(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_RW)
}
#[inline]
pub fn pte_wrprotect(pte: pte_t) -> pte_t {
    pte_t(pte_val(pte) & !_PAGE_RW)
}
#[inline]
pub fn pte_mkexec(pte: pte_t) -> pte_t {
    pte_t(pte.0 & !_PAGE_NX)
}
#[inline]
pub fn pte_mknoexec(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_NX)
}
#[inline]
pub fn pte_mkspecial(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_SPECIAL)
}
#[inline]
pub fn pte_mksoft_dirty(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_SOFT_DIRTY)
}
#[inline]
pub fn pte_clear_soft_dirty(pte: pte_t) -> pte_t {
    pte_t(pte.0 & !_PAGE_SOFT_DIRTY)
}
#[inline]
pub fn pte_mkuffd_wp(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_UFFD_WP)
}
#[inline]
pub fn pte_clear_uffd_wp(pte: pte_t) -> pte_t {
    pte_t(pte.0 & !_PAGE_UFFD_WP)
}
#[inline]
pub fn pte_mkglobal(pte: pte_t) -> pte_t {
    pte_t(pte.0 | _PAGE_GLOBAL)
}
#[inline]
pub fn pte_clrglobal(pte: pte_t) -> pte_t {
    pte_t(pte.0 & !_PAGE_GLOBAL)
}

/// Replace the protection bits of `pte` with `prot`, preserving the PFN
/// and the bits listed in `_PAGE_CHG_MASK`.
///
/// Ref: pgtable.h `pte_modify`.
#[inline]
pub fn pte_modify(pte: pte_t, prot: pgprot_t) -> pte_t {
    pte_t((pte.0 & _PAGE_CHG_MASK) | (prot.0 & !_PAGE_CHG_MASK))
}

// ---------------------------------------------------------------------------
// PMD-level set/clear — needed by `pmd_huge`/anonymous-THP code paths.
// Mirrors `pmd_mk*` in pgtable.h.
// ---------------------------------------------------------------------------

#[inline]
pub fn pmd_mkdirty(pmd: pmd_t) -> pmd_t {
    pmd_t(pmd.0 | _PAGE_DIRTY | _PAGE_SOFT_DIRTY)
}
#[inline]
pub fn pmd_mkyoung(pmd: pmd_t) -> pmd_t {
    pmd_t(pmd.0 | _PAGE_ACCESSED)
}
#[inline]
pub fn pmd_mkwrite(pmd: pmd_t) -> pmd_t {
    pmd_t(pmd.0 | _PAGE_RW)
}
#[inline]
pub fn pmd_wrprotect(pmd: pmd_t) -> pmd_t {
    pmd_t(pmd.0 & !_PAGE_RW)
}
#[inline]
pub fn pmd_mkhuge(pmd: pmd_t) -> pmd_t {
    pmd_t(pmd.0 | _PAGE_PSE)
}
#[inline]
pub fn pmd_clear(pmdp: *mut pmd_t) {
    unsafe { (*pmdp).0 = 0 };
}

// ---------------------------------------------------------------------------
// `ptep_*` clear / wrprotect helpers — Linux uses these to mutate live PTEs
// while keeping the dirty / young bits coherent.  Signatures match
// `arch/x86/include/asm/pgtable.h` so callers from C-style code paths line up.
//
// `_mm` and `_addr` are present to match the Linux signature; we don't use
// them yet (no mm_struct, no per-page TLB invalidation) but keeping the args
// avoids churn when M11/M12 land.
// ---------------------------------------------------------------------------

#[inline]
pub fn ptep_get(ptep: *const pte_t) -> pte_t {
    unsafe { *ptep }
}

/// Linux `pte_clear(mm, addr, ptep)` — write a zero entry, no TLB flush
/// (callers do that themselves via `flush_tlb_*`).
#[inline]
pub fn pte_clear(_mm: *mut (), _addr: u64, ptep: *mut pte_t) {
    unsafe { (*ptep).0 = 0 };
}

/// Linux `ptep_get_and_clear` — atomic read-and-clear of a PTE word.
#[inline]
pub fn ptep_get_and_clear(_mm: *mut (), _addr: u64, ptep: *mut pte_t) -> pte_t {
    unsafe {
        let old = *ptep;
        (*ptep).0 = 0;
        old
    }
}

/// Linux `ptep_set_wrprotect` — clear `_PAGE_RW` on a live PTE.
#[inline]
pub fn ptep_set_wrprotect(_mm: *mut (), _addr: u64, ptep: *mut pte_t) {
    unsafe {
        (*ptep).0 &= !_PAGE_RW;
    }
}

/// Linux `ptep_test_and_clear_young` — clear `_PAGE_ACCESSED`, return prior.
#[inline]
pub fn ptep_test_and_clear_young(_mm: *mut (), _addr: u64, ptep: *mut pte_t) -> bool {
    unsafe {
        let was_young = ((*ptep).0 & _PAGE_ACCESSED) != 0;
        (*ptep).0 &= !_PAGE_ACCESSED;
        was_young
    }
}

/// Linux `set_pte_at(mm, addr, ptep, pte)` — `mm` and `addr` are recorded
/// for arches that need them; on x86 we just write the word.
#[inline]
pub unsafe fn set_pte_at(_mm: *mut (), _addr: u64, ptep: *mut pte_t, pte: pte_t) {
    unsafe { *ptep = pte };
}

// ---------------------------------------------------------------------------
// set_* — write a page-table entry word.
//
// Linux does these as barrier-less writes on x86 (the CPU serialises them).
// Ref: Linux `set_pte`, `set_pmd`, etc. — arch/x86/include/asm/pgtable.h
// ---------------------------------------------------------------------------

#[inline]
pub unsafe fn set_pte(ptep: *mut pte_t, pte: pte_t) {
    unsafe { *ptep = pte }
}
#[inline]
pub unsafe fn set_pmd(pmdp: *mut pmd_t, pmd: pmd_t) {
    unsafe { *pmdp = pmd }
}
#[inline]
pub unsafe fn set_pud(pudp: *mut pud_t, pud: pud_t) {
    unsafe { *pudp = pud }
}
#[inline]
pub unsafe fn set_pgd(pgdp: *mut pgd_t, pgd: pgd_t) {
    unsafe { *pgdp = pgd }
}

// ---------------------------------------------------------------------------
// pXd_index — index of the address in a page-table level.
//
// Ref: Linux `include/linux/pgtable.h:48-71`
// ---------------------------------------------------------------------------

/// Index into the PGD (PML4) for `addr`.
#[inline]
pub fn pgd_index(addr: u64) -> usize {
    ((addr >> PGDIR_SHIFT) & (PTRS_PER_PGD as u64 - 1)) as usize
}

/// Index into the P4D for `addr`.  Always 0 on 4-level x86_64 because the
/// P4D level is folded — kept for ABI parity with the LA57 build.
///
/// Ref: `vendor/linux/include/asm-generic/pgtable-nop4d.h`
#[inline]
pub fn p4d_index(_addr: u64) -> usize {
    0
}

/// Index into the PUD for `addr`.
#[inline]
pub fn pud_index(addr: u64) -> usize {
    ((addr >> PUD_SHIFT) & (PTRS_PER_PUD as u64 - 1)) as usize
}

/// Index into the PMD for `addr`.
#[inline]
pub fn pmd_index(addr: u64) -> usize {
    ((addr >> PMD_SHIFT) & (PTRS_PER_PMD as u64 - 1)) as usize
}

/// Index into the PTE table for `addr`.
#[inline]
pub fn pte_index(addr: u64) -> usize {
    ((addr >> PAGE_SHIFT) & (PTRS_PER_PTE as u64 - 1)) as usize
}

// ---------------------------------------------------------------------------
// pXd_offset — pointer to the entry that covers `addr` in a table.
//
// On x86-64 with identity mapping, the physical table base address stored
// in a page-table entry is also a valid virtual address.
//
// Ref: Linux `pgd_offset_pgd`, `pud_offset`, `pmd_offset`, `pte_offset_kernel`
//      include/linux/pgtable.h:96-128
// ---------------------------------------------------------------------------

/// Return a pointer to the PGD entry covering `addr` in the table at `pgd`.
///
/// Ref: Linux `pgd_offset_pgd()` — `include/linux/pgtable.h:140`
#[inline]
pub unsafe fn pgd_offset_pgd(pgd: *mut pgd_t, addr: u64) -> *mut pgd_t {
    unsafe { pgd.add(pgd_index(addr)) }
}

/// Return a pointer to the P4D entry covering `addr` — folded into the PGD.
///
/// On 4-level x86_64 (`PTRS_PER_P4D == 1`) the P4D level "is" the PGD entry,
/// so this is a transparent pointer cast.  Mirrors how
/// `include/asm-generic/pgtable-nop4d.h` defines `p4d_offset(pgd, addr)`
/// when `__PAGETABLE_P4D_FOLDED` is set.
///
/// # Safety
/// `pgdp` must point to a valid PGD entry word.
#[inline]
pub unsafe fn p4d_offset(pgdp: *mut pgd_t, _addr: u64) -> *mut p4d_t {
    pgdp as *mut p4d_t
}

/// Return a pointer to the PUD entry covering `addr`.
///
/// On 4-level x86_64 the PUD table base is encoded in the PGD entry (the
/// folded P4D forwards directly to PGD), so we read through `p4dp`.
///
/// Ref: Linux `pud_offset()` — `include/linux/pgtable.h:133`
#[inline]
pub unsafe fn pud_offset(p4dp: *mut p4d_t, addr: u64) -> *mut pud_t {
    unsafe {
        let val = (*p4dp).0;
        let pud_base = entry_base_ptr(val) as *mut pud_t;
        pud_base.add(pud_index(addr))
    }
}

/// Return a pointer to the PMD entry covering `addr`.
///
/// The PMD table base is encoded in the PUD entry (`*pudp`).
///
/// Ref: Linux `pmd_offset()` — `include/linux/pgtable.h:125`
#[inline]
pub unsafe fn pmd_offset(pudp: *mut pud_t, addr: u64) -> *mut pmd_t {
    unsafe {
        let pud_val = (*pudp).0;
        let pmd_base = entry_base_ptr(pud_val) as *mut pmd_t;
        pmd_base.add(pmd_index(addr))
    }
}

/// Return a pointer to the PTE entry covering `addr`.
///
/// Ref: Linux `pte_offset_kernel()` — `include/linux/pgtable.h:96`
#[inline]
pub unsafe fn pte_offset_kernel(pmdp: *mut pmd_t, addr: u64) -> *mut pte_t {
    unsafe {
        let pmd_val = (*pmdp).0;
        let pte_base = entry_base_ptr(pmd_val) as *mut pte_t;
        pte_base.add(pte_index(addr))
    }
}

// ---------------------------------------------------------------------------
// CR3 — read the current kernel PGD base address.
// ---------------------------------------------------------------------------

/// Read CR3 and return the physical base address of the current PML4 table.
///
/// On our identity-mapped kernel, this physical address equals its virtual
/// address, so it can be used directly as a pointer.
///
/// # Safety
/// Must run in ring 0 on real hardware. Not available in the host test runner.
///
/// Ref: Intel SDM Vol. 3A §2.5 — Control Registers
#[cfg(not(test))]
#[inline]
pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {0}, cr3",
            out(reg) cr3,
            options(nomem, nostack, preserves_flags),
        );
    }
    cr3 & PTE_PFN_MASK
}

#[cfg(not(test))]
static INIT_PGD_PHYS: AtomicU64 = AtomicU64::new(0);

/// Remember the kernel's bootstrap PGD, mirroring Linux's stable
/// `init_mm.pgd`. New user address spaces must copy kernel mappings from this
/// root, not from whichever user mm currently happens to be loaded in CR3.
#[cfg(not(test))]
pub fn record_init_pgd_from_current_cr3() -> u64 {
    let cr3 = read_cr3();
    match INIT_PGD_PHYS.compare_exchange(0, cr3, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => cr3,
        Err(existing) => existing,
    }
}

#[cfg(not(test))]
pub fn init_pgd_phys() -> u64 {
    let saved = INIT_PGD_PHYS.load(Ordering::Acquire);
    if saved != 0 {
        saved
    } else {
        record_init_pgd_from_current_cr3()
    }
}

// ---------------------------------------------------------------------------
// Kernel PGD accessor
// ---------------------------------------------------------------------------

/// Return a pointer to the first entry of the kernel's PGD table.
///
/// In production this is `read_cr3()`; in tests it is the mock PML4.
///
/// Conceptually mirrors Linux's `init_mm.pgd` (used by `pgd_offset_k`).
///
/// Ref: Linux `pgd_offset_k(addr)` = `pgd_offset(&init_mm, addr)`
///      `include/linux/pgtable.h:156`
#[cfg(not(test))]
fn init_pgd() -> *mut pgd_t {
    phys_to_virt(init_pgd_phys()) as *mut pgd_t
}

#[cfg(test)]
fn init_pgd() -> *mut pgd_t {
    unsafe { test_pool::pml4_base() as *mut pgd_t }
}

/// Public wrapper around the host-only mock PML4 base, exposed so the
/// `memory::pagewalk` host tests can drive the walker without making
/// `init_pgd` and `test_pool` `pub`.
#[cfg(test)]
pub fn init_pgd_for_test() -> *mut pgd_t {
    init_pgd()
}

/// Public wrapper around `test_pool::reset` for the same reason.
///
/// # Safety
/// Same as `test_pool::reset`: the global test pool is not reentrant —
/// callers must hold the page-table test lock.
#[cfg(test)]
pub unsafe fn reset_test_pool() {
    unsafe { test_pool::reset() };
}

#[inline]
fn pgd_phys_to_ptr(pgd_phys: u64) -> *mut pgd_t {
    entry_base_ptr(pgd_phys) as *mut pgd_t
}

/// Allocate an empty PGD root for Linux-style temporary kernel address spaces.
///
/// This is used by EFI runtime calls to mirror Linux's dedicated `efi_mm.pgd`
/// rather than inserting firmware mappings into the normal kernel root.
pub unsafe fn alloc_kernel_page_table_root() -> Option<u64> {
    unsafe {
        alloc_pt_page(PtPageTraceContext::from_indices(
            "alloc_kernel_page_table_root",
            "pgd_root",
            0,
            0,
            0,
        ))
    }
}

/// Synchronize kernel mappings into a temporary PGD while preserving one
/// excluded virtual window.
///
/// Mirrors the intent of Linux `efi_sync_low_kernel_mappings()`: share normal
/// kernel mappings with `efi_mm`, but leave the EFI runtime VA window owned by
/// the temporary root. The excluded window is PUD-granular, matching the
/// `BUILD_BUG_ON()` alignment checks in Linux's x86 EFI code.
///
/// Returns the number of top-level/table entries copied.
///
/// # Safety
/// `dst_pgd_phys` must be a valid, private PGD root allocated for a temporary
/// address space. The caller must serialize updates against users of that root.
pub unsafe fn sync_kernel_mappings_around_window(
    dst_pgd_phys: u64,
    exclude_start: u64,
    exclude_end: u64,
) -> Option<usize> {
    if exclude_start >= exclude_end
        || exclude_start & (PUD_SIZE - 1) != 0
        || exclude_end & (PUD_SIZE - 1) != 0
    {
        return None;
    }

    let dst_pgd = pgd_phys_to_ptr(dst_pgd_phys);
    let src_pgd = init_pgd();
    let start_pgd = pgd_index(exclude_start);
    let end_pgd = pgd_index(exclude_end - 1);
    let mut copied = 0usize;

    unsafe {
        for idx in 0..PTRS_PER_PGD {
            let src_entry = *src_pgd.add(idx);
            let dst_entry = dst_pgd.add(idx);
            if idx < start_pgd || idx > end_pgd {
                *dst_entry = src_entry;
                copied += 1;
                continue;
            }

            if pgd_none(src_entry) {
                continue;
            }

            if pgd_none(*dst_entry) {
                let new_phys = alloc_pt_page(PtPageTraceContext::from_indices(
                    "sync_kernel_mappings_around_window",
                    "pud",
                    idx,
                    0,
                    0,
                ))?;
                set_pgd(dst_entry, __pgd(new_phys | (src_entry.0 & !PTE_PFN_MASK)));
            }

            let src_pud = entry_base_ptr(src_entry.0) as *const pud_t;
            let dst_pud = entry_base_ptr((*dst_entry).0) as *mut pud_t;
            let exclude_pud_start = if idx == start_pgd {
                pud_index(exclude_start)
            } else {
                0
            };
            let exclude_pud_end = if idx == end_pgd {
                pud_index(exclude_end - 1) + 1
            } else {
                PTRS_PER_PUD
            };

            for pud_idx in 0..PTRS_PER_PUD {
                if pud_idx >= exclude_pud_start && pud_idx < exclude_pud_end {
                    continue;
                }
                *dst_pud.add(pud_idx) = *src_pud.add(pud_idx);
                copied += 1;
            }
        }
    }

    Some(copied)
}

/// Map one kernel page into an explicit PGD root.
///
/// This is the non-panicking sibling of `map_kernel_page()` used by temporary
/// roots such as EFI's `efi_mm`.
pub unsafe fn map_kernel_page_in_pgd(
    pgd_phys: u64,
    addr: u64,
    phys_addr: u64,
    prot: pgprot_t,
) -> Option<()> {
    if phys_addr & (PAGE_SIZE - 1) != 0 {
        return None;
    }

    unsafe {
        let pgdp = pgd_offset_pgd(pgd_phys_to_ptr(pgd_phys), addr);
        let pudp = pud_alloc_kernel(pgdp, addr)?;
        let pmdp = pmd_alloc_kernel(pudp, addr)?;
        let ptep = pte_alloc_kernel(pmdp, addr)?;
        set_pte(ptep, pfn_pte(phys_addr >> PAGE_SHIFT, prot));
    }
    Some(())
}

/// Translate `virt` through an explicit PGD root.
pub fn virt_to_phys_in_pgd(pgd_phys: u64, virt: u64) -> Option<u64> {
    unsafe {
        let pgdp = pgd_offset_pgd(pgd_phys_to_ptr(pgd_phys), virt);
        let pgd = *pgdp;
        if pgd_none(pgd) {
            return None;
        }

        let p4dp = p4d_offset(pgdp, virt);
        let pudp = pud_offset(p4dp, virt);
        let pud = *pudp;
        if pud_none(pud) {
            return None;
        }
        if pud_huge(pud) {
            return Some((pud.0 & PTE_PFN_MASK) + (virt & (PUD_SIZE - 1)));
        }

        let pmdp = pmd_offset(pudp, virt);
        let pmd = *pmdp;
        if pmd_none(pmd) {
            return None;
        }
        if pmd_huge(pmd) {
            return Some((pmd.0 & PTE_PFN_MASK) + (virt & (PMD_SIZE - 1)));
        }

        let ptep = pte_offset_kernel(pmdp, virt);
        let pte = *ptep;
        if pte_none(pte) {
            return None;
        }
        Some(pte_phys(pte) | (virt & !PAGE_MASK))
    }
}

#[cfg(not(test))]
#[inline]
pub unsafe fn use_temporary_kernel_pgd(pgd_phys: u64) -> u64 {
    let previous = read_cr3();
    unsafe {
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) pgd_phys & PTE_PFN_MASK,
            options(nostack, preserves_flags),
        );
    }
    previous
}

#[cfg(test)]
#[inline]
pub unsafe fn use_temporary_kernel_pgd(_pgd_phys: u64) -> u64 {
    0
}

#[cfg(not(test))]
#[inline]
pub unsafe fn unuse_temporary_kernel_pgd(previous_pgd_phys: u64) {
    unsafe {
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) previous_pgd_phys & PTE_PFN_MASK,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
pub unsafe fn unuse_temporary_kernel_pgd(_previous_pgd_phys: u64) {}

// ---------------------------------------------------------------------------
// Physical page allocator for page-table pages
// ---------------------------------------------------------------------------

/// Allocate one 4 KiB zeroed page for use as a page-table level.
///
/// Returns the physical (= virtual, identity-mapped) base address of the
/// fresh page.  The page is tagged `PGTY_TABLE` in the buddy's mem_map.
///
/// Equivalent to Linux `pte_alloc_one_kernel` / `pud_alloc_one` etc.
///
/// Ref: Linux `mm/memory.c` — `__pte_alloc_kernel`, `__pmd_alloc`, `__pud_alloc`
#[derive(Clone, Copy)]
struct PtPageTraceContext {
    reason: &'static str,
    level: &'static str,
    addr: u64,
    pgd_idx: usize,
    pud_idx: usize,
    pmd_idx: usize,
}

impl PtPageTraceContext {
    fn from_addr(reason: &'static str, level: &'static str, addr: u64) -> Self {
        Self {
            reason,
            level,
            addr,
            pgd_idx: pgd_index(addr),
            pud_idx: pud_index(addr),
            pmd_idx: pmd_index(addr),
        }
    }

    fn from_indices(
        reason: &'static str,
        level: &'static str,
        pgd_idx: usize,
        pud_idx: usize,
        pmd_idx: usize,
    ) -> Self {
        let addr = ((pgd_idx as u64) << PGDIR_SHIFT)
            | ((pud_idx as u64) << PUD_SHIFT)
            | ((pmd_idx as u64) << PMD_SHIFT);
        Self {
            reason,
            level,
            addr,
            pgd_idx,
            pud_idx,
            pmd_idx,
        }
    }
}

#[cfg(not(test))]
const PT_PAGE_LIFE_TRACE_LEN: usize = 256;
#[cfg(not(test))]
const PT_PAGE_LIFE_DUMP_RECENT: u64 = 48;
#[cfg(not(test))]
const PT_PAGE_LIFE_DUMP_MATCH_LIMIT: usize = 16;
#[cfg(not(test))]
const PT_PAGE_LIFE_DUMP_LIMIT: u64 = 8;

#[cfg(not(test))]
#[derive(Clone, Copy)]
enum PtPageLifeOp {
    Empty,
    Alloc,
    Free,
}

#[cfg(not(test))]
impl PtPageLifeOp {
    fn as_str(self) -> &'static str {
        match self {
            PtPageLifeOp::Empty => "empty",
            PtPageLifeOp::Alloc => "alloc",
            PtPageLifeOp::Free => "free",
        }
    }
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
struct PtPageLifeEvent {
    seq: u64,
    op: PtPageLifeOp,
    reason: &'static str,
    level: &'static str,
    addr: u64,
    pgd_idx: usize,
    pud_idx: usize,
    pmd_idx: usize,
    phys: u64,
    pfn: usize,
    page: usize,
    page_type: u32,
}

#[cfg(not(test))]
impl PtPageLifeEvent {
    const fn empty() -> Self {
        Self {
            seq: 0,
            op: PtPageLifeOp::Empty,
            reason: "",
            level: "",
            addr: 0,
            pgd_idx: 0,
            pud_idx: 0,
            pmd_idx: 0,
            phys: 0,
            pfn: 0,
            page: 0,
            page_type: 0,
        }
    }
}

#[cfg(not(test))]
struct PtPageLifeTrace {
    next_seq: u64,
    events: [PtPageLifeEvent; PT_PAGE_LIFE_TRACE_LEN],
}

#[cfg(not(test))]
impl PtPageLifeTrace {
    const fn new() -> Self {
        Self {
            next_seq: 0,
            events: [PtPageLifeEvent::empty(); PT_PAGE_LIFE_TRACE_LEN],
        }
    }

    fn record(&mut self, mut event: PtPageLifeEvent) {
        let seq = self.next_seq.wrapping_add(1);
        self.next_seq = seq;
        event.seq = seq;
        self.events[(seq.wrapping_sub(1) as usize) % PT_PAGE_LIFE_TRACE_LEN] = event;
    }

    fn event_for_seq(&self, seq: u64) -> Option<PtPageLifeEvent> {
        if seq == 0 || seq > self.next_seq {
            return None;
        }
        let event = self.events[(seq.wrapping_sub(1) as usize) % PT_PAGE_LIFE_TRACE_LEN];
        (event.seq == seq).then_some(event)
    }
}

#[cfg(not(test))]
static PT_PAGE_LIFE_TRACE: Mutex<PtPageLifeTrace> = Mutex::new(PtPageLifeTrace::new());
#[cfg(not(test))]
static PT_PAGE_LIFE_DUMPS: AtomicU64 = AtomicU64::new(0);

#[cfg(not(test))]
fn record_pt_page_alloc(
    phys: u64,
    page: *const crate::mm::page::Page,
    page_type: u32,
    ctx: PtPageTraceContext,
) {
    PT_PAGE_LIFE_TRACE.lock().record(PtPageLifeEvent {
        seq: 0,
        op: PtPageLifeOp::Alloc,
        reason: ctx.reason,
        level: ctx.level,
        addr: ctx.addr,
        pgd_idx: ctx.pgd_idx,
        pud_idx: ctx.pud_idx,
        pmd_idx: ctx.pmd_idx,
        phys,
        pfn: (phys >> PAGE_SHIFT) as usize,
        page: page as usize,
        page_type,
    });
}

#[cfg(not(test))]
pub fn record_pt_page_free(
    phys: u64,
    page: *const crate::mm::page::Page,
    page_type: u32,
    level: &'static str,
    pgd_idx: usize,
    pud_idx: usize,
    pmd_idx: usize,
) {
    PT_PAGE_LIFE_TRACE.lock().record(PtPageLifeEvent {
        seq: 0,
        op: PtPageLifeOp::Free,
        reason: "free_user_page_tables",
        level,
        addr: ((pgd_idx as u64) << PGDIR_SHIFT)
            | ((pud_idx as u64) << PUD_SHIFT)
            | ((pmd_idx as u64) << PMD_SHIFT),
        pgd_idx,
        pud_idx,
        pmd_idx,
        phys,
        pfn: (phys >> PAGE_SHIFT) as usize,
        page: page as usize,
        page_type,
    });
}

#[cfg(test)]
pub fn record_pt_page_free(
    _phys: u64,
    _page: *const crate::mm::page::Page,
    _page_type: u32,
    _level: &'static str,
    _pgd_idx: usize,
    _pud_idx: usize,
    _pmd_idx: usize,
) {
}

#[cfg(not(test))]
fn pt_page_life_target_match(event_phys: u64, target_phys: u64) -> bool {
    target_phys != u64::MAX
        && (event_phys == target_phys
            || (event_phys & 0xffff_ffff_f000) == (target_phys & 0xffff_ffff_f000)
            || (event_phys & 0xffff_ffff) == (target_phys & 0xffff_ffff))
}

#[cfg(not(test))]
fn log_pt_page_life_event(prefix: &str, target_phys: u64, event: PtPageLifeEvent) {
    crate::kernel::printk::log_error!(
        "mm",
        "pt_page_life {} seq={} op={} reason={} level={} addr={:#018x} pgd={} pud={} pmd={} phys={:#018x} pfn={:#x} page={:#x} page_type={:#x} target_match={}",
        prefix,
        event.seq,
        event.op.as_str(),
        event.reason,
        event.level,
        event.addr,
        event.pgd_idx,
        event.pud_idx,
        event.pmd_idx,
        event.phys,
        event.pfn,
        event.page,
        event.page_type,
        pt_page_life_target_match(event.phys, target_phys)
    );
}

#[cfg(not(test))]
pub fn dump_pt_page_life_trace(reason: &'static str, target_phys: u64) {
    let dump_nr = PT_PAGE_LIFE_DUMPS.fetch_add(1, Ordering::Relaxed);
    if dump_nr >= PT_PAGE_LIFE_DUMP_LIMIT {
        return;
    }

    let trace = PT_PAGE_LIFE_TRACE.lock();
    let total = trace.next_seq;
    crate::kernel::printk::log_error!(
        "mm",
        "pt_page_life dump reason={} target_phys={:#018x} dump={} total_seq={}",
        reason,
        target_phys,
        dump_nr + 1,
        total
    );

    if total == 0 {
        return;
    }

    let earliest = total
        .saturating_sub(PT_PAGE_LIFE_TRACE_LEN as u64)
        .saturating_add(1);
    let mut matches = 0usize;
    let mut seq = earliest;
    while seq <= total && matches < PT_PAGE_LIFE_DUMP_MATCH_LIMIT {
        if let Some(event) = trace.event_for_seq(seq)
            && pt_page_life_target_match(event.phys, target_phys)
        {
            log_pt_page_life_event("match", target_phys, event);
            matches += 1;
        }
        seq += 1;
    }
    if matches == 0 {
        crate::kernel::printk::log_error!(
            "mm",
            "pt_page_life no matching phys records for target={:#018x}",
            target_phys
        );
    }

    let recent = core::cmp::min(
        PT_PAGE_LIFE_DUMP_RECENT,
        core::cmp::min(total, PT_PAGE_LIFE_TRACE_LEN as u64),
    );
    let mut seq = total.saturating_sub(recent).saturating_add(1);
    while seq <= total {
        if let Some(event) = trace.event_for_seq(seq) {
            log_pt_page_life_event("recent", target_phys, event);
        }
        seq += 1;
    }
}

#[cfg(test)]
pub fn dump_pt_page_life_trace(_reason: &'static str, _target_phys: u64) {}

#[cfg(not(test))]
unsafe fn alloc_pt_page(ctx: PtPageTraceContext) -> Option<u64> {
    use core::sync::atomic::Ordering;

    use crate::mm::buddy::{page_to_pfn, with_global_buddy};
    use crate::mm::frame::PAGE_SIZE;
    use crate::mm::page_flags::{GFP_KERNEL, PGTY_TABLE, encode_page_type};

    let page_ptr = with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL))?;
    let pfn = page_to_pfn(page_ptr);
    let phys = (pfn * PAGE_SIZE) as u64;
    unsafe extern "C" {
        static _kernel_phys_start: u8;
        static _kernel_phys_end: u8;
    }
    let kernel_phys_start = unsafe { &_kernel_phys_start as *const u8 as u64 };
    let kernel_phys_end = unsafe { &_kernel_phys_end as *const u8 as u64 };
    if phys >= kernel_phys_start && phys < kernel_phys_end {
        crate::kernel::printk::log_error!(
            "mm",
            "alloc_pt_page: buddy returned kernel image page phys={:#x} kernel={:#x}..{:#x}",
            phys,
            kernel_phys_start,
            kernel_phys_end
        );
        return None;
    }

    unsafe {
        // Zero the page (required — pXd_none checks for raw == 0).
        core::ptr::write_bytes(phys_to_virt(phys), 0, PAGE_SIZE);
        // Tag the page so the buddy won't double-free it.
        let page_type = encode_page_type(PGTY_TABLE);
        (*page_ptr).page_type.store(page_type, Ordering::Relaxed);
        record_pt_page_alloc(phys, page_ptr, page_type, ctx);
    }
    Some(phys)
}

#[cfg(test)]
unsafe fn alloc_pt_page(_ctx: PtPageTraceContext) -> Option<u64> {
    unsafe { test_pool::alloc() }
}

#[cfg(not(test))]
fn kernel_phys_bounds() -> Option<(u64, u64)> {
    unsafe extern "C" {
        static _kernel_phys_start: u8;
        static _kernel_phys_end: u8;
    }

    let start = unsafe { &_kernel_phys_start as *const u8 as u64 };
    let end = unsafe { &_kernel_phys_end as *const u8 as u64 };
    Some((start, end))
}

#[cfg(test)]
fn kernel_phys_bounds() -> Option<(u64, u64)> {
    None
}

// ---------------------------------------------------------------------------
// pXd_alloc_kernel — allocate & install intermediate table if absent.
//
// These mirror Linux's `__pud_alloc`, `__pmd_alloc`, `__pte_alloc_kernel`.
// Ref: Linux `mm/memory.c`
// ---------------------------------------------------------------------------

/// If the PGD entry `*pgdp` is absent, allocate a new PUD page and install
/// it; then return a pointer to the PUD entry covering `addr`.
///
/// # Safety
/// `pgdp` must point to a valid PGD entry word.
///
/// Ref: Linux `__pud_alloc()` — `mm/memory.c`
pub unsafe fn pud_alloc_kernel(pgdp: *mut pgd_t, addr: u64) -> Option<*mut pud_t> {
    unsafe {
        if pgd_none(*pgdp) {
            let new_phys = alloc_pt_page(PtPageTraceContext::from_addr(
                "pud_alloc_kernel",
                "pud",
                addr,
            ))?;
            set_pgd(pgdp, __pgd(new_phys | _KERNPG_TABLE));
        }
        let p4dp = p4d_offset(pgdp, addr);
        Some(pud_offset(p4dp, addr))
    }
}

/// If the PUD entry `*pudp` is absent, allocate a new PMD page and install
/// it; then return a pointer to the PMD entry covering `addr`.
///
/// Ref: Linux `__pmd_alloc()` — `mm/memory.c`
pub unsafe fn pmd_alloc_kernel(pudp: *mut pud_t, addr: u64) -> Option<*mut pmd_t> {
    unsafe {
        if pud_none(*pudp) {
            let new_phys = alloc_pt_page(PtPageTraceContext::from_addr(
                "pmd_alloc_kernel",
                "pmd",
                addr,
            ))?;
            set_pud(pudp, __pud(new_phys | _KERNPG_TABLE));
        }
        Some(pmd_offset(pudp, addr))
    }
}

/// If the PMD entry `*pmdp` is absent, allocate a new PT page and install
/// it; then return a pointer to the PTE covering `addr`.
///
/// Ref: Linux `__pte_alloc_kernel()` — `mm/memory.c`
pub unsafe fn pte_alloc_kernel(pmdp: *mut pmd_t, addr: u64) -> Option<*mut pte_t> {
    unsafe {
        if pmd_none(*pmdp) {
            let new_phys = alloc_pt_page(PtPageTraceContext::from_addr(
                "pte_alloc_kernel",
                "pte",
                addr,
            ))?;
            set_pmd(pmdp, __pmd(new_phys | _KERNPG_TABLE));
        }
        Some(pte_offset_kernel(pmdp, addr))
    }
}

// ---------------------------------------------------------------------------
// pXd_alloc — allocate & install intermediate table with caller-provided flags.
//
// These are the user-space counterparts of `pXd_alloc_kernel`.  The caller
// passes `_PAGE_TABLE` for user VMAs or `_KERNPG_TABLE` for kernel mappings.
//
// Ref: Linux `mm/memory.c` — `__pud_alloc`, `__pmd_alloc`, `__pte_alloc`
// ---------------------------------------------------------------------------

/// If the PGD entry `*pgdp` is absent, allocate a new PUD page and install
/// it with `table_flags`; then return a pointer to the PUD entry covering `addr`.
///
/// For user VMAs, pass `_PAGE_TABLE` (which includes `_PAGE_USER`).
///
/// # Safety
/// `pgdp` must point to a valid PGD entry word.
///
/// Ref: Linux `__pud_alloc()` — `mm/memory.c`
pub unsafe fn pud_alloc(pgdp: *mut pgd_t, addr: u64, table_flags: u64) -> Option<*mut pud_t> {
    unsafe {
        if pgd_none(*pgdp) {
            let new_phys = alloc_pt_page(PtPageTraceContext::from_addr("pud_alloc", "pud", addr))?;
            set_pgd(pgdp, __pgd(new_phys | table_flags));
        } else {
            (*pgdp).0 |= table_flags;
        }
        let p4dp = p4d_offset(pgdp, addr);
        Some(pud_offset(p4dp, addr))
    }
}

/// If the PUD entry `*pudp` is absent, allocate a new PMD page and install
/// it with `table_flags`; then return a pointer to the PMD entry covering `addr`.
///
/// Ref: Linux `__pmd_alloc()` — `mm/memory.c`
pub unsafe fn pmd_alloc(pudp: *mut pud_t, addr: u64, table_flags: u64) -> Option<*mut pmd_t> {
    unsafe {
        if pud_none(*pudp) || pud_huge(*pudp) {
            let new_phys = alloc_pt_page(PtPageTraceContext::from_addr("pmd_alloc", "pmd", addr))?;
            set_pud(pudp, __pud(new_phys | table_flags));
        } else {
            (*pudp).0 |= table_flags;
        }
        Some(pmd_offset(pudp, addr))
    }
}

/// If the PMD entry `*pmdp` is absent, allocate a new PT page and install
/// it with `table_flags`; then return a pointer to the PTE covering `addr`.
///
/// Ref: Linux `__pte_alloc()` — `mm/memory.c`
pub unsafe fn pte_alloc(pmdp: *mut pmd_t, addr: u64, table_flags: u64) -> Option<*mut pte_t> {
    unsafe {
        if pmd_none(*pmdp) || pmd_huge(*pmdp) {
            let new_phys = alloc_pt_page(PtPageTraceContext::from_addr("pte_alloc", "pte", addr))?;
            set_pmd(pmdp, __pmd(new_phys | table_flags));
        } else {
            (*pmdp).0 |= table_flags;
        }
        Some(pte_offset_kernel(pmdp, addr))
    }
}

/// Clone PGD slot 0 for a process page table without sharing the boot-time
/// identity-map lower tables.
///
/// Lupos still executes some kernel paths through the low identity mapping, so
/// process PGDs must retain slot 0 until the kernel is fully higher-half-only.
/// The boot PGD's slot 0 lower tables are shared with the direct-map window,
/// though.  If a user VMA below 512 GiB faults through those shared tables,
/// `pte_alloc()` may split a huge PMD and accidentally punch a hole in the
/// direct map.  Linux avoids this class by keeping kernel/user address spaces
/// disjoint; this transitional helper gives Lupos the same isolation property
/// for the copied identity slot.
///
/// The caller should copy the top-level kernel entries first, then call this
/// for `dst[0]`.
///
/// # Safety
/// `dst_pgd0` must point at the destination PGD's first entry and `src_pgd0`
/// at the saved init PGD's first entry.  The destination PGD must not be active
/// on another CPU while this runs.
pub unsafe fn clone_low_identity_pgd_slot_for_user(
    dst_pgd0: *mut pgd_t,
    src_pgd0: *const pgd_t,
) -> Option<()> {
    unsafe {
        let src_pgd = *src_pgd0;
        if pgd_none(src_pgd) {
            *dst_pgd0 = pgd_t(0);
            return Some(());
        }

        let new_pud_phys = alloc_pt_page(PtPageTraceContext::from_indices(
            "clone_low_identity",
            "pud",
            0,
            0,
            0,
        ))?;
        let new_pud = entry_base_ptr(new_pud_phys) as *mut pud_t;
        let src_pud = entry_base_ptr(src_pgd.0) as *const pud_t;

        for idx in 0..PTRS_PER_PUD {
            let src_entry = *src_pud.add(idx);
            if pud_none(src_entry) {
                *new_pud.add(idx) = src_entry;
                continue;
            }

            if pud_huge(src_entry) {
                *new_pud.add(idx) = src_entry;
                continue;
            }

            let new_pmd_phys = alloc_pt_page(PtPageTraceContext::from_indices(
                "clone_low_identity",
                "pmd",
                0,
                idx,
                0,
            ))?;
            let new_pmd = entry_base_ptr(new_pmd_phys) as *mut pmd_t;
            let src_pmd = entry_base_ptr(src_entry.0) as *const pmd_t;
            let mut pmd_idx = 0usize;
            while pmd_idx < PTRS_PER_PMD {
                let src_pmd_entry = *src_pmd.add(pmd_idx);
                if pmd_none(src_pmd_entry) || pmd_huge(src_pmd_entry) {
                    *new_pmd.add(pmd_idx) = src_pmd_entry;
                    pmd_idx += 1;
                    continue;
                }

                let new_pte_phys = alloc_pt_page(PtPageTraceContext::from_indices(
                    "clone_low_identity",
                    "pte",
                    0,
                    idx,
                    pmd_idx,
                ))?;
                let new_pte = entry_base_ptr(new_pte_phys) as *mut pte_t;
                let src_pte = entry_base_ptr(src_pmd_entry.0) as *const pte_t;
                core::ptr::copy_nonoverlapping(src_pte, new_pte, PTRS_PER_PTE);
                *new_pmd.add(pmd_idx) =
                    pmd_t((new_pte_phys & PTE_PFN_MASK) | (src_pmd_entry.0 & !PTE_PFN_MASK));
                pmd_idx += 1;
            }
            *new_pud.add(idx) =
                pud_t((new_pmd_phys & PTE_PFN_MASK) | (src_entry.0 & !PTE_PFN_MASK));
        }

        *dst_pgd0 = pgd_t((new_pud_phys & PTE_PFN_MASK) | (src_pgd.0 & !PTE_PFN_MASK));
        Some(())
    }
}

// ---------------------------------------------------------------------------
// TLB invalidation helpers
// ---------------------------------------------------------------------------

/// Invalidate the TLB entry for a single virtual address on the local CPU.
///
/// Must be called after modifying a live PTE/PMD/PUD to ensure the CPU
/// picks up the new mapping.  For cross-CPU invalidation see `tlb.rs`.
///
/// Ref: Intel SDM Vol. 3A §4.10.4.1 — `INVLPG`
#[cfg(not(test))]
#[inline]
pub unsafe fn flush_tlb_page(addr: u64) {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        unsafe {
            core::arch::asm!(
                "invlpg [{0}]",
                in(reg) addr,
                options(nostack, preserves_flags),
            );
        }
        return;
    }
    let mm = unsafe {
        if !(*current).mm.is_null() {
            (*current).mm
        } else {
            (*current).active_mm
        }
    };
    if mm.is_null() {
        unsafe {
            core::arch::asm!(
                "invlpg [{0}]",
                in(reg) addr,
                options(nostack, preserves_flags),
            );
        }
    } else {
        let _ =
            unsafe { crate::arch::x86::mm::tlb::flush_tlb_mm_range(mm, addr, addr + PAGE_SIZE) };
    }
}

/// No-op in host test runner (no real TLB).
#[cfg(test)]
#[inline]
pub unsafe fn flush_tlb_page(_addr: u64) {}

/// Invalidate TLB entries for every page in `[start, end)` on the local CPU.
///
/// Loops `invlpg` over every page-aligned address in the range.  For small
/// ranges this is cheaper than a full CR3 reload; a future SMP milestone will
/// add cross-CPU shootdown via `flush_tlb_others`.
///
/// Ref: Linux `flush_tlb_range()` — `arch/x86/include/asm/tlbflush.h`
#[cfg(not(test))]
#[inline]
pub unsafe fn flush_tlb_range(start: u64, end: u64) {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        let mut addr = start & PAGE_MASK;
        while addr < end {
            unsafe {
                core::arch::asm!(
                    "invlpg [{0}]",
                    in(reg) addr,
                    options(nostack, preserves_flags),
                );
            }
            addr += PAGE_SIZE;
        }
        return;
    }
    let mm = unsafe {
        if !(*current).mm.is_null() {
            (*current).mm
        } else {
            (*current).active_mm
        }
    };
    if mm.is_null() {
        let mut addr = start & PAGE_MASK;
        while addr < end {
            unsafe {
                core::arch::asm!(
                    "invlpg [{0}]",
                    in(reg) addr,
                    options(nostack, preserves_flags),
                );
            }
            addr += PAGE_SIZE;
        }
    } else {
        let _ = unsafe { crate::arch::x86::mm::tlb::flush_tlb_mm_range(mm, start, end) };
    }
}

/// No-op in host test runner (no real TLB).
#[cfg(test)]
#[inline]
pub unsafe fn flush_tlb_range(_start: u64, _end: u64) {}

#[cfg(not(test))]
#[inline]
pub unsafe fn flush_tlb_all_local() {
    let cr3 = read_cr3();
    unsafe {
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) cr3,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
pub unsafe fn flush_tlb_all_local() {}

/// Map one physical frame into the reserved temporary kmap window.
///
/// The window is intentionally small and is meant for short-lived mapping
/// of a single page at a time.
pub unsafe fn kmap(frame: crate::mm::frame::PhysFrame) -> *mut u8 {
    let phys = frame.start_address();
    let addr = KMAP_START;
    unsafe { map_kernel_page(addr, phys, PAGE_KERNEL) };

    #[cfg(test)]
    {
        addr as *mut u8
    }

    #[cfg(not(test))]
    {
        // Keep the reserved kmap window installed, but fall back to the
        // permanent direct map if the boot environment doesn't expose the
        // temporary alias yet.
        if virt_to_phys(addr).is_some() {
            addr as *mut u8
        } else {
            phys_to_virt(phys)
        }
    }
}

/// Unmap a page previously mapped with `kmap`.
pub unsafe fn kunmap(addr: *mut u8) {
    if addr.is_null() {
        return;
    }
    if addr as u64 != KMAP_START {
        return;
    }
    unsafe { unmap_kernel_page(KMAP_START) };
}

// ---------------------------------------------------------------------------
// Public API — map / unmap / translate
// ---------------------------------------------------------------------------

/// Map a single kernel page: `addr` (virtual) → `phys_addr` with `prot`.
///
/// Walks or creates PGD→PUD→PMD→PT, then writes the leaf PTE.  New
/// intermediate table pages are allocated from the buddy allocator via
/// `alloc_pt_page()` and tagged `PGTY_TABLE`.
///
/// Equivalent to what Linux does in `vmap_pte_range` for each page during
/// vmalloc setup (`mm/vmalloc.c`).
///
/// # Panics
/// Panics if any intermediate table allocation fails (OOM).
///
/// # Safety
/// - `addr` must be a canonical virtual address in the kernel half.
/// - `phys_addr` must be 4 KiB aligned.
/// - The caller must flush the TLB if the mapping was previously present.
pub unsafe fn map_kernel_page(addr: u64, phys_addr: u64, prot: pgprot_t) {
    debug_assert_eq!(
        phys_addr & 0xFFF,
        0,
        "map_kernel_page: phys_addr must be page-aligned"
    );

    unsafe {
        let pgdp = pgd_offset_pgd(init_pgd(), addr);

        let pudp = pud_alloc_kernel(pgdp, addr).expect("map_kernel_page: OOM allocating PUD");
        let pmdp = pmd_alloc_kernel(pudp, addr).expect("map_kernel_page: OOM allocating PMD");
        let ptep = pte_alloc_kernel(pmdp, addr).expect("map_kernel_page: OOM allocating PT");

        set_pte(ptep, pfn_pte(phys_addr >> PAGE_SHIFT, prot));

        #[cfg(not(test))]
        core::arch::asm!(
            "invlpg [{0}]",
            in(reg) addr,
            options(nostack, preserves_flags),
        );
    }
}

/// Unmap a single kernel page at `addr` and flush the local TLB.
///
/// Clears the leaf PTE.  Does *not* free the backing physical frame —
/// the caller (`vfree`) must do that.  Does nothing if any level of the
/// page-table walk is absent (idempotent).
///
/// # Safety
/// The caller is responsible for freeing the physical frame after this call.
pub unsafe fn unmap_kernel_page(addr: u64) {
    unsafe {
        let pgdp = pgd_offset_pgd(init_pgd(), addr);
        if pgd_none(*pgdp) {
            return;
        }
        let p4dp = p4d_offset(pgdp, addr);
        let pudp = pud_offset(p4dp, addr);
        if pud_none(*pudp) {
            return;
        }
        let pmdp = pmd_offset(pudp, addr);
        if pmd_none(*pmdp) {
            return;
        }
        let ptep = pte_offset_kernel(pmdp, addr);
        set_pte(ptep, __pte(0));

        // Flush the local TLB entry.
        #[cfg(not(test))]
        core::arch::asm!(
            "invlpg [{0}]",
            in(reg) addr,
            options(nostack, preserves_flags),
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelImageLayout {
    pub mapping_start: u64,
    pub mapping_end: u64,
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub text_start: u64,
    pub text_end: u64,
    pub rodata_start: u64,
    pub rodata_end: u64,
    pub data_start: u64,
    pub bss_end: u64,
}

impl KernelImageLayout {
    pub const fn is_valid(self) -> bool {
        self.mapping_start < self.mapping_end
            && self.kernel_start < self.kernel_end
            && self.mapping_start <= self.kernel_start
            && self.kernel_end <= self.mapping_end
            && self.kernel_start <= self.text_start
            && self.text_start <= self.text_end
            && self.text_end <= self.rodata_start
            && self.rodata_start <= self.rodata_end
            && self.rodata_end <= self.data_start
            && self.data_start <= self.bss_end
            && self.bss_end <= self.kernel_end
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KernelImageProtectStats {
    pub split_pmds: u64,
    pub updated_pmds: u64,
    pub updated_ptes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KernelRangeUpdateStats {
    pub split_pmds: u64,
    pub updated_ptes: u64,
}

pub const fn kernel_image_page_flags(layout: KernelImageLayout, addr: u64) -> u64 {
    let page = addr & PAGE_MASK;
    if page < layout.kernel_start || page >= layout.kernel_end {
        __PAGE_KERNEL
    } else if page >= layout.text_start && page < layout.text_end {
        __PAGE_KERNEL_ROX
    } else if page >= layout.data_start && page < layout.bss_end {
        __PAGE_KERNEL
    } else {
        __PAGE_KERNEL_RO
    }
}

unsafe fn split_kernel_huge_pmd(
    pmdp: *mut pmd_t,
    pmd: pmd_t,
    pmd_base: u64,
    layout: KernelImageLayout,
) -> Option<u64> {
    let pt_phys = unsafe {
        alloc_pt_page(PtPageTraceContext::from_addr(
            "split_kernel_huge_pmd",
            "pte",
            pmd_base,
        ))?
    };
    if layout.mapping_start == 0 && pt_phys >= layout.kernel_start && pt_phys < layout.kernel_end {
        crate::kernel::printk::log_error!(
            "mm",
            "kernel W^X: page-table allocation overlaps kernel image: pmd_base={:#x} pt_phys={:#x} kernel={:#x}..{:#x}",
            pmd_base,
            pt_phys,
            layout.kernel_start,
            layout.kernel_end
        );
        return None;
    }
    let pt = entry_base_ptr(pt_phys) as *mut pte_t;
    let phys_base = pmd.0 & PTE_PFN_MASK;

    let mut i = 0usize;
    while i < PTRS_PER_PTE {
        let virt = pmd_base + (i as u64 * PAGE_SIZE);
        let phys = phys_base + (i as u64 * PAGE_SIZE);
        let flags = kernel_image_page_flags(layout, virt);
        unsafe { set_pte(pt.add(i), pte_t((phys & PTE_PFN_MASK) | flags)) };
        i += 1;
    }

    unsafe { set_pmd(pmdp, pmd_t((pt_phys & PTE_PFN_MASK) | _KERNPG_TABLE)) };
    Some(pt_phys)
}

unsafe fn split_huge_pmd_preserving_flags(
    pmdp: *mut pmd_t,
    pmd: pmd_t,
    pmd_base: u64,
) -> Option<u64> {
    let pt_phys = unsafe {
        alloc_pt_page(PtPageTraceContext::from_addr(
            "split_huge_pmd",
            "pte",
            pmd_base,
        ))?
    };
    let pt = entry_base_ptr(pt_phys) as *mut pte_t;
    let phys_base = pmd.0 & PTE_PFN_MASK;
    let flags = (pmd.0 & !PTE_PFN_MASK) & !_PAGE_PSE;

    let mut i = 0usize;
    while i < PTRS_PER_PTE {
        let phys = phys_base + (i as u64 * PAGE_SIZE);
        unsafe { set_pte(pt.add(i), pte_t((phys & PTE_PFN_MASK) | flags)) };
        i += 1;
    }

    unsafe { set_pmd(pmdp, pmd_t((pt_phys & PTE_PFN_MASK) | _KERNPG_TABLE)) };
    Some(pt_phys)
}

/// Split the early 2 MiB kernel-image huge mappings and apply Linux W^X
/// permissions to the high kernel map.
///
/// The early Linux boot-protocol entry builds a coarse 1 GiB
/// `__START_KERNEL_map` window so Rust can start quickly. Linux later refines those permissions
/// before running userspace: text is read-only executable, rodata is read-only
/// NX, and data/BSS stay writable NX. This helper performs the same transition
/// on Lupos' early page tables.
///
/// # Safety
/// The supplied layout must describe the active higher-half kernel mapping,
/// and the caller must run after the buddy allocator can provide page-table
/// pages.
pub unsafe fn protect_kernel_image_mappings(
    layout: KernelImageLayout,
) -> Option<KernelImageProtectStats> {
    if !layout.is_valid()
        || layout.mapping_start & (PMD_SIZE - 1) != 0
        || layout.mapping_end & (PMD_SIZE - 1) != 0
    {
        return None;
    }

    let mut stats = KernelImageProtectStats::default();
    let pgd = init_pgd();
    let mut addr = layout.mapping_start;
    while addr < layout.mapping_end {
        unsafe {
            let pgdp = pgd_offset_pgd(pgd, addr);
            if pgd_none(*pgdp) {
                addr += PMD_SIZE;
                continue;
            }
            let p4dp = p4d_offset(pgdp, addr);
            let pudp = pud_offset(p4dp, addr);
            if pud_none(*pudp) || pud_huge(*pudp) {
                addr += PMD_SIZE;
                continue;
            }
            let pmdp = pmd_offset(pudp, addr);
            if pmd_none(*pmdp) {
                addr += PMD_SIZE;
                continue;
            }

            let pmd = *pmdp;
            if pmd_huge(pmd) {
                let pmd_end = addr + PMD_SIZE;
                if addr < layout.kernel_end && pmd_end > layout.kernel_start {
                    split_kernel_huge_pmd(pmdp, pmd, addr, layout)?;
                    stats.split_pmds += 1;
                    stats.updated_ptes += PTRS_PER_PTE as u64;
                } else if pmd.0 & _PAGE_NX == 0 {
                    set_pmd(pmdp, pmd_t(pmd.0 | _PAGE_NX));
                    stats.updated_pmds += 1;
                }
                addr += PMD_SIZE;
                continue;
            }

            let ptep = pte_offset_kernel(pmdp, addr);
            let pte_table_phys = (ptep as u64).wrapping_sub(PAGE_OFFSET) & PAGE_MASK;
            if let Some((kernel_phys_start, kernel_phys_end)) = kernel_phys_bounds() {
                if pte_table_phys >= kernel_phys_start && pte_table_phys < kernel_phys_end {
                    let pmdp_phys = (pmdp as u64).wrapping_sub(PAGE_OFFSET);
                    crate::kernel::printk::log_error!(
                        "mm",
                        "W^X bad PMD: addr={:#x} pmdp={:#x} pmd={:#x} pte_table={:#x}",
                        addr,
                        pmdp_phys,
                        pmd.0,
                        pte_table_phys
                    );
                    return None;
                }
            }
            let mut i = 0usize;
            while i < PTRS_PER_PTE {
                let virt = addr + (i as u64 * PAGE_SIZE);
                let pte = *ptep.add(i);
                if pte.0 & _PAGE_PRESENT != 0 {
                    let phys = pte.0 & PTE_PFN_MASK;
                    let flags = kernel_image_page_flags(layout, virt);
                    set_pte(ptep.add(i), pte_t(phys | flags));
                    stats.updated_ptes += 1;
                }
                i += 1;
            }
        }
        addr += PMD_SIZE;
    }

    unsafe { flush_tlb_all_local() };
    Some(stats)
}

/// Apply one kernel PTE protection to a live page-aligned kernel range.
///
/// This is the small `set_memory_*()` subset needed by Linux's x86
/// `free_init_pages()` path: before pages from the kernel image are returned
/// to the buddy allocator they must be writable and non-executable through
/// every kernel alias that might later reuse them.
pub unsafe fn set_kernel_page_range_flags(
    start: u64,
    end: u64,
    flags: u64,
) -> Option<KernelRangeUpdateStats> {
    if start >= end || start & (PAGE_SIZE - 1) != 0 || end & (PAGE_SIZE - 1) != 0 {
        return None;
    }

    let pgd = init_pgd();
    let mut stats = KernelRangeUpdateStats::default();
    let mut addr = start;
    while addr < end {
        unsafe {
            let pgdp = pgd_offset_pgd(pgd, addr);
            if pgd_none(*pgdp) {
                addr += PAGE_SIZE;
                continue;
            }
            let p4dp = p4d_offset(pgdp, addr);
            let pudp = pud_offset(p4dp, addr);
            if pud_none(*pudp) || pud_huge(*pudp) {
                addr += PAGE_SIZE;
                continue;
            }
            let pmdp = pmd_offset(pudp, addr);
            if pmd_none(*pmdp) {
                addr += PAGE_SIZE;
                continue;
            }

            let pmd = *pmdp;
            if pmd_huge(pmd) {
                split_huge_pmd_preserving_flags(pmdp, pmd, addr & !(PMD_SIZE - 1))?;
                stats.split_pmds += 1;
            }

            let ptep = pte_offset_kernel(pmdp, addr);
            let pte = *ptep;
            if pte.0 & _PAGE_PRESENT != 0 {
                set_pte(ptep, pte_t((pte.0 & PTE_PFN_MASK) | flags));
                stats.updated_ptes += 1;
            }
        }
        addr += PAGE_SIZE;
    }

    unsafe { flush_tlb_all_local() };
    Some(stats)
}

/// Set or clear Linux's dynamic `_PAGE_ENC` bit over a kernel range.
///
/// This is the SME/SEV subset of Linux
/// `arch/x86/mm/pat/set_memory.c::__set_memory_enc_pgtable()`: walk the active
/// kernel page tables, update the encryption attribute in each present leaf,
/// split 2 MiB PMDs when a subrange needs 4 KiB granularity, then flush the
/// affected TLB range.
///
/// # Safety
/// The caller must ensure `enc_mask` is the active x86 SME C-bit mask and that
/// the range belongs to the live kernel mapping.
pub unsafe fn set_kernel_page_encryption_mask(
    start: u64,
    numpages: usize,
    enc_mask: u64,
    encrypt: bool,
) -> Result<KernelRangeUpdateStats, i32> {
    use crate::include::uapi::errno::EINVAL;

    if numpages == 0 || enc_mask == 0 {
        return Ok(KernelRangeUpdateStats::default());
    }
    if start & (PAGE_SIZE - 1) != 0 {
        return Err(EINVAL);
    }
    let Some(bytes) = (numpages as u64).checked_mul(PAGE_SIZE) else {
        return Err(EINVAL);
    };
    let Some(end) = start.checked_add(bytes) else {
        return Err(EINVAL);
    };

    let pgd = init_pgd();
    let mut stats = KernelRangeUpdateStats::default();
    let mut addr = start;
    while addr < end {
        unsafe {
            let pgdp = pgd_offset_pgd(pgd, addr);
            if pgd_none(*pgdp) {
                return Err(EINVAL);
            }
            let p4dp = p4d_offset(pgdp, addr);
            let pudp = pud_offset(p4dp, addr);
            if pud_none(*pudp) {
                return Err(EINVAL);
            }
            let pud = *pudp;
            if pud_huge(pud) {
                let pud_base = addr & !(PUD_SIZE - 1);
                if addr != pud_base || end.saturating_sub(addr) < PUD_SIZE {
                    return Err(EINVAL);
                }
                let new = if encrypt {
                    pud.0 | enc_mask
                } else {
                    pud.0 & !enc_mask
                };
                set_pud(pudp, pud_t(new));
                stats.updated_ptes += (PUD_SIZE / PAGE_SIZE) as u64;
                addr += PUD_SIZE;
                continue;
            }

            let pmdp = pmd_offset(pudp, addr);
            if pmd_none(*pmdp) {
                return Err(EINVAL);
            }
            let pmd = *pmdp;
            if pmd_huge(pmd) {
                let pmd_base = addr & !(PMD_SIZE - 1);
                if addr == pmd_base && end.saturating_sub(addr) >= PMD_SIZE {
                    let new = if encrypt {
                        pmd.0 | enc_mask
                    } else {
                        pmd.0 & !enc_mask
                    };
                    set_pmd(pmdp, pmd_t(new));
                    stats.updated_ptes += PTRS_PER_PTE as u64;
                    addr += PMD_SIZE;
                    continue;
                }
                split_huge_pmd_preserving_flags(pmdp, pmd, pmd_base).ok_or(EINVAL)?;
                stats.split_pmds += 1;
            }

            let ptep = pte_offset_kernel(pmdp, addr);
            let pte = *ptep;
            if pte.0 & _PAGE_PRESENT == 0 {
                return Err(EINVAL);
            }
            let new = if encrypt {
                pte.0 | enc_mask
            } else {
                pte.0 & !enc_mask
            };
            set_pte(ptep, pte_t(new));
            stats.updated_ptes += 1;
        }
        addr += PAGE_SIZE;
    }

    unsafe { flush_tlb_range(start, end) };
    Ok(stats)
}

/// Walk the kernel page tables and return the physical address that `virt`
/// currently maps to, or `None` if the mapping is absent at any level.
///
/// Ref: Linux `virt_to_phys()` / `slow_virt_to_phys()` —
///      `arch/x86/include/asm/io.h`, `arch/x86/mm/physaddr.c`
pub fn virt_to_phys(virt: u64) -> Option<u64> {
    use crate::mm::pagewalk::{MmWalk, MmWalkOps, PageWalkAction, walk_kernel_page_table_range};

    /// Single-PTE lookup callback — captures the leaf entry that covers
    /// `target` and short-circuits the walk by returning `Err(STOP)`.
    struct Lookup {
        target: u64,
        result: Option<u64>,
    }

    /// Sentinel "we found what we wanted, abort" — non-zero like Linux.
    const STOP: i32 = 1;

    impl MmWalkOps for Lookup {
        fn pte_entry(
            &mut self,
            ptep: *mut pte_t,
            addr: u64,
            _next: u64,
            _walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            // We always pass a single-page range so this fires at most once,
            // but we still bound-check defensively.
            if addr == (self.target & PAGE_MASK) {
                let pte = unsafe { *ptep };
                if !pte_none(pte) {
                    self.result = Some(pte_phys(pte) | (self.target & !PAGE_MASK));
                }
                return Err(STOP);
            }
            Ok(())
        }

        fn pmd_entry(
            &mut self,
            pmdp: *mut pmd_t,
            addr: u64,
            _next: u64,
            walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            // Honor 2 MiB huge pages — Linux's `pmd_huge` path.
            let pmd = unsafe { *pmdp };
            if pmd_huge(pmd) {
                let base = pmd.0 & PTE_PFN_MASK;
                // `addr` may not be PMD-aligned when the walker is invoked on a
                // sub-PMD interval.  For huge PMD mappings compute the offset
                // from the 2 MiB-aligned base address.
                let pmd_base = addr & !(PMD_SIZE - 1);
                self.result = Some(base + (self.target - pmd_base));
                walk.action = PageWalkAction::Continue;
                return Err(STOP);
            }
            Ok(())
        }

        fn pud_entry(
            &mut self,
            pudp: *mut pud_t,
            addr: u64,
            _next: u64,
            walk: &mut MmWalk<'_>,
        ) -> Result<(), i32> {
            let pud = unsafe { *pudp };
            if pud_huge(pud) {
                let base = pud.0 & PTE_PFN_MASK;
                let pud_base = addr & !(PUD_SIZE - 1);
                self.result = Some(base + (self.target - pud_base));
                walk.action = PageWalkAction::Continue;
                return Err(STOP);
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

    let aligned = virt & PAGE_MASK;
    let mut lookup = Lookup {
        target: virt,
        result: None,
    };
    let pgd = init_pgd();
    let _ = unsafe {
        walk_kernel_page_table_range(
            aligned,
            aligned + PAGE_SIZE,
            &mut lookup,
            pgd,
            core::ptr::null_mut(),
        )
    };
    lookup.result
}

// ---------------------------------------------------------------------------
// Test page-table pool
// ---------------------------------------------------------------------------

/// Static pool of 4 KiB page-table pages for host unit tests.
///
/// In production, page-table pages are allocated from the buddy allocator
/// via `alloc_pt_page`.  The test runner has no buddy allocator, so we
/// pre-allocate a small set of zeroed pages here.  `reset()` clears all
/// entries and resets the cursor so each test starts with a clean slate.
#[cfg(test)]
pub(crate) mod test_pool {
    use core::ptr::addr_of_mut;

    /// Number of table pages in the pool (PML4 + PDPT + PD + several PTs).
    const POOL_PAGES: usize = 12;
    /// Size of one table page in bytes (512 × u64 = 4 KiB).
    const PAGE_BYTES: usize = 512 * 8;

    /// One 4 KiB page properly aligned for use as a hardware page table.
    #[repr(C, align(4096))]
    struct TablePage([u8; PAGE_BYTES]);

    static mut TABLE_POOL: [TablePage; POOL_PAGES] =
        [const { TablePage([0u8; PAGE_BYTES]) }; POOL_PAGES];

    /// Next free page index.  Index 0 is reserved as the PML4.
    static mut TABLE_CURSOR: usize = 1;

    /// Reset all pages to zero and restart the cursor.  Must be called at
    /// the start of each test while `TEST_LOCK` is held.
    pub unsafe fn reset() {
        unsafe {
            // Zero whole pool via raw pointer — avoids creating &mut to static.
            core::ptr::write_bytes(
                addr_of_mut!(TABLE_POOL) as *mut u8,
                0,
                POOL_PAGES * PAGE_BYTES,
            );
            addr_of_mut!(TABLE_CURSOR).write(1);
        }
    }

    /// Return the virtual address of the PML4 page (index 0 in the pool).
    pub unsafe fn pml4_base() -> u64 {
        unsafe { addr_of_mut!(TABLE_POOL) as u64 }
    }

    /// Hand out one page from the pool (called by `alloc_pt_page`).
    pub unsafe fn alloc() -> Option<u64> {
        unsafe {
            let cursor = addr_of_mut!(TABLE_CURSOR).read();
            if cursor >= POOL_PAGES {
                return None;
            }
            let ptr = (addr_of_mut!(TABLE_POOL) as *mut u8).add(cursor * PAGE_BYTES) as u64;
            addr_of_mut!(TABLE_CURSOR).write(cursor + 1);
            Some(ptr)
        }
    }
}

// ---------------------------------------------------------------------------
// Swap PTE encoding — x86_64 non-present PTE layout
//
// When _PAGE_PRESENT (bit 0) is clear the CPU ignores the PTE, so Linux
// repurposes the bits to encode a swap entry.
//
// Layout (from vendor/linux/arch/x86/include/asm/pgtable_64.h:211-237):
//
//   bits [63:59] — swap type  (5 bits, MAX_SWAPFILES = 32)
//   bit  [8]     — _PAGE_BIT_PROTNONE: must stay CLEAR for a real swap PTE
//                  (set = PROT_NONE mapping, distinct from swap)
//   bits [58:14] — inverted swap offset  (= ~offset << 14 >> 5)
//   bits [13:0]  — zero / unused
//
// The generic SwpEntry lives in memory::swap.  These arch helpers convert
// between that generic form and the raw u64 stored in the pte_t.
// ---------------------------------------------------------------------------

/// Encode a swap type + offset into the arch-specific non-present PTE value.
///
/// Ref: Linux `__swp_entry()` — arch/x86/include/asm/pgtable_64.h:230
#[inline]
pub fn arch_swp_entry(swap_type: u8, offset: u32) -> u64 {
    let t = swap_type as u64;
    let off = offset as u64;
    // Invert offset so that zero-offset is distinguishable from pte_none(0).
    ((!off) << 14 >> 5) | (t << 59)
}

/// Decode the swap type from an arch non-present PTE value.
///
/// Ref: Linux `__swp_type()` — arch/x86/include/asm/pgtable_64.h:221
#[inline]
pub fn arch_swp_type(pte_val: u64) -> u8 {
    (pte_val >> 59) as u8
}

/// Decode the swap offset from an arch non-present PTE value.
///
/// Ref: Linux `__swp_offset()` — arch/x86/include/asm/pgtable_64.h:224
#[inline]
pub fn arch_swp_offset(pte_val: u64) -> u32 {
    ((!pte_val) << 5 >> 14) as u32
}

/// Return true iff `pte` is a swap PTE: not present AND not zero.
///
/// A zero PTE is `pte_none` (unallocated slot).
/// A PTE with `_PAGE_PRESENT` set is a hardware mapping.
/// Everything else is a swap entry.
///
/// Ref: Linux `is_swap_pte()` — include/linux/mm.h
#[inline]
pub fn is_swap_pte(pte: pte_t) -> bool {
    !pte_present(pte) && !pte_none(pte)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::mm::frame::PhysFrame;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

    fn setup() {
        unsafe { test_pool::reset() };
    }

    // ── Shift/index constants ────────────────────────────────────────────

    #[test]
    fn pgdir_shift_is_39() {
        assert_eq!(PGDIR_SHIFT, 39);
    }

    #[test]
    fn pud_shift_is_30() {
        assert_eq!(PUD_SHIFT, 30);
    }

    #[test]
    fn pmd_shift_is_21() {
        assert_eq!(PMD_SHIFT, 21);
    }

    #[test]
    fn page_shift_is_12() {
        assert_eq!(PAGE_SHIFT, 12);
    }

    #[test]
    fn ptrs_per_level_all_512() {
        assert_eq!(PTRS_PER_PGD, 512);
        assert_eq!(PTRS_PER_PUD, 512);
        assert_eq!(PTRS_PER_PMD, 512);
        assert_eq!(PTRS_PER_PTE, 512);
    }

    // ── Flag bit positions ───────────────────────────────────────────────

    #[test]
    fn page_flag_bit_positions_match_linux() {
        assert_eq!(_PAGE_PRESENT, 1 << 0);
        assert_eq!(_PAGE_RW, 1 << 1);
        assert_eq!(_PAGE_USER, 1 << 2);
        assert_eq!(_PAGE_ACCESSED, 1 << 5);
        assert_eq!(_PAGE_DIRTY, 1 << 6);
        assert_eq!(_PAGE_NX, 1u64 << 63);
    }

    #[test]
    fn kernpg_table_is_present_rw_accessed_dirty() {
        assert_eq!(
            _KERNPG_TABLE,
            _PAGE_PRESENT | _PAGE_RW | _PAGE_ACCESSED | _PAGE_DIRTY
        );
    }

    // ── Index functions ──────────────────────────────────────────────────

    #[test]
    fn pgd_index_extracts_bits_47_39() {
        // bit 39 set → pgd_index = 1
        assert_eq!(pgd_index(1u64 << 39), 1);
        // all 9 bits set → 511
        assert_eq!(pgd_index(0x0000_FF80_0000_0000u64), 511);
    }

    #[test]
    fn pud_index_extracts_bits_38_30() {
        assert_eq!(pud_index(1u64 << 30), 1);
    }

    #[test]
    fn pmd_index_extracts_bits_29_21() {
        assert_eq!(pmd_index(1u64 << 21), 1);
    }

    #[test]
    fn pte_index_extracts_bits_20_12() {
        assert_eq!(pte_index(1u64 << 12), 1);
    }

    #[test]
    fn all_512_pgd_indices_are_distinct() {
        for i in 0..512usize {
            assert_eq!(pgd_index((i as u64) << 39), i);
        }
    }

    // ── pfn_pte / pte_pfn round-trip ────────────────────────────────────

    #[test]
    fn pfn_pte_pte_pfn_roundtrip() {
        let pfn: u64 = 0x1234;
        let pte = pfn_pte(pfn, PAGE_KERNEL);
        assert_eq!(pte_pfn(pte), pfn);
    }

    #[test]
    fn pte_phys_extracts_physical_address() {
        let phys: u64 = 0x0000_0000_0020_0000; // 2 MiB, page-aligned
        let pte = pfn_pte(phys >> PAGE_SHIFT, PAGE_KERNEL);
        assert_eq!(pte_phys(pte), phys);
    }

    // ── map_kernel_page / virt_to_phys ──────────────────────────────────

    #[test]
    fn map_then_virt_to_phys_returns_phys() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        let phys: u64 = 0x0000_0000_0020_0000;
        let virt: u64 = 0x0000_0000_0040_0000;
        unsafe { map_kernel_page(virt, phys, PAGE_KERNEL) };
        assert_eq!(virt_to_phys(virt), Some(phys));
    }

    #[test]
    fn virt_to_phys_accounts_for_pmd_huge_page_offset() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        // Map one 2 MiB huge page at virt_base -> phys_base, then translate an
        // address within that huge mapping that is *not* 2 MiB-aligned.
        let virt_base: u64 = 0x0000_0000_0080_0000; // 8 MiB (2 MiB-aligned)
        let phys_base: u64 = 0x0000_0000_0060_0000; // 6 MiB (2 MiB-aligned)

        unsafe {
            let pgd = init_pgd();
            let pgdp = pgd_offset_pgd(pgd, virt_base);
            let pudp = pud_alloc_kernel(pgdp, virt_base).expect("pud_alloc_kernel");
            let pmdp = pmd_alloc_kernel(pudp, virt_base).expect("pmd_alloc_kernel");
            set_pmd(
                pmdp,
                __pmd(phys_base | _PAGE_PRESENT | _PAGE_RW | _PAGE_PSE),
            );
        }

        let off: u64 = 0x1d20_00; // inside the 2 MiB window, intentionally non-aligned
        let virt = virt_base + off;
        assert_eq!(virt_to_phys(virt), Some(phys_base + off));
    }

    #[test]
    fn protect_kernel_image_splits_huge_pmd_and_applies_wx_permissions() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let virt_base: u64 = 0xffff_ffff_8000_0000;
        let phys_base: u64 = 0x0000_0000_0020_0000;
        unsafe {
            let pgd = init_pgd();
            let pgdp = pgd_offset_pgd(pgd, virt_base);
            let pudp = pud_alloc_kernel(pgdp, virt_base).expect("pud_alloc_kernel");
            let pmdp = pmd_alloc_kernel(pudp, virt_base).expect("pmd_alloc_kernel");
            set_pmd(
                pmdp,
                __pmd(phys_base | _PAGE_PRESENT | _PAGE_RW | _PAGE_PSE),
            );

            let pmdp_next =
                pmd_alloc_kernel(pudp, virt_base + PMD_SIZE).expect("pmd_alloc_kernel next");
            set_pmd(
                pmdp_next,
                __pmd(phys_base + PMD_SIZE | _PAGE_PRESENT | _PAGE_RW | _PAGE_PSE),
            );
        }

        let layout = KernelImageLayout {
            mapping_start: virt_base,
            mapping_end: virt_base + 2 * PMD_SIZE,
            kernel_start: virt_base,
            kernel_end: virt_base + 4 * PAGE_SIZE,
            text_start: virt_base,
            text_end: virt_base + PAGE_SIZE,
            rodata_start: virt_base + PAGE_SIZE,
            rodata_end: virt_base + 2 * PAGE_SIZE,
            data_start: virt_base + 2 * PAGE_SIZE,
            bss_end: virt_base + 4 * PAGE_SIZE,
        };

        let stats = unsafe { protect_kernel_image_mappings(layout).expect("protect") };
        assert_eq!(stats.split_pmds, 1);
        assert_eq!(stats.updated_pmds, 1);

        unsafe {
            let pgd = init_pgd();
            let pgdp = pgd_offset_pgd(pgd, virt_base);
            let p4dp = p4d_offset(pgdp, virt_base);
            let pudp = pud_offset(p4dp, virt_base);
            let pmdp = pmd_offset(pudp, virt_base);
            assert!(!pmd_huge(*pmdp));

            let text = (*pte_offset_kernel(pmdp, virt_base)).0;
            assert_eq!(text & _PAGE_RW, 0);
            assert_eq!(text & _PAGE_NX, 0);

            let rodata = (*pte_offset_kernel(pmdp, virt_base + PAGE_SIZE)).0;
            assert_eq!(rodata & _PAGE_RW, 0);
            assert_ne!(rodata & _PAGE_NX, 0);

            let data = (*pte_offset_kernel(pmdp, virt_base + 2 * PAGE_SIZE)).0;
            assert_ne!(data & _PAGE_RW, 0);
            assert_ne!(data & _PAGE_NX, 0);

            let outside_kernel = (*pte_offset_kernel(pmdp, virt_base + 8 * PAGE_SIZE)).0;
            assert_ne!(outside_kernel & _PAGE_RW, 0);
            assert_ne!(outside_kernel & _PAGE_NX, 0);

            let pmdp_next = pmd_offset(pudp, virt_base + PMD_SIZE);
            assert!(pmd_huge(*pmdp_next));
            assert_ne!((*pmdp_next).0 & _PAGE_NX, 0);
        }
    }

    #[test]
    fn clone_low_identity_deep_copies_split_pte_tables() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let virt_base: u64 = 0x0000_0000_0020_0000;
        let phys_base: u64 = 0x0000_0000_0020_0000;
        unsafe {
            let pgd = init_pgd();
            let pgdp = pgd_offset_pgd(pgd, virt_base);
            let pudp = pud_alloc_kernel(pgdp, virt_base).expect("pud_alloc_kernel");
            let pmdp = pmd_alloc_kernel(pudp, virt_base).expect("pmd_alloc_kernel");
            set_pmd(
                pmdp,
                __pmd(phys_base | _PAGE_PRESENT | _PAGE_RW | _PAGE_PSE),
            );
        }

        let layout = KernelImageLayout {
            mapping_start: 0,
            mapping_end: 4 * PMD_SIZE,
            kernel_start: virt_base,
            kernel_end: virt_base + 2 * PAGE_SIZE,
            text_start: virt_base,
            text_end: virt_base + PAGE_SIZE,
            rodata_start: virt_base + PAGE_SIZE,
            rodata_end: virt_base + 2 * PAGE_SIZE,
            data_start: virt_base + 2 * PAGE_SIZE,
            bss_end: virt_base + 2 * PAGE_SIZE,
        };
        unsafe { protect_kernel_image_mappings(layout).expect("protect") };

        let dst_pgd_phys = unsafe {
            alloc_pt_page(PtPageTraceContext::from_indices(
                "test_dst_pgd",
                "pgd_root",
                0,
                0,
                0,
            ))
            .expect("dst pgd")
        };
        let dst_pgd = entry_base_ptr(dst_pgd_phys) as *mut pgd_t;
        unsafe {
            clone_low_identity_pgd_slot_for_user(dst_pgd, init_pgd_for_test()).expect("clone");

            let src_pgdp = pgd_offset_pgd(init_pgd_for_test(), virt_base);
            let src_p4dp = p4d_offset(src_pgdp, virt_base);
            let src_pudp = pud_offset(src_p4dp, virt_base);
            let src_pmdp = pmd_offset(src_pudp, virt_base);
            let src_ptep = pte_offset_kernel(src_pmdp, virt_base);

            let dst_pgdp = pgd_offset_pgd(dst_pgd, virt_base);
            let dst_p4dp = p4d_offset(dst_pgdp, virt_base);
            let dst_pudp = pud_offset(dst_p4dp, virt_base);
            let dst_pmdp = pmd_offset(dst_pudp, virt_base);
            let dst_ptep = pte_offset_kernel(dst_pmdp, virt_base);

            assert_ne!(
                (*src_pmdp).0 & PTE_PFN_MASK,
                (*dst_pmdp).0 & PTE_PFN_MASK,
                "split identity PTE table must not be shared with user PGD"
            );

            let original_src = (*src_ptep).0;
            set_pte(dst_ptep, pte_t(0));
            assert_eq!((*src_ptep).0, original_src);
            assert_eq!((*dst_ptep).0, 0);
        }
    }

    #[test]
    fn set_kernel_page_range_flags_splits_huge_pmd_and_updates_only_target_pages() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let virt_base: u64 = 0xffff_ffff_8020_0000;
        let phys_base: u64 = 0x0000_0000_0040_0000;
        unsafe {
            let pgd = init_pgd();
            let pgdp = pgd_offset_pgd(pgd, virt_base);
            let pudp = pud_alloc_kernel(pgdp, virt_base).expect("pud_alloc_kernel");
            let pmdp = pmd_alloc_kernel(pudp, virt_base).expect("pmd_alloc_kernel");
            set_pmd(
                pmdp,
                __pmd(phys_base | _PAGE_PRESENT | _PAGE_RW | _PAGE_PSE),
            );
        }

        let start = virt_base + PAGE_SIZE;
        let end = virt_base + 3 * PAGE_SIZE;
        let stats = unsafe { set_kernel_page_range_flags(start, end, __PAGE_KERNEL) }
            .expect("set range flags");
        assert_eq!(stats.split_pmds, 1);
        assert_eq!(stats.updated_ptes, 2);

        unsafe {
            let pgd = init_pgd();
            let pgdp = pgd_offset_pgd(pgd, virt_base);
            let p4dp = p4d_offset(pgdp, virt_base);
            let pudp = pud_offset(p4dp, virt_base);
            let pmdp = pmd_offset(pudp, virt_base);
            assert!(!pmd_huge(*pmdp));

            let outside_before = (*pte_offset_kernel(pmdp, virt_base)).0;
            assert_ne!(outside_before & _PAGE_RW, 0);
            assert_eq!(outside_before & _PAGE_NX, 0);

            let first_freed = (*pte_offset_kernel(pmdp, start)).0;
            assert_ne!(first_freed & _PAGE_RW, 0);
            assert_ne!(first_freed & _PAGE_NX, 0);

            let outside_after = (*pte_offset_kernel(pmdp, end)).0;
            assert_ne!(outside_after & _PAGE_RW, 0);
            assert_eq!(outside_after & _PAGE_NX, 0);
        }
    }

    #[test]
    fn virt_to_phys_unmapped_returns_none() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        assert_eq!(virt_to_phys(0x0000_0000_0010_0000), None);
    }

    #[test]
    fn two_different_vas_map_independently() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        let phys_a: u64 = 0x0000_0000_0010_0000;
        let phys_b: u64 = 0x0000_0000_0020_0000;
        let virt_a: u64 = 0x0000_0000_0030_0000;
        let virt_b: u64 = 0x0000_0000_0031_0000; // same PT, next entry
        unsafe {
            map_kernel_page(virt_a, phys_a, PAGE_KERNEL);
            map_kernel_page(virt_b, phys_b, PAGE_KERNEL);
        }
        assert_eq!(virt_to_phys(virt_a), Some(phys_a));
        assert_eq!(virt_to_phys(virt_b), Some(phys_b));
    }

    #[test]
    fn remap_overwrites_previous_pte() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        let phys_old: u64 = 0x0000_0000_0010_0000;
        let phys_new: u64 = 0x0000_0000_0020_0000;
        let virt: u64 = 0x0000_0000_0040_0000;
        unsafe {
            map_kernel_page(virt, phys_old, PAGE_KERNEL);
            map_kernel_page(virt, phys_new, PAGE_KERNEL);
        }
        assert_eq!(virt_to_phys(virt), Some(phys_new));
    }

    // ── unmap_kernel_page ───────────────────────────────────────────────

    #[test]
    fn unmap_makes_virt_to_phys_return_none() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        let phys: u64 = 0x0000_0000_0060_0000;
        let virt: u64 = 0x0000_0000_0070_0000;
        unsafe { map_kernel_page(virt, phys, PAGE_KERNEL) };
        assert_eq!(virt_to_phys(virt), Some(phys));
        unsafe { unmap_kernel_page(virt) };
        assert_eq!(virt_to_phys(virt), None);
    }

    #[test]
    fn unmap_nonexistent_is_noop() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        // Must not panic.
        unsafe { unmap_kernel_page(0x0000_0000_0080_0000) };
    }

    // ── vmalloc-window addresses (upper canonical half) ──────────────────

    #[test]
    fn map_in_vmalloc_window() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        let virt: u64 = 0xFFFF_C900_0000_0000; // VMALLOC_START equivalent
        let phys: u64 = 0x0000_0000_0050_0000;
        unsafe { map_kernel_page(virt, phys, PAGE_KERNEL) };
        assert_eq!(virt_to_phys(virt), Some(phys));
    }

    #[test]
    fn unmap_in_vmalloc_window() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();
        let virt: u64 = 0xFFFF_C900_0001_0000;
        let phys: u64 = 0x0000_0000_0060_0000;
        unsafe { map_kernel_page(virt, phys, PAGE_KERNEL) };
        assert_eq!(virt_to_phys(virt), Some(phys));
        unsafe { unmap_kernel_page(virt) };
        assert_eq!(virt_to_phys(virt), None);
    }

    #[test]
    fn kmap_roundtrip_uses_reserved_slot() {
        let _g = TEST_LOCK.lock().unwrap();
        setup();

        let frame = PhysFrame(0x123);
        let mapped = unsafe { kmap(frame) };
        assert_eq!(mapped as u64, KMAP_START);
        assert_eq!(virt_to_phys(KMAP_START), Some(frame.start_address()));

        unsafe { kunmap(mapped) };
        assert_eq!(virt_to_phys(KMAP_START), None);
    }
}
