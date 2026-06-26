//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_mxl-gsw1xx.c
//! test-origin: linux:vendor/linux/net/dsa/tag_mxl-gsw1xx.c
//! DSA special tag support for MaxLinear GSW1xx switch chips.

pub const GSW1XX_TAG_NAME: &str = "gsw1xx";
pub const GSW1XX_HEADER_LEN: usize = 8;
pub const ETH_P_MXLGSW: u16 = 0x88c3;
pub const GSW1XX_TX_PORT_MAP: u16 = 0x00ff;
pub const GSW1XX_TX_PORT_MAP_EN: u16 = 1 << 15;
pub const GSW1XX_TX_CLASS_EN: u16 = 1 << 14;
pub const GSW1XX_TX_TIME_STAMP_EN: u16 = 1 << 13;
pub const GSW1XX_TX_LRN_DIS: u16 = 1 << 12;
pub const GSW1XX_TX_CLASS: u16 = 0x0f00;
pub const GSW1XX_RX_PORT_MAP: u16 = 0xff00;
pub const DSA_TAG_PROTO_MXL_GSW1XX_VALUE: u8 = 31;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for MaxLinear GSW1xx 8 byte protocol";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Gsw1xxRxFrame {
    pub source_port: u16,
    pub tag_removed: bool,
    pub etype_header_stripped: bool,
}

pub const GSW1XX_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: GSW1XX_TAG_NAME,
    proto: DSA_TAG_PROTO_MXL_GSW1XX_VALUE,
    needed_headroom: GSW1XX_HEADER_LEN,
};

const fn field_prep_u16(mask: u16, value: u16) -> u16 {
    (value << mask.trailing_zeros()) & mask
}

const fn field_get_u16(mask: u16, value: u16) -> u16 {
    (value & mask) >> mask.trailing_zeros()
}

pub const fn gsw1xx_tag_xmit(port_mask: u16) -> [u16; 4] {
    let tag =
        field_prep_u16(GSW1XX_TX_PORT_MAP, port_mask) | GSW1XX_TX_PORT_MAP_EN | GSW1XX_TX_LRN_DIS;
    [ETH_P_MXLGSW.to_be(), tag.to_be(), 0, 0]
}

pub const fn gsw1xx_tag_rcv(
    tag_words_be: Option<[u16; 4]>,
    user_port_exists: bool,
) -> Option<Gsw1xxRxFrame> {
    let tag = match tag_words_be {
        Some(tag) => tag,
        None => return None,
    };
    if u16::from_be(tag[0]) != ETH_P_MXLGSW {
        return None;
    }
    let source_port = field_get_u16(GSW1XX_RX_PORT_MAP, u16::from_be(tag[1]));
    if !user_port_exists {
        return None;
    }
    Some(Gsw1xxRxFrame {
        source_port,
        tag_removed: true,
        etype_header_stripped: true,
    })
}

pub const fn gsw1xx_module_ops() -> &'static DsaDeviceOps {
    &GSW1XX_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:gsw1xx", "dsa_tag:id-31"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_mxl_gsw1xx_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_mxl-gsw1xx.c"
        ));
        assert!(source.contains("#define GSW1XX_TAG_NAME\t\t\"gsw1xx\""));
        assert!(source.contains("#define GSW1XX_HEADER_LEN\t\t8"));
        assert!(source.contains("#define GSW1XX_TX_PORT_MAP\t\tGENMASK(7, 0)"));
        assert!(source.contains("#define GSW1XX_TX_PORT_MAP_EN\t\tBIT(15)"));
        assert!(source.contains("#define GSW1XX_TX_LRN_DIS\t\tBIT(12)"));
        assert!(source.contains("#define GSW1XX_RX_PORT_MAP\t\tGENMASK(15, 8)"));
        assert!(source.contains("skb_push(skb, GSW1XX_HEADER_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, GSW1XX_HEADER_LEN);"));
        assert!(source.contains("gsw1xx_tag[0] = htons(ETH_P_MXLGSW);"));
        assert!(source.contains("FIELD_PREP(GSW1XX_TX_PORT_MAP, dsa_xmit_port_mask(skb, dev))"));
        assert!(source.contains("GSW1XX_TX_PORT_MAP_EN | GSW1XX_TX_LRN_DIS"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, GSW1XX_HEADER_LEN)))"));
        assert!(source.contains("if (unlikely(ntohs(gsw1xx_tag[0]) != ETH_P_MXLGSW))"));
        assert!(source.contains("port = FIELD_GET(GSW1XX_RX_PORT_MAP, ntohs(gsw1xx_tag[1]));"));
        assert!(source.contains("skb_pull_rcsum(skb, GSW1XX_HEADER_LEN);"));
        assert!(source.contains("dsa_strip_etype_header(skb, GSW1XX_HEADER_LEN);"));
        assert!(source.contains(".proto\t\t\t= DSA_TAG_PROTO_MXL_GSW1XX"));
        assert!(source.contains(".needed_headroom\t= GSW1XX_HEADER_LEN"));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        assert!(dsa.contains("#define DSA_TAG_PROTO_MXL_GSW1XX_VALUE\t\t31"));
    }

    #[test]
    fn gsw1xx_xmit_and_receive_preserve_word_layout() {
        assert_eq!(
            gsw1xx_tag_xmit(0x2a),
            [
                ETH_P_MXLGSW.to_be(),
                (GSW1XX_TX_PORT_MAP_EN | GSW1XX_TX_LRN_DIS | 0x2a).to_be(),
                0,
                0
            ]
        );
        let rx =
            gsw1xx_tag_rcv(Some([ETH_P_MXLGSW.to_be(), 0x0500u16.to_be(), 0, 0]), true).unwrap();
        assert_eq!(rx.source_port, 5);
        assert!(rx.tag_removed);
        assert!(rx.etype_header_stripped);
        assert_eq!(gsw1xx_tag_rcv(None, true), None);
        assert_eq!(
            gsw1xx_tag_rcv(Some([0, 0x0500u16.to_be(), 0, 0]), true),
            None
        );
        assert_eq!(
            gsw1xx_tag_rcv(Some([ETH_P_MXLGSW.to_be(), 0x0500u16.to_be(), 0, 0]), false),
            None
        );
        assert_eq!(gsw1xx_module_ops(), &GSW1XX_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:gsw1xx", "dsa_tag:id-31"]);
    }
}
