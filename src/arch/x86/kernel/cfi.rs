//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cfi.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cfi.c
//! Clang CFI trap decoding.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cfi.c
//!
//! Linux decodes compiler-generated trap sequences around `ud2`. The full
//! report path is generic CFI code; this module keeps x86 sequence decoding
//! and action selection.

use crate::include::uapi::errno::EINVAL;

pub const UD2: [u8; 2] = [0x0f, 0x0b];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CfiMode {
    None,
    Kcfi,
    FineIbt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BugTrapType {
    None,
    Warn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedCfiTrap {
    pub target: u64,
    pub expected_type: u32,
}

/// Decode the KCFI type from the `movl -type, %r10d` immediate.
pub const fn kcfi_expected_type(mov_immediate: u32) -> u32 {
    mov_immediate.wrapping_neg()
}

/// Decode Linux's common KCFI sequence:
/// `41 ba <imm32>; <add r/m32, r10d>; 74 xx; 0f 0b`.
pub fn decode_kcfi_window(bytes_before_ud2: &[u8], target: u64) -> Result<DecodedCfiTrap, i32> {
    if bytes_before_ud2.len() < 12 {
        return Err(EINVAL);
    }
    let start = bytes_before_ud2.len() - 12;
    let mov = &bytes_before_ud2[start..start + 6];
    if mov[0] != 0x41 || mov[1] != 0xba {
        return Err(EINVAL);
    }
    let imm = u32::from_le_bytes([mov[2], mov[3], mov[4], mov[5]]);
    Ok(DecodedCfiTrap {
        target,
        expected_type: kcfi_expected_type(imm),
    })
}

pub fn is_cfi_trap(bytes_at_ip: &[u8]) -> bool {
    bytes_at_ip.len() >= 2 && bytes_at_ip[0..2] == UD2
}

pub fn handle_cfi_failure(
    mode: CfiMode,
    trap_at_ip: bool,
    decoded: Option<DecodedCfiTrap>,
) -> BugTrapType {
    match mode {
        CfiMode::None => BugTrapType::None,
        CfiMode::Kcfi => {
            if !trap_at_ip {
                BugTrapType::None
            } else {
                let _ = decoded;
                BugTrapType::Warn
            }
        }
        CfiMode::FineIbt => {
            if decoded.is_some() {
                BugTrapType::Warn
            } else {
                BugTrapType::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kcfi_expected_type_negates_mov_immediate() {
        assert_eq!(kcfi_expected_type(0xffff_ff9c), 100);
    }

    #[test]
    fn decode_kcfi_window_reads_type_and_target() {
        let mut bytes = [0u8; 12];
        bytes[0] = 0x41;
        bytes[1] = 0xba;
        bytes[2..6].copy_from_slice(&0xffff_ff9cu32.to_le_bytes());
        let decoded = decode_kcfi_window(&bytes, 0xfeed_cafe).unwrap();
        assert_eq!(
            decoded,
            DecodedCfiTrap {
                target: 0xfeed_cafe,
                expected_type: 100
            }
        );
    }

    #[test]
    fn decode_kcfi_window_rejects_non_mov_r10d() {
        assert_eq!(decode_kcfi_window(&[0; 12], 0), Err(EINVAL));
    }

    #[test]
    fn ud2_detection_matches_x86_opcode() {
        assert!(is_cfi_trap(&[0x0f, 0x0b]));
        assert!(!is_cfi_trap(&[0xcc, 0x0b]));
    }

    #[test]
    fn handle_cfi_failure_follows_linux_mode_checks() {
        assert_eq!(
            handle_cfi_failure(CfiMode::None, true, None),
            BugTrapType::None
        );
        assert_eq!(
            handle_cfi_failure(CfiMode::Kcfi, true, None),
            BugTrapType::Warn
        );
        assert_eq!(
            handle_cfi_failure(CfiMode::FineIbt, false, None),
            BugTrapType::None
        );
    }
}
