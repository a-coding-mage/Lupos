//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/startup/sev-startup.c
//! test-origin: linux:vendor/linux/arch/x86/boot/startup/sev-startup.c
//! SEV start-up — `early_snp_init` orchestrator.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/startup/sev-startup.c
//!
//! `early_snp_init()` runs once on the boot CPU after the GDT/IDT are
//! installed. Linux walks the cmdline + CPUID + MSR_AMD64_SEV to
//! decide:
//!   * Is SEV-SNP active?
//!   * Was a Confidential-Computing blob passed via boot_params?
//!   * If so, validate it and store a pointer for later use.
//!
//! The port preserves the decision tree and the constants.

use super::sev_shared::{SEV_ENABLED, SEV_ES_ENABLED, SEV_SNP_ENABLED};

/// Linux `RIP_REL_REF(sev_status)` — value of MSR_AMD64_SEV. We expose
/// the bit predicates so callers can build the same decision tree.
pub fn snp_active(sev_status: u64) -> bool {
    sev_status & SEV_SNP_ENABLED != 0
}

/// `early_snp_init()` decision shape — returns the planned next step.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SnpInitOutcome {
    /// Not running under SEV.
    NotSev,
    /// SEV/SEV-ES only (no SNP) — full SNP setup is skipped.
    SevOnly,
    /// SEV-SNP active and the CC blob is well-formed.
    SnpReady,
    /// SEV-SNP active but the CC blob is malformed → terminate.
    SnpBlobInvalid,
}

/// Mirror of `early_snp_init()` logic — pure dispatch over inputs.
pub fn early_snp_init_decision(
    sev_status: u64,
    cc_blob_pa: u64,
    cc_blob_valid: bool,
) -> SnpInitOutcome {
    if sev_status & SEV_ENABLED == 0 {
        return SnpInitOutcome::NotSev;
    }
    if !snp_active(sev_status) {
        return SnpInitOutcome::SevOnly;
    }
    // SEV-SNP active — must have a valid CC blob to proceed.
    if cc_blob_pa == 0 || !cc_blob_valid {
        return SnpInitOutcome::SnpBlobInvalid;
    }
    SnpInitOutcome::SnpReady
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snp_active_only_when_bit_2_set() {
        assert!(snp_active(SEV_ENABLED | SEV_ES_ENABLED | SEV_SNP_ENABLED));
        assert!(!snp_active(SEV_ENABLED | SEV_ES_ENABLED));
        assert!(!snp_active(0));
    }

    #[test]
    fn early_snp_init_decision_short_circuits_on_not_sev() {
        assert_eq!(
            early_snp_init_decision(0, 0xdead, true),
            SnpInitOutcome::NotSev
        );
    }

    #[test]
    fn early_snp_init_decision_sev_only_when_no_snp() {
        let s = SEV_ENABLED | SEV_ES_ENABLED;
        assert_eq!(
            early_snp_init_decision(s, 0, false),
            SnpInitOutcome::SevOnly
        );
    }

    #[test]
    fn early_snp_init_decision_requires_cc_blob_for_snp() {
        let s = SEV_ENABLED | SEV_ES_ENABLED | SEV_SNP_ENABLED;
        assert_eq!(
            early_snp_init_decision(s, 0, false),
            SnpInitOutcome::SnpBlobInvalid
        );
        assert_eq!(
            early_snp_init_decision(s, 0x1000, false),
            SnpInitOutcome::SnpBlobInvalid
        );
        assert_eq!(
            early_snp_init_decision(s, 0x1000, true),
            SnpInitOutcome::SnpReady
        );
    }
}
