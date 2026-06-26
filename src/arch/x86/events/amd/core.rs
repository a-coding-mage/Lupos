//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/amd/core.c
//! test-origin: linux:vendor/linux/arch/x86/events/amd/core.c
//! AMD core PMU capability model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/amd/core.c

use crate::arch::x86::events::core::{PmuFeature, PmuVendor, X86PmuCapabilities};

pub const AMD64_NUM_COUNTERS_CORE: u8 = 6;
pub const AMD64_NUM_COUNTERS_FAMILY17H: u8 = 6;
pub const AMD64_NUM_COUNTERS_FAMILY19H: u8 = 6;

pub const fn amd_pmu_capabilities(family: u8, has_perfctr_core: bool) -> X86PmuCapabilities {
    let counters = if has_perfctr_core {
        AMD64_NUM_COUNTERS_CORE
    } else {
        4
    };
    let mut caps = X86PmuCapabilities {
        vendor: PmuVendor::Amd,
        version: if family >= 0x17 { 2 } else { 1 },
        counters,
        counter_bits: 48,
        fixed_counters: 0,
        features: 0,
    }
    .with_feature(PmuFeature::CoreCounters);
    if family >= 0x17 {
        caps = caps.with_feature(PmuFeature::Ibs);
    }
    if family >= 0x19 {
        caps = caps.with_feature(PmuFeature::BranchStack);
    }
    caps
}

pub const fn amd_event_is_guest_only(config: u64) -> bool {
    config & (1u64 << 40) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family19h_exposes_ibs_and_branch_stack_model() {
        let caps = amd_pmu_capabilities(0x19, true);
        assert_eq!(caps.counters, 6);
        assert!(caps.has(PmuFeature::Ibs));
        assert!(caps.has(PmuFeature::BranchStack));
    }
}
