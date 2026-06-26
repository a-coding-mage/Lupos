//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/p5.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/p5.c
//! Pentium P5 MCE model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/p5.c

use ::core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86::kernel::cpu::CpuFeatures;

pub const MSR_IA32_P5_MC_ADDR: u32 = 0;
pub const MSR_IA32_P5_MC_TYPE: u32 = 1;

static MCE_P5_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P5MachineCheck {
    pub addr: u32,
    pub typ: u32,
    pub thermal_failure: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum P5InitDecision {
    DisabledByDefault,
    UnsupportedCpu,
    EnableCr4Mce,
}

pub fn enable_p5_mce() {
    MCE_P5_ENABLED.store(true, Ordering::Release);
}

pub fn disable_p5_mce() {
    MCE_P5_ENABLED.store(false, Ordering::Release);
}

pub fn p5_mce_enabled() -> bool {
    MCE_P5_ENABLED.load(Ordering::Acquire)
}

pub const fn pentium_machine_check(addr: u32, typ: u32) -> P5MachineCheck {
    P5MachineCheck {
        addr,
        typ,
        thermal_failure: (typ & (1 << 5)) != 0,
    }
}

pub fn intel_p5_mcheck_init(features: CpuFeatures) -> P5InitDecision {
    if !p5_mce_enabled() {
        return P5InitDecision::DisabledByDefault;
    }
    if !features.has_mce() {
        return P5InitDecision::UnsupportedCpu;
    }
    P5InitDecision::EnableCr4Mce
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::kernel::cpuid::CpuidResult;

    fn features(mce: bool) -> CpuFeatures {
        CpuFeatures::from_cpuid(
            CpuidResult {
                eax: 0,
                ebx: 0,
                ecx: 0,
                edx: if mce { 1 << 7 } else { 0 },
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
    fn p5_init_is_disabled_until_explicitly_enabled() {
        disable_p5_mce();
        assert_eq!(
            intel_p5_mcheck_init(features(true)),
            P5InitDecision::DisabledByDefault
        );
        enable_p5_mce();
        assert_eq!(
            intel_p5_mcheck_init(features(false)),
            P5InitDecision::UnsupportedCpu
        );
        assert_eq!(
            intel_p5_mcheck_init(features(true)),
            P5InitDecision::EnableCr4Mce
        );
        disable_p5_mce();
    }

    #[test]
    fn p5_report_flags_thermal_failure_bit() {
        let report = pentium_machine_check(0x1000, 1 << 5);
        assert!(report.thermal_failure);
        assert_eq!(report.addr, 0x1000);
    }
}
