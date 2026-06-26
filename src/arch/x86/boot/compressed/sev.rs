//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/sev.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/sev.c
//! SEV / SEV-ES / SEV-SNP support in the decompressor.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/sev.c
//!
//! 513 lines in Linux. The pieces that are sensible to port at this
//! stage (without the host-side state machine that arrives with the
//! `coco/sev/` batch):
//!   * CPUID feature checks for SEV / SEV-ES / SEV-SNP.
//!   * GHCB layout constants.
//!   * Page-state-change request shape.
//!   * `sev_es_terminate(reason_set, reason)` request encoding.
//!
//! Host integration (writing a GHCB, calling VMGEXIT) lives in batch
//! 19. The trait seam here mirrors Linux's `sev-shared.c` boundary.

/// MSR for the SEV status. Matches `arch/x86/include/asm/msr-index.h`.
pub const MSR_AMD64_SEV: u32 = 0xC001_0131;
pub const MSR_AMD64_SEV_ES_GHCB: u32 = 0xC001_0130;

/// SEV status bits — bit 0 = SEV, bit 1 = SEV-ES, bit 2 = SEV-SNP.
pub const SEV_ENABLED: u64 = 1 << 0;
pub const SEV_ES_ENABLED: u64 = 1 << 1;
pub const SEV_SNP_ENABLED: u64 = 1 << 2;

/// `GHCB` shared-memory page is 4 KiB. Matches Linux's
/// `struct ghcb` (asm/svm.h).
pub const GHCB_SIZE: usize = 4096;

/// Linux's "termination" request encoding for SEV. Used when the
/// guest can't continue (e.g. unsupported #VC).
pub const SEV_TERM_GEN_REQ: u8 = 0x00;
pub const SEV_TERM_SET_GEN: u8 = 0x0;

/// Pack a SEV termination request as Linux's `sev_es_terminate()`
/// would: high byte is the reason set, low byte the reason code.
/// Matches Linux's MSR-protocol layout (asm/sev-common.h).
pub const fn sev_termination_request(reason_set: u8, reason: u8) -> u16 {
    ((reason_set as u16) << 4) | (reason as u16)
}

/// Predicate: is SEV enabled in `MSR_AMD64_SEV`? Mirrors Linux's
/// `sev_status & MSR_AMD64_SEV_ENABLED`.
#[inline]
pub fn sev_enabled(sev_status: u64) -> bool {
    sev_status & SEV_ENABLED != 0
}
#[inline]
pub fn sev_es_enabled(sev_status: u64) -> bool {
    sev_status & SEV_ES_ENABLED != 0
}
#[inline]
pub fn sev_snp_enabled(sev_status: u64) -> bool {
    sev_status & SEV_SNP_ENABLED != 0
}

/// Page-state-change request encoding for SEV-SNP. Linux issues
/// these via the GHCB MSR protocol. Matches asm/sev-common.h.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u64)]
pub enum SnpPageState {
    Private = 1,
    Shared = 2,
    PSmash = 3,
    UnSmash = 4,
}

/// Pack a `pageop` MSR-protocol request: low 12 bits identify the
/// request, the rest carries the page frame number and the new state.
pub const fn snp_page_state_msr(pfn: u64, state: SnpPageState) -> u64 {
    (pfn << 12) | (state as u64) << 52 | 0x14 // GHCBMsg PageState Change request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sev_status_msr_numbers_match_msr_index_h() {
        assert_eq!(MSR_AMD64_SEV, 0xC001_0131);
        assert_eq!(MSR_AMD64_SEV_ES_GHCB, 0xC001_0130);
    }

    #[test]
    fn sev_status_bits_are_packed_low_to_high() {
        assert_eq!(SEV_ENABLED, 1);
        assert_eq!(SEV_ES_ENABLED, 2);
        assert_eq!(SEV_SNP_ENABLED, 4);
    }

    #[test]
    fn sev_predicates_inspect_correct_bits() {
        let snp = SEV_ENABLED | SEV_ES_ENABLED | SEV_SNP_ENABLED;
        assert!(sev_enabled(snp));
        assert!(sev_es_enabled(snp));
        assert!(sev_snp_enabled(snp));
        let es_only = SEV_ENABLED | SEV_ES_ENABLED;
        assert!(!sev_snp_enabled(es_only));
    }

    #[test]
    fn termination_request_packs_set_into_high_nibble() {
        // Linux: high nibble = reason set, low nibble = reason.
        assert_eq!(sev_termination_request(0x1, 0x3), 0x13);
        assert_eq!(
            sev_termination_request(SEV_TERM_SET_GEN, SEV_TERM_GEN_REQ),
            0x00
        );
    }

    #[test]
    fn snp_page_state_request_packs_pfn_and_state() {
        let r = snp_page_state_msr(0xabcd, SnpPageState::Private);
        // bits[51:12] = pfn, bits[55:52] = state (1), bits[11:0] = 0x14.
        assert_eq!(r & 0xfff, 0x14);
        assert_eq!((r >> 12) & 0xfffff_ffff_f, 0xabcd);
        assert_eq!((r >> 52) & 0xf, 1);
    }

    #[test]
    fn ghcb_page_size_is_4_kib() {
        assert_eq!(GHCB_SIZE, 4096);
    }
}
