//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_mac.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_mac.c
//! Xtables source MAC address match.

pub const MODULE_AUTHOR: &str = "Netfilter Core Team <coreteam@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "Xtables: MAC address match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_mac", "ip6t_mac"];

pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const ARPHRD_ETHER: u16 = 1;
pub const ETH_HLEN: usize = 14;
pub const MAC_HOOKS: u32 =
    (1 << NF_INET_PRE_ROUTING) | (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_FORWARD);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMacInfo {
    pub srcaddr: [u8; 6],
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacPacketContext {
    pub dev_type: Option<u16>,
    pub mac_header_was_set: bool,
    pub mac_header_len: usize,
    pub eth_source: [u8; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
    pub hooks: u32,
}

pub const MAC_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "mac",
        family: NFPROTO_IPV4,
        matchsize: core::mem::size_of::<XtMacInfo>(),
        hooks: MAC_HOOKS,
    },
    XtMatch {
        name: "mac",
        family: NFPROTO_IPV6,
        matchsize: core::mem::size_of::<XtMacInfo>(),
        hooks: MAC_HOOKS,
    },
];

pub const fn mac_mt(ctx: MacPacketContext, info: XtMacInfo) -> bool {
    let Some(ARPHRD_ETHER) = ctx.dev_type else {
        return false;
    };
    if !ctx.mac_header_was_set || ctx.mac_header_len < ETH_HLEN {
        return false;
    }

    mac_addr_equal(ctx.eth_source, info.srcaddr) ^ info.invert
}

pub const fn mac_addr_equal(left: [u8; 6], right: [u8; 6]) -> bool {
    let mut i = 0;
    while i < 6 {
        if left[i] != right[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub const fn mac_mt_init() -> &'static [XtMatch; 2] {
    &MAC_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_mac_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_mac.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Netfilter Core Team"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_mac\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_mac\");"));
        assert!(source.contains("if (skb->dev == NULL || skb->dev->type != ARPHRD_ETHER)"));
        assert!(source.contains("skb_mac_header_len(skb) < ETH_HLEN"));
        assert!(source.contains("ret  = ether_addr_equal(eth_hdr(skb)->h_source, info->srcaddr);"));
        assert!(source.contains("ret ^= info->invert;"));
        assert!(source.contains(".name\t\t= \"mac\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".matchsize\t= sizeof(struct xt_mac_info)"));
        assert!(source.contains("xt_register_matches(mac_mt_reg, ARRAY_SIZE(mac_mt_reg));"));

        assert_eq!(MAC_MT_REG[0].family, NFPROTO_IPV4);
        assert_eq!(MAC_MT_REG[1].family, NFPROTO_IPV6);
        assert_eq!(MODULE_ALIASES, ["ipt_mac", "ip6t_mac"]);
    }

    #[test]
    fn mac_match_requires_ethernet_header_and_honors_invert() {
        let ctx = MacPacketContext {
            dev_type: Some(ARPHRD_ETHER),
            mac_header_was_set: true,
            mac_header_len: ETH_HLEN,
            eth_source: [0x02, 0, 0, 0, 0, 1],
        };
        let info = XtMacInfo {
            srcaddr: [0x02, 0, 0, 0, 0, 1],
            invert: false,
        };
        assert!(mac_mt(ctx, info));
        assert!(!mac_mt(
            ctx,
            XtMacInfo {
                invert: true,
                ..info
            }
        ));
        assert!(mac_mt(
            ctx,
            XtMacInfo {
                srcaddr: [0, 0, 0, 0, 0, 0],
                invert: true,
            }
        ));
        assert!(!mac_mt(
            MacPacketContext {
                dev_type: None,
                ..ctx
            },
            info
        ));
        assert!(!mac_mt(
            MacPacketContext {
                mac_header_len: ETH_HLEN - 1,
                ..ctx
            },
            info
        ));
    }
}
