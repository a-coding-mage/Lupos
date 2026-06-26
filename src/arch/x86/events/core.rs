//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/core.c
//! test-origin: linux:vendor/linux/arch/x86/events/core.c
//! Generic x86 perf-event PMU state.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/core.c

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PmuVendor {
    Generic,
    Intel,
    Amd,
    Zhaoxin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PmuFeature {
    CoreCounters,
    FixedCounters,
    BranchStack,
    DebugStore,
    Precise,
    Uncore,
    Rapl,
    Msr,
    Probe,
    Ibs,
    Iommu,
}

pub const fn feature_bit(feature: PmuFeature) -> u64 {
    1u64 << feature as u8
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86PmuCapabilities {
    pub vendor: PmuVendor,
    pub version: u8,
    pub counters: u8,
    pub counter_bits: u8,
    pub fixed_counters: u8,
    pub features: u64,
}

impl X86PmuCapabilities {
    pub const fn new(vendor: PmuVendor) -> Self {
        Self {
            vendor,
            version: 0,
            counters: 0,
            counter_bits: 0,
            fixed_counters: 0,
            features: 0,
        }
    }

    pub const fn with_feature(mut self, feature: PmuFeature) -> Self {
        self.features |= feature_bit(feature);
        self
    }

    pub const fn has(self, feature: PmuFeature) -> bool {
        self.features & feature_bit(feature) != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PmuEvent {
    pub event_select: u8,
    pub unit_mask: u8,
    pub edge: bool,
    pub interrupt: bool,
    pub any_thread: bool,
    pub enable: bool,
    pub invert: bool,
    pub cmask: u8,
}

pub const EVNTSEL_USR: u64 = 1 << 16;
pub const EVNTSEL_OS: u64 = 1 << 17;
pub const EVNTSEL_EDGE: u64 = 1 << 18;
pub const EVNTSEL_INT: u64 = 1 << 20;
pub const EVNTSEL_ANY: u64 = 1 << 21;
pub const EVNTSEL_ENABLE: u64 = 1 << 22;
pub const EVNTSEL_INV: u64 = 1 << 23;

pub const fn encode_evntsel(event: PmuEvent) -> u64 {
    let mut raw = event.event_select as u64 | ((event.unit_mask as u64) << 8);
    raw |= EVNTSEL_USR | EVNTSEL_OS;
    if event.edge {
        raw |= EVNTSEL_EDGE;
    }
    if event.interrupt {
        raw |= EVNTSEL_INT;
    }
    if event.any_thread {
        raw |= EVNTSEL_ANY;
    }
    if event.enable {
        raw |= EVNTSEL_ENABLE;
    }
    if event.invert {
        raw |= EVNTSEL_INV;
    }
    raw | ((event.cmask as u64) << 24)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PmuRegistration {
    pub name: &'static str,
    pub capabilities: X86PmuCapabilities,
    pub registered: bool,
}

impl PmuRegistration {
    pub const fn disabled(name: &'static str, capabilities: X86PmuCapabilities) -> Self {
        Self {
            name,
            capabilities,
            registered: false,
        }
    }
}

pub const fn validate_counter_index(
    caps: X86PmuCapabilities,
    index: u8,
    fixed: bool,
) -> Result<(), i32> {
    let limit = if fixed {
        caps.fixed_counters
    } else {
        caps.counters
    };
    if index < limit { Ok(()) } else { Err(EINVAL) }
}

pub const fn hardware_pmu_programming_enabled() -> bool {
    false
}

pub const fn hardware_pmu_errno() -> i32 {
    EOPNOTSUPP
}

pub const fn unsupported_registration(
    name: &'static str,
    caps: X86PmuCapabilities,
) -> Result<PmuRegistration, i32> {
    let _ = name;
    let _ = caps;
    Err(EOPNOTSUPP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_validation_uses_fixed_or_gp_limits() {
        let caps = X86PmuCapabilities {
            vendor: PmuVendor::Intel,
            version: 5,
            counters: 4,
            counter_bits: 48,
            fixed_counters: 3,
            features: feature_bit(PmuFeature::FixedCounters),
        };
        assert_eq!(validate_counter_index(caps, 3, false), Ok(()));
        assert_eq!(validate_counter_index(caps, 4, false), Err(EINVAL));
        assert_eq!(validate_counter_index(caps, 2, true), Ok(()));
        assert_eq!(validate_counter_index(caps, 3, true), Err(EINVAL));
    }
}
