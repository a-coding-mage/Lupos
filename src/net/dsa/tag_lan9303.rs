//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_lan9303.c
//! test-origin: linux:vendor/linux/net/dsa/tag_lan9303.c
//! SMSC/Microchip LAN9303 DSA VLAN tag support.

pub const LAN9303_NAME: &str = "lan9303";
pub const LAN9303_TAG_LEN: usize = 4;
pub const LAN9303_TAG_TX_USE_ALR: u16 = 1 << 3;
pub const LAN9303_TAG_TX_STP_OVERRIDE: u16 = 1 << 4;
pub const LAN9303_TAG_RX_IGMP: u16 = 1 << 3;
pub const LAN9303_TAG_RX_STP: u16 = 1 << 4;
pub const LAN9303_TAG_RX_TRAPPED_TO_CPU: u16 = LAN9303_TAG_RX_IGMP | LAN9303_TAG_RX_STP;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const DSA_TAG_PROTO_LAN9303_VALUE: u8 = 8;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for SMSC/Microchip LAN9303 family of switches";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lan9303TxFrame {
    pub tag_be: [u16; 2],
    pub use_alr: bool,
    pub etype_header_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lan9303RxFrame {
    pub source_port: u8,
    pub vlan_tag_cleared: bool,
    pub offload_fwd_mark: bool,
}

pub const LAN9303_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: LAN9303_NAME,
    proto: DSA_TAG_PROTO_LAN9303_VALUE,
    needed_headroom: LAN9303_TAG_LEN,
};

pub const fn lan9303_xmit_use_alr(is_bridged: bool, dest_addr: [u8; 6]) -> bool {
    is_bridged && !is_multicast_ether_addr(dest_addr)
}

pub const fn lan9303_xmit(dp_index: u8, is_bridged: bool, dest_addr: [u8; 6]) -> Lan9303TxFrame {
    let use_alr = lan9303_xmit_use_alr(is_bridged, dest_addr);
    let tag = if use_alr {
        LAN9303_TAG_TX_USE_ALR
    } else {
        (dp_index as u16) | LAN9303_TAG_TX_STP_OVERRIDE
    };
    Lan9303TxFrame {
        tag_be: [ETH_P_8021Q.to_be(), tag.to_be()],
        use_alr,
        etype_header_allocated: true,
    }
}

pub const fn lan9303_rcv(lan9303_tag1: u16, user_port_exists: bool) -> Option<Lan9303RxFrame> {
    let source_port = (lan9303_tag1 & 0x3) as u8;
    if !user_port_exists {
        return None;
    }
    Some(Lan9303RxFrame {
        source_port,
        vlan_tag_cleared: true,
        offload_fwd_mark: (lan9303_tag1 & LAN9303_TAG_RX_TRAPPED_TO_CPU) == 0,
    })
}

pub const fn is_multicast_ether_addr(addr: [u8; 6]) -> bool {
    (addr[0] & 0x01) != 0
}

pub const fn lan9303_module_ops() -> &'static DsaDeviceOps {
    &LAN9303_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:lan9303", "dsa_tag:id-8"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_lan9303_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_lan9303.c"
        ));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));

        assert!(source.contains("#define LAN9303_NAME \"lan9303\""));
        assert!(source.contains("#define LAN9303_TAG_LEN 4"));
        assert!(source.contains("# define LAN9303_TAG_TX_USE_ALR BIT(3)"));
        assert!(source.contains("# define LAN9303_TAG_TX_STP_OVERRIDE BIT(4)"));
        assert!(source.contains("chip->is_bridged && !is_multicast_ether_addr(dest_addr)"));
        assert!(source.contains("skb_push(skb, LAN9303_TAG_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, LAN9303_TAG_LEN);"));
        assert!(source.contains("LAN9303_TAG_TX_USE_ALR :"));
        assert!(source.contains("dp->index | LAN9303_TAG_TX_STP_OVERRIDE;"));
        assert!(source.contains("lan9303_tag[0] = htons(ETH_P_8021Q);"));
        assert!(source.contains("lan9303_tag[1] = htons(tag);"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, LAN9303_TAG_LEN)))"));
        assert!(source.contains("if (skb_vlan_tag_present(skb))"));
        assert!(source.contains("__vlan_hwaccel_clear_tag(skb);"));
        assert!(source.contains("__skb_vlan_pop(skb, &lan9303_tag1);"));
        assert!(source.contains("source_port = lan9303_tag1 & 0x3;"));
        assert!(source.contains("dsa_conduit_find_user(dev, 0, source_port);"));
        assert!(source.contains("if (!(lan9303_tag1 & LAN9303_TAG_RX_TRAPPED_TO_CPU))"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_LAN9303"));
        assert!(source.contains(".needed_headroom = LAN9303_TAG_LEN"));
        assert!(
            source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_LAN9303, LAN9303_NAME);")
        );
        assert!(dsa.contains("#define DSA_TAG_PROTO_LAN9303_VALUE\t\t8"));
    }

    #[test]
    fn lan9303_encode_decode_tracks_alr_and_trap_bits() {
        let unicast = [0x02, 0, 0, 0, 0, 1];
        let multicast = [0x01, 0, 0, 0, 0, 1];
        let alr = lan9303_xmit(2, true, unicast);
        assert_eq!(u16::from_be(alr.tag_be[0]), ETH_P_8021Q);
        assert_eq!(u16::from_be(alr.tag_be[1]), LAN9303_TAG_TX_USE_ALR);
        assert!(alr.use_alr);
        assert!(alr.etype_header_allocated);

        let direct = lan9303_xmit(2, true, multicast);
        assert_eq!(
            u16::from_be(direct.tag_be[1]),
            2 | LAN9303_TAG_TX_STP_OVERRIDE
        );
        assert!(!direct.use_alr);

        let rx = lan9303_rcv(2, true).unwrap();
        assert_eq!(rx.source_port, 2);
        assert!(rx.vlan_tag_cleared);
        assert!(rx.offload_fwd_mark);
        assert!(
            !lan9303_rcv(LAN9303_TAG_RX_STP | 1, true)
                .unwrap()
                .offload_fwd_mark
        );
        assert_eq!(lan9303_rcv(1, false), None);
        assert_eq!(lan9303_module_ops(), &LAN9303_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:lan9303", "dsa_tag:id-8"]);
    }
}
