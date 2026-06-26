//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_rtl4_a.c
//! test-origin: linux:vendor/linux/net/dsa/tag_rtl4_a.c
//! Realtek 4 byte protocol A DSA tag support.

pub const RTL4_A_NAME: &str = "rtl4a";
pub const RTL4_A_HDR_LEN: usize = 4;
pub const RTL4_A_PROTOCOL_SHIFT: u16 = 12;
pub const RTL4_A_PROTOCOL_RTL8366RB: u16 = 0xa;
pub const ETH_P_REALTEK: u16 = 0x8899;
pub const ETH_ZLEN: usize = 60;
pub const DSA_TAG_PROTO_RTL4_A_VALUE: u8 = 17;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Realtek 4 byte protocol A tags";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rtl4aTxFrame {
    pub tag_be: [u16; 2],
    pub padded_to_eth_zlen: bool,
    pub etype_header_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rtl4aRxFrame {
    pub source_port: u8,
    pub tag_removed: bool,
    pub etype_header_stripped: bool,
    pub offload_fwd_mark: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rtl4aRxResult {
    Frame(Rtl4aRxFrame),
    PassThrough,
    Drop,
}

pub const RTL4_A_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: RTL4_A_NAME,
    proto: DSA_TAG_PROTO_RTL4_A_VALUE,
    needed_headroom: RTL4_A_HDR_LEN,
};

pub const fn rtl4a_tag_xmit(frame_len: usize, port_mask: u8) -> Rtl4aTxFrame {
    let out = (RTL4_A_PROTOCOL_RTL8366RB << RTL4_A_PROTOCOL_SHIFT) | port_mask as u16;
    Rtl4aTxFrame {
        tag_be: [ETH_P_REALTEK.to_be(), out.to_be()],
        padded_to_eth_zlen: frame_len < ETH_ZLEN,
        etype_header_allocated: true,
    }
}

pub const fn rtl4a_tag_rcv(
    ethertype_be: u16,
    protport_be: u16,
    user_port_exists: bool,
) -> Rtl4aRxResult {
    let etype = u16::from_be(ethertype_be);
    if etype != ETH_P_REALTEK {
        return Rtl4aRxResult::PassThrough;
    }

    let protport = u16::from_be(protport_be);
    let prot = (protport >> RTL4_A_PROTOCOL_SHIFT) & 0x0f;
    if prot != RTL4_A_PROTOCOL_RTL8366RB {
        return Rtl4aRxResult::Drop;
    }

    if !user_port_exists {
        return Rtl4aRxResult::Drop;
    }

    Rtl4aRxResult::Frame(Rtl4aRxFrame {
        source_port: (protport & 0xff) as u8,
        tag_removed: true,
        etype_header_stripped: true,
        offload_fwd_mark: true,
    })
}

pub const fn rtl4a_module_ops() -> &'static DsaDeviceOps {
    &RTL4_A_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:rtl4a", "dsa_tag:id-17"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_rtl4_a_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_rtl4_a.c"
        ));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        let if_ether = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/if_ether.h"
        ));

        assert!(source.contains("#define RTL4_A_NAME\t\t\"rtl4a\""));
        assert!(source.contains("#define RTL4_A_HDR_LEN\t\t4"));
        assert!(source.contains("#define RTL4_A_PROTOCOL_SHIFT\t12"));
        assert!(source.contains("#define RTL4_A_PROTOCOL_RTL8366RB\t0xa"));
        assert!(source.contains("__skb_put_padto(skb, ETH_ZLEN, false)"));
        assert!(source.contains("skb_push(skb, RTL4_A_HDR_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, RTL4_A_HDR_LEN);"));
        assert!(source.contains("*p = htons(ETH_P_REALTEK);"));
        assert!(source.contains("out = (RTL4_A_PROTOCOL_RTL8366RB << RTL4_A_PROTOCOL_SHIFT);"));
        assert!(source.contains("out |= dsa_xmit_port_mask(skb, dev);"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, RTL4_A_HDR_LEN)))"));
        assert!(source.contains("if (etype != ETH_P_REALTEK)"));
        assert!(source.contains("return skb;"));
        assert!(source.contains("prot = (protport >> RTL4_A_PROTOCOL_SHIFT) & 0x0f;"));
        assert!(source.contains("port = protport & 0xff;"));
        assert!(source.contains("dsa_conduit_find_user(dev, 0, port);"));
        assert!(source.contains("skb_pull_rcsum(skb, RTL4_A_HDR_LEN);"));
        assert!(source.contains("dsa_strip_etype_header(skb, RTL4_A_HDR_LEN);"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_RTL4_A"));
        assert!(source.contains(".needed_headroom = RTL4_A_HDR_LEN"));
        assert!(source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_RTL4_A, RTL4_A_NAME);"));
        assert!(dsa.contains("#define DSA_TAG_PROTO_RTL4_A_VALUE\t\t17"));
        assert!(if_ether.contains("#define ETH_P_REALTEK\t0x8899"));
    }

    #[test]
    fn rtl4a_encode_decode_handles_passthrough_and_protocol() {
        let tx = rtl4a_tag_xmit(42, 0x05);
        assert_eq!(u16::from_be(tx.tag_be[0]), ETH_P_REALTEK);
        assert_eq!(
            u16::from_be(tx.tag_be[1]),
            (RTL4_A_PROTOCOL_RTL8366RB << RTL4_A_PROTOCOL_SHIFT) | 5
        );
        assert!(tx.padded_to_eth_zlen);
        assert!(tx.etype_header_allocated);

        assert_eq!(
            rtl4a_tag_rcv(0x0800u16.to_be(), 0, true),
            Rtl4aRxResult::PassThrough
        );
        let protport = ((RTL4_A_PROTOCOL_RTL8366RB << RTL4_A_PROTOCOL_SHIFT) | 7).to_be();
        assert_eq!(
            rtl4a_tag_rcv(ETH_P_REALTEK.to_be(), protport, true),
            Rtl4aRxResult::Frame(Rtl4aRxFrame {
                source_port: 7,
                tag_removed: true,
                etype_header_stripped: true,
                offload_fwd_mark: true,
            })
        );
        assert_eq!(
            rtl4a_tag_rcv(
                ETH_P_REALTEK.to_be(),
                (1u16 << RTL4_A_PROTOCOL_SHIFT).to_be(),
                true
            ),
            Rtl4aRxResult::Drop
        );
        assert_eq!(
            rtl4a_tag_rcv(ETH_P_REALTEK.to_be(), protport, false),
            Rtl4aRxResult::Drop
        );
        assert_eq!(rtl4a_module_ops(), &RTL4_A_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:rtl4a", "dsa_tag:id-17"]);
    }
}
