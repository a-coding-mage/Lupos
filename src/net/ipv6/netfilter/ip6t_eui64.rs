//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/ip6t_eui64.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/ip6t_eui64.c
//! Xtables IPv6 EUI-64 address match.

pub const MODULE_AUTHOR: &str = "Andras Kis-Szabo <kisza@sch.bme.hu>";
pub const MODULE_DESCRIPTION: &str = "Xtables: IPv6 EUI64 address match";
pub const MODULE_LICENSE: &str = "GPL";

pub const NFPROTO_IPV6: u8 = 10;
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const ARPHRD_ETHER: u16 = 1;
pub const ETH_HLEN: usize = 14;
pub const ETH_P_IPV6: u16 = 0x86dd;
pub const EUI64_HOOKS: u32 =
    (1 << NF_INET_PRE_ROUTING) | (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_FORWARD);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Eui64PacketContext {
    pub dev_type: Option<u16>,
    pub mac_header_was_set: bool,
    pub mac_header_len: usize,
    pub eth_proto: u16,
    pub ipv6_version: u8,
    pub eth_source: [u8; 6],
    pub ipv6_source: [u8; 16],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Eui64MatchResult {
    pub matched: bool,
    pub hotdrop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
    pub hooks: u32,
}

pub const EUI64_MT6_REG: XtMatch = XtMatch {
    name: "eui64",
    family: NFPROTO_IPV6,
    matchsize: core::mem::size_of::<i32>(),
    hooks: EUI64_HOOKS,
};

pub const fn eui64_from_mac(mac: [u8; 6]) -> [u8; 8] {
    [
        mac[0] ^ 0x02,
        mac[1],
        mac[2],
        0xff,
        0xfe,
        mac[3],
        mac[4],
        mac[5],
    ]
}

pub const fn eui64_mt6(ctx: Eui64PacketContext) -> Eui64MatchResult {
    let Some(ARPHRD_ETHER) = ctx.dev_type else {
        return Eui64MatchResult {
            matched: false,
            hotdrop: false,
        };
    };
    if !ctx.mac_header_was_set || ctx.mac_header_len < ETH_HLEN {
        return Eui64MatchResult {
            matched: false,
            hotdrop: true,
        };
    }
    if ctx.eth_proto != ETH_P_IPV6.to_be() || ctx.ipv6_version != 0x6 {
        return Eui64MatchResult {
            matched: false,
            hotdrop: false,
        };
    }
    let eui64 = eui64_from_mac(ctx.eth_source);
    let matched = ctx.ipv6_source[8] == eui64[0]
        && ctx.ipv6_source[9] == eui64[1]
        && ctx.ipv6_source[10] == eui64[2]
        && ctx.ipv6_source[11] == eui64[3]
        && ctx.ipv6_source[12] == eui64[4]
        && ctx.ipv6_source[13] == eui64[5]
        && ctx.ipv6_source[14] == eui64[6]
        && ctx.ipv6_source[15] == eui64[7];
    Eui64MatchResult {
        matched,
        hotdrop: false,
    }
}

pub const fn eui64_mt6_init() -> &'static XtMatch {
    &EUI64_MT6_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip6t_eui64_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/ip6t_eui64.c"
        ));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: IPv6 EUI64 address match\")"));
        assert!(source.contains("MODULE_AUTHOR(\"Andras Kis-Szabo"));
        assert!(source.contains("if (!skb->dev || skb->dev->type != ARPHRD_ETHER)"));
        assert!(source.contains("par->hotdrop = true;"));
        assert!(source.contains("eth_hdr(skb)->h_proto == htons(ETH_P_IPV6)"));
        assert!(source.contains("ipv6_hdr(skb)->version == 0x6"));
        assert!(source.contains("memcpy(eui64, eth_hdr(skb)->h_source, 3);"));
        assert!(source.contains("eui64[3] = 0xff;"));
        assert!(source.contains("eui64[4] = 0xfe;"));
        assert!(source.contains("eui64[0] ^= 0x02;"));
        assert!(source.contains("!memcmp(ipv6_hdr(skb)->saddr.s6_addr + 8, eui64"));
        assert!(source.contains(".name\t\t= \"eui64\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".matchsize\t= sizeof(int)"));
        assert!(source.contains("xt_register_match(&eui64_mt6_reg);"));

        assert_eq!(EUI64_MT6_REG.name, "eui64");
        assert_eq!(
            eui64_from_mac([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            [0x02, 0x11, 0x22, 0xff, 0xfe, 0x33, 0x44, 0x55]
        );
    }

    #[test]
    fn eui64_match_checks_ethernet_ipv6_and_hotdrop() {
        let mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let mut ipv6_source = [0u8; 16];
        ipv6_source[0..8].copy_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0]);
        ipv6_source[8..16].copy_from_slice(&eui64_from_mac(mac));
        let ctx = Eui64PacketContext {
            dev_type: Some(ARPHRD_ETHER),
            mac_header_was_set: true,
            mac_header_len: ETH_HLEN,
            eth_proto: ETH_P_IPV6.to_be(),
            ipv6_version: 6,
            eth_source: mac,
            ipv6_source,
        };
        assert_eq!(
            eui64_mt6(ctx),
            Eui64MatchResult {
                matched: true,
                hotdrop: false
            }
        );
        assert_eq!(
            eui64_mt6(Eui64PacketContext {
                mac_header_was_set: false,
                ..ctx
            }),
            Eui64MatchResult {
                matched: false,
                hotdrop: true
            }
        );
        assert!(
            !eui64_mt6(Eui64PacketContext {
                eth_proto: 0,
                ..ctx
            })
            .matched
        );
    }
}
