//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/intel/p6.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/p6.c
//! Intel P6 PMU model.

use crate::arch::x86::events::core::{EVNTSEL_ENABLE, PmuFeature, PmuVendor, X86PmuCapabilities};
use crate::arch::x86::events::utils::EventConstraint;

pub const PERF_COUNT_HW_CPU_CYCLES: usize = 0;
pub const PERF_COUNT_HW_INSTRUCTIONS: usize = 1;
pub const PERF_COUNT_HW_CACHE_REFERENCES: usize = 2;
pub const PERF_COUNT_HW_CACHE_MISSES: usize = 3;
pub const PERF_COUNT_HW_BRANCH_INSTRUCTIONS: usize = 4;
pub const PERF_COUNT_HW_BRANCH_MISSES: usize = 5;
pub const PERF_COUNT_HW_BUS_CYCLES: usize = 6;
pub const PERF_COUNT_HW_STALLED_CYCLES_FRONTEND: usize = 7;
pub const P6_PERFMON_EVENT_MAX: usize = 8;

pub const PERF_COUNT_HW_CACHE_MAX: usize = 6;
pub const PERF_COUNT_HW_CACHE_OP_MAX: usize = 3;
pub const PERF_COUNT_HW_CACHE_RESULT_MAX: usize = 2;

pub const CACHE_L1D: usize = 0;
pub const CACHE_L1I: usize = 1;
pub const CACHE_LL: usize = 2;
pub const CACHE_DTLB: usize = 3;
pub const CACHE_ITLB: usize = 4;
pub const CACHE_BPU: usize = 5;

pub const OP_READ: usize = 0;
pub const OP_WRITE: usize = 1;
pub const OP_PREFETCH: usize = 2;

pub const RESULT_ACCESS: usize = 0;
pub const RESULT_MISS: usize = 1;

pub const MSR_P6_EVNTSEL0: u32 = 0x186;
pub const MSR_P6_PERFCTR0: u32 = 0x0c1;
pub const P6_NOP_EVENT: u64 = 0x0000_002e;
pub const INTEL_PENTIUM_PRO: u32 = 0x061;

pub const P6_PERFMON_EVENT_MAP: [u64; P6_PERFMON_EVENT_MAX] = [
    0x0079, // CPU_CLK_UNHALTED
    0x00c0, // INST_RETIRED
    0x0f2e, // L2_RQSTS:M:E:S:I
    0x012e, // L2_RQSTS:I
    0x00c4, // BR_INST_RETIRED
    0x00c5, // BR_MISS_PRED_RETIRED
    0x0062, // BUS_DRDY_CLOCKS
    0x00a2, // RESOURCE_STALLS
];

pub const P6_HW_CACHE_EVENT_IDS: [[[i64; PERF_COUNT_HW_CACHE_RESULT_MAX];
    PERF_COUNT_HW_CACHE_OP_MAX]; PERF_COUNT_HW_CACHE_MAX] = [
    [[0x0043, 0x0045], [0, 0x0f29], [0, 0]],
    [[0x0080, 0x0f28], [-1, -1], [0, 0]],
    [[0, 0], [0, 0x0025], [0, 0]],
    [[0x0043, 0], [0, 0], [0, 0]],
    [[0x0080, 0x0085], [-1, -1], [-1, -1]],
    [[0x00c4, 0x00c5], [-1, -1], [-1, -1]],
];

pub const P6_EVENT_CONSTRAINTS: [EventConstraint; 6] = [
    intel_event_constraint(0xc1, 0x1),
    intel_event_constraint(0x10, 0x1),
    intel_event_constraint(0x11, 0x2),
    intel_event_constraint(0x12, 0x2),
    intel_event_constraint(0x13, 0x2),
    intel_event_constraint(0x14, 0x1),
];

pub const P6_FORMAT_ATTRS: [&str; 6] = [
    "event:config:0-7",
    "umask:config:8-15",
    "edge:config:18",
    "pc:config:19",
    "inv:config:23",
    "cmask:config:24-31",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P6Pmu {
    pub name: &'static str,
    pub eventsel: u32,
    pub perfctr: u32,
    pub max_events: usize,
    pub apic: bool,
    pub max_period: u64,
    pub version: u8,
    pub cntr_mask64: u64,
    pub cntval_bits: u8,
    pub cntval_mask: u64,
    pub formats: &'static [&'static str],
}

pub const P6_PMU: P6Pmu = P6Pmu {
    name: "p6",
    eventsel: MSR_P6_EVNTSEL0,
    perfctr: MSR_P6_PERFCTR0,
    max_events: P6_PERFMON_EVENT_MAX,
    apic: true,
    max_period: (1u64 << 31) - 1,
    version: 0,
    cntr_mask64: 0x3,
    cntval_bits: 32,
    cntval_mask: (1u64 << 32) - 1,
    formats: &P6_FORMAT_ATTRS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P6InitPlan {
    pub pmu: P6Pmu,
    pub add_rdpmc_quirk: bool,
    pub copy_hw_cache_event_ids: bool,
}

pub const fn intel_event_constraint(event: u64, counters: u64) -> EventConstraint {
    EventConstraint {
        event,
        mask: 0xffff,
        counters,
    }
}

pub const fn p6_pmu_capabilities(family: u8, model: u8) -> Option<X86PmuCapabilities> {
    if family != 6 || model >= 0x0f {
        return None;
    }
    Some(
        X86PmuCapabilities {
            vendor: PmuVendor::Intel,
            version: 0,
            counters: 2,
            counter_bits: 32,
            fixed_counters: 0,
            features: 0,
        }
        .with_feature(PmuFeature::CoreCounters),
    )
}

pub const fn p6_event_requires_counter0(event: u8) -> bool {
    event == 0xc1 || event == 0x10 || event == 0x14
}

pub const fn p6_pmu_event_map(hw_event: usize) -> Option<u64> {
    if hw_event < P6_PERFMON_EVENT_MAX {
        Some(P6_PERFMON_EVENT_MAP[hw_event])
    } else {
        None
    }
}

pub const fn p6_hw_cache_event_id(cache: usize, op: usize, result: usize) -> Option<i64> {
    if cache < PERF_COUNT_HW_CACHE_MAX
        && op < PERF_COUNT_HW_CACHE_OP_MAX
        && result < PERF_COUNT_HW_CACHE_RESULT_MAX
    {
        Some(P6_HW_CACHE_EVENT_IDS[cache][op][result])
    } else {
        None
    }
}

pub const fn p6_pmu_disable_all(eventsel0: u64) -> u64 {
    eventsel0 & !EVNTSEL_ENABLE
}

pub const fn p6_pmu_enable_all(eventsel0: u64) -> u64 {
    eventsel0 | EVNTSEL_ENABLE
}

pub const fn p6_pmu_disable_event_config() -> u64 {
    P6_NOP_EVENT
}

pub const fn p6_pmu_enable_event_config(config: u64) -> u64 {
    config
}

pub const fn p6_pmu_rdpmc_quirk(stepping: u8) -> bool {
    stepping < 9
}

pub const fn p6_pmu_init_plan(x86_vfm: u32) -> P6InitPlan {
    P6InitPlan {
        pmu: P6_PMU,
        add_rdpmc_quirk: x86_vfm == INTEL_PENTIUM_PRO,
        copy_hw_cache_event_ids: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p6_supports_early_family6_only() {
        assert!(p6_pmu_capabilities(6, 0x0d).is_some());
        assert_eq!(p6_pmu_capabilities(6, 0x1a), None);
        assert_eq!(p6_pmu_capabilities(6, 0x0d).unwrap().counter_bits, 32);
    }

    #[test]
    fn p6_tables_match_linux_event_constants() {
        assert_eq!(p6_pmu_event_map(PERF_COUNT_HW_CPU_CYCLES), Some(0x0079));
        assert_eq!(p6_pmu_event_map(PERF_COUNT_HW_INSTRUCTIONS), Some(0x00c0));
        assert_eq!(p6_pmu_event_map(PERF_COUNT_HW_BRANCH_MISSES), Some(0x00c5));
        assert_eq!(
            p6_hw_cache_event_id(CACHE_L1D, OP_READ, RESULT_ACCESS),
            Some(0x0043)
        );
        assert_eq!(
            p6_hw_cache_event_id(CACHE_L1I, OP_WRITE, RESULT_ACCESS),
            Some(-1)
        );
        assert_eq!(
            p6_hw_cache_event_id(CACHE_BPU, OP_READ, RESULT_MISS),
            Some(0x00c5)
        );
    }

    #[test]
    fn p6_constraints_and_formats_match_source() {
        assert_eq!(P6_EVENT_CONSTRAINTS[0], intel_event_constraint(0xc1, 0x1));
        assert_eq!(P6_EVENT_CONSTRAINTS[2], intel_event_constraint(0x11, 0x2));
        assert!(p6_event_requires_counter0(0xc1));
        assert!(!p6_event_requires_counter0(0x11));
        assert_eq!(P6_FORMAT_ATTRS[0], "event:config:0-7");
        assert_eq!(P6_FORMAT_ATTRS[5], "cmask:config:24-31");
    }

    #[test]
    fn p6_pmu_operations_follow_single_global_enable_register() {
        assert_eq!(p6_pmu_disable_all(EVNTSEL_ENABLE | 0x55), 0x55);
        assert_eq!(p6_pmu_enable_all(0x55), EVNTSEL_ENABLE | 0x55);
        assert_eq!(p6_pmu_disable_event_config(), P6_NOP_EVENT);
        assert_eq!(p6_pmu_enable_event_config(0x00c0), 0x00c0);
    }

    #[test]
    fn p6_descriptor_and_init_plan_match_linux_source_shape() {
        assert_eq!(P6_PMU.name, "p6");
        assert_eq!(P6_PMU.eventsel, MSR_P6_EVNTSEL0);
        assert_eq!(P6_PMU.perfctr, MSR_P6_PERFCTR0);
        assert_eq!(P6_PMU.max_period, (1u64 << 31) - 1);
        assert_eq!(P6_PMU.cntr_mask64, 0x3);
        assert_eq!(P6_PMU.cntval_bits, 32);
        assert_eq!(P6_PMU.cntval_mask, (1u64 << 32) - 1);
        assert!(p6_pmu_init_plan(INTEL_PENTIUM_PRO).add_rdpmc_quirk);
        assert!(!p6_pmu_init_plan(0x06f).add_rdpmc_quirk);
        assert!(p6_pmu_init_plan(0x06f).copy_hw_cache_event_ids);
        assert!(p6_pmu_rdpmc_quirk(8));
        assert!(!p6_pmu_rdpmc_quirk(9));
    }

    #[test]
    fn p6_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/intel/p6.c"
        ));
        assert!(source.contains("static const u64 p6_perfmon_event_map[]"));
        assert!(source.contains("[PERF_COUNT_HW_CPU_CYCLES]\t\t= 0x0079"));
        assert!(source.contains("p6_hw_cache_event_ids"));
        assert!(source.contains("P6_NOP_EVENT\t\t\t0x0000002EULL"));
        assert!(source.contains("INTEL_EVENT_CONSTRAINT(0xc1, 0x1)"));
        assert!(source.contains("PMU_FORMAT_ATTR(event,\t\"config:0-7\""));
        assert!(source.contains("val &= ~ARCH_PERFMON_EVENTSEL_ENABLE"));
        assert!(source.contains("val |= ARCH_PERFMON_EVENTSEL_ENABLE"));
        assert!(source.contains(".eventsel\t\t= MSR_P6_EVNTSEL0"));
        assert!(source.contains(".perfctr\t\t= MSR_P6_PERFCTR0"));
        assert!(source.contains(".cntval_bits\t\t= 32"));
        assert!(source.contains("boot_cpu_data.x86_vfm == INTEL_PENTIUM_PRO"));
        assert!(source.contains("memcpy(hw_cache_event_ids, p6_hw_cache_event_ids"));
    }
}
