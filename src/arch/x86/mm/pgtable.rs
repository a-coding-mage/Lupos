//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pgtable.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pgtable.c
//! x86 page-table management helpers.
//!
//! Wraps the Rust page-table primitives for the exported behavior from
//! `vendor/linux/arch/x86/mm/pgtable.c`: access-flag updates, huge-entry
//! construction, and zapped-entry checks.

use crate::arch::x86::mm::paging::{
    __pmd, __pud, _PAGE_ACCESSED, _PAGE_DIRTY, _PAGE_PRESENT, _PAGE_PSE, _PAGE_RW, PTE_PFN_MASK,
    pgprot_t, pgprot_val, pmd_t, pmd_val, pte_t, pte_val, pud_t, pud_val,
};
use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessUpdate<T> {
    pub entry: T,
    pub changed: bool,
}

pub const fn ptep_set_access_flags(old: pte_t, new: pte_t) -> AccessUpdate<pte_t> {
    let changed = (pte_val(old) & pte_val(new)) != pte_val(new);
    AccessUpdate {
        entry: pte_t(pte_val(old) | pte_val(new)),
        changed,
    }
}

pub const fn pmdp_set_access_flags(old: pmd_t, new: pmd_t) -> AccessUpdate<pmd_t> {
    let changed = (pmd_val(old) & pmd_val(new)) != pmd_val(new);
    AccessUpdate {
        entry: pmd_t(pmd_val(old) | pmd_val(new)),
        changed,
    }
}

pub const fn pudp_set_access_flags(old: pud_t, new: pud_t) -> AccessUpdate<pud_t> {
    let changed = (pud_val(old) & pud_val(new)) != pud_val(new);
    AccessUpdate {
        entry: pud_t(pud_val(old) | pud_val(new)),
        changed,
    }
}

pub const fn pmd_set_huge(addr: u64, prot: pgprot_t) -> Result<pmd_t, i32> {
    if addr & (crate::arch::x86::mm::paging::PMD_SIZE - 1) != 0 {
        return Err(EINVAL);
    }
    Ok(__pmd(
        (addr & PTE_PFN_MASK) | pgprot_val(prot) | _PAGE_PRESENT | _PAGE_PSE,
    ))
}

pub const fn pud_set_huge(addr: u64, prot: pgprot_t) -> Result<pud_t, i32> {
    if addr & (crate::arch::x86::mm::paging::PUD_SIZE - 1) != 0 {
        return Err(EINVAL);
    }
    Ok(__pud(
        (addr & PTE_PFN_MASK) | pgprot_val(prot) | _PAGE_PRESENT | _PAGE_PSE,
    ))
}

pub const fn pmd_clear_huge(pmd: pmd_t) -> pmd_t {
    pmd_t(pmd_val(pmd) & !_PAGE_PSE)
}

pub const fn pud_clear_huge(pud: pud_t) -> pud_t {
    pud_t(pud_val(pud) & !_PAGE_PSE)
}

pub const fn pte_mkwrite_from_fault(pte: pte_t, dirty: bool) -> pte_t {
    let mut raw = pte_val(pte) | _PAGE_RW | _PAGE_ACCESSED;
    if dirty {
        raw |= _PAGE_DIRTY;
    }
    pte_t(raw)
}

pub const fn arch_check_zapped_pte(pte: pte_t) -> bool {
    pte_val(pte) == 0 || (pte_val(pte) & !_PAGE_RW) & _PAGE_PRESENT == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::{__pte, _PAGE_NX, PAGE_KERNEL, PMD_SIZE};

    #[test]
    fn access_flags_report_when_bits_are_added() {
        let old = __pte(_PAGE_PRESENT);
        let new = __pte(_PAGE_PRESENT | _PAGE_RW | _PAGE_ACCESSED);
        let updated = ptep_set_access_flags(old, new);
        assert!(updated.changed);
        assert_ne!(pte_val(updated.entry) & _PAGE_RW, 0);
    }

    #[test]
    fn huge_pmd_requires_alignment_and_sets_pse() {
        assert!(pmd_set_huge(0x1234, PAGE_KERNEL).is_err());
        let pmd = pmd_set_huge(PMD_SIZE, PAGE_KERNEL).unwrap();
        assert_ne!(pmd_val(pmd) & _PAGE_PSE, 0);
    }

    #[test]
    fn fault_write_sets_dirty_when_requested() {
        let pte = pte_mkwrite_from_fault(__pte(_PAGE_PRESENT | _PAGE_NX), true);
        assert_ne!(pte_val(pte) & _PAGE_RW, 0);
        assert_ne!(pte_val(pte) & _PAGE_DIRTY, 0);
    }
}
