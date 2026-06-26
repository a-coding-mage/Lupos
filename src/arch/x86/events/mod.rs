//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events
//! test-origin: linux:vendor/linux/arch/x86/events
//! x86 PMU and perf-event hardware models.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/events/amd/brs.c
//! - vendor/linux/arch/x86/events/amd/core.c
//! - vendor/linux/arch/x86/events/amd/ibs.c
//! - vendor/linux/arch/x86/events/amd/iommu.c
//! - vendor/linux/arch/x86/events/amd/lbr.c
//! - vendor/linux/arch/x86/events/amd/power.c
//! - vendor/linux/arch/x86/events/amd/uncore.c
//! - vendor/linux/arch/x86/events/core.c
//! - vendor/linux/arch/x86/events/intel/bts.c
//! - vendor/linux/arch/x86/events/intel/core.c
//! - vendor/linux/arch/x86/events/intel/cstate.c
//! - vendor/linux/arch/x86/events/intel/ds.c
//! - vendor/linux/arch/x86/events/intel/knc.c
//! - vendor/linux/arch/x86/events/intel/lbr.c
//! - vendor/linux/arch/x86/events/intel/p4.c
//! - vendor/linux/arch/x86/events/intel/p6.c
//! - vendor/linux/arch/x86/events/intel/pt.c
//! - vendor/linux/arch/x86/events/intel/uncore.c
//! - vendor/linux/arch/x86/events/intel/uncore_discovery.c
//! - vendor/linux/arch/x86/events/intel/uncore_nhmex.c
//! - vendor/linux/arch/x86/events/intel/uncore_snb.c
//! - vendor/linux/arch/x86/events/intel/uncore_snbep.c
//! - vendor/linux/arch/x86/events/msr.c
//! - vendor/linux/arch/x86/events/probe.c
//! - vendor/linux/arch/x86/events/rapl.c
//! - vendor/linux/arch/x86/events/utils.c
//! - vendor/linux/arch/x86/events/zhaoxin/core.c

pub mod amd;
pub mod core;
pub mod intel;
pub mod msr;
pub mod probe;
pub mod rapl;
pub mod utils;
pub mod zhaoxin;

pub use self::core::{
    PmuEvent, PmuFeature, PmuRegistration, PmuVendor, X86PmuCapabilities, encode_evntsel,
    hardware_pmu_errno, hardware_pmu_programming_enabled, validate_counter_index,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::EOPNOTSUPP;

    #[test]
    fn batch11_batch12_models_stay_fail_closed_for_hardware() {
        assert!(!hardware_pmu_programming_enabled());
        assert_eq!(hardware_pmu_errno(), EOPNOTSUPP);
        assert!(!amd::ibs::ibs_available(false, true));
        assert!(!intel::pt::pt_available(false, true));
    }

    #[test]
    fn pmu_event_encoding_and_counter_masks_are_stable() {
        let event = PmuEvent {
            event_select: 0x3c,
            unit_mask: 0x01,
            edge: false,
            interrupt: true,
            any_thread: false,
            enable: true,
            invert: false,
            cmask: 0,
        };
        assert_eq!(encode_evntsel(event), 0x0053_013c);
        assert_eq!(utils::counter_mask(4), 0x0f);
        assert_eq!(utils::first_allowed_counter(0b1010), Some(1));
    }

    #[test]
    fn vendor_models_produce_expected_capabilities() {
        let amd = amd::core::amd_pmu_capabilities(0x19, true);
        assert!(amd.has(PmuFeature::BranchStack));
        let intel = intel::core::intel_pmu_capabilities(6, 0x8f, true);
        assert!(intel.has(PmuFeature::Precise));
        let zx = zhaoxin::core::zhaoxin_pmu_capabilities(true);
        assert_eq!(zx.vendor, PmuVendor::Zhaoxin);
    }
}
