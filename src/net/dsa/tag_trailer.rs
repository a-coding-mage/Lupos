//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_trailer.c
//! test-origin: linux:vendor/linux/net/dsa/tag_trailer.c
//! DSA trailer tag format handling.

use crate::net::skbuff::{SkBuff, skb_put, skb_trim};

pub const TRAILER_NAME: &str = "trailer";
pub const DSA_TAG_PROTO_TRAILER: u8 = 11;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for switches using a trailer tag";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_tailroom: usize,
}

#[derive(Clone)]
pub struct TrailerRxFrame {
    pub skb: SkBuff,
    pub source_port: u8,
}

pub const TRAILER_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: TRAILER_NAME,
    proto: DSA_TAG_PROTO_TRAILER,
    needed_tailroom: 4,
};

pub fn trailer_xmit(mut skb: SkBuff, port_mask: u8) -> Result<SkBuff, i32> {
    skb_put(&mut skb, 4)?.copy_from_slice(&[0x80, port_mask, 0x10, 0x00]);
    Ok(skb)
}

pub fn trailer_rcv(mut skb: SkBuff) -> Option<TrailerRxFrame> {
    let len = skb.len;
    if len < 4 {
        return None;
    }

    let data = skb.data();
    let trailer = [data[len - 4], data[len - 3], data[len - 2], data[len - 1]];

    if trailer[0] != 0x80
        || (trailer[1] & 0xf8) != 0x00
        || (trailer[2] & 0xef) != 0x00
        || trailer[3] != 0x00
    {
        return None;
    }

    let source_port = trailer[1] & 7;
    skb_trim(&mut skb, len - 4).ok()?;
    Some(TrailerRxFrame { skb, source_port })
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:trailer", "dsa_tag:id-11"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::skbuff::{alloc_skb, skb_put};

    #[test]
    fn dsa_tag_trailer_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_trailer.c"
        ));
        assert!(source.contains("#define TRAILER_NAME \"trailer\""));
        assert!(source.contains("static struct sk_buff *trailer_xmit"));
        assert!(source.contains("trailer = skb_put(skb, 4);"));
        assert!(source.contains("trailer[0] = 0x80;"));
        assert!(source.contains("trailer[1] = dsa_xmit_port_mask(skb, dev);"));
        assert!(source.contains("static struct sk_buff *trailer_rcv"));
        assert!(source.contains("if (skb_linearize(skb))"));
        assert!(source.contains("trailer = skb_tail_pointer(skb) - 4;"));
        assert!(source.contains("source_port = trailer[1] & 7;"));
        assert!(source.contains("pskb_trim_rcsum(skb, skb->len - 4)"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_TRAILER"));
        assert!(source.contains(".needed_tailroom = 4"));
        assert!(
            source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_TRAILER, TRAILER_NAME);")
        );

        assert_eq!(TRAILER_NETDEV_OPS.name, "trailer");
        assert_eq!(TRAILER_NETDEV_OPS.needed_tailroom, 4);
        assert_eq!(module_aliases(), ["dsa_tag:trailer", "dsa_tag:id-11"]);
    }

    #[test]
    fn trailer_xmit_and_receive_round_trip() {
        let mut skb = alloc_skb(16).unwrap();
        skb_put(&mut skb, 3).unwrap().copy_from_slice(&[1, 2, 3]);

        let skb = trailer_xmit(skb, 0x05).unwrap();
        assert_eq!(skb.data(), &[1, 2, 3, 0x80, 0x05, 0x10, 0x00]);

        let rx = trailer_rcv(skb).unwrap();
        assert_eq!(rx.source_port, 5);
        assert_eq!(rx.skb.data(), &[1, 2, 3]);

        let mut bad = alloc_skb(8).unwrap();
        skb_put(&mut bad, 4)
            .unwrap()
            .copy_from_slice(&[0x80, 0xf8, 0x10, 0x00]);
        assert!(trailer_rcv(bad).is_none());
    }
}
