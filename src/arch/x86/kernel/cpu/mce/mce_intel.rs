//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce
//! Intel-specific x86 MCE helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/intel.c

use super::core::{
    MCG_CMCI_P, MCG_EXT_CTL_LMCE_EN, MCG_LMCE_P, MCG_SER_P, MCI_CTL2_CMCI_EN,
    MCI_CTL2_CMCI_THRESHOLD_MASK, MCI_STATUS_MISCV, McaConfig, Mce, PAGE_SHIFT, mci_misc_addr_lsb,
    mci_misc_addr_mode,
};
use crate::arch::x86::kernel::cpu::{CpuFeatures, CpuVendor};

pub const CMCI_THRESHOLD: u64 = 1;
pub const CMCI_STORM_THRESHOLD: u64 = 32749;
pub const FEAT_CTL_LOCKED: u64 = 1 << 0;
pub const FEAT_CTL_LMCE_ENABLED: u64 = 1 << 20;

pub const INTEL_HASWELL: u32 = 0x306c0;
pub const INTEL_HASWELL_L: u32 = 0x40650;
pub const INTEL_HASWELL_G: u32 = 0x40660;
pub const INTEL_BROADWELL: u32 = 0x306d0;
pub const INTEL_SKYLAKE_X: u32 = 0x50650;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CmciSupportInput {
    pub vendor: CpuVendor,
    pub features: CpuFeatures,
    pub cfg: McaConfig,
    pub lapic_max_lvt: u8,
    pub mcg_cap: u64,
}

pub fn cmci_supported(input: CmciSupportInput) -> Option<usize> {
    if input.cfg.cmci_disabled || input.cfg.ignore_ce {
        return None;
    }
    if !matches!(input.vendor, CpuVendor::Intel | CpuVendor::Zhaoxin) {
        return None;
    }
    if !input.features.has_apic() || input.lapic_max_lvt < 6 {
        return None;
    }
    if (input.mcg_cap & MCG_CMCI_P) == 0 {
        return None;
    }
    Some(super::core::mce_bank_count(input.mcg_cap))
}

pub const fn lmce_supported(mcg_cap: u64, feat_ctl: u64, cfg: McaConfig) -> bool {
    if cfg.lmce_disabled {
        return false;
    }
    if (mcg_cap & (MCG_SER_P | MCG_LMCE_P)) != (MCG_SER_P | MCG_LMCE_P) {
        return false;
    }
    (feat_ctl & FEAT_CTL_LOCKED) != 0 && (feat_ctl & FEAT_CTL_LMCE_ENABLED) != 0
}

pub const fn intel_init_lmce_ext_ctl(current: u64, supported: bool) -> u64 {
    if supported {
        current | MCG_EXT_CTL_LMCE_EN
    } else {
        current
    }
}

pub const fn intel_clear_lmce_ext_ctl(current: u64, supported: bool) -> u64 {
    if supported {
        current & !MCG_EXT_CTL_LMCE_EN
    } else {
        current
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CmciThresholdDecision {
    pub value: u64,
    pub bios_zero_threshold: bool,
}

pub const fn cmci_pick_threshold(val: u64, bios_cmci_threshold: bool) -> CmciThresholdDecision {
    if (val & MCI_CTL2_CMCI_THRESHOLD_MASK) == CMCI_STORM_THRESHOLD {
        return CmciThresholdDecision {
            value: val,
            bios_zero_threshold: false,
        };
    }

    if !bios_cmci_threshold {
        CmciThresholdDecision {
            value: (val & !MCI_CTL2_CMCI_THRESHOLD_MASK) | CMCI_THRESHOLD,
            bios_zero_threshold: false,
        }
    } else if (val & MCI_CTL2_CMCI_THRESHOLD_MASK) == 0 {
        CmciThresholdDecision {
            value: val | CMCI_THRESHOLD,
            bios_zero_threshold: true,
        }
    } else {
        CmciThresholdDecision {
            value: val,
            bios_zero_threshold: false,
        }
    }
}

pub const fn mce_intel_handle_storm(default_threshold: u16, on: bool) -> u64 {
    if on {
        CMCI_STORM_THRESHOLD
    } else {
        default_threshold as u64
    }
}

pub fn intel_filter_mce(m: &Mce, cpu_vfm: u32) -> bool {
    matches!(
        cpu_vfm,
        INTEL_HASWELL | INTEL_HASWELL_L | INTEL_BROADWELL | INTEL_HASWELL_G | INTEL_SKYLAKE_X
    ) && m.bank == 0
        && (m.status & 0xa000_0000_ffff_ffff) == 0x8000_0000_000f_0005
}

pub fn intel_mce_usable_address(m: &Mce) -> bool {
    (m.status & MCI_STATUS_MISCV) != 0
        && mci_misc_addr_lsb(m.misc) <= PAGE_SHIFT
        && mci_misc_addr_mode(m.misc) == MCI_MISC_ADDR_PHYS
}

pub const MCI_MISC_ADDR_SEGOFF: u8 = 0;
pub const MCI_MISC_ADDR_LINEAR: u8 = 1;
pub const MCI_MISC_ADDR_PHYS: u8 = 2;
pub const MCI_MISC_ADDR_MEM: u8 = 3;
pub const MCI_MISC_ADDR_GENERIC: u8 = 7;

pub const fn cmci_claim_value(val: u64) -> u64 {
    val | MCI_CTL2_CMCI_EN
}

#[cfg(test)]
mod tests {
    use super::super::core::{MCI_STATUS_ADDRV, MCI_STATUS_VAL};
    use super::*;
    use crate::arch::x86::kernel::cpuid::CpuidResult;

    fn features_with_apic() -> CpuFeatures {
        CpuFeatures::from_cpuid(
            CpuidResult {
                eax: 0,
                ebx: 0,
                ecx: 0,
                edx: 1 << 9,
            },
            CpuidResult {
                eax: 0,
                ebx: 0,
                ecx: 0,
                edx: 0,
            },
            CpuidResult {
                eax: 0,
                ebx: 0,
                ecx: 0,
                edx: 0,
            },
        )
    }

    #[test]
    fn cmci_support_requires_vendor_apic_lvt_and_capability() {
        let input = CmciSupportInput {
            vendor: CpuVendor::Intel,
            features: features_with_apic(),
            cfg: McaConfig::default(),
            lapic_max_lvt: 6,
            mcg_cap: MCG_CMCI_P | 7,
        };
        assert_eq!(cmci_supported(input), Some(7));

        assert_eq!(
            cmci_supported(CmciSupportInput {
                vendor: CpuVendor::Amd,
                ..input
            }),
            None
        );
    }

    #[test]
    fn cmci_threshold_selection_honors_storm_and_bios_rules() {
        assert_eq!(
            cmci_pick_threshold(0, false),
            CmciThresholdDecision {
                value: CMCI_THRESHOLD,
                bios_zero_threshold: false
            }
        );
        assert_eq!(
            cmci_pick_threshold(CMCI_STORM_THRESHOLD, false).value,
            CMCI_STORM_THRESHOLD
        );
        assert!(cmci_pick_threshold(0, true).bios_zero_threshold);
    }

    #[test]
    fn lmce_requires_ser_lmce_and_feat_ctl_lock() {
        let cfg = McaConfig::default();
        assert!(lmce_supported(
            MCG_SER_P | MCG_LMCE_P,
            FEAT_CTL_LOCKED | FEAT_CTL_LMCE_ENABLED,
            cfg
        ));
        assert!(!lmce_supported(MCG_LMCE_P, FEAT_CTL_LOCKED, cfg));
        assert_eq!(intel_init_lmce_ext_ctl(0, true), MCG_EXT_CTL_LMCE_EN);
        assert_eq!(intel_clear_lmce_ext_ctl(MCG_EXT_CTL_LMCE_EN, true), 0);
    }

    #[test]
    fn intel_usable_address_requires_physical_page_granularity() {
        let m = Mce {
            status: MCI_STATUS_VAL | MCI_STATUS_ADDRV | MCI_STATUS_MISCV,
            misc: (MCI_MISC_ADDR_PHYS as u64) << 6,
            ..Mce::default()
        };
        assert!(intel_mce_usable_address(&m));

        let bad = Mce {
            misc: ((MCI_MISC_ADDR_LINEAR as u64) << 6) | 13,
            ..m
        };
        assert!(!intel_mce_usable_address(&bad));
    }

    #[test]
    fn intel_errata_filter_matches_known_bank_zero_signature() {
        let m = Mce {
            bank: 0,
            status: 0x8000_0000_000f_0005,
            ..Mce::default()
        };
        assert!(intel_filter_mce(&m, INTEL_HASWELL));
    }
}
