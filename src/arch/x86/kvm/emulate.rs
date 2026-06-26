//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/emulate.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/emulate.c
//! Instruction emulator for I/O and MMIO paths.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/emulate.c

// `emulate.c` decodes the guest instruction stream when a VMEXIT needs
// software emulation (MMIO, port I/O on legacy devices). The decoder
// is gigantic; we model just enough to classify a 1-byte opcode group
// for tests.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmuOp {
    InByte,
    InWord,
    OutByte,
    OutWord,
    MovToCr,
    MovFromCr,
    Unknown,
}

pub const fn classify_opcode(byte: u8) -> EmuOp {
    match byte {
        0xe4 => EmuOp::InByte,
        0xe5 => EmuOp::InWord,
        0xe6 => EmuOp::OutByte,
        0xe7 => EmuOp::OutWord,
        _ => EmuOp::Unknown,
    }
}

pub const fn classify_two_byte_opcode(byte: u8) -> EmuOp {
    match byte {
        0x20 => EmuOp::MovFromCr,
        0x22 => EmuOp::MovToCr,
        _ => EmuOp::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_in_out_instructions() {
        assert_eq!(classify_opcode(0xe4), EmuOp::InByte);
        assert_eq!(classify_opcode(0xe6), EmuOp::OutByte);
        assert_eq!(classify_opcode(0x90), EmuOp::Unknown);
    }

    #[test]
    fn classifies_cr_move_instructions() {
        assert_eq!(classify_two_byte_opcode(0x20), EmuOp::MovFromCr);
        assert_eq!(classify_two_byte_opcode(0x22), EmuOp::MovToCr);
    }
}
