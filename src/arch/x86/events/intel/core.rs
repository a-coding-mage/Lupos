//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/core.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/core.c
//! Intel architectural PMU capability model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/core.c

use crate::arch::x86::events::core::{PmuFeature, PmuVendor, X86PmuCapabilities};

pub const INTEL_ARCH_PMU_VERSION_MIN: u8 = 1;

pub const fn intel_pmu_capabilities(
    family: u8,
    model: u8,
    architectural_pmu: bool,
) -> X86PmuCapabilities {
    let version = if architectural_pmu { 5 } else { 0 };
    let mut caps = X86PmuCapabilities {
        vendor: PmuVendor::Intel,
        version,
        counters: if architectural_pmu { 8 } else { 0 },
        counter_bits: if architectural_pmu { 48 } else { 0 },
        fixed_counters: if architectural_pmu { 4 } else { 0 },
        features: 0,
    };
    if architectural_pmu {
        caps = caps
            .with_feature(PmuFeature::CoreCounters)
            .with_feature(PmuFeature::FixedCounters)
            .with_feature(PmuFeature::DebugStore)
            .with_feature(PmuFeature::BranchStack);
    }
    if family == 6 && model >= 0x2a {
        caps = caps.with_feature(PmuFeature::Precise);
    }
    caps
}

pub const fn intel_event_needs_fixed_counter(event_select: u8, unit_mask: u8) -> bool {
    event_select == 0x3c && unit_mask == 0x00
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architectural_pmu_exposes_fixed_and_precise_features() {
        let caps = intel_pmu_capabilities(6, 0x55, true);
        assert_eq!(caps.counters, 8);
        assert!(caps.has(PmuFeature::FixedCounters));
        assert!(caps.has(PmuFeature::Precise));
        assert!(intel_event_needs_fixed_counter(0x3c, 0));
    }
}
