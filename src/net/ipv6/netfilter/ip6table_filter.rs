//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/ip6table_filter.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/ip6table_filter.c
//! IPv6 iptables filter table registration.

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Netfilter Core Team <coreteam@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "ip6tables filter table";
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_IP6_PRI_FILTER: i32 = 0;
pub const NF_ACCEPT: i32 = 1;
pub const NF_DROP: i32 = 0;
pub const FILTER_VALID_HOOKS: u32 =
    (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_FORWARD) | (1 << NF_INET_LOCAL_OUT);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTable {
    pub name: &'static str,
    pub valid_hooks: u32,
    pub af: u8,
    pub priority: i32,
}

pub const PACKET_FILTER: XtTable = XtTable {
    name: "filter",
    valid_hooks: FILTER_VALID_HOOKS,
    af: NFPROTO_IPV6,
    priority: NF_IP6_PRI_FILTER,
};

pub const fn forward_verdict(forward: bool) -> i32 {
    if forward { -NF_ACCEPT - 1 } else { NF_DROP - 1 }
}

pub const fn ip6table_filter_table_init(alloc_ok: bool, register_ret: i32) -> Result<i32, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(register_ret)
    }
}

pub const fn ip6table_filter_net_init(forward: bool, table_init_ret: i32) -> Result<(), i32> {
    if !forward && table_init_ret < 0 {
        return Err(table_init_ret);
    }
    Ok(())
}

pub const fn ip6table_filter_init(
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

pub const fn ip6table_filter_fini(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip6table_filter_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/ip6table_filter.c"
        ));
        assert!(source.contains("#define FILTER_VALID_HOOKS"));
        assert!(source.contains("MODULE_AUTHOR(\"Netfilter Core Team"));
        assert!(source.contains(".name\t\t= \"filter\""));
        assert!(source.contains(".af\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".priority\t= NF_IP6_PRI_FILTER"));
        assert!(source.contains("static bool forward = true;"));
        assert!(source.contains("module_param(forward, bool, 0000);"));
        assert!(source.contains("ip6t_alloc_initial_table(&packet_filter);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("Entry 1 is the FORWARD hook"));
        assert!(source.contains("forward ? -NF_ACCEPT - 1 : NF_DROP - 1;"));
        assert!(source.contains("if (!forward)"));
        assert!(source.contains("xt_unregister_table_pre_exit(net, NFPROTO_IPV6, \"filter\");"));
        assert!(source.contains("xt_hook_ops_alloc(&packet_filter, ip6t_do_table);"));
        assert!(source.contains("xt_register_template(&packet_filter"));
    }

    #[test]
    fn filter_table_uses_forward_policy_and_registration_edges() {
        assert_eq!(PACKET_FILTER.name, "filter");
        assert_eq!(forward_verdict(true), -2);
        assert_eq!(forward_verdict(false), -1);
        assert_eq!(ip6table_filter_table_init(false, 0), Err(-ENOMEM));
        assert_eq!(ip6table_filter_table_init(true, -4), Err(-4));
        assert_eq!(ip6table_filter_table_init(true, 0), Ok(0));
        assert_eq!(ip6table_filter_net_init(true, -9), Ok(()));
        assert_eq!(ip6table_filter_net_init(false, -9), Err(-9));
        assert_eq!(ip6table_filter_init(Err(-1), 0, 0), Err(-1));
        assert_eq!(ip6table_filter_init(Ok(()), -2, 0), Err(-2));
        assert_eq!(ip6table_filter_init(Ok(()), 0, -3), Err(-3));
        assert!(ip6table_filter_fini(true));
    }
}
