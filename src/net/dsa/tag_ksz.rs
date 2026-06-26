//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_ksz.c
//! test-origin: linux:vendor/linux/net/dsa/tag_ksz.c
//! Microchip KSZ tail tag formats.

pub const KSZ8795_NAME: &str = "ksz8795";
pub const KSZ9477_NAME: &str = "ksz9477";
pub const KSZ9893_NAME: &str = "ksz9893";
pub const LAN937X_NAME: &str = "lan937x";
pub const KSZ_PTP_TAG_LEN: usize = 4;
pub const KSZ_EGRESS_TAG_LEN: usize = 1;
pub const KSZ_INGRESS_TAG_LEN: usize = 1;
pub const KSZ_HWTS_EN: u8 = 0;
pub const KSZ8795_TAIL_TAG_EG_PORT_M: u8 = 0x03;
pub const KSZ8795_TAIL_TAG_OVERRIDE: u8 = 1 << 6;
pub const KSZ8795_TAIL_TAG_LOOKUP: u8 = 1 << 7;
pub const KSZ9477_INGRESS_TAG_LEN: usize = 2;
pub const KSZ9477_PTP_TAG_LEN: usize = 4;
pub const KSZ9477_PTP_TAG_INDICATION: u8 = 1 << 7;
pub const KSZ9477_TAIL_TAG_EG_PORT_M: u8 = 0x07;
pub const KSZ9477_TAIL_TAG_PRIO: u16 = 0x0180;
pub const KSZ9477_TAIL_TAG_OVERRIDE: u16 = 1 << 9;
pub const KSZ9477_TAIL_TAG_LOOKUP: u16 = 1 << 10;
pub const KSZ9893_TAIL_TAG_PRIO: u8 = 0x18;
pub const KSZ9893_TAIL_TAG_OVERRIDE: u8 = 1 << 5;
pub const KSZ9893_TAIL_TAG_LOOKUP: u8 = 1 << 6;
pub const LAN937X_EGRESS_TAG_LEN: usize = 2;
pub const LAN937X_TAIL_TAG_BLOCKING_OVERRIDE: u16 = 1 << 11;
pub const LAN937X_TAIL_TAG_LOOKUP: u16 = 1 << 12;
pub const LAN937X_TAIL_TAG_VALID: u16 = 1 << 13;
pub const LAN937X_TAIL_TAG_PRIO: u16 = 0x0700;
pub const LAN937X_TAIL_TAG_PORT_MASK: u8 = 7;
pub const DSA_TAG_PROTO_KSZ9477_VALUE: u8 = 6;
pub const DSA_TAG_PROTO_KSZ9893_VALUE: u8 = 7;
pub const DSA_TAG_PROTO_KSZ8795_VALUE: u8 = 14;
pub const DSA_TAG_PROTO_LAN937X_VALUE: u8 = 27;
pub const MODULE_DESCRIPTION: &str =
    "DSA tag driver for Microchip 8795/937x/9477/9893 families of switches";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_tailroom: usize,
    pub connect: bool,
    pub disconnect: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KszTaggerPrivate {
    pub hwtstamp_enabled: bool,
    pub xmit_worker_running: bool,
    pub xmit_work_fn_registered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KszTxContext {
    pub port_mask: u16,
    pub queue_mapping: u16,
    pub link_local_dest: bool,
    pub checksum_partial: bool,
    pub checksum_help_failed: bool,
    pub hwtstamp_enabled: bool,
    pub update_correction: bool,
    pub correction: i64,
    pub clone_present: bool,
    pub xmit_worker_running: bool,
    pub xmit_work_fn_registered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KszTxAction<const N: usize> {
    Tail { bytes: [u8; N], len: usize },
    DeferredQueued,
    Drop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KszRxFrame {
    pub source_port: u8,
    pub strip_len: usize,
    pub offload_fwd_mark: bool,
    pub timestamp_ns: Option<u64>,
}

pub const KSZ8795_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: KSZ8795_NAME,
    proto: DSA_TAG_PROTO_KSZ8795_VALUE,
    needed_tailroom: KSZ_INGRESS_TAG_LEN,
    connect: false,
    disconnect: false,
};
pub const KSZ9477_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: KSZ9477_NAME,
    proto: DSA_TAG_PROTO_KSZ9477_VALUE,
    needed_tailroom: KSZ9477_INGRESS_TAG_LEN + KSZ_PTP_TAG_LEN,
    connect: true,
    disconnect: true,
};
pub const KSZ9893_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: KSZ9893_NAME,
    proto: DSA_TAG_PROTO_KSZ9893_VALUE,
    needed_tailroom: KSZ_INGRESS_TAG_LEN + KSZ_PTP_TAG_LEN,
    connect: true,
    disconnect: true,
};
pub const LAN937X_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: LAN937X_NAME,
    proto: DSA_TAG_PROTO_LAN937X_VALUE,
    needed_tailroom: LAN937X_EGRESS_TAG_LEN + KSZ_PTP_TAG_LEN,
    connect: true,
    disconnect: true,
};

const fn field_prep_u16(mask: u16, value: u16) -> u16 {
    (value << mask.trailing_zeros()) & mask
}

const fn field_get_u32(mask: u32, value: u32) -> u32 {
    (value & mask) >> mask.trailing_zeros()
}

pub const fn ksz_connect() -> KszTaggerPrivate {
    KszTaggerPrivate {
        hwtstamp_enabled: false,
        xmit_worker_running: true,
        xmit_work_fn_registered: false,
    }
}

pub const fn ksz_hwtstamp_set_state(mut priv_data: KszTaggerPrivate, on: bool) -> KszTaggerPrivate {
    priv_data.hwtstamp_enabled = on;
    priv_data
}

pub const fn ksz_disconnect(_priv_data: KszTaggerPrivate) -> KszTaggerPrivate {
    KszTaggerPrivate {
        hwtstamp_enabled: false,
        xmit_worker_running: false,
        xmit_work_fn_registered: false,
    }
}

pub const fn ksz_decode_tstamp(tstamp: u32) -> u64 {
    (field_get_u32(0xc000_0000, tstamp) as u64) * 1_000_000_000
        + field_get_u32(0x3fff_ffff, tstamp) as u64
}

pub const fn ksz_xmit_timestamp(ctx: KszTxContext) -> Option<[u8; KSZ_PTP_TAG_LEN]> {
    if !ctx.hwtstamp_enabled {
        return None;
    }
    let mut raw = 0u32;
    if ctx.update_correction && ctx.correction < 0 {
        let ns = ((-ctx.correction) as u64) >> 16;
        let sec = ns / 1_000_000_000;
        let nsec = ns % 1_000_000_000;
        raw = (((sec & 3) as u32) << 30) | nsec as u32;
    }
    Some(raw.to_be_bytes())
}

pub const fn ksz_defer_xmit(ctx: KszTxContext) -> bool {
    ctx.clone_present && ctx.xmit_work_fn_registered && ctx.xmit_worker_running
}

pub const fn ksz8795_xmit(ctx: KszTxContext) -> KszTxAction<1> {
    if ctx.checksum_partial && ctx.checksum_help_failed {
        return KszTxAction::Drop;
    }
    let mut tag = (ctx.port_mask & 0xff) as u8;
    if ctx.link_local_dest {
        tag |= KSZ8795_TAIL_TAG_OVERRIDE;
    }
    KszTxAction::Tail {
        bytes: [tag],
        len: KSZ_INGRESS_TAG_LEN,
    }
}

pub const fn ksz8795_rcv(tag: u8, user_found: bool) -> Option<KszRxFrame> {
    if !user_found {
        return None;
    }
    Some(KszRxFrame {
        source_port: tag & KSZ8795_TAIL_TAG_EG_PORT_M,
        strip_len: KSZ_EGRESS_TAG_LEN,
        offload_fwd_mark: true,
        timestamp_ns: None,
    })
}

pub const fn ksz9477_xmit(ctx: KszTxContext) -> KszTxAction<6> {
    if ctx.checksum_partial && ctx.checksum_help_failed {
        return KszTxAction::Drop;
    }
    let mut bytes = [0; 6];
    let mut len = 0;
    if let Some(ts) = ksz_xmit_timestamp(ctx) {
        bytes[0] = ts[0];
        bytes[1] = ts[1];
        bytes[2] = ts[2];
        bytes[3] = ts[3];
        len = 4;
    }
    let prio = (ctx.queue_mapping & 0x7) as u16;
    let mut val = ctx.port_mask | field_prep_u16(KSZ9477_TAIL_TAG_PRIO, prio);
    if ctx.link_local_dest {
        val |= KSZ9477_TAIL_TAG_OVERRIDE;
    }
    bytes[len] = (val >> 8) as u8;
    bytes[len + 1] = val as u8;
    len += KSZ9477_INGRESS_TAG_LEN;
    if ksz_defer_xmit(ctx) {
        KszTxAction::DeferredQueued
    } else {
        KszTxAction::Tail { bytes, len }
    }
}

pub const fn ksz9477_rcv(tag: &[u8], user_found: bool) -> Option<KszRxFrame> {
    if !user_found || tag.is_empty() {
        return None;
    }
    let tail = tag[tag.len() - 1];
    let mut strip_len = KSZ_EGRESS_TAG_LEN;
    let mut timestamp_ns = None;
    if (tail & KSZ9477_PTP_TAG_INDICATION) != 0 {
        if tag.len() < KSZ_PTP_TAG_LEN + KSZ_EGRESS_TAG_LEN {
            return None;
        }
        let raw = u32::from_be_bytes([
            tag[tag.len() - 5],
            tag[tag.len() - 4],
            tag[tag.len() - 3],
            tag[tag.len() - 2],
        ]);
        strip_len += KSZ_PTP_TAG_LEN;
        timestamp_ns = Some(ksz_decode_tstamp(raw));
    }
    Some(KszRxFrame {
        source_port: tail & KSZ9477_TAIL_TAG_EG_PORT_M,
        strip_len,
        offload_fwd_mark: true,
        timestamp_ns,
    })
}

pub const fn ksz9893_xmit(ctx: KszTxContext) -> KszTxAction<5> {
    if ctx.checksum_partial && ctx.checksum_help_failed {
        return KszTxAction::Drop;
    }
    let mut bytes = [0; 5];
    let mut len = 0;
    if let Some(ts) = ksz_xmit_timestamp(ctx) {
        bytes[0] = ts[0];
        bytes[1] = ts[1];
        bytes[2] = ts[2];
        bytes[3] = ts[3];
        len = 4;
    }
    let prio = (ctx.queue_mapping & 0x3) as u8;
    let mut tag = (ctx.port_mask & 0xff) as u8 | ((prio << 3) & KSZ9893_TAIL_TAG_PRIO);
    if ctx.link_local_dest {
        tag |= KSZ9893_TAIL_TAG_OVERRIDE;
    }
    bytes[len] = tag;
    len += KSZ_INGRESS_TAG_LEN;
    if ksz_defer_xmit(ctx) {
        KszTxAction::DeferredQueued
    } else {
        KszTxAction::Tail { bytes, len }
    }
}

pub const fn lan937x_xmit(ctx: KszTxContext) -> KszTxAction<6> {
    if ctx.checksum_partial && ctx.checksum_help_failed {
        return KszTxAction::Drop;
    }
    let mut bytes = [0; 6];
    let mut len = 0;
    if let Some(ts) = ksz_xmit_timestamp(ctx) {
        bytes[0] = ts[0];
        bytes[1] = ts[1];
        bytes[2] = ts[2];
        bytes[3] = ts[3];
        len = 4;
    }
    let prio = (ctx.queue_mapping & 0x7) as u16;
    let mut val = ctx.port_mask | field_prep_u16(LAN937X_TAIL_TAG_PRIO, prio);
    if ctx.link_local_dest {
        val |= LAN937X_TAIL_TAG_BLOCKING_OVERRIDE;
    }
    val |= LAN937X_TAIL_TAG_VALID;
    bytes[len] = (val >> 8) as u8;
    bytes[len + 1] = val as u8;
    len += LAN937X_EGRESS_TAG_LEN;
    if ksz_defer_xmit(ctx) {
        KszTxAction::DeferredQueued
    } else {
        KszTxAction::Tail { bytes, len }
    }
}

pub fn module_aliases() -> [&'static str; 8] {
    [
        "dsa_tag:ksz8795",
        "dsa_tag:id-14",
        "dsa_tag:ksz9477",
        "dsa_tag:id-6",
        "dsa_tag:ksz9893",
        "dsa_tag:id-7",
        "dsa_tag:lan937x",
        "dsa_tag:id-27",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> KszTxContext {
        KszTxContext {
            port_mask: 0x05,
            queue_mapping: 3,
            link_local_dest: false,
            checksum_partial: false,
            checksum_help_failed: false,
            hwtstamp_enabled: false,
            update_correction: false,
            correction: 0,
            clone_present: false,
            xmit_worker_running: false,
            xmit_work_fn_registered: false,
        }
    }

    #[test]
    fn tag_ksz_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_ksz.c"
        ));
        let common = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dsa/ksz_common.h"
        ));
        assert!(source.contains("#define KSZ8795_NAME \"ksz8795\""));
        assert!(source.contains("#define KSZ9477_NAME \"ksz9477\""));
        assert!(source.contains("#define LAN937X_NAME \"lan937x\""));
        assert!(source.contains("#define KSZ_PTP_TAG_LEN\t\t\t4"));
        assert!(source.contains("#define KSZ8795_TAIL_TAG_EG_PORT_M\tGENMASK(1, 0)"));
        assert!(source.contains("*tag = dsa_xmit_port_mask(skb, dev);"));
        assert!(source.contains("*tag |= KSZ8795_TAIL_TAG_OVERRIDE;"));
        assert!(source.contains("port = tag[0] & KSZ9477_TAIL_TAG_EG_PORT_M;"));
        assert!(source.contains("if (tag[0] & KSZ9477_PTP_TAG_INDICATION)"));
        assert!(source.contains("val |= FIELD_PREP(KSZ9477_TAIL_TAG_PRIO, prio);"));
        assert!(source.contains("*tag |= FIELD_PREP(KSZ9893_TAIL_TAG_PRIO, prio);"));
        assert!(source.contains("val |= LAN937X_TAIL_TAG_VALID;"));
        assert!(source.contains(".needed_tailroom = KSZ9477_INGRESS_TAG_LEN + KSZ_PTP_TAG_LEN"));
        assert!(common.contains("#define KSZ_TSTAMP_SEC_MASK  GENMASK(31, 30)"));
        assert!(common.contains("FIELD_GET(KSZ_TSTAMP_NSEC_MASK, tstamp)"));
    }

    #[test]
    fn ksz8795_tail_tag_sets_port_and_override() {
        assert_eq!(
            ksz8795_xmit(KszTxContext {
                link_local_dest: true,
                ..ctx()
            }),
            KszTxAction::Tail {
                bytes: [KSZ8795_TAIL_TAG_OVERRIDE | 0x05],
                len: 1
            }
        );
        assert_eq!(ksz8795_rcv(0x83, true).unwrap().source_port, 3);
    }

    #[test]
    fn ksz9477_and_9893_encode_priority_timestamp_and_defer() {
        let hw = KszTxContext {
            hwtstamp_enabled: true,
            update_correction: true,
            correction: -((1_000_000_123i64) << 16),
            ..ctx()
        };
        assert_eq!(ksz_xmit_timestamp(hw), Some(0x4000_007bu32.to_be_bytes()));
        assert_eq!(ksz_decode_tstamp(0x8000_0005), 2_000_000_005);

        let tx = ksz9477_xmit(hw);
        let KszTxAction::Tail { bytes, len } = tx else {
            panic!("expected tail tag");
        };
        assert_eq!(len, 6);
        assert_eq!(&bytes[..4], &0x4000_007bu32.to_be_bytes());
        assert_eq!(u16::from_be_bytes([bytes[4], bytes[5]]), 0x0180 | 0x05);

        let rx = ksz9477_rcv(&[0x80, 0x00, 0x00, 0x09, 0x82], true).unwrap();
        assert_eq!(rx.source_port, 2);
        assert_eq!(rx.strip_len, 5);
        assert_eq!(rx.timestamp_ns, Some(2_000_000_009));

        assert_eq!(
            ksz9893_xmit(KszTxContext {
                clone_present: true,
                xmit_worker_running: true,
                xmit_work_fn_registered: true,
                ..ctx()
            }),
            KszTxAction::DeferredQueued
        );
    }

    #[test]
    fn lan937x_sets_valid_priority_and_override_bits() {
        let tx = lan937x_xmit(KszTxContext {
            link_local_dest: true,
            ..ctx()
        });
        let KszTxAction::Tail { bytes, len } = tx else {
            panic!("expected tail tag");
        };
        assert_eq!(len, 2);
        let val = u16::from_be_bytes([bytes[0], bytes[1]]);
        assert_eq!(
            val,
            LAN937X_TAIL_TAG_VALID | LAN937X_TAIL_TAG_BLOCKING_OVERRIDE | 0x0300 | 0x05
        );
        assert!(
            ksz_disconnect(ksz_hwtstamp_set_state(ksz_connect(), true)).xmit_worker_running
                == false
        );
        assert_eq!(
            LAN937X_NETDEV_OPS.needed_tailroom,
            LAN937X_EGRESS_TAG_LEN + KSZ_PTP_TAG_LEN
        );
    }
}
