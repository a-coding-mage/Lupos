//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_mtk.c
//! test-origin: linux:vendor/linux/net/dsa/tag_mtk.c
//! Mediatek DSA tag support.

pub const MTK_NAME: &str = "mtk";
pub const MTK_HDR_LEN: usize = 4;
pub const MTK_HDR_XMIT_UNTAGGED: u8 = 0;
pub const MTK_HDR_XMIT_TAGGED_TPID_8100: u8 = 1;
pub const MTK_HDR_XMIT_TAGGED_TPID_88A8: u8 = 2;
pub const MTK_HDR_RECV_SOURCE_PORT_MASK: u16 = 0x0007;
pub const MTK_HDR_XMIT_DP_BIT_MASK: u8 = 0x3f;
pub const MTK_HDR_XMIT_SA_DIS: u8 = 1 << 6;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_8021AD: u16 = 0x88a8;
pub const ETH_P_8021Q_BE: u16 = ETH_P_8021Q.to_be();
pub const ETH_P_8021AD_BE: u16 = ETH_P_8021AD.to_be();
pub const DSA_TAG_PROTO_MTK_VALUE: u8 = 9;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Mediatek switches";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MtkTxFrame {
    pub tag: [u8; MTK_HDR_LEN],
    pub queue_mapping: u8,
    pub etype_header_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MtkRxFrame {
    pub source_port: u8,
    pub tag_removed: bool,
    pub etype_header_stripped: bool,
    pub offload_fwd_mark: bool,
}

pub const MTK_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: MTK_NAME,
    proto: DSA_TAG_PROTO_MTK_VALUE,
    needed_headroom: MTK_HDR_LEN,
};

const fn field_prep_u8(mask: u8, value: u8) -> u8 {
    (value << mask.trailing_zeros()) & mask
}

pub const fn mtk_tag_xmit(
    skb_protocol_be: u16,
    dp_index: u8,
    port_mask: u8,
    tag_control: [u8; 2],
) -> MtkTxFrame {
    let xmit_tpid = match skb_protocol_be {
        ETH_P_8021Q_BE => MTK_HDR_XMIT_TAGGED_TPID_8100,
        ETH_P_8021AD_BE => MTK_HDR_XMIT_TAGGED_TPID_88A8,
        _ => MTK_HDR_XMIT_UNTAGGED,
    };

    let mut tag = [
        xmit_tpid,
        field_prep_u8(MTK_HDR_XMIT_DP_BIT_MASK, port_mask),
        tag_control[0],
        tag_control[1],
    ];
    let etype_header_allocated = xmit_tpid == MTK_HDR_XMIT_UNTAGGED;
    if etype_header_allocated {
        tag[2] = 0;
        tag[3] = 0;
    }

    MtkTxFrame {
        tag,
        queue_mapping: dp_index,
        etype_header_allocated,
    }
}

pub const fn mtk_tag_rcv(
    tag: Option<[u8; MTK_HDR_LEN]>,
    user_port_exists: bool,
) -> Option<MtkRxFrame> {
    let tag = match tag {
        Some(tag) => tag,
        None => return None,
    };
    let hdr = u16::from_be_bytes([tag[0], tag[1]]);
    let source_port = (hdr & MTK_HDR_RECV_SOURCE_PORT_MASK) as u8;

    if !user_port_exists {
        return None;
    }

    Some(MtkRxFrame {
        source_port,
        tag_removed: true,
        etype_header_stripped: true,
        offload_fwd_mark: true,
    })
}

pub const fn mtk_module_ops() -> &'static DsaDeviceOps {
    &MTK_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:mtk", "dsa_tag:id-9"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_mtk_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_mtk.c"
        ));
        assert!(source.contains("#define MTK_NAME\t\t\"mtk\""));
        assert!(source.contains("#define MTK_HDR_LEN\t\t4"));
        assert!(source.contains("#define MTK_HDR_XMIT_UNTAGGED\t\t0"));
        assert!(source.contains("#define MTK_HDR_XMIT_TAGGED_TPID_8100\t1"));
        assert!(source.contains("#define MTK_HDR_XMIT_TAGGED_TPID_88A8\t2"));
        assert!(source.contains("skb_set_queue_mapping(skb, dp->index);"));
        assert!(source.contains("case htons(ETH_P_8021Q):"));
        assert!(source.contains("case htons(ETH_P_8021AD):"));
        assert!(source.contains("skb_push(skb, MTK_HDR_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, MTK_HDR_LEN);"));
        assert!(source.contains("mtk_tag[0] = xmit_tpid;"));
        assert!(source.contains("FIELD_PREP(MTK_HDR_XMIT_DP_BIT_MASK,"));
        assert!(source.contains("if (xmit_tpid == MTK_HDR_XMIT_UNTAGGED)"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, MTK_HDR_LEN)))"));
        assert!(source.contains("hdr = ntohs(*phdr);"));
        assert!(source.contains("skb_pull_rcsum(skb, MTK_HDR_LEN);"));
        assert!(source.contains("dsa_strip_etype_header(skb, MTK_HDR_LEN);"));
        assert!(source.contains("port = (hdr & MTK_HDR_RECV_SOURCE_PORT_MASK);"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains(".proto\t\t= DSA_TAG_PROTO_MTK"));
        assert!(source.contains(".needed_headroom = MTK_HDR_LEN"));
        assert!(source.contains("MODULE_DESCRIPTION(\"DSA tag driver for Mediatek switches\")"));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        assert!(dsa.contains("#define DSA_TAG_PROTO_MTK_VALUE\t\t\t9"));
    }

    #[test]
    fn mtk_xmit_and_receive_follow_header_modes() {
        let untagged = mtk_tag_xmit(0x0800u16.to_be(), 3, 0x25, [0xaa, 0xbb]);
        assert_eq!(untagged.queue_mapping, 3);
        assert_eq!(untagged.tag, [0, 0x25, 0, 0]);
        assert!(untagged.etype_header_allocated);

        let vlan = mtk_tag_xmit(ETH_P_8021Q_BE, 2, 0x3f, [0x12, 0x34]);
        assert_eq!(vlan.tag, [1, 0x3f, 0x12, 0x34]);
        assert!(!vlan.etype_header_allocated);

        let rx = mtk_tag_rcv(Some([0, 5, 0, 0]), true).unwrap();
        assert_eq!(rx.source_port, 5);
        assert!(rx.tag_removed);
        assert!(rx.etype_header_stripped);
        assert!(rx.offload_fwd_mark);
        assert_eq!(mtk_tag_rcv(None, true), None);
        assert_eq!(mtk_tag_rcv(Some([0, 1, 0, 0]), false), None);
        assert_eq!(mtk_module_ops(), &MTK_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:mtk", "dsa_tag:id-9"]);
    }
}
