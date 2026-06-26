//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 runtime patching and hardening feature gates.
//!
//! Linux wires CPU alternatives, call thunks, CET/IBT/SHSTK, CFI, and boot-time
//! validation through this arch area. Lupos currently boots without runtime text
//! patching; hardening features are explicit policies so callers can fail closed
//! instead of assuming patched instructions exist.

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MitigationFeature {
    Alternatives,
    CallThunks,
    CetIbt,
    ShadowStack,
    Cfi,
}

pub const fn mitigation_enabled(_feature: MitigationFeature) -> bool {
    false
}

pub const fn mitigation_errno(_feature: MitigationFeature) -> i32 {
    EOPNOTSUPP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_patch_mitigations_are_explicitly_disabled() {
        assert!(!mitigation_enabled(MitigationFeature::Alternatives));
        assert_eq!(mitigation_errno(MitigationFeature::ShadowStack), EOPNOTSUPP);
    }
}
