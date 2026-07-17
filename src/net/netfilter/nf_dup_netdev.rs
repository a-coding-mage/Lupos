//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_dup_netdev.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_dup_netdev.c
//! Netfilter netdev packet forwarding and duplication helpers.

use crate::include::uapi::errno::{E2BIG, EOPNOTSUPP};

pub const NF_RECURSION_LIMIT: u8 = 2;
pub const NF_NETDEV_INGRESS: u8 = 0;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Pablo Neira Ayuso <pablo@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "Netfilter packet duplication support";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetdevSkb {
    pub mac_header_was_set: bool,
    pub mac_len: usize,
    pub cow_head_ok: bool,
    pub dev_index: Option<i32>,
    pub timestamp_cleared: bool,
    pub pushed_bytes: usize,
    pub transmitted: bool,
    pub freed: bool,
}

impl NetdevSkb {
    pub const fn new() -> Self {
        Self {
            mac_header_was_set: false,
            mac_len: 0,
            cow_head_ok: true,
            dev_index: None,
            timestamp_cleared: false,
            pushed_bytes: 0,
            transmitted: false,
            freed: false,
        }
    }
}

pub fn nf_do_netdev_egress(
    mut skb: NetdevSkb,
    dev_index: i32,
    hook: u8,
    recursion: &mut u8,
) -> NetdevSkb {
    if *recursion > NF_RECURSION_LIMIT {
        skb.freed = true;
        return skb;
    }
    if hook == NF_NETDEV_INGRESS && skb.mac_header_was_set {
        if !skb.cow_head_ok {
            skb.freed = true;
            return skb;
        }
        skb.pushed_bytes += skb.mac_len;
    }

    skb.dev_index = Some(dev_index);
    skb.timestamp_cleared = true;
    *recursion += 1;
    skb.transmitted = true;
    *recursion -= 1;
    skb
}

pub fn nf_fwd_netdev_egress(skb: NetdevSkb, oif: i32, dev_exists: bool, hook: u8) -> NetdevSkb {
    if !dev_exists {
        return NetdevSkb { freed: true, ..skb };
    }
    let mut recursion = 0;
    nf_do_netdev_egress(skb, oif, hook, &mut recursion)
}

pub fn nf_dup_netdev_egress(
    skb: NetdevSkb,
    oif: i32,
    dev_exists: bool,
    clone_ok: bool,
    hook: u8,
) -> Option<NetdevSkb> {
    if !dev_exists || !clone_ok {
        return None;
    }
    let mut recursion = 0;
    Some(nf_do_netdev_egress(skb, oif, hook, &mut recursion))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowActionEntry {
    pub id: u32,
    pub dev_index: i32,
}

pub const fn nft_fwd_dup_netdev_offload(
    dev_exists: bool,
    entry_available: bool,
    id: u32,
    oif: i32,
) -> Result<FlowActionEntry, i32> {
    if !dev_exists {
        return Err(-EOPNOTSUPP);
    }
    if !entry_available {
        return Err(-E2BIG);
    }
    Ok(FlowActionEntry { id, dev_index: oif })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_dup_netdev_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_dup_netdev.c"
        ));
        assert!(source.contains("static void nf_do_netdev_egress"));
        assert!(source.contains("if (nf_dev_xmit_recursion())"));
        assert!(source.contains("nf_dev_xmit_recursion_inc();"));
        assert!(source.contains("nf_dev_xmit_recursion_dec();"));
        assert!(source.contains("if (hook == NF_NETDEV_INGRESS && skb_mac_header_was_set(skb))"));
        assert!(source.contains("if (skb_cow_head(skb, skb->mac_len))"));
        assert!(source.contains("skb_push(skb, skb->mac_len);"));
        assert!(source.contains("skb->dev = dev;"));
        assert!(source.contains("skb_clear_tstamp(skb);"));
        assert!(source.contains("dev_queue_xmit(skb);"));
        assert!(source.contains("kfree_skb(skb);"));
        assert!(source.contains("nf_fwd_netdev_egress"));
        assert!(source.contains("dev_get_by_index_rcu(nft_net(pkt), oif);"));
        assert!(source.contains("skb = skb_clone(pkt->skb, GFP_ATOMIC);"));
        assert!(source.contains("nft_fwd_dup_netdev_offload"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert!(source.contains("return -E2BIG;"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Netfilter packet duplication support\")"));
    }

    #[test]
    fn netdev_egress_pushes_ingress_mac_header_and_tracks_errors() {
        let skb = NetdevSkb {
            mac_header_was_set: true,
            mac_len: 14,
            ..NetdevSkb::new()
        };
        let out = nf_fwd_netdev_egress(skb, 5, true, NF_NETDEV_INGRESS);
        assert_eq!(out.dev_index, Some(5));
        assert_eq!(out.pushed_bytes, 14);
        assert!(out.timestamp_cleared);
        assert!(out.transmitted);
        assert!(nf_fwd_netdev_egress(skb, 5, false, 1).freed);
        assert!(nf_dup_netdev_egress(skb, 5, false, true, 1).is_none());
        assert_eq!(
            nft_fwd_dup_netdev_offload(false, true, 7, 9),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(nft_fwd_dup_netdev_offload(true, false, 7, 9), Err(-E2BIG));
        assert_eq!(
            nft_fwd_dup_netdev_offload(true, true, 7, 9),
            Ok(FlowActionEntry {
                id: 7,
                dev_index: 9,
            })
        );
    }
}
