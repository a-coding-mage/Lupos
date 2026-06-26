//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/nf_dup_ipv6.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/nf_dup_ipv6.c
//! IPv6 packet duplication route and output decisions.

pub const MODULE_DESCRIPTION: &str = "nf_dup_ipv6: IPv6 packet duplication";
pub const MODULE_LICENSE: &str = "GPL";
pub const NF_INET_PRE_ROUTING: u32 = 0;
pub const NF_INET_LOCAL_IN: u32 = 1;
pub const ETH_P_IPV6: u16 = 0x86dd;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DupIpv6Packet {
    pub hop_limit: u8,
    pub protocol: u16,
    pub dev_set: bool,
    pub dst_set: bool,
    pub conntrack_reset: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DupIpv6Result {
    Sent(DupIpv6Packet),
    Dropped(DupIpv6Packet),
    SkippedReentrant,
    SkippedCopyFailed,
}

pub const fn nf_dup_ipv6_route(mut skb: DupIpv6Packet, route_error: bool) -> Option<DupIpv6Packet> {
    if route_error {
        return None;
    }
    skb.dst_set = true;
    skb.dev_set = true;
    skb.protocol = ETH_P_IPV6.to_be();
    Some(skb)
}

pub const fn nf_dup_ipv6(
    skb: DupIpv6Packet,
    hooknum: u32,
    in_nf_duplicate: bool,
    copy_ok: bool,
    route_error: bool,
) -> DupIpv6Result {
    if in_nf_duplicate {
        return DupIpv6Result::SkippedReentrant;
    }
    if !copy_ok {
        return DupIpv6Result::SkippedCopyFailed;
    }

    let mut copy = skb;
    copy.conntrack_reset = true;
    if hooknum == NF_INET_PRE_ROUTING || hooknum == NF_INET_LOCAL_IN {
        copy.hop_limit = copy.hop_limit.wrapping_sub(1);
    }

    match nf_dup_ipv6_route(copy, route_error) {
        Some(routed) => DupIpv6Result::Sent(routed),
        None => DupIpv6Result::Dropped(copy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_dup_ipv6_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/nf_dup_ipv6.c"
        ));
        assert!(source.contains("static bool nf_dup_ipv6_route"));
        assert!(source.contains("memset(&fl6, 0, sizeof(fl6));"));
        assert!(source.contains("if (oif != -1)"));
        assert!(source.contains("fl6.flowi6_oif = oif;"));
        assert!(source.contains("fl6.daddr = *gw;"));
        assert!(source.contains("fl6.flowi6_flags = FLOWI_FLAG_KNOWN_NH;"));
        assert!(source.contains("dst = ip6_route_output(net, NULL, &fl6);"));
        assert!(source.contains("skb_dst_drop(skb);"));
        assert!(source.contains("skb_dst_set(skb, dst);"));
        assert!(source.contains("skb->protocol = htons(ETH_P_IPV6);"));
        assert!(source.contains("void nf_dup_ipv6"));
        assert!(source.contains("if (current->in_nf_duplicate)"));
        assert!(source.contains("skb = pskb_copy(skb, GFP_ATOMIC);"));
        assert!(source.contains("nf_reset_ct(skb);"));
        assert!(source.contains("--iph->hop_limit;"));
        assert!(source.contains("ip6_local_out(net, skb->sk, skb);"));
        assert!(source.contains("kfree_skb(skb);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(nf_dup_ipv6);"));

        let skb = DupIpv6Packet {
            hop_limit: 64,
            protocol: 0,
            dev_set: false,
            dst_set: false,
            conntrack_reset: false,
        };
        assert_eq!(
            nf_dup_ipv6(skb, NF_INET_PRE_ROUTING, false, true, false),
            DupIpv6Result::Sent(DupIpv6Packet {
                hop_limit: 63,
                protocol: ETH_P_IPV6.to_be(),
                dev_set: true,
                dst_set: true,
                conntrack_reset: true,
            })
        );
        assert_eq!(
            nf_dup_ipv6(skb, 4, true, true, false),
            DupIpv6Result::SkippedReentrant
        );
        assert_eq!(
            nf_dup_ipv6(skb, 4, false, false, false),
            DupIpv6Result::SkippedCopyFailed
        );
        assert!(matches!(
            nf_dup_ipv6(skb, NF_INET_LOCAL_IN, false, true, true),
            DupIpv6Result::Dropped(_)
        ));
    }
}
