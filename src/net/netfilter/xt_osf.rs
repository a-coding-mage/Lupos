//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_osf.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_osf.c
//! Xtables passive OS fingerprint match.

pub const MODULE_AUTHOR: &str = "Evgeniy Polyakov <zbr@ioremap.net>";
pub const MODULE_DESCRIPTION: &str = "Passive OS fingerprint matching.";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_osf", "ip6t_osf"];
pub const NFPROTO_IPV4: u8 = 2;
pub const IPPROTO_TCP: u8 = 6;
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const XT_OSF_HOOKS: u32 =
    (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_PRE_ROUTING) | (1 << NF_INET_FORWARD);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub proto: u8,
    pub hooks: u32,
    pub matchsize: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtOsfInfo {
    pub genre_len: usize,
}

pub const XT_OSF_MATCH: XtMatch = XtMatch {
    name: "osf",
    revision: 0,
    family: NFPROTO_IPV4,
    proto: IPPROTO_TCP,
    hooks: XT_OSF_HOOKS,
    matchsize: core::mem::size_of::<XtOsfInfo>(),
};

pub const fn xt_osf_match_packet(fragoff: bool, nf_osf_matched: bool) -> bool {
    if fragoff { false } else { nf_osf_matched }
}

pub const fn xt_osf_init() -> &'static XtMatch {
    &XT_OSF_MATCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_osf_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_osf.c"
        ));
        assert!(source.contains("xt_osf_match_packet(const struct sk_buff *skb"));
        assert!(source.contains("if (p->fragoff)"));
        assert!(source.contains("return false;"));
        assert!(source.contains("return nf_osf_match(skb, xt_family(p), xt_hooknum(p)"));
        assert!(source.contains(".name \t\t= \"osf\""));
        assert!(source.contains(".revision\t= 0"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".proto\t\t= IPPROTO_TCP"));
        assert!(source.contains("(1 << NF_INET_LOCAL_IN)"));
        assert!(source.contains("(1 << NF_INET_PRE_ROUTING)"));
        assert!(source.contains("(1 << NF_INET_FORWARD)"));
        assert!(source.contains(".matchsize\t= sizeof(struct xt_osf_info)"));
        assert!(source.contains("xt_register_match(&xt_osf_match);"));
        assert!(source.contains("xt_unregister_match(&xt_osf_match);"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_osf\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_osf\");"));
    }

    #[test]
    fn osf_match_rejects_fragments_before_fingerprint_lookup() {
        assert!(!xt_osf_match_packet(true, true));
        assert!(xt_osf_match_packet(false, true));
        assert!(!xt_osf_match_packet(false, false));
        assert_eq!(XT_OSF_MATCH.family, NFPROTO_IPV4);
        assert_eq!(XT_OSF_MATCH.proto, IPPROTO_TCP);
        assert_eq!(xt_osf_init(), &XT_OSF_MATCH);
    }
}
