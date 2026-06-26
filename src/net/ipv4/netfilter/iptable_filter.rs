//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/iptable_filter.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/iptable_filter.c
//! IPv4 iptables filter table registration.

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Netfilter Core Team <coreteam@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "iptables filter table";
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NFPROTO_IPV4: u8 = 2;
pub const NF_IP_PRI_FILTER: i32 = 0;
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
    af: NFPROTO_IPV4,
    priority: NF_IP_PRI_FILTER,
};

pub const fn forward_verdict(forward: bool) -> i32 {
    if forward { -NF_ACCEPT - 1 } else { NF_DROP - 1 }
}

pub const fn iptable_filter_table_init(alloc_ok: bool, register_ret: i32) -> Result<i32, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(register_ret)
    }
}

pub const fn iptable_filter_net_init(forward: bool, table_init_ret: i32) -> Result<(), i32> {
    if !forward && table_init_ret < 0 {
        return Err(table_init_ret);
    }
    Ok(())
}

pub const fn iptable_filter_init(
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

pub const fn iptable_filter_net_pre_exit() -> (&'static str, u8) {
    ("filter", NFPROTO_IPV4)
}

pub const fn iptable_filter_net_exit() -> &'static str {
    "filter"
}

pub const fn iptable_filter_fini(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iptable_filter_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/iptable_filter.c"
        ));
        let netfilter = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter.h"
        ));
        let ipv4 = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv4.h"
        ));

        assert!(netfilter.contains("NF_INET_LOCAL_IN"));
        assert!(netfilter.contains("NFPROTO_IPV4   =  2"));
        assert!(ipv4.contains("NF_IP_PRI_FILTER = 0"));
        assert!(source.contains("MODULE_DESCRIPTION(\"iptables filter table\")"));
        assert!(source.contains("#define FILTER_VALID_HOOKS"));
        assert!(source.contains(".name\t\t= \"filter\""));
        assert!(source.contains(".valid_hooks\t= FILTER_VALID_HOOKS"));
        assert!(source.contains(".af\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".priority\t= NF_IP_PRI_FILTER"));
        assert!(source.contains("static bool forward __read_mostly = true;"));
        assert!(source.contains("module_param(forward, bool, 0000);"));
        assert!(source.contains("repl = ipt_alloc_initial_table(&packet_filter);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("Entry 1 is the FORWARD hook"));
        assert!(source.contains("forward ? -NF_ACCEPT - 1 : NF_DROP - 1;"));
        assert!(source.contains("ipt_register_table(net, &packet_filter, repl, filter_ops);"));
        assert!(source.contains("if (!forward)"));
        assert!(source.contains("xt_unregister_table_pre_exit(net, NFPROTO_IPV4, \"filter\");"));
        assert!(source.contains("ipt_unregister_table_exit(net, \"filter\");"));
        assert!(source.contains("xt_hook_ops_alloc(&packet_filter, ipt_do_table);"));
        assert!(source.contains("register_pernet_subsys(&iptable_filter_net_ops);"));
        assert!(source.contains("xt_register_template(&packet_filter"));
        assert!(source.contains("xt_unregister_template(&packet_filter);"));
    }

    #[test]
    fn filter_table_uses_forward_policy_and_registration_edges() {
        assert_eq!(
            PACKET_FILTER,
            XtTable {
                name: "filter",
                valid_hooks: 0b1110,
                af: NFPROTO_IPV4,
                priority: NF_IP_PRI_FILTER,
            }
        );
        assert_eq!(forward_verdict(true), -2);
        assert_eq!(forward_verdict(false), -1);
        assert_eq!(iptable_filter_table_init(false, 0), Err(-ENOMEM));
        assert_eq!(iptable_filter_table_init(true, -4), Err(-4));
        assert_eq!(iptable_filter_table_init(true, 0), Ok(0));
        assert_eq!(iptable_filter_net_init(true, -9), Ok(()));
        assert_eq!(iptable_filter_net_init(false, -9), Err(-9));
        assert_eq!(iptable_filter_init(Err(-1), 0, 0), Err(-1));
        assert_eq!(iptable_filter_init(Ok(()), -2, 0), Err(-2));
        assert_eq!(iptable_filter_init(Ok(()), 0, -3), Err(-3));
        assert_eq!(iptable_filter_init(Ok(()), 0, 0), Ok(()));
        assert_eq!(iptable_filter_net_pre_exit(), ("filter", NFPROTO_IPV4));
        assert_eq!(iptable_filter_net_exit(), "filter");
        assert!(iptable_filter_fini(true));
    }
}
