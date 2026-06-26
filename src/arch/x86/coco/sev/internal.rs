//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/sev/internal.h
//! test-origin: linux:vendor/linux/arch/x86/coco/sev/internal.h
//! Private SEV header helpers shared by the SEV translation units.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/sev/internal.h

use super::core::{GHCB_TERM_PVALIDATE, SEV_TERM_SET_LINUX, SevTermination};
use super::vc_shared::Ghcb;

pub const DR7_RESET_VALUE: u64 = 0x400;
pub const MSR_AMD64_SEV_ES_GHCB: u32 = 0xc001_0130;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SevEsRuntimeData {
    pub ghcb_page: Ghcb,
    pub backup_ghcb: Ghcb,
    pub ghcb_active: bool,
    pub backup_ghcb_active: bool,
    pub dr7: u64,
}

impl SevEsRuntimeData {
    pub fn linux_default() -> Self {
        Self {
            dr7: DR7_RESET_VALUE,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GhcbState {
    pub ghcb_pa: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GhcbMsrWritePlan {
    pub msr: u32,
    pub low: u32,
    pub high: u32,
}

pub const fn sev_es_rd_ghcb_msr_value(raw_msr_value: u64) -> u64 {
    raw_msr_value
}

pub const fn sev_es_wr_ghcb_msr_plan(val: u64) -> GhcbMsrWritePlan {
    GhcbMsrWritePlan {
        msr: MSR_AMD64_SEV_ES_GHCB,
        low: val as u32,
        high: (val >> 32) as u32,
    }
}

pub const fn svsm_get_caa_plan(use_cas: bool, per_cpu_caa: u64, boot_svsm_ca_page: u64) -> u64 {
    if use_cas {
        per_cpu_caa
    } else {
        boot_svsm_ca_page
    }
}

pub const fn svsm_get_caa_pa_plan(
    use_cas: bool,
    per_cpu_caa_pa: u64,
    boot_svsm_caa_pa: u64,
) -> u64 {
    if use_cas {
        per_cpu_caa_pa
    } else {
        boot_svsm_caa_pa
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PvalidateTerminatePlan {
    pub pfn: u64,
    pub action: bool,
    pub page_size: u32,
    pub ret: i32,
    pub svsm_ret: u64,
    pub warn: bool,
    pub termination: SevTermination,
}

pub const fn pval_terminate_plan(
    pfn: u64,
    action: bool,
    page_size: u32,
    ret: i32,
    svsm_ret: u64,
) -> PvalidateTerminatePlan {
    PvalidateTerminatePlan {
        pfn,
        action,
        page_size,
        ret,
        svsm_ret,
        warn: true,
        termination: SevTermination {
            set: SEV_TERM_SET_LINUX,
            reason: GHCB_TERM_PVALIDATE,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_data_layout_carries_linux_ghcb_state_and_dr7_default() {
        let data = SevEsRuntimeData::linux_default();
        assert_eq!(data.dr7, DR7_RESET_VALUE);
        assert!(!data.ghcb_active);
        assert!(!data.backup_ghcb_active);
        assert_eq!(GhcbState::default().ghcb_pa, None);
    }

    #[test]
    fn ghcb_msr_helpers_match_linux_native_msr_split() {
        let val = 0x1122_3344_5566_7788;
        assert_eq!(sev_es_rd_ghcb_msr_value(val), val);
        assert_eq!(
            sev_es_wr_ghcb_msr_plan(val),
            GhcbMsrWritePlan {
                msr: MSR_AMD64_SEV_ES_GHCB,
                low: 0x5566_7788,
                high: 0x1122_3344,
            }
        );
    }

    #[test]
    fn svsm_caa_helpers_select_per_cpu_or_boot_page_like_linux() {
        assert_eq!(svsm_get_caa_plan(true, 0x1000, 0x2000), 0x1000);
        assert_eq!(svsm_get_caa_plan(false, 0x1000, 0x2000), 0x2000);
        assert_eq!(svsm_get_caa_pa_plan(true, 0x3000, 0x4000), 0x3000);
        assert_eq!(svsm_get_caa_pa_plan(false, 0x3000, 0x4000), 0x4000);
    }

    #[test]
    fn pvalidate_terminate_plan_matches_linux_warn_then_terminate() {
        assert_eq!(
            pval_terminate_plan(0x123, true, 1, -22, 0x8000_1006),
            PvalidateTerminatePlan {
                pfn: 0x123,
                action: true,
                page_size: 1,
                ret: -22,
                svsm_ret: 0x8000_1006,
                warn: true,
                termination: SevTermination {
                    set: SEV_TERM_SET_LINUX,
                    reason: GHCB_TERM_PVALIDATE,
                },
            }
        );
    }
}
