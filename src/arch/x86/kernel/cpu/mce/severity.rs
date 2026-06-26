//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/severity.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/severity.c
//! x86 MCE severity grading.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/severity.c

use super::core::{
    MCACOD, MCACOD_DATA, MCACOD_INSTR, MCACOD_L3WB, MCACOD_SCRUB, MCACOD_SCRUBMSK,
    MCE_IN_KERNEL_COPYIN, MCE_IN_KERNEL_RECOV, MCG_STATUS_EIPV, MCG_STATUS_MCIP, MCG_STATUS_RIPV,
    MCI_ADDR, MCI_STATUS_AR, MCI_STATUS_DEFERRED, MCI_STATUS_EN, MCI_STATUS_OVER, MCI_STATUS_PCC,
    MCI_STATUS_S, MCI_STATUS_UC, MCI_STATUS_VAL, MCI_UC_AR, MCI_UC_S, MCI_UC_SAR, McaConfig, Mce,
    MceVendorFlags,
};
use crate::arch::x86::kernel::cpu::CpuVendor;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SeverityLevel {
    No,
    Deferred,
    Ucna,
    Keep,
    Some,
    Ao,
    Uc,
    Ar,
    Panic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MceContext {
    Kernel,
    User,
    KernelRecoverable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeverityResult {
    pub level: SeverityLevel,
    pub message: &'static str,
}

pub const fn mc_recoverable(mcgstatus: u64) -> bool {
    (mcgstatus & (MCG_STATUS_RIPV | MCG_STATUS_EIPV)) == (MCG_STATUS_RIPV | MCG_STATUS_EIPV)
}

pub fn error_context(m: &mut Mce, recoverable_fixup: bool, copy_from_user: bool) -> MceContext {
    if (m.cs & 3) == 3 {
        return MceContext::User;
    }
    if !mc_recoverable(m.mcgstatus) {
        return MceContext::Kernel;
    }
    if copy_from_user {
        m.kflags |= MCE_IN_KERNEL_COPYIN | MCE_IN_KERNEL_RECOV;
        return MceContext::KernelRecoverable;
    }
    if recoverable_fixup {
        m.kflags |= MCE_IN_KERNEL_RECOV;
        MceContext::KernelRecoverable
    } else {
        MceContext::Kernel
    }
}

fn amd_severity(m: &Mce, flags: MceVendorFlags, context: MceContext) -> SeverityResult {
    if (m.status & MCI_STATUS_PCC) != 0 {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Processor Context Corrupt",
        };
    }
    if (m.status & MCI_STATUS_DEFERRED) != 0 {
        return SeverityResult {
            level: SeverityLevel::Deferred,
            message: "Deferred error",
        };
    }
    if (m.status & MCI_STATUS_UC) == 0 {
        return SeverityResult {
            level: SeverityLevel::Keep,
            message: "Corrected error",
        };
    }
    if (m.status & MCI_STATUS_OVER) != 0 && !flags.overflow_recov {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Overflowed uncorrected error without MCA Overflow Recovery",
        };
    }
    if !flags.succor {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Uncorrected error without MCA Recovery",
        };
    }
    if context == MceContext::Kernel {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Uncorrected unrecoverable error in kernel context",
        };
    }
    SeverityResult {
        level: SeverityLevel::Ar,
        message: "Action required",
    }
}

fn intel_severity(
    m: &Mce,
    cfg: McaConfig,
    context: MceContext,
    is_exception: bool,
) -> SeverityResult {
    if (m.status & MCI_STATUS_VAL) == 0 {
        return SeverityResult {
            level: SeverityLevel::No,
            message: "Invalid",
        };
    }
    if is_exception && (m.status & MCI_STATUS_EN) == 0 {
        return SeverityResult {
            level: SeverityLevel::No,
            message: "Not enabled",
        };
    }
    if (m.status & MCI_STATUS_PCC) != 0 {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Processor context corrupt",
        };
    }
    if is_exception && (m.mcgstatus & MCG_STATUS_MCIP) == 0 {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "MCIP not set in MCA handler",
        };
    }
    if is_exception && (m.mcgstatus & (MCG_STATUS_RIPV | MCG_STATUS_EIPV)) == 0 {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Neither restart nor error IP",
        };
    }
    if is_exception
        && matches!(context, MceContext::Kernel | MceContext::KernelRecoverable)
        && (m.mcgstatus & MCG_STATUS_RIPV) == 0
    {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "In kernel and no restart IP",
        };
    }
    if !cfg.ser && (m.status & MCI_STATUS_UC) == 0 {
        return SeverityResult {
            level: SeverityLevel::Keep,
            message: "Corrected error",
        };
    }
    if cfg.ser && (m.status & (MCI_UC_AR | MCACOD_SCRUBMSK)) == (MCI_STATUS_UC | MCACOD_SCRUB) {
        return SeverityResult {
            level: SeverityLevel::Ao,
            message: "Action optional: memory scrubbing error",
        };
    }
    if cfg.ser && (m.status & (MCI_UC_AR | MCACOD)) == (MCI_STATUS_UC | MCACOD_L3WB) {
        return SeverityResult {
            level: SeverityLevel::Ao,
            message: "Action optional: last level cache writeback error",
        };
    }
    if cfg.ser && (m.status & MCI_UC_SAR) == MCI_STATUS_UC {
        return SeverityResult {
            level: SeverityLevel::Ucna,
            message: "Uncorrected no action required",
        };
    }
    if cfg.ser && (m.status & (MCI_STATUS_OVER | MCI_UC_SAR)) == (MCI_STATUS_UC | MCI_STATUS_AR) {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Illegal combination (UCNA with AR=1)",
        };
    }
    if cfg.ser && (m.status & MCI_STATUS_S) == 0 {
        return SeverityResult {
            level: SeverityLevel::Keep,
            message: "Non signaled machine check",
        };
    }
    if cfg.ser && (m.status & (MCI_STATUS_OVER | MCI_UC_SAR)) == (MCI_STATUS_OVER | MCI_UC_SAR) {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Action required with lost events",
        };
    }
    if cfg.ser
        && (m.status & (MCI_STATUS_OVER | MCI_UC_SAR | MCI_ADDR | MCACOD))
            == (MCI_UC_SAR | MCI_ADDR | MCACOD_DATA)
    {
        return match context {
            MceContext::KernelRecoverable | MceContext::User => SeverityResult {
                level: SeverityLevel::Ar,
                message: "Action required: data load error",
            },
            MceContext::Kernel => SeverityResult {
                level: SeverityLevel::Panic,
                message: "Data load in unrecoverable area of kernel",
            },
        };
    }
    if cfg.ser
        && (m.status & (MCI_STATUS_OVER | MCI_UC_SAR | MCI_ADDR | MCACOD))
            == (MCI_UC_SAR | MCI_ADDR | MCACOD_INSTR)
    {
        return match context {
            MceContext::User => SeverityResult {
                level: SeverityLevel::Ar,
                message: "Action required: instruction fetch error in a user process",
            },
            _ => SeverityResult {
                level: SeverityLevel::Panic,
                message: "Instruction fetch error in kernel",
            },
        };
    }
    if cfg.ser && (m.status & (MCI_STATUS_OVER | MCI_UC_SAR)) == MCI_UC_SAR {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Action required: unknown MCACOD",
        };
    }
    if cfg.ser && (m.status & (MCI_STATUS_OVER | MCI_UC_SAR)) == MCI_UC_S {
        return SeverityResult {
            level: SeverityLevel::Some,
            message: "Action optional: unknown MCACOD",
        };
    }
    if (m.status & (MCI_STATUS_OVER | MCI_STATUS_UC)) == (MCI_STATUS_OVER | MCI_STATUS_UC) {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Overflowed uncorrected",
        };
    }
    if (m.status & MCI_STATUS_UC) != 0 && context == MceContext::Kernel {
        return SeverityResult {
            level: SeverityLevel::Panic,
            message: "Uncorrected in kernel",
        };
    }
    if (m.status & MCI_STATUS_UC) != 0 {
        return SeverityResult {
            level: SeverityLevel::Uc,
            message: "Uncorrected",
        };
    }
    SeverityResult {
        level: SeverityLevel::Some,
        message: "No match",
    }
}

pub fn mce_severity(
    m: &Mce,
    cfg: McaConfig,
    flags: MceVendorFlags,
    context: MceContext,
    is_exception: bool,
) -> SeverityResult {
    if matches!(m.cpuvendor, CpuVendor::Amd | CpuVendor::Hygon) {
        amd_severity(m, flags, context)
    } else {
        intel_severity(m, cfg, context, is_exception)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcc_is_always_panic() {
        let m = Mce {
            status: MCI_STATUS_VAL | MCI_STATUS_EN | MCI_STATUS_PCC,
            cpuvendor: CpuVendor::Intel,
            ..Mce::default()
        };
        assert_eq!(
            mce_severity(
                &m,
                McaConfig::default(),
                MceVendorFlags::default(),
                MceContext::User,
                true
            )
            .level,
            SeverityLevel::Panic
        );
    }

    #[test]
    fn amd_deferred_and_succor_paths_match_linux_order() {
        let deferred = Mce {
            cpuvendor: CpuVendor::Amd,
            status: MCI_STATUS_VAL | MCI_STATUS_DEFERRED,
            ..Mce::default()
        };
        assert_eq!(
            mce_severity(
                &deferred,
                McaConfig::default(),
                MceVendorFlags::default(),
                MceContext::User,
                true
            )
            .level,
            SeverityLevel::Deferred
        );

        let ar = Mce {
            cpuvendor: CpuVendor::Amd,
            status: MCI_STATUS_VAL | MCI_STATUS_UC,
            ..Mce::default()
        };
        assert_eq!(
            mce_severity(
                &ar,
                McaConfig::default(),
                MceVendorFlags {
                    succor: true,
                    ..MceVendorFlags::default()
                },
                MceContext::User,
                true
            )
            .level,
            SeverityLevel::Ar
        );
    }

    #[test]
    fn intel_ser_data_load_is_recoverable_only_outside_plain_kernel() {
        let cfg = McaConfig {
            ser: true,
            ..McaConfig::default()
        };
        let m = Mce {
            cpuvendor: CpuVendor::Intel,
            status: MCI_STATUS_VAL | MCI_STATUS_EN | MCI_UC_SAR | MCI_ADDR | MCACOD_DATA,
            mcgstatus: MCG_STATUS_MCIP | MCG_STATUS_RIPV | MCG_STATUS_EIPV,
            ..Mce::default()
        };
        assert_eq!(
            mce_severity(
                &m,
                cfg,
                MceVendorFlags::default(),
                MceContext::KernelRecoverable,
                true
            )
            .level,
            SeverityLevel::Ar
        );
        assert_eq!(
            mce_severity(&m, cfg, MceVendorFlags::default(), MceContext::Kernel, true).level,
            SeverityLevel::Panic
        );
    }

    #[test]
    fn error_context_marks_copyin_and_fixup_recovery() {
        let mut m = Mce {
            mcgstatus: MCG_STATUS_RIPV | MCG_STATUS_EIPV,
            ..Mce::default()
        };
        assert_eq!(
            error_context(&mut m, false, true),
            MceContext::KernelRecoverable
        );
        assert_ne!(m.kflags & MCE_IN_KERNEL_COPYIN, 0);
    }
}
