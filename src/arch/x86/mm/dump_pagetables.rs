//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/dump_pagetables.c
//! test-origin: linux:vendor/linux/arch/x86/mm/dump_pagetables.c
//! Page-table dump and W+X detection helpers.
//!
//! Mirrors the protection decoding and W+X checks in
//! `vendor/linux/arch/x86/mm/dump_pagetables.c`. The live page-table walker
//! remains in `crate::mm::pagewalk`; this module provides pure decoding
//! that both debugfs and tests can share.

use crate::arch::x86::mm::paging::{
    _PAGE_GLOBAL, _PAGE_NX, _PAGE_PAT, _PAGE_PCD, _PAGE_PRESENT, _PAGE_PSE, _PAGE_PWT, _PAGE_RW,
    _PAGE_USER, PAGE_SIZE, PMD_SIZE, PTE_FLAGS_MASK, PUD_SIZE, p4d_t, p4d_val, pgd_t, pgd_val,
    pmd_huge, pmd_t, pmd_val, pte_t, pte_val, pud_huge, pud_t, pud_val,
};
use crate::mm::pagewalk::{MmWalk, MmWalkOps, PageWalkAction, walk_kernel_page_table_range};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DumpLevel {
    Pgd,
    P4d,
    Pud,
    Pmd,
    Pte,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtSummary {
    pub present: bool,
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub global: bool,
    pub huge: bool,
    pub cache_bits: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageDumpEntry {
    pub start: u64,
    pub end: u64,
    pub level: DumpLevel,
    pub prot: ProtSummary,
    pub wx: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WxAuditResult {
    pub checked_pages: u64,
    pub wx_pages: u64,
    pub first_wx_address: Option<u64>,
}

impl WxAuditResult {
    pub const fn passed(self) -> bool {
        self.wx_pages == 0
    }

    pub const fn merge(self, other: Self) -> Self {
        Self {
            checked_pages: self.checked_pages + other.checked_pages,
            wx_pages: self.wx_pages + other.wx_pages,
            first_wx_address: match self.first_wx_address {
                Some(addr) => Some(addr),
                None => other.first_wx_address,
            },
        }
    }
}

pub const X86_USER_SPACE_END: u64 = 0x0000_8000_0000_0000;
pub const X86_KERNEL_SPACE_START: u64 = 0xffff_8000_0000_0000;
pub const X86_KERNEL_SPACE_END: u64 = 0xffff_ffff_ffff_f000;

pub const fn decode_prot(raw: u64) -> ProtSummary {
    ProtSummary {
        present: raw & _PAGE_PRESENT != 0,
        writable: raw & _PAGE_RW != 0,
        executable: raw & _PAGE_NX == 0,
        user: raw & _PAGE_USER != 0,
        global: raw & _PAGE_GLOBAL != 0,
        huge: raw & _PAGE_PSE != 0,
        cache_bits: raw & (_PAGE_PAT | _PAGE_PCD | _PAGE_PWT),
    }
}

pub const fn is_wx(raw: u64) -> bool {
    let prot = decode_prot(raw);
    prot.present && prot.writable && prot.executable
}

pub const fn note_page(start: u64, end: u64, level: DumpLevel, raw: u64) -> PageDumpEntry {
    PageDumpEntry {
        start,
        end,
        level,
        prot: decode_prot(raw),
        wx: is_wx(raw),
    }
}

pub const fn ptdump_entries_checkwx(entries: &[PageDumpEntry]) -> bool {
    let mut i = 0;
    while i < entries.len() {
        if entries[i].wx {
            return false;
        }
        i += 1;
    }
    true
}

pub const fn effective_prot(parent: u64, raw: u64, level: usize) -> u64 {
    let prot = raw & PTE_FLAGS_MASK;
    if level == 0 {
        return prot;
    }

    (parent & prot & (_PAGE_PRESENT | _PAGE_USER | _PAGE_RW)) | ((parent | prot) & _PAGE_NX)
}

const fn pages_spanned(start: u64, end: u64) -> u64 {
    (end - start) / PAGE_SIZE
}

struct WxAudit {
    prot_levels: [u64; 5],
    result: WxAuditResult,
}

impl WxAudit {
    const fn new() -> Self {
        Self {
            prot_levels: [0; 5],
            result: WxAuditResult {
                checked_pages: 0,
                wx_pages: 0,
                first_wx_address: None,
            },
        }
    }

    fn record_effective(&mut self, level: usize, raw: u64) -> u64 {
        let parent = if level == 0 {
            0
        } else {
            self.prot_levels[level - 1]
        };
        let effective = effective_prot(parent, raw, level);
        self.prot_levels[level] = effective;
        effective
    }

    fn note_leaf(&mut self, start: u64, end: u64, raw: u64, effective: u64) {
        if raw & _PAGE_PRESENT == 0 {
            return;
        }

        let pages = pages_spanned(start, end);
        self.result.checked_pages += pages;
        if effective & _PAGE_RW != 0 && effective & _PAGE_NX == 0 {
            self.result.wx_pages += pages;
            if self.result.first_wx_address.is_none() {
                self.result.first_wx_address = Some(start);
            }
        }
    }
}

impl MmWalkOps for WxAudit {
    fn pgd_entry(
        &mut self,
        pgd: *mut pgd_t,
        _addr: u64,
        _next: u64,
        _walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        self.record_effective(0, unsafe { pgd_val(*pgd) });
        Ok(())
    }

    fn p4d_entry(
        &mut self,
        p4d: *mut p4d_t,
        _addr: u64,
        _next: u64,
        _walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        self.record_effective(1, unsafe { p4d_val(*p4d) });
        Ok(())
    }

    fn pud_entry(
        &mut self,
        pud: *mut pud_t,
        addr: u64,
        next: u64,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        let pud = unsafe { *pud };
        let effective = self.record_effective(2, pud_val(pud));
        if pud_huge(pud) {
            self.note_leaf(addr, next.min(addr + PUD_SIZE), pud_val(pud), effective);
            walk.action = PageWalkAction::Continue;
        }
        Ok(())
    }

    fn pmd_entry(
        &mut self,
        pmd: *mut pmd_t,
        addr: u64,
        next: u64,
        walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        let pmd = unsafe { *pmd };
        let effective = self.record_effective(3, pmd_val(pmd));
        if pmd_huge(pmd) {
            self.note_leaf(addr, next.min(addr + PMD_SIZE), pmd_val(pmd), effective);
            walk.action = PageWalkAction::Continue;
        }
        Ok(())
    }

    fn pte_entry(
        &mut self,
        pte: *mut pte_t,
        addr: u64,
        next: u64,
        _walk: &mut MmWalk<'_>,
    ) -> Result<(), i32> {
        let raw = unsafe { pte_val(*pte) };
        let effective = self.record_effective(4, raw);
        self.note_leaf(addr, next, raw, effective);
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

/// Walk a page-table root and count writable executable leaf mappings.
///
/// Mirrors Linux `ptdump_walk_pgd_level_core(..., checkwx=true)`: callbacks
/// compute effective permissions from parent levels, then W+X accounting is
/// done only on leaf mappings.
///
/// # Safety
/// `pgd` must point to a live PGD for the whole walk.
pub unsafe fn ptdump_walk_pgd_level_checkwx_range(
    pgd: *mut pgd_t,
    start: u64,
    end: u64,
) -> Result<WxAuditResult, i32> {
    let mut audit = WxAudit::new();
    unsafe {
        walk_kernel_page_table_range(start, end, &mut audit, pgd, core::ptr::null_mut())?;
    }
    Ok(audit.result)
}

/// Walk the canonical x86_64 user and kernel halves and account W+X leaves.
///
/// Mirrors Linux `ptdump_walk_pgd_level_checkwx()`. The guard hole is skipped
/// by scanning the lower canonical half and the higher canonical half as two
/// ranges.
///
/// # Safety
/// `pgd` must point to a live PGD for the whole walk.
pub unsafe fn ptdump_walk_pgd_level_checkwx(pgd: *mut pgd_t) -> Result<WxAuditResult, i32> {
    let low = unsafe { ptdump_walk_pgd_level_checkwx_range(pgd, 0, X86_USER_SPACE_END)? };
    let high = unsafe {
        ptdump_walk_pgd_level_checkwx_range(pgd, X86_KERNEL_SPACE_START, X86_KERNEL_SPACE_END)?
    };
    Ok(low.merge(high))
}

#[cfg(not(test))]
pub fn ptdump_check_wx() -> bool {
    use crate::arch::x86::mm::paging::{init_pgd_phys, phys_to_virt};

    let pgd = phys_to_virt(init_pgd_phys()) as *mut pgd_t;
    match unsafe { ptdump_walk_pgd_level_checkwx(pgd) } {
        Ok(result) if result.passed() => {
            crate::log_info!(
                "",
                "x86/mm: Checked W+X mappings: passed, no W+X pages found."
            );
            true
        }
        Ok(result) => {
            if let Some(addr) = result.first_wx_address {
                crate::log_warn!(
                    "",
                    "x86/mm: Found insecure W+X mapping at address {:#x}",
                    addr
                );
            }
            crate::log_info!(
                "",
                "x86/mm: Checked W+X mappings: FAILED, {} W+X pages found.",
                result.wx_pages
            );
            false
        }
        Err(err) => {
            crate::log_warn!("", "x86/mm: W+X page-table audit failed: {}", err);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::{
        __pgprot, __pmd, _PAGE_ACCESSED, _PAGE_NX, _PAGE_PRESENT, _PAGE_PSE, _PAGE_RW, PAGE_KERNEL,
        init_pgd_for_test, map_kernel_page, pgd_offset_pgd, pmd_alloc_kernel, pud_alloc_kernel,
        reset_test_pool, set_pmd,
    };
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

    #[test]
    fn writable_executable_entry_is_reported() {
        let raw = _PAGE_PRESENT | _PAGE_RW;
        let entry = note_page(0x1000, 0x2000, DumpLevel::Pte, raw);
        assert!(entry.wx);
        assert!(!ptdump_entries_checkwx(&[entry]));
    }

    #[test]
    fn nx_or_readonly_entries_are_not_wx() {
        assert!(!is_wx(_PAGE_PRESENT | _PAGE_RW | _PAGE_NX));
        assert!(!is_wx(_PAGE_PRESENT));
    }

    #[test]
    fn effective_prot_inherits_nx_and_intersects_rw() {
        let parent = _PAGE_PRESENT | _PAGE_RW | _PAGE_NX;
        let leaf = _PAGE_PRESENT | _PAGE_RW;
        let eff = effective_prot(parent, leaf, 4);
        assert_ne!(eff & _PAGE_NX, 0);
        assert_ne!(eff & _PAGE_RW, 0);

        let ro_parent = _PAGE_PRESENT | _PAGE_NX;
        let eff = effective_prot(ro_parent, leaf, 4);
        assert_eq!(eff & _PAGE_RW, 0);
    }

    #[test]
    fn ptdump_checkwx_passes_for_kernel_nx_mapping() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { reset_test_pool() };

        let virt = 0x0000_0000_0040_0000;
        let phys = 0x0000_0000_0020_0000;
        unsafe { map_kernel_page(virt, phys, PAGE_KERNEL) };

        let pgd = init_pgd_for_test();
        let result =
            unsafe { ptdump_walk_pgd_level_checkwx_range(pgd, virt, virt + PAGE_SIZE).unwrap() };
        assert!(result.passed());
        assert_eq!(result.checked_pages, 1);
        assert_eq!(result.wx_pages, 0);
    }

    #[test]
    fn ptdump_checkwx_reports_rwx_pte() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { reset_test_pool() };

        let virt = 0x0000_0000_0050_0000;
        let phys = 0x0000_0000_0030_0000;
        unsafe {
            map_kernel_page(
                virt,
                phys,
                __pgprot(_PAGE_PRESENT | _PAGE_RW | _PAGE_ACCESSED),
            )
        };

        let pgd = init_pgd_for_test();
        let result =
            unsafe { ptdump_walk_pgd_level_checkwx_range(pgd, virt, virt + PAGE_SIZE).unwrap() };
        assert!(!result.passed());
        assert_eq!(result.checked_pages, 1);
        assert_eq!(result.wx_pages, 1);
        assert_eq!(result.first_wx_address, Some(virt));
    }

    #[test]
    fn ptdump_checkwx_reports_huge_pmd_pages() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { reset_test_pool() };

        let virt = 0x0000_0000_0080_0000;
        let phys = 0x0000_0000_0060_0000;
        unsafe {
            let pgd = init_pgd_for_test();
            let pgdp = pgd_offset_pgd(pgd, virt);
            let pudp = pud_alloc_kernel(pgdp, virt).expect("pud_alloc_kernel");
            let pmdp = pmd_alloc_kernel(pudp, virt).expect("pmd_alloc_kernel");
            set_pmd(pmdp, __pmd(phys | _PAGE_PRESENT | _PAGE_RW | _PAGE_PSE));
        }

        let pgd = init_pgd_for_test();
        let result =
            unsafe { ptdump_walk_pgd_level_checkwx_range(pgd, virt, virt + PMD_SIZE).unwrap() };
        assert_eq!(result.checked_pages, PMD_SIZE / PAGE_SIZE);
        assert_eq!(result.wx_pages, PMD_SIZE / PAGE_SIZE);
        assert_eq!(result.first_wx_address, Some(virt));
    }
}
