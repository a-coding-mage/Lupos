//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/fred.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/fred.c
//! FRED (Flexible Return and Event Delivery) early-CPU setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/fred.c
//!
//! FRED replaces the legacy IDT-based exception delivery model with an
//! MSR-based one: each CPU writes its entry-point address and per-vector
//! stack-level table into `MSR_IA32_FRED_*`. Lupos' IDT path remains the
//! production dispatcher today; this module captures FRED's MSR layout,
//! `STKLVL` packing, and configuration-register bit fields so the
//! transition can land later without ABI drift.
//!
//! Intel SDM Vol. 3A §6.2 — "Flexible Return and Event Delivery".

#![allow(dead_code)]

// === FRED MSRs — mirror vendor/linux/arch/x86/include/asm/msr-index.h ===

pub const MSR_IA32_FRED_RSP0: u32 = 0x1cc;
pub const MSR_IA32_FRED_RSP1: u32 = 0x1cd;
pub const MSR_IA32_FRED_RSP2: u32 = 0x1ce;
pub const MSR_IA32_FRED_RSP3: u32 = 0x1cf;
pub const MSR_IA32_FRED_STKLVLS: u32 = 0x1d0;
pub const MSR_IA32_FRED_SSP1: u32 = 0x1d1;
pub const MSR_IA32_FRED_SSP2: u32 = 0x1d2;
pub const MSR_IA32_FRED_SSP3: u32 = 0x1d3;
pub const MSR_IA32_FRED_CONFIG: u32 = 0x1d4;

// === CR4 bit — mirror vendor/linux/arch/x86/include/uapi/asm/processor-flags.h ===

pub const X86_CR4_FRED_BIT: u32 = 32;
pub const X86_CR4_FRED: u64 = 1u64 << X86_CR4_FRED_BIT;

// === FRED_CONFIG layout — mirror vendor/linux/arch/x86/include/asm/fred.h ===

pub const FRED_CONFIG_REDZONE_AMOUNT: u64 = 1;
pub const FRED_CONFIG_REDZONE: u64 = FRED_CONFIG_REDZONE_AMOUNT << 6;

pub const fn fred_config_int_stklvl(level: u64) -> u64 {
    level << 9
}

pub const fn fred_config_entrypoint(entry_va: u64) -> u64 {
    entry_va
}

// === STKLVL packing — 2 bits per vector ===

pub const FRED_DB_STACK_LEVEL: u64 = 1;
pub const FRED_NMI_STACK_LEVEL: u64 = 2;
pub const FRED_MC_STACK_LEVEL: u64 = 2;
pub const FRED_DF_STACK_LEVEL: u64 = 3;

/// Vector indices used by FRED's `STKLVLS` table — mirror `asm/trapnr.h`.
pub const X86_TRAP_DB: u32 = 1;
pub const X86_TRAP_NMI: u32 = 2;
pub const X86_TRAP_DF: u32 = 8;
pub const X86_TRAP_MC: u32 = 18;

/// `FRED_STKLVL(vector, lvl)` — pack a level into the `STKLVLS` MSR.
pub const fn fred_stklvl(vector: u32, level: u64) -> u64 {
    level << (2 * vector)
}

/// Compute the full `MSR_IA32_FRED_STKLVLS` value Linux writes from
/// `cpu_init_fred_rsps`.
pub const fn fred_stklvls_default() -> u64 {
    fred_stklvl(X86_TRAP_DB, FRED_DB_STACK_LEVEL)
        | fred_stklvl(X86_TRAP_NMI, FRED_NMI_STACK_LEVEL)
        | fred_stklvl(X86_TRAP_MC, FRED_MC_STACK_LEVEL)
        | fred_stklvl(X86_TRAP_DF, FRED_DF_STACK_LEVEL)
}

/// Compute the `MSR_IA32_FRED_CONFIG` value written from
/// `cpu_init_fred_exceptions`.
pub const fn fred_config(entry_va: u64) -> u64 {
    FRED_CONFIG_REDZONE | fred_config_int_stklvl(0) | fred_config_entrypoint(entry_va)
}

/// Trait seam for `wrmsrq` (`rdmsr` is never needed in this module).
pub trait FredMsr {
    fn wrmsrq(&self, msr: u32, value: u64);
}

/// Trait seam for `cr4_set_bits` and `idt_invalidate`.
pub trait FredCpu {
    fn cr4_set_bits(&self, mask: u64);
    fn idt_invalidate(&self);
    fn load_ss(&self, selector: u16);
    fn this_cpu_fred_rsp0(&self) -> u64;
    fn ist_top_va(&self, vector: u32) -> u64;
}

/// `__KERNEL_DS` GDT selector — bit 3 of the GDT, ring 0.
pub const KERNEL_DS: u16 = 0x10;

/// Linux's `cpu_init_fred_exceptions` — boot/online path that flips a CPU
/// onto FRED.
pub fn cpu_init_fred_exceptions<M, C>(msr: &M, cpu: &C, entry_va: u64)
where
    M: FredMsr,
    C: FredCpu,
{
    cpu.load_ss(KERNEL_DS);
    msr.wrmsrq(MSR_IA32_FRED_CONFIG, fred_config(entry_va));
    msr.wrmsrq(MSR_IA32_FRED_STKLVLS, 0);
    msr.wrmsrq(MSR_IA32_FRED_RSP0, cpu.this_cpu_fred_rsp0());
    msr.wrmsrq(MSR_IA32_FRED_RSP1, 0);
    msr.wrmsrq(MSR_IA32_FRED_RSP2, 0);
    msr.wrmsrq(MSR_IA32_FRED_RSP3, 0);
    cpu.cr4_set_bits(X86_CR4_FRED);
    cpu.idt_invalidate();
}

/// Linux's `cpu_init_fred_rsps` — called after the per-CPU IST stacks
/// have been allocated.
pub fn cpu_init_fred_rsps<M, C>(msr: &M, cpu: &C)
where
    M: FredMsr,
    C: FredCpu,
{
    msr.wrmsrq(MSR_IA32_FRED_STKLVLS, fred_stklvls_default());
    msr.wrmsrq(MSR_IA32_FRED_RSP1, cpu.ist_top_va(X86_TRAP_DB));
    msr.wrmsrq(MSR_IA32_FRED_RSP2, cpu.ist_top_va(X86_TRAP_NMI));
    msr.wrmsrq(MSR_IA32_FRED_RSP3, cpu.ist_top_va(X86_TRAP_DF));
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    extern crate alloc;
    use alloc::vec::Vec;

    #[derive(Default)]
    struct MockMsr {
        writes: RefCell<Vec<(u32, u64)>>,
    }

    impl FredMsr for MockMsr {
        fn wrmsrq(&self, msr: u32, value: u64) {
            self.writes.borrow_mut().push((msr, value));
        }
    }

    #[derive(Default)]
    struct MockCpu {
        cr4_bits: RefCell<u64>,
        idt_invalidated: RefCell<bool>,
        ss: RefCell<u16>,
    }

    impl FredCpu for MockCpu {
        fn cr4_set_bits(&self, mask: u64) {
            *self.cr4_bits.borrow_mut() |= mask;
        }
        fn idt_invalidate(&self) {
            *self.idt_invalidated.borrow_mut() = true;
        }
        fn load_ss(&self, selector: u16) {
            *self.ss.borrow_mut() = selector;
        }
        fn this_cpu_fred_rsp0(&self) -> u64 {
            0xDEAD_BEEF_0000_0000
        }
        fn ist_top_va(&self, vector: u32) -> u64 {
            // Synthetic stack-top: vector * 0x1000.
            (vector as u64) * 0x1000
        }
    }

    #[test]
    fn msr_indices_match_linux_layout() {
        assert_eq!(MSR_IA32_FRED_RSP0, 0x1cc);
        assert_eq!(MSR_IA32_FRED_STKLVLS, 0x1d0);
        assert_eq!(MSR_IA32_FRED_CONFIG, 0x1d4);
    }

    #[test]
    fn cr4_fred_bit_is_32() {
        assert_eq!(X86_CR4_FRED_BIT, 32);
        assert_eq!(X86_CR4_FRED, 1u64 << 32);
    }

    #[test]
    fn fred_config_redzone_sits_at_bit_6() {
        assert_eq!(FRED_CONFIG_REDZONE, 0x40);
    }

    #[test]
    fn stklvl_packs_two_bits_per_vector() {
        // DB(1)=1 → bits 2-3 = 0b01 = 0x4
        assert_eq!(fred_stklvl(X86_TRAP_DB, FRED_DB_STACK_LEVEL), 0x4);
        // NMI(2)=2 → bits 4-5 = 0b10 = 0x20
        assert_eq!(fred_stklvl(X86_TRAP_NMI, FRED_NMI_STACK_LEVEL), 0x20);
        // DF(8)=3 → bits 16-17 = 0b11 = 0x30000
        assert_eq!(fred_stklvl(X86_TRAP_DF, FRED_DF_STACK_LEVEL), 0x3_0000);
    }

    #[test]
    fn stklvls_default_combines_db_nmi_mc_df() {
        let expected = (1u64 << 2)            // DB
            | (2u64 << 4)                     // NMI
            | (2u64 << 36)                    // MC (vector 18 → shift 36)
            | (3u64 << 16); // DF
        assert_eq!(fred_stklvls_default(), expected);
    }

    #[test]
    fn config_word_combines_redzone_and_entrypoint() {
        let entry = 0xFFFF_FFFF_8000_0000;
        let cfg = fred_config(entry);
        assert!(cfg & FRED_CONFIG_REDZONE != 0);
        // Entrypoint must round-trip in the low bits (it's the OR base).
        assert_eq!(cfg & 0x0000_003F, 0);
        assert_eq!(cfg & 0xFFFF_FFFF_0000_0000, entry & 0xFFFF_FFFF_0000_0000);
    }

    #[test]
    fn init_exceptions_writes_msrs_loads_ss_sets_cr4() {
        let msr = MockMsr::default();
        let cpu = MockCpu::default();
        cpu_init_fred_exceptions(&msr, &cpu, 0xFFFF_FFFF_8000_1234);

        let writes = msr.writes.borrow();
        // 6 wrmsrq calls: CONFIG, STKLVLS, RSP0, RSP1, RSP2, RSP3.
        assert_eq!(writes.len(), 6);
        assert_eq!(writes[0].0, MSR_IA32_FRED_CONFIG);
        assert_eq!(writes[1].0, MSR_IA32_FRED_STKLVLS);
        assert_eq!(writes[2].0, MSR_IA32_FRED_RSP0);
        assert_eq!(writes[2].1, 0xDEAD_BEEF_0000_0000);
        assert_eq!(*cpu.ss.borrow(), KERNEL_DS);
        assert_eq!(*cpu.cr4_bits.borrow(), X86_CR4_FRED);
        assert!(*cpu.idt_invalidated.borrow());
    }

    #[test]
    fn init_rsps_writes_stklvls_and_three_rsps() {
        let msr = MockMsr::default();
        let cpu = MockCpu::default();
        cpu_init_fred_rsps(&msr, &cpu);

        let writes = msr.writes.borrow();
        assert_eq!(writes.len(), 4);
        assert_eq!(writes[0].0, MSR_IA32_FRED_STKLVLS);
        assert_eq!(writes[0].1, fred_stklvls_default());
        assert_eq!(writes[1].0, MSR_IA32_FRED_RSP1);
        assert_eq!(writes[1].1, 0x1000);
        assert_eq!(writes[2].0, MSR_IA32_FRED_RSP2);
        assert_eq!(writes[2].1, 0x2000);
        assert_eq!(writes[3].0, MSR_IA32_FRED_RSP3);
        assert_eq!(writes[3].1, 0x8000);
    }
}
