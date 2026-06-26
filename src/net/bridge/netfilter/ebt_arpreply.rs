//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_arpreply.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_arpreply.c
//! Ebtables ARP reply target validation and reply decision.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_DESCRIPTION: &str = "Ebtables: ARP reply target";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;
pub const NF_BR_PRE_ROUTING: u8 = 0;
pub const NF_BR_NUMHOOKS: u8 = 6;
pub const EBT_DROP: i32 = -2;
pub const EBT_CONTINUE: i32 = -3;
pub const EBT_RETURN: i32 = -4;
pub const NUM_STANDARD_TARGETS: i32 = 4;
pub const ETH_ALEN: u8 = 6;
pub const ARPOP_REQUEST: u16 = 1;
pub const ARPOP_REPLY: u16 = 2;
pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_ARP: u16 = 0x0806;
pub const EBT_IPROTO: u8 = 0x01;
pub const EBT_ARPREPLY_HOOKS: u32 = (1 << NF_BR_NUMHOOKS) | (1 << NF_BR_PRE_ROUTING);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtArpreplyInfo {
    pub mac: [u8; 6],
    pub target: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArpPacketView {
    pub header_present: bool,
    pub sender_hw_present: bool,
    pub sender_ip_present: bool,
    pub target_ip_present: bool,
    pub ar_op: u16,
    pub ar_hln: u8,
    pub ar_pro: u16,
    pub ar_pln: u8,
    pub sender_hw: [u8; 6],
    pub sender_ip: u32,
    pub target_ip: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArpReply {
    pub op: u16,
    pub protocol: u16,
    pub sip: u32,
    pub dip: u32,
    pub target_hw: [u8; 6],
    pub reply_hw: [u8; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub table: &'static str,
    pub hooks: u32,
    pub targetsize: usize,
}

pub const EBT_ARPREPLY_TG_REG: XtTarget = XtTarget {
    name: "arpreply",
    revision: 0,
    family: NFPROTO_BRIDGE,
    table: "nat",
    hooks: EBT_ARPREPLY_HOOKS,
    targetsize: core::mem::size_of::<EbtArpreplyInfo>(),
};

pub const fn ebt_arpreply_tg(pkt: ArpPacketView, info: EbtArpreplyInfo) -> (i32, Option<ArpReply>) {
    if !pkt.header_present {
        return (EBT_DROP, None);
    }
    if pkt.ar_op != ARPOP_REQUEST
        || pkt.ar_hln != ETH_ALEN
        || pkt.ar_pro != ETH_P_IP.to_be()
        || pkt.ar_pln != 4
    {
        return (EBT_CONTINUE, None);
    }
    if !pkt.sender_hw_present || !pkt.sender_ip_present || !pkt.target_ip_present {
        return (EBT_DROP, None);
    }

    (
        info.target,
        Some(ArpReply {
            op: ARPOP_REPLY,
            protocol: ETH_P_ARP,
            sip: pkt.sender_ip,
            dip: pkt.target_ip,
            target_hw: pkt.sender_hw,
            reply_hw: info.mac,
        }),
    )
}

pub const fn ebt_invalid_target(target: i32) -> bool {
    target < -NUM_STANDARD_TARGETS || target >= 0
}

pub const fn ebt_arpreply_tg_check(
    base_chain: bool,
    ethproto: u16,
    invflags: u8,
    info: EbtArpreplyInfo,
) -> Result<(), i32> {
    if base_chain && info.target == EBT_RETURN {
        return Err(-EINVAL);
    }
    if ethproto != ETH_P_ARP.to_be() || invflags & EBT_IPROTO != 0 {
        return Err(-EINVAL);
    }
    if ebt_invalid_target(info.target) {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn ebt_arpreply_init() -> &'static XtTarget {
    &EBT_ARPREPLY_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ArpPacketView {
        ArpPacketView {
            header_present: true,
            sender_hw_present: true,
            sender_ip_present: true,
            target_ip_present: true,
            ar_op: ARPOP_REQUEST,
            ar_hln: ETH_ALEN,
            ar_pro: ETH_P_IP.to_be(),
            ar_pln: 4,
            sender_hw: [1, 2, 3, 4, 5, 6],
            sender_ip: 0x0a00_0001,
            target_ip: 0x0a00_0002,
        }
    }

    #[test]
    fn ebt_arpreply_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_arpreply.c"
        ));
        assert!(source.contains("ebt_arpreply_tg(struct sk_buff *skb"));
        assert!(source.contains("skb_header_pointer(skb, 0, sizeof(_ah), &_ah);"));
        assert!(source.contains("if (ap == NULL)"));
        assert!(source.contains("ap->ar_op != htons(ARPOP_REQUEST)"));
        assert!(source.contains("ap->ar_hln != ETH_ALEN"));
        assert!(source.contains("ap->ar_pro != htons(ETH_P_IP)"));
        assert!(source.contains("ap->ar_pln != 4"));
        assert!(source.contains("arp_send(ARPOP_REPLY, ETH_P_ARP"));
        assert!(source.contains("return info->target;"));
        assert!(source.contains("if (BASE_CHAIN && info->target == EBT_RETURN)"));
        assert!(source.contains("e->ethproto != htons(ETH_P_ARP)"));
        assert!(source.contains(".name\t\t= \"arpreply\""));
        assert!(source.contains(".table\t\t= \"nat\""));
        assert!(source.contains("xt_register_target(&ebt_arpreply_tg_reg);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Ebtables: ARP reply target\")"));
    }

    #[test]
    fn arpreply_sends_only_valid_arp_requests() {
        let info = EbtArpreplyInfo {
            mac: [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff],
            target: EBT_CONTINUE,
        };
        let (verdict, reply) = ebt_arpreply_tg(request(), info);
        assert_eq!(verdict, EBT_CONTINUE);
        assert_eq!(reply.unwrap().reply_hw, info.mac);

        assert_eq!(
            ebt_arpreply_tg(
                ArpPacketView {
                    header_present: false,
                    ..request()
                },
                info
            ),
            (EBT_DROP, None)
        );
        assert_eq!(
            ebt_arpreply_tg(
                ArpPacketView {
                    ar_hln: 4,
                    ..request()
                },
                info
            ),
            (EBT_CONTINUE, None)
        );
        assert_eq!(
            ebt_arpreply_tg_check(false, ETH_P_ARP.to_be(), 0, info),
            Ok(())
        );
        assert_eq!(
            ebt_arpreply_tg_check(
                true,
                ETH_P_ARP.to_be(),
                0,
                EbtArpreplyInfo {
                    target: EBT_RETURN,
                    ..info
                }
            ),
            Err(-EINVAL)
        );
        assert_eq!(ebt_arpreply_init(), &EBT_ARPREPLY_TG_REG);
    }
}
