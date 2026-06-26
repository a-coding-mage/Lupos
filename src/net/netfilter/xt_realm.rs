//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_realm.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_realm.c
//! Xtables IPv4 routing realm match.

pub const MODULE_AUTHOR: &str = "Sampsa Ranta <sampsa@netsonic.fi>";
pub const MODULE_DESCRIPTION: &str = "Xtables: Routing realm match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "ipt_realm";

pub const NFPROTO_IPV4: u8 = 2;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NF_INET_POST_ROUTING: u8 = 4;
pub const REALM_HOOKS: u32 = (1 << NF_INET_POST_ROUTING)
    | (1 << NF_INET_FORWARD)
    | (1 << NF_INET_LOCAL_OUT)
    | (1 << NF_INET_LOCAL_IN);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtRealmInfo {
    pub id: u32,
    pub mask: u32,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DstEntry {
    pub tclassid: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub matchsize: usize,
    pub hooks: u32,
    pub family: u8,
}

pub const REALM_MT_REG: XtMatch = XtMatch {
    name: "realm",
    matchsize: core::mem::size_of::<XtRealmInfo>(),
    hooks: REALM_HOOKS,
    family: NFPROTO_IPV4,
};

pub fn realm_mt(info: XtRealmInfo, dst: DstEntry) -> bool {
    (info.id == (dst.tclassid & info.mask)) ^ info.invert
}

pub const fn realm_mt_init() -> &'static XtMatch {
    &REALM_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_realm_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_realm.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Sampsa Ranta"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: Routing realm match\")"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_realm\")"));
        assert!(source.contains("realm_mt(const struct sk_buff *skb"));
        assert!(source.contains("skb_dst(skb)"));
        assert!(source.contains("dst->tclassid & info->mask"));
        assert!(source.contains("^ info->invert"));
        assert!(source.contains(".name\t\t= \"realm\""));
        assert!(source.contains(".hooks\t\t= (1 << NF_INET_POST_ROUTING)"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains("xt_register_match(&realm_mt_reg);"));
        assert!(source.contains("xt_unregister_match(&realm_mt_reg);"));

        let info = XtRealmInfo {
            id: 0x1200,
            mask: 0xff00,
            invert: false,
        };
        assert!(realm_mt(info, DstEntry { tclassid: 0x1234 }));
        assert!(!realm_mt(info, DstEntry { tclassid: 0x2234 }));
        assert!(realm_mt(
            XtRealmInfo {
                invert: true,
                ..info
            },
            DstEntry { tclassid: 0x2234 },
        ));
        assert_eq!(REALM_MT_REG.name, "realm");
        assert_eq!(REALM_MT_REG.family, NFPROTO_IPV4);
        assert_eq!(REALM_MT_REG.hooks, REALM_HOOKS);
        assert_eq!(realm_mt_init(), &REALM_MT_REG);
    }
}
