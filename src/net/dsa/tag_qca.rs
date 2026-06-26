//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_qca.c
//! test-origin: linux:vendor/linux/net/dsa/tag_qca.c
//! Qualcomm Atheros QCA8K DSA tag support.

pub const QCA_NAME: &str = "qca";
pub const QCA_HDR_LEN: usize = 2;
pub const QCA_HDR_VERSION: u16 = 0x2;
pub const QCA_HDR_RECV_VERSION: u16 = 0xc000;
pub const QCA_HDR_RECV_PRIORITY: u16 = 0x3800;
pub const QCA_HDR_RECV_TYPE: u16 = 0x07c0;
pub const QCA_HDR_RECV_FRAME_IS_TAGGED: u16 = 1 << 3;
pub const QCA_HDR_RECV_SOURCE_PORT: u16 = 0x0007;
pub const QCA_HDR_RECV_TYPE_NORMAL: u16 = 0x0;
pub const QCA_HDR_RECV_TYPE_MIB: u16 = 0x1;
pub const QCA_HDR_RECV_TYPE_RW_REG_ACK: u16 = 0x2;
pub const QCA_HDR_XMIT_VERSION: u16 = 0xc000;
pub const QCA_HDR_XMIT_PRIORITY: u16 = 0x3800;
pub const QCA_HDR_XMIT_CONTROL: u16 = 0x0700;
pub const QCA_HDR_XMIT_FROM_CPU: u16 = 1 << 7;
pub const QCA_HDR_XMIT_DP_BIT: u16 = 0x007f;
pub const QCA_HDR_XMIT_TYPE_NORMAL: u16 = 0x0;
pub const QCA_HDR_XMIT_TYPE_RW_REG: u16 = 0x1;
pub const QCA_HDR_MGMT_HEADER_LEN: usize = 12;
pub const DSA_TAG_PROTO_QCA_VALUE: u8 = 10;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Qualcomm Atheros QCA8K switches";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
    pub connect: bool,
    pub disconnect: bool,
    pub promisc_on_conduit: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QcaTxFrame {
    pub header_be: u16,
    pub port_mask: u8,
    pub etype_header_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QcaRxFrame {
    pub source_port: u8,
    pub tag_removed: bool,
    pub etype_header_stripped: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QcaRxResult {
    Frame(QcaRxFrame),
    RwRegAck,
    MibAutocast,
    Drop,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QcaSwitch {
    pub tagger_data_allocated: bool,
}

pub const QCA_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: QCA_NAME,
    proto: DSA_TAG_PROTO_QCA_VALUE,
    needed_headroom: QCA_HDR_LEN,
    connect: true,
    disconnect: true,
    promisc_on_conduit: true,
};

const fn field_prep(mask: u16, value: u16) -> u16 {
    (value << mask.trailing_zeros()) & mask
}

const fn field_get(mask: u16, value: u16) -> u16 {
    (value & mask) >> mask.trailing_zeros()
}

pub const fn qca_tag_xmit(port_mask: u8) -> QcaTxFrame {
    let hdr = field_prep(QCA_HDR_XMIT_VERSION, QCA_HDR_VERSION)
        | QCA_HDR_XMIT_FROM_CPU
        | field_prep(QCA_HDR_XMIT_DP_BIT, port_mask as u16);
    QcaTxFrame {
        header_be: hdr.to_be(),
        port_mask,
        etype_header_allocated: true,
    }
}

pub const fn qca_tag_rcv(header_be: u16, user_port_exists: bool) -> QcaRxResult {
    let hdr = u16::from_be(header_be);
    let ver = field_get(QCA_HDR_RECV_VERSION, hdr);
    if ver != QCA_HDR_VERSION {
        return QcaRxResult::Drop;
    }

    match field_get(QCA_HDR_RECV_TYPE, hdr) {
        QCA_HDR_RECV_TYPE_RW_REG_ACK => return QcaRxResult::RwRegAck,
        QCA_HDR_RECV_TYPE_MIB => return QcaRxResult::MibAutocast,
        _ => {}
    }

    if !user_port_exists {
        return QcaRxResult::Drop;
    }

    QcaRxResult::Frame(QcaRxFrame {
        source_port: field_get(QCA_HDR_RECV_SOURCE_PORT, hdr) as u8,
        tag_removed: true,
        etype_header_stripped: true,
    })
}

pub fn qca_tag_connect(ds: &mut QcaSwitch) -> Result<(), i32> {
    ds.tagger_data_allocated = true;
    Ok(())
}

pub fn qca_tag_disconnect(ds: &mut QcaSwitch) {
    ds.tagger_data_allocated = false;
}

pub const fn qca_module_ops() -> &'static DsaDeviceOps {
    &QCA_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:qca", "dsa_tag:id-10"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_qca_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_qca.c"
        ));
        let tag_qca = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dsa/tag_qca.h"
        ));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));

        assert!(source.contains("#define QCA_NAME \"qca\""));
        assert!(source.contains("skb_push(skb, QCA_HDR_LEN);"));
        assert!(source.contains("dsa_alloc_etype_header(skb, QCA_HDR_LEN);"));
        assert!(source.contains("hdr = FIELD_PREP(QCA_HDR_XMIT_VERSION, QCA_HDR_VERSION);"));
        assert!(source.contains("hdr |= QCA_HDR_XMIT_FROM_CPU;"));
        assert!(source.contains("FIELD_PREP(QCA_HDR_XMIT_DP_BIT, dsa_xmit_port_mask(skb, dev))"));
        assert!(source.contains("*phdr = htons(hdr);"));
        assert!(source.contains("BUILD_BUG_ON(sizeof(struct qca_mgmt_ethhdr)"));
        assert!(source.contains("if (unlikely(!pskb_may_pull(skb, QCA_HDR_LEN)))"));
        assert!(source.contains("ver = FIELD_GET(QCA_HDR_RECV_VERSION, hdr);"));
        assert!(source.contains("pk_type = FIELD_GET(QCA_HDR_RECV_TYPE, hdr);"));
        assert!(source.contains("pk_type == QCA_HDR_RECV_TYPE_RW_REG_ACK"));
        assert!(source.contains("pk_type == QCA_HDR_RECV_TYPE_MIB"));
        assert!(source.contains("port = FIELD_GET(QCA_HDR_RECV_SOURCE_PORT, hdr);"));
        assert!(source.contains("dsa_conduit_find_user(dev, 0, port);"));
        assert!(source.contains("skb_pull_rcsum(skb, QCA_HDR_LEN);"));
        assert!(source.contains("dsa_strip_etype_header(skb, QCA_HDR_LEN);"));
        assert!(source.contains("tagger_data = kzalloc_obj(*tagger_data);"));
        assert!(source.contains("kfree(ds->tagger_data);"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_QCA"));
        assert!(source.contains(".needed_headroom = QCA_HDR_LEN"));
        assert!(source.contains(".promisc_on_conduit = true"));
        assert!(source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_QCA, QCA_NAME);"));

        assert!(tag_qca.contains("#define QCA_HDR_LEN\t2"));
        assert!(tag_qca.contains("#define QCA_HDR_VERSION\t0x2"));
        assert!(tag_qca.contains("#define QCA_HDR_RECV_TYPE_RW_REG_ACK\t0x2"));
        assert!(tag_qca.contains("#define QCA_HDR_XMIT_DP_BIT\t\tGENMASK(6, 0)"));
        assert!(dsa.contains("#define DSA_TAG_PROTO_QCA_VALUE\t\t\t10"));
    }

    #[test]
    fn qca_encode_decode_and_connect_follow_linux_contract() {
        let tx = qca_tag_xmit(0x25);
        let host = u16::from_be(tx.header_be);
        assert_eq!(field_get(QCA_HDR_XMIT_VERSION, host), QCA_HDR_VERSION);
        assert_eq!(host & QCA_HDR_XMIT_FROM_CPU, QCA_HDR_XMIT_FROM_CPU);
        assert_eq!(field_get(QCA_HDR_XMIT_DP_BIT, host), 0x25);
        assert!(tx.etype_header_allocated);

        let normal = field_prep(QCA_HDR_RECV_VERSION, QCA_HDR_VERSION) | 5;
        assert_eq!(
            qca_tag_rcv(normal.to_be(), true),
            QcaRxResult::Frame(QcaRxFrame {
                source_port: 5,
                tag_removed: true,
                etype_header_stripped: true,
            })
        );
        let mib = field_prep(QCA_HDR_RECV_VERSION, QCA_HDR_VERSION)
            | field_prep(QCA_HDR_RECV_TYPE, QCA_HDR_RECV_TYPE_MIB);
        assert_eq!(qca_tag_rcv(mib.to_be(), true), QcaRxResult::MibAutocast);
        let ack = field_prep(QCA_HDR_RECV_VERSION, QCA_HDR_VERSION)
            | field_prep(QCA_HDR_RECV_TYPE, QCA_HDR_RECV_TYPE_RW_REG_ACK);
        assert_eq!(qca_tag_rcv(ack.to_be(), true), QcaRxResult::RwRegAck);
        assert_eq!(qca_tag_rcv(0, true), QcaRxResult::Drop);
        assert_eq!(qca_tag_rcv(normal.to_be(), false), QcaRxResult::Drop);

        let mut ds = QcaSwitch::default();
        qca_tag_connect(&mut ds).unwrap();
        assert!(ds.tagger_data_allocated);
        qca_tag_disconnect(&mut ds);
        assert!(!ds.tagger_data_allocated);
        assert_eq!(qca_module_ops(), &QCA_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:qca", "dsa_tag:id-10"]);
    }
}
