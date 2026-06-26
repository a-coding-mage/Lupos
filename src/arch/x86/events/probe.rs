//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/probe.c
//! test-origin: linux:vendor/linux/arch/x86/events/probe.c
//! x86 PMU probe decision model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/probe.c

use super::core::{PmuFeature, PmuVendor, X86PmuCapabilities};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PmuProbeKind {
    None,
    IntelArchitectural,
    Amd,
    Zhaoxin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PmuProbeResult {
    pub kind: PmuProbeKind,
    pub capabilities: X86PmuCapabilities,
}

pub const fn probe_from_vendor(
    vendor: PmuVendor,
    version: u8,
    counters: u8,
    fixed_counters: u8,
) -> PmuProbeResult {
    let mut caps = X86PmuCapabilities {
        vendor,
        version,
        counters,
        counter_bits: 48,
        fixed_counters,
        features: 0,
    }
    .with_feature(PmuFeature::Probe)
    .with_feature(PmuFeature::CoreCounters);
    let kind = match vendor {
        PmuVendor::Intel => {
            caps = caps.with_feature(PmuFeature::FixedCounters);
            PmuProbeKind::IntelArchitectural
        }
        PmuVendor::Amd => PmuProbeKind::Amd,
        PmuVendor::Zhaoxin => PmuProbeKind::Zhaoxin,
        PmuVendor::Generic => PmuProbeKind::None,
    };
    PmuProbeResult {
        kind,
        capabilities: caps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_classifies_known_vendors() {
        assert_eq!(
            probe_from_vendor(PmuVendor::Intel, 5, 8, 4).kind,
            PmuProbeKind::IntelArchitectural
        );
        assert_eq!(
            probe_from_vendor(PmuVendor::Generic, 0, 0, 0).kind,
            PmuProbeKind::None
        );
    }
}
