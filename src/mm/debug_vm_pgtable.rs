//! linux-parity: partial
//! linux-source: vendor/linux/mm/debug_vm_pgtable.c
//! test-origin: linux:vendor/linux/mm/debug_vm_pgtable.c
//! Source-backed model of generic page-table helper invariants.

extern crate alloc;

use alloc::vec::Vec;

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
pub const PMD_SHIFT: u64 = 21;
pub const PMD_SIZE: u64 = 1 << PMD_SHIFT;
pub const PUD_SHIFT: u64 = 30;
pub const PUD_SIZE: u64 = 1 << PUD_SHIFT;
pub const RANDOM_NZVALUE: u64 = 0xff;
pub const ULONG_MAX: u64 = u64::MAX;

pub const VM_NONE: u64 = 0;
pub const VM_READ: u64 = 1 << 0;
pub const VM_WRITE: u64 = 1 << 1;
pub const VM_EXEC: u64 = 1 << 2;
pub const VM_SHARED: u64 = 1 << 3;
pub const VM_FLAGS_START: u64 = VM_NONE;
pub const VM_FLAGS_END: u64 = VM_SHARED | VM_EXEC | VM_WRITE | VM_READ;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PgEntry {
    pub pfn: u64,
    pub present: bool,
    pub young: bool,
    pub dirty: bool,
    pub writable: bool,
    pub huge: bool,
    pub special: bool,
    pub protnone: bool,
    pub soft_dirty: bool,
    pub exclusive: bool,
    pub swap: bool,
    pub migration: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedPfns {
    pub fixed_alignment: u64,
    pub fixed_pgd_pfn: u64,
    pub fixed_p4d_pfn: u64,
    pub fixed_pud_pfn: u64,
    pub fixed_pmd_pfn: u64,
    pub fixed_pte_pfn: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PgtableBasicResult {
    pub same: bool,
    pub young_round_trip: bool,
    pub dirty_round_trip: bool,
    pub writable_round_trip: bool,
    pub wrprotect_clean_is_clean: bool,
    pub wrprotect_dirty_stays_dirty: bool,
}

pub const fn pfn_entry(pfn: u64) -> PgEntry {
    PgEntry {
        pfn,
        present: true,
        young: true,
        dirty: false,
        writable: false,
        huge: false,
        special: false,
        protnone: false,
        soft_dirty: false,
        exclusive: false,
        swap: false,
        migration: false,
    }
}

pub const fn entry_same(a: PgEntry, b: PgEntry) -> bool {
    a.pfn == b.pfn
        && a.present == b.present
        && a.young == b.young
        && a.dirty == b.dirty
        && a.writable == b.writable
        && a.huge == b.huge
        && a.special == b.special
        && a.protnone == b.protnone
        && a.soft_dirty == b.soft_dirty
        && a.exclusive == b.exclusive
        && a.swap == b.swap
        && a.migration == b.migration
}

pub const fn entry_mkyoung(mut entry: PgEntry) -> PgEntry {
    entry.young = true;
    entry
}

pub const fn entry_mkold(mut entry: PgEntry) -> PgEntry {
    entry.young = false;
    entry
}

pub const fn entry_mkdirty(mut entry: PgEntry) -> PgEntry {
    entry.dirty = true;
    entry
}

pub const fn entry_mkclean(mut entry: PgEntry) -> PgEntry {
    entry.dirty = false;
    entry
}

pub const fn entry_mkwrite(mut entry: PgEntry) -> PgEntry {
    entry.writable = true;
    entry
}

pub const fn entry_wrprotect(mut entry: PgEntry) -> PgEntry {
    entry.writable = false;
    entry
}

pub const fn entry_mkhuge(mut entry: PgEntry) -> PgEntry {
    entry.huge = true;
    entry
}

pub const fn entry_mkspecial(mut entry: PgEntry) -> PgEntry {
    entry.special = true;
    entry
}

pub const fn entry_mkprotnone(mut entry: PgEntry) -> PgEntry {
    entry.present = true;
    entry.protnone = true;
    entry
}

pub const fn entry_mksoft_dirty(mut entry: PgEntry) -> PgEntry {
    entry.soft_dirty = true;
    entry
}

pub const fn entry_clear_soft_dirty(mut entry: PgEntry) -> PgEntry {
    entry.soft_dirty = false;
    entry
}

pub const fn entry_swp_mkexclusive(mut entry: PgEntry) -> PgEntry {
    entry.swap = true;
    entry.exclusive = true;
    entry
}

pub const fn entry_swp_clear_exclusive(mut entry: PgEntry) -> PgEntry {
    entry.exclusive = false;
    entry
}

pub const fn basic_entry_tests(entry: PgEntry) -> PgtableBasicResult {
    PgtableBasicResult {
        same: entry_same(entry, entry),
        young_round_trip: entry_mkyoung(entry_mkold(entry_mkyoung(entry))).young
            && !entry_mkold(entry_mkyoung(entry_mkold(entry))).young,
        dirty_round_trip: entry_mkdirty(entry_mkclean(entry_mkdirty(entry))).dirty
            && !entry_mkclean(entry_mkdirty(entry_mkclean(entry))).dirty,
        writable_round_trip: entry_mkwrite(entry_wrprotect(entry_mkwrite(entry))).writable
            && !entry_wrprotect(entry_mkwrite(entry_wrprotect(entry))).writable,
        wrprotect_clean_is_clean: !entry_wrprotect(entry_mkclean(entry)).dirty,
        wrprotect_dirty_stays_dirty: entry_wrprotect(entry_mkdirty(entry)).dirty,
    }
}

pub fn debug_vm_pgtable_test_plan(
    transparent_hugepage: bool,
    transparent_pud: bool,
    huge_vmap: bool,
    hugetlb: bool,
) -> Vec<&'static str> {
    let mut tests = Vec::new();
    tests.extend([
        "pte_basic_tests",
        "p4d_basic_tests",
        "pgd_basic_tests",
        "pte_special_tests",
        "pte_protnone_tests",
        "pte_soft_dirty_tests",
        "pte_swap_soft_dirty_tests",
        "pte_swap_exclusive_tests",
        "pte_swap_tests",
        "swap_migration_tests",
        "pte_clear_tests",
        "pte_advanced_tests",
        "pmd_clear_tests",
        "pmd_populate_tests",
        "pud_clear_tests",
        "pud_populate_tests",
        "p4d_clear_tests",
        "pgd_clear_tests",
        "p4d_populate_tests",
        "pgd_populate_tests",
    ]);
    if transparent_hugepage {
        tests.extend([
            "pmd_basic_tests",
            "pmd_advanced_tests",
            "pmd_leaf_tests",
            "pmd_protnone_tests",
            "pmd_soft_dirty_tests",
            "pmd_leaf_soft_dirty_tests",
            "pmd_softleaf_tests",
            "pmd_thp_tests",
        ]);
    }
    if transparent_hugepage && transparent_pud {
        tests.extend([
            "pud_basic_tests",
            "pud_advanced_tests",
            "pud_leaf_tests",
            "pud_thp_tests",
        ]);
    }
    if huge_vmap {
        tests.extend(["pmd_huge_tests", "pud_huge_tests"]);
    }
    if hugetlb {
        tests.push("hugetlb_basic_tests");
    }
    tests
}

pub const fn phys_align_check(pstart: u64, pend: u64, psize: u64) -> Option<(u64, u64)> {
    let start = if pstart == 0 { PAGE_SIZE } else { pstart };
    let aligned_start = align(start, psize);
    let aligned_end = aligned_start + psize;
    if aligned_end > aligned_start && aligned_end <= pend {
        Some((aligned_start, psize))
    } else {
        None
    }
}

pub fn init_fixed_pfns(ranges: &[(u64, u64)], start_kernel_phys: u64) -> FixedPfns {
    let mut phys = start_kernel_phys;
    let mut fixed_alignment = PAGE_SIZE;

    for &(pstart, pend) in ranges {
        if let Some((candidate, alignment)) = phys_align_check(pstart, pend, PUD_SIZE) {
            phys = candidate;
            fixed_alignment = alignment;
        }
        if fixed_alignment == PUD_SIZE {
            break;
        }
        if fixed_alignment < PMD_SIZE {
            if let Some((candidate, alignment)) = phys_align_check(pstart, pend, PMD_SIZE) {
                phys = candidate;
                fixed_alignment = alignment;
            }
        }
    }

    FixedPfns {
        fixed_alignment,
        fixed_pgd_pfn: (phys & !(PUD_SIZE * 512 - 1)) >> PAGE_SHIFT,
        fixed_p4d_pfn: (phys & !(PUD_SIZE * 512 - 1)) >> PAGE_SHIFT,
        fixed_pud_pfn: (phys & !(PUD_SIZE - 1)) >> PAGE_SHIFT,
        fixed_pmd_pfn: (phys & !(PMD_SIZE - 1)) >> PAGE_SHIFT,
        fixed_pte_pfn: (phys & !(PAGE_SIZE - 1)) >> PAGE_SHIFT,
    }
}

pub const fn align(value: u64, alignment: u64) -> u64 {
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgtable_basic_semantics_match_linux_debug_test_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/debug_vm_pgtable.c"
        ));
        let docs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/Documentation/mm/arch_pgtable_helpers.rst"
        ));

        assert!(source.contains("#define RANDOM_NZVALUE\tGENMASK(7, 0)"));
        assert!(source.contains("static void __init pte_basic_tests"));
        assert!(source.contains("WARN_ON(!pte_same(pte, pte));"));
        assert!(source.contains("WARN_ON(!pte_young(pte_mkyoung(pte_mkold(pte))));"));
        assert!(source.contains("WARN_ON(!pte_dirty(pte_mkdirty(pte_mkclean(pte))));"));
        assert!(
            source.contains("WARN_ON(!pte_write(pte_mkwrite(pte_wrprotect(pte), args->vma)));")
        );
        assert!(source.contains("#define VM_FLAGS_START\t(VM_NONE)"));
        assert!(
            source.contains("#define VM_FLAGS_END\t(VM_SHARED | VM_EXEC | VM_WRITE | VM_READ)")
        );
        assert!(source.contains("late_initcall(debug_vm_pgtable);"));
        assert!(docs.contains("PTE Page Table Helpers"));

        let result = basic_entry_tests(pfn_entry(7));
        assert!(result.same);
        assert!(result.young_round_trip);
        assert!(result.dirty_round_trip);
        assert!(result.writable_round_trip);
        assert!(result.wrprotect_clean_is_clean);
        assert!(result.wrprotect_dirty_stays_dirty);
        assert_eq!(VM_FLAGS_START, 0);
        assert_eq!(VM_FLAGS_END, 0xf);
    }

    #[test]
    fn fixed_pfn_alignment_follows_linux_mem_range_search() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/debug_vm_pgtable.c"
        ));
        assert!(source.contains("if (pstart == 0)"));
        assert!(source.contains("aligned_start = ALIGN(pstart, psize);"));
        assert!(source.contains("phys_align_check(pstart, pend, PUD_SIZE"));
        assert!(source.contains("phys_align_check(pstart, pend, PMD_SIZE"));

        assert_eq!(
            phys_align_check(0, PAGE_SIZE * 2, PAGE_SIZE),
            Some((PAGE_SIZE, PAGE_SIZE))
        );
        assert_eq!(phys_align_check(0x1234, 0x2000, PAGE_SIZE), None);

        let pfns = init_fixed_pfns(
            &[(0x1000, PMD_SIZE * 2), (PUD_SIZE, PUD_SIZE * 2)],
            0xdead_000,
        );
        assert_eq!(pfns.fixed_alignment, PUD_SIZE);
        assert_eq!(pfns.fixed_pud_pfn, PUD_SIZE >> PAGE_SHIFT);

        let plan = debug_vm_pgtable_test_plan(true, true, true, true);
        assert!(plan.contains(&"pte_basic_tests"));
        assert!(plan.contains(&"pmd_thp_tests"));
        assert!(plan.contains(&"pud_thp_tests"));
        assert!(plan.contains(&"hugetlb_basic_tests"));
    }
}
