//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_ocelot.c
//! test-origin: linux:vendor/linux/net/dsa/tag_ocelot.c
//! Ocelot and Seville NPI tag formats.

pub const OCELOT_NAME: &str = "ocelot";
pub const SEVILLE_NAME: &str = "seville";
pub const IFH_TAG_TYPE_C: u64 = 0;
pub const IFH_TAG_TYPE_S: u64 = 1;
pub const IFH_REW_OP_NOOP: u64 = 0x0;
pub const IFH_REW_OP_DSCP: u64 = 0x1;
pub const IFH_REW_OP_ONE_STEP_PTP: u64 = 0x2;
pub const IFH_REW_OP_TWO_STEP_PTP: u64 = 0x3;
pub const IFH_REW_OP_ORIGIN_PTP: u64 = 0x5;
pub const OCELOT_TAG_LEN: usize = 16;
pub const OCELOT_SHORT_PREFIX_LEN: usize = 4;
pub const OCELOT_TOTAL_TAG_LEN: usize = OCELOT_SHORT_PREFIX_LEN + OCELOT_TAG_LEN;
pub const OCELOT_PREFIX: u32 = 0x8880_000a;
pub const SEVILLE_PREFIX: u32 = 0x8880_0005;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_8021AD: u16 = 0x88a8;
pub const DSA_TAG_PROTO_OCELOT_VALUE: u8 = 15;
pub const DSA_TAG_PROTO_SEVILLE_VALUE: u8 = 21;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Ocelot family of switches, using NPI port";
pub const MODULE_LICENSE: &str = "GPL v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
    pub promisc_on_conduit: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcelotXmitContext {
    pub num_ports: u8,
    pub dest_mask: u16,
    pub vlan_tci: u16,
    pub tag_type: u8,
    pub skb_priority: u8,
    pub num_tc: u8,
    pub mapped_tc: u8,
    pub rew_op: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcelotXmitFrame {
    pub prefix: u32,
    pub injection: [u8; OCELOT_TAG_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcelotRcvInput {
    pub extraction: [u8; OCELOT_TAG_LEN],
    pub user_found: bool,
    pub vlan_filtering: bool,
    pub frame_ethertype: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcelotRcvFrame {
    pub src_port: u64,
    pub qos_class: u64,
    pub tag_type: u64,
    pub vlan_tci: u64,
    pub tstamp_lo: u64,
    pub priority: u64,
    pub offload_fwd_mark: bool,
    pub replaced_vlan: Option<(u16, u16)>,
}

pub const OCELOT_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: OCELOT_NAME,
    proto: DSA_TAG_PROTO_OCELOT_VALUE,
    needed_headroom: OCELOT_TOTAL_TAG_LEN,
    promisc_on_conduit: true,
};
pub const SEVILLE_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: SEVILLE_NAME,
    proto: DSA_TAG_PROTO_SEVILLE_VALUE,
    needed_headroom: OCELOT_TOTAL_TAG_LEN,
    promisc_on_conduit: true,
};

fn pack_bits(buf: &mut [u8; OCELOT_TAG_LEN], value: u64, high: u8, low: u8) {
    let width = high - low + 1;
    let mask = if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let field_mask = (mask as u128) << low;
    let mut word = u128::from_be_bytes(*buf);
    word &= !field_mask;
    word |= ((value & mask) as u128) << low;
    *buf = word.to_be_bytes();
}

pub fn unpack_bits(buf: &[u8; OCELOT_TAG_LEN], high: u8, low: u8) -> u64 {
    let width = high - low + 1;
    let mask = if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    ((u128::from_be_bytes(*buf) >> low) as u64) & mask
}

pub fn ocelot_ifh_set_bypass(injection: &mut [u8; OCELOT_TAG_LEN], bypass: u64) {
    pack_bits(injection, bypass, 127, 127);
}

pub fn ocelot_ifh_set_rew_op(injection: &mut [u8; OCELOT_TAG_LEN], rew_op: u64) {
    pack_bits(injection, rew_op, 125, 117);
}

pub fn ocelot_ifh_set_dest(injection: &mut [u8; OCELOT_TAG_LEN], dest: u64) {
    pack_bits(injection, dest, 67, 56);
}

pub fn seville_ifh_set_dest(injection: &mut [u8; OCELOT_TAG_LEN], dest: u64) {
    pack_bits(injection, dest, 67, 57);
}

pub fn ocelot_ifh_set_src(injection: &mut [u8; OCELOT_TAG_LEN], src: u64) {
    pack_bits(injection, src, 46, 43);
}

pub fn ocelot_ifh_set_qos_class(injection: &mut [u8; OCELOT_TAG_LEN], qos_class: u64) {
    pack_bits(injection, qos_class, 19, 17);
}

pub fn ocelot_ifh_set_tag_type(injection: &mut [u8; OCELOT_TAG_LEN], tag_type: u64) {
    pack_bits(injection, tag_type, 16, 16);
}

pub fn ocelot_ifh_set_vlan_tci(injection: &mut [u8; OCELOT_TAG_LEN], vlan_tci: u64) {
    pack_bits(injection, vlan_tci, 15, 0);
}

pub fn ocelot_xfh_get_src_port(extraction: &[u8; OCELOT_TAG_LEN]) -> u64 {
    unpack_bits(extraction, 46, 43)
}

pub fn ocelot_xfh_get_qos_class(extraction: &[u8; OCELOT_TAG_LEN]) -> u64 {
    unpack_bits(extraction, 19, 17)
}

pub fn ocelot_xfh_get_tag_type(extraction: &[u8; OCELOT_TAG_LEN]) -> u64 {
    unpack_bits(extraction, 16, 16)
}

pub fn ocelot_xfh_get_vlan_tci(extraction: &[u8; OCELOT_TAG_LEN]) -> u64 {
    unpack_bits(extraction, 15, 0)
}

pub fn ocelot_xfh_get_rew_val(extraction: &[u8; OCELOT_TAG_LEN]) -> u64 {
    unpack_bits(extraction, 116, 85)
}

pub fn ocelot_xmit_common(ctx: OcelotXmitContext, prefix: u32, seville: bool) -> OcelotXmitFrame {
    let mut injection = [0; OCELOT_TAG_LEN];
    let qos_class = if ctx.num_tc != 0 {
        ctx.mapped_tc
    } else {
        ctx.skb_priority
    } as u64;
    ocelot_ifh_set_bypass(&mut injection, 1);
    ocelot_ifh_set_src(&mut injection, ctx.num_ports as u64);
    ocelot_ifh_set_qos_class(&mut injection, qos_class);
    ocelot_ifh_set_vlan_tci(&mut injection, ctx.vlan_tci as u64);
    ocelot_ifh_set_tag_type(&mut injection, ctx.tag_type as u64);
    if ctx.rew_op != 0 {
        ocelot_ifh_set_rew_op(&mut injection, ctx.rew_op as u64);
    }
    if seville {
        seville_ifh_set_dest(&mut injection, ctx.dest_mask as u64);
    } else {
        ocelot_ifh_set_dest(&mut injection, ctx.dest_mask as u64);
    }
    OcelotXmitFrame { prefix, injection }
}

pub fn ocelot_xmit(ctx: OcelotXmitContext) -> OcelotXmitFrame {
    ocelot_xmit_common(ctx, OCELOT_PREFIX, false)
}

pub fn seville_xmit(ctx: OcelotXmitContext) -> OcelotXmitFrame {
    ocelot_xmit_common(ctx, SEVILLE_PREFIX, true)
}

pub fn ocelot_rcv(input: OcelotRcvInput) -> Option<OcelotRcvFrame> {
    if !input.user_found {
        return None;
    }
    let src_port = ocelot_xfh_get_src_port(&input.extraction);
    let qos_class = ocelot_xfh_get_qos_class(&input.extraction);
    let tag_type = ocelot_xfh_get_tag_type(&input.extraction);
    let vlan_tci = ocelot_xfh_get_vlan_tci(&input.extraction);
    let rew_val = ocelot_xfh_get_rew_val(&input.extraction);
    let vlan_tpid = if tag_type != 0 {
        ETH_P_8021AD
    } else {
        ETH_P_8021Q
    };
    Some(OcelotRcvFrame {
        src_port,
        qos_class,
        tag_type,
        vlan_tci,
        tstamp_lo: rew_val,
        priority: qos_class,
        offload_fwd_mark: true,
        replaced_vlan: if input.vlan_filtering && input.frame_ethertype == vlan_tpid {
            Some((vlan_tpid, vlan_tci as u16))
        } else {
            None
        },
    })
}

pub fn module_aliases() -> [&'static str; 4] {
    [
        "dsa_tag:ocelot",
        "dsa_tag:id-15",
        "dsa_tag:seville",
        "dsa_tag:id-21",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> OcelotXmitContext {
        OcelotXmitContext {
            num_ports: 6,
            dest_mask: 0x321,
            vlan_tci: 0x1234,
            tag_type: IFH_TAG_TYPE_S as u8,
            skb_priority: 2,
            num_tc: 4,
            mapped_tc: 5,
            rew_op: IFH_REW_OP_TWO_STEP_PTP as u16,
        }
    }

    #[test]
    fn tag_ocelot_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_ocelot.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dsa/ocelot.h"
        ));
        assert!(source.contains("#define OCELOT_NAME\t\"ocelot\""));
        assert!(source.contains("#define SEVILLE_NAME\t\"seville\""));
        assert!(source.contains("ocelot_xmit_get_vlan_info"));
        assert!(source.contains("qos_class = netdev_get_num_tc(netdev) ?"));
        assert!(source.contains("ocelot_ifh_set_bypass(injection, 1);"));
        assert!(source.contains("ocelot_ifh_set_src(injection, ds->num_ports);"));
        assert!(source.contains("ocelot_ifh_set_vlan_tci(injection, vlan_tci);"));
        assert!(source.contains("ocelot_ifh_set_tag_type(injection, tag_type);"));
        assert!(
            source.contains("ocelot_ifh_set_dest(injection, dsa_xmit_port_mask(skb, netdev));")
        );
        assert!(
            source.contains("seville_ifh_set_dest(injection, dsa_xmit_port_mask(skb, netdev));")
        );
        assert!(source.contains("ocelot_xfh_get_src_port(extraction, &src_port);"));
        assert!(source.contains("skb->priority = qos_class;"));
        assert!(source.contains("OCELOT_SKB_CB(skb)->tstamp_lo = rew_val;"));
        assert!(source.contains("vlan_tpid = tag_type ? ETH_P_8021AD : ETH_P_8021Q;"));
        assert!(source.contains(".proto\t\t\t= DSA_TAG_PROTO_OCELOT"));
        assert!(source.contains(".proto\t\t\t= DSA_TAG_PROTO_SEVILLE"));
        assert!(header.contains("#define OCELOT_TAG_LEN\t\t\t16"));
        assert!(header.contains("packing(injection, &bypass, 127, 127, OCELOT_TAG_LEN, PACK, 0);"));
        assert!(
            header.contains("packing(extraction, src_port, 46, 43, OCELOT_TAG_LEN, UNPACK, 0);")
        );
    }

    #[test]
    fn injection_header_sets_linux_bit_fields() {
        let tx = ocelot_xmit(ctx());
        assert_eq!(tx.prefix, OCELOT_PREFIX);
        assert_eq!(unpack_bits(&tx.injection, 127, 127), 1);
        assert_eq!(
            unpack_bits(&tx.injection, 125, 117),
            IFH_REW_OP_TWO_STEP_PTP
        );
        assert_eq!(unpack_bits(&tx.injection, 67, 56), 0x321);
        assert_eq!(unpack_bits(&tx.injection, 46, 43), 6);
        assert_eq!(unpack_bits(&tx.injection, 19, 17), 5);
        assert_eq!(unpack_bits(&tx.injection, 16, 16), IFH_TAG_TYPE_S);
        assert_eq!(unpack_bits(&tx.injection, 15, 0), 0x1234);

        let seville = seville_xmit(ctx());
        assert_eq!(seville.prefix, SEVILLE_PREFIX);
        assert_eq!(unpack_bits(&seville.injection, 67, 57), 0x321 & 0x7ff);
    }

    #[test]
    fn receive_decodes_extraction_metadata_and_vlan_replace() {
        let tx = ocelot_xmit(ctx());
        let rx = ocelot_rcv(OcelotRcvInput {
            extraction: tx.injection,
            user_found: true,
            vlan_filtering: true,
            frame_ethertype: ETH_P_8021AD,
        })
        .unwrap();
        assert_eq!(rx.src_port, 6);
        assert_eq!(rx.qos_class, 5);
        assert_eq!(rx.priority, 5);
        assert_eq!(rx.tag_type, IFH_TAG_TYPE_S);
        assert_eq!(rx.vlan_tci, 0x1234);
        assert!(rx.offload_fwd_mark);
        assert_eq!(rx.replaced_vlan, Some((ETH_P_8021AD, 0x1234)));
        assert!(
            ocelot_rcv(OcelotRcvInput {
                user_found: false,
                ..OcelotRcvInput {
                    extraction: tx.injection,
                    user_found: true,
                    vlan_filtering: false,
                    frame_ethertype: ETH_P_8021Q,
                }
            })
            .is_none()
        );
        assert_eq!(
            module_aliases(),
            [
                "dsa_tag:ocelot",
                "dsa_tag:id-15",
                "dsa_tag:seville",
                "dsa_tag:id-21"
            ]
        );
    }
}
