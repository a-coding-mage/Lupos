//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/zhaoxin/core.c
//! test-origin: linux:vendor/linux/arch/x86/events/zhaoxin/core.c
//! Zhaoxin architectural PMU model.

use crate::arch::x86::events::core::{PmuFeature, PmuVendor, X86PmuCapabilities};
use crate::arch::x86::events::utils::EventConstraint;
use crate::include::uapi::errno::ENODEV;

pub const PERF_COUNT_HW_CPU_CYCLES: usize = 0;
pub const PERF_COUNT_HW_INSTRUCTIONS: usize = 1;
pub const PERF_COUNT_HW_CACHE_REFERENCES: usize = 2;
pub const PERF_COUNT_HW_CACHE_MISSES: usize = 3;
pub const PERF_COUNT_HW_BRANCH_INSTRUCTIONS: usize = 4;
pub const PERF_COUNT_HW_BRANCH_MISSES: usize = 5;
pub const PERF_COUNT_HW_BUS_CYCLES: usize = 6;
pub const PERF_COUNT_HW_STALLED_CYCLES_FRONTEND: usize = 7;
pub const PERF_COUNT_HW_STALLED_CYCLES_BACKEND: usize = 8;
pub const PERF_COUNT_HW_REF_CPU_CYCLES: usize = 9;
pub const PERF_COUNT_HW_MAX: usize = 10;

pub const PERF_COUNT_HW_CACHE_MAX: usize = 7;
pub const PERF_COUNT_HW_CACHE_OP_MAX: usize = 3;
pub const PERF_COUNT_HW_CACHE_RESULT_MAX: usize = 2;

pub const CACHE_L1D: usize = 0;
pub const CACHE_L1I: usize = 1;
pub const CACHE_LL: usize = 2;
pub const CACHE_DTLB: usize = 3;
pub const CACHE_ITLB: usize = 4;
pub const CACHE_BPU: usize = 5;
pub const CACHE_NODE: usize = 6;

pub const OP_READ: usize = 0;
pub const OP_WRITE: usize = 1;
pub const OP_PREFETCH: usize = 2;

pub const RESULT_ACCESS: usize = 0;
pub const RESULT_MISS: usize = 1;

pub const ARCH_PERFMON_EVENTS_COUNT: u8 = 7;
pub const INTEL_PMC_IDX_FIXED: u8 = 32;
pub const ARCH_PERFMON_EVENTSEL_EVENT: u64 = 0x0000_00ff;
pub const ARCH_PERFMON_EVENTSEL_USR: u64 = 1 << 16;
pub const ARCH_PERFMON_EVENTSEL_OS: u64 = 1 << 17;
pub const ARCH_PERFMON_EVENTSEL_ENABLE: u64 = 1 << 22;
pub const MSR_ARCH_PERFMON_PERFCTR0: u32 = 0x0000_00c1;
pub const MSR_ARCH_PERFMON_EVENTSEL0: u32 = 0x0000_0186;
pub const MSR_ARCH_PERFMON_FIXED_CTR_CTRL: u32 = 0x0000_038d;
pub const MSR_CORE_PERF_GLOBAL_STATUS: u32 = 0x0000_038e;
pub const MSR_CORE_PERF_GLOBAL_CTRL: u32 = 0x0000_038f;
pub const MSR_CORE_PERF_GLOBAL_OVF_CTRL: u32 = 0x0000_0390;
pub const APIC_LVTPC: u32 = 0x340;
pub const APIC_DM_NMI: u32 = 0x400;

pub const ZX_PMON_EVENT_MAP: [u64; PERF_COUNT_HW_MAX] =
    [0x0082, 0x00c0, 0x0515, 0x051a, 0, 0, 0x0083, 0, 0, 0];

pub const ZXC_EVENT_CONSTRAINTS: [EventConstraint; 1] = [fixed_event_constraint(0x0082, 1)];
pub const ZXD_EVENT_CONSTRAINTS: [EventConstraint; 3] = [
    fixed_event_constraint(0x00c0, 0),
    fixed_event_constraint(0x0082, 1),
    fixed_event_constraint(0x0083, 2),
];

pub const ZXD_HW_CACHE_EVENT_IDS: [[[i64; PERF_COUNT_HW_CACHE_RESULT_MAX];
    PERF_COUNT_HW_CACHE_OP_MAX]; PERF_COUNT_HW_CACHE_MAX] = [
    [[0x0042, 0x0538], [0x0043, 0x0562], [-1, -1]],
    [[0x0300, 0x0301], [-1, -1], [0x030a, 0x030b]],
    [[-1, -1], [-1, -1], [-1, -1]],
    [[0x0042, 0x052c], [0x0043, 0x0530], [0x0564, 0x0565]],
    [[0x00c0, 0x0534], [-1, -1], [-1, -1]],
    [[0x0700, 0x0709], [-1, -1], [-1, -1]],
    [[-1, -1], [-1, -1], [-1, -1]],
];

pub const ZXE_HW_CACHE_EVENT_IDS: [[[i64; PERF_COUNT_HW_CACHE_RESULT_MAX];
    PERF_COUNT_HW_CACHE_OP_MAX]; PERF_COUNT_HW_CACHE_MAX] = [
    [[0x0568, 0x054b], [0x0669, 0x0562], [-1, -1]],
    [[0x0300, 0x0301], [-1, -1], [0x030a, 0x030b]],
    [[0, 0], [0, 0], [0, 0]],
    [[0x0568, 0x052c], [0x0669, 0x0530], [0x0564, 0x0565]],
    [[0x00c0, 0x0534], [-1, -1], [-1, -1]],
    [[0x0028, 0x0029], [-1, -1], [-1, -1]],
    [[-1, -1], [-1, -1], [-1, -1]],
];

pub const ZX_ARCH_FORMATS: [&str; 5] = [
    "event:config:0-7",
    "umask:config:8-15",
    "edge:config:18",
    "inv:config:23",
    "cmask:config:24-31",
];

pub const ZX_ARCH_EVENTS_MAP: [(usize, &str); 7] = [
    (PERF_COUNT_HW_CPU_CYCLES, "cpu cycles"),
    (PERF_COUNT_HW_INSTRUCTIONS, "instructions"),
    (PERF_COUNT_HW_BUS_CYCLES, "bus cycles"),
    (PERF_COUNT_HW_CACHE_REFERENCES, "cache references"),
    (PERF_COUNT_HW_CACHE_MISSES, "cache misses"),
    (PERF_COUNT_HW_BRANCH_INSTRUCTIONS, "branch instructions"),
    (PERF_COUNT_HW_BRANCH_MISSES, "branch misses"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZhaoxinProfile {
    Zxc,
    Zxd,
    Zxe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinHwEvent {
    pub idx: u8,
    pub config: u64,
    pub config_base: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinMsrWrite {
    pub msr: u32,
    pub value: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinIrqResult {
    pub handled: u32,
    pub apic_write: (u32, u32),
    pub disabled_global_ctrl: bool,
    pub enabled_global_ctrl: bool,
    pub ack_count: u32,
    pub irq_stat_inc: u32,
    pub overflow_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinPmuDescriptor {
    pub name: &'static str,
    pub eventsel: u32,
    pub perfctr: u32,
    pub max_events: usize,
    pub apic: bool,
    pub max_period: u64,
    pub formats: &'static [&'static str],
}

pub const ZHAOXIN_PMU_DESCRIPTOR: ZhaoxinPmuDescriptor = ZhaoxinPmuDescriptor {
    name: "zhaoxin",
    eventsel: MSR_ARCH_PERFMON_EVENTSEL0,
    perfctr: MSR_ARCH_PERFMON_PERFCTR0,
    max_events: PERF_COUNT_HW_MAX,
    apic: true,
    max_period: (1u64 << 47) - 1,
    formats: &ZX_ARCH_FORMATS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinCpuid10 {
    pub version_id: u8,
    pub num_counters: u8,
    pub bit_width: u8,
    pub mask_length: u8,
    pub events_mask: u64,
    pub fixed_counters: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinCpuId {
    pub family: u8,
    pub model: u8,
    pub stepping: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZhaoxinCacheTable {
    None,
    Zxd,
    Zxe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinInitPlan {
    pub capabilities: X86PmuCapabilities,
    pub profile: ZhaoxinProfile,
    pub event_map: [u64; PERF_COUNT_HW_MAX],
    pub cache_table: ZhaoxinCacheTable,
    pub constraints: &'static [EventConstraint],
    pub cntr_mask64: u64,
    pub cntval_bits: u8,
    pub cntval_mask: u64,
    pub events_maskl: u64,
    pub events_mask_len: u8,
    pub fixed_cntr_mask64: u64,
    pub intel_ctrl: u64,
    pub enabled_ack: bool,
    pub max_period: u64,
    pub add_arch_events_quirk: bool,
}

pub const fn fixed_event_constraint(event: u64, fixed_counter: u8) -> EventConstraint {
    EventConstraint {
        event,
        mask: 0xffff,
        counters: 1u64 << (INTEL_PMC_IDX_FIXED + fixed_counter),
    }
}

pub const fn genmask_count(count: u8) -> u64 {
    if count == 0 {
        0
    } else if count >= 64 {
        u64::MAX
    } else {
        (1u64 << count) - 1
    }
}

pub const fn cntval_mask(bits: u8) -> u64 {
    genmask_count(bits)
}

pub const fn x86_config(event: u8, umask: u8, inv: bool, cmask: u8) -> u64 {
    let mut value = event as u64 | ((umask as u64) << 8) | ((cmask as u64) << 24);
    if inv {
        value |= 1 << 23;
    }
    value
}

pub const fn zhaoxin_pmu_disable_all() -> ZhaoxinMsrWrite {
    ZhaoxinMsrWrite {
        msr: MSR_CORE_PERF_GLOBAL_CTRL,
        value: 0,
    }
}

pub const fn zhaoxin_pmu_enable_all(intel_ctrl: u64) -> ZhaoxinMsrWrite {
    ZhaoxinMsrWrite {
        msr: MSR_CORE_PERF_GLOBAL_CTRL,
        value: intel_ctrl,
    }
}

pub const fn zhaoxin_pmu_ack_status(ack: u64) -> ZhaoxinMsrWrite {
    ZhaoxinMsrWrite {
        msr: MSR_CORE_PERF_GLOBAL_OVF_CTRL,
        value: ack,
    }
}

pub const fn zxc_pmu_ack_status_plan(ack: u64, intel_ctrl: u64) -> [ZhaoxinMsrWrite; 3] {
    [
        zhaoxin_pmu_enable_all(intel_ctrl),
        zhaoxin_pmu_ack_status(ack),
        zhaoxin_pmu_disable_all(),
    ]
}

pub const fn zhaoxin_pmu_disable_fixed(hwc: ZhaoxinHwEvent, fixed_ctrl: u64) -> ZhaoxinMsrWrite {
    let idx = hwc.idx - INTEL_PMC_IDX_FIXED;
    let mask = 0x0f_u64 << (idx * 4);
    ZhaoxinMsrWrite {
        msr: hwc.config_base,
        value: fixed_ctrl & !mask,
    }
}

pub const fn zhaoxin_pmu_enable_fixed(hwc: ZhaoxinHwEvent, fixed_ctrl: u64) -> ZhaoxinMsrWrite {
    let idx = hwc.idx - INTEL_PMC_IDX_FIXED;
    let mut bits = 0x8_u64;
    if (hwc.config & ARCH_PERFMON_EVENTSEL_USR) != 0 {
        bits |= 0x2;
    }
    if (hwc.config & ARCH_PERFMON_EVENTSEL_OS) != 0 {
        bits |= 0x1;
    }
    bits <<= idx * 4;
    let mask = 0x0f_u64 << (idx * 4);
    ZhaoxinMsrWrite {
        msr: hwc.config_base,
        value: (fixed_ctrl & !mask) | bits,
    }
}

pub const fn zhaoxin_pmu_enable_event_plan(
    hwc: ZhaoxinHwEvent,
    fixed_ctrl: u64,
) -> ZhaoxinMsrWrite {
    if hwc.config_base == MSR_ARCH_PERFMON_FIXED_CTR_CTRL {
        zhaoxin_pmu_enable_fixed(hwc, fixed_ctrl)
    } else {
        ZhaoxinMsrWrite {
            msr: hwc.config_base,
            value: hwc.config | ARCH_PERFMON_EVENTSEL_ENABLE,
        }
    }
}

pub const fn zhaoxin_pmu_disable_event_plan(
    hwc: ZhaoxinHwEvent,
    fixed_ctrl: u64,
) -> ZhaoxinMsrWrite {
    if hwc.config_base == MSR_ARCH_PERFMON_FIXED_CTR_CTRL {
        zhaoxin_pmu_disable_fixed(hwc, fixed_ctrl)
    } else {
        ZhaoxinMsrWrite {
            msr: hwc.config_base,
            value: hwc.config & !ARCH_PERFMON_EVENTSEL_ENABLE,
        }
    }
}

pub fn zhaoxin_pmu_handle_irq(
    statuses: &[u64],
    active_mask: u64,
    period_reload_mask: u64,
    enabled_ack: bool,
) -> ZhaoxinIrqResult {
    let mut result = ZhaoxinIrqResult {
        handled: 0,
        apic_write: (APIC_LVTPC, APIC_DM_NMI),
        disabled_global_ctrl: true,
        enabled_global_ctrl: true,
        ack_count: 0,
        irq_stat_inc: 0,
        overflow_count: 0,
    };

    let mut index = 0usize;
    while index < statuses.len() {
        let mut status = statuses[index];
        if status == 0 {
            break;
        }

        let _ack_uses_zxc_sequence = enabled_ack;
        result.ack_count += 1;
        result.irq_stat_inc += 1;

        status &= !(1u64 << 63);
        if status == 0 {
            break;
        }

        let mut bit = 0u8;
        while bit < 64 {
            if (status & (1u64 << bit)) != 0 {
                result.handled += 1;
                if (active_mask & (1u64 << bit)) != 0 && (period_reload_mask & (1u64 << bit)) != 0 {
                    result.overflow_count += 1;
                }
            }
            bit += 1;
        }

        index += 1;
    }

    result
}

pub const fn zhaoxin_pmu_event_map(hw_event: usize, event_map: &[u64; PERF_COUNT_HW_MAX]) -> u64 {
    event_map[hw_event]
}

pub const fn zhaoxin_get_event_constraints(
    event_config: u64,
    constraints: &'static [EventConstraint],
) -> Option<EventConstraint> {
    let mut index = 0usize;
    while index < constraints.len() {
        let constraint = constraints[index];
        if (event_config & constraint.mask) == constraint.event {
            return Some(constraint);
        }
        index += 1;
    }
    None
}

pub const fn zhaoxin_event_sysfs_event(config: u64) -> u64 {
    config & ARCH_PERFMON_EVENTSEL_EVENT
}

pub const fn zhaoxin_arch_events_quirk(
    mut event_map: [u64; PERF_COUNT_HW_MAX],
    events_mask: u64,
) -> [u64; PERF_COUNT_HW_MAX] {
    let mut bit = 0usize;
    while bit < ZX_ARCH_EVENTS_MAP.len() {
        if (events_mask & (1u64 << bit)) != 0 {
            event_map[ZX_ARCH_EVENTS_MAP[bit].0] = 0;
        }
        bit += 1;
    }
    event_map
}

pub const fn zhaoxin_pmu_init(
    cpuid: ZhaoxinCpuid10,
    cpu: ZhaoxinCpuId,
) -> Result<ZhaoxinInitPlan, i32> {
    if cpuid.mask_length < ARCH_PERFMON_EVENTS_COUNT - 1 {
        return Err(ENODEV);
    }
    if cpuid.version_id != 2 {
        return Err(ENODEV);
    }

    let cntr_mask64 = genmask_count(cpuid.num_counters);
    let cntval_mask = cntval_mask(cpuid.bit_width);
    let fixed_cntr_mask64 = genmask_count(cpuid.fixed_counters);
    let mut event_map = ZX_PMON_EVENT_MAP;
    let mut cache_table = ZhaoxinCacheTable::None;
    let mut constraints: &'static [EventConstraint] = &[];
    let mut enabled_ack = false;
    let mut max_period = (1u64 << 47) - 1;
    let profile;

    match cpu.family {
        0x06 => {
            if (cpu.model == 0x0f && cpu.stepping >= 0x0e) || cpu.model == 0x19 {
                profile = ZhaoxinProfile::Zxc;
                max_period = cntval_mask >> 1;
                enabled_ack = true;
                constraints = &ZXC_EVENT_CONSTRAINTS;
                event_map[PERF_COUNT_HW_INSTRUCTIONS] = 0;
                event_map[PERF_COUNT_HW_CACHE_REFERENCES] = 0;
                event_map[PERF_COUNT_HW_CACHE_MISSES] = 0;
                event_map[PERF_COUNT_HW_BUS_CYCLES] = 0;
            } else {
                return Err(ENODEV);
            }
        }
        0x07 => {
            event_map[PERF_COUNT_HW_STALLED_CYCLES_FRONTEND] = x86_config(0x01, 0x01, true, 0x01);
            event_map[PERF_COUNT_HW_STALLED_CYCLES_BACKEND] = x86_config(0x0f, 0x04, false, 0);
            constraints = &ZXD_EVENT_CONSTRAINTS;
            match cpu.model {
                0x1b => {
                    profile = ZhaoxinProfile::Zxd;
                    cache_table = ZhaoxinCacheTable::Zxd;
                    event_map[PERF_COUNT_HW_BRANCH_INSTRUCTIONS] = 0x0700;
                    event_map[PERF_COUNT_HW_BRANCH_MISSES] = 0x0709;
                }
                0x3b => {
                    profile = ZhaoxinProfile::Zxe;
                    cache_table = ZhaoxinCacheTable::Zxe;
                    event_map[PERF_COUNT_HW_BRANCH_INSTRUCTIONS] = 0x0028;
                    event_map[PERF_COUNT_HW_BRANCH_MISSES] = 0x0029;
                }
                _ => return Err(ENODEV),
            }
        }
        _ => return Err(ENODEV),
    }

    let intel_ctrl = cntr_mask64 | (fixed_cntr_mask64 << INTEL_PMC_IDX_FIXED);
    let capabilities = X86PmuCapabilities {
        vendor: PmuVendor::Zhaoxin,
        version: cpuid.version_id,
        counters: cpuid.num_counters,
        counter_bits: cpuid.bit_width,
        fixed_counters: cpuid.fixed_counters,
        features: 0,
    }
    .with_feature(PmuFeature::CoreCounters)
    .with_feature(PmuFeature::FixedCounters);

    Ok(ZhaoxinInitPlan {
        capabilities,
        profile,
        event_map,
        cache_table,
        constraints,
        cntr_mask64,
        cntval_bits: cpuid.bit_width,
        cntval_mask,
        events_maskl: cpuid.events_mask,
        events_mask_len: cpuid.mask_length,
        fixed_cntr_mask64,
        intel_ctrl,
        enabled_ack,
        max_period,
        add_arch_events_quirk: true,
    })
}

pub const fn zhaoxin_pmu_capabilities(cpuid_pmu: bool) -> X86PmuCapabilities {
    if !cpuid_pmu {
        return X86PmuCapabilities {
            vendor: PmuVendor::Zhaoxin,
            version: 0,
            counters: 0,
            counter_bits: 0,
            fixed_counters: 0,
            features: 0,
        };
    }
    match zhaoxin_pmu_init(
        ZhaoxinCpuid10 {
            version_id: 2,
            num_counters: 4,
            bit_width: 48,
            mask_length: ARCH_PERFMON_EVENTS_COUNT,
            events_mask: 0,
            fixed_counters: 3,
        },
        ZhaoxinCpuId {
            family: 0x07,
            model: 0x1b,
            stepping: 0,
        },
    ) {
        Ok(plan) => plan.capabilities,
        Err(_) => X86PmuCapabilities {
            vendor: PmuVendor::Zhaoxin,
            version: 0,
            counters: 0,
            counter_bits: 0,
            fixed_counters: 0,
            features: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CPUID_OK: ZhaoxinCpuid10 = ZhaoxinCpuid10 {
        version_id: 2,
        num_counters: 4,
        bit_width: 48,
        mask_length: ARCH_PERFMON_EVENTS_COUNT,
        events_mask: 0,
        fixed_counters: 3,
    };

    #[test]
    fn zhaoxin_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/zhaoxin/core.c"
        ));
        let perf_event = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/perf_event.h"
        ));
        let asm_perf = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/perf_event.h"
        ));
        let msr_index = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/msr-index.h"
        ));

        assert!(source.contains("static u64 zx_pmon_event_map[PERF_COUNT_HW_MAX]"));
        assert!(source.contains("[PERF_COUNT_HW_CPU_CYCLES]        = 0x0082"));
        assert!(source.contains("FIXED_EVENT_CONSTRAINT(0x0082, 1)"));
        assert!(source.contains("FIXED_EVENT_CONSTRAINT(0x00c0, 0)"));
        assert!(source.contains("zxd_hw_cache_event_ids"));
        assert!(source.contains("zxe_hw_cache_event_ids"));
        assert!(source.contains("wrmsrq(MSR_CORE_PERF_GLOBAL_CTRL, 0);"));
        assert!(source.contains("wrmsrq(MSR_CORE_PERF_GLOBAL_CTRL, x86_pmu.intel_ctrl);"));
        assert!(source.contains("wrmsrq(MSR_CORE_PERF_GLOBAL_OVF_CTRL, ack);"));
        assert!(source.contains("ZXC needs global control enabled"));
        assert!(source.contains("mask = 0xfULL << (idx * 4);"));
        assert!(source.contains("bits = 0x8ULL;"));
        assert!(source.contains("bits |= 0x2;"));
        assert!(source.contains("bits |= 0x1;"));
        assert!(source.contains("apic_write(APIC_LVTPC, APIC_DM_NMI);"));
        assert!(source.contains("if (__test_and_clear_bit(63, (unsigned long *)&status))"));
        assert!(source.contains("return zx_pmon_event_map[hw_event];"));
        assert!(source.contains("PMU_FORMAT_ATTR(event,\t\"config:0-7\");"));
        assert!(source.contains(".name\t\t\t= \"zhaoxin\""));
        assert!(source.contains(".max_period\t\t= (1ULL << 47) - 1"));
        assert!(source.contains("zx_arch_events_map"));
        assert!(source.contains("if (eax.split.mask_length < ARCH_PERFMON_EVENTS_COUNT - 1)"));
        assert!(source.contains("if (version != 2)"));
        assert!(
            source.contains(
                "x86_pmu.intel_ctrl |= x86_pmu.fixed_cntr_mask64 << INTEL_PMC_IDX_FIXED;"
            )
        );
        assert!(perf_event.contains("union x86_pmu_config"));
        assert!(asm_perf.contains("#define ARCH_PERFMON_EVENTS_COUNT\t\t\t7"));
        assert!(msr_index.contains("#define MSR_CORE_PERF_GLOBAL_CTRL\t0x0000038f"));
    }

    #[test]
    fn event_maps_constraints_formats_and_cache_tables_match_linux() {
        assert_eq!(ZX_PMON_EVENT_MAP[PERF_COUNT_HW_CPU_CYCLES], 0x0082);
        assert_eq!(ZX_PMON_EVENT_MAP[PERF_COUNT_HW_INSTRUCTIONS], 0x00c0);
        assert_eq!(ZX_PMON_EVENT_MAP[PERF_COUNT_HW_BUS_CYCLES], 0x0083);
        assert_eq!(ZXC_EVENT_CONSTRAINTS[0], fixed_event_constraint(0x0082, 1));
        assert_eq!(ZXD_EVENT_CONSTRAINTS[0], fixed_event_constraint(0x00c0, 0));
        assert_eq!(
            ZXD_HW_CACHE_EVENT_IDS[CACHE_L1D][OP_READ][RESULT_MISS],
            0x0538
        );
        assert_eq!(
            ZXD_HW_CACHE_EVENT_IDS[CACHE_BPU][OP_READ][RESULT_MISS],
            0x0709
        );
        assert_eq!(
            ZXE_HW_CACHE_EVENT_IDS[CACHE_L1D][OP_WRITE][RESULT_ACCESS],
            0x0669
        );
        assert_eq!(
            ZXE_HW_CACHE_EVENT_IDS[CACHE_BPU][OP_READ][RESULT_ACCESS],
            0x0028
        );
        assert_eq!(ZX_ARCH_FORMATS[0], "event:config:0-7");
        assert_eq!(ZX_ARCH_FORMATS[4], "cmask:config:24-31");
        assert_eq!(ZHAOXIN_PMU_DESCRIPTOR.name, "zhaoxin");
        assert_eq!(ZHAOXIN_PMU_DESCRIPTOR.max_period, (1u64 << 47) - 1);
    }

    #[test]
    fn fixed_and_global_msr_plans_follow_source_bit_layout() {
        assert_eq!(
            zhaoxin_pmu_disable_all(),
            ZhaoxinMsrWrite {
                msr: MSR_CORE_PERF_GLOBAL_CTRL,
                value: 0,
            }
        );
        assert_eq!(
            zhaoxin_pmu_enable_all(0x33),
            ZhaoxinMsrWrite {
                msr: MSR_CORE_PERF_GLOBAL_CTRL,
                value: 0x33,
            }
        );
        assert_eq!(
            zxc_pmu_ack_status_plan(0x55, 0xaa),
            [
                ZhaoxinMsrWrite {
                    msr: MSR_CORE_PERF_GLOBAL_CTRL,
                    value: 0xaa,
                },
                ZhaoxinMsrWrite {
                    msr: MSR_CORE_PERF_GLOBAL_OVF_CTRL,
                    value: 0x55,
                },
                ZhaoxinMsrWrite {
                    msr: MSR_CORE_PERF_GLOBAL_CTRL,
                    value: 0,
                },
            ]
        );

        let hwc = ZhaoxinHwEvent {
            idx: INTEL_PMC_IDX_FIXED + 1,
            config: ARCH_PERFMON_EVENTSEL_USR | ARCH_PERFMON_EVENTSEL_OS,
            config_base: MSR_ARCH_PERFMON_FIXED_CTR_CTRL,
        };
        assert_eq!(
            zhaoxin_pmu_enable_fixed(hwc, 0xffff),
            ZhaoxinMsrWrite {
                msr: MSR_ARCH_PERFMON_FIXED_CTR_CTRL,
                value: (0xffff & !(0x0f << 4)) | (0x0b << 4),
            }
        );
        assert_eq!(
            zhaoxin_pmu_disable_fixed(hwc, 0xffff).value,
            0xffff & !(0x0f << 4)
        );
        assert_eq!(
            zhaoxin_pmu_enable_event_plan(
                ZhaoxinHwEvent {
                    idx: 0,
                    config: 0x12,
                    config_base: 0x186,
                },
                0
            )
            .value,
            0x12 | ARCH_PERFMON_EVENTSEL_ENABLE
        );
    }

    #[test]
    fn irq_plan_acks_clears_condchgd_counts_active_overflows_and_reenables() {
        let result = zhaoxin_pmu_handle_irq(&[0], 0, 0, false);
        assert_eq!(result.handled, 0);
        assert!(result.disabled_global_ctrl);
        assert!(result.enabled_global_ctrl);

        let result = zhaoxin_pmu_handle_irq(&[(1 << 63) | 0b101, 0], 0b001, 0b001, true);
        assert_eq!(result.apic_write, (APIC_LVTPC, APIC_DM_NMI));
        assert_eq!(result.ack_count, 1);
        assert_eq!(result.irq_stat_inc, 1);
        assert_eq!(result.handled, 2);
        assert_eq!(result.overflow_count, 1);
    }

    #[test]
    fn init_rejects_bad_cpuid_and_selects_zxc_zxd_zxe_profiles() {
        assert_eq!(
            zhaoxin_pmu_init(
                ZhaoxinCpuid10 {
                    mask_length: 5,
                    ..CPUID_OK
                },
                ZhaoxinCpuId {
                    family: 0x07,
                    model: 0x1b,
                    stepping: 0,
                },
            ),
            Err(ENODEV)
        );
        assert_eq!(
            zhaoxin_pmu_init(
                ZhaoxinCpuid10 {
                    version_id: 3,
                    ..CPUID_OK
                },
                ZhaoxinCpuId {
                    family: 0x07,
                    model: 0x1b,
                    stepping: 0,
                },
            ),
            Err(ENODEV)
        );

        let zxc = zhaoxin_pmu_init(
            CPUID_OK,
            ZhaoxinCpuId {
                family: 0x06,
                model: 0x0f,
                stepping: 0x0e,
            },
        )
        .unwrap();
        assert_eq!(zxc.profile, ZhaoxinProfile::Zxc);
        assert!(zxc.enabled_ack);
        assert_eq!(zxc.max_period, zxc.cntval_mask >> 1);
        assert_eq!(zxc.event_map[PERF_COUNT_HW_INSTRUCTIONS], 0);
        assert_eq!(zxc.constraints, &ZXC_EVENT_CONSTRAINTS);

        let zxd = zhaoxin_pmu_init(
            CPUID_OK,
            ZhaoxinCpuId {
                family: 0x07,
                model: 0x1b,
                stepping: 0,
            },
        )
        .unwrap();
        assert_eq!(zxd.profile, ZhaoxinProfile::Zxd);
        assert_eq!(zxd.cache_table, ZhaoxinCacheTable::Zxd);
        assert_eq!(zxd.event_map[PERF_COUNT_HW_BRANCH_INSTRUCTIONS], 0x0700);
        assert_eq!(
            zxd.event_map[PERF_COUNT_HW_STALLED_CYCLES_FRONTEND],
            x86_config(0x01, 0x01, true, 0x01)
        );
        assert_eq!(
            zxd.intel_ctrl,
            zxd.cntr_mask64 | (zxd.fixed_cntr_mask64 << 32)
        );

        let zxe = zhaoxin_pmu_init(
            CPUID_OK,
            ZhaoxinCpuId {
                family: 0x07,
                model: 0x3b,
                stepping: 0,
            },
        )
        .unwrap();
        assert_eq!(zxe.profile, ZhaoxinProfile::Zxe);
        assert_eq!(zxe.cache_table, ZhaoxinCacheTable::Zxe);
        assert_eq!(zxe.event_map[PERF_COUNT_HW_BRANCH_MISSES], 0x0029);

        assert_eq!(
            zhaoxin_pmu_init(
                CPUID_OK,
                ZhaoxinCpuId {
                    family: 0x06,
                    model: 0x0f,
                    stepping: 0x0d,
                },
            ),
            Err(ENODEV)
        );
    }

    #[test]
    fn quirk_event_sysfs_constraints_and_capability_helper_follow_source_shape() {
        let zxd = zhaoxin_pmu_init(
            CPUID_OK,
            ZhaoxinCpuId {
                family: 0x07,
                model: 0x1b,
                stepping: 0,
            },
        )
        .unwrap();
        assert_eq!(
            zhaoxin_pmu_event_map(PERF_COUNT_HW_CPU_CYCLES, &zxd.event_map),
            0x0082
        );
        assert_eq!(zhaoxin_event_sysfs_event(0x1234), 0x34);
        assert_eq!(
            zhaoxin_get_event_constraints(0x00c0, zxd.constraints),
            Some(fixed_event_constraint(0x00c0, 0))
        );
        assert_eq!(zhaoxin_get_event_constraints(0x7777, zxd.constraints), None);

        let quirked = zhaoxin_arch_events_quirk(zxd.event_map, (1 << 1) | (1 << 5));
        assert_eq!(quirked[PERF_COUNT_HW_INSTRUCTIONS], 0);
        assert_eq!(quirked[PERF_COUNT_HW_BRANCH_INSTRUCTIONS], 0);

        let caps = zhaoxin_pmu_capabilities(true);
        assert_eq!(caps.vendor, PmuVendor::Zhaoxin);
        assert_eq!(caps.version, 2);
        assert_eq!(caps.counters, 4);
        assert!(caps.has(PmuFeature::FixedCounters));
        assert_eq!(zhaoxin_pmu_capabilities(false).counters, 0);
    }
}
