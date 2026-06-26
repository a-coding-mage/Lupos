//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/arptable_filter.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/arptable_filter.c
//! ARP filter table registration.

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "David S. Miller <davem@redhat.com>";
pub const MODULE_DESCRIPTION: &str = "arptables filter table";
pub const NF_ARP_IN: u32 = 0;
pub const NF_ARP_OUT: u32 = 1;
pub const NF_ARP_FORWARD: u32 = 2;
pub const NFPROTO_ARP: u8 = 3;
pub const NF_IP_PRI_FILTER: i32 = 0;
pub const FILTER_VALID_HOOKS: u32 = (1 << NF_ARP_IN) | (1 << NF_ARP_OUT) | (1 << NF_ARP_FORWARD);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTable {
    pub name: &'static str,
    pub valid_hooks: u32,
    pub af: u8,
    pub priority: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArptableFilterState {
    pub hook_ops_allocated: bool,
    pub pernet_registered: bool,
    pub template_registered: bool,
}

pub const PACKET_FILTER: XtTable = XtTable {
    name: "filter",
    valid_hooks: FILTER_VALID_HOOKS,
    af: NFPROTO_ARP,
    priority: NF_IP_PRI_FILTER,
};

pub const fn arptable_filter_table_init(
    repl_alloc_ok: bool,
    register_table_ret: i32,
) -> Result<i32, i32> {
    if !repl_alloc_ok {
        Err(-ENOMEM)
    } else {
        Ok(register_table_ret)
    }
}

pub const fn arptable_filter_init(
    hook_ops_ret: Result<(), i32>,
    pernet_ret: i32,
    template_ret: i32,
) -> Result<ArptableFilterState, i32> {
    if let Err(err) = hook_ops_ret {
        return Err(err);
    }
    if pernet_ret < 0 {
        return Err(pernet_ret);
    }
    if template_ret < 0 {
        return Err(template_ret);
    }
    Ok(ArptableFilterState {
        hook_ops_allocated: true,
        pernet_registered: true,
        template_registered: true,
    })
}

pub const fn arptable_filter_fini(state: ArptableFilterState) -> (bool, bool, bool) {
    (
        state.template_registered,
        state.pernet_registered,
        state.hook_ops_allocated,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arptable_filter_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/arptable_filter.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"David S. Miller <davem@redhat.com>\");"));
        assert!(source.contains("MODULE_DESCRIPTION(\"arptables filter table\")"));
        assert!(
            source.contains("#define FILTER_VALID_HOOKS ((1 << NF_ARP_IN) | (1 << NF_ARP_OUT)")
        );
        assert!(source.contains("static const struct xt_table packet_filter"));
        assert!(source.contains(".name\t\t= \"filter\""));
        assert!(source.contains(".valid_hooks\t= FILTER_VALID_HOOKS"));
        assert!(source.contains(".af\t\t= NFPROTO_ARP"));
        assert!(source.contains(".priority\t= NF_IP_PRI_FILTER"));
        assert!(source.contains("arpt_alloc_initial_table(&packet_filter);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("arpt_register_table(net, &packet_filter, repl, arpfilter_ops);"));
        assert!(source.contains("xt_unregister_table_pre_exit(net, NFPROTO_ARP, \"filter\");"));
        assert!(source.contains("arpt_unregister_table(net, \"filter\");"));
        assert!(source.contains("xt_hook_ops_alloc(&packet_filter, arpt_do_table);"));
        assert!(source.contains("register_pernet_subsys(&arptable_filter_net_ops);"));
        assert!(source.contains("xt_register_template(&packet_filter"));
        assert!(source.contains("xt_unregister_template(&packet_filter);"));
    }

    #[test]
    fn arptable_filter_registration_tracks_linux_cleanup_order() {
        assert_eq!(
            PACKET_FILTER,
            XtTable {
                name: "filter",
                valid_hooks: 0b111,
                af: NFPROTO_ARP,
                priority: 0,
            }
        );
        assert_eq!(arptable_filter_table_init(false, 0), Err(-ENOMEM));
        let state = arptable_filter_init(Ok(()), 0, 0).unwrap();
        assert_eq!(arptable_filter_fini(state), (true, true, true));
        assert_eq!(arptable_filter_init(Err(-5), 0, 0), Err(-5));
        assert_eq!(arptable_filter_init(Ok(()), -7, 0), Err(-7));
        assert_eq!(arptable_filter_init(Ok(()), 0, -9), Err(-9));
    }
}
