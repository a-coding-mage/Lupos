//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_yt921x.c
//! test-origin: linux:vendor/linux/net/dsa/tag_yt921x.c
//! Motorcomm YT921x extended CPU port tag support.

pub const YT921X_TAG_NAME: &str = "yt921x";
pub const YT921X_TAG_LEN: usize = 8;
pub const YT921X_TAG_PORT_EN: u16 = 1 << 15;
pub const YT921X_TAG_RX_PORT_M: u16 = 0x7800;
pub const YT921X_TAG_PRIO_M: u16 = 0x0700;
pub const YT921X_TAG_CODE_EN: u16 = 1 << 7;
pub const YT921X_TAG_CODE_M: u16 = 0x007e;
pub const YT921X_TAG_TX_PORTS_M: u16 = 0x07ff;
pub const ETH_P_YT921X: u16 = 0x9988;
pub const DSA_TAG_PROTO_YT921X_VALUE: u8 = 30;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Motorcomm YT921x switches";
pub const MODULE_LICENSE: &str = "GPL";

pub const YT921X_TAG_CODE_FORWARD: u16 = 0;
pub const YT921X_TAG_CODE_UNK_UCAST: u16 = 0x19;
pub const YT921X_TAG_CODE_UNK_MCAST: u16 = 0x1a;
pub const YT921X_TAG_CODE_PORT_COPY: u16 = 0x1b;
pub const YT921X_TAG_CODE_FDB_COPY: u16 = 0x1c;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Yt921xTxFrame {
    pub tag_be: [u16; 4],
    pub etype_header_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Yt921xRxFrame {
    pub source_port: u8,
    pub priority: u8,
    pub code_enabled: bool,
    pub code: Option<u16>,
    pub tag_removed: bool,
    pub etype_header_stripped: bool,
    pub offload_fwd_mark: bool,
}

pub const YT921X_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: YT921X_TAG_NAME,
    proto: DSA_TAG_PROTO_YT921X_VALUE,
    needed_headroom: YT921X_TAG_LEN,
};

const fn field_prep(mask: u16, value: u16) -> u16 {
    (value << mask.trailing_zeros()) & mask
}

const fn field_get(mask: u16, value: u16) -> u16 {
    (value & mask) >> mask.trailing_zeros()
}

pub const fn yt921x_tag_prio(priority: u8) -> u16 {
    field_prep(YT921X_TAG_PRIO_M, priority as u16)
}

pub const fn yt921x_tag_code(code: u16) -> u16 {
    field_prep(YT921X_TAG_CODE_M, code)
}

pub const fn yt921x_tag_tx_ports(port_mask: u16) -> u16 {
    field_prep(YT921X_TAG_TX_PORTS_M, port_mask)
}

pub const fn yt921x_tag_xmit(priority: u8, port_mask: u16) -> Yt921xTxFrame {
    let rx_ctrl =
        yt921x_tag_code(YT921X_TAG_CODE_FORWARD) | YT921X_TAG_CODE_EN | yt921x_tag_prio(priority);
    let tx_ctrl = yt921x_tag_tx_ports(port_mask) | YT921X_TAG_PORT_EN;
    Yt921xTxFrame {
        tag_be: [ETH_P_YT921X.to_be(), 0, rx_ctrl.to_be(), tx_ctrl.to_be()],
        etype_header_allocated: true,
    }
}

pub const fn yt921x_tag_rcv(tag_be: [u16; 4], user_port_exists: bool) -> Option<Yt921xRxFrame> {
    if u16::from_be(tag_be[0]) != ETH_P_YT921X {
        return None;
    }

    let rx = u16::from_be(tag_be[2]);
    if (rx & YT921X_TAG_PORT_EN) == 0 {
        return None;
    }

    if !user_port_exists {
        return None;
    }

    let code_enabled = (rx & YT921X_TAG_CODE_EN) != 0;
    let code = if code_enabled {
        Some(field_get(YT921X_TAG_CODE_M, rx))
    } else {
        None
    };
    let offload_fwd_mark = match code {
        Some(YT921X_TAG_CODE_FORWARD)
        | Some(YT921X_TAG_CODE_PORT_COPY)
        | Some(YT921X_TAG_CODE_FDB_COPY) => true,
        _ => false,
    };

    Some(Yt921xRxFrame {
        source_port: field_get(YT921X_TAG_RX_PORT_M, rx) as u8,
        priority: field_get(YT921X_TAG_PRIO_M, rx) as u8,
        code_enabled,
        code,
        tag_removed: true,
        etype_header_stripped: true,
        offload_fwd_mark,
    })
}

pub const fn yt921x_module_ops() -> &'static DsaDeviceOps {
    &YT921X_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:yt921x", "dsa_tag:id-30"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_yt921x_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_yt921x.c"
        ));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        let if_ether = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/if_ether.h"
        ));

        assert!(source.contains("#define YT921X_TAG_NAME\t\"yt921x\""));
        assert!(source.contains("#define YT921X_TAG_LEN\t8"));
        assert!(source.contains("#define YT921X_TAG_PORT_EN\t\tBIT(15)"));
        assert!(source.contains("#define YT921X_TAG_RX_PORT_M\t\tGENMASK(14, 11)"));
        assert!(source.contains("#define YT921X_TAG_PRIO_M\t\tGENMASK(10, 8)"));
        assert!(source.contains("#define YT921X_TAG_CODE_EN\t\tBIT(7)"));
        assert!(source.contains("#define YT921X_TAG_CODE_M\t\tGENMASK(6, 1)"));
        assert!(source.contains("#define YT921X_TAG_TX_PORTS_M\t\tGENMASK(10, 0)"));
        assert!(source.contains("YT921X_TAG_CODE_FORWARD = 0"));
        assert!(source.contains("YT921X_TAG_CODE_UNK_UCAST = 0x19"));
        assert!(source.contains("YT921X_TAG_CODE_FDB_COPY = 0x1c"));
        assert!(source.contains("skb_push(skb, YT921X_TAG_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, YT921X_TAG_LEN);"));
        assert!(source.contains("tag[0] = htons(ETH_P_YT921X);"));
        assert!(source.contains("tag[1] = 0;"));
        assert!(source.contains("YT921X_TAG_CODE(YT921X_TAG_CODE_FORWARD) | YT921X_TAG_CODE_EN"));
        assert!(source.contains("YT921X_TAG_PRIO(skb->priority);"));
        assert!(source.contains("YT921X_TAG_TX_PORTS(dsa_xmit_port_mask(skb, netdev))"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, YT921X_TAG_LEN)))"));
        assert!(source.contains("tag[0] != htons(ETH_P_YT921X)"));
        assert!(source.contains("(rx & YT921X_TAG_PORT_EN) == 0"));
        assert!(source.contains("port = FIELD_GET(YT921X_TAG_RX_PORT_M, rx);"));
        assert!(source.contains("skb->priority = FIELD_GET(YT921X_TAG_PRIO_M, rx);"));
        assert!(source.contains("if (!(rx & YT921X_TAG_CODE_EN))"));
        assert!(source.contains("case YT921X_TAG_CODE_FORWARD:"));
        assert!(source.contains("case YT921X_TAG_CODE_UNK_UCAST:"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains("skb_pull_rcsum(skb, YT921X_TAG_LEN);"));
        assert!(source.contains("dsa_strip_etype_header(skb, YT921X_TAG_LEN);"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_YT921X"));
        assert!(source.contains(".needed_headroom = YT921X_TAG_LEN"));
        assert!(
            source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_YT921X, YT921X_TAG_NAME);")
        );
        assert!(dsa.contains("#define DSA_TAG_PROTO_YT921X_VALUE\t\t30"));
        assert!(if_ether.contains("#define ETH_P_YT921X\t0x9988"));
    }

    #[test]
    fn yt921x_encode_decode_tracks_ports_priority_and_codes() {
        let tx = yt921x_tag_xmit(5, 0x155);
        assert_eq!(u16::from_be(tx.tag_be[0]), ETH_P_YT921X);
        assert_eq!(u16::from_be(tx.tag_be[1]), 0);
        assert_eq!(
            u16::from_be(tx.tag_be[2]),
            YT921X_TAG_CODE_EN | yt921x_tag_prio(5)
        );
        assert_eq!(
            u16::from_be(tx.tag_be[3]),
            YT921X_TAG_PORT_EN | yt921x_tag_tx_ports(0x155)
        );
        assert!(tx.etype_header_allocated);

        let rx_word = YT921X_TAG_PORT_EN
            | field_prep(YT921X_TAG_RX_PORT_M, 3)
            | yt921x_tag_prio(6)
            | YT921X_TAG_CODE_EN
            | yt921x_tag_code(YT921X_TAG_CODE_PORT_COPY);
        let rx = yt921x_tag_rcv([ETH_P_YT921X.to_be(), 0, rx_word.to_be(), 0], true).unwrap();
        assert_eq!(rx.source_port, 3);
        assert_eq!(rx.priority, 6);
        assert_eq!(rx.code, Some(YT921X_TAG_CODE_PORT_COPY));
        assert!(rx.offload_fwd_mark);
        assert!(rx.tag_removed);
        assert!(rx.etype_header_stripped);

        let unknown_ucast = YT921X_TAG_PORT_EN
            | field_prep(YT921X_TAG_RX_PORT_M, 1)
            | YT921X_TAG_CODE_EN
            | yt921x_tag_code(YT921X_TAG_CODE_UNK_UCAST);
        assert!(
            !yt921x_tag_rcv([ETH_P_YT921X.to_be(), 0, unknown_ucast.to_be(), 0], true)
                .unwrap()
                .offload_fwd_mark
        );
        let no_code = YT921X_TAG_PORT_EN | field_prep(YT921X_TAG_RX_PORT_M, 2);
        let no_code_rx =
            yt921x_tag_rcv([ETH_P_YT921X.to_be(), 0, no_code.to_be(), 0], true).unwrap();
        assert!(!no_code_rx.code_enabled);
        assert_eq!(no_code_rx.code, None);
        assert_eq!(
            yt921x_tag_rcv([0x0800u16.to_be(), 0, rx_word.to_be(), 0], true),
            None
        );
        assert_eq!(yt921x_tag_rcv([ETH_P_YT921X.to_be(), 0, 0, 0], true), None);
        assert_eq!(
            yt921x_tag_rcv([ETH_P_YT921X.to_be(), 0, rx_word.to_be(), 0], false),
            None
        );
        assert_eq!(yt921x_module_ops(), &YT921X_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:yt921x", "dsa_tag:id-30"]);
    }
}
