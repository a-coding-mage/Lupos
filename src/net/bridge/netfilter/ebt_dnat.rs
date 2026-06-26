//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_dnat.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_dnat.c
//! Ebtables destination MAC NAT target.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_DESCRIPTION: &str = "Ebtables: Destination MAC address translation";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;
pub const NF_BR_PRE_ROUTING: u8 = 0;
pub const NF_BR_LOCAL_OUT: u8 = 3;
pub const NF_BR_BROUTING: u8 = 5;
pub const NF_BR_NUMHOOKS: u8 = 6;
pub const EBT_DROP: i32 = -2;
pub const EBT_RETURN: i32 = -4;
pub const NUM_STANDARD_TARGETS: i32 = 4;
pub const PACKET_HOST: u8 = 0;
pub const PACKET_BROADCAST: u8 = 1;
pub const PACKET_MULTICAST: u8 = 2;
pub const PACKET_OTHERHOST: u8 = 3;
pub const EBT_DNAT_HOOKS: u32 = (1 << NF_BR_NUMHOOKS)
    | (1 << NF_BR_PRE_ROUTING)
    | (1 << NF_BR_LOCAL_OUT)
    | (1 << NF_BR_BROUTING);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtNatInfo {
    pub mac: [u8; 6],
    pub target: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnatPacket {
    pub writable: bool,
    pub h_dest: [u8; 6],
    pub pkt_type: u8,
    pub hooknum: u8,
    pub in_dev_addr: [u8; 6],
    pub bridge_dev_addr: [u8; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub hooks: u32,
    pub targetsize: usize,
}

pub const EBT_DNAT_TG_REG: XtTarget = XtTarget {
    name: "dnat",
    revision: 0,
    family: NFPROTO_BRIDGE,
    hooks: EBT_DNAT_HOOKS,
    targetsize: core::mem::size_of::<EbtNatInfo>(),
};

pub fn ebt_dnat_tg(packet: &mut DnatPacket, info: EbtNatInfo) -> i32 {
    if !packet.writable {
        return EBT_DROP;
    }
    packet.h_dest = info.mac;

    if is_multicast_ether_addr(info.mac) {
        packet.pkt_type = if is_broadcast_ether_addr(info.mac) {
            PACKET_BROADCAST
        } else {
            PACKET_MULTICAST
        };
    } else {
        let dev_addr = match packet.hooknum {
            NF_BR_BROUTING => Some(packet.in_dev_addr),
            NF_BR_PRE_ROUTING => Some(packet.bridge_dev_addr),
            _ => None,
        };
        if let Some(dev_addr) = dev_addr {
            packet.pkt_type = if info.mac == dev_addr {
                PACKET_HOST
            } else {
                PACKET_OTHERHOST
            };
        }
    }
    info.target
}

pub const fn is_broadcast_ether_addr(mac: [u8; 6]) -> bool {
    let mut i = 0;
    while i < 6 {
        if mac[i] != 0xff {
            return false;
        }
        i += 1;
    }
    true
}

pub const fn is_multicast_ether_addr(mac: [u8; 6]) -> bool {
    mac[0] & 1 != 0
}

pub const fn ebt_invalid_target(target: i32) -> bool {
    target < -NUM_STANDARD_TARGETS || target >= 0
}

pub fn ebt_dnat_tg_check(
    base_chain: bool,
    table: &str,
    hook_mask: u32,
    info: EbtNatInfo,
) -> Result<(), i32> {
    if base_chain && info.target == EBT_RETURN {
        return Err(-EINVAL);
    }
    let hook_mask = hook_mask & !(1 << NF_BR_NUMHOOKS);
    if ((table != "nat" || hook_mask & !((1 << NF_BR_PRE_ROUTING) | (1 << NF_BR_LOCAL_OUT)) != 0)
        && (table != "broute" || hook_mask & !(1 << NF_BR_BROUTING) != 0))
        || ebt_invalid_target(info.target)
    {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn ebt_dnat_init() -> &'static XtTarget {
    &EBT_DNAT_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt() -> DnatPacket {
        DnatPacket {
            writable: true,
            h_dest: [0; 6],
            pkt_type: PACKET_OTHERHOST,
            hooknum: NF_BR_PRE_ROUTING,
            in_dev_addr: [0x02, 0, 0, 0, 0, 1],
            bridge_dev_addr: [0x02, 0, 0, 0, 0, 2],
        }
    }

    #[test]
    fn ebt_dnat_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_dnat.c"
        ));
        assert!(source.contains("ebt_dnat_tg(struct sk_buff *skb"));
        assert!(source.contains("if (skb_ensure_writable(skb, 0))"));
        assert!(source.contains("ether_addr_copy(eth_hdr(skb)->h_dest, info->mac);"));
        assert!(source.contains("if (is_multicast_ether_addr(info->mac))"));
        assert!(source.contains("skb->pkt_type = PACKET_BROADCAST;"));
        assert!(source.contains("skb->pkt_type = PACKET_MULTICAST;"));
        assert!(source.contains("case NF_BR_BROUTING:"));
        assert!(source.contains("case NF_BR_PRE_ROUTING:"));
        assert!(source.contains("skb->pkt_type = PACKET_HOST;"));
        assert!(source.contains("skb->pkt_type = PACKET_OTHERHOST;"));
        assert!(source.contains("if (BASE_CHAIN && info->target == EBT_RETURN)"));
        assert!(source.contains("strcmp(par->table, \"nat\")"));
        assert!(source.contains("strcmp(par->table, \"broute\")"));
        assert!(source.contains(".name\t\t= \"dnat\""));
        assert!(
            source
                .contains("MODULE_DESCRIPTION(\"Ebtables: Destination MAC address translation\")")
        );
    }

    #[test]
    fn dnat_rewrites_destination_and_sets_packet_type_by_target_mac() {
        let info = EbtNatInfo {
            mac: [0x02, 0, 0, 0, 0, 2],
            target: -1,
        };
        let mut packet = pkt();
        assert_eq!(ebt_dnat_tg(&mut packet, info), -1);
        assert_eq!(packet.h_dest, info.mac);
        assert_eq!(packet.pkt_type, PACKET_HOST);

        let mut packet = pkt();
        assert_eq!(
            ebt_dnat_tg(
                &mut packet,
                EbtNatInfo {
                    mac: [0xff; 6],
                    ..info
                }
            ),
            -1
        );
        assert_eq!(packet.pkt_type, PACKET_BROADCAST);
        packet.writable = false;
        assert_eq!(ebt_dnat_tg(&mut packet, info), EBT_DROP);
        assert_eq!(
            ebt_dnat_tg_check(false, "nat", (1 << NF_BR_PRE_ROUTING), info),
            Ok(())
        );
        assert_eq!(
            ebt_dnat_tg_check(false, "broute", (1 << NF_BR_BROUTING), info),
            Ok(())
        );
        assert_eq!(
            ebt_dnat_tg_check(
                true,
                "nat",
                (1 << NF_BR_PRE_ROUTING),
                EbtNatInfo {
                    target: EBT_RETURN,
                    ..info
                }
            ),
            Err(-EINVAL)
        );
        assert_eq!(ebt_dnat_init(), &EBT_DNAT_TG_REG);
    }
}
