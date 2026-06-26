//! linux-parity: complete
//! linux-source: vendor/linux/kernel/power/em_netlink_autogen.c
//! test-origin: linux:vendor/linux/kernel/power/em_netlink_autogen.c
//! Generated generic-netlink family description for dev-energymodel.

pub const DEV_ENERGYMODEL_FAMILY_NAME: &str = "dev-energymodel";
pub const DEV_ENERGYMODEL_FAMILY_VERSION: u8 = 1;
pub const GENL_CMD_CAP_DO: u32 = 1;
pub const GENL_CMD_CAP_DUMP: u32 = 2;
pub const NLA_U32: &'static str = "NLA_U32";

pub const DEV_ENERGYMODEL_CMD_GET_PERF_DOMAINS: u8 = 1;
pub const DEV_ENERGYMODEL_CMD_GET_PERF_TABLE: u8 = 2;
pub const DEV_ENERGYMODEL_A_PERF_DOMAIN_PERF_DOMAIN_ID: u16 = 2;
pub const DEV_ENERGYMODEL_A_PERF_TABLE_PERF_DOMAIN_ID: u16 = 1;
pub const DEV_ENERGYMODEL_NLGRP_EVENT: usize = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnergyModelNetlinkOp {
    pub cmd: u8,
    pub doit: bool,
    pub dumpit: bool,
    pub policy_attr: Option<u16>,
    pub maxattr: Option<u16>,
    pub flags: u32,
}

pub const DEV_ENERGYMODEL_NL_OPS: [EnergyModelNetlinkOp; 3] = [
    EnergyModelNetlinkOp {
        cmd: DEV_ENERGYMODEL_CMD_GET_PERF_DOMAINS,
        doit: true,
        dumpit: false,
        policy_attr: Some(DEV_ENERGYMODEL_A_PERF_DOMAIN_PERF_DOMAIN_ID),
        maxattr: Some(DEV_ENERGYMODEL_A_PERF_DOMAIN_PERF_DOMAIN_ID),
        flags: GENL_CMD_CAP_DO,
    },
    EnergyModelNetlinkOp {
        cmd: DEV_ENERGYMODEL_CMD_GET_PERF_DOMAINS,
        doit: false,
        dumpit: true,
        policy_attr: None,
        maxattr: None,
        flags: GENL_CMD_CAP_DUMP,
    },
    EnergyModelNetlinkOp {
        cmd: DEV_ENERGYMODEL_CMD_GET_PERF_TABLE,
        doit: true,
        dumpit: false,
        policy_attr: Some(DEV_ENERGYMODEL_A_PERF_TABLE_PERF_DOMAIN_ID),
        maxattr: Some(DEV_ENERGYMODEL_A_PERF_TABLE_PERF_DOMAIN_ID),
        flags: GENL_CMD_CAP_DO,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnergyModelNetlinkFamily {
    pub name: &'static str,
    pub version: u8,
    pub netnsok: bool,
    pub parallel_ops: bool,
    pub ops: &'static [EnergyModelNetlinkOp],
    pub multicast_groups: &'static [&'static str],
}

pub const DEV_ENERGYMODEL_NL_FAMILY: EnergyModelNetlinkFamily = EnergyModelNetlinkFamily {
    name: DEV_ENERGYMODEL_FAMILY_NAME,
    version: DEV_ENERGYMODEL_FAMILY_VERSION,
    netnsok: true,
    parallel_ops: true,
    ops: &DEV_ENERGYMODEL_NL_OPS,
    multicast_groups: &["event"],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_model_netlink_family_matches_generated_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/power/em_netlink_autogen.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/power/em_netlink_autogen.h"
        ));
        assert!(source.contains("auto-generated from:"));
        assert!(source.contains("dev_energymodel_get_perf_domains_nl_policy"));
        assert!(source.contains(".type = NLA_U32"));
        assert!(source.contains(".cmd\t\t= DEV_ENERGYMODEL_CMD_GET_PERF_DOMAINS"));
        assert!(source.contains(".dumpit\t= dev_energymodel_nl_get_perf_domains_dumpit"));
        assert!(source.contains(".cmd\t\t= DEV_ENERGYMODEL_CMD_GET_PERF_TABLE"));
        assert!(source.contains("[DEV_ENERGYMODEL_NLGRP_EVENT] = { \"event\", }"));
        assert!(source.contains(".name\t\t= DEV_ENERGYMODEL_FAMILY_NAME"));
        assert!(source.contains(".netnsok\t= true"));
        assert!(source.contains(".parallel_ops\t= true"));
        assert!(header.contains("extern struct genl_family dev_energymodel_nl_family;"));

        assert_eq!(DEV_ENERGYMODEL_NL_FAMILY.name, "dev-energymodel");
        assert_eq!(DEV_ENERGYMODEL_NL_FAMILY.ops.len(), 3);
        assert!(DEV_ENERGYMODEL_NL_FAMILY.ops[0].doit);
        assert!(DEV_ENERGYMODEL_NL_FAMILY.ops[1].dumpit);
        assert_eq!(DEV_ENERGYMODEL_NL_FAMILY.multicast_groups, ["event"]);
    }
}
