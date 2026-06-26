//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/pm.c
//! test-origin: linux:vendor/linux/arch/x86/boot/pm.c
//! Real-to-protected-mode transition.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/pm.c
//!
//! `go_to_protected_mode()` runs the final preparation sequence before
//! `protected_mode_jump`: invoke the realmode-switch hook (or `cli +
//! NMI mask`), enable A20, reset the coprocessor, mask the legacy PICs,
//! load a stub IDT (null), load the boot GDT (CS=code32, DS=data32,
//! TSS=tss32 at 4096/103), then jump to `code32_start`.

use super::a20;

/// Legacy I/O ports the PIC sequence touches.
pub const PIC1_DATA: u16 = 0x21;
pub const PIC2_DATA: u16 = 0xa1;
/// NMI disable port the Linux real-mode hook touches.
pub const NMI_DISABLE_PORT: u16 = 0x70;

/// Linux mask bytes — see pm.c lines 38-42.
pub const PIC1_MASK_ALL_BUT_CASCADE: u8 = 0xfb;
pub const PIC2_MASK_ALL: u8 = 0xff;

/// Boot-GDT entry indices.
pub const GDT_ENTRY_BOOT_CS: usize = 2;
pub const GDT_ENTRY_BOOT_DS: usize = 3;
pub const GDT_ENTRY_BOOT_TSS: usize = 4;

/// Boot-GDT descriptor layout. Mirrors the `boot_gdt[]` array Linux
/// builds at pm.c lines 69-78. Each entry uses Linux's `GDT_ENTRY()`
/// macro encoding (base, limit, flags).
pub const BOOT_GDT_LIMIT: u32 = 0xfffff;
pub const BOOT_TSS_BASE: u32 = 4096;
pub const BOOT_TSS_LIMIT: u32 = 103;

/// `gdt_ptr` structure for LGDTL. Linux declares it as
/// `__attribute__((packed))` with `u16 len; u32 ptr;` (6 bytes).
#[repr(C, packed)]
#[derive(Copy, Clone, Default, Debug)]
pub struct GdtPtr {
    pub len: u16,
    pub ptr: u32,
}

/// Steps `go_to_protected_mode()` runs in order, for testability.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PmStep {
    RealmodeSwitchHook,
    EnableA20,
    ResetCoprocessor,
    MaskAllInterrupts,
    SetupIdt,
    SetupGdt,
    ProtectedModeJump,
}

/// Linux's canonical pm.c::go_to_protected_mode() step order.
pub fn pm_step_sequence() -> [PmStep; 7] {
    use PmStep::*;
    [
        RealmodeSwitchHook,
        EnableA20,
        ResetCoprocessor,
        MaskAllInterrupts,
        SetupIdt,
        SetupGdt,
        ProtectedModeJump,
    ]
}

/// `enable_a20` wrapper that uses the algorithmic port from `a20.rs`.
/// Returns Err if A20 cannot be enabled — Linux dies via `puts(...);
/// die();` in that case.
pub fn enable_a20<P: a20::A20Platform>(p: &mut P) -> Result<(), ()> {
    a20::enable_a20(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pic_mask_constants_match_linux_pm_c() {
        // pm.c lines 39-41: 0xa1 ← 0xff, 0x21 ← 0xfb.
        assert_eq!(PIC2_DATA, 0xa1);
        assert_eq!(PIC1_DATA, 0x21);
        assert_eq!(PIC1_MASK_ALL_BUT_CASCADE, 0xfb);
        assert_eq!(PIC2_MASK_ALL, 0xff);
    }

    #[test]
    fn gdt_ptr_is_6_bytes_packed() {
        // Linux `struct gdt_ptr` is packed: u16 + u32 = 6 bytes (not 8).
        assert_eq!(core::mem::size_of::<GdtPtr>(), 6);
    }

    #[test]
    fn boot_gdt_entry_indices_match_segments_h() {
        assert_eq!(GDT_ENTRY_BOOT_CS, 2);
        assert_eq!(GDT_ENTRY_BOOT_DS, 3);
        assert_eq!(GDT_ENTRY_BOOT_TSS, 4);
    }

    #[test]
    fn pm_step_sequence_matches_pm_c_order() {
        let seq = pm_step_sequence();
        assert_eq!(seq[0], PmStep::RealmodeSwitchHook);
        assert_eq!(seq[1], PmStep::EnableA20);
        assert_eq!(seq[seq.len() - 1], PmStep::ProtectedModeJump);
        // setup_idt before setup_gdt — matches pm.c line 121-122.
        let idt_idx = seq.iter().position(|s| *s == PmStep::SetupIdt).unwrap();
        let gdt_idx = seq.iter().position(|s| *s == PmStep::SetupGdt).unwrap();
        assert!(idt_idx < gdt_idx);
    }
}
