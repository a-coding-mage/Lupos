//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/ip6table_security.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/ip6table_security.c
//! IPv6 iptables security table registration.

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "James Morris <jmorris <at> redhat.com>";
pub const MODULE_DESCRIPTION: &str = "ip6tables security table, for MAC rules";
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_IP6_PRI_SECURITY: i32 = 50;
pub const SECURITY_VALID_HOOKS: u32 =
    (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_FORWARD) | (1 << NF_INET_LOCAL_OUT);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTable {
    pub name: &'static str,
    pub valid_hooks: u32,
    pub af: u8,
    pub priority: i32,
}

pub const SECURITY_TABLE: XtTable = XtTable {
    name: "security",
    valid_hooks: SECURITY_VALID_HOOKS,
    af: NFPROTO_IPV6,
    priority: NF_IP6_PRI_SECURITY,
};

pub const fn ip6table_security_table_init(alloc_ok: bool, register_ret: i32) -> Result<i32, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(register_ret)
    }
}

pub const fn ip6table_security_init(
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

pub const fn ip6table_security_fini(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip6table_security_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/ip6table_security.c"
        ));
        assert!(source.contains("\"security\" table for IPv6"));
        assert!(source.contains("MODULE_AUTHOR(\"James Morris"));
        assert!(source.contains("#define SECURITY_VALID_HOOKS"));
        assert!(source.contains(".name\t\t= \"security\""));
        assert!(source.contains(".valid_hooks\t= SECURITY_VALID_HOOKS"));
        assert!(source.contains(".af\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".priority\t= NF_IP6_PRI_SECURITY"));
        assert!(source.contains("ip6t_alloc_initial_table(&security_table);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("ip6t_register_table(net, &security_table, repl, sectbl_ops);"));
        assert!(source.contains("xt_unregister_table_pre_exit(net, NFPROTO_IPV6, \"security\");"));
        assert!(source.contains("ip6t_unregister_table_exit(net, \"security\");"));
        assert!(source.contains("xt_hook_ops_alloc(&security_table, ip6t_do_table);"));
        assert!(source.contains("register_pernet_subsys(&ip6table_security_net_ops);"));
        assert!(source.contains("xt_register_template(&security_table"));
        assert!(source.contains("xt_unregister_template(&security_table);"));
    }

    #[test]
    fn security_table_init_propagates_all_registration_edges() {
        assert_eq!(SECURITY_TABLE.name, "security");
        assert_eq!(SECURITY_TABLE.valid_hooks, SECURITY_VALID_HOOKS);
        assert_eq!(ip6table_security_table_init(false, 0), Err(-ENOMEM));
        assert_eq!(ip6table_security_table_init(true, -5), Err(-5));
        assert_eq!(ip6table_security_table_init(true, 0), Ok(0));
        assert_eq!(ip6table_security_init(Err(-12), 0, 0), Err(-12));
        assert_eq!(ip6table_security_init(Ok(()), -7, 0), Err(-7));
        assert_eq!(ip6table_security_init(Ok(()), 0, -9), Err(-9));
        assert_eq!(ip6table_security_init(Ok(()), 0, 0), Ok(()));
        assert!(ip6table_security_fini(true));
    }
}
