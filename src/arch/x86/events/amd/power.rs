//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/amd/power.c
//! test-origin: linux:vendor/linux/arch/x86/events/amd/power.c
//! AMD power perf-event model.

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOENT};

pub const AMD_POWER_EVENT_MASK: u64 = 0xff;
pub const AMD_POWER_EVENTSEL_PKG: u64 = 1;

pub const MSR_F15H_CU_PWR_ACCUMULATOR: u64 = 0xc001_007a;
pub const MSR_F15H_CU_MAX_PWR_ACCUMULATOR: u64 = 0xc001_007b;
pub const MSR_F15H_PTSC: u64 = 0xc001_0280;

pub const PERF_HES_STOPPED: u64 = 0x01;
pub const PERF_HES_UPTODATE: u64 = 0x02;
pub const PERF_EF_START: i32 = 0x01;
pub const PERF_EF_UPDATE: i32 = 0x04;
pub const PERF_PMU_CAP_NO_EXCLUDE: u64 = 0x0040;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdPowerPmuDescriptor {
    pub name: &'static str,
    pub task_ctx_nr: &'static str,
    pub capabilities: u64,
    pub attr_groups: &'static [&'static str],
}

pub const AMD_POWER_ATTR_GROUPS: [&str; 3] = ["cpumask", "format", "events"];

pub const AMD_POWER_EVENTS: [(&str, &str); 3] = [
    ("power-pkg", "event=0x01"),
    ("power-pkg.unit", "mWatts"),
    ("power-pkg.scale", "1.000000e-3"),
];

pub const AMD_POWER_FORMAT: (&str, &str) = ("event", "config:0-7");

pub const AMD_POWER_PMU: AmdPowerPmuDescriptor = AmdPowerPmuDescriptor {
    name: "power",
    task_ctx_nr: "perf_invalid_context",
    capabilities: PERF_PMU_CAP_NO_EXCLUDE,
    attr_groups: &AMD_POWER_ATTR_GROUPS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdPowerEventAttr {
    pub typ: u32,
    pub config: u64,
    pub sample_period: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdPowerPerfEvent {
    pub pmu_type: u32,
    pub attr: AmdPowerEventAttr,
    pub state: u64,
    pub ptsc: u64,
    pub pwr_acc: u64,
    pub count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuHotplugResult {
    pub cpu_mask: u64,
    pub migrate: Option<(u8, u8)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdPowerInitInput {
    pub vendor_amd: bool,
    pub family: u8,
    pub acc_power: bool,
    pub cpuid_ecx_80000007: u32,
    pub max_cu_acc_power_msr: Result<u64, i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdPowerInitPlan {
    pub cpu_pwr_sample_ratio: u32,
    pub max_cu_acc_power: u64,
    pub setup_cpuhp_state: bool,
    pub register_pmu: bool,
}

pub const fn power_event_code() -> u64 {
    AMD_POWER_EVENTSEL_PKG
}

pub const fn amd_power_supported(vendor_amd: bool, family: u8, acc_power: bool) -> bool {
    vendor_amd && family == 0x15 && acc_power
}

pub const fn pmu_event_init(
    event_type: u32,
    pmu_type: u32,
    sample_period: u64,
    config: u64,
) -> Result<(), i32> {
    let cfg = config & AMD_POWER_EVENT_MASK;
    if event_type != pmu_type {
        return Err(ENOENT);
    }
    if sample_period != 0 {
        return Err(EINVAL);
    }
    if cfg != AMD_POWER_EVENTSEL_PKG {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn make_event(pmu_type: u32, config: u64) -> AmdPowerPerfEvent {
    AmdPowerPerfEvent {
        pmu_type,
        attr: AmdPowerEventAttr {
            typ: pmu_type,
            config,
            sample_period: 0,
        },
        state: 0,
        ptsc: 0,
        pwr_acc: 0,
        count: 0,
    }
}

pub fn event_update(
    event: &mut AmdPowerPerfEvent,
    new_pwr_acc: u64,
    new_ptsc: u64,
    cpu_pwr_sample_ratio: u32,
    max_cu_acc_power: u64,
) -> u64 {
    let prev_pwr_acc = event.pwr_acc;
    let prev_ptsc = event.ptsc;
    let mut delta = if new_pwr_acc < prev_pwr_acc {
        max_cu_acc_power + new_pwr_acc - prev_pwr_acc
    } else {
        new_pwr_acc - prev_pwr_acc
    };
    delta = delta.saturating_mul(cpu_pwr_sample_ratio as u64 * 1000);
    let tdelta = new_ptsc - prev_ptsc;
    let consumed = if tdelta == 0 { 0 } else { delta / tdelta };
    event.count = event.count.wrapping_add(consumed);
    consumed
}

pub fn pmu_event_start(event: &mut AmdPowerPerfEvent, ptsc: u64, pwr_acc: u64) {
    if (event.state & PERF_HES_STOPPED) == 0 {
        return;
    }
    event.state = 0;
    event.ptsc = ptsc;
    event.pwr_acc = pwr_acc;
}

pub fn pmu_event_stop(
    event: &mut AmdPowerPerfEvent,
    mode: i32,
    new_pwr_acc: u64,
    new_ptsc: u64,
    cpu_pwr_sample_ratio: u32,
    max_cu_acc_power: u64,
) {
    if (event.state & PERF_HES_STOPPED) == 0 {
        event.state |= PERF_HES_STOPPED;
    }
    if (mode & PERF_EF_UPDATE) != 0 && (event.state & PERF_HES_UPTODATE) == 0 {
        event_update(
            event,
            new_pwr_acc,
            new_ptsc,
            cpu_pwr_sample_ratio,
            max_cu_acc_power,
        );
        event.state |= PERF_HES_UPTODATE;
    }
}

pub fn pmu_event_add(event: &mut AmdPowerPerfEvent, mode: i32, ptsc: u64, pwr_acc: u64) -> i32 {
    event.state = PERF_HES_UPTODATE | PERF_HES_STOPPED;
    if (mode & PERF_EF_START) != 0 {
        pmu_event_start(event, ptsc, pwr_acc);
    }
    0
}

pub fn pmu_event_del(
    event: &mut AmdPowerPerfEvent,
    new_pwr_acc: u64,
    new_ptsc: u64,
    cpu_pwr_sample_ratio: u32,
    max_cu_acc_power: u64,
) {
    pmu_event_stop(
        event,
        PERF_EF_UPDATE,
        new_pwr_acc,
        new_ptsc,
        cpu_pwr_sample_ratio,
        max_cu_acc_power,
    );
}

pub const fn cpu_is_set(mask: u64, cpu: u8) -> bool {
    cpu < 64 && (mask & (1u64 << cpu)) != 0
}

pub const fn set_cpu(mask: u64, cpu: u8) -> u64 {
    if cpu < 64 { mask | (1u64 << cpu) } else { mask }
}

pub const fn clear_cpu(mask: u64, cpu: u8) -> u64 {
    if cpu < 64 {
        mask & !(1u64 << cpu)
    } else {
        mask
    }
}

pub const fn any_but(mask: u64, cpu: u8, nr_cpumask_bits: u8) -> Option<u8> {
    let mut candidate = 0u8;
    while candidate < nr_cpumask_bits {
        if candidate != cpu && cpu_is_set(mask, candidate) {
            return Some(candidate);
        }
        candidate += 1;
    }
    None
}

pub const fn power_cpu_init(cpu: u8, sibling_mask: u64, cpu_mask: u64, nr_cpumask_bits: u8) -> u64 {
    match any_but(sibling_mask, cpu, nr_cpumask_bits) {
        Some(_) => cpu_mask,
        None => set_cpu(cpu_mask, cpu),
    }
}

pub const fn power_cpu_exit(
    cpu: u8,
    sibling_mask: u64,
    cpu_mask: u64,
    nr_cpumask_bits: u8,
) -> CpuHotplugResult {
    if !cpu_is_set(cpu_mask, cpu) {
        return CpuHotplugResult {
            cpu_mask,
            migrate: None,
        };
    }
    let mut mask = clear_cpu(cpu_mask, cpu);
    match any_but(sibling_mask, cpu, nr_cpumask_bits) {
        Some(target) => {
            mask = set_cpu(mask, target);
            CpuHotplugResult {
                cpu_mask: mask,
                migrate: Some((cpu, target)),
            }
        }
        None => CpuHotplugResult {
            cpu_mask: mask,
            migrate: None,
        },
    }
}

pub const fn amd_power_pmu_init(input: AmdPowerInitInput) -> Result<AmdPowerInitPlan, i32> {
    if !amd_power_supported(input.vendor_amd, input.family, input.acc_power) {
        return Err(ENODEV);
    }
    match input.max_cu_acc_power_msr {
        Ok(max_cu_acc_power) => Ok(AmdPowerInitPlan {
            cpu_pwr_sample_ratio: input.cpuid_ecx_80000007,
            max_cu_acc_power,
            setup_cpuhp_state: true,
            register_pmu: true,
        }),
        Err(_) => Err(ENODEV),
    }
}

pub const fn amd_power_pmu_exit_plan() -> (&'static str, &'static str) {
    (
        "cpuhp_remove_state_nocalls(CPUHP_AP_PERF_X86_AMD_POWER_ONLINE)",
        "perf_pmu_unregister(&pmu_class)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amd_power_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/amd/power.c"
        ));
        assert!(source.contains("#define AMD_POWER_EVENT_MASK\t\t0xFFULL"));
        assert!(source.contains("#define AMD_POWER_EVENTSEL_PKG\t\t1"));
        assert!(source.contains("rdmsrq(MSR_F15H_CU_PWR_ACCUMULATOR, new_pwr_acc);"));
        assert!(source.contains("delta *= cpu_pwr_sample_ratio * 1000;"));
        assert!(source.contains("do_div(delta, tdelta);"));
        assert!(source.contains("hwc->state = PERF_HES_UPTODATE | PERF_HES_STOPPED;"));
        assert!(source.contains("cfg = event->attr.config & AMD_POWER_EVENT_MASK;"));
        assert!(source.contains("EVENT_ATTR_STR(power-pkg, power_pkg, \"event=0x01\");"));
        assert!(source.contains("EVENT_ATTR_STR(power-pkg.unit, power_pkg_unit, \"mWatts\");"));
        assert!(source.contains("PMU_FORMAT_ATTR(event, \"config:0-7\");"));
        assert!(source.contains("X86_MATCH_VENDOR_FAM(AMD, 0x15, NULL)"));
        assert!(source.contains("boot_cpu_has(X86_FEATURE_ACC_POWER)"));
        assert!(source.contains("cpuid_ecx(0x80000007)"));
        assert!(source.contains("MSR_F15H_CU_MAX_PWR_ACCUMULATOR"));
        assert!(source.contains("cpuhp_setup_state(CPUHP_AP_PERF_X86_AMD_POWER_ONLINE"));
        assert!(source.contains("perf_pmu_register(&pmu_class, \"power\", -1);"));
    }

    #[test]
    fn power_event_and_pmu_metadata_match_linux() {
        assert_eq!(power_event_code(), AMD_POWER_EVENTSEL_PKG);
        assert!(amd_power_supported(true, 0x15, true));
        assert!(!amd_power_supported(true, 0x17, true));
        assert_eq!(AMD_POWER_EVENTS[0], ("power-pkg", "event=0x01"));
        assert_eq!(AMD_POWER_EVENTS[1], ("power-pkg.unit", "mWatts"));
        assert_eq!(AMD_POWER_EVENTS[2], ("power-pkg.scale", "1.000000e-3"));
        assert_eq!(AMD_POWER_FORMAT, ("event", "config:0-7"));
        assert_eq!(AMD_POWER_PMU.name, "power");
        assert_eq!(AMD_POWER_PMU.task_ctx_nr, "perf_invalid_context");
        assert_eq!(AMD_POWER_PMU.capabilities, PERF_PMU_CAP_NO_EXCLUDE);
    }

    #[test]
    fn event_init_masks_config_and_rejects_unsupported_modes() {
        assert_eq!(pmu_event_init(1, 2, 0, AMD_POWER_EVENTSEL_PKG), Err(ENOENT));
        assert_eq!(
            pmu_event_init(1, 1, 10, AMD_POWER_EVENTSEL_PKG),
            Err(EINVAL)
        );
        assert_eq!(pmu_event_init(1, 1, 0, 0), Err(EINVAL));
        assert_eq!(
            pmu_event_init(1, 1, 0, AMD_POWER_EVENTSEL_PKG | 0x100),
            Ok(())
        );
    }

    #[test]
    fn update_computes_micro_watts_and_handles_accumulator_wrap() {
        let mut event = make_event(1, AMD_POWER_EVENTSEL_PKG);
        event.pwr_acc = 100;
        event.ptsc = 10;
        let delta = event_update(&mut event, 160, 40, 2, 1000);
        assert_eq!(delta, 4000);
        assert_eq!(event.count, 4000);

        event.pwr_acc = 950;
        event.ptsc = 40;
        let delta = event_update(&mut event, 10, 70, 3, 1000);
        assert_eq!(delta, 6000);
        assert_eq!(event.count, 10000);
    }

    #[test]
    fn add_start_stop_and_delete_follow_hwc_state_rules() {
        let mut event = make_event(1, AMD_POWER_EVENTSEL_PKG);
        assert_eq!(pmu_event_add(&mut event, 0, 10, 100), 0);
        assert_eq!(event.state, PERF_HES_UPTODATE | PERF_HES_STOPPED);

        assert_eq!(pmu_event_add(&mut event, PERF_EF_START, 20, 200), 0);
        assert_eq!(event.state, 0);
        assert_eq!(event.ptsc, 20);
        assert_eq!(event.pwr_acc, 200);

        pmu_event_stop(&mut event, PERF_EF_UPDATE, 260, 50, 1, 1000);
        assert_eq!(event.state, PERF_HES_STOPPED | PERF_HES_UPTODATE);
        assert_eq!(event.count, 2000);

        event.state = 0;
        event.pwr_acc = 260;
        event.ptsc = 50;
        pmu_event_del(&mut event, 290, 80, 1, 1000);
        assert_eq!(event.state, PERF_HES_STOPPED | PERF_HES_UPTODATE);
        assert_eq!(event.count, 3000);
    }

    #[test]
    fn cpu_hotplug_masks_pick_one_cpu_per_compute_unit() {
        assert_eq!(power_cpu_init(0, 0b0001, 0, 4), 0b0001);
        assert_eq!(power_cpu_init(1, 0b0011, 0b0001, 4), 0b0001);
        assert_eq!(
            power_cpu_exit(0, 0b0011, 0b0001, 4),
            CpuHotplugResult {
                cpu_mask: 0b0010,
                migrate: Some((0, 1)),
            }
        );
        assert_eq!(
            power_cpu_exit(2, 0b1100, 0b0001, 4),
            CpuHotplugResult {
                cpu_mask: 0b0001,
                migrate: None,
            }
        );
    }

    #[test]
    fn init_and_exit_plans_match_module_lifecycle() {
        assert_eq!(
            amd_power_pmu_init(AmdPowerInitInput {
                vendor_amd: false,
                family: 0x15,
                acc_power: true,
                cpuid_ecx_80000007: 7,
                max_cu_acc_power_msr: Ok(100),
            }),
            Err(ENODEV)
        );
        let plan = amd_power_pmu_init(AmdPowerInitInput {
            vendor_amd: true,
            family: 0x15,
            acc_power: true,
            cpuid_ecx_80000007: 7,
            max_cu_acc_power_msr: Ok(100),
        })
        .unwrap();
        assert_eq!(
            plan,
            AmdPowerInitPlan {
                cpu_pwr_sample_ratio: 7,
                max_cu_acc_power: 100,
                setup_cpuhp_state: true,
                register_pmu: true,
            }
        );
        assert_eq!(
            amd_power_pmu_exit_plan(),
            (
                "cpuhp_remove_state_nocalls(CPUHP_AP_PERF_X86_AMD_POWER_ONLINE)",
                "perf_pmu_unregister(&pmu_class)"
            )
        );
    }
}
