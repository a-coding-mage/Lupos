//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_pkttype.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_pkttype.c
//! Xtables link-layer packet type match.

pub const MODULE_AUTHOR: &str = "Michal Ludvig <michal@logix.cz>";
pub const MODULE_DESCRIPTION: &str = "Xtables: link layer packet type match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_pkttype", "ip6t_pkttype"];

pub const NFPROTO_UNSPEC: u8 = 0;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const PACKET_BROADCAST: u8 = 1;
pub const PACKET_MULTICAST: u8 = 2;
pub const PACKET_LOOPBACK: u8 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtPkttypeInfo {
    pub pkttype: u8,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacketContext {
    pub pkt_type: u8,
    pub family: u8,
    pub ipv4_daddr: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const PKTTYPE_MT_REG: XtMatch = XtMatch {
    name: "pkttype",
    revision: 0,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtPkttypeInfo>(),
};

pub fn effective_packet_type(ctx: PacketContext) -> u8 {
    if ctx.pkt_type != PACKET_LOOPBACK {
        ctx.pkt_type
    } else if ctx.family == NFPROTO_IPV4 && ctx.ipv4_daddr.is_some_and(ipv4_is_multicast) {
        PACKET_MULTICAST
    } else if ctx.family == NFPROTO_IPV6 {
        PACKET_MULTICAST
    } else {
        PACKET_BROADCAST
    }
}

pub fn pkttype_mt(info: XtPkttypeInfo, ctx: PacketContext) -> bool {
    (effective_packet_type(ctx) == info.pkttype) ^ info.invert
}

pub const fn pkttype_mt_init() -> &'static XtMatch {
    &PKTTYPE_MT_REG
}

pub const fn ipv4_is_multicast(addr: u32) -> bool {
    (addr & 0xf000_0000) == 0xe000_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_pkttype_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_pkttype.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Michal Ludvig"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_pkttype\")"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_pkttype\")"));
        assert!(source.contains("if (skb->pkt_type != PACKET_LOOPBACK)"));
        assert!(source.contains("ipv4_is_multicast(ip_hdr(skb)->daddr)"));
        assert!(source.contains("type = PACKET_MULTICAST;"));
        assert!(source.contains("type = PACKET_BROADCAST;"));
        assert!(source.contains(".name      = \"pkttype\""));
        assert!(source.contains(".family    = NFPROTO_UNSPEC"));

        let multicast_loopback = PacketContext {
            pkt_type: PACKET_LOOPBACK,
            family: NFPROTO_IPV4,
            ipv4_daddr: Some(0xe000_00fb),
        };
        assert_eq!(effective_packet_type(multicast_loopback), PACKET_MULTICAST);
        assert!(pkttype_mt(
            XtPkttypeInfo {
                pkttype: PACKET_MULTICAST,
                invert: false,
            },
            multicast_loopback,
        ));
        assert_eq!(
            effective_packet_type(PacketContext {
                pkt_type: PACKET_LOOPBACK,
                family: NFPROTO_IPV4,
                ipv4_daddr: Some(0x0a00_0001),
            }),
            PACKET_BROADCAST
        );
        assert!(pkttype_mt(
            XtPkttypeInfo {
                pkttype: PACKET_BROADCAST,
                invert: true,
            },
            multicast_loopback,
        ));
        assert_eq!(PKTTYPE_MT_REG.name, "pkttype");
    }
}
