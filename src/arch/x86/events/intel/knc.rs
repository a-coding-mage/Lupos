//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/intel/knc.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/knc.c
//! Intel Knights Corner PMU model.

use crate::arch::x86::events::core::{
    EVNTSEL_ENABLE, EVNTSEL_INT, PmuFeature, PmuVendor, X86PmuCapabilities,
};
use crate::arch::x86::events::utils::EventConstraint;

pub const KNC_MODEL: u8 = 0x57;
pub const PERF_COUNT_HW_CPU_CYCLES: usize = 0;
pub const PERF_COUNT_HW_INSTRUCTIONS: usize = 1;
pub const PERF_COUNT_HW_CACHE_REFERENCES: usize = 2;
pub const PERF_COUNT_HW_CACHE_MISSES: usize = 3;
pub const PERF_COUNT_HW_BRANCH_INSTRUCTIONS: usize = 4;
pub const PERF_COUNT_HW_BRANCH_MISSES: usize = 5;
pub const KNC_PERFMON_EVENT_MAX: usize = 6;

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

pub const MSR_KNC_PERFCTR0: u32 = 0x0000_0020;
pub const MSR_KNC_EVNTSEL0: u32 = 0x0000_0028;
pub const MSR_KNC_IA32_PERF_GLOBAL_STATUS: u32 = 0x0000_002d;
pub const MSR_KNC_IA32_PERF_GLOBAL_OVF_CONTROL: u32 = 0x0000_002e;
pub const MSR_KNC_IA32_PERF_GLOBAL_CTRL: u32 = 0x0000_002f;
pub const KNC_ENABLE_COUNTER0: u64 = 0x0000_0001;
pub const KNC_ENABLE_COUNTER1: u64 = 0x0000_0002;
pub const KNC_ENABLE_COUNTERS: u64 = KNC_ENABLE_COUNTER0 | KNC_ENABLE_COUNTER1;

pub const KNC_PERFMON_EVENT_MAP: [u64; KNC_PERFMON_EVENT_MAX] =
    [0x002a, 0x0016, 0x0028, 0x0029, 0x0012, 0x002b];

pub const KNC_HW_CACHE_EVENT_IDS: [[[i64; PERF_COUNT_HW_CACHE_RESULT_MAX];
    PERF_COUNT_HW_CACHE_OP_MAX]; PERF_COUNT_HW_CACHE_MAX] = [
    [
        [EVNTSEL_INT as i64, 0x0003],
        [0x0001, 0x0004],
        [0x0011, 0x001c],
    ],
    [[0x000c, 0x000e], [-1, -1], [0, 0]],
    [[0, 0x10cb], [0x10cc, 0], [0x10fc, 0x10fe]],
    [[EVNTSEL_INT as i64, 0x0002], [0x0001, 0x0002], [0, 0]],
    [[0x000c, 0x000d], [-1, -1], [-1, -1]],
    [[0x0012, 0x002b], [-1, -1], [-1, -1]],
];

pub const KNC_EVENT_CONSTRAINTS: [EventConstraint; 21] = [
    intel_event_constraint(0xc3, 0x1),
    intel_event_constraint(0xc4, 0x1),
    intel_event_constraint(0xc8, 0x1),
    intel_event_constraint(0xc9, 0x1),
    intel_event_constraint(0xca, 0x1),
    intel_event_constraint(0xcb, 0x1),
    intel_event_constraint(0xcc, 0x1),
    intel_event_constraint(0xce, 0x1),
    intel_event_constraint(0xcf, 0x1),
    intel_event_constraint(0xd7, 0x1),
    intel_event_constraint(0xe3, 0x1),
    intel_event_constraint(0xe6, 0x1),
    intel_event_constraint(0xe7, 0x1),
    intel_event_constraint(0xf1, 0x1),
    intel_event_constraint(0xf2, 0x1),
    intel_event_constraint(0xf6, 0x1),
    intel_event_constraint(0xf7, 0x1),
    intel_event_constraint(0xfc, 0x1),
    intel_event_constraint(0xfd, 0x1),
    intel_event_constraint(0xfe, 0x1),
    intel_event_constraint(0xff, 0x1),
];

pub const KNC_FORMAT_ATTRS: [&str; 5] = [
    "event:config:0-7",
    "umask:config:8-15",
    "edge:config:18",
    "inv:config:23",
    "cmask:config:24-31",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KncPmu {
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

pub const KNC_PMU: KncPmu = KncPmu {
    name: "knc",
    eventsel: MSR_KNC_EVNTSEL0,
    perfctr: MSR_KNC_PERFCTR0,
    max_events: KNC_PERFMON_EVENT_MAX,
    apic: true,
    max_period: (1u64 << 39) - 1,
    version: 0,
    cntr_mask64: 0x3,
    cntval_bits: 40,
    cntval_mask: (1u64 << 40) - 1,
    formats: &KNC_FORMAT_ATTRS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KncIrqResult {
    pub handled: u32,
    pub ack_count: u32,
    pub loop_stuck: bool,
    pub reenabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KncInitPlan {
    pub pmu: KncPmu,
    pub copy_hw_cache_event_ids: bool,
}

pub const fn intel_event_constraint(event: u64, counters: u64) -> EventConstraint {
    EventConstraint {
        event,
        mask: 0xffff,
        counters,
    }
}

pub const fn knc_pmu_capabilities(model: u8) -> Option<X86PmuCapabilities> {
    if model != KNC_MODEL {
        return None;
    }
    Some(
        X86PmuCapabilities {
            vendor: PmuVendor::Intel,
            version: 0,
            counters: 2,
            counter_bits: 40,
            fixed_counters: 0,
            features: 0,
        }
        .with_feature(PmuFeature::CoreCounters),
    )
}

pub const fn knc_pmu_event_map(hw_event: usize) -> Option<u64> {
    if hw_event < KNC_PERFMON_EVENT_MAX {
        Some(KNC_PERFMON_EVENT_MAP[hw_event])
    } else {
        None
    }
}

pub const fn knc_hw_cache_event_id(cache: usize, op: usize, result: usize) -> Option<i64> {
    if cache < PERF_COUNT_HW_CACHE_MAX
        && op < PERF_COUNT_HW_CACHE_OP_MAX
        && result < PERF_COUNT_HW_CACHE_RESULT_MAX
    {
        Some(KNC_HW_CACHE_EVENT_IDS[cache][op][result])
    } else {
        None
    }
}

pub const fn knc_pmu_disable_all(global_ctrl: u64) -> u64 {
    global_ctrl & !KNC_ENABLE_COUNTERS
}

pub const fn knc_pmu_enable_all(global_ctrl: u64) -> u64 {
    global_ctrl | KNC_ENABLE_COUNTERS
}

pub const fn knc_pmu_disable_event_config(config: u64) -> u64 {
    config & !EVNTSEL_ENABLE
}

pub const fn knc_pmu_enable_event_config(config: u64) -> u64 {
    config | EVNTSEL_ENABLE
}

pub const fn knc_pmu_event_msr(config_base: u32, idx: u8) -> u32 {
    config_base + idx as u32
}

pub const fn knc_pmu_handle_irq_plan(
    statuses: &[u64],
    active_mask: u64,
    cpuc_enabled: bool,
) -> KncIrqResult {
    let mut handled = 0u32;
    let mut ack_count = 0u32;
    let mut loop_stuck = false;
    if statuses.is_empty() || statuses[0] == 0 {
        return KncIrqResult {
            handled,
            ack_count,
            loop_stuck,
            reenabled: true,
        };
    }

    let mut index = 0usize;
    let mut loops = 0u32;
    while index < statuses.len() {
        let status = statuses[index];
        if status == 0 {
            break;
        }
        ack_count += 1;
        loops += 1;
        if loops > 100 {
            loop_stuck = true;
            break;
        }

        let mut bit = 0u32;
        while bit < 64 {
            if (status & (1u64 << bit)) != 0 {
                handled += 1;
                let _active = (active_mask & (1u64 << bit)) != 0;
            }
            bit += 1;
        }
        index += 1;
    }

    KncIrqResult {
        handled,
        ack_count,
        loop_stuck,
        reenabled: cpuc_enabled,
    }
}

pub const fn knc_pmu_init_plan() -> KncInitPlan {
    KncInitPlan {
        pmu: KNC_PMU,
        copy_hw_cache_event_ids: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knc_is_model_specific() {
        assert!(knc_pmu_capabilities(KNC_MODEL).is_some());
        assert_eq!(knc_pmu_capabilities(0x55), None);
        assert_eq!(knc_pmu_capabilities(KNC_MODEL).unwrap().version, 0);
    }

    #[test]
    fn knc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/intel/knc.c"
        ));
        assert!(source.contains("static const u64 knc_perfmon_event_map[]"));
        assert!(source.contains("[PERF_COUNT_HW_CPU_CYCLES]\t\t= 0x002a"));
        assert!(source.contains("knc_hw_cache_event_ids"));
        assert!(source.contains("INTEL_EVENT_CONSTRAINT(0xc3, 0x1)"));
        assert!(source.contains("MSR_KNC_IA32_PERF_GLOBAL_STATUS"));
        assert!(source.contains("KNC_ENABLE_COUNTER0"));
        assert!(source.contains("val &= ~(KNC_ENABLE_COUNTER0|KNC_ENABLE_COUNTER1);"));
        assert!(source.contains("val |= (KNC_ENABLE_COUNTER0|KNC_ENABLE_COUNTER1);"));
        assert!(source.contains("val &= ~ARCH_PERFMON_EVENTSEL_ENABLE;"));
        assert!(source.contains("val |= ARCH_PERFMON_EVENTSEL_ENABLE;"));
        assert!(source.contains("knc_pmu_handle_irq"));
        assert!(source.contains("if (++loops > 100)"));
        assert!(
            source.contains("for_each_set_bit(bit, (unsigned long *)&status, X86_PMC_IDX_MAX)")
        );
        assert!(source.contains("PMU_FORMAT_ATTR(event,\t\"config:0-7\""));
        assert!(source.contains(".name\t\t\t= \"knc\""));
        assert!(source.contains(".eventsel\t\t= MSR_KNC_EVNTSEL0"));
        assert!(source.contains(".cntval_bits\t\t= 40"));
        assert!(source.contains("memcpy(hw_cache_event_ids, knc_hw_cache_event_ids"));
    }

    #[test]
    fn knc_event_and_cache_tables_match_linux_constants() {
        assert_eq!(knc_pmu_event_map(PERF_COUNT_HW_CPU_CYCLES), Some(0x002a));
        assert_eq!(knc_pmu_event_map(PERF_COUNT_HW_BRANCH_MISSES), Some(0x002b));
        assert_eq!(
            knc_hw_cache_event_id(CACHE_L1D, OP_READ, RESULT_ACCESS),
            Some(EVNTSEL_INT as i64)
        );
        assert_eq!(
            knc_hw_cache_event_id(CACHE_LL, OP_PREFETCH, RESULT_MISS),
            Some(0x10fe)
        );
        assert_eq!(
            knc_hw_cache_event_id(CACHE_ITLB, OP_WRITE, RESULT_ACCESS),
            Some(-1)
        );
    }

    #[test]
    fn knc_constraints_formats_and_descriptor_match_source_shape() {
        assert_eq!(KNC_EVENT_CONSTRAINTS[0], intel_event_constraint(0xc3, 0x1));
        assert_eq!(KNC_EVENT_CONSTRAINTS[20], intel_event_constraint(0xff, 0x1));
        assert_eq!(KNC_FORMAT_ATTRS[0], "event:config:0-7");
        assert_eq!(KNC_FORMAT_ATTRS[4], "cmask:config:24-31");
        assert_eq!(KNC_PMU.name, "knc");
        assert_eq!(KNC_PMU.eventsel, MSR_KNC_EVNTSEL0);
        assert_eq!(KNC_PMU.perfctr, MSR_KNC_PERFCTR0);
        assert_eq!(KNC_PMU.max_period, (1u64 << 39) - 1);
        assert_eq!(KNC_PMU.cntval_bits, 40);
        assert_eq!(KNC_PMU.cntval_mask, (1u64 << 40) - 1);
        assert!(knc_pmu_init_plan().copy_hw_cache_event_ids);
    }

    #[test]
    fn knc_counter_enable_and_event_enable_match_msr_ops() {
        assert_eq!(knc_pmu_disable_all(KNC_ENABLE_COUNTERS | 0x80), 0x80);
        assert_eq!(knc_pmu_enable_all(0x80), KNC_ENABLE_COUNTERS | 0x80);
        assert_eq!(knc_pmu_disable_event_config(EVNTSEL_ENABLE | 0x2a), 0x2a);
        assert_eq!(knc_pmu_enable_event_config(0x2a), EVNTSEL_ENABLE | 0x2a);
        assert_eq!(knc_pmu_event_msr(MSR_KNC_EVNTSEL0, 1), 0x29);
    }

    #[test]
    fn knc_irq_plan_acks_status_until_empty_or_loop_limit() {
        assert_eq!(
            knc_pmu_handle_irq_plan(&[0], 0, true),
            KncIrqResult {
                handled: 0,
                ack_count: 0,
                loop_stuck: false,
                reenabled: true,
            }
        );
        assert_eq!(
            knc_pmu_handle_irq_plan(&[0b101, 0], 0b001, true),
            KncIrqResult {
                handled: 2,
                ack_count: 1,
                loop_stuck: false,
                reenabled: true,
            }
        );
        let stuck = [1u64; 101];
        let result = knc_pmu_handle_irq_plan(&stuck, 1, false);
        assert!(result.loop_stuck);
        assert_eq!(result.ack_count, 101);
        assert!(!result.reenabled);
    }
}
