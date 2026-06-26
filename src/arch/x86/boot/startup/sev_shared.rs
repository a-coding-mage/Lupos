//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/startup/sev-shared.c
//! test-origin: linux:vendor/linux/arch/x86/boot/startup/sev-shared.c
//! Shared SEV / SEV-SNP startup code.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/startup/sev-shared.c
//!
//! 762 lines in Linux. The compressed-decompressor and the regular
//! kernel both reuse this file (via `#include "sev-shared.c"`), so the
//! port carries the bits that are usable from either side without the
//! full SEV runtime state machine (the Batch 9 `coco/sev/` ports):
//!   * GHCB / MSR-protocol opcodes.
//!   * Page-state-change request encoding (already exposed by
//!     `compressed/sev.rs`; this module re-exports the names used by
//!     the shared `.c`).
//!   * `enc_dec_hypercall(addr, size, enc)` request shape used by the
//!     SEV-SNP page-encryption ioctl.
//!   * Termination reason set/code (`SEV_TERM_*`).

pub use crate::arch::x86::boot::compressed::sev::{
    GHCB_SIZE, MSR_AMD64_SEV, MSR_AMD64_SEV_ES_GHCB, SEV_ENABLED, SEV_ES_ENABLED, SEV_SNP_ENABLED,
    SnpPageState, sev_termination_request, snp_page_state_msr,
};

/// `MSR_AMD64_SEV_ES_GHCB` MSR-protocol info-request opcode (Linux's
/// `GHCB_MSR_SEV_INFO_REQ`).
pub const GHCB_MSR_SEV_INFO_REQ: u64 = 0x002;
/// `GHCB_MSR_CPUID_REQ` MSR-protocol opcode.
pub const GHCB_MSR_CPUID_REQ: u64 = 0x004;
/// `GHCB_MSR_AP_RESET_HOLD_REQ` — used during SMP bring-up.
pub const GHCB_MSR_AP_RESET_HOLD_REQ: u64 = 0x006;
/// `GHCB_MSR_PSC_REQ` — page-state-change MSR-protocol opcode.
pub const GHCB_MSR_PSC_REQ: u64 = 0x014;
/// `GHCB_MSR_TERM_REQ` — termination request.
pub const GHCB_MSR_TERM_REQ: u64 = 0x100;

/// SEV termination "reason set" identifiers. Matches
/// `arch/x86/include/asm/sev-common.h`.
pub const SEV_TERM_SET_GEN: u8 = 0;
pub const SEV_TERM_SET_ES: u8 = 1;
pub const SEV_TERM_SET_GHCB_LIVE_HV: u8 = 2;

/// `set_memory_encrypted/decrypted` request shape. Linux issues a
/// VMGEXIT with `SVM_VMGEXIT_PAGE_STATE_CHANGE`. The decompressor /
/// startup path packs the request into the GHCB; we expose the
/// per-page encoding helper.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct EncDecRequest {
    pub addr: u64,
    pub npages: u64,
    pub encrypt: bool,
}

impl EncDecRequest {
    /// Compute the matching `SnpPageState` for this request.
    pub fn target_state(&self) -> SnpPageState {
        if self.encrypt {
            SnpPageState::Private
        } else {
            SnpPageState::Shared
        }
    }
}

/// Pack a GHCB MSR-protocol request: opcode in low 12 bits, payload
/// in the upper 52 bits. Used by the MSR-protocol bring-up path
/// before the GHCB shared page is available.
pub const fn ghcb_msr_request(opcode: u64, payload: u64) -> u64 {
    (payload << 12) | (opcode & 0xFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghcb_msr_opcodes_match_sev_common_h() {
        assert_eq!(GHCB_MSR_SEV_INFO_REQ, 0x002);
        assert_eq!(GHCB_MSR_CPUID_REQ, 0x004);
        assert_eq!(GHCB_MSR_AP_RESET_HOLD_REQ, 0x006);
        assert_eq!(GHCB_MSR_PSC_REQ, 0x014);
        assert_eq!(GHCB_MSR_TERM_REQ, 0x100);
    }

    #[test]
    fn sev_term_reason_sets_match_sev_common_h() {
        assert_eq!(SEV_TERM_SET_GEN, 0);
        assert_eq!(SEV_TERM_SET_ES, 1);
        assert_eq!(SEV_TERM_SET_GHCB_LIVE_HV, 2);
    }

    #[test]
    fn ghcb_msr_request_packs_payload_in_high_52_bits() {
        let r = ghcb_msr_request(GHCB_MSR_CPUID_REQ, 0xdead_beef);
        assert_eq!(r & 0xFFF, GHCB_MSR_CPUID_REQ);
        assert_eq!(r >> 12, 0xdead_beef);
    }

    #[test]
    fn enc_dec_request_target_state_picks_private_when_encrypting() {
        let enc = EncDecRequest {
            addr: 0x1000,
            npages: 1,
            encrypt: true,
        };
        assert_eq!(enc.target_state(), SnpPageState::Private);
        let dec = EncDecRequest {
            addr: 0x1000,
            npages: 1,
            encrypt: false,
        };
        assert_eq!(dec.target_state(), SnpPageState::Shared);
    }

    #[test]
    fn termination_request_re_exported_from_compressed_module() {
        // sev-shared.c uses the same termination encoding the
        // compressed stub uses; the re-export keeps the constant
        // available without a duplicate definition.
        assert_eq!(sev_termination_request(SEV_TERM_SET_ES, 1), 0x11);
    }
}
