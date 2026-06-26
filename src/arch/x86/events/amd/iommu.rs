//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/amd/iommu.c
//! test-origin: linux:vendor/linux/arch/x86/events/amd/iommu.c
//! AMD IOMMU performance-counter PMU model.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOENT, ENOMEM, ENOSPC};

pub const IOMMU_NAME_SIZE: usize = 24;

pub const IOMMU_PC_COUNTER_REG: u64 = 0x00;
pub const IOMMU_PC_COUNTER_SRC_REG: u64 = 0x08;
pub const IOMMU_PC_PASID_MATCH_REG: u64 = 0x10;
pub const IOMMU_PC_DOMID_MATCH_REG: u64 = 0x18;
pub const IOMMU_PC_DEVID_MATCH_REG: u64 = 0x20;
pub const IOMMU_PC_COUNTER_REPORT_REG: u64 = 0x28;
pub const PC_MAX_SPEC_BNKS: u8 = 64;
pub const PC_MAX_SPEC_CNTRS: u8 = 16;

pub const PERF_HES_STOPPED: u64 = 0x01;
pub const PERF_HES_UPTODATE: u64 = 0x02;
pub const PERF_EF_START: i32 = 0x01;
pub const PERF_EF_RELOAD: i32 = 0x02;
pub const PERF_EF_UPDATE: i32 = 0x04;
pub const PERF_ATTACH_TASK: u64 = 0x0004;
pub const PERF_PMU_CAP_NO_EXCLUDE: u64 = 0x0040;
pub const IOMMU_COUNTER_MASK: u64 = (1u64 << 48) - 1;

pub const IOMMU_FORMAT_ATTRS: [(&str, &str); 7] = [
    ("csource", "config:0-7"),
    ("devid", "config:8-23"),
    ("domid", "config:24-39"),
    ("pasid", "config:40-59"),
    ("devid_mask", "config1:0-15"),
    ("domid_mask", "config1:16-31"),
    ("pasid_mask", "config1:32-51"),
];

pub const AMD_IOMMU_V2_EVENT_DESCS: [(&str, &str); 24] = [
    ("mem_pass_untrans", "csource=0x01"),
    ("mem_pass_pretrans", "csource=0x02"),
    ("mem_pass_excl", "csource=0x03"),
    ("mem_target_abort", "csource=0x04"),
    ("mem_trans_total", "csource=0x05"),
    ("mem_iommu_tlb_pte_hit", "csource=0x06"),
    ("mem_iommu_tlb_pte_mis", "csource=0x07"),
    ("mem_iommu_tlb_pde_hit", "csource=0x08"),
    ("mem_iommu_tlb_pde_mis", "csource=0x09"),
    ("mem_dte_hit", "csource=0x0a"),
    ("mem_dte_mis", "csource=0x0b"),
    ("page_tbl_read_tot", "csource=0x0c"),
    ("page_tbl_read_nst", "csource=0x0d"),
    ("page_tbl_read_gst", "csource=0x0e"),
    ("int_dte_hit", "csource=0x0f"),
    ("int_dte_mis", "csource=0x10"),
    ("cmd_processed", "csource=0x11"),
    ("cmd_processed_inv", "csource=0x12"),
    ("tlb_inv", "csource=0x13"),
    ("ign_rd_wr_mmio_1ff8h", "csource=0x14"),
    ("vapic_int_non_guest", "csource=0x15"),
    ("vapic_int_guest", "csource=0x16"),
    ("smi_recv", "csource=0x17"),
    ("smi_blk", "csource=0x18"),
];

pub const AMD_IOMMU_ATTR_GROUPS: [&str; 3] = ["format", "cpumask", "events"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdIommuPmu {
    pub banks: u8,
    pub counters_per_bank: u8,
    pub counter_bits: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdIommuPmuDescriptor {
    pub task_ctx_nr: &'static str,
    pub capabilities: u64,
    pub attr_groups: &'static [&'static str],
}

pub const IOMMU_PMU: AmdIommuPmuDescriptor = AmdIommuPmuDescriptor {
    task_ctx_nr: "perf_invalid_context",
    capabilities: PERF_PMU_CAP_NO_EXCLUDE,
    attr_groups: &AMD_IOMMU_ATTR_GROUPS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdIommuHw {
    pub conf: u64,
    pub conf1: u64,
    pub iommu_bank: u8,
    pub iommu_cntr: u8,
    pub state: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdIommuEvent {
    pub attr_type: u32,
    pub pmu_type: u32,
    pub config: u64,
    pub config1: u64,
    pub attach_state: u64,
    pub sampling: bool,
    pub cpu: i32,
    pub hw: AmdIommuHw,
    pub count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PerfAmdIommu {
    pub max_banks: u8,
    pub max_counters: u8,
    pub cntr_assign_mask: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IommuPcWrite {
    pub bank: u8,
    pub counter: u8,
    pub reg: u64,
    pub value: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IommuStartPlan {
    pub writes: Vec<IommuPcWrite>,
    pub warn_not_stopped: bool,
    pub warn_not_uptodate: bool,
    pub update_userpage: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IommuStopPlan {
    pub read_added: u64,
    pub writes: Vec<IommuPcWrite>,
    pub warn_already_stopped: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IommuAddPlan {
    pub started: Option<IommuStartPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IommuDelPlan {
    pub stop: IommuStopPlan,
    pub clear_result: Result<(), i32>,
    pub update_userpage: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitOneIommuPlan {
    pub idx: u32,
    pub name: [u8; IOMMU_NAME_SIZE],
    pub max_banks: u8,
    pub max_counters: u8,
    pub register: bool,
    pub list_add_tail: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdIommuDiscovery {
    pub exists: bool,
    pub max_banks: u8,
    pub max_counters: u8,
    pub register_result: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmdIommuPcInitPlan {
    pub initialized: Vec<InitOneIommuPlan>,
    pub free_events_attrs: bool,
    pub cpumask_cpu: Option<u8>,
}

pub const fn get_csource(conf: u64) -> u64 {
    conf & 0xff
}

pub const fn get_devid(conf: u64) -> u64 {
    (conf >> 8) & 0xffff
}

pub const fn get_domid(conf: u64) -> u64 {
    (conf >> 24) & 0xffff
}

pub const fn get_pasid(conf: u64) -> u64 {
    (conf >> 40) & 0xfffff
}

pub const fn get_devid_mask(conf1: u64) -> u64 {
    conf1 & 0xffff
}

pub const fn get_domid_mask(conf1: u64) -> u64 {
    (conf1 >> 16) & 0xffff
}

pub const fn get_pasid_mask(conf1: u64) -> u64 {
    (conf1 >> 32) & 0xfffff
}

pub const fn match_reg_value(value: u64, mask: u64) -> u64 {
    let reg = value | (mask << 32);
    if reg == 0 { 0 } else { reg | (1 << 31) }
}

pub const fn iommu_counter_shift(bank: u8, counter: u8) -> u8 {
    bank + counter + bank * 3
}

pub const fn iommu_pmu_from_capability(cap: u32) -> Option<AmdIommuPmu> {
    let banks = (cap & 0xff) as u8;
    let counters = ((cap >> 8) & 0xff) as u8;
    if banks == 0 || counters == 0 {
        None
    } else {
        Some(AmdIommuPmu {
            banks,
            counters_per_bank: counters,
            counter_bits: 48,
        })
    }
}

pub fn get_next_avail_iommu_bnk_cntr(perf_iommu: &mut PerfAmdIommu) -> Result<(u8, u8), i32> {
    let mut bank = 0u8;
    while bank < perf_iommu.max_banks {
        let mut counter = 0u8;
        while counter < perf_iommu.max_counters {
            let shift = iommu_counter_shift(bank, counter);
            let bit = 1u64 << shift;
            if (perf_iommu.cntr_assign_mask & bit) == 0 {
                perf_iommu.cntr_assign_mask |= bit;
                return Ok((bank, counter));
            }
            counter += 1;
        }
        bank += 1;
    }
    Err(ENOSPC)
}

pub fn clear_avail_iommu_bnk_cntr(
    perf_iommu: &mut PerfAmdIommu,
    bank: u8,
    counter: u8,
) -> Result<(), i32> {
    if bank > perf_iommu.max_banks || counter > perf_iommu.max_counters {
        return Err(EINVAL);
    }
    let shift = iommu_counter_shift(bank, counter);
    perf_iommu.cntr_assign_mask &= !(1u64 << shift);
    Ok(())
}

pub fn perf_iommu_event_init(event: &mut AmdIommuEvent) -> Result<(), i32> {
    if event.attr_type != event.pmu_type {
        return Err(ENOENT);
    }
    if event.sampling || (event.attach_state & PERF_ATTACH_TASK) != 0 {
        return Err(EINVAL);
    }
    if event.cpu < 0 {
        return Err(EINVAL);
    }
    event.hw.conf = event.config;
    event.hw.conf1 = event.config1;
    Ok(())
}

pub fn perf_iommu_enable_event_plan(event: &AmdIommuEvent) -> Vec<IommuPcWrite> {
    let bank = event.hw.iommu_bank;
    let counter = event.hw.iommu_cntr;
    let conf = event.hw.conf;
    let conf1 = event.hw.conf1;

    Vec::from([
        IommuPcWrite {
            bank,
            counter,
            reg: IOMMU_PC_COUNTER_SRC_REG,
            value: get_csource(conf),
        },
        IommuPcWrite {
            bank,
            counter,
            reg: IOMMU_PC_DEVID_MATCH_REG,
            value: match_reg_value(get_devid(conf), get_devid_mask(conf1)),
        },
        IommuPcWrite {
            bank,
            counter,
            reg: IOMMU_PC_PASID_MATCH_REG,
            value: match_reg_value(get_pasid(conf), get_pasid_mask(conf1)),
        },
        IommuPcWrite {
            bank,
            counter,
            reg: IOMMU_PC_DOMID_MATCH_REG,
            value: match_reg_value(get_domid(conf), get_domid_mask(conf1)),
        },
    ])
}

pub fn perf_iommu_disable_event_plan(event: &AmdIommuEvent) -> Vec<IommuPcWrite> {
    Vec::from([IommuPcWrite {
        bank: event.hw.iommu_bank,
        counter: event.hw.iommu_cntr,
        reg: IOMMU_PC_COUNTER_SRC_REG,
        value: 0,
    }])
}

pub fn perf_iommu_start(event: &mut AmdIommuEvent, flags: i32) -> IommuStartPlan {
    let warn_not_stopped = (event.hw.state & PERF_HES_STOPPED) == 0;
    if warn_not_stopped {
        return IommuStartPlan {
            writes: Vec::new(),
            warn_not_stopped,
            warn_not_uptodate: false,
            update_userpage: false,
        };
    }
    let warn_not_uptodate = (event.hw.state & PERF_HES_UPTODATE) == 0;
    event.hw.state = 0;

    let mut writes = perf_iommu_enable_event_plan(event);
    if (flags & PERF_EF_RELOAD) != 0 {
        writes.push(IommuPcWrite {
            bank: event.hw.iommu_bank,
            counter: event.hw.iommu_cntr,
            reg: IOMMU_PC_COUNTER_REG,
            value: 0,
        });
    }

    IommuStartPlan {
        writes,
        warn_not_stopped,
        warn_not_uptodate,
        update_userpage: true,
    }
}

pub fn perf_iommu_read(event: &mut AmdIommuEvent, counter_read: Result<u64, i32>) -> u64 {
    let Ok(count) = counter_read else {
        return 0;
    };
    let count = count & IOMMU_COUNTER_MASK;
    event.count = event.count.wrapping_add(count);
    count
}

pub fn perf_iommu_stop(
    event: &mut AmdIommuEvent,
    _flags: i32,
    counter_read: Result<u64, i32>,
) -> IommuStopPlan {
    if (event.hw.state & PERF_HES_UPTODATE) != 0 {
        return IommuStopPlan {
            read_added: 0,
            writes: Vec::new(),
            warn_already_stopped: false,
        };
    }

    let read_added = perf_iommu_read(event, counter_read);
    event.hw.state |= PERF_HES_UPTODATE;
    let writes = perf_iommu_disable_event_plan(event);
    let warn_already_stopped = (event.hw.state & PERF_HES_STOPPED) != 0;
    event.hw.state |= PERF_HES_STOPPED;

    IommuStopPlan {
        read_added,
        writes,
        warn_already_stopped,
    }
}

pub fn perf_iommu_add(
    perf_iommu: &mut PerfAmdIommu,
    event: &mut AmdIommuEvent,
    flags: i32,
) -> Result<IommuAddPlan, i32> {
    event.hw.state = PERF_HES_UPTODATE | PERF_HES_STOPPED;
    let (bank, counter) = get_next_avail_iommu_bnk_cntr(perf_iommu)?;
    event.hw.iommu_bank = bank;
    event.hw.iommu_cntr = counter;

    let started = if (flags & PERF_EF_START) != 0 {
        Some(perf_iommu_start(event, PERF_EF_RELOAD))
    } else {
        None
    };
    Ok(IommuAddPlan { started })
}

pub fn perf_iommu_del(
    perf_iommu: &mut PerfAmdIommu,
    event: &mut AmdIommuEvent,
    counter_read: Result<u64, i32>,
) -> IommuDelPlan {
    let stop = perf_iommu_stop(event, PERF_EF_UPDATE, counter_read);
    let clear_result =
        clear_avail_iommu_bnk_cntr(perf_iommu, event.hw.iommu_bank, event.hw.iommu_cntr);
    IommuDelPlan {
        stop,
        clear_result,
        update_userpage: true,
    }
}

pub fn init_events_attrs(alloc_ok: bool) -> Result<Vec<&'static str>, i32> {
    if !alloc_ok {
        return Err(ENOMEM);
    }
    Ok(AMD_IOMMU_V2_EVENT_DESCS
        .iter()
        .map(|(name, _)| *name)
        .collect())
}

pub fn iommu_name(idx: u32) -> [u8; IOMMU_NAME_SIZE] {
    let mut name = [0u8; IOMMU_NAME_SIZE];
    let prefix = b"amd_iommu_";
    let mut pos = 0usize;
    while pos < prefix.len() {
        name[pos] = prefix[pos];
        pos += 1;
    }

    let mut digits = [0u8; 10];
    let mut value = idx;
    let mut len = 0usize;
    loop {
        digits[len] = b'0' + (value % 10) as u8;
        len += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while len > 0 && pos < IOMMU_NAME_SIZE - 1 {
        len -= 1;
        name[pos] = digits[len];
        pos += 1;
    }
    name
}

pub fn init_one_iommu(
    idx: u32,
    alloc_ok: bool,
    discovery: AmdIommuDiscovery,
) -> Result<InitOneIommuPlan, i32> {
    if !alloc_ok {
        return Err(ENOMEM);
    }
    if !discovery.exists || discovery.max_banks == 0 || discovery.max_counters == 0 {
        return Err(EINVAL);
    }
    if discovery.register_result != 0 {
        return Err(discovery.register_result);
    }

    Ok(InitOneIommuPlan {
        idx,
        name: iommu_name(idx),
        max_banks: discovery.max_banks,
        max_counters: discovery.max_counters,
        register: true,
        list_add_tail: true,
    })
}

pub fn amd_iommu_pc_init(
    pc_supported: bool,
    events_attrs_result: Result<Vec<&'static str>, i32>,
    discoveries: &[AmdIommuDiscovery],
) -> Result<AmdIommuPcInitPlan, i32> {
    if !pc_supported {
        return Err(ENODEV);
    }
    let _attrs = events_attrs_result?;

    let mut initialized = Vec::new();
    for (idx, discovery) in discoveries.iter().copied().enumerate() {
        if let Ok(plan) = init_one_iommu(idx as u32, true, discovery) {
            initialized.push(plan);
        }
    }

    if initialized.is_empty() {
        return Ok(AmdIommuPcInitPlan {
            initialized,
            free_events_attrs: true,
            cpumask_cpu: None,
        })
        .and(Err(ENODEV));
    }

    Ok(AmdIommuPcInitPlan {
        initialized,
        free_events_attrs: false,
        cpumask_cpu: Some(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> AmdIommuEvent {
        AmdIommuEvent {
            attr_type: 7,
            pmu_type: 7,
            config: 0,
            config1: 0,
            attach_state: 0,
            sampling: false,
            cpu: 0,
            hw: AmdIommuHw {
                conf: 0,
                conf1: 0,
                iommu_bank: 0,
                iommu_cntr: 0,
                state: 0,
            },
            count: 0,
        }
    }

    #[test]
    fn iommu_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/amd/iommu.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/amd/iommu.h"
        ));
        let perf_event = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/perf_event.h"
        ));

        assert!(source.contains("#define GET_CSOURCE(x)     ((x)->conf & 0xFFULL)"));
        assert!(source.contains("#define GET_DEVID(x)       (((x)->conf >> 8)  & 0xFFFFULL)"));
        assert!(source.contains("#define GET_DOMID(x)       (((x)->conf >> 24) & 0xFFFFULL)"));
        assert!(source.contains("#define GET_PASID(x)       (((x)->conf >> 40) & 0xFFFFFULL)"));
        assert!(source.contains("PMU_FORMAT_ATTR(csource,    \"config:0-7\");"));
        assert!(source.contains("AMD_IOMMU_EVENT_DESC(mem_pass_untrans,        \"csource=0x01\")"));
        assert!(source.contains("AMD_IOMMU_EVENT_DESC(smi_blk,                 \"csource=0x18\")"));
        assert!(source.contains("shift = bank + (bank*3) + cntr;"));
        assert!(source.contains("retval = -ENOSPC;"));
        assert!(source.contains("if ((bank > max_banks) || (cntr > max_cntrs))"));
        assert!(source.contains("if (event->attr.type != event->pmu->type)"));
        assert!(
            source.contains("is_sampling_event(event) || event->attach_state & PERF_ATTACH_TASK")
        );
        assert!(source.contains("hwc->conf  = event->attr.config;"));
        assert!(source.contains("hwc->conf1 = event->attr.config1;"));
        assert!(
            source.contains(
                "amd_iommu_pc_set_reg(iommu, bank, cntr, IOMMU_PC_COUNTER_SRC_REG, &reg);"
            )
        );
        assert!(source.contains("reg = GET_DEVID(hwc) | (reg << 32);"));
        assert!(source.contains("reg |= BIT(31);"));
        assert!(source.contains("count &= GENMASK_ULL(47, 0);"));
        assert!(source.contains("event->hw.state = PERF_HES_UPTODATE | PERF_HES_STOPPED;"));
        assert!(source.contains("perf_iommu_start(event, PERF_EF_RELOAD);"));
        assert!(source.contains("perf_iommu_stop(event, PERF_EF_UPDATE);"));
        assert!(
            source.contains("snprintf(perf_iommu->name, IOMMU_NAME_SIZE, \"amd_iommu_%u\", idx);")
        );
        assert!(source.contains("cpumask_set_cpu(0, &iommu_cpumask);"));
        assert!(header.contains("#define IOMMU_PC_COUNTER_SRC_REG\t\t0x08"));
        assert!(header.contains("#define PC_MAX_SPEC_BNKS\t\t\t64"));
        assert!(perf_event.contains("#define PERF_ATTACH_TASK\t\t0x0004"));
        assert!(perf_event.contains("#define PERF_EF_RELOAD\t\t\t0x02"));
    }

    #[test]
    fn masks_formats_events_and_descriptor_match_source() {
        let conf = 0xabcde_u64 << 40 | 0x1234_u64 << 24 | 0x5678_u64 << 8 | 0x9a;
        let conf1 = 0xfedcb_u64 << 32 | 0x1357_u64 << 16 | 0x2468;
        assert_eq!(get_csource(conf), 0x9a);
        assert_eq!(get_devid(conf), 0x5678);
        assert_eq!(get_domid(conf), 0x1234);
        assert_eq!(get_pasid(conf), 0xabcde);
        assert_eq!(get_devid_mask(conf1), 0x2468);
        assert_eq!(get_domid_mask(conf1), 0x1357);
        assert_eq!(get_pasid_mask(conf1), 0xfedcb);
        assert_eq!(IOMMU_FORMAT_ATTRS[0], ("csource", "config:0-7"));
        assert_eq!(IOMMU_FORMAT_ATTRS[6], ("pasid_mask", "config1:32-51"));
        assert_eq!(
            AMD_IOMMU_V2_EVENT_DESCS[0],
            ("mem_pass_untrans", "csource=0x01")
        );
        assert_eq!(AMD_IOMMU_V2_EVENT_DESCS[23], ("smi_blk", "csource=0x18"));
        assert_eq!(IOMMU_PMU.task_ctx_nr, "perf_invalid_context");
        assert_eq!(IOMMU_PMU.capabilities, PERF_PMU_CAP_NO_EXCLUDE);
    }

    #[test]
    fn allocation_and_clear_use_linux_shift_formula_and_bounds() {
        let mut pmu = PerfAmdIommu {
            max_banks: 2,
            max_counters: 2,
            cntr_assign_mask: 0,
        };
        assert_eq!(get_next_avail_iommu_bnk_cntr(&mut pmu), Ok((0, 0)));
        assert_eq!(pmu.cntr_assign_mask, 1);
        assert_eq!(get_next_avail_iommu_bnk_cntr(&mut pmu), Ok((0, 1)));
        assert_eq!(get_next_avail_iommu_bnk_cntr(&mut pmu), Ok((1, 0)));
        assert_ne!(pmu.cntr_assign_mask & (1 << iommu_counter_shift(1, 0)), 0);
        assert_eq!(get_next_avail_iommu_bnk_cntr(&mut pmu), Ok((1, 1)));
        assert_eq!(get_next_avail_iommu_bnk_cntr(&mut pmu), Err(ENOSPC));
        assert_eq!(clear_avail_iommu_bnk_cntr(&mut pmu, 1, 1), Ok(()));
        assert_eq!(clear_avail_iommu_bnk_cntr(&mut pmu, 3, 0), Err(EINVAL));
    }

    #[test]
    fn event_init_rejects_wrong_scope_and_copies_config() {
        let mut ev = event();
        ev.attr_type = 1;
        assert_eq!(perf_iommu_event_init(&mut ev), Err(ENOENT));
        ev = event();
        ev.sampling = true;
        assert_eq!(perf_iommu_event_init(&mut ev), Err(EINVAL));
        ev = event();
        ev.attach_state = PERF_ATTACH_TASK;
        assert_eq!(perf_iommu_event_init(&mut ev), Err(EINVAL));
        ev = event();
        ev.cpu = -1;
        assert_eq!(perf_iommu_event_init(&mut ev), Err(EINVAL));
        ev = event();
        ev.config = 0x123;
        ev.config1 = 0x456;
        assert_eq!(perf_iommu_event_init(&mut ev), Ok(()));
        assert_eq!(ev.hw.conf, 0x123);
        assert_eq!(ev.hw.conf1, 0x456);
    }

    #[test]
    fn enable_disable_start_read_stop_program_expected_registers() {
        let mut ev = event();
        ev.hw.conf = (0x12 << 40) | (0x34 << 24) | (0x56 << 8) | 0x78;
        ev.hw.conf1 = (0x9a << 32) | (0xbc << 16) | 0xde;
        ev.hw.iommu_bank = 1;
        ev.hw.iommu_cntr = 2;
        let writes = perf_iommu_enable_event_plan(&ev);
        assert_eq!(writes[0].reg, IOMMU_PC_COUNTER_SRC_REG);
        assert_eq!(writes[0].value, 0x78);
        assert_eq!(writes[1].reg, IOMMU_PC_DEVID_MATCH_REG);
        assert_eq!(writes[1].value, 0x8000_0000 | (0xde << 32) | 0x56);
        assert_eq!(writes[2].reg, IOMMU_PC_PASID_MATCH_REG);
        assert_eq!(writes[2].value, 0x8000_0000 | (0x9a << 32) | 0x12);
        assert_eq!(writes[3].reg, IOMMU_PC_DOMID_MATCH_REG);
        assert_eq!(writes[3].value, 0x8000_0000 | (0xbc << 32) | 0x34);
        assert_eq!(perf_iommu_disable_event_plan(&ev)[0].value, 0);

        ev.hw.state = PERF_HES_STOPPED | PERF_HES_UPTODATE;
        let start = perf_iommu_start(&mut ev, PERF_EF_RELOAD);
        assert!(!start.warn_not_stopped);
        assert!(!start.warn_not_uptodate);
        assert!(start.update_userpage);
        assert_eq!(start.writes.last().unwrap().reg, IOMMU_PC_COUNTER_REG);
        assert_eq!(ev.hw.state, 0);

        assert_eq!(perf_iommu_read(&mut ev, Ok((1 << 52) | 7)), 7);
        assert_eq!(ev.count, 7);
        let stop = perf_iommu_stop(&mut ev, PERF_EF_UPDATE, Ok(5));
        assert_eq!(stop.read_added, 5);
        assert_eq!(ev.count, 12);
        assert_eq!(stop.writes[0].reg, IOMMU_PC_COUNTER_SRC_REG);
        assert_eq!(ev.hw.state, PERF_HES_STOPPED | PERF_HES_UPTODATE);
    }

    #[test]
    fn add_and_del_allocate_start_stop_and_clear_counter() {
        let mut pmu = PerfAmdIommu {
            max_banks: 1,
            max_counters: 1,
            cntr_assign_mask: 0,
        };
        let mut ev = event();
        let add = perf_iommu_add(&mut pmu, &mut ev, PERF_EF_START).unwrap();
        assert!(add.started.is_some());
        assert_eq!(ev.hw.iommu_bank, 0);
        assert_eq!(ev.hw.iommu_cntr, 0);
        assert_ne!(pmu.cntr_assign_mask, 0);
        let del = perf_iommu_del(&mut pmu, &mut ev, Ok(9));
        assert_eq!(del.stop.read_added, 9);
        assert_eq!(del.clear_result, Ok(()));
        assert_eq!(pmu.cntr_assign_mask, 0);
        assert!(del.update_userpage);
    }

    #[test]
    fn init_event_attrs_and_iommu_registration_follow_linux_lifecycle() {
        assert_eq!(init_events_attrs(false), Err(ENOMEM));
        let attrs = init_events_attrs(true).unwrap();
        assert_eq!(attrs.len(), AMD_IOMMU_V2_EVENT_DESCS.len());
        assert_eq!(attrs[0], "mem_pass_untrans");

        let name = iommu_name(12);
        assert_eq!(&name[..12], b"amd_iommu_12");
        assert_eq!(
            init_one_iommu(
                0,
                true,
                AmdIommuDiscovery {
                    exists: false,
                    max_banks: 1,
                    max_counters: 1,
                    register_result: 0,
                },
            ),
            Err(EINVAL)
        );
        let one = init_one_iommu(
            1,
            true,
            AmdIommuDiscovery {
                exists: true,
                max_banks: 2,
                max_counters: 4,
                register_result: 0,
            },
        )
        .unwrap();
        assert_eq!(one.idx, 1);
        assert!(one.register);
        assert!(one.list_add_tail);

        assert_eq!(
            amd_iommu_pc_init(false, Ok(attrs.clone()), &[]),
            Err(ENODEV)
        );
        assert_eq!(
            amd_iommu_pc_init(
                true,
                Ok(attrs.clone()),
                &[AmdIommuDiscovery {
                    exists: false,
                    max_banks: 0,
                    max_counters: 0,
                    register_result: 0,
                }]
            ),
            Err(ENODEV)
        );
        let init = amd_iommu_pc_init(
            true,
            Ok(attrs),
            &[AmdIommuDiscovery {
                exists: true,
                max_banks: 2,
                max_counters: 4,
                register_result: 0,
            }],
        )
        .unwrap();
        assert_eq!(init.initialized.len(), 1);
        assert_eq!(init.cpumask_cpu, Some(0));
        assert!(!init.free_events_attrs);
    }

    #[test]
    fn capability_zero_disables_iommu_pmu() {
        assert_eq!(iommu_pmu_from_capability(0), None);
        assert_eq!(
            iommu_pmu_from_capability(0x0201),
            Some(AmdIommuPmu {
                banks: 1,
                counters_per_bank: 2,
                counter_bits: 48,
            })
        );
    }
}
