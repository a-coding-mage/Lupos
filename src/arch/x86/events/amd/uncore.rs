//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/amd/uncore.c
//! test-origin: linux:vendor/linux/arch/x86/events/amd/uncore.c
//! AMD uncore PMU model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/amd/uncore.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmdUncoreUnit {
    DataFabric,
    L3,
    UnifiedMemoryController,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdUncorePmu {
    pub unit: AmdUncoreUnit,
    pub counters: u8,
}

pub const fn uncore_pmu(unit: AmdUncoreUnit, discovered: bool) -> Option<AmdUncorePmu> {
    if !discovered {
        return None;
    }
    let counters = match unit {
        AmdUncoreUnit::DataFabric => 4,
        AmdUncoreUnit::L3 => 6,
        AmdUncoreUnit::UnifiedMemoryController => 4,
    };
    Some(AmdUncorePmu { unit, counters })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncore_discovery_is_explicitly_gated() {
        assert_eq!(uncore_pmu(AmdUncoreUnit::L3, false), None);
        assert_eq!(
            uncore_pmu(AmdUncoreUnit::L3, true),
            Some(AmdUncorePmu {
                unit: AmdUncoreUnit::L3,
                counters: 6,
            })
        );
    }
}
