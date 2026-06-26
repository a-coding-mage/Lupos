//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce
//! AMD-specific x86 MCE helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/amd.c

use super::core::{
    MCE_CHECK_DFR_REGS, MCI_STATUS_DEFERRED, MCI_STATUS_EN, MCI_STATUS_PADDRV, MCI_STATUS_POISON,
    McaMsr, Mce, MceBank, MceVendorFlags, mca_msr_reg, xec,
};
use crate::arch::x86::kernel::cpu::CpuSignature;

pub const NR_BLOCKS: usize = 5;
pub const THRESHOLD_MAX: u16 = 0x0fff;
pub const INT_TYPE_APIC: u32 = 0x0002_0000;
pub const MASK_VALID_HI: u32 = 0x8000_0000;
pub const MASK_CNTP_HI: u32 = 0x4000_0000;
pub const MASK_LOCKED_HI: u32 = 0x2000_0000;
pub const MASK_LVTOFF_HI: u32 = 0x00f0_0000;
pub const MASK_COUNT_EN_HI: u32 = 0x0008_0000;
pub const MASK_INT_TYPE_HI: u32 = 0x0006_0000;
pub const MASK_OVERFLOW_HI: u32 = 0x0001_0000;
pub const MASK_ERR_COUNT_HI: u32 = 0x0000_0fff;
pub const MASK_BLKPTR_LO: u32 = 0xff00_0000;
pub const MCG_XBLK_ADDR: u32 = 0xc000_0400;
pub const MSR_CU_DEF_ERR: u32 = 0xc000_0410;
pub const MASK_DEF_LVTOFF: u64 = 0x0000_00f0;
pub const SMCA_THR_LVT_OFF: u64 = 0x0000_f000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmcaBankType {
    Cs,
    CsV2,
    Ls,
    LsV2,
    If,
    L2Cache,
    L3Cache,
    DecodeUnit,
    ExecutionUnit,
    FloatingPoint,
    Umc,
    UmcV2,
    Pcie,
    PcieV2,
    Psp,
    PspV2,
    Smu,
    SmuV2,
    Reserved,
    Unknown,
}

impl Default for SmcaBankType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AmdBankInfo {
    pub bank_type: SmcaBankType,
    pub paddrv: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmcaHwid {
    pub bank_type: SmcaBankType,
    pub hwid: u16,
    pub mcatype: u16,
}

pub const SMCA_HWIDS: &[SmcaHwid] = &[
    SmcaHwid {
        bank_type: SmcaBankType::Cs,
        hwid: 0x02e,
        mcatype: 0,
    },
    SmcaHwid {
        bank_type: SmcaBankType::CsV2,
        hwid: 0x02e,
        mcatype: 2,
    },
    SmcaHwid {
        bank_type: SmcaBankType::If,
        hwid: 0x0b0,
        mcatype: 1,
    },
    SmcaHwid {
        bank_type: SmcaBankType::L2Cache,
        hwid: 0x0b0,
        mcatype: 2,
    },
    SmcaHwid {
        bank_type: SmcaBankType::DecodeUnit,
        hwid: 0x0b0,
        mcatype: 3,
    },
    SmcaHwid {
        bank_type: SmcaBankType::ExecutionUnit,
        hwid: 0x0b0,
        mcatype: 5,
    },
    SmcaHwid {
        bank_type: SmcaBankType::FloatingPoint,
        hwid: 0x0b0,
        mcatype: 6,
    },
    SmcaHwid {
        bank_type: SmcaBankType::L3Cache,
        hwid: 0x0b0,
        mcatype: 7,
    },
    SmcaHwid {
        bank_type: SmcaBankType::Ls,
        hwid: 0x0b0,
        mcatype: 0,
    },
    SmcaHwid {
        bank_type: SmcaBankType::LsV2,
        hwid: 0x0b0,
        mcatype: 0x10,
    },
    SmcaHwid {
        bank_type: SmcaBankType::Pcie,
        hwid: 0x046,
        mcatype: 0,
    },
    SmcaHwid {
        bank_type: SmcaBankType::PcieV2,
        hwid: 0x046,
        mcatype: 1,
    },
    SmcaHwid {
        bank_type: SmcaBankType::Psp,
        hwid: 0x0ff,
        mcatype: 0,
    },
    SmcaHwid {
        bank_type: SmcaBankType::PspV2,
        hwid: 0x0ff,
        mcatype: 1,
    },
    SmcaHwid {
        bank_type: SmcaBankType::Reserved,
        hwid: 0,
        mcatype: 0,
    },
    SmcaHwid {
        bank_type: SmcaBankType::Smu,
        hwid: 0x001,
        mcatype: 0,
    },
    SmcaHwid {
        bank_type: SmcaBankType::SmuV2,
        hwid: 0x001,
        mcatype: 1,
    },
    SmcaHwid {
        bank_type: SmcaBankType::Umc,
        hwid: 0x096,
        mcatype: 0,
    },
    SmcaHwid {
        bank_type: SmcaBankType::UmcV2,
        hwid: 0x096,
        mcatype: 1,
    },
];

pub const fn hwid_mcatype(hwid: u16, mcatype: u16) -> u32 {
    ((hwid as u32) << 16) | mcatype as u32
}

pub fn smca_get_bank_type(hwid: u16, mcatype: u16) -> SmcaBankType {
    let mut i = 0;
    while i < SMCA_HWIDS.len() {
        let entry = SMCA_HWIDS[i];
        if entry.hwid == hwid && entry.mcatype == mcatype {
            return entry.bank_type;
        }
        i += 1;
    }
    SmcaBankType::Unknown
}

pub const fn smca_get_name(kind: SmcaBankType) -> Option<&'static str> {
    match kind {
        SmcaBankType::Cs | SmcaBankType::CsV2 => Some("coherent_station"),
        SmcaBankType::DecodeUnit => Some("decode_unit"),
        SmcaBankType::ExecutionUnit => Some("execution_unit"),
        SmcaBankType::FloatingPoint => Some("floating_point"),
        SmcaBankType::If => Some("insn_fetch"),
        SmcaBankType::L2Cache => Some("l2_cache"),
        SmcaBankType::L3Cache => Some("l3_cache"),
        SmcaBankType::Ls | SmcaBankType::LsV2 => Some("load_store"),
        SmcaBankType::Pcie | SmcaBankType::PcieV2 => Some("pcie"),
        SmcaBankType::Psp | SmcaBankType::PspV2 => Some("psp"),
        SmcaBankType::Reserved => Some("reserved"),
        SmcaBankType::Smu | SmcaBankType::SmuV2 => Some("smu"),
        SmcaBankType::Umc => Some("umc"),
        SmcaBankType::UmcV2 => Some("umc_v2"),
        SmcaBankType::Unknown => None,
    }
}

pub fn amd_filter_mce(m: &Mce, sig: CpuSignature, bank_type: SmcaBankType) -> bool {
    if sig.family == 0x19
        && sig.model == 0x50
        && sig.stepping == 0
        && (m.status & MCI_STATUS_EN) == 0
    {
        return true;
    }

    if sig.family == 0x17
        && (0x10..=0x2f).contains(&sig.model)
        && bank_type == SmcaBankType::If
        && xec(m.status, 0x3f) == 10
    {
        return true;
    }

    sig.family < 0x17 && m.bank == 4 && xec(m.status, 0x1f) == 0x05
}

pub fn amd_mce_is_memory_error(m: &Mce, flags: MceVendorFlags, bank_type: SmcaBankType) -> bool {
    if flags.smca {
        xec(m.status, 0x3f) == 0 && matches!(bank_type, SmcaBankType::Umc | SmcaBankType::UmcV2)
    } else {
        m.bank == 4 && xec(m.status, 0x1f) == 8
    }
}

pub fn amd_mce_usable_address(m: &Mce, flags: MceVendorFlags, bank: AmdBankInfo) -> bool {
    if !flags.smca {
        if amd_mce_is_memory_error(m, flags, bank.bank_type) {
            return true;
        }
        if m.bank == 4 {
            return false;
        }
    }

    if bank.paddrv {
        return (m.status & MCI_STATUS_PADDRV) != 0;
    }

    (m.status & MCI_STATUS_POISON) != 0
}

pub fn smca_extract_err_addr(m: &mut Mce, flags: MceVendorFlags, bank: MceBank) {
    if !flags.smca {
        return;
    }

    let lsb = if bank.lsb_in_status {
        ((m.status >> 24) & 0x3f) as u8
    } else {
        ((m.addr >> 56) & 0x3f) as u8
    };
    let high = if bank.lsb_in_status { 56 } else { 55 };
    let mask = if lsb as u32 > high {
        0
    } else {
        ((1u64 << (high + 1)) - 1) & !((1u64 << lsb) - 1)
    };
    m.addr &= mask;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdClearPlan {
    pub reset_threshold_bank: u8,
    pub clear_deferred_status_msr: Option<u32>,
    pub clear_status_msr: Option<u32>,
}

pub fn amd_clear_bank_plan(m: &Mce, flags: MceVendorFlags) -> Result<AmdClearPlan, i32> {
    let clear_deferred_status_msr = if flags.smca && (m.status & MCI_STATUS_DEFERRED) != 0 {
        Some(super::core::MSR_AMD64_SMCA_MC0_DESTAT + 0x10 * m.bank as u32)
    } else {
        None
    };
    let clear_status_msr = if flags.smca && (m.kflags & MCE_CHECK_DFR_REGS) != 0 {
        None
    } else {
        Some(mca_msr_reg(m.bank as usize, McaMsr::Status, flags.smca)?)
    };
    Ok(AmdClearPlan {
        reset_threshold_bank: m.bank,
        clear_deferred_status_msr,
        clear_status_msr,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmdStormAction {
    EnableThresholdInterrupt { bank: usize },
    DisableThresholdInterrupt { bank: usize },
}

pub const fn mce_amd_handle_storm(bank: usize, on: bool) -> AmdStormAction {
    if on {
        AmdStormAction::EnableThresholdInterrupt { bank }
    } else {
        AmdStormAction::DisableThresholdInterrupt { bank }
    }
}

#[cfg(test)]
mod tests {
    use super::super::core::{MCI_STATUS_ADDRV, MCI_STATUS_VAL};
    use super::*;
    use crate::arch::x86::kernel::cpu::CpuVendor;

    #[test]
    fn smca_hwid_table_decodes_names() {
        assert_eq!(smca_get_bank_type(0x096, 0), SmcaBankType::Umc);
        assert_eq!(smca_get_bank_type(0x096, 1), SmcaBankType::UmcV2);
        assert_eq!(smca_get_name(SmcaBankType::Umc), Some("umc"));
        assert_eq!(hwid_mcatype(0x96, 1), 0x0096_0001);
    }

    #[test]
    fn amd_memory_error_detection_matches_legacy_and_smca_rules() {
        let legacy = Mce {
            cpuvendor: CpuVendor::Amd,
            bank: 4,
            status: 8 << 16,
            ..Mce::default()
        };
        assert!(amd_mce_is_memory_error(
            &legacy,
            MceVendorFlags::default(),
            SmcaBankType::Unknown
        ));

        let smca = Mce {
            cpuvendor: CpuVendor::Amd,
            bank: 2,
            status: MCI_STATUS_VAL,
            ..Mce::default()
        };
        assert!(amd_mce_is_memory_error(
            &smca,
            MceVendorFlags {
                smca: true,
                ..MceVendorFlags::default()
            },
            SmcaBankType::Umc
        ));
    }

    #[test]
    fn amd_usable_address_prefers_northbridge_paddrv_and_poison_rules() {
        let nb = Mce {
            cpuvendor: CpuVendor::Amd,
            bank: 4,
            status: MCI_STATUS_ADDRV | (8 << 16),
            ..Mce::default()
        };
        assert!(amd_mce_usable_address(
            &nb,
            MceVendorFlags::default(),
            AmdBankInfo::default()
        ));

        let paddrv = Mce {
            cpuvendor: CpuVendor::Amd,
            bank: 1,
            status: MCI_STATUS_ADDRV | MCI_STATUS_PADDRV,
            ..Mce::default()
        };
        assert!(amd_mce_usable_address(
            &paddrv,
            MceVendorFlags {
                smca: true,
                ..MceVendorFlags::default()
            },
            AmdBankInfo {
                paddrv: true,
                bank_type: SmcaBankType::Ls,
            }
        ));
    }

    #[test]
    fn amd_errata_filter_rejects_known_bogus_records() {
        let m = Mce {
            status: 10 << 16,
            ..Mce::default()
        };
        let sig = CpuSignature {
            family: 0x17,
            model: 0x20,
            stepping: 0,
            processor_type: 0,
        };
        assert!(amd_filter_mce(&m, sig, SmcaBankType::If));
    }

    #[test]
    fn smca_address_extraction_uses_lsb_source() {
        let mut m = Mce {
            addr: (4u64 << 56) | 0xffff,
            status: MCI_STATUS_ADDRV,
            ..Mce::default()
        };
        smca_extract_err_addr(
            &mut m,
            MceVendorFlags {
                smca: true,
                ..MceVendorFlags::default()
            },
            MceBank::default(),
        );
        assert_eq!(m.addr, 0xfff0);
    }

    #[test]
    fn clear_plan_preserves_deferred_only_status_rule() {
        let m = Mce {
            bank: 3,
            status: MCI_STATUS_DEFERRED,
            kflags: MCE_CHECK_DFR_REGS,
            ..Mce::default()
        };
        let plan = amd_clear_bank_plan(
            &m,
            MceVendorFlags {
                smca: true,
                ..MceVendorFlags::default()
            },
        )
        .unwrap();
        assert_eq!(
            plan.clear_deferred_status_msr,
            Some(super::super::core::MSR_AMD64_SMCA_MC0_DESTAT + 0x30)
        );
        assert_eq!(plan.clear_status_msr, None);
    }
}
