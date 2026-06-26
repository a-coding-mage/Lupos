//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/hyperv.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/hyperv.c
//! Hyper-V enlightenments exposed to KVM guests.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/hyperv.c

// `hyperv.c` lets a KVM guest see Hyper-V virtualization MSRs (HV_X64_*).
// Pertinent surface: SInt vectors, hypercall page, reference TSC. We
// model the enabled-feature bitmap; the runtime VMM owns activations.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HypervFeatureSet {
    pub bits: u64,
}

pub const HV_FEATURE_HYPERCALL: u64 = 1 << 0;
pub const HV_FEATURE_REFERENCE_TSC: u64 = 1 << 1;
pub const HV_FEATURE_SYNIC: u64 = 1 << 2;
pub const HV_FEATURE_VP_INDEX: u64 = 1 << 3;

impl HypervFeatureSet {
    pub const fn enable(&self, bit: u64) -> HypervFeatureSet {
        HypervFeatureSet {
            bits: self.bits | bit,
        }
    }

    pub const fn has(&self, bit: u64) -> bool {
        self.bits & bit != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_is_additive() {
        let f = HypervFeatureSet::default()
            .enable(HV_FEATURE_HYPERCALL)
            .enable(HV_FEATURE_SYNIC);
        assert!(f.has(HV_FEATURE_HYPERCALL));
        assert!(f.has(HV_FEATURE_SYNIC));
        assert!(!f.has(HV_FEATURE_REFERENCE_TSC));
    }
}
