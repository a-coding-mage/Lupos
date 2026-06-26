//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_length.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_length.c
//! Xtables packet length match.

pub const MODULE_AUTHOR: &str = "James Morris <jmorris@intercode.com.au>";
pub const MODULE_DESCRIPTION: &str = "Xtables: Packet length (Layer3,4,5) match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_length", "ip6t_length"];

pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtLengthInfo {
    pub min: u32,
    pub max: u32,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
}

pub const LENGTH_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "length",
        family: NFPROTO_IPV4,
        matchsize: core::mem::size_of::<XtLengthInfo>(),
    },
    XtMatch {
        name: "length",
        family: NFPROTO_IPV6,
        matchsize: core::mem::size_of::<XtLengthInfo>(),
    },
];

pub const fn length_mt(pktlen: u32, info: XtLengthInfo) -> bool {
    (pktlen >= info.min && pktlen <= info.max) != info.invert
}

pub const fn length_mt_ipv4(skb_ip_totlen: u32, info: XtLengthInfo) -> bool {
    length_mt(skb_ip_totlen, info)
}

pub const fn length_mt6(skb_len: u32, info: XtLengthInfo) -> bool {
    length_mt(skb_len, info)
}

pub const fn length_mt_init() -> &'static [XtMatch; 2] {
    &LENGTH_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_length_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_length.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"James Morris"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_length\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_length\");"));
        assert!(source.contains("length_mt(const struct sk_buff *skb"));
        assert!(source.contains("u32 pktlen = skb_ip_totlen(skb);"));
        assert!(
            source.contains("return (pktlen >= info->min && pktlen <= info->max) ^ info->invert;")
        );
        assert!(source.contains("length_mt6(const struct sk_buff *skb"));
        assert!(source.contains("u32 pktlen = skb->len;"));
        assert!(source.contains(".name\t\t= \"length\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".matchsize\t= sizeof(struct xt_length_info)"));
        assert!(source.contains("xt_register_matches(length_mt_reg, ARRAY_SIZE(length_mt_reg));"));
        assert!(
            source.contains("xt_unregister_matches(length_mt_reg, ARRAY_SIZE(length_mt_reg));")
        );

        assert_eq!(MODULE_ALIASES, ["ipt_length", "ip6t_length"]);
        assert_eq!(LENGTH_MT_REG[0].family, NFPROTO_IPV4);
        assert_eq!(LENGTH_MT_REG[1].family, NFPROTO_IPV6);
    }

    #[test]
    fn length_match_uses_inclusive_range_and_invert() {
        let info = XtLengthInfo {
            min: 20,
            max: 40,
            invert: false,
        };
        assert!(length_mt_ipv4(20, info));
        assert!(length_mt6(40, info));
        assert!(!length_mt_ipv4(41, info));
        assert!(length_mt_ipv4(
            41,
            XtLengthInfo {
                invert: true,
                ..info
            }
        ));
        assert_eq!(length_mt_init().len(), 2);
    }
}
