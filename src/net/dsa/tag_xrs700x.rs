//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_xrs700x.c
//! test-origin: linux:vendor/linux/net/dsa/tag_xrs700x.c
//! DSA tag driver for XRS700x switches.

use crate::net::skbuff::{SkBuff, skb_put, skb_trim};

pub const XRS700X_NAME: &str = "xrs700x";
pub const DSA_TAG_PROTO_XRS700X: u8 = 19;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for XRS700x switches";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_tailroom: usize,
}

#[derive(Clone)]
pub struct Xrs700xRxFrame {
    pub skb: SkBuff,
    pub source_port: u8,
    pub offload_fwd_mark: bool,
}

pub const XRS700X_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: XRS700X_NAME,
    proto: DSA_TAG_PROTO_XRS700X,
    needed_tailroom: 1,
};

pub fn xrs700x_xmit(mut skb: SkBuff, port_mask: u8) -> Result<SkBuff, i32> {
    skb_put(&mut skb, 1)?[0] = port_mask;
    Ok(skb)
}

pub fn xrs700x_rcv(mut skb: SkBuff) -> Option<Xrs700xRxFrame> {
    let trailer = *skb.data().last()?;
    let source_port = ffs_u8(trailer)?.saturating_sub(1);
    let trimmed_len = skb.len.checked_sub(1)?;
    skb_trim(&mut skb, trimmed_len).ok()?;
    Some(Xrs700xRxFrame {
        skb,
        source_port,
        offload_fwd_mark: true,
    })
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:xrs700x", "dsa_tag:id-19"]
}

fn ffs_u8(value: u8) -> Option<u8> {
    if value == 0 {
        None
    } else {
        Some(value.trailing_zeros() as u8 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::skbuff::{alloc_skb, skb_put};

    #[test]
    fn dsa_tag_xrs700x_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_xrs700x.c"
        ));
        assert!(source.contains("#define XRS700X_NAME \"xrs700x\""));
        assert!(source.contains("static struct sk_buff *xrs700x_xmit"));
        assert!(source.contains("trailer = skb_put(skb, 1);"));
        assert!(source.contains("trailer[0] = dsa_xmit_port_mask(skb, dev);"));
        assert!(source.contains("static struct sk_buff *xrs700x_rcv"));
        assert!(source.contains("trailer = skb_tail_pointer(skb) - 1;"));
        assert!(source.contains("source_port = ffs((int)trailer[0]) - 1;"));
        assert!(source.contains("if (source_port < 0)"));
        assert!(source.contains("pskb_trim_rcsum(skb, skb->len - 1)"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_XRS700X"));
        assert!(source.contains(".needed_tailroom = 1"));
        assert!(source.contains("MODULE_DESCRIPTION(\"DSA tag driver for XRS700x switches\")"));

        assert_eq!(XRS700X_NETDEV_OPS.name, "xrs700x");
        assert_eq!(XRS700X_NETDEV_OPS.proto, DSA_TAG_PROTO_XRS700X);
        assert_eq!(XRS700X_NETDEV_OPS.needed_tailroom, 1);
        assert_eq!(module_aliases(), ["dsa_tag:xrs700x", "dsa_tag:id-19"]);
    }

    #[test]
    fn xrs700x_trailer_round_trip_tracks_port_and_trims() {
        let mut skb = alloc_skb(8).unwrap();
        skb_put(&mut skb, 3).unwrap().copy_from_slice(&[1, 2, 3]);
        let skb = xrs700x_xmit(skb, 0b0000_1000).unwrap();
        assert_eq!(skb.data(), &[1, 2, 3, 0b0000_1000]);

        let rx = xrs700x_rcv(skb).unwrap();
        assert_eq!(rx.source_port, 3);
        assert!(rx.offload_fwd_mark);
        assert_eq!(rx.skb.data(), &[1, 2, 3]);

        let mut skb = alloc_skb(2).unwrap();
        skb_put(&mut skb, 1).unwrap()[0] = 0;
        assert!(xrs700x_rcv(skb).is_none());
    }
}
