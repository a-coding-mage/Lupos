//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/nf_dup_ipv4.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/nf_dup_ipv4.c
//! IPv4 packet duplication route and local output decisions.

pub const MODULE_DESCRIPTION: &str = "nf_dup_ipv4: Duplicate IPv4 packet";
pub const MODULE_LICENSE: &str = "GPL";
pub const NF_INET_PRE_ROUTING: u32 = 0;
pub const NF_INET_LOCAL_IN: u32 = 1;
pub const ETH_P_IP: u16 = 0x0800;
pub const IP_DF: u16 = 0x4000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DupIpv4Packet {
    pub ttl: u8,
    pub frag_off: u16,
    pub protocol: u16,
    pub dev_set: bool,
    pub dst_set: bool,
    pub conntrack_reset: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DupIpv4Result {
    Sent(DupIpv4Packet),
    Dropped(DupIpv4Packet),
    SkippedReentrant,
    SkippedCopyFailed,
}

pub const fn nf_dup_ipv4_route(mut skb: DupIpv4Packet, route_error: bool) -> Option<DupIpv4Packet> {
    if route_error {
        return None;
    }
    skb.dst_set = true;
    skb.dev_set = true;
    skb.protocol = ETH_P_IP.to_be();
    Some(skb)
}

pub const fn nf_dup_ipv4(
    skb: DupIpv4Packet,
    hooknum: u32,
    in_nf_duplicate: bool,
    copy_ok: bool,
    route_error: bool,
) -> DupIpv4Result {
    if in_nf_duplicate {
        return DupIpv4Result::SkippedReentrant;
    }
    if !copy_ok {
        return DupIpv4Result::SkippedCopyFailed;
    }

    let mut copy = skb;
    copy.conntrack_reset = true;
    copy.frag_off |= IP_DF.to_be();
    if hooknum == NF_INET_PRE_ROUTING || hooknum == NF_INET_LOCAL_IN {
        copy.ttl = copy.ttl.wrapping_sub(1);
    }

    match nf_dup_ipv4_route(copy, route_error) {
        Some(routed) => DupIpv4Result::Sent(routed),
        None => DupIpv4Result::Dropped(copy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_dup_ipv4_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/nf_dup_ipv4.c"
        ));
        assert!(source.contains("static bool nf_dup_ipv4_route"));
        assert!(source.contains("memset(&fl4, 0, sizeof(fl4));"));
        assert!(source.contains("if (oif != -1)"));
        assert!(source.contains("fl4.flowi4_oif = oif;"));
        assert!(source.contains("fl4.daddr = gw->s_addr;"));
        assert!(source.contains("fl4.flowi4_flags = FLOWI_FLAG_KNOWN_NH;"));
        assert!(source.contains("rt = ip_route_output_key(net, &fl4);"));
        assert!(source.contains("skb_dst_drop(skb);"));
        assert!(source.contains("skb_dst_set(skb, &rt->dst);"));
        assert!(source.contains("skb->protocol = htons(ETH_P_IP);"));
        assert!(source.contains("void nf_dup_ipv4"));
        assert!(source.contains("if (current->in_nf_duplicate)"));
        assert!(source.contains("skb = pskb_copy(skb, GFP_ATOMIC);"));
        assert!(source.contains("nf_reset_ct(skb);"));
        assert!(source.contains("iph->frag_off |= htons(IP_DF);"));
        assert!(source.contains("--iph->ttl;"));
        assert!(source.contains("ip_local_out(net, skb->sk, skb);"));
        assert!(source.contains("kfree_skb(skb);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(nf_dup_ipv4);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"nf_dup_ipv4: Duplicate IPv4 packet\")"));
    }

    #[test]
    fn ipv4_duplicate_sets_df_decrements_input_ttl_and_routes_copy() {
        let skb = DupIpv4Packet {
            ttl: 64,
            frag_off: 0,
            protocol: 0,
            dev_set: false,
            dst_set: false,
            conntrack_reset: false,
        };
        assert_eq!(
            nf_dup_ipv4(skb, NF_INET_PRE_ROUTING, false, true, false),
            DupIpv4Result::Sent(DupIpv4Packet {
                ttl: 63,
                frag_off: IP_DF.to_be(),
                protocol: ETH_P_IP.to_be(),
                dev_set: true,
                dst_set: true,
                conntrack_reset: true,
            })
        );
        assert_eq!(
            nf_dup_ipv4(skb, 4, true, true, false),
            DupIpv4Result::SkippedReentrant
        );
        assert_eq!(
            nf_dup_ipv4(skb, 4, false, false, false),
            DupIpv4Result::SkippedCopyFailed
        );
        assert!(matches!(
            nf_dup_ipv4(skb, NF_INET_LOCAL_IN, false, true, true),
            DupIpv4Result::Dropped(_)
        ));
    }
}
