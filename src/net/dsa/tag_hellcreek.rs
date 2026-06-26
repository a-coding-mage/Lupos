//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_hellcreek.c
//! test-origin: linux:vendor/linux/net/dsa/tag_hellcreek.c
//! DSA tag driver for Hirschmann Hellcreek switches.

use crate::net::skbuff::{SkBuff, skb_put, skb_trim};

pub const HELLCREEK_NAME: &str = "hellcreek";
pub const HELLCREEK_TAG_LEN: usize = 1;
pub const DSA_TAG_PROTO_HELLCREEK: u8 = 18;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Hirschmann Hellcreek TSN switches";
pub const MODULE_LICENSE: &str = "Dual MIT/GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_tailroom: usize,
}

#[derive(Clone)]
pub struct HellcreekRxFrame {
    pub skb: SkBuff,
    pub source_port: u8,
    pub user_dev_ifindex: u32,
    pub offload_fwd_mark: bool,
}

pub const HELLCREEK_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: HELLCREEK_NAME,
    proto: DSA_TAG_PROTO_HELLCREEK,
    needed_tailroom: HELLCREEK_TAG_LEN,
};

pub fn hellcreek_xmit(
    mut skb: SkBuff,
    port_mask: u8,
    checksum_help_failed: bool,
) -> Option<SkBuff> {
    if checksum_help_failed {
        return None;
    }
    skb_put(&mut skb, HELLCREEK_TAG_LEN).ok()?[0] = port_mask;
    Some(skb)
}

pub fn hellcreek_rcv<F>(mut skb: SkBuff, find_user: F) -> Option<HellcreekRxFrame>
where
    F: FnOnce(u8) -> Option<u32>,
{
    let tag = *skb.data().last()?;
    let source_port = tag & 0x03;
    let user_dev_ifindex = find_user(source_port)?;
    let trimmed_len = skb.len.checked_sub(HELLCREEK_TAG_LEN)?;
    skb_trim(&mut skb, trimmed_len).ok()?;
    Some(HellcreekRxFrame {
        skb,
        source_port,
        user_dev_ifindex,
        offload_fwd_mark: true,
    })
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:hellcreek", "dsa_tag:id-18"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::skbuff::{alloc_skb, skb_put};

    #[test]
    fn dsa_tag_hellcreek_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_hellcreek.c"
        ));
        assert!(source.contains("#define HELLCREEK_NAME\t\t\"hellcreek\""));
        assert!(source.contains("#define HELLCREEK_TAG_LEN\t1"));
        assert!(source.contains("static struct sk_buff *hellcreek_xmit"));
        assert!(source.contains("skb->ip_summed == CHECKSUM_PARTIAL"));
        assert!(source.contains("skb_checksum_help(skb)"));
        assert!(source.contains("tag  = skb_put(skb, HELLCREEK_TAG_LEN);"));
        assert!(source.contains("*tag = dsa_xmit_port_mask(skb, dev);"));
        assert!(source.contains("static struct sk_buff *hellcreek_rcv"));
        assert!(source.contains("u8 *tag = skb_tail_pointer(skb) - HELLCREEK_TAG_LEN;"));
        assert!(source.contains("unsigned int port = tag[0] & 0x03;"));
        assert!(source.contains("dsa_conduit_find_user(dev, 0, port);"));
        assert!(source.contains("pskb_trim_rcsum(skb, skb->len - HELLCREEK_TAG_LEN)"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains(".proto\t  = DSA_TAG_PROTO_HELLCREEK"));
        assert!(source.contains(".needed_tailroom = HELLCREEK_TAG_LEN"));
        assert!(source.contains("MODULE_LICENSE(\"Dual MIT/GPL\");"));

        assert_eq!(HELLCREEK_NETDEV_OPS.name, "hellcreek");
        assert_eq!(HELLCREEK_NETDEV_OPS.proto, DSA_TAG_PROTO_HELLCREEK);
        assert_eq!(HELLCREEK_NETDEV_OPS.needed_tailroom, HELLCREEK_TAG_LEN);
        assert_eq!(module_aliases(), ["dsa_tag:hellcreek", "dsa_tag:id-18"]);
    }

    #[test]
    fn hellcreek_tag_round_trip_tracks_low_two_port_bits() {
        let mut skb = alloc_skb(8).unwrap();
        skb_put(&mut skb, 3).unwrap().copy_from_slice(&[1, 2, 3]);
        let skb = hellcreek_xmit(skb, 0b0000_0110, false).unwrap();
        assert_eq!(skb.data(), &[1, 2, 3, 0b0000_0110]);

        let rx = hellcreek_rcv(skb, |port| Some(100 + port as u32)).unwrap();
        assert_eq!(rx.source_port, 2);
        assert_eq!(rx.user_dev_ifindex, 102);
        assert!(rx.offload_fwd_mark);
        assert_eq!(rx.skb.data(), &[1, 2, 3]);

        let mut skb = alloc_skb(4).unwrap();
        skb_put(&mut skb, 1).unwrap()[0] = 1;
        assert!(hellcreek_rcv(skb, |_| None).is_none());

        let skb = alloc_skb(2).unwrap();
        assert!(hellcreek_xmit(skb, 1, true).is_none());
    }
}
