//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/iptable_security.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/iptable_security.c
//! IPv4 iptables security table.

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "James Morris <jmorris <at> redhat.com>";
pub const MODULE_DESCRIPTION: &str = "iptables security table, for MAC rules";
pub const NFPROTO_IPV4: u8 = 2;
pub const NF_IP_PRI_SECURITY: i32 = 50;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
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
    af: NFPROTO_IPV4,
    priority: NF_IP_PRI_SECURITY,
};

pub fn iptable_security_table_init(
    alloc_initial_table_ok: bool,
    register_ret: i32,
) -> Result<(), i32> {
    if !alloc_initial_table_ok {
        return Err(-ENOMEM);
    }
    if register_ret < 0 {
        return Err(register_ret);
    }
    Ok(())
}

pub fn iptable_security_init(
    hook_ops_ret: Result<(), i32>,
    register_pernet_ret: i32,
    register_template_ret: i32,
) -> Result<(), i32> {
    hook_ops_ret?;
    if register_pernet_ret < 0 {
        return Err(register_pernet_ret);
    }
    if register_template_ret < 0 {
        return Err(register_template_ret);
    }
    Ok(())
}

pub const fn iptable_security_net_pre_exit() -> (&'static str, u8) {
    ("security", NFPROTO_IPV4)
}

pub const fn iptable_security_net_exit() -> &'static str {
    "security"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iptable_security_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/iptable_security.c"
        ));
        assert!(source.contains("MODULE_DESCRIPTION(\"iptables security table, for MAC rules\")"));
        assert!(source.contains("#define SECURITY_VALID_HOOKS\t(1 << NF_INET_LOCAL_IN)"));
        assert!(source.contains("(1 << NF_INET_FORWARD)"));
        assert!(source.contains("(1 << NF_INET_LOCAL_OUT)"));
        assert!(source.contains(".name\t\t= \"security\""));
        assert!(source.contains(".valid_hooks\t= SECURITY_VALID_HOOKS"));
        assert!(source.contains(".af\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".priority\t= NF_IP_PRI_SECURITY"));
        assert!(source.contains("repl = ipt_alloc_initial_table(&security_table);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("ipt_register_table(net, &security_table, repl, sectbl_ops);"));
        assert!(source.contains("xt_unregister_table_pre_exit(net, NFPROTO_IPV4, \"security\");"));
        assert!(source.contains("ipt_unregister_table_exit(net, \"security\");"));
        assert!(source.contains("xt_hook_ops_alloc(&security_table, ipt_do_table);"));
        assert!(source.contains("register_pernet_subsys(&iptable_security_net_ops);"));
        assert!(source.contains("xt_register_template(&security_table"));
        assert!(source.contains("xt_unregister_template(&security_table);"));

        assert_eq!(SECURITY_TABLE.valid_hooks, 0b1110);
        assert_eq!(SECURITY_TABLE.af, NFPROTO_IPV4);
        assert_eq!(SECURITY_TABLE.priority, NF_IP_PRI_SECURITY);
    }

    #[test]
    fn iptable_security_init_propagates_linux_error_paths() {
        assert_eq!(iptable_security_table_init(false, 0), Err(-ENOMEM));
        assert_eq!(iptable_security_table_init(true, -7), Err(-7));
        assert_eq!(iptable_security_table_init(true, 0), Ok(()));
        assert_eq!(iptable_security_init(Err(-12), 0, 0), Err(-12));
        assert_eq!(iptable_security_init(Ok(()), -3, 0), Err(-3));
        assert_eq!(iptable_security_init(Ok(()), 0, -4), Err(-4));
        assert_eq!(iptable_security_init(Ok(()), 0, 0), Ok(()));
        assert_eq!(iptable_security_net_pre_exit(), ("security", NFPROTO_IPV4));
        assert_eq!(iptable_security_net_exit(), "security");
    }
}
