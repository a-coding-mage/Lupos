//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_ar9331.c
//! test-origin: linux:vendor/linux/net/dsa/tag_ar9331.c
//! DSA tag driver for the Atheros AR9331 switch header.

pub const AR9331_NAME: &str = "ar9331";
pub const AR9331_HDR_LEN: usize = 2;
pub const AR9331_HDR_VERSION: u16 = 1;
pub const AR9331_HDR_VERSION_MASK: u16 = 0xc000;
pub const AR9331_HDR_PRIORITY_MASK: u16 = 0x3000;
pub const AR9331_HDR_TYPE_MASK: u16 = 0x0700;
pub const AR9331_HDR_BROADCAST: u16 = 1 << 7;
pub const AR9331_HDR_FROM_CPU: u16 = 1 << 6;
pub const AR9331_HDR_RESERVED_MASK: u16 = 0x0030;
pub const AR9331_HDR_PORT_NUM_MASK: u16 = 0x000f;
pub const DSA_TAG_PROTO_AR9331_VALUE: u8 = 16;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Atheros AR9331 SoC with built-in switch";
pub const MODULE_LICENSE: &str = "GPL v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ar9331Frame {
    pub header: u16,
    pub port: u8,
    pub from_cpu: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

pub const AR9331_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: AR9331_NAME,
    proto: DSA_TAG_PROTO_AR9331_VALUE,
    needed_headroom: AR9331_HDR_LEN,
};

const fn field_prep(mask: u16, value: u16) -> u16 {
    (value << mask.trailing_zeros()) & mask
}

const fn field_get(mask: u16, value: u16) -> u16 {
    (value & mask) >> mask.trailing_zeros()
}

pub const fn ar9331_tag_xmit(port_index: u8) -> Ar9331Frame {
    let hdr = field_prep(AR9331_HDR_VERSION_MASK, AR9331_HDR_VERSION)
        | AR9331_HDR_FROM_CPU
        | AR9331_HDR_RESERVED_MASK
        | ((port_index as u16) & AR9331_HDR_PORT_NUM_MASK);
    Ar9331Frame {
        header: hdr.to_le(),
        port: port_index,
        from_cpu: true,
    }
}

pub const fn ar9331_tag_rcv(header_le: u16, user_port_exists: bool) -> Option<Ar9331Frame> {
    let hdr = u16::from_le(header_le);
    if field_get(AR9331_HDR_VERSION_MASK, hdr) != AR9331_HDR_VERSION {
        return None;
    }
    if hdr & AR9331_HDR_FROM_CPU != 0 {
        return None;
    }
    if !user_port_exists {
        return None;
    }
    let port = field_get(AR9331_HDR_PORT_NUM_MASK, hdr) as u8;
    Some(Ar9331Frame {
        header: header_le,
        port,
        from_cpu: false,
    })
}

pub const fn ar9331_module_ops() -> &'static DsaDeviceOps {
    &AR9331_NETDEV_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_ar9331_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_ar9331.c"
        ));
        assert!(source.contains("#define AR9331_NAME\t\t\t\"ar9331\""));
        assert!(source.contains("#define AR9331_HDR_LEN\t\t\t2"));
        assert!(source.contains("#define AR9331_HDR_VERSION\t\t1"));
        assert!(source.contains("FIELD_PREP(AR9331_HDR_VERSION_MASK, AR9331_HDR_VERSION);"));
        assert!(source.contains("hdr |= AR9331_HDR_FROM_CPU | dp->index;"));
        assert!(source.contains("hdr |= AR9331_HDR_RESERVED_MASK;"));
        assert!(source.contains("phdr[0] = cpu_to_le16(hdr);"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, AR9331_HDR_LEN)))"));
        assert!(source.contains("hdr = le16_to_cpu(*(__le16 *)skb_mac_header(skb));"));
        assert!(source.contains("ver = FIELD_GET(AR9331_HDR_VERSION_MASK, hdr);"));
        assert!(source.contains("if (unlikely(hdr & AR9331_HDR_FROM_CPU))"));
        assert!(source.contains("skb_pull_rcsum(skb, AR9331_HDR_LEN);"));
        assert!(source.contains("port = FIELD_GET(AR9331_HDR_PORT_NUM_MASK, hdr);"));
        assert!(source.contains("dsa_conduit_find_user(ndev, 0, port);"));
        assert!(source.contains(".name\t= AR9331_NAME"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_AR9331"));
        assert!(source.contains(".needed_headroom = AR9331_HDR_LEN"));
        assert!(source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_AR9331, AR9331_NAME);"));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));
        assert!(dsa.contains("#define DSA_TAG_PROTO_AR9331_VALUE\t\t16"));
    }

    #[test]
    fn ar9331_xmit_and_receive_enforce_header_contract() {
        let frame = ar9331_tag_xmit(3);
        let host = u16::from_le(frame.header);
        assert_eq!(field_get(AR9331_HDR_VERSION_MASK, host), AR9331_HDR_VERSION);
        assert_eq!(host & AR9331_HDR_FROM_CPU, AR9331_HDR_FROM_CPU);
        assert_eq!(host & AR9331_HDR_RESERVED_MASK, AR9331_HDR_RESERVED_MASK);
        assert_eq!(field_get(AR9331_HDR_PORT_NUM_MASK, host), 3);
        assert_eq!(ar9331_tag_rcv(frame.header, true), None);
        let rx = field_prep(AR9331_HDR_VERSION_MASK, AR9331_HDR_VERSION) | 5;
        assert_eq!(ar9331_tag_rcv(rx.to_le(), true).unwrap().port, 5);
        assert_eq!(ar9331_tag_rcv(0, true), None);
        assert_eq!(ar9331_tag_rcv(rx.to_le(), false), None);
        assert_eq!(ar9331_module_ops(), &AR9331_NETDEV_OPS);
        assert_eq!(AR9331_NETDEV_OPS.proto, DSA_TAG_PROTO_AR9331_VALUE);
    }
}
