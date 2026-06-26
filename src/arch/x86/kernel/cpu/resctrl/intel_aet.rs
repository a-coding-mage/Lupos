//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/resctrl/intel_aet.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/resctrl/intel_aet.c
//! Intel Architectural Event Tracing (AET) extension for resctrl.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/resctrl/intel_aet.c

// AET extends the resctrl monitoring layer with telemetry events for
// memory bandwidth and prefetcher control. The driver advertises a small
// event table; we model the lookup.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AetEvent {
    LlcMisses,
    L3Occupancy,
    LocalMb,
    TotalMb,
}

pub const fn event_id(event: AetEvent) -> u32 {
    match event {
        AetEvent::LlcMisses => 1,
        AetEvent::L3Occupancy => 2,
        AetEvent::LocalMb => 3,
        AetEvent::TotalMb => 4,
    }
}

pub const fn event_name(event: AetEvent) -> &'static str {
    match event {
        AetEvent::LlcMisses => "llc_misses",
        AetEvent::L3Occupancy => "llc_occupancy",
        AetEvent::LocalMb => "mbm_local_bytes",
        AetEvent::TotalMb => "mbm_total_bytes",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_names_match_resctrl_sysfs() {
        assert_eq!(event_name(AetEvent::LlcMisses), "llc_misses");
        assert_eq!(event_name(AetEvent::L3Occupancy), "llc_occupancy");
        assert_eq!(event_id(AetEvent::TotalMb), 4);
    }
}
