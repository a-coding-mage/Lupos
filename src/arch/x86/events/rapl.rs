//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/rapl.c
//! test-origin: linux:vendor/linux/arch/x86/events/rapl.c
//! x86 RAPL perf-event model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/rapl.c

use super::core::{PmuFeature, PmuVendor, X86PmuCapabilities};
use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaplDomain {
    Package,
    PowerPlane0,
    PowerPlane1,
    Dram,
    Platform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RaplEvent {
    pub domain: RaplDomain,
    pub energy_unit_shift: u8,
}

pub const fn rapl_capabilities(vendor: PmuVendor, package_domains: bool) -> X86PmuCapabilities {
    let counters = if package_domains { 1 } else { 0 };
    X86PmuCapabilities {
        vendor,
        version: 1,
        counters,
        counter_bits: 32,
        fixed_counters: 0,
        features: 0,
    }
    .with_feature(PmuFeature::Rapl)
}

pub const fn encode_rapl_event(domain: RaplDomain) -> u64 {
    match domain {
        RaplDomain::Package => 0x01,
        RaplDomain::PowerPlane0 => 0x02,
        RaplDomain::PowerPlane1 => 0x04,
        RaplDomain::Dram => 0x08,
        RaplDomain::Platform => 0x10,
    }
}

pub const fn rapl_programming_errno() -> i32 {
    EOPNOTSUPP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapl_domains_use_distinct_event_bits() {
        assert_eq!(encode_rapl_event(RaplDomain::Package), 0x01);
        assert_eq!(encode_rapl_event(RaplDomain::Dram), 0x08);
    }
}
