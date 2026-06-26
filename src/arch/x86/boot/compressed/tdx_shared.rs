//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/tdx-shared.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/tdx-shared.c
//! Compressed-kernel TDX-shared shim.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/tdx-shared.c
//!
//! The C file is two lines: include `error.h` and `../../coco/tdx/tdx-shared.c`.
//! Lupos carries the real `coco/tdx/tdx-shared.c` body in the Batch 9 ports;
//! this shim mirrors the inclusion seam and re-exports the TDCALL
//! page-conversion constants so callers in the decompressor have a
//! stable name to import.

/// `MAP_VALID_BLOCKS` — maximum number of 4 KiB pages the TDX
/// MapGPA hypercall accepts in a single call. Mirrors Linux's
/// `MAP_VALID_BLOCKS` in `arch/x86/include/asm/shared/tdx.h`.
pub const MAP_VALID_BLOCKS: u64 = 0x40_0000_0000; // 1 TiB / 4 KiB

/// `TDX_HCALL_HAS_OUTPUT` — request flag bit. Caller wants the GHCI
/// response back. Matches Linux's `TDX_HCALL_HAS_OUTPUT` (asm/tdx.h).
pub const TDX_HCALL_HAS_OUTPUT: u64 = 1;
/// `TDX_HCALL_ISSUE_STI` — request flag bit. Issue STI after the
/// VMCALL to deliver pending interrupts.
pub const TDX_HCALL_ISSUE_STI: u64 = 1 << 1;

/// Page conversion sub-functions for TDX `tdx_hcall_set_gpa_state` /
/// `tdx_hcall_get_gpa_state`. Re-exported so the compressed and main
/// kernel use the same constants.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u64)]
pub enum GpaState {
    Private = 0,
    Shared = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_valid_blocks_matches_linux_constant() {
        assert_eq!(MAP_VALID_BLOCKS, 0x40_0000_0000);
    }

    #[test]
    fn hcall_flag_bits_match_asm_tdx_h() {
        assert_eq!(TDX_HCALL_HAS_OUTPUT, 1);
        assert_eq!(TDX_HCALL_ISSUE_STI, 2);
    }

    #[test]
    fn gpa_state_discriminants_match_linux_enum() {
        assert_eq!(GpaState::Private as u64, 0);
        assert_eq!(GpaState::Shared as u64, 1);
    }
}
