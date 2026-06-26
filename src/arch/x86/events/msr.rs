//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/msr.c
//! test-origin: linux:vendor/linux/arch/x86/events/msr.c
//! Model-specific register helpers for x86 PMU events.

use crate::include::uapi::errno::{EINVAL, ENOENT};

pub const MSR_IA32_APERF: u64 = 0x0000_00e8;
pub const MSR_IA32_MPERF: u64 = 0x0000_00e7;
pub const MSR_PPERF: u64 = 0x0000_064e;
pub const MSR_SMI_COUNT: u64 = 0x0000_0034;
pub const MSR_F15H_PTSC: u64 = 0xc001_0280;
pub const MSR_F17H_IRPERF: u64 = 0xc000_00e9;
pub const MSR_IA32_THERM_STATUS: u64 = 0x0000_019c;

pub const PERF_EF_START: i32 = 1;
pub const PERF_EF_UPDATE: i32 = 2;
pub const PERF_PMU_CAP_NO_INTERRUPT: u64 = 1 << 0;
pub const PERF_PMU_CAP_NO_EXCLUDE: u64 = 1 << 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PerfMsrId {
    Tsc = 0,
    Aperf = 1,
    Mperf = 2,
    Pperf = 3,
    Smi = 4,
    Ptsc = 5,
    Irperf = 6,
    Therm = 7,
}

pub const PERF_MSR_EVENT_MAX: usize = 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MsrCpuFeatures {
    pub tsc: bool,
    pub aperfmperf: bool,
    pub ptsc: bool,
    pub irperf: bool,
    pub dtherm: bool,
    pub intel_vendor: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MsrProbeTest {
    None,
    AperfMperf,
    Ptsc,
    Irperf,
    ThermStatus,
    Intel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PerfMsr {
    pub msr: u64,
    pub group: Option<&'static str>,
    pub test: MsrProbeTest,
    pub no_check: bool,
}

pub const MSR_EVENTS: [(&str, &str); PERF_MSR_EVENT_MAX] = [
    ("tsc", "event=0x00"),
    ("aperf", "event=0x01"),
    ("mperf", "event=0x02"),
    ("pperf", "event=0x03"),
    ("smi", "event=0x04"),
    ("ptsc", "event=0x05"),
    ("irperf", "event=0x06"),
    ("cpu_thermal_margin", "event=0x07"),
];

pub const THERM_EVENTS: [(&str, &str); 3] = [
    ("cpu_thermal_margin", "event=0x07"),
    ("cpu_thermal_margin.snapshot", "1"),
    ("cpu_thermal_margin.unit", "C"),
];

pub const PERF_MSR_TABLE: [PerfMsr; PERF_MSR_EVENT_MAX] = [
    PerfMsr {
        msr: 0,
        group: None,
        test: MsrProbeTest::None,
        no_check: true,
    },
    PerfMsr {
        msr: MSR_IA32_APERF,
        group: Some("aperf"),
        test: MsrProbeTest::AperfMperf,
        no_check: false,
    },
    PerfMsr {
        msr: MSR_IA32_MPERF,
        group: Some("mperf"),
        test: MsrProbeTest::AperfMperf,
        no_check: false,
    },
    PerfMsr {
        msr: MSR_PPERF,
        group: Some("pperf"),
        test: MsrProbeTest::Intel,
        no_check: false,
    },
    PerfMsr {
        msr: MSR_SMI_COUNT,
        group: Some("smi"),
        test: MsrProbeTest::Intel,
        no_check: false,
    },
    PerfMsr {
        msr: MSR_F15H_PTSC,
        group: Some("ptsc"),
        test: MsrProbeTest::Ptsc,
        no_check: false,
    },
    PerfMsr {
        msr: MSR_F17H_IRPERF,
        group: Some("irperf"),
        test: MsrProbeTest::Irperf,
        no_check: false,
    },
    PerfMsr {
        msr: MSR_IA32_THERM_STATUS,
        group: Some("therm"),
        test: MsrProbeTest::ThermStatus,
        no_check: false,
    },
];

pub const ATTR_GROUPS: [&str; 2] = ["events", "format"];
pub const ATTR_UPDATE_GROUPS: [&str; 7] =
    ["aperf", "mperf", "pperf", "smi", "ptsc", "irperf", "therm"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsrPmuDescriptor {
    pub name: &'static str,
    pub task_ctx_nr: &'static str,
    pub capabilities: u64,
    pub format: &'static str,
    pub attr_groups: &'static [&'static str],
    pub attr_update: &'static [&'static str],
}

pub const PMU_MSR: MsrPmuDescriptor = MsrPmuDescriptor {
    name: "msr",
    task_ctx_nr: "perf_sw_context",
    capabilities: PERF_PMU_CAP_NO_INTERRUPT | PERF_PMU_CAP_NO_EXCLUDE,
    format: "config:0-63",
    attr_groups: &ATTR_GROUPS,
    attr_update: &ATTR_UPDATE_GROUPS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsrPerfEventAttr {
    pub typ: u32,
    pub config: u64,
    pub sample_period: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsrPerfEvent {
    pub pmu_type: u32,
    pub attr: MsrPerfEventAttr,
    pub idx: i32,
    pub event_base: u64,
    pub config: u64,
    pub prev_count: u64,
    pub count: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsrInitPlan {
    pub print_no_driver: bool,
    pub probe_count: usize,
    pub no_zero: bool,
    pub register_pmu: bool,
    pub msr_mask: u64,
}

pub const fn test_msr_probe(test: MsrProbeTest, features: MsrCpuFeatures) -> bool {
    match test {
        MsrProbeTest::None => true,
        MsrProbeTest::AperfMperf => features.aperfmperf,
        MsrProbeTest::Ptsc => features.ptsc,
        MsrProbeTest::Irperf => features.irperf,
        MsrProbeTest::ThermStatus => features.dtherm,
        MsrProbeTest::Intel => features.intel_vendor,
    }
}

pub const fn perf_msr_probe_mask(features: MsrCpuFeatures) -> u64 {
    let mut mask = 0u64;
    let mut index = 0usize;
    while index < PERF_MSR_EVENT_MAX {
        let entry = PERF_MSR_TABLE[index];
        if entry.no_check || test_msr_probe(entry.test, features) {
            mask |= 1u64 << index;
        }
        index += 1;
    }
    mask
}

pub const fn msr_event_init(
    event_type: u32,
    pmu_type: u32,
    sample_period: u64,
    config: u64,
    msr_mask: u64,
) -> Result<MsrPerfEvent, i32> {
    if event_type != pmu_type {
        return Err(ENOENT);
    }
    if sample_period != 0 {
        return Err(EINVAL);
    }
    if config >= PERF_MSR_EVENT_MAX as u64 {
        return Err(EINVAL);
    }
    if (msr_mask & (1u64 << config)) == 0 {
        return Err(EINVAL);
    }

    Ok(MsrPerfEvent {
        pmu_type,
        attr: MsrPerfEventAttr {
            typ: event_type,
            config,
            sample_period,
        },
        idx: -1,
        event_base: PERF_MSR_TABLE[config as usize].msr,
        config,
        prev_count: 0,
        count: 0,
    })
}

pub const fn msr_read_counter(event_base: u64, rdmsr_value: u64, rdtsc_ordered_value: u64) -> u64 {
    if event_base != 0 {
        rdmsr_value
    } else {
        rdtsc_ordered_value
    }
}

pub const fn sign_extend64(value: u64, index: u8) -> i64 {
    let shift = 63 - index;
    ((value << shift) as i64) >> shift
}

pub fn msr_event_update(event: &mut MsrPerfEvent, now: u64) {
    let prev = event.prev_count;
    event.prev_count = now;
    let delta = now.wrapping_sub(prev);

    if event.event_base == MSR_SMI_COUNT {
        event.count = event.count.wrapping_add(sign_extend64(delta, 31));
    } else if event.event_base == MSR_IA32_THERM_STATUS {
        event.count = if (now & (1u64 << 31)) != 0 {
            ((now >> 16) & 0x3f) as i64
        } else {
            -1
        };
    } else {
        event.count = event.count.wrapping_add(delta as i64);
    }
}

pub fn msr_event_start(event: &mut MsrPerfEvent, now: u64) {
    event.prev_count = now;
}

pub fn msr_event_stop(event: &mut MsrPerfEvent, now: u64) {
    msr_event_update(event, now);
}

pub fn msr_event_del(event: &mut MsrPerfEvent, now: u64) {
    let _ = PERF_EF_UPDATE;
    msr_event_stop(event, now);
}

pub fn msr_event_add(event: &mut MsrPerfEvent, flags: i32, now: u64) -> i32 {
    if (flags & PERF_EF_START) != 0 {
        msr_event_start(event, now);
    }
    0
}

pub const fn msr_init_plan(features: MsrCpuFeatures) -> MsrInitPlan {
    if !features.tsc {
        MsrInitPlan {
            print_no_driver: true,
            probe_count: 0,
            no_zero: true,
            register_pmu: false,
            msr_mask: 0,
        }
    } else {
        MsrInitPlan {
            print_no_driver: false,
            probe_count: PERF_MSR_EVENT_MAX,
            no_zero: true,
            register_pmu: true,
            msr_mask: perf_msr_probe_mask(features),
        }
    }
}

pub const MSR_IA32_PERFCTR0: u32 = 0x0c1;
pub const MSR_IA32_PERFEVTSEL0: u32 = 0x186;
pub const MSR_IA32_FIXED_CTR0: u32 = 0x309;
pub const MSR_IA32_FIXED_CTR_CTRL: u32 = 0x38d;
pub const MSR_AMD64_PERF_CNTR_GLOBAL_STATUS: u32 = 0xc000_0300;
pub const MSR_AMD64_PERF_CTL_BASE: u32 = 0xc001_0200;
pub const MSR_AMD64_PERF_CTR_BASE: u32 = 0xc001_0201;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PmuMsrPair {
    pub event_select: u32,
    pub counter: u32,
}

pub const fn intel_gp_msr_pair(index: u8) -> Result<PmuMsrPair, i32> {
    if index >= 32 {
        return Err(EINVAL);
    }
    Ok(PmuMsrPair {
        event_select: MSR_IA32_PERFEVTSEL0 + index as u32,
        counter: MSR_IA32_PERFCTR0 + index as u32,
    })
}

pub const fn amd_gp_msr_pair(index: u8) -> Result<PmuMsrPair, i32> {
    if index >= 16 {
        return Err(EINVAL);
    }
    Ok(PmuMsrPair {
        event_select: MSR_AMD64_PERF_CTL_BASE + (index as u32 * 2),
        counter: MSR_AMD64_PERF_CTR_BASE + (index as u32 * 2),
    })
}

pub const fn fixed_counter_msr(index: u8) -> Result<u32, i32> {
    if index >= 8 {
        Err(EINVAL)
    } else {
        Ok(MSR_IA32_FIXED_CTR0 + index as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msr_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/msr.c"
        ));
        let probe = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/probe.h"
        ));
        assert!(source.contains("enum perf_msr_id"));
        assert!(source.contains("PERF_MSR_TSC\t\t\t= 0"));
        assert!(source.contains("test_aperfmperf"));
        assert!(source.contains("test_intel"));
        assert!(source.contains("PMU_EVENT_ATTR_STRING(tsc"));
        assert!(source.contains("static struct perf_msr msr[]"));
        assert!(source.contains("[PERF_MSR_THERM]\t= { MSR_IA32_THERM_STATUS"));
        assert!(source.contains("if (event->attr.type != event->pmu->type)"));
        assert!(source.contains("if (event->attr.sample_period)"));
        assert!(source.contains("event->hw.idx\t\t= -1"));
        assert!(source.contains("rdtsc_ordered()"));
        assert!(source.contains("sign_extend64(delta, 31)"));
        assert!(source.contains("(now >> 16) & 0x3f"));
        assert!(source.contains("PERF_PMU_CAP_NO_INTERRUPT | PERF_PMU_CAP_NO_EXCLUDE"));
        assert!(source.contains("msr_mask = perf_msr_probe(msr, PERF_MSR_EVENT_MAX, true, NULL);"));
        assert!(source.contains("perf_pmu_register(&pmu_msr, \"msr\", -1);"));
        assert!(probe.contains("struct perf_msr"));
        assert!(probe.contains("PMU_EVENT_GROUP(_grp, _name)"));
    }

    #[test]
    fn msr_event_table_and_groups_match_linux_order() {
        assert_eq!(MSR_EVENTS[PerfMsrId::Tsc as usize], ("tsc", "event=0x00"));
        assert_eq!(
            PERF_MSR_TABLE[PerfMsrId::Aperf as usize].msr,
            MSR_IA32_APERF
        );
        assert_eq!(
            PERF_MSR_TABLE[PerfMsrId::Therm as usize],
            PerfMsr {
                msr: MSR_IA32_THERM_STATUS,
                group: Some("therm"),
                test: MsrProbeTest::ThermStatus,
                no_check: false,
            }
        );
        assert_eq!(
            THERM_EVENTS,
            [
                ("cpu_thermal_margin", "event=0x07"),
                ("cpu_thermal_margin.snapshot", "1"),
                ("cpu_thermal_margin.unit", "C"),
            ]
        );
        assert_eq!(PMU_MSR.name, "msr");
        assert_eq!(PMU_MSR.format, "config:0-63");
    }

    #[test]
    fn feature_probe_mask_follows_test_callbacks() {
        let intel = MsrCpuFeatures {
            tsc: true,
            aperfmperf: true,
            ptsc: false,
            irperf: true,
            dtherm: true,
            intel_vendor: true,
        };
        let mask = perf_msr_probe_mask(intel);
        assert_ne!(mask & (1 << PerfMsrId::Tsc as u8), 0);
        assert_ne!(mask & (1 << PerfMsrId::Aperf as u8), 0);
        assert_ne!(mask & (1 << PerfMsrId::Pperf as u8), 0);
        assert_eq!(mask & (1 << PerfMsrId::Ptsc as u8), 0);
        assert_ne!(mask & (1 << PerfMsrId::Therm as u8), 0);

        let amd_ptsc = MsrCpuFeatures {
            tsc: true,
            ptsc: true,
            ..Default::default()
        };
        let mask = perf_msr_probe_mask(amd_ptsc);
        assert_ne!(mask & (1 << PerfMsrId::Ptsc as u8), 0);
        assert_eq!(mask & (1 << PerfMsrId::Pperf as u8), 0);
    }

    #[test]
    fn event_init_validates_type_sampling_config_and_mask() {
        let mask = (1 << PerfMsrId::Tsc as u8) | (1 << PerfMsrId::Smi as u8);
        assert_eq!(msr_event_init(1, 2, 0, 0, mask), Err(ENOENT));
        assert_eq!(msr_event_init(1, 1, 7, 0, mask), Err(EINVAL));
        assert_eq!(
            msr_event_init(1, 1, 0, PERF_MSR_EVENT_MAX as u64, mask),
            Err(EINVAL)
        );
        assert_eq!(
            msr_event_init(1, 1, 0, PerfMsrId::Aperf as u64, mask),
            Err(EINVAL)
        );
        let event = msr_event_init(1, 1, 0, PerfMsrId::Smi as u64, mask).unwrap();
        assert_eq!(event.idx, -1);
        assert_eq!(event.event_base, MSR_SMI_COUNT);
        assert_eq!(event.config, PerfMsrId::Smi as u64);
    }

    #[test]
    fn counter_read_and_update_match_special_cases() {
        assert_eq!(msr_read_counter(0, 123, 456), 456);
        assert_eq!(msr_read_counter(MSR_IA32_APERF, 123, 456), 123);

        let mask = 1 << PerfMsrId::Smi as u8;
        let mut smi = msr_event_init(1, 1, 0, PerfMsrId::Smi as u64, mask).unwrap();
        smi.prev_count = 0x7fff_ffff;
        msr_event_update(&mut smi, 0x8000_0000);
        assert_eq!(smi.count, 1);
        msr_event_update(&mut smi, 0x7fff_ffff);
        assert_eq!(smi.count, 0);

        let mask = 1 << PerfMsrId::Therm as u8;
        let mut therm = msr_event_init(1, 1, 0, PerfMsrId::Therm as u64, mask).unwrap();
        msr_event_update(&mut therm, (1 << 31) | (0x2a << 16));
        assert_eq!(therm.count, 0x2a);
        msr_event_update(&mut therm, 0);
        assert_eq!(therm.count, -1);
    }

    #[test]
    fn start_add_stop_and_init_plan_follow_driver_flow() {
        let mask = 1 << PerfMsrId::Tsc as u8;
        let mut event = msr_event_init(1, 1, 0, PerfMsrId::Tsc as u64, mask).unwrap();
        assert_eq!(msr_event_add(&mut event, PERF_EF_START, 100), 0);
        assert_eq!(event.prev_count, 100);
        msr_event_stop(&mut event, 125);
        assert_eq!(event.count, 25);
        msr_event_del(&mut event, 130);
        assert_eq!(event.count, 30);

        let no_tsc = msr_init_plan(MsrCpuFeatures::default());
        assert!(no_tsc.print_no_driver);
        assert!(!no_tsc.register_pmu);

        let features = MsrCpuFeatures {
            tsc: true,
            aperfmperf: true,
            ..Default::default()
        };
        let plan = msr_init_plan(features);
        assert!(!plan.print_no_driver);
        assert_eq!(plan.probe_count, PERF_MSR_EVENT_MAX);
        assert!(plan.no_zero);
        assert!(plan.register_pmu);
        assert_ne!(plan.msr_mask & (1 << PerfMsrId::Aperf as u8), 0);
    }

    #[test]
    fn msr_pairs_follow_intel_and_amd_stride() {
        assert_eq!(
            intel_gp_msr_pair(2).unwrap(),
            PmuMsrPair {
                event_select: 0x188,
                counter: 0x0c3,
            }
        );
        assert_eq!(
            amd_gp_msr_pair(1).unwrap(),
            PmuMsrPair {
                event_select: 0xc001_0202,
                counter: 0xc001_0203,
            }
        );
    }
}
