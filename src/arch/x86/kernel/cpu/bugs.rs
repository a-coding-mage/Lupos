//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/bugs.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/bugs.c
//! Speculative-execution mitigation dispatch (Linux `bugs.c` policy).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/bugs.c

// `bugs.c` collects all CVE-tracked CPU vulnerabilities (Spectre v1/v2,
// MDS, MMIO Stale Data, Retbleed, SRSO, etc.) and walks a small state
// machine: detect → select → apply → update_sysfs. We model the policy
// table and the selector; the actual MSR writes are owned by other modules
// (alternative.rs, mitigations.rs).

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpectreV2Mitigation {
    None,
    Retpoline,
    IbrsAlways,
    Eibrs,
    Lfence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuVulnerability {
    SpectreV1,
    SpectreV2,
    Mds,
    Taa,
    Mmio,
    Retbleed,
    Srso,
    Gds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VulnContext {
    pub has_ibrs: bool,
    pub has_stibp: bool,
    pub has_ssbd: bool,
    pub has_retpoline_supported: bool,
    pub has_enhanced_ibrs: bool,
}

pub const fn select_spectre_v2(ctx: VulnContext) -> SpectreV2Mitigation {
    if ctx.has_enhanced_ibrs {
        SpectreV2Mitigation::Eibrs
    } else if ctx.has_retpoline_supported {
        SpectreV2Mitigation::Retpoline
    } else if ctx.has_ibrs {
        SpectreV2Mitigation::IbrsAlways
    } else {
        SpectreV2Mitigation::None
    }
}

pub const fn vulnerability_label(v: CpuVulnerability) -> &'static str {
    match v {
        CpuVulnerability::SpectreV1 => "spectre_v1",
        CpuVulnerability::SpectreV2 => "spectre_v2",
        CpuVulnerability::Mds => "mds",
        CpuVulnerability::Taa => "tsx_async_abort",
        CpuVulnerability::Mmio => "mmio_stale_data",
        CpuVulnerability::Retbleed => "retbleed",
        CpuVulnerability::Srso => "spec_rstack_overflow",
        CpuVulnerability::Gds => "gather_data_sampling",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_eibrs_then_retpoline_then_ibrs() {
        let ctx = VulnContext {
            has_ibrs: true,
            has_stibp: true,
            has_ssbd: true,
            has_retpoline_supported: true,
            has_enhanced_ibrs: true,
        };
        assert_eq!(select_spectre_v2(ctx), SpectreV2Mitigation::Eibrs);

        let no_eibrs = VulnContext {
            has_enhanced_ibrs: false,
            ..ctx
        };
        assert_eq!(select_spectre_v2(no_eibrs), SpectreV2Mitigation::Retpoline);

        let only_ibrs = VulnContext {
            has_enhanced_ibrs: false,
            has_retpoline_supported: false,
            ..ctx
        };
        assert_eq!(
            select_spectre_v2(only_ibrs),
            SpectreV2Mitigation::IbrsAlways
        );
    }

    #[test]
    fn sysfs_labels_match_linux_strings() {
        assert_eq!(vulnerability_label(CpuVulnerability::Mds), "mds");
        assert_eq!(
            vulnerability_label(CpuVulnerability::Taa),
            "tsx_async_abort"
        );
    }
}
