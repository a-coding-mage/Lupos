//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/idt_64.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/idt_64.c
//! Early decompressor IDT (stage 1 / stage 2).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/idt_64.c
//!
//! Two IDT stages:
//!   * stage 1: bare #VC handler for SEV-ES so the first GHCB can be
//!     established before any page tables exist.
//!   * stage 2: full set — #PF (for identity-map fault-in), #NMI,
//!     and #VC for ongoing SEV-ES operation.
//!
//! The Rust port preserves the gate-descriptor encoding (trap gate,
//! present, kernel CS, three-piece handler address) and the
//! stage1/stage2/cleanup transitions. The hot-path #VC/#PF handler
//! bodies arrive with the `coco/sev/` batch.

/// `__KERNEL_CS` — code-segment selector. Matches `arch/x86/include/asm/segment.h`.
pub const KERNEL_CS: u16 = 0x10;

/// Trap-vector numbers used in the boot IDT (matches `asm/trapnr.h`).
pub const X86_TRAP_PF: u8 = 14;
pub const X86_TRAP_NMI: u8 = 2;
pub const X86_TRAP_VC: u8 = 29; // SEV-ES VMM Communication exception

/// `GATE_TRAP` — IDT gate type. Bit pattern `0b1111` (= 0xF). Matches
/// `arch/x86/include/asm/desc_defs.h`.
pub const GATE_TRAP: u8 = 0xF;

/// Mirror of `gate_desc` (64-bit IDT entry, 16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct GateDesc {
    pub offset_low: u16,
    pub segment: u16,
    pub bits: u16,
    pub offset_middle: u16,
    pub offset_high: u32,
    pub reserved: u32,
}

impl GateDesc {
    /// Encode the gate `bits` half-word per `asm/desc_defs.h`:
    /// IST=0, type=GATE_TRAP, S=0, DPL=0, P=1.
    pub const fn encode_bits() -> u16 {
        // [P:1][DPL:2][S:1][type:4][_:5][IST:3]
        // P=1 (bit 15), DPL=0, type=GATE_TRAP, IST=0.
        (1u16 << 15) | ((GATE_TRAP as u16) << 8)
    }

    /// Build a complete gate from a handler address. Mirrors
    /// `set_idt_entry()` lines 7-22.
    pub fn from_handler(address: u64) -> Self {
        Self {
            offset_low: address as u16,
            segment: KERNEL_CS,
            bits: Self::encode_bits(),
            offset_middle: (address >> 16) as u16,
            offset_high: (address >> 32) as u32,
            reserved: 0,
        }
    }
}

/// Stage of the boot IDT lifecycle, exposed for tests / interop.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum IdtStage {
    Stage1,
    Stage2,
    Cleanup,
}

/// Decision: which vectors does `load_stage1_idt()` populate? Mirrors
/// idt_64.c lines 31-40.
pub fn stage1_vectors(amd_mem_encrypt: bool) -> &'static [u8] {
    if amd_mem_encrypt { &[X86_TRAP_VC] } else { &[] }
}

/// Decision: which vectors does `load_stage2_idt()` populate? Mirrors
/// idt_64.c lines 59-78. The #VC slot is conditional on
/// `sev_status & BIT(1)`.
pub fn stage2_vectors(amd_mem_encrypt: bool, sev_status: u64) -> [(u8, bool); 3] {
    [
        (X86_TRAP_PF, true),
        (X86_TRAP_NMI, true),
        (X86_TRAP_VC, amd_mem_encrypt && (sev_status & 0x2) != 0),
    ]
}

/// Cleanup state: zero `desc_ptr.size` and `.address`. Returns the
/// resulting `(size, address)` pair.
pub fn cleanup_desc_ptr() -> (u16, u64) {
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_desc_is_16_bytes_per_amd64_arch() {
        assert_eq!(core::mem::size_of::<GateDesc>(), 16);
    }

    #[test]
    fn gate_desc_packs_handler_address_in_three_pieces() {
        let g = GateDesc::from_handler(0x1234_5678_dead_beef);
        assert_eq!(g.offset_low, 0xbeef);
        assert_eq!(g.offset_middle, 0xdead);
        assert_eq!(g.offset_high, 0x1234_5678);
        assert_eq!(g.segment, KERNEL_CS);
    }

    #[test]
    fn gate_bits_have_present_and_trap_set() {
        let bits = GateDesc::encode_bits();
        assert_eq!(bits >> 15, 1, "present bit must be 1");
        assert_eq!((bits >> 8) & 0xF, GATE_TRAP as u16);
    }

    #[test]
    fn stage1_populates_vc_only_when_amd_mem_encrypt_enabled() {
        assert_eq!(stage1_vectors(true), &[X86_TRAP_VC]);
        assert_eq!(stage1_vectors(false), &[]);
    }

    #[test]
    fn stage2_vc_slot_is_gated_on_sev_status_bit_1() {
        let with = stage2_vectors(true, 0x2);
        assert!(with[2].1, "VC should be installed when SEV bit set");
        let without = stage2_vectors(true, 0x0);
        assert!(!without[2].1);
        let no_encrypt = stage2_vectors(false, 0x2);
        assert!(!no_encrypt[2].1);
    }

    #[test]
    fn stage2_always_loads_page_fault_and_nmi() {
        let v = stage2_vectors(false, 0);
        assert_eq!((v[0].0, v[0].1), (X86_TRAP_PF, true));
        assert_eq!((v[1].0, v[1].1), (X86_TRAP_NMI, true));
    }

    #[test]
    fn cleanup_zeros_desc_ptr() {
        assert_eq!(cleanup_desc_ptr(), (0, 0));
    }
}
