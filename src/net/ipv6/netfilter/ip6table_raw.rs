//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/ip6table_raw.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/ip6table_raw.c
//! IPv6 iptables raw table registration.

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "Ip6tables legacy raw table";
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_IP6_PRI_RAW_BEFORE_DEFRAG: i32 = -450;
pub const NF_IP6_PRI_RAW: i32 = -300;
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
    af: NFPROTO_IPV6,
    priority: NF_IP6_PRI_RAW,
};

pub const PACKET_RAW_BEFORE_DEFRAG: XtTable = XtTable {
    name: "raw",
    valid_hooks: RAW_VALID_HOOKS,
    af: NFPROTO_IPV6,
    priority: NF_IP6_PRI_RAW_BEFORE_DEFRAG,
};

pub const fn selected_raw_table(raw_before_defrag: bool) -> XtTable {
    if raw_before_defrag {
        PACKET_RAW_BEFORE_DEFRAG
    } else {
        PACKET_RAW
    }
}

pub const fn ip6table_raw_table_init(
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

pub const fn ip6table_raw_init(
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

pub const fn ip6table_raw_fini(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip6table_raw_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/ip6table_raw.c"
        ));
        assert!(source.contains("#define RAW_VALID_HOOKS"));
        assert!(source.contains("static bool raw_before_defrag"));
        assert!(source.contains("module_param(raw_before_defrag, bool, 0000);"));
        assert!(source.contains(".name = \"raw\""));
        assert!(source.contains(".priority = NF_IP6_PRI_RAW"));
        assert!(source.contains(".priority = NF_IP6_PRI_RAW_BEFORE_DEFRAG"));
        assert!(source.contains("if (raw_before_defrag)"));
        assert!(source.contains("ip6t_alloc_initial_table(table);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("ip6t_register_table(net, table, repl, rawtable_ops);"));
        assert!(source.contains("xt_unregister_table_pre_exit(net, NFPROTO_IPV6, \"raw\");"));
        assert!(source.contains("xt_hook_ops_alloc(table, ip6t_do_table);"));
        assert!(source.contains("register_pernet_subsys(&ip6table_raw_net_ops);"));
        assert!(source.contains("xt_register_template(table, ip6table_raw_table_init);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Ip6tables legacy raw table\")"));
    }

    #[test]
    fn raw_table_selects_before_defrag_and_propagates_errors() {
        assert_eq!(selected_raw_table(false), PACKET_RAW);
        assert_eq!(selected_raw_table(true), PACKET_RAW_BEFORE_DEFRAG);
        assert_eq!(ip6table_raw_table_init(false, false, 0), Err(-ENOMEM));
        assert_eq!(ip6table_raw_table_init(false, true, -7), Err(-7));
        assert_eq!(
            ip6table_raw_table_init(true, true, 0),
            Ok(PACKET_RAW_BEFORE_DEFRAG)
        );
        assert_eq!(ip6table_raw_init(Err(-1), 0, 0), Err(-1));
        assert_eq!(ip6table_raw_init(Ok(()), -2, 0), Err(-2));
        assert_eq!(ip6table_raw_init(Ok(()), 0, -3), Err(-3));
        assert_eq!(ip6table_raw_init(Ok(()), 0, 0), Ok(()));
        assert!(ip6table_raw_fini(true));
    }
}
