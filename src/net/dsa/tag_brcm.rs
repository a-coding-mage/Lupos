//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_brcm.c
//! test-origin: linux:vendor/linux/net/dsa/tag_brcm.c
//! Broadcom DSA in-frame tag formats.

use crate::lib::crc::crc32_main::crc32_le;

pub const BRCM_NAME: &str = "brcm";
pub const BRCM_LEGACY_NAME: &str = "brcm-legacy";
pub const BRCM_LEGACY_FCS_NAME: &str = "brcm-legacy-fcs";
pub const BRCM_PREPEND_NAME: &str = "brcm-prepend";
pub const BRCM_LEG_TAG_LEN: usize = 6;
pub const BRCM_LEG_TYPE_HI: u8 = 0x88;
pub const BRCM_LEG_TYPE_LO: u8 = 0x74;
pub const BRCM_LEG_UNICAST: u8 = 0 << 5;
pub const BRCM_LEG_MULTICAST: u8 = 1 << 5;
pub const BRCM_LEG_EGRESS: u8 = 2 << 5;
pub const BRCM_LEG_INGRESS: u8 = 3 << 5;
pub const BRCM_LEG_PORT_ID: u8 = 0x0f;
pub const BRCM_TAG_LEN: usize = 4;
pub const BRCM_OPCODE_SHIFT: u8 = 5;
pub const BRCM_OPCODE_MASK: u8 = 0x07;
pub const BRCM_IG_TC_SHIFT: u8 = 2;
pub const BRCM_IG_TC_MASK: u16 = 0x07;
pub const BRCM_IG_DSTMAP2_MASK: u8 = 1;
pub const BRCM_IG_DSTMAP1_MASK: u16 = 0xff;
pub const BRCM_EG_RC_RSVD: u8 = 3 << 6;
pub const BRCM_EG_PID_MASK: u8 = 0x1f;
pub const ETH_ZLEN: usize = 60;
pub const ETH_FCS_LEN: usize = 4;
pub const VLAN_HLEN: usize = 4;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const DSA_TAG_PROTO_BRCM_VALUE: u8 = 1;
pub const DSA_TAG_PROTO_BRCM_PREPEND_VALUE: u8 = 2;
pub const DSA_TAG_PROTO_BRCM_LEGACY_VALUE: u8 = 22;
pub const DSA_TAG_PROTO_BRCM_LEGACY_FCS_VALUE: u8 = 29;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Broadcom switches using in-frame headers";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrcmTx {
    pub tag: [u8; BRCM_TAG_LEN],
    pub offset: usize,
    pub queue_mapping: u16,
    pub min_len_with_tag: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrcmLegacyTx {
    pub tag: [u8; BRCM_LEG_TAG_LEN],
    pub min_len_with_tag: usize,
    pub fcs: Option<[u8; ETH_FCS_LEN]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrcmRx {
    pub source_port: u8,
    pub strip_len: usize,
    pub offload_fwd_mark: bool,
}

pub const BRCM_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: BRCM_NAME,
    proto: DSA_TAG_PROTO_BRCM_VALUE,
    needed_headroom: BRCM_TAG_LEN,
};
pub const BRCM_LEGACY_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: BRCM_LEGACY_NAME,
    proto: DSA_TAG_PROTO_BRCM_LEGACY_VALUE,
    needed_headroom: BRCM_LEG_TAG_LEN,
};
pub const BRCM_LEGACY_FCS_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: BRCM_LEGACY_FCS_NAME,
    proto: DSA_TAG_PROTO_BRCM_LEGACY_FCS_VALUE,
    needed_headroom: BRCM_LEG_TAG_LEN,
};
pub const BRCM_PREPEND_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: BRCM_PREPEND_NAME,
    proto: DSA_TAG_PROTO_BRCM_PREPEND_VALUE,
    needed_headroom: BRCM_TAG_LEN,
};

pub const fn brcm_tag_set_port_queue(port: u8, queue: u16) -> u16 {
    ((port as u16) << 8) | queue
}

pub const fn brcm_tag_get_port(value: u16) -> u8 {
    (value >> 8) as u8
}

pub const fn brcm_tag_get_queue(value: u16) -> u8 {
    (value & 0xff) as u8
}

pub const fn brcm_leg_len_hi(len: usize) -> u8 {
    ((len >> 8) & 0x07) as u8
}

pub const fn brcm_leg_len_lo(len: usize) -> u8 {
    (len & 0xff) as u8
}

pub const fn brcm_tag_xmit_ll(port_index: u8, queue: u16, port_mask: u16, offset: usize) -> BrcmTx {
    BrcmTx {
        tag: [
            (1 << BRCM_OPCODE_SHIFT) | (((queue & BRCM_IG_TC_MASK) as u8) << BRCM_IG_TC_SHIFT),
            0,
            ((port_mask >> 8) as u8) & BRCM_IG_DSTMAP2_MASK,
            (port_mask & BRCM_IG_DSTMAP1_MASK) as u8,
        ],
        offset,
        queue_mapping: brcm_tag_set_port_queue(port_index, queue),
        min_len_with_tag: ETH_ZLEN + BRCM_TAG_LEN,
    }
}

pub const fn brcm_tag_xmit(port_index: u8, queue: u16, port_mask: u16) -> BrcmTx {
    brcm_tag_xmit_ll(port_index, queue, port_mask, 12)
}

pub const fn brcm_tag_xmit_prepend(port_index: u8, queue: u16, port_mask: u16) -> BrcmTx {
    brcm_tag_xmit_ll(port_index, queue, port_mask, 0)
}

pub const fn brcm_tag_rcv_ll(
    brcm_tag: [u8; BRCM_TAG_LEN],
    user_found: bool,
    link_local_dest: bool,
) -> Option<BrcmRx> {
    if ((brcm_tag[0] >> BRCM_OPCODE_SHIFT) & BRCM_OPCODE_MASK) != 0 {
        return None;
    }
    if (brcm_tag[2] & BRCM_EG_RC_RSVD) != 0 {
        return None;
    }
    if !user_found {
        return None;
    }
    Some(BrcmRx {
        source_port: brcm_tag[3] & BRCM_EG_PID_MASK,
        strip_len: BRCM_TAG_LEN,
        offload_fwd_mark: !link_local_dest,
    })
}

pub const fn brcm_leg_tag_xmit(port_index: u8) -> BrcmLegacyTx {
    BrcmLegacyTx {
        tag: [
            BRCM_LEG_TYPE_HI,
            BRCM_LEG_TYPE_LO,
            BRCM_LEG_EGRESS,
            0,
            0,
            port_index & BRCM_LEG_PORT_ID,
        ],
        min_len_with_tag: ETH_ZLEN + BRCM_LEG_TAG_LEN,
        fcs: None,
    }
}

pub fn brcm_leg_fcs_tag_xmit(port_index: u8, frame: &[u8]) -> BrcmLegacyTx {
    let fcs_len = frame.len();
    let fcs_val = crc32_le(!0, frame) ^ !0;
    BrcmLegacyTx {
        tag: [
            BRCM_LEG_TYPE_HI,
            BRCM_LEG_TYPE_LO,
            BRCM_LEG_EGRESS | brcm_leg_len_hi(fcs_len),
            brcm_leg_len_lo(fcs_len),
            0,
            port_index & BRCM_LEG_PORT_ID,
        ],
        min_len_with_tag: ETH_ZLEN + BRCM_LEG_TAG_LEN,
        fcs: Some(fcs_val.to_le_bytes()),
    }
}

pub const fn brcm_leg_tag_rcv(
    brcm_tag: [u8; BRCM_LEG_TAG_LEN],
    following_proto: u16,
    following_tci: u16,
    user_found: bool,
    link_local_dest: bool,
) -> Option<BrcmRx> {
    if !user_found {
        return None;
    }
    let vlan_zero = following_proto == ETH_P_8021Q && following_tci == 0;
    Some(BrcmRx {
        source_port: brcm_tag[5] & BRCM_LEG_PORT_ID,
        strip_len: if vlan_zero {
            BRCM_LEG_TAG_LEN + VLAN_HLEN
        } else {
            BRCM_LEG_TAG_LEN
        },
        offload_fwd_mark: !link_local_dest,
    })
}

pub fn module_aliases() -> [&'static str; 8] {
    [
        "dsa_tag:brcm",
        "dsa_tag:id-1",
        "dsa_tag:brcm-legacy",
        "dsa_tag:id-22",
        "dsa_tag:brcm-legacy-fcs",
        "dsa_tag:id-29",
        "dsa_tag:brcm-prepend",
        "dsa_tag:id-2",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_brcm_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_brcm.c"
        ));
        let brcm = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dsa/brcm.h"
        ));
        assert!(source.contains("#define BRCM_NAME\t\t\"brcm\""));
        assert!(source.contains("#define BRCM_LEG_TAG_LEN\t6"));
        assert!(source.contains("#define BRCM_TAG_LEN\t4"));
        assert!(source.contains("#define BRCM_OPCODE_SHIFT\t5"));
        assert!(source.contains("brcm_tag[0] = (1 << BRCM_OPCODE_SHIFT)"));
        assert!(source.contains("port_mask = dsa_xmit_port_mask(skb, dev);"));
        assert!(source.contains("skb_set_queue_mapping(skb, BRCM_TAG_SET_PORT_QUEUE"));
        assert!(source.contains("if (unlikely((brcm_tag[0] >> BRCM_OPCODE_SHIFT)"));
        assert!(source.contains("if (unlikely(brcm_tag[2] & BRCM_EG_RC_RSVD))"));
        assert!(source.contains("source_port = brcm_tag[3] & BRCM_EG_PID_MASK;"));
        assert!(source.contains("brcm_tag[2] = BRCM_LEG_EGRESS;"));
        assert!(source.contains("brcm_tag[5] = dp->index & BRCM_LEG_PORT_ID;"));
        assert!(source.contains("fcs_val = cpu_to_le32(crc32_le(~0, skb->data, fcs_len) ^ ~0);"));
        assert!(source.contains("if (proto[0] == htons(ETH_P_8021Q) && proto[1] == 0)"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_BRCM"));
        assert!(source.contains(".proto = DSA_TAG_PROTO_BRCM_LEGACY_FCS"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_BRCM_PREPEND"));
        assert!(brcm.contains("#define BRCM_TAG_SET_PORT_QUEUE(p, q)\t((p) << 8 | q)"));
    }

    #[test]
    fn broadcom_ingress_and_egress_tags_round_trip_fields() {
        let tx = brcm_tag_xmit(4, 6, 0x0102);
        assert_eq!(tx.tag, [0x20 | (6 << 2), 0, 1, 2]);
        assert_eq!(tx.offset, 12);
        assert_eq!(brcm_tag_get_port(tx.queue_mapping), 4);
        assert_eq!(brcm_tag_get_queue(tx.queue_mapping), 6);

        let rx = brcm_tag_rcv_ll([0, 0, 0, 0x13], true, false).unwrap();
        assert_eq!(rx.source_port, 0x13);
        assert_eq!(rx.strip_len, BRCM_TAG_LEN);
        assert!(rx.offload_fwd_mark);
        assert!(brcm_tag_rcv_ll([0x20, 0, 0, 1], true, false).is_none());
        assert!(brcm_tag_rcv_ll([0, 0, BRCM_EG_RC_RSVD, 1], true, false).is_none());
    }

    #[test]
    fn legacy_tags_match_port_vlan_and_fcs_rules() {
        let legacy = brcm_leg_tag_xmit(17);
        assert_eq!(
            legacy.tag,
            [BRCM_LEG_TYPE_HI, BRCM_LEG_TYPE_LO, BRCM_LEG_EGRESS, 0, 0, 1]
        );

        let fcs = brcm_leg_fcs_tag_xmit(3, b"123456789");
        assert_eq!(fcs.tag[2], BRCM_LEG_EGRESS);
        assert_eq!(fcs.tag[3], 9);
        assert_eq!(fcs.fcs, Some(0xcbf4_3926u32.to_le_bytes()));

        let rx = brcm_leg_tag_rcv(legacy.tag, ETH_P_8021Q, 0, true, true).unwrap();
        assert_eq!(rx.source_port, 1);
        assert_eq!(rx.strip_len, BRCM_LEG_TAG_LEN + VLAN_HLEN);
        assert!(!rx.offload_fwd_mark);
        assert_eq!(
            module_aliases(),
            [
                "dsa_tag:brcm",
                "dsa_tag:id-1",
                "dsa_tag:brcm-legacy",
                "dsa_tag:id-22",
                "dsa_tag:brcm-legacy-fcs",
                "dsa_tag:id-29",
                "dsa_tag:brcm-prepend",
                "dsa_tag:id-2"
            ]
        );
    }
}
