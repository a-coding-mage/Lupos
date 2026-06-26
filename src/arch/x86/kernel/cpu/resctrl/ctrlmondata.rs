//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/resctrl/ctrlmondata.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/resctrl/ctrlmondata.c
//! x86 resctrl control-domain MSR update model.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const CDP_NUM_TYPES: usize = 3;
pub const MSR_IA32_L3_QOS_EXT_CFG: u32 = 0xc000_03ff;
pub const SDCIAE_ENABLE_BIT: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResctrlConfType {
    CdpNone,
    CdpCode,
    CdpData,
}

impl ResctrlConfType {
    pub const fn index(self) -> usize {
        match self {
            Self::CdpNone => 0,
            Self::CdpCode => 1,
            Self::CdpData => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResctrlStagedConfig {
    pub new_ctrl: u32,
    pub have_new_ctrl: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdtCtrlDomain {
    pub id: i32,
    pub cpu_mask: u64,
    pub staged_config: [ResctrlStagedConfig; CDP_NUM_TYPES],
    pub ctrl_val: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdtResource {
    pub io_alloc_capable: bool,
    pub sdciae_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsrParam {
    pub domain_id: i32,
    pub low: u32,
    pub high: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainMsrUpdate {
    pub cpu_mask: u64,
    pub param: MsrParam,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainsUpdatePlan {
    pub result: i32,
    pub updates: Vec<DomainMsrUpdate>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsrBitOp {
    pub cpu_mask: u64,
    pub msr: u32,
    pub bit: u8,
    pub set: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoAllocEnablePlan {
    pub result: i32,
    pub sdciae_enabled: bool,
    pub ops: Vec<MsrBitOp>,
}

pub const fn resctrl_get_config_index(closid: u32, typ: ResctrlConfType) -> u32 {
    match typ {
        ResctrlConfType::CdpNone => closid,
        ResctrlConfType::CdpCode => closid * 2 + 1,
        ResctrlConfType::CdpData => closid * 2,
    }
}

pub const fn cpumask_test_cpu(cpu: u32, cpu_mask: u64) -> bool {
    cpu < 64 && (cpu_mask & (1u64 << cpu)) != 0
}

pub fn resctrl_arch_update_one(
    _resource: &RdtResource,
    domain: &mut RdtCtrlDomain,
    current_cpu: u32,
    closid: u32,
    typ: ResctrlConfType,
    cfg_val: u32,
) -> Result<MsrParam, i32> {
    if !cpumask_test_cpu(current_cpu, domain.cpu_mask) {
        return Err(EINVAL);
    }

    let idx = resctrl_get_config_index(closid, typ) as usize;
    if idx >= domain.ctrl_val.len() {
        return Err(EINVAL);
    }

    domain.ctrl_val[idx] = cfg_val;
    Ok(MsrParam {
        domain_id: domain.id,
        low: idx as u32,
        high: idx as u32 + 1,
    })
}

pub fn resctrl_arch_update_domains(
    _resource: &RdtResource,
    domains: &mut [RdtCtrlDomain],
    closid: u32,
) -> DomainsUpdatePlan {
    let mut updates = Vec::new();

    for domain in domains {
        let mut msr_param: Option<MsrParam> = None;

        for typ in [
            ResctrlConfType::CdpNone,
            ResctrlConfType::CdpCode,
            ResctrlConfType::CdpData,
        ] {
            let cfg = domain.staged_config[typ.index()];
            if !cfg.have_new_ctrl {
                continue;
            }

            let idx = resctrl_get_config_index(closid, typ) as usize;
            if idx >= domain.ctrl_val.len() {
                continue;
            }
            if cfg.new_ctrl == domain.ctrl_val[idx] {
                continue;
            }

            domain.ctrl_val[idx] = cfg.new_ctrl;
            match &mut msr_param {
                Some(param) => {
                    param.low = param.low.min(idx as u32);
                    param.high = param.high.max(idx as u32 + 1);
                }
                None => {
                    msr_param = Some(MsrParam {
                        domain_id: domain.id,
                        low: idx as u32,
                        high: idx as u32 + 1,
                    });
                }
            }
        }

        if let Some(param) = msr_param {
            updates.push(DomainMsrUpdate {
                cpu_mask: domain.cpu_mask,
                param,
            });
        }
    }

    DomainsUpdatePlan { result: 0, updates }
}

pub fn resctrl_arch_get_config(
    domain: &RdtCtrlDomain,
    closid: u32,
    typ: ResctrlConfType,
) -> Option<u32> {
    let idx = resctrl_get_config_index(closid, typ) as usize;
    domain.ctrl_val.get(idx).copied()
}

pub const fn resctrl_arch_get_io_alloc_enabled(resource: &RdtResource) -> bool {
    resource.sdciae_enabled
}

pub const fn resctrl_sdciae_set_one_amd(cpu_mask: u64, enable: bool) -> MsrBitOp {
    MsrBitOp {
        cpu_mask,
        msr: MSR_IA32_L3_QOS_EXT_CFG,
        bit: SDCIAE_ENABLE_BIT,
        set: enable,
    }
}

pub fn resctrl_arch_io_alloc_enable(
    resource: &mut RdtResource,
    domains: &[RdtCtrlDomain],
    enable: bool,
) -> IoAllocEnablePlan {
    let mut ops = Vec::new();

    if resource.io_alloc_capable && resource.sdciae_enabled != enable {
        for domain in domains {
            ops.push(resctrl_sdciae_set_one_amd(domain.cpu_mask, enable));
        }
        resource.sdciae_enabled = enable;
    }

    IoAllocEnablePlan {
        result: 0,
        sdciae_enabled: resource.sdciae_enabled,
        ops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(id: i32, cpu_mask: u64, ctrl_val: &[u32]) -> RdtCtrlDomain {
        RdtCtrlDomain {
            id,
            cpu_mask,
            staged_config: [ResctrlStagedConfig::default(); CDP_NUM_TYPES],
            ctrl_val: ctrl_val.to_vec(),
        }
    }

    #[test]
    fn ctrlmondata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/resctrl/ctrlmondata.c"
        ));
        let internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/resctrl/internal.h"
        ));
        let resctrl = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/resctrl.h"
        ));
        let msr_index = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/msr-index.h"
        ));

        assert!(source.contains("int resctrl_arch_update_one"));
        assert!(source.contains("if (!cpumask_test_cpu(smp_processor_id(), &d->hdr.cpu_mask))"));
        assert!(source.contains("hw_dom->ctrl_val[idx] = cfg_val;"));
        assert!(source.contains("hw_res->msr_update(&msr_param);"));
        assert!(source.contains("int resctrl_arch_update_domains"));
        assert!(source.contains("for (t = 0; t < CDP_NUM_TYPES; t++)"));
        assert!(source.contains("if (!cfg->have_new_ctrl)"));
        assert!(source.contains("if (cfg->new_ctrl == hw_dom->ctrl_val[idx])"));
        assert!(source.contains("smp_call_function_any(&d->hdr.cpu_mask, rdt_ctrl_update"));
        assert!(source.contains("u32 resctrl_arch_get_config"));
        assert!(source.contains("bool resctrl_arch_get_io_alloc_enabled"));
        assert!(source.contains("msr_set_bit(MSR_IA32_L3_QOS_EXT_CFG, SDCIAE_ENABLE_BIT);"));
        assert!(source.contains("msr_clear_bit(MSR_IA32_L3_QOS_EXT_CFG, SDCIAE_ENABLE_BIT);"));
        assert!(source.contains("if (hw_res->r_resctrl.cache.io_alloc_capable &&"));
        assert!(internal.contains("#define SDCIAE_ENABLE_BIT\t\t1"));
        assert!(resctrl.contains("#define CDP_NUM_TYPES\t(CDP_DATA + 1)"));
        assert!(resctrl.contains("return closid * 2 + 1;"));
        assert!(resctrl.contains("return closid * 2;"));
        assert!(msr_index.contains("#define MSR_IA32_L3_QOS_EXT_CFG"));
    }

    #[test]
    fn config_index_matches_resctrl_cdp_mapping() {
        assert_eq!(resctrl_get_config_index(3, ResctrlConfType::CdpNone), 3);
        assert_eq!(resctrl_get_config_index(3, ResctrlConfType::CdpCode), 7);
        assert_eq!(resctrl_get_config_index(3, ResctrlConfType::CdpData), 6);
    }

    #[test]
    fn update_one_requires_current_cpu_in_domain_and_writes_single_index() {
        let resource = RdtResource {
            io_alloc_capable: false,
            sdciae_enabled: false,
        };
        let mut dom = domain(10, 0b0100, &[0; 8]);

        assert_eq!(
            resctrl_arch_update_one(&resource, &mut dom, 1, 2, ResctrlConfType::CdpCode, 0xbeef),
            Err(EINVAL)
        );

        let param =
            resctrl_arch_update_one(&resource, &mut dom, 2, 2, ResctrlConfType::CdpCode, 0xbeef)
                .unwrap();
        assert_eq!(
            param,
            MsrParam {
                domain_id: 10,
                low: 5,
                high: 6,
            }
        );
        assert_eq!(dom.ctrl_val[5], 0xbeef);
    }

    #[test]
    fn update_domains_batches_changed_staged_configs_per_domain() {
        let resource = RdtResource {
            io_alloc_capable: false,
            sdciae_enabled: false,
        };
        let mut domains = [domain(1, 0b0011, &[0; 8]), domain(2, 0b1100, &[0; 8])];
        domains[0].staged_config[ResctrlConfType::CdpCode.index()] = ResctrlStagedConfig {
            new_ctrl: 0x11,
            have_new_ctrl: true,
        };
        domains[0].staged_config[ResctrlConfType::CdpData.index()] = ResctrlStagedConfig {
            new_ctrl: 0x22,
            have_new_ctrl: true,
        };
        domains[1].staged_config[ResctrlConfType::CdpNone.index()] = ResctrlStagedConfig {
            new_ctrl: 0,
            have_new_ctrl: true,
        };

        let plan = resctrl_arch_update_domains(&resource, &mut domains, 1);
        assert_eq!(plan.result, 0);
        assert_eq!(plan.updates.len(), 1);
        assert_eq!(
            plan.updates[0],
            DomainMsrUpdate {
                cpu_mask: 0b0011,
                param: MsrParam {
                    domain_id: 1,
                    low: 2,
                    high: 4,
                },
            }
        );
        assert_eq!(domains[0].ctrl_val[2], 0x22);
        assert_eq!(domains[0].ctrl_val[3], 0x11);
    }

    #[test]
    fn get_config_reads_arch_ctrl_array_by_cdp_type() {
        let dom = domain(1, 1, &[10, 11, 12, 13, 14, 15]);
        assert_eq!(
            resctrl_arch_get_config(&dom, 2, ResctrlConfType::CdpNone),
            Some(12)
        );
        assert_eq!(
            resctrl_arch_get_config(&dom, 2, ResctrlConfType::CdpData),
            Some(14)
        );
        assert_eq!(
            resctrl_arch_get_config(&dom, 2, ResctrlConfType::CdpCode),
            Some(15)
        );
        assert_eq!(
            resctrl_arch_get_config(&dom, 8, ResctrlConfType::CdpNone),
            None
        );
    }

    #[test]
    fn io_alloc_enable_updates_sdciae_on_each_domain_only_when_needed() {
        let mut resource = RdtResource {
            io_alloc_capable: true,
            sdciae_enabled: false,
        };
        let domains = [domain(1, 0b0011, &[0]), domain(2, 0b1100, &[0])];

        assert!(!resctrl_arch_get_io_alloc_enabled(&resource));
        let plan = resctrl_arch_io_alloc_enable(&mut resource, &domains, true);
        assert_eq!(plan.result, 0);
        assert!(plan.sdciae_enabled);
        assert_eq!(
            plan.ops,
            [
                MsrBitOp {
                    cpu_mask: 0b0011,
                    msr: MSR_IA32_L3_QOS_EXT_CFG,
                    bit: SDCIAE_ENABLE_BIT,
                    set: true,
                },
                MsrBitOp {
                    cpu_mask: 0b1100,
                    msr: MSR_IA32_L3_QOS_EXT_CFG,
                    bit: SDCIAE_ENABLE_BIT,
                    set: true,
                },
            ]
        );

        let plan = resctrl_arch_io_alloc_enable(&mut resource, &domains, true);
        assert!(plan.ops.is_empty());

        resource.io_alloc_capable = false;
        let plan = resctrl_arch_io_alloc_enable(&mut resource, &domains, false);
        assert!(plan.sdciae_enabled);
        assert!(plan.ops.is_empty());
    }
}
