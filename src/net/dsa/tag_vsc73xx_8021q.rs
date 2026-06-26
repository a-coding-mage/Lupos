//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_vsc73xx_8021q.c
//! test-origin: linux:vendor/linux/net/dsa/tag_vsc73xx_8021q.c
//! VSC73XX DSA 802.1Q tag handling.

pub const VSC73XX_8021Q_NAME: &str = "vsc73xx-8021q";
pub const DSA_TAG_PROTO_VSC73XX_8021Q: u8 = 28;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const VLAN_HLEN: usize = 4;
pub const VLAN_PRIO_SHIFT: u16 = 13;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for VSC73XX family of switches, using VLAN";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
    pub promisc_on_conduit: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vsc73xxTxContext {
    pub offload_fwd_mark: bool,
    pub bridge_vlan_enabled: bool,
    pub standalone_vid: u16,
    pub bridge_vid: u16,
    pub queue_mapping: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vsc73xxTxAction {
    Bypass,
    Tag { ethertype: u16, tci: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vsc73xxRxDecode {
    pub src_port: Option<u8>,
    pub switch_id: Option<u8>,
    pub vbid: Option<u16>,
    pub vid: Option<u16>,
    pub user_found: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vsc73xxRxFrame {
    pub src_port: u8,
    pub switch_id: u8,
    pub vbid: u16,
    pub vid: u16,
    pub offload_fwd_mark: bool,
}

pub const VSC73XX_8021Q_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: VSC73XX_8021Q_NAME,
    proto: DSA_TAG_PROTO_VSC73XX_8021Q,
    needed_headroom: VLAN_HLEN,
    promisc_on_conduit: true,
};

pub const fn vsc73xx_xmit_action(ctx: Vsc73xxTxContext) -> Vsc73xxTxAction {
    if ctx.offload_fwd_mark && ctx.bridge_vlan_enabled {
        return Vsc73xxTxAction::Bypass;
    }

    let tx_vid = if ctx.offload_fwd_mark {
        ctx.bridge_vid
    } else {
        ctx.standalone_vid
    };
    let pcp = netdev_txq_to_tc(ctx.queue_mapping);
    Vsc73xxTxAction::Tag {
        ethertype: ETH_P_8021Q,
        tci: ((pcp as u16) << VLAN_PRIO_SHIFT) | tx_vid,
    }
}

pub const fn vsc73xx_rcv(decoded: Vsc73xxRxDecode) -> Option<Vsc73xxRxFrame> {
    if !decoded.user_found {
        return None;
    }

    Some(Vsc73xxRxFrame {
        src_port: match decoded.src_port {
            Some(port) => port,
            None => return None,
        },
        switch_id: match decoded.switch_id {
            Some(id) => id,
            None => return None,
        },
        vbid: match decoded.vbid {
            Some(vbid) => vbid,
            None => return None,
        },
        vid: match decoded.vid {
            Some(vid) => vid,
            None => return None,
        },
        offload_fwd_mark: true,
    })
}

pub const fn netdev_txq_to_tc(queue_mapping: u16) -> u8 {
    (queue_mapping & 0x7) as u8
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:vsc73xx-8021q", "dsa_tag:id-28"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsc73xx_8021q_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_vsc73xx_8021q.c"
        ));
        assert!(source.contains("#define VSC73XX_8021Q_NAME \"vsc73xx-8021q\""));
        assert!(source.contains("static struct sk_buff *"));
        assert!(source.contains("vsc73xx_xmit(struct sk_buff *skb"));
        assert!(source.contains("dsa_tag_8021q_standalone_vid(dp)"));
        assert!(source.contains("if (skb->offload_fwd_mark)"));
        assert!(source.contains("if (br_vlan_enabled(br))"));
        assert!(source.contains("return skb;"));
        assert!(source.contains("tx_vid = dsa_tag_8021q_bridge_vid(bridge_num);"));
        assert!(source.contains("pcp = netdev_txq_to_tc(netdev, queue_mapping);"));
        assert!(source.contains("dsa_8021q_xmit(skb, netdev, ETH_P_8021Q"));
        assert!(source.contains("dsa_8021q_rcv(skb, &src_port, &switch_id, &vbid, &vid);"));
        assert!(source.contains("dsa_tag_8021q_find_user(netdev, src_port, switch_id"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains(".proto\t\t\t= DSA_TAG_PROTO_VSC73XX_8021Q"));
        assert!(source.contains(".needed_headroom\t= VLAN_HLEN"));
        assert!(source.contains(".promisc_on_conduit\t= true"));

        assert_eq!(VSC73XX_8021Q_NETDEV_OPS.name, "vsc73xx-8021q");
        assert_eq!(VSC73XX_8021Q_NETDEV_OPS.needed_headroom, VLAN_HLEN);
        assert!(VSC73XX_8021Q_NETDEV_OPS.promisc_on_conduit);
        assert_eq!(module_aliases(), ["dsa_tag:vsc73xx-8021q", "dsa_tag:id-28"]);
    }

    #[test]
    fn vsc73xx_xmit_selects_standalone_or_bridge_vid() {
        assert_eq!(
            vsc73xx_xmit_action(Vsc73xxTxContext {
                offload_fwd_mark: false,
                bridge_vlan_enabled: false,
                standalone_vid: 12,
                bridge_vid: 40,
                queue_mapping: 3,
            }),
            Vsc73xxTxAction::Tag {
                ethertype: ETH_P_8021Q,
                tci: (3 << VLAN_PRIO_SHIFT) | 12
            }
        );
        assert_eq!(
            vsc73xx_xmit_action(Vsc73xxTxContext {
                offload_fwd_mark: true,
                bridge_vlan_enabled: false,
                standalone_vid: 12,
                bridge_vid: 40,
                queue_mapping: 1,
            }),
            Vsc73xxTxAction::Tag {
                ethertype: ETH_P_8021Q,
                tci: (1 << VLAN_PRIO_SHIFT) | 40
            }
        );
        assert_eq!(
            vsc73xx_xmit_action(Vsc73xxTxContext {
                offload_fwd_mark: true,
                bridge_vlan_enabled: true,
                standalone_vid: 12,
                bridge_vid: 40,
                queue_mapping: 1,
            }),
            Vsc73xxTxAction::Bypass
        );
    }

    #[test]
    fn vsc73xx_receive_requires_a_resolved_user() {
        assert_eq!(
            vsc73xx_rcv(Vsc73xxRxDecode {
                src_port: Some(2),
                switch_id: Some(0),
                vbid: Some(5),
                vid: Some(100),
                user_found: true,
            }),
            Some(Vsc73xxRxFrame {
                src_port: 2,
                switch_id: 0,
                vbid: 5,
                vid: 100,
                offload_fwd_mark: true,
            })
        );
        assert_eq!(
            vsc73xx_rcv(Vsc73xxRxDecode {
                src_port: Some(2),
                switch_id: Some(0),
                vbid: Some(5),
                vid: Some(100),
                user_found: false,
            }),
            None
        );
    }
}
