//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 runtime patching and hardening feature gates.
//!
//! Linux wires CPU alternatives, call thunks, CET/IBT/SHSTK, CFI, and boot-time
//! validation through this arch area. Report the actual selected runtime state;
//! metadata support is not conflated with enabling a hardware mitigation.

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MitigationFeature {
    Alternatives,
    CallThunks,
    CetIbt,
    ShadowStack,
    Cfi,
}

pub fn mitigation_enabled(feature: MitigationFeature) -> bool {
    match feature {
        MitigationFeature::Alternatives => true,
        MitigationFeature::CetIbt => crate::arch::x86::kernel::cet::kernel_ibt_enabled(),
        MitigationFeature::CallThunks | MitigationFeature::ShadowStack | MitigationFeature::Cfi => {
            false
        }
    }
}

pub fn mitigation_errno(feature: MitigationFeature) -> i32 {
    if mitigation_enabled(feature) {
        0
    } else {
        EOPNOTSUPP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_patch_mitigations_report_the_selected_state() {
        assert!(mitigation_enabled(MitigationFeature::Alternatives));
        assert_eq!(mitigation_errno(MitigationFeature::Alternatives), 0);
        assert_eq!(mitigation_errno(MitigationFeature::ShadowStack), EOPNOTSUPP);
    }
}
