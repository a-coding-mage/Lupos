//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_mxl862xx.c
//! test-origin: linux:vendor/linux/net/dsa/tag_mxl862xx.c
//! DSA special tag for MaxLinear 862xx switch chips.

pub const MXL862_NAME: &str = "mxl862xx";
pub const MXL862_HEADER_LEN: usize = 8;
pub const MXL862_SUBIF_ID: u16 = 0x001f;
pub const MXL862_IGP_EGP: u16 = 0x000f;
pub const ETH_P_MXLGSW: u16 = 0x88c3;
pub const DSA_TAG_PROTO_MXL862_VALUE: u8 = 32;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for MaxLinear MxL862xx switches";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mxl862TxFrame {
    pub tag_words_be: [u16; 4],
    pub sub_interface: u16,
    pub etype_header_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mxl862RxFrame {
    pub source_port: u16,
    pub tag_removed: bool,
    pub etype_header_stripped: bool,
    pub offload_fwd_mark: bool,
}

pub const MXL862_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: MXL862_NAME,
    proto: DSA_TAG_PROTO_MXL862_VALUE,
    needed_headroom: MXL862_HEADER_LEN,
};

const fn field_prep_u16(mask: u16, value: u16) -> u16 {
    (value << mask.trailing_zeros()) & mask
}

const fn field_get_u16(mask: u16, value: u16) -> u16 {
    (value & mask) >> mask.trailing_zeros()
}

pub const fn mxl862_tag_xmit(dp_index: u16, cpu_port: u16) -> Mxl862TxFrame {
    let sub_interface = dp_index + 16 - cpu_port;
    Mxl862TxFrame {
        tag_words_be: [
            ETH_P_MXLGSW.to_be(),
            0,
            field_prep_u16(MXL862_SUBIF_ID, sub_interface).to_be(),
            field_prep_u16(MXL862_IGP_EGP, cpu_port).to_be(),
        ],
        sub_interface,
        etype_header_allocated: true,
    }
}

pub const fn mxl862_tag_rcv(
    tag_words_be: Option<[u16; 4]>,
    h_dest: [u8; 6],
    user_port_exists: bool,
) -> Option<Mxl862RxFrame> {
    let tag = match tag_words_be {
        Some(tag) => tag,
        None => return None,
    };
    if tag[0] != ETH_P_MXLGSW.to_be() {
        return None;
    }
    let source_port = field_get_u16(MXL862_IGP_EGP, u16::from_be(tag[3]));
    if !user_port_exists {
        return None;
    }
    Some(Mxl862RxFrame {
        source_port,
        tag_removed: true,
        etype_header_stripped: true,
        offload_fwd_mark: !is_link_local_ether_addr(h_dest),
    })
}

pub const fn is_link_local_ether_addr(addr: [u8; 6]) -> bool {
    addr[0] == 0x01
        && addr[1] == 0x80
        && addr[2] == 0xc2
        && addr[3] == 0x00
        && addr[4] == 0x00
        && (addr[5] & 0xf0) == 0x00
}

pub const fn mxl862_module_ops() -> &'static DsaDeviceOps {
    &MXL862_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:mxl862xx", "dsa_tag:id-32"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_mxl862xx_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_mxl862xx.c"
        ));
        assert!(source.contains("#define MXL862_NAME\t\"mxl862xx\""));
        assert!(source.contains("#define MXL862_HEADER_LEN\t8"));
        assert!(source.contains("#define MXL862_SUBIF_ID\t\tGENMASK(4, 0)"));
        assert!(source.contains("#define MXL862_IGP_EGP\t\tGENMASK(3, 0)"));
        assert!(source.contains("cpu_port = cpu_dp->index;"));
        assert!(source.contains("sub_interface = dp->index + 16 - cpu_port;"));
        assert!(source.contains("skb_push(skb, MXL862_HEADER_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, MXL862_HEADER_LEN);"));
        assert!(source.contains("mxl862_tag[0] = htons(ETH_P_MXLGSW);"));
        assert!(
            source.contains("mxl862_tag[2] = htons(FIELD_PREP(MXL862_SUBIF_ID, sub_interface));")
        );
        assert!(source.contains("mxl862_tag[3] = htons(FIELD_PREP(MXL862_IGP_EGP, cpu_port));"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, MXL862_HEADER_LEN)))"));
        assert!(source.contains("if (unlikely(mxl862_tag[0] != htons(ETH_P_MXLGSW)))"));
        assert!(source.contains("port = FIELD_GET(MXL862_IGP_EGP, ntohs(mxl862_tag[3]));"));
        assert!(source.contains("if (likely(!is_link_local_ether_addr(eth_hdr(skb)->h_dest)))"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains("dsa_strip_etype_header(skb, MXL862_HEADER_LEN);"));
        assert!(source.contains(".proto = DSA_TAG_PROTO_MXL862"));
        assert!(source.contains(".needed_headroom = MXL862_HEADER_LEN"));
        let eth = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/if_ether.h"
        ));
        assert!(eth.contains("#define ETH_P_MXLGSW\t0x88C3"));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        assert!(dsa.contains("#define DSA_TAG_PROTO_MXL862_VALUE\t\t32"));
    }

    #[test]
    fn mxl862_xmit_and_receive_use_cpu_relative_subinterface() {
        let tx = mxl862_tag_xmit(3, 6);
        assert_eq!(tx.sub_interface, 13);
        assert_eq!(
            tx.tag_words_be,
            [ETH_P_MXLGSW.to_be(), 0, 13u16.to_be(), 6u16.to_be()]
        );
        assert!(tx.etype_header_allocated);

        let rx = mxl862_tag_rcv(
            Some([
                ETH_P_MXLGSW.to_be(),
                0,
                0,
                field_prep_u16(MXL862_IGP_EGP, 4).to_be(),
            ]),
            [0x02, 0, 0, 0, 0, 1],
            true,
        )
        .unwrap();
        assert_eq!(rx.source_port, 4);
        assert!(rx.offload_fwd_mark);
        assert_eq!(
            mxl862_tag_rcv(Some([0, 0, 0, 0]), [0x02, 0, 0, 0, 0, 1], true),
            None
        );
        assert_eq!(
            mxl862_tag_rcv(
                Some([ETH_P_MXLGSW.to_be(), 0, 0, 0]),
                [0x02, 0, 0, 0, 0, 1],
                false
            ),
            None
        );
        let link_local = mxl862_tag_rcv(
            Some([ETH_P_MXLGSW.to_be(), 0, 0, 0]),
            [0x01, 0x80, 0xc2, 0, 0, 0x0f],
            true,
        )
        .unwrap();
        assert!(!link_local.offload_fwd_mark);
        assert_eq!(mxl862_module_ops(), &MXL862_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:mxl862xx", "dsa_tag:id-32"]);
    }
}
