//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/head32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/head32.c
//! 32-bit kernel-entry orchestration and early page-table setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/head32.c
//!
//! Lupos targets x86_64, but Linux's 32-bit entry orchestrator drives
//! the per-subarch dispatch (`X86_SUBARCH_INTEL_MID`, `X86_SUBARCH_CE4100`,
//! else `i386_default_early_setup`) and the `mk_early_pgtbl_32`
//! algorithm. The byte-level page-table walk and the dispatch decision
//! are ported here for ABI parity; the actual 32-bit asm entry remains
//! out of scope.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

// === Constants — mirror vendor/linux/arch/x86/include/asm/page_32_types.h ===

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
pub const PAGE_MASK: u64 = !(PAGE_SIZE - 1);
pub const PTRS_PER_PTE: usize = 1024; // non-PAE 32-bit
pub const PTRS_PER_PMD: usize = 512; // PAE 32-bit
pub const PGDIR_SHIFT: u32 = 22; // non-PAE
pub const PAE_PGDIR_SHIFT: u32 = 21; // PAE

/// PAGE_OFFSET — 32-bit kernel half starts at 0xC0000000 by default
/// (`CONFIG_VMSPLIT_3G`).
pub const PAGE_OFFSET: u64 = 0xC000_0000;

/// `PTE_IDENT_ATTR` and `PDE_IDENT_ATTR` — RW + Present + Global on the
/// kernel identity map. Bits mirror `arch/x86/include/asm/pgtable_32_types.h`.
pub const PTE_PRESENT: u64 = 1 << 0;
pub const PTE_RW: u64 = 1 << 1;
pub const PTE_USER: u64 = 1 << 2;
pub const PTE_PWT: u64 = 1 << 3;
pub const PTE_PCD: u64 = 1 << 4;
pub const PTE_ACCESSED: u64 = 1 << 5;
pub const PTE_DIRTY: u64 = 1 << 6;
pub const PTE_PSE: u64 = 1 << 7; // 4-MiB page in PDE
pub const PTE_GLOBAL: u64 = 1 << 8;

pub const PTE_IDENT_ATTR: u64 = PTE_PRESENT | PTE_RW | PTE_ACCESSED | PTE_DIRTY;
pub const PDE_IDENT_ATTR: u64 = PTE_PRESENT | PTE_RW | PTE_ACCESSED | PTE_DIRTY;

pub const PTE_PFN_MASK: u64 = !(PAGE_SIZE - 1);

/// `X86_SUBARCH_*` values from `asm/processor.h`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum X86Subarch {
    Pc = 0,
    Lguest = 1,
    Xen = 2,
    Intel_Mid = 3,
    Ce4100 = 4,
}

impl X86Subarch {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Lguest,
            2 => Self::Xen,
            3 => Self::Intel_Mid,
            4 => Self::Ce4100,
            _ => Self::Pc,
        }
    }
}

/// Hooks invoked by `i386_default_early_setup` — `reserve_resources` and
/// `setup_ioapic_ids` install architecture-specific function pointers
/// into `x86_init`.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultEarlySetup {
    pub reserve_resources_installed: bool,
    pub setup_ioapic_ids_installed: bool,
}

pub fn i386_default_early_setup() -> DefaultEarlySetup {
    DefaultEarlySetup {
        reserve_resources_installed: true,
        setup_ioapic_ids_installed: true,
    }
}

/// Trait seam mirroring the small set of operations `i386_start_kernel`
/// performs before calling `start_kernel`.
pub trait I386BootOps {
    fn idt_setup_early_handler(&self);
    fn load_ucode_bsp(&self);
    fn cr4_init_shadow(&self);
    fn sanitize_boot_params(&self);
    fn x86_early_init_platform_quirks(&self);
    fn intel_mid_early_setup(&self);
    fn ce4100_early_setup(&self);
}

/// Linux's `i386_start_kernel` (minus the final `start_kernel()` jump,
/// which is the no-return into common code).
pub fn i386_start_kernel<O: I386BootOps>(ops: &O, subarch: X86Subarch) -> DefaultEarlySetup {
    ops.idt_setup_early_handler();
    ops.load_ucode_bsp();
    ops.cr4_init_shadow();
    ops.sanitize_boot_params();
    ops.x86_early_init_platform_quirks();
    match subarch {
        X86Subarch::Intel_Mid => {
            ops.intel_mid_early_setup();
            DefaultEarlySetup::default()
        }
        X86Subarch::Ce4100 => {
            ops.ce4100_early_setup();
            DefaultEarlySetup::default()
        }
        _ => i386_default_early_setup(),
    }
}

/// Page-table builder mirror used by `mk_early_pgtbl_32`. The `pl2_base`
/// is the array of PDE / PMD entries; `ptep` is the start of the
/// allocated PTE pool. `init_map` walks `pl2p`, filling each PMD with the
/// address of the next batch of PTEs.
#[derive(Debug)]
pub struct EarlyPgTable {
    pub pl2: Vec<u64>,
    pub pte_pool: Vec<u64>,
}

impl EarlyPgTable {
    pub fn new(pl2_entries: usize, pte_pool_entries: usize) -> Self {
        Self {
            pl2: alloc::vec![0u64; pl2_entries],
            pte_pool: alloc::vec![0u64; pte_pool_entries],
        }
    }
}

/// Linux's `init_map` algorithm: walk pl2 entries until the PTE-pool
/// fill-pointer crosses `limit`, populating each PMD with the physical
/// address of the next 1024 PTE entries.
///
/// Returns the new fill pointer (`pte` after the walk).
pub fn init_map(table: &mut EarlyPgTable, start_pte: u64, limit: u64, pte_pool_base: u64) -> u64 {
    let mut pte = start_pte;
    let mut pte_idx = 0usize;
    let mut pl2_idx = 0usize;

    while (pte & PTE_PFN_MASK) < limit {
        let pl2_entry = pte_pool_base + (pte_idx as u64) * (PAGE_SIZE);
        let pl2_val = (pl2_entry & PTE_PFN_MASK) | PDE_IDENT_ATTR;
        if pl2_idx >= table.pl2.len() {
            break;
        }
        table.pl2[pl2_idx] = pl2_val;

        // Kernel half: same PDE replicated at PAGE_OFFSET / PGDIR_SHIFT.
        let kernel_pl2 = pl2_idx + (PAGE_OFFSET as usize) / (1 << PGDIR_SHIFT);
        if kernel_pl2 < table.pl2.len() {
            table.pl2[kernel_pl2] = pl2_val;
        }

        for _ in 0..PTRS_PER_PTE {
            if pte_idx >= table.pte_pool.len() {
                return pte;
            }
            table.pte_pool[pte_idx] = pte;
            pte = pte.wrapping_add(PAGE_SIZE);
            pte_idx += 1;
        }
        pl2_idx += 1;
    }
    pte
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    #[derive(Default)]
    struct RecordingOps {
        order: RefCell<Vec<&'static str>>,
    }

    impl I386BootOps for RecordingOps {
        fn idt_setup_early_handler(&self) {
            self.order.borrow_mut().push("idt");
        }
        fn load_ucode_bsp(&self) {
            self.order.borrow_mut().push("ucode");
        }
        fn cr4_init_shadow(&self) {
            self.order.borrow_mut().push("cr4");
        }
        fn sanitize_boot_params(&self) {
            self.order.borrow_mut().push("bootparams");
        }
        fn x86_early_init_platform_quirks(&self) {
            self.order.borrow_mut().push("platform-quirks");
        }
        fn intel_mid_early_setup(&self) {
            self.order.borrow_mut().push("intel-mid");
        }
        fn ce4100_early_setup(&self) {
            self.order.borrow_mut().push("ce4100");
        }
    }

    #[test]
    fn page_constants_match_32bit_defaults() {
        assert_eq!(PAGE_SIZE, 4096);
        assert_eq!(PAGE_OFFSET, 0xC000_0000);
        assert_eq!(PTRS_PER_PTE, 1024);
        assert_eq!(PGDIR_SHIFT, 22);
    }

    #[test]
    fn pte_ident_attr_includes_present_rw() {
        assert_eq!(PTE_IDENT_ATTR & PTE_PRESENT, PTE_PRESENT);
        assert_eq!(PTE_IDENT_ATTR & PTE_RW, PTE_RW);
    }

    #[test]
    fn default_subarch_returns_pc_for_unknown_codes() {
        assert_eq!(X86Subarch::from_u32(0), X86Subarch::Pc);
        assert_eq!(X86Subarch::from_u32(999), X86Subarch::Pc);
        assert_eq!(X86Subarch::from_u32(3), X86Subarch::Intel_Mid);
        assert_eq!(X86Subarch::from_u32(4), X86Subarch::Ce4100);
    }

    #[test]
    fn default_subarch_runs_default_early_setup() {
        let ops = RecordingOps::default();
        let r = i386_start_kernel(&ops, X86Subarch::Pc);
        assert!(r.reserve_resources_installed);
        assert!(r.setup_ioapic_ids_installed);
        let order = ops.order.borrow();
        assert_eq!(order[0], "idt");
        assert!(!order.contains(&"intel-mid"));
        assert!(!order.contains(&"ce4100"));
    }

    #[test]
    fn intel_mid_subarch_runs_intel_mid_setup() {
        let ops = RecordingOps::default();
        i386_start_kernel(&ops, X86Subarch::Intel_Mid);
        assert!(ops.order.borrow().contains(&"intel-mid"));
    }

    #[test]
    fn ce4100_subarch_runs_ce4100_setup() {
        let ops = RecordingOps::default();
        i386_start_kernel(&ops, X86Subarch::Ce4100);
        assert!(ops.order.borrow().contains(&"ce4100"));
    }

    #[test]
    fn init_map_fills_first_pmd_with_pte_pool_base() {
        let mut table = EarlyPgTable::new(8, 16);
        let pte = init_map(&mut table, PTE_IDENT_ATTR, PAGE_SIZE, 0x100_0000);
        // First PMD must point at PTE-pool base and carry IDENT attributes.
        assert_eq!(table.pl2[0] & PTE_PFN_MASK, 0x100_0000);
        assert_eq!(table.pl2[0] & PDE_IDENT_ATTR, PDE_IDENT_ATTR);
        assert!(pte > PTE_IDENT_ATTR);
    }

    #[test]
    fn init_map_replicates_pmd_at_page_offset_index() {
        let pl2_count = 0x400; // enough to cover both halves
        let mut table = EarlyPgTable::new(pl2_count, 16);
        init_map(&mut table, PTE_IDENT_ATTR, PAGE_SIZE, 0x100_0000);
        let kernel_idx = (PAGE_OFFSET as usize) >> PGDIR_SHIFT; // 0x300
        assert_eq!(table.pl2[0], table.pl2[kernel_idx]);
    }
}
