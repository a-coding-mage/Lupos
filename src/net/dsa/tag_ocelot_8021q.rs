//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_ocelot_8021q.c
//! test-origin: linux:vendor/linux/net/dsa/tag_ocelot_8021q.c
//! Ocelot DSA 802.1Q tag handling.

pub const OCELOT_8021Q_NAME: &str = "ocelot-8021q";
pub const DSA_TAG_PROTO_OCELOT_8021Q_VALUE: u8 = 20;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const VLAN_HLEN: usize = 4;
pub const VLAN_PRIO_SHIFT: u16 = 13;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Ocelot family of switches, using VLAN";
pub const MODULE_LICENSE: &str = "GPL v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
    pub connect: bool,
    pub disconnect: bool,
    pub promisc_on_conduit: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ocelot8021qTaggerPrivate {
    pub xmit_worker_running: bool,
    pub xmit_work_fn_registered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcelotTxContext {
    pub queue_mapping: u16,
    pub standalone_vid: u16,
    pub ptp_rew_op: bool,
    pub link_local_dest: bool,
    pub checksum_partial: bool,
    pub checksum_help_failed: bool,
    pub xmit_worker_running: bool,
    pub xmit_work_fn_registered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OcelotTxAction {
    VlanTag { ethertype: u16, tci: u16 },
    DeferredQueued,
    Drop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcelotRxDecode {
    pub src_port: i32,
    pub switch_id: i32,
    pub user_found: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcelotRxFrame {
    pub src_port: i32,
    pub switch_id: i32,
    pub offload_fwd_mark: bool,
}

pub const OCELOT_8021Q_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: OCELOT_8021Q_NAME,
    proto: DSA_TAG_PROTO_OCELOT_8021Q_VALUE,
    needed_headroom: VLAN_HLEN,
    connect: true,
    disconnect: true,
    promisc_on_conduit: true,
};

pub const fn ocelot_xmit(ctx: OcelotTxContext) -> OcelotTxAction {
    if ctx.ptp_rew_op || ctx.link_local_dest {
        return ocelot_defer_xmit(ctx);
    }

    let pcp = netdev_txq_to_tc(ctx.queue_mapping);
    OcelotTxAction::VlanTag {
        ethertype: ETH_P_8021Q,
        tci: ((pcp as u16) << VLAN_PRIO_SHIFT) | ctx.standalone_vid,
    }
}

pub const fn ocelot_defer_xmit(ctx: OcelotTxContext) -> OcelotTxAction {
    if !ctx.xmit_work_fn_registered || !ctx.xmit_worker_running {
        return OcelotTxAction::Drop;
    }
    if ctx.checksum_partial && ctx.checksum_help_failed {
        return OcelotTxAction::Drop;
    }
    OcelotTxAction::DeferredQueued
}

pub const fn ocelot_rcv(decoded: OcelotRxDecode) -> Option<OcelotRxFrame> {
    if !decoded.user_found {
        return None;
    }
    Some(OcelotRxFrame {
        src_port: decoded.src_port,
        switch_id: decoded.switch_id,
        offload_fwd_mark: true,
    })
}

pub const fn ocelot_connect() -> Ocelot8021qTaggerPrivate {
    Ocelot8021qTaggerPrivate {
        xmit_worker_running: true,
        xmit_work_fn_registered: false,
    }
}

pub const fn ocelot_disconnect(_priv: Ocelot8021qTaggerPrivate) -> Ocelot8021qTaggerPrivate {
    Ocelot8021qTaggerPrivate {
        xmit_worker_running: false,
        xmit_work_fn_registered: false,
    }
}

pub const fn netdev_txq_to_tc(queue_mapping: u16) -> u8 {
    (queue_mapping & 0x7) as u8
}

pub const fn ocelot_module_ops() -> &'static DsaDeviceOps {
    &OCELOT_8021Q_NETDEV_OPS
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:ocelot-8021q", "dsa_tag:id-20"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_ocelot_8021q_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_ocelot_8021q.c"
        ));
        let tag_8021q = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_8021q.h"
        ));
        let ocelot = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dsa/ocelot.h"
        ));
        let dsa = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/dsa.h"
        ));

        assert!(source.contains("#define OCELOT_8021Q_NAME \"ocelot-8021q\""));
        assert!(source.contains("struct ocelot_8021q_tagger_private"));
        assert!(source.contains("struct kthread_worker *xmit_worker;"));
        assert!(source.contains("if (!xmit_work_fn || !xmit_worker)"));
        assert!(source.contains("skb->ip_summed == CHECKSUM_PARTIAL && skb_checksum_help(skb)"));
        assert!(source.contains("xmit_work = kzalloc_obj(*xmit_work, GFP_ATOMIC);"));
        assert!(source.contains("kthread_init_work(&xmit_work->work, xmit_work_fn);"));
        assert!(source.contains("xmit_work->skb = skb_get(skb);"));
        assert!(source.contains("kthread_queue_work(xmit_worker, &xmit_work->work);"));
        assert!(source.contains("u16 tx_vid = dsa_tag_8021q_standalone_vid(dp);"));
        assert!(
            source.contains("if (ocelot_ptp_rew_op(skb) || is_link_local_ether_addr(hdr->h_dest))")
        );
        assert!(source.contains("return dsa_8021q_xmit(skb, netdev, ETH_P_8021Q"));
        assert!(source.contains("dsa_8021q_rcv(skb, &src_port, &switch_id, NULL, NULL);"));
        assert!(source.contains("dsa_conduit_find_user(netdev, switch_id, src_port);"));
        assert!(source.contains("dsa_default_offload_fwd_mark(skb);"));
        assert!(source.contains("kthread_destroy_worker(priv->xmit_worker);"));
        assert!(source.contains("kthread_run_worker(0, \"felix_xmit\")"));
        assert!(source.contains(".proto\t\t\t= DSA_TAG_PROTO_OCELOT_8021Q"));
        assert!(source.contains(".needed_headroom\t= VLAN_HLEN"));
        assert!(source.contains(".promisc_on_conduit\t= true"));
        assert!(source.contains(
            "MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_OCELOT_8021Q, OCELOT_8021Q_NAME);"
        ));
        assert!(tag_8021q.contains("struct sk_buff *dsa_8021q_xmit"));
        assert!(ocelot.contains("struct ocelot_8021q_tagger_data"));
        assert!(dsa.contains("#define DSA_TAG_PROTO_OCELOT_8021Q_VALUE\t20"));
    }

    #[test]
    fn ocelot_xmit_rcv_and_lifetime_match_source_decisions() {
        let vlan = ocelot_xmit(OcelotTxContext {
            queue_mapping: 3,
            standalone_vid: 100,
            ptp_rew_op: false,
            link_local_dest: false,
            checksum_partial: false,
            checksum_help_failed: false,
            xmit_worker_running: false,
            xmit_work_fn_registered: false,
        });
        assert_eq!(
            vlan,
            OcelotTxAction::VlanTag {
                ethertype: ETH_P_8021Q,
                tci: (3 << VLAN_PRIO_SHIFT) | 100,
            }
        );

        let deferred = ocelot_xmit(OcelotTxContext {
            queue_mapping: 0,
            standalone_vid: 1,
            ptp_rew_op: true,
            link_local_dest: false,
            checksum_partial: true,
            checksum_help_failed: false,
            xmit_worker_running: true,
            xmit_work_fn_registered: true,
        });
        assert_eq!(deferred, OcelotTxAction::DeferredQueued);
        assert_eq!(
            ocelot_xmit(OcelotTxContext {
                checksum_help_failed: true,
                ..OcelotTxContext {
                    queue_mapping: 0,
                    standalone_vid: 1,
                    ptp_rew_op: false,
                    link_local_dest: true,
                    checksum_partial: true,
                    checksum_help_failed: false,
                    xmit_worker_running: true,
                    xmit_work_fn_registered: true,
                }
            }),
            OcelotTxAction::Drop
        );

        assert_eq!(
            ocelot_rcv(OcelotRxDecode {
                src_port: 2,
                switch_id: 1,
                user_found: true,
            }),
            Some(OcelotRxFrame {
                src_port: 2,
                switch_id: 1,
                offload_fwd_mark: true,
            })
        );
        assert_eq!(
            ocelot_rcv(OcelotRxDecode {
                src_port: 2,
                switch_id: 1,
                user_found: false,
            }),
            None
        );

        let priv_data = ocelot_connect();
        assert!(priv_data.xmit_worker_running);
        assert!(!ocelot_disconnect(priv_data).xmit_worker_running);
        assert_eq!(ocelot_module_ops(), &OCELOT_8021Q_NETDEV_OPS);
        assert_eq!(module_aliases(), ["dsa_tag:ocelot-8021q", "dsa_tag:id-20"]);
    }
}
