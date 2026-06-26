//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/pmu_intel.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/pmu_intel.c
//! Intel PMU exposure for VMX guests.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/pmu_intel.c

pub const INTEL_PMC_MAX_GENERIC: u8 = 8;
pub const INTEL_PMC_MAX_FIXED: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntelPmuCaps {
    pub version: u8,
    pub generic_counters: u8,
    pub fixed_counters: u8,
    pub pebs: bool,
}

pub const fn sanitize_intel_pmu_caps(caps: IntelPmuCaps) -> IntelPmuCaps {
    IntelPmuCaps {
        version: caps.version,
        generic_counters: if caps.generic_counters > INTEL_PMC_MAX_GENERIC {
            INTEL_PMC_MAX_GENERIC
        } else {
            caps.generic_counters
        },
        fixed_counters: if caps.fixed_counters > INTEL_PMC_MAX_FIXED {
            INTEL_PMC_MAX_FIXED
        } else {
            caps.fixed_counters
        },
        pebs: caps.pebs && caps.version >= 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pmu_caps_are_clamped_to_kvm_exposed_limits() {
        let caps = sanitize_intel_pmu_caps(IntelPmuCaps {
            version: 1,
            generic_counters: 12,
            fixed_counters: 7,
            pebs: true,
        });
        assert_eq!(caps.generic_counters, INTEL_PMC_MAX_GENERIC);
        assert_eq!(caps.fixed_counters, INTEL_PMC_MAX_FIXED);
        assert!(!caps.pebs);
    }
}
