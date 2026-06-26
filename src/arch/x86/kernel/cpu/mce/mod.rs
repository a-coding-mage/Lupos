//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce
//! x86 Machine Check Architecture models.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/amd.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/apei.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/core.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/dev-mcelog.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/genpool.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/inject.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/intel.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/p5.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/severity.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/threshold.c
//! - vendor/linux/arch/x86/kernel/cpu/mce/winchip.c

#[path = "mce_amd.rs"]
pub mod amd;
pub mod apei;
#[path = "mce_core.rs"]
pub mod core;
pub mod dev_mcelog;
pub mod genpool;
pub mod inject;
#[path = "mce_intel.rs"]
pub mod intel;
pub mod p5;
pub mod severity;
pub mod threshold;
pub mod winchip;

pub use self::core::{
    MAX_NR_BANKS, MCA_ADDR, MCA_CTL, MCA_MISC, MCA_STATUS, MCACOD, MCACOD_DATA, MCACOD_INSTR,
    MCACOD_L3WB, MCACOD_SCRUB, MCACOD_SCRUBMSK, MCE_CHECK_DFR_REGS, MCE_HANDLED_CEC,
    MCE_HANDLED_EDAC, MCE_HANDLED_EXTLOG, MCE_HANDLED_MCELOG, MCE_HANDLED_NFIT, MCE_HANDLED_UC,
    MCE_IN_KERNEL_COPYIN, MCE_IN_KERNEL_RECOV, MCG_BANKCNT_MASK, MCG_CMCI_P, MCG_CTL_P, MCG_ELOG_P,
    MCG_EXT_CNT_MASK, MCG_EXT_CNT_SHIFT, MCG_EXT_P, MCG_LMCE_P, MCG_SER_P, MCG_STATUS_EIPV,
    MCG_STATUS_LMCES, MCG_STATUS_MCIP, MCG_STATUS_RIPV, MCG_STATUS_SEAM_NR, MCI_ADDR,
    MCI_STATUS_ADDRV, MCI_STATUS_AR, MCI_STATUS_CEC_MASK, MCI_STATUS_CEC_SHIFT,
    MCI_STATUS_DEFERRED, MCI_STATUS_EN, MCI_STATUS_MISCV, MCI_STATUS_OVER, MCI_STATUS_PADDRV,
    MCI_STATUS_PCC, MCI_STATUS_POISON, MCI_STATUS_S, MCI_STATUS_SCRUB, MCI_STATUS_SYNDV,
    MCI_STATUS_TCC, MCI_STATUS_UC, MCI_STATUS_VAL, MCI_UC_AR, MCI_UC_S, MCI_UC_SAR, McaConfig,
    McaMsr, Mce, MceBank, MceBankSet, MceEventSink, MceHwErr, MceRecordSource, MceVendorFlags,
    McpFlags, MsrAccess, StaticRecordSource, UnsupportedMsr, machine_check_poll, mca_msr_reg,
    mce_available, mce_is_correctable, mce_is_memory_error, mce_log, mce_prep_record,
    mce_usable_address,
};
pub use self::severity::{MceContext, SeverityLevel, SeverityResult, mce_severity};
