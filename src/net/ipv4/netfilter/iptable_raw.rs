//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/iptable_raw.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/iptable_raw.c
//! IPv4 iptables raw table registration.

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "iptables legacy raw table";
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NFPROTO_IPV4: u8 = 2;
pub const NF_IP_PRI_RAW_BEFORE_DEFRAG: i32 = -450;
pub const NF_IP_PRI_RAW: i32 = -300;
pub const RAW_VALID_HOOKS: u32 = (1 << NF_INET_PRE_ROUTING) | (1 << NF_INET_LOCAL_OUT);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTable {
    pub name: &'static str,
    pub valid_hooks: u32,
    pub af: u8,
    pub priority: i32,
}

pub const PACKET_RAW: XtTable = XtTable {
    name: "raw",
    valid_hooks: RAW_VALID_HOOKS,
    af: NFPROTO_IPV4,
    priority: NF_IP_PRI_RAW,
};

pub const PACKET_RAW_BEFORE_DEFRAG: XtTable = XtTable {
    name: "raw",
    valid_hooks: RAW_VALID_HOOKS,
    af: NFPROTO_IPV4,
    priority: NF_IP_PRI_RAW_BEFORE_DEFRAG,
};

pub const fn selected_raw_table(raw_before_defrag: bool) -> XtTable {
    if raw_before_defrag {
        PACKET_RAW_BEFORE_DEFRAG
    } else {
        PACKET_RAW
    }
}

pub const fn iptable_raw_table_init(
    raw_before_defrag: bool,
    alloc_ok: bool,
    register_ret: i32,
) -> Result<XtTable, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(selected_raw_table(raw_before_defrag))
    }
}

pub const fn iptable_raw_init(
    hook_alloc_ret: Result<(), i32>,
    register_pernet_ret: i32,
    register_template_ret: i32,
) -> Result<(), i32> {
    if let Err(err) = hook_alloc_ret {
        return Err(err);
    }
    if register_pernet_ret < 0 {
        return Err(register_pernet_ret);
    }
    if register_template_ret < 0 {
        return Err(register_template_ret);
    }
    Ok(())
}

pub const fn iptable_raw_net_pre_exit() -> (&'static str, u8) {
    ("raw", NFPROTO_IPV4)
}

pub const fn iptable_raw_net_exit() -> &'static str {
    "raw"
}

pub const fn iptable_raw_fini() -> XtTable {
    PACKET_RAW
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iptable_raw_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/iptable_raw.c"
        ));
        let netfilter = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter.h"
        ));
        let ipv4 = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv4.h"
        ));

        assert!(netfilter.contains("NF_INET_PRE_ROUTING"));
        assert!(netfilter.contains("NFPROTO_IPV4   =  2"));
        assert!(ipv4.contains("NF_IP_PRI_RAW_BEFORE_DEFRAG = -450"));
        assert!(ipv4.contains("NF_IP_PRI_RAW = -300"));
        assert!(source.contains("#define RAW_VALID_HOOKS"));
        assert!(source.contains("static bool raw_before_defrag"));
        assert!(source.contains("MODULE_PARM_DESC(raw_before_defrag"));
        assert!(source.contains("module_param(raw_before_defrag, bool, 0000);"));
        assert!(source.contains(".name = \"raw\""));
        assert!(source.contains(".af = NFPROTO_IPV4"));
        assert!(source.contains(".priority = NF_IP_PRI_RAW"));
        assert!(source.contains(".priority = NF_IP_PRI_RAW_BEFORE_DEFRAG"));
        assert!(source.contains("if (raw_before_defrag)"));
        assert!(source.contains("repl = ipt_alloc_initial_table(table);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("ipt_register_table(net, table, repl, rawtable_ops);"));
        assert!(source.contains("xt_unregister_table_pre_exit(net, NFPROTO_IPV4, \"raw\");"));
        assert!(source.contains("ipt_unregister_table_exit(net, \"raw\");"));
        assert!(source.contains("xt_hook_ops_alloc(table, ipt_do_table);"));
        assert!(source.contains("register_pernet_subsys(&iptable_raw_net_ops);"));
        assert!(source.contains("xt_register_template(table"));
        assert!(source.contains("xt_unregister_template(&packet_raw);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"iptables legacy raw table\")"));
    }

    #[test]
    fn raw_table_selects_before_defrag_and_propagates_errors() {
        assert_eq!(selected_raw_table(false), PACKET_RAW);
        assert_eq!(selected_raw_table(true), PACKET_RAW_BEFORE_DEFRAG);
        assert_eq!(iptable_raw_table_init(false, false, 0), Err(-ENOMEM));
        assert_eq!(iptable_raw_table_init(false, true, -7), Err(-7));
        assert_eq!(
            iptable_raw_table_init(true, true, 0),
            Ok(PACKET_RAW_BEFORE_DEFRAG)
        );
        assert_eq!(iptable_raw_init(Err(-1), 0, 0), Err(-1));
        assert_eq!(iptable_raw_init(Ok(()), -2, 0), Err(-2));
        assert_eq!(iptable_raw_init(Ok(()), 0, -3), Err(-3));
        assert_eq!(iptable_raw_init(Ok(()), 0, 0), Ok(()));
        assert_eq!(iptable_raw_net_pre_exit(), ("raw", NFPROTO_IPV4));
        assert_eq!(iptable_raw_net_exit(), "raw");
        assert_eq!(iptable_raw_fini(), PACKET_RAW);
    }
}
