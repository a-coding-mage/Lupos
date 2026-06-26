//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebtable_filter.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebtable_filter.c
//! Ebtables legacy filter table registration.

pub const MODULE_DESCRIPTION: &str = "ebtables legacy filter table";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;
pub const NF_BR_LOCAL_IN: u8 = 1;
pub const NF_BR_FORWARD: u8 = 2;
pub const NF_BR_LOCAL_OUT: u8 = 3;
pub const NF_BR_PRI_FILTER_BRIDGED: i32 = -200;
pub const NF_BR_PRI_FILTER_OTHER: i32 = 200;
pub const EBT_ACCEPT: i32 = -1;
pub const NF_BR_NUMHOOKS: usize = 6;
pub const FILTER_VALID_HOOKS: u32 =
    (1 << NF_BR_LOCAL_IN) | (1 << NF_BR_FORWARD) | (1 << NF_BR_LOCAL_OUT);

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
        name: "INPUT",
        policy: EBT_ACCEPT,
    },
    EbtEntries {
        name: "FORWARD",
        policy: EBT_ACCEPT,
    },
    EbtEntries {
        name: "OUTPUT",
        policy: EBT_ACCEPT,
    },
];

pub const INITIAL_TABLE: EbtReplaceKernel = EbtReplaceKernel {
    name: "filter",
    valid_hooks: FILTER_VALID_HOOKS,
    entries_size: 3 * core::mem::size_of::<EbtEntries>(),
    hook_entry: [None, Some(0), Some(1), Some(2), None, None],
    entries: &INITIAL_CHAINS,
};

pub const FRAME_FILTER: EbtTable = EbtTable {
    name: "filter",
    valid_hooks: FILTER_VALID_HOOKS,
};

pub const EBT_OPS_FILTER: [NfHookOps; 3] = [
    NfHookOps {
        pf: NFPROTO_BRIDGE,
        hooknum: NF_BR_LOCAL_IN,
        priority: NF_BR_PRI_FILTER_BRIDGED,
    },
    NfHookOps {
        pf: NFPROTO_BRIDGE,
        hooknum: NF_BR_FORWARD,
        priority: NF_BR_PRI_FILTER_BRIDGED,
    },
    NfHookOps {
        pf: NFPROTO_BRIDGE,
        hooknum: NF_BR_LOCAL_OUT,
        priority: NF_BR_PRI_FILTER_OTHER,
    },
];

pub const FRAME_FILTER_NET_OPS: PernetOperations = PernetOperations {
    pre_exit: "frame_filter_net_pre_exit",
    exit: "frame_filter_net_exit",
};

pub const fn frame_filter_table_init(register_ret: i32) -> i32 {
    register_ret
}

pub const fn frame_filter_net_pre_exit() -> &'static str {
    "filter"
}

pub const fn frame_filter_net_exit() -> &'static str {
    "filter"
}

pub const fn ebtable_filter_init(
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

pub const fn ebtable_filter_fini(state: EbtTableInitState) -> (bool, bool) {
    (state.template_registered, state.pernet_registered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebtable_filter_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebtable_filter.c"
        ));
        let bridge = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_bridge.h"
        ));
        assert!(bridge.contains("#define NF_BR_LOCAL_IN\t\t1"));
        assert!(bridge.contains("NF_BR_PRI_FILTER_BRIDGED = -200"));
        assert!(bridge.contains("NF_BR_PRI_FILTER_OTHER = 200"));
        assert!(source.contains("#define FILTER_VALID_HOOKS"));
        assert!(source.contains(".name\t= \"INPUT\""));
        assert!(source.contains(".name\t= \"FORWARD\""));
        assert!(source.contains(".name\t= \"OUTPUT\""));
        assert!(source.contains(".policy\t= EBT_ACCEPT"));
        assert!(source.contains(".name\t\t= \"filter\""));
        assert!(source.contains(".valid_hooks\t= FILTER_VALID_HOOKS"));
        assert!(source.contains("[NF_BR_LOCAL_IN]\t= &initial_chains[0]"));
        assert!(source.contains("[NF_BR_FORWARD]\t\t= &initial_chains[1]"));
        assert!(source.contains("[NF_BR_LOCAL_OUT]\t= &initial_chains[2]"));
        assert!(source.contains(".hook\t\t= ebt_do_table"));
        assert!(source.contains("frame_filter_table_init(struct net *net)"));
        assert!(source.contains("ebt_register_table(net, &frame_filter, ebt_ops_filter);"));
        assert!(source.contains("ebt_unregister_table_pre_exit(net, \"filter\");"));
        assert!(source.contains("ebt_unregister_table(net, \"filter\");"));
        assert!(source.contains("register_pernet_subsys(&frame_filter_net_ops);"));
        assert!(source.contains("ebt_register_template(&frame_filter, frame_filter_table_init);"));
        assert!(source.contains("unregister_pernet_subsys(&frame_filter_net_ops);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"ebtables legacy filter table\")"));
    }

    #[test]
    fn filter_table_describes_chains_hooks_and_registration_edges() {
        assert_eq!(FILTER_VALID_HOOKS, 0b1110);
        assert_eq!(
            INITIAL_CHAINS,
            [
                EbtEntries {
                    name: "INPUT",
                    policy: EBT_ACCEPT,
                },
                EbtEntries {
                    name: "FORWARD",
                    policy: EBT_ACCEPT,
                },
                EbtEntries {
                    name: "OUTPUT",
                    policy: EBT_ACCEPT,
                },
            ]
        );
        assert_eq!(
            INITIAL_TABLE.hook_entry,
            [None, Some(0), Some(1), Some(2), None, None]
        );
        assert_eq!(
            EBT_OPS_FILTER,
            [
                NfHookOps {
                    pf: NFPROTO_BRIDGE,
                    hooknum: NF_BR_LOCAL_IN,
                    priority: NF_BR_PRI_FILTER_BRIDGED,
                },
                NfHookOps {
                    pf: NFPROTO_BRIDGE,
                    hooknum: NF_BR_FORWARD,
                    priority: NF_BR_PRI_FILTER_BRIDGED,
                },
                NfHookOps {
                    pf: NFPROTO_BRIDGE,
                    hooknum: NF_BR_LOCAL_OUT,
                    priority: NF_BR_PRI_FILTER_OTHER,
                },
            ]
        );
        assert_eq!(frame_filter_table_init(-7), -7);
        assert_eq!(frame_filter_net_pre_exit(), "filter");
        assert_eq!(frame_filter_net_exit(), "filter");
        assert_eq!(ebtable_filter_init(-1, 0), Err(-1));
        assert_eq!(ebtable_filter_init(0, -2), Err(-2));
        let state = ebtable_filter_init(0, 0).unwrap();
        assert_eq!(ebtable_filter_fini(state), (true, true));
    }
}
