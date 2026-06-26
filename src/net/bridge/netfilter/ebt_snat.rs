//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_snat.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_snat.c
//! Ebtables source MAC NAT target.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_DESCRIPTION: &str = "Ebtables: Source MAC address translation";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;
pub const NF_BR_POST_ROUTING: u8 = 4;
pub const NF_BR_NUMHOOKS: u8 = 6;
pub const EBT_ACCEPT: i32 = -1;
pub const EBT_DROP: i32 = -2;
pub const EBT_CONTINUE: i32 = -3;
pub const EBT_RETURN: i32 = -4;
pub const NUM_STANDARD_TARGETS: i32 = 4;
pub const EBT_VERDICT_BITS: i32 = 0x0000_000f;
pub const NAT_ARP_BIT: i32 = 0x0000_0010;
pub const ETH_ALEN: u8 = 6;
pub const ETH_P_ARP: u16 = 0x0806;
pub const EBT_SNAT_HOOKS: u32 = (1 << NF_BR_NUMHOOKS) | (1 << NF_BR_POST_ROUTING);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtNatInfo {
    pub mac: [u8; 6],
    pub target: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnatPacket {
    pub writable: bool,
    pub h_source: [u8; 6],
    pub h_proto: u16,
    pub arp_hln: Option<u8>,
    pub arp_sender_hw: [u8; 6],
    pub store_bits_ok: bool,
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

pub const EBT_SNAT_TG_REG: XtTarget = XtTarget {
    name: "snat",
    revision: 0,
    family: NFPROTO_BRIDGE,
    table: "nat",
    hooks: EBT_SNAT_HOOKS,
    targetsize: core::mem::size_of::<EbtNatInfo>(),
};

pub fn ebt_snat_tg(packet: &mut SnatPacket, info: EbtNatInfo) -> i32 {
    if !packet.writable {
        return EBT_DROP;
    }

    packet.h_source = info.mac;
    if info.target & NAT_ARP_BIT == 0 && packet.h_proto == ETH_P_ARP.to_be() {
        let Some(arp_hln) = packet.arp_hln else {
            return EBT_DROP;
        };
        if arp_hln == ETH_ALEN {
            if !packet.store_bits_ok {
                return EBT_DROP;
            }
            packet.arp_sender_hw = info.mac;
        }
    }

    info.target | !EBT_VERDICT_BITS
}

pub const fn ebt_invalid_target(target: i32) -> bool {
    target < -NUM_STANDARD_TARGETS || target >= 0
}

pub fn ebt_snat_tg_check(hook_mask: u32, info: EbtNatInfo) -> Result<(), i32> {
    let mut tmp = info.target | !EBT_VERDICT_BITS;
    if hook_mask & (1 << NF_BR_NUMHOOKS) != 0 && tmp == EBT_RETURN {
        return Err(-EINVAL);
    }
    if ebt_invalid_target(tmp) {
        return Err(-EINVAL);
    }
    tmp = info.target | EBT_VERDICT_BITS;
    if (tmp & !NAT_ARP_BIT) != !NAT_ARP_BIT {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn ebt_snat_init() -> &'static XtTarget {
    &EBT_SNAT_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebt_snat_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_snat.c"
        ));
        assert!(source.contains("ebt_snat_tg(struct sk_buff *skb"));
        assert!(source.contains("if (skb_ensure_writable(skb, 0))"));
        assert!(source.contains("ether_addr_copy(eth_hdr(skb)->h_source, info->mac);"));
        assert!(source.contains("if (!(info->target & NAT_ARP_BIT)"));
        assert!(source.contains("eth_hdr(skb)->h_proto == htons(ETH_P_ARP)"));
        assert!(source.contains("if (ap == NULL)"));
        assert!(source.contains("if (ap->ar_hln != ETH_ALEN)"));
        assert!(source.contains("if (skb_store_bits(skb, sizeof(_ah), info->mac, ETH_ALEN))"));
        assert!(source.contains("return info->target | ~EBT_VERDICT_BITS;"));
        assert!(source.contains("if (BASE_CHAIN && tmp == EBT_RETURN)"));
        assert!(source.contains("if ((tmp & ~NAT_ARP_BIT) != ~NAT_ARP_BIT)"));
        assert!(source.contains(".name\t\t= \"snat\""));
        assert!(source.contains(".table\t\t= \"nat\""));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Ebtables: Source MAC address translation\")")
        );

        let info = EbtNatInfo {
            mac: [0x02, 0, 0, 0, 0, 9],
            target: EBT_ACCEPT & !NAT_ARP_BIT,
        };
        let mut packet = SnatPacket {
            writable: true,
            h_source: [0; 6],
            h_proto: ETH_P_ARP.to_be(),
            arp_hln: Some(ETH_ALEN),
            arp_sender_hw: [0; 6],
            store_bits_ok: true,
        };
        assert_eq!(
            ebt_snat_tg(&mut packet, info),
            EBT_ACCEPT | !EBT_VERDICT_BITS
        );
        assert_eq!(packet.h_source, info.mac);
        assert_eq!(packet.arp_sender_hw, info.mac);

        packet.store_bits_ok = false;
        assert_eq!(ebt_snat_tg(&mut packet, info), EBT_DROP);
        packet.writable = false;
        assert_eq!(ebt_snat_tg(&mut packet, info), EBT_DROP);
        assert_eq!(
            ebt_snat_tg_check((1 << NF_BR_NUMHOOKS) | (1 << NF_BR_POST_ROUTING), info),
            Ok(())
        );
        assert_eq!(
            ebt_snat_tg_check(
                (1 << NF_BR_NUMHOOKS) | (1 << NF_BR_POST_ROUTING),
                EbtNatInfo {
                    target: EBT_RETURN,
                    ..info
                }
            ),
            Err(-EINVAL)
        );
        assert_eq!(ebt_snat_init(), &EBT_SNAT_TG_REG);
    }
}
