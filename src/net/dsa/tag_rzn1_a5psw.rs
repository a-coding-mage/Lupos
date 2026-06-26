//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_rzn1_a5psw.c
//! test-origin: linux:vendor/linux/net/dsa/tag_rzn1_a5psw.c
//! Renesas RZ/N1 A5PSW DSA tag handling.

pub const A5PSW_NAME: &str = "a5psw";
pub const ETH_P_DSA_A5PSW: u16 = 0xe001;
pub const A5PSW_TAG_LEN: usize = 8;
pub const A5PSW_CTRL_DATA_FORCE_FORWARD: u16 = 1;
pub const A5PSW_CTRL_DATA_PORT: u16 = 0x000f;
pub const ETH_ZLEN: usize = 60;
pub const DSA_TAG_PROTO_RZN1_A5PSW_VALUE: u8 = 26;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Renesas RZ/N1 A5PSW switch";
pub const MODULE_LICENSE: &str = "GPL v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct A5pswTag {
    pub ctrl_tag: u16,
    pub ctrl_data: u16,
    pub ctrl_data2_hi: u16,
    pub ctrl_data2_lo: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct A5pswTxFrame {
    pub tag: A5pswTag,
    pub padded_len: usize,
    pub etype_header_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct A5pswRxFrame {
    pub source_port: u16,
    pub tag_removed: bool,
    pub etype_header_stripped: bool,
    pub offload_fwd_mark: bool,
}

pub const A5PSW_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: A5PSW_NAME,
    proto: DSA_TAG_PROTO_RZN1_A5PSW_VALUE,
    needed_headroom: A5PSW_TAG_LEN,
};

const fn field_prep_u16(mask: u16, value: u16) -> u16 {
    (value << mask.trailing_zeros()) & mask
}

const fn field_get_u16(mask: u16, value: u16) -> u16 {
    (value & mask) >> mask.trailing_zeros()
}

pub const fn a5psw_tag_xmit(
    skb_len: usize,
    port_mask: u16,
    skb_put_padto_failed: bool,
) -> Option<A5pswTxFrame> {
    if skb_put_padto_failed {
        return None;
    }

    let padded_len = if skb_len < ETH_ZLEN {
        ETH_ZLEN
    } else {
        skb_len
    };
    Some(A5pswTxFrame {
        tag: A5pswTag {
            ctrl_tag: ETH_P_DSA_A5PSW.to_be(),
            ctrl_data: A5PSW_CTRL_DATA_FORCE_FORWARD.to_be(),
            ctrl_data2_hi: 0,
            ctrl_data2_lo: field_prep_u16(A5PSW_CTRL_DATA_PORT, port_mask).to_be(),
        },
        padded_len,
        etype_header_allocated: true,
    })
}

pub const fn a5psw_tag_rcv(tag: Option<A5pswTag>, user_port_exists: bool) -> Option<A5pswRxFrame> {
    let tag = match tag {
        Some(tag) => tag,
        None => return None,
    };
    if tag.ctrl_tag != ETH_P_DSA_A5PSW.to_be() {
        return None;
    }
    let source_port = field_get_u16(A5PSW_CTRL_DATA_PORT, u16::from_be(tag.ctrl_data));
    if !user_port_exists {
        return None;
    }
    Some(A5pswRxFrame {
        source_port,
        tag_removed: true,
        etype_header_stripped: true,
        offload_fwd_mark: true,
    })
}

pub const fn a5psw_module_ops() -> &'static DsaDeviceOps {
    &A5PSW_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:a5psw", "dsa_tag:id-26"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_rzn1_a5psw_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_rzn1_a5psw.c"
        ));
        assert!(source.contains("#define A5PSW_NAME\t\t\t\"a5psw\""));
        assert!(source.contains("#define ETH_P_DSA_A5PSW\t\t\t0xE001"));
        assert!(source.contains("#define A5PSW_TAG_LEN\t\t\t8"));
        assert!(source.contains("BUILD_BUG_ON(sizeof(*ptag) != A5PSW_TAG_LEN);"));
        assert!(source.contains("if (__skb_put_padto(skb, ETH_ZLEN, false))"));
        assert!(source.contains("skb_push(skb, A5PSW_TAG_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, A5PSW_TAG_LEN);"));
        assert!(source.contains(
            "data2_val = FIELD_PREP(A5PSW_CTRL_DATA_PORT, dsa_xmit_port_mask(skb, dev));"
        ));
        assert!(source.contains("ptag->ctrl_tag = htons(ETH_P_DSA_A5PSW);"));
        assert!(source.contains("ptag->ctrl_data = htons(A5PSW_CTRL_DATA_FORCE_FORWARD);"));
        assert!(source.contains("ptag->ctrl_data2_lo = htons(data2_val);"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, A5PSW_TAG_LEN)))"));
        assert!(source.contains("if (tag->ctrl_tag != htons(ETH_P_DSA_A5PSW))"));
        assert!(source.contains("port = FIELD_GET(A5PSW_CTRL_DATA_PORT, ntohs(tag->ctrl_data));"));
        assert!(source.contains("skb_pull_rcsum(skb, A5PSW_TAG_LEN);"));
        assert!(source.contains("dsa_strip_etype_header(skb, A5PSW_TAG_LEN);"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_RZN1_A5PSW"));
        assert!(source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_A5PSW, A5PSW_NAME);"));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        assert!(dsa.contains("#define DSA_TAG_PROTO_RZN1_A5PSW_VALUE\t\t26"));
    }

    #[test]
    fn a5psw_xmit_pads_and_receive_uses_ctrl_data_port_bits() {
        let tx = a5psw_tag_xmit(42, 7, false).unwrap();
        assert_eq!(tx.padded_len, ETH_ZLEN);
        assert_eq!(tx.tag.ctrl_tag, ETH_P_DSA_A5PSW.to_be());
        assert_eq!(tx.tag.ctrl_data, A5PSW_CTRL_DATA_FORCE_FORWARD.to_be());
        assert_eq!(tx.tag.ctrl_data2_lo, 7u16.to_be());
        assert!(tx.etype_header_allocated);
        assert_eq!(a5psw_tag_xmit(42, 7, true), None);

        let rx = a5psw_tag_rcv(
            Some(A5pswTag {
                ctrl_tag: ETH_P_DSA_A5PSW.to_be(),
                ctrl_data: 4u16.to_be(),
                ctrl_data2_hi: 0,
                ctrl_data2_lo: 0,
            }),
            true,
        )
        .unwrap();
        assert_eq!(rx.source_port, 4);
        assert!(rx.offload_fwd_mark);
        assert_eq!(
            a5psw_tag_rcv(
                Some(A5pswTag {
                    ctrl_tag: 0,
                    ctrl_data: 4u16.to_be(),
                    ctrl_data2_hi: 0,
                    ctrl_data2_lo: 0,
                }),
                true
            ),
            None
        );
        assert_eq!(
            a5psw_tag_rcv(
                Some(A5pswTag {
                    ctrl_tag: ETH_P_DSA_A5PSW.to_be(),
                    ctrl_data: 4u16.to_be(),
                    ctrl_data2_hi: 0,
                    ctrl_data2_lo: 0,
                }),
                false
            ),
            None
        );
        assert_eq!(a5psw_module_ops(), &A5PSW_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:a5psw", "dsa_tag:id-26"]);
    }
}
