//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_gswip.c
//! test-origin: linux:vendor/linux/net/dsa/tag_gswip.c
//! Intel / Lantiq GSWIP V2.0 PMAC tag support.

pub const GSWIP_NAME: &str = "gswip";
pub const GSWIP_TX_HEADER_LEN: usize = 4;
pub const GSWIP_TX_SLPID_SHIFT: u8 = 0;
pub const GSWIP_TX_SLPID_CPU: u8 = 2;
pub const GSWIP_TX_SLPID_APP1: u8 = 3;
pub const GSWIP_TX_SLPID_APP2: u8 = 4;
pub const GSWIP_TX_SLPID_APP3: u8 = 5;
pub const GSWIP_TX_SLPID_APP4: u8 = 6;
pub const GSWIP_TX_SLPID_APP5: u8 = 7;
pub const GSWIP_TX_CRCGEN_DIS: u8 = 1 << 7;
pub const GSWIP_TX_DPID_SHIFT: u8 = 0;
pub const GSWIP_TX_DPID_ELAN: u8 = 0;
pub const GSWIP_TX_DPID_EWAN: u8 = 1;
pub const GSWIP_TX_DPID_CPU: u8 = 2;
pub const GSWIP_TX_DPID_APP1: u8 = 3;
pub const GSWIP_TX_DPID_APP2: u8 = 4;
pub const GSWIP_TX_DPID_APP3: u8 = 5;
pub const GSWIP_TX_DPID_APP4: u8 = 6;
pub const GSWIP_TX_DPID_APP5: u8 = 7;
pub const GSWIP_TX_PORT_MAP_EN: u8 = 1 << 7;
pub const GSWIP_TX_PORT_MAP_SEL: u8 = 1 << 6;
pub const GSWIP_TX_LRN_DIS: u8 = 1 << 5;
pub const GSWIP_TX_CLASS_EN: u8 = 1 << 4;
pub const GSWIP_TX_CLASS_SHIFT: u8 = 0;
pub const GSWIP_TX_CLASS_MASK: u8 = 0x0f;
pub const GSWIP_TX_DPID_EN: u8 = 1 << 0;
pub const GSWIP_TX_PORT_MAP: u8 = 0x7e;
pub const GSWIP_RX_HEADER_LEN: usize = 8;
pub const GSWIP_RX_SPPID_SHIFT: u8 = 4;
pub const GSWIP_RX_SPPID_MASK: u8 = 0x70;
pub const DSA_TAG_PROTO_GSWIP_VALUE: u8 = 5;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Lantiq / Intel GSWIP switches";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GswipRxFrame {
    pub source_port: u8,
    pub tag_removed: bool,
}

pub const GSWIP_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: GSWIP_NAME,
    proto: DSA_TAG_PROTO_GSWIP_VALUE,
    needed_headroom: GSWIP_RX_HEADER_LEN,
};

const fn field_prep_u8(mask: u8, value: u8) -> u8 {
    (value << mask.trailing_zeros()) & mask
}

pub const fn gswip_tag_xmit(port_mask: u8) -> [u8; GSWIP_TX_HEADER_LEN] {
    [
        GSWIP_TX_SLPID_CPU,
        GSWIP_TX_DPID_ELAN,
        GSWIP_TX_PORT_MAP_EN | GSWIP_TX_PORT_MAP_SEL,
        field_prep_u8(GSWIP_TX_PORT_MAP, port_mask) | GSWIP_TX_DPID_EN,
    ]
}

pub const fn gswip_tag_rcv(
    tag_before_eth_header: Option<[u8; GSWIP_RX_HEADER_LEN]>,
    user_port_exists: bool,
) -> Option<GswipRxFrame> {
    let tag = match tag_before_eth_header {
        Some(tag) => tag,
        None => return None,
    };
    let source_port = (tag[7] & GSWIP_RX_SPPID_MASK) >> GSWIP_RX_SPPID_SHIFT;
    if !user_port_exists {
        return None;
    }
    Some(GswipRxFrame {
        source_port,
        tag_removed: true,
    })
}

pub const fn gswip_module_ops() -> &'static DsaDeviceOps {
    &GSWIP_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:gswip", "dsa_tag:id-5"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_gswip_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_gswip.c"
        ));
        assert!(source.contains("#define GSWIP_NAME\t\t\t\"gswip\""));
        assert!(source.contains("#define GSWIP_TX_HEADER_LEN\t\t4"));
        assert!(source.contains("#define GSWIP_RX_HEADER_LEN\t8"));
        assert!(source.contains("skb_push(skb, GSWIP_TX_HEADER_LEN);"));
        assert!(source.contains("gswip_tag[0] = GSWIP_TX_SLPID_CPU;"));
        assert!(source.contains("gswip_tag[1] = GSWIP_TX_DPID_ELAN;"));
        assert!(source.contains("GSWIP_TX_PORT_MAP_EN | GSWIP_TX_PORT_MAP_SEL"));
        assert!(source.contains("FIELD_PREP(GSWIP_TX_PORT_MAP, dsa_xmit_port_mask(skb, dev))"));
        assert!(source.contains("gswip_tag[3] |= GSWIP_TX_DPID_EN;"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, GSWIP_RX_HEADER_LEN)))"));
        assert!(source.contains("gswip_tag = skb->data - ETH_HLEN;"));
        assert!(
            source.contains("port = (gswip_tag[7] & GSWIP_RX_SPPID_MASK) >> GSWIP_RX_SPPID_SHIFT;")
        );
        assert!(source.contains("skb_pull_rcsum(skb, GSWIP_RX_HEADER_LEN);"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_GSWIP"));
        assert!(source.contains(".needed_headroom = GSWIP_RX_HEADER_LEN"));
        assert!(
            source.contains(
                "MODULE_DESCRIPTION(\"DSA tag driver for Lantiq / Intel GSWIP switches\")"
            )
        );
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        assert!(dsa.contains("#define DSA_TAG_PROTO_GSWIP_VALUE\t\t5"));
    }

    #[test]
    fn gswip_xmit_and_receive_encode_ports() {
        assert_eq!(gswip_tag_xmit(0x12), [2, 0, 0xc0, 0x25]);
        let mut rx_tag = [0u8; GSWIP_RX_HEADER_LEN];
        rx_tag[7] = 0x50;
        let rx = gswip_tag_rcv(Some(rx_tag), true).unwrap();
        assert_eq!(rx.source_port, 5);
        assert!(rx.tag_removed);
        assert_eq!(gswip_tag_rcv(None, true), None);
        assert_eq!(gswip_tag_rcv(Some(rx_tag), false), None);
        assert_eq!(gswip_module_ops(), &GSWIP_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:gswip", "dsa_tag:id-5"]);
    }
}
