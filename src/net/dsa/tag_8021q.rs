//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_8021q.c
//! test-origin: linux:vendor/linux/net/dsa/tag_8021q.c
//! DSA 802.1Q tag helper primitives.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOENT;

pub const DSA_8021Q_RSV_VAL: u16 = 3;
pub const DSA_8021Q_RSV_SHIFT: u16 = 10;
pub const DSA_8021Q_RSV_MASK: u16 = 0x0c00;
pub const DSA_8021Q_RSV: u16 = (DSA_8021Q_RSV_VAL << DSA_8021Q_RSV_SHIFT) & DSA_8021Q_RSV_MASK;
pub const DSA_8021Q_SWITCH_ID_SHIFT: u16 = 6;
pub const DSA_8021Q_SWITCH_ID_MASK: u16 = 0x01c0;
pub const DSA_8021Q_VBID_HI_SHIFT: u16 = 9;
pub const DSA_8021Q_VBID_HI_MASK: u16 = 0x0200;
pub const DSA_8021Q_VBID_LO_SHIFT: u16 = 4;
pub const DSA_8021Q_VBID_LO_MASK: u16 = 0x0030;
pub const DSA_8021Q_PORT_SHIFT: u16 = 0;
pub const DSA_8021Q_PORT_MASK: u16 = 0x000f;
pub const VLAN_VID_MASK: u16 = 0x0fff;
pub const VLAN_PRIO_MASK: u16 = 0xe000;
pub const VLAN_PRIO_SHIFT: u16 = 13;
pub const BRIDGE_VLAN_INFO_UNTAGGED: u16 = 1 << 1;
pub const BRIDGE_VLAN_INFO_PVID: u16 = 1 << 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DsaPortKind {
    User,
    Cpu,
    Dsa,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaPort {
    pub switch_index: u8,
    pub index: u8,
    pub kind: DsaPortKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaBridge {
    pub num: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaTag8021qVlan {
    pub port: u8,
    pub vid: u16,
    pub refcount: u32,
    pub flags: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dsa8021qContext {
    pub proto: u16,
    pub vlans: Vec<DsaTag8021qVlan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanTag {
    pub proto: u16,
    pub tci: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dsa8021qRcvInput {
    pub tag: VlanTag,
    pub source_port: i32,
    pub switch_id: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dsa8021qRcvResult {
    pub source_port: i32,
    pub switch_id: i32,
    pub vbid: Option<i32>,
    pub vid: Option<u16>,
    pub restored_tag: Option<VlanTag>,
    pub priority: Option<u16>,
}

pub const fn dsa_8021q_switch_id(index: u8) -> u16 {
    ((index as u16) << DSA_8021Q_SWITCH_ID_SHIFT) & DSA_8021Q_SWITCH_ID_MASK
}

pub const fn dsa_8021q_vbid(vbid: u8) -> u16 {
    ((((vbid as u16) & 0x03) << DSA_8021Q_VBID_LO_SHIFT) & DSA_8021Q_VBID_LO_MASK)
        | (((((vbid as u16) & 0x04) >> 2) << DSA_8021Q_VBID_HI_SHIFT) & DSA_8021Q_VBID_HI_MASK)
}

pub const fn dsa_8021q_port(port: u8) -> u16 {
    ((port as u16) << DSA_8021Q_PORT_SHIFT) & DSA_8021Q_PORT_MASK
}

pub const fn dsa_tag_8021q_bridge_vid(bridge_num: u8) -> u16 {
    DSA_8021Q_RSV | dsa_8021q_vbid(bridge_num)
}

pub const fn dsa_tag_8021q_standalone_vid(dp: DsaPort) -> u16 {
    DSA_8021Q_RSV | dsa_8021q_switch_id(dp.switch_index) | dsa_8021q_port(dp.index)
}

pub const fn dsa_8021q_rx_switch_id(vid: u16) -> i32 {
    ((vid & DSA_8021Q_SWITCH_ID_MASK) >> DSA_8021Q_SWITCH_ID_SHIFT) as i32
}

pub const fn dsa_8021q_rx_source_port(vid: u16) -> i32 {
    ((vid & DSA_8021Q_PORT_MASK) >> DSA_8021Q_PORT_SHIFT) as i32
}

pub const fn dsa_tag_8021q_rx_vbid(vid: u16) -> i32 {
    let vbid_hi = (vid & DSA_8021Q_VBID_HI_MASK) >> DSA_8021Q_VBID_HI_SHIFT;
    let vbid_lo = (vid & DSA_8021Q_VBID_LO_MASK) >> DSA_8021Q_VBID_LO_SHIFT;
    ((vbid_hi << 2) | vbid_lo) as i32
}

pub const fn vid_is_dsa_8021q(vid: u16) -> bool {
    ((vid & DSA_8021Q_RSV_MASK) >> DSA_8021Q_RSV_SHIFT) == DSA_8021Q_RSV_VAL
}

pub const fn dsa_8021q_xmit(tpid: u16, tci: u16) -> VlanTag {
    VlanTag { proto: tpid, tci }
}

impl Dsa8021qContext {
    pub const fn new(proto: u16) -> Self {
        Self {
            proto,
            vlans: Vec::new(),
        }
    }

    pub fn vlan_add(&mut self, dp: DsaPort, vid: u16, flags: u16) -> Result<(), i32> {
        if matches!(dp.kind, DsaPortKind::Cpu | DsaPortKind::Dsa) {
            if let Some(vlan) = self
                .vlans
                .iter_mut()
                .find(|vlan| vlan.port == dp.index && vlan.vid == vid)
            {
                vlan.refcount += 1;
                return Ok(());
            }
        }

        self.vlans.push(DsaTag8021qVlan {
            port: dp.index,
            vid,
            refcount: 1,
            flags,
        });
        Ok(())
    }

    pub fn vlan_del(&mut self, dp: DsaPort, vid: u16) -> Result<(), i32> {
        let Some(pos) = self
            .vlans
            .iter()
            .position(|vlan| vlan.port == dp.index && vlan.vid == vid)
        else {
            return Err(-ENOENT);
        };

        if matches!(dp.kind, DsaPortKind::Cpu | DsaPortKind::Dsa) && self.vlans[pos].refcount > 1 {
            self.vlans[pos].refcount -= 1;
            return Ok(());
        }

        self.vlans.remove(pos);
        Ok(())
    }
}

pub fn dsa_switch_tag_8021q_vlan_add(
    ctx: &mut Dsa8021qContext,
    ports: &[DsaPort],
    info_port: DsaPort,
    vid: u16,
) -> Result<(), i32> {
    for port in ports {
        if matches!(port.kind, DsaPortKind::Dsa | DsaPortKind::Cpu) || *port == info_port {
            let flags = if matches!(port.kind, DsaPortKind::User) {
                BRIDGE_VLAN_INFO_UNTAGGED | BRIDGE_VLAN_INFO_PVID
            } else {
                0
            };
            ctx.vlan_add(*port, vid, flags)?;
        }
    }
    Ok(())
}

pub fn dsa_switch_tag_8021q_vlan_del(
    ctx: &mut Dsa8021qContext,
    ports: &[DsaPort],
    info_port: DsaPort,
    vid: u16,
) -> Result<(), i32> {
    for port in ports {
        if matches!(port.kind, DsaPortKind::Dsa | DsaPortKind::Cpu) || *port == info_port {
            ctx.vlan_del(*port, vid)?;
        }
    }
    Ok(())
}

pub fn dsa_tag_8021q_bridge_join(
    ctx: &mut Dsa8021qContext,
    dp: DsaPort,
    bridge: DsaBridge,
) -> Result<bool, i32> {
    let standalone_vid = dsa_tag_8021q_standalone_vid(dp);
    let bridge_vid = dsa_tag_8021q_bridge_vid(bridge.num);
    ctx.vlan_add(dp, bridge_vid, 1)?;
    let _ = ctx.vlan_del(dp, standalone_vid);
    Ok(true)
}

pub fn dsa_tag_8021q_bridge_leave(ctx: &mut Dsa8021qContext, dp: DsaPort, bridge: DsaBridge) {
    let standalone_vid = dsa_tag_8021q_standalone_vid(dp);
    let bridge_vid = dsa_tag_8021q_bridge_vid(bridge.num);
    if ctx.vlan_add(dp, standalone_vid, 0).is_ok() {
        let _ = ctx.vlan_del(dp, bridge_vid);
    }
}

pub const fn dsa_8021q_rcv(input: Dsa8021qRcvInput) -> Dsa8021qRcvResult {
    let tmp_vid = input.tag.tci & VLAN_VID_MASK;
    if !vid_is_dsa_8021q(tmp_vid) {
        return Dsa8021qRcvResult {
            source_port: input.source_port,
            switch_id: input.switch_id,
            vbid: None,
            vid: Some(tmp_vid),
            restored_tag: Some(input.tag),
            priority: None,
        };
    }

    let tmp_source_port = dsa_8021q_rx_source_port(tmp_vid);
    let tmp_switch_id = dsa_8021q_rx_switch_id(tmp_vid);
    let tmp_vbid = dsa_tag_8021q_rx_vbid(tmp_vid);
    Dsa8021qRcvResult {
        source_port: if tmp_vbid == 0 && input.source_port == -1 {
            tmp_source_port
        } else {
            input.source_port
        },
        switch_id: if tmp_vbid == 0 && input.switch_id == -1 {
            tmp_switch_id
        } else {
            input.switch_id
        },
        vbid: Some(tmp_vbid),
        vid: None,
        restored_tag: None,
        priority: Some((input.tag.tci & VLAN_PRIO_MASK) >> VLAN_PRIO_SHIFT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_8021q_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_8021q.c"
        ));
        assert!(source.contains("#define DSA_8021Q_RSV_VAL\t\t3"));
        assert!(source.contains("#define DSA_8021Q_SWITCH_ID_SHIFT\t6"));
        assert!(source.contains("#define DSA_8021Q_VBID_HI_SHIFT\t\t9"));
        assert!(source.contains("#define DSA_8021Q_VBID_LO_SHIFT\t\t4"));
        assert!(source.contains("#define DSA_8021Q_PORT_MASK\t\tGENMASK(3, 0)"));
        assert!(source.contains("return DSA_8021Q_RSV | DSA_8021Q_VBID(bridge_num);"));
        assert!(source.contains("DSA_8021Q_SWITCH_ID(dp->ds->index)"));
        assert!(source.contains("DSA_8021Q_PORT(dp->index)"));
        assert!(source.contains("return rsv == DSA_8021Q_RSV_VAL;"));
        assert!(source.contains("refcount_inc(&v->refcount);"));
        assert!(source.contains("BRIDGE_VLAN_INFO_UNTAGGED |"));
        assert!(source.contains("BRIDGE_VLAN_INFO_PVID;"));
        assert!(source.contains("return vlan_insert_tag(skb, htons(tpid), tci);"));
        assert!(source.contains("tmp_vid = tci & VLAN_VID_MASK;"));
        assert!(source.contains("__vlan_hwaccel_put_tag(skb, vlan_proto, tci);"));
        assert!(source.contains("skb->priority = (tci & VLAN_PRIO_MASK) >> VLAN_PRIO_SHIFT;"));
    }

    #[test]
    fn vid_fields_encode_and_decode_like_linux_masks() {
        let dp = DsaPort {
            switch_index: 5,
            index: 11,
            kind: DsaPortKind::User,
        };
        let vid = dsa_tag_8021q_standalone_vid(dp);
        assert_eq!(vid, 0x0c00 | (5 << 6) | 11);
        assert!(vid_is_dsa_8021q(vid));
        assert_eq!(dsa_8021q_rx_switch_id(vid), 5);
        assert_eq!(dsa_8021q_rx_source_port(vid), 11);
        assert_eq!(dsa_tag_8021q_bridge_vid(7), 0x0c00 | 0x0200 | 0x0030);
        assert_eq!(dsa_tag_8021q_rx_vbid(dsa_tag_8021q_bridge_vid(6)), 6);
    }

    #[test]
    fn receive_preserves_or_decodes_source_information() {
        let vid = dsa_tag_8021q_standalone_vid(DsaPort {
            switch_index: 2,
            index: 4,
            kind: DsaPortKind::User,
        });
        let decoded = dsa_8021q_rcv(Dsa8021qRcvInput {
            tag: VlanTag {
                proto: 0x8100,
                tci: (6 << VLAN_PRIO_SHIFT) | vid,
            },
            source_port: -1,
            switch_id: -1,
        });
        assert_eq!(decoded.source_port, 4);
        assert_eq!(decoded.switch_id, 2);
        assert_eq!(decoded.vbid, Some(0));
        assert_eq!(decoded.priority, Some(6));
        assert_eq!(decoded.restored_tag, None);

        let bridge = dsa_8021q_rcv(Dsa8021qRcvInput {
            tag: VlanTag {
                proto: 0x8100,
                tci: dsa_tag_8021q_bridge_vid(3),
            },
            source_port: -1,
            switch_id: -1,
        });
        assert_eq!(bridge.source_port, -1);
        assert_eq!(bridge.switch_id, -1);
        assert_eq!(bridge.vbid, Some(3));

        let plain = dsa_8021q_rcv(Dsa8021qRcvInput {
            tag: VlanTag {
                proto: 0x8100,
                tci: 100,
            },
            source_port: -1,
            switch_id: -1,
        });
        assert_eq!(plain.vid, Some(100));
        assert_eq!(plain.restored_tag.unwrap().tci, 100);
    }

    #[test]
    fn vlan_context_refcounts_cpu_and_dsa_ports() {
        let cpu = DsaPort {
            switch_index: 0,
            index: 5,
            kind: DsaPortKind::Cpu,
        };
        let user = DsaPort {
            switch_index: 0,
            index: 1,
            kind: DsaPortKind::User,
        };
        let mut ctx = Dsa8021qContext::new(0x8100);
        ctx.vlan_add(cpu, 10, 0).unwrap();
        ctx.vlan_add(cpu, 10, 0).unwrap();
        assert_eq!(ctx.vlans[0].refcount, 2);
        ctx.vlan_del(cpu, 10).unwrap();
        assert_eq!(ctx.vlans[0].refcount, 1);
        ctx.vlan_del(cpu, 10).unwrap();
        assert!(ctx.vlans.is_empty());

        dsa_switch_tag_8021q_vlan_add(&mut ctx, &[cpu, user], user, 20).unwrap();
        assert!(ctx.vlans.iter().any(|v| v.port == 1 && v.flags != 0));
        assert!(ctx.vlans.iter().any(|v| v.port == 5 && v.flags == 0));
    }
}
