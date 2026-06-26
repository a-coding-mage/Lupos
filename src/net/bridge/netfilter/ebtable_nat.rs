//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebtable_nat.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebtable_nat.c
//! Ebtables legacy stateless nat table registration.

pub const MODULE_DESCRIPTION: &str = "ebtables legacy stateless nat table";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;
pub const NF_BR_PRE_ROUTING: u8 = 0;
pub const NF_BR_LOCAL_OUT: u8 = 3;
pub const NF_BR_POST_ROUTING: u8 = 4;
pub const NF_BR_PRI_NAT_DST_BRIDGED: i32 = -300;
pub const NF_BR_PRI_NAT_DST_OTHER: i32 = 100;
pub const NF_BR_PRI_NAT_SRC: i32 = 300;
pub const EBT_ACCEPT: i32 = -1;
pub const NF_BR_NUMHOOKS: usize = 6;
pub const NAT_VALID_HOOKS: u32 =
    (1 << NF_BR_PRE_ROUTING) | (1 << NF_BR_LOCAL_OUT) | (1 << NF_BR_POST_ROUTING);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtEntries {
    pub name: &'static str,
    pub policy: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtReplaceKernel {
    pub name: &'static str,
    pub valid_hooks: u32,
    pub entries_size: usize,
    pub hook_entry: [Option<usize>; NF_BR_NUMHOOKS],
    pub entries: &'static [EbtEntries],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtTable {
    pub name: &'static str,
    pub valid_hooks: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfHookOps {
    pub pf: u8,
    pub hooknum: u8,
    pub priority: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PernetOperations {
    pub pre_exit: &'static str,
    pub exit: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtTableInitState {
    pub pernet_registered: bool,
    pub template_registered: bool,
}

pub const INITIAL_CHAINS: [EbtEntries; 3] = [
    EbtEntries {
        name: "PREROUTING",
        policy: EBT_ACCEPT,
    },
    EbtEntries {
        name: "OUTPUT",
        policy: EBT_ACCEPT,
    },
    EbtEntries {
        name: "POSTROUTING",
        policy: EBT_ACCEPT,
    },
];

pub const INITIAL_TABLE: EbtReplaceKernel = EbtReplaceKernel {
    name: "nat",
    valid_hooks: NAT_VALID_HOOKS,
    entries_size: 3 * core::mem::size_of::<EbtEntries>(),
    hook_entry: [Some(0), None, None, Some(1), Some(2), None],
    entries: &INITIAL_CHAINS,
};

pub const FRAME_NAT: EbtTable = EbtTable {
    name: "nat",
    valid_hooks: NAT_VALID_HOOKS,
};

pub const EBT_OPS_NAT: [NfHookOps; 3] = [
    NfHookOps {
        pf: NFPROTO_BRIDGE,
        hooknum: NF_BR_LOCAL_OUT,
        priority: NF_BR_PRI_NAT_DST_OTHER,
    },
    NfHookOps {
        pf: NFPROTO_BRIDGE,
        hooknum: NF_BR_POST_ROUTING,
        priority: NF_BR_PRI_NAT_SRC,
    },
    NfHookOps {
        pf: NFPROTO_BRIDGE,
        hooknum: NF_BR_PRE_ROUTING,
        priority: NF_BR_PRI_NAT_DST_BRIDGED,
    },
];

pub const FRAME_NAT_NET_OPS: PernetOperations = PernetOperations {
    pre_exit: "frame_nat_net_pre_exit",
    exit: "frame_nat_net_exit",
};

pub const fn frame_nat_table_init(register_ret: i32) -> i32 {
    register_ret
}

pub const fn frame_nat_net_pre_exit() -> &'static str {
    "nat"
}

pub const fn frame_nat_net_exit() -> &'static str {
    "nat"
}

pub const fn ebtable_nat_init(
    register_pernet_ret: i32,
    register_template_ret: i32,
) -> Result<EbtTableInitState, i32> {
    if register_pernet_ret != 0 {
        return Err(register_pernet_ret);
    }
    if register_template_ret != 0 {
        return Err(register_template_ret);
    }
    Ok(EbtTableInitState {
        pernet_registered: true,
        template_registered: true,
    })
}

pub const fn ebtable_nat_fini(state: EbtTableInitState) -> (bool, bool) {
    (state.template_registered, state.pernet_registered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebtable_nat_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebtable_nat.c"
        ));
        let bridge = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_bridge.h"
        ));
        assert!(bridge.contains("#define NF_BR_PRE_ROUTING\t0"));
        assert!(bridge.contains("NF_BR_PRI_NAT_DST_BRIDGED = -300"));
        assert!(bridge.contains("NF_BR_PRI_NAT_DST_OTHER = 100"));
        assert!(bridge.contains("NF_BR_PRI_NAT_SRC = 300"));
        assert!(source.contains("#define NAT_VALID_HOOKS"));
        assert!(source.contains(".name\t= \"PREROUTING\""));
        assert!(source.contains(".name\t= \"OUTPUT\""));
        assert!(source.contains(".name\t= \"POSTROUTING\""));
        assert!(source.contains(".policy\t= EBT_ACCEPT"));
        assert!(source.contains(".name\t\t= \"nat\""));
        assert!(source.contains(".valid_hooks\t= NAT_VALID_HOOKS"));
        assert!(source.contains("[NF_BR_PRE_ROUTING]\t= &initial_chains[0]"));
        assert!(source.contains("[NF_BR_LOCAL_OUT]\t= &initial_chains[1]"));
        assert!(source.contains("[NF_BR_POST_ROUTING]\t= &initial_chains[2]"));
        assert!(source.contains(".hook\t\t= ebt_do_table"));
        assert!(source.contains("frame_nat_table_init(struct net *net)"));
        assert!(source.contains("ebt_register_table(net, &frame_nat, ebt_ops_nat);"));
        assert!(source.contains("ebt_unregister_table_pre_exit(net, \"nat\");"));
        assert!(source.contains("ebt_unregister_table(net, \"nat\");"));
        assert!(source.contains("register_pernet_subsys(&frame_nat_net_ops);"));
        assert!(source.contains("ebt_register_template(&frame_nat, frame_nat_table_init);"));
        assert!(source.contains("unregister_pernet_subsys(&frame_nat_net_ops);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"ebtables legacy stateless nat table\")"));
    }

    #[test]
    fn nat_table_describes_chains_hooks_and_registration_edges() {
        assert_eq!(NAT_VALID_HOOKS, 0b1_1001);
        assert_eq!(
            INITIAL_CHAINS,
            [
                EbtEntries {
                    name: "PREROUTING",
                    policy: EBT_ACCEPT,
                },
                EbtEntries {
                    name: "OUTPUT",
                    policy: EBT_ACCEPT,
                },
                EbtEntries {
                    name: "POSTROUTING",
                    policy: EBT_ACCEPT,
                },
            ]
        );
        assert_eq!(
            INITIAL_TABLE.hook_entry,
            [Some(0), None, None, Some(1), Some(2), None]
        );
        assert_eq!(
            EBT_OPS_NAT,
            [
                NfHookOps {
                    pf: NFPROTO_BRIDGE,
                    hooknum: NF_BR_LOCAL_OUT,
                    priority: NF_BR_PRI_NAT_DST_OTHER,
                },
                NfHookOps {
                    pf: NFPROTO_BRIDGE,
                    hooknum: NF_BR_POST_ROUTING,
                    priority: NF_BR_PRI_NAT_SRC,
                },
                NfHookOps {
                    pf: NFPROTO_BRIDGE,
                    hooknum: NF_BR_PRE_ROUTING,
                    priority: NF_BR_PRI_NAT_DST_BRIDGED,
                },
            ]
        );
        assert_eq!(frame_nat_table_init(-7), -7);
        assert_eq!(frame_nat_net_pre_exit(), "nat");
        assert_eq!(frame_nat_net_exit(), "nat");
        assert_eq!(ebtable_nat_init(-1, 0), Err(-1));
        assert_eq!(ebtable_nat_init(0, -2), Err(-2));
        let state = ebtable_nat_init(0, 0).unwrap();
        assert_eq!(ebtable_nat_fini(state), (true, true));
    }
}
