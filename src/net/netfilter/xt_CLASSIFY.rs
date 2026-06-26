//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_CLASSIFY.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_CLASSIFY.c
//! Xtables qdisc classification target.

pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "Xtables: Qdisc classification";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 3] = ["ipt_CLASSIFY", "ip6t_CLASSIFY", "arpt_CLASSIFY"];
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_ARP: u8 = 3;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NF_INET_POST_ROUTING: u8 = 4;
pub const NF_ARP_OUT: u8 = 1;
pub const NF_ARP_FORWARD: u8 = 2;

pub const CLASSIFY_INET_HOOKS: u32 =
    (1 << NF_INET_LOCAL_OUT) | (1 << NF_INET_FORWARD) | (1 << NF_INET_POST_ROUTING);
pub const CLASSIFY_ARP_HOOKS: u32 = (1 << NF_ARP_OUT) | (1 << NF_ARP_FORWARD);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClassifySkb {
    pub priority: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtClassifyTargetInfo {
    pub priority: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub hooks: u32,
    pub targetsize: usize,
}

pub const CLASSIFY_TG_REG: [XtTarget; 3] = [
    XtTarget {
        name: "CLASSIFY",
        revision: 0,
        family: NFPROTO_IPV4,
        hooks: CLASSIFY_INET_HOOKS,
        targetsize: core::mem::size_of::<XtClassifyTargetInfo>(),
    },
    XtTarget {
        name: "CLASSIFY",
        revision: 0,
        family: NFPROTO_ARP,
        hooks: CLASSIFY_ARP_HOOKS,
        targetsize: core::mem::size_of::<XtClassifyTargetInfo>(),
    },
    XtTarget {
        name: "CLASSIFY",
        revision: 0,
        family: NFPROTO_IPV6,
        hooks: CLASSIFY_INET_HOOKS,
        targetsize: core::mem::size_of::<XtClassifyTargetInfo>(),
    },
];

pub const fn classify_tg(skb: &mut ClassifySkb, info: XtClassifyTargetInfo) -> u32 {
    skb.priority = info.priority;
    XT_CONTINUE
}

pub const fn classify_tg_init() -> &'static [XtTarget; 3] {
    &CLASSIFY_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_classify_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_CLASSIFY.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Patrick McHardy"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: Qdisc classification\")"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_CLASSIFY\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_CLASSIFY\");"));
        assert!(source.contains("MODULE_ALIAS(\"arpt_CLASSIFY\");"));
        assert!(source.contains("skb->priority = clinfo->priority;"));
        assert!(source.contains("return XT_CONTINUE;"));
        assert!(source.contains(".name       = \"CLASSIFY\""));
        assert!(source.contains(".family     = NFPROTO_IPV4"));
        assert!(source.contains(".family     = NFPROTO_ARP"));
        assert!(source.contains(".family     = NFPROTO_IPV6"));
        assert!(
            source.contains("xt_register_targets(classify_tg_reg, ARRAY_SIZE(classify_tg_reg));")
        );
        assert!(
            source.contains("xt_unregister_targets(classify_tg_reg, ARRAY_SIZE(classify_tg_reg));")
        );

        let mut skb = ClassifySkb::default();
        assert_eq!(
            classify_tg(&mut skb, XtClassifyTargetInfo { priority: 0x10020 }),
            XT_CONTINUE
        );
        assert_eq!(skb.priority, 0x10020);
        assert_eq!(classify_tg_init().len(), 3);
        assert_eq!(
            MODULE_ALIASES,
            ["ipt_CLASSIFY", "ip6t_CLASSIFY", "arpt_CLASSIFY"]
        );
    }
}
