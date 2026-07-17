//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_redirect.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_redirect.c
//! Ebtables packet redirection target.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_DESCRIPTION: &str = "Ebtables: Packet redirection to localhost";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;
pub const NF_BR_PRE_ROUTING: u8 = 0;
pub const NF_BR_BROUTING: u8 = 5;
pub const NF_BR_NUMHOOKS: u8 = 6;
pub const EBT_ACCEPT: i32 = -1;
pub const EBT_DROP: i32 = -2;
pub const EBT_CONTINUE: i32 = -3;
pub const EBT_RETURN: i32 = -4;
pub const NUM_STANDARD_TARGETS: i32 = 4;
pub const PACKET_HOST: u8 = 0;
pub const EBT_REDIRECT_HOOKS: u32 =
    (1 << NF_BR_NUMHOOKS) | (1 << NF_BR_PRE_ROUTING) | (1 << NF_BR_BROUTING);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtRedirectInfo {
    pub target: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RedirectPacket {
    pub writable: bool,
    pub hooknum: u8,
    pub h_dest: [u8; 6],
    pub master_dev_addr: Option<[u8; 6]>,
    pub in_dev_addr: [u8; 6],
    pub pkt_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub hooks: u32,
    pub targetsize: usize,
}

pub const EBT_REDIRECT_TG_REG: XtTarget = XtTarget {
    name: "redirect",
    revision: 0,
    family: NFPROTO_BRIDGE,
    hooks: EBT_REDIRECT_HOOKS,
    targetsize: core::mem::size_of::<EbtRedirectInfo>(),
};

pub fn ebt_redirect_tg(packet: &mut RedirectPacket, info: EbtRedirectInfo) -> i32 {
    if !packet.writable {
        return EBT_DROP;
    }
    packet.h_dest = if packet.hooknum != NF_BR_BROUTING {
        let Some(master_dev_addr) = packet.master_dev_addr else {
            return EBT_DROP;
        };
        master_dev_addr
    } else {
        packet.in_dev_addr
    };
    packet.pkt_type = PACKET_HOST;
    info.target
}

pub const fn ebt_invalid_target(target: i32) -> bool {
    target < -NUM_STANDARD_TARGETS || target >= 0
}

pub fn ebt_redirect_tg_check(
    table: &str,
    hook_mask: u32,
    info: EbtRedirectInfo,
) -> Result<(), i32> {
    let base_chain = hook_mask & (1 << NF_BR_NUMHOOKS) != 0;
    if base_chain && info.target == EBT_RETURN {
        return Err(-EINVAL);
    }

    let hook_mask = hook_mask & !(1 << NF_BR_NUMHOOKS);
    if ((table != "nat" || hook_mask & !(1 << NF_BR_PRE_ROUTING) != 0)
        && (table != "broute" || hook_mask & !(1 << NF_BR_BROUTING) != 0))
        || ebt_invalid_target(info.target)
    {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn ebt_redirect_init() -> &'static XtTarget {
    &EBT_REDIRECT_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebt_redirect_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_redirect.c"
        ));
        assert!(source.contains("ebt_redirect_tg(struct sk_buff *skb"));
        assert!(source.contains("if (skb_ensure_writable(skb, 0))"));
        assert!(source.contains("return EBT_DROP;"));
        assert!(source.contains("if (xt_hooknum(par) != NF_BR_BROUTING)"));
        assert!(source.contains("dev = netdev_master_upper_dev_get_rcu(xt_in(par));"));
        assert!(source.contains("if (!dev)"));
        assert!(source.contains("ether_addr_copy(eth_hdr(skb)->h_dest, dev->dev_addr);"));
        assert!(source.contains("xt_in(par)->dev_addr"));
        assert!(source.contains("skb->pkt_type = PACKET_HOST;"));
        assert!(source.contains("return info->target;"));
        assert!(source.contains("if (BASE_CHAIN && info->target == EBT_RETURN)"));
        assert!(source.contains("hook_mask = par->hook_mask & ~(1 << NF_BR_NUMHOOKS);"));
        assert!(source.contains(".name\t\t= \"redirect\""));
        assert!(source.contains(".family\t\t= NFPROTO_BRIDGE"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Ebtables: Packet redirection to localhost\")")
        );

        let mut packet = RedirectPacket {
            writable: true,
            hooknum: NF_BR_PRE_ROUTING,
            h_dest: [0; 6],
            master_dev_addr: Some([0x02, 0, 0, 0, 0, 1]),
            in_dev_addr: [0x02, 0, 0, 0, 0, 2],
            pkt_type: 9,
        };
        assert_eq!(
            ebt_redirect_tg(&mut packet, EbtRedirectInfo { target: EBT_ACCEPT }),
            EBT_ACCEPT
        );
        assert_eq!(packet.h_dest, [0x02, 0, 0, 0, 0, 1]);
        assert_eq!(packet.pkt_type, PACKET_HOST);
        packet.hooknum = NF_BR_BROUTING;
        assert_eq!(
            ebt_redirect_tg(&mut packet, EbtRedirectInfo { target: EBT_DROP }),
            EBT_DROP
        );
        assert_eq!(packet.h_dest, [0x02, 0, 0, 0, 0, 2]);
        packet.hooknum = NF_BR_PRE_ROUTING;
        packet.master_dev_addr = None;
        assert_eq!(
            ebt_redirect_tg(&mut packet, EbtRedirectInfo { target: EBT_ACCEPT }),
            EBT_DROP
        );
        packet.writable = false;
        assert_eq!(
            ebt_redirect_tg(&mut packet, EbtRedirectInfo { target: EBT_ACCEPT }),
            EBT_DROP
        );

        assert_eq!(
            ebt_redirect_tg_check(
                "nat",
                (1 << NF_BR_NUMHOOKS) | (1 << NF_BR_PRE_ROUTING),
                EbtRedirectInfo { target: EBT_ACCEPT }
            ),
            Ok(())
        );
        assert_eq!(
            ebt_redirect_tg_check(
                "nat",
                (1 << NF_BR_NUMHOOKS) | (1 << NF_BR_PRE_ROUTING),
                EbtRedirectInfo { target: EBT_RETURN }
            ),
            Err(-EINVAL)
        );
        assert_eq!(ebt_redirect_init(), &EBT_REDIRECT_TG_REG);
    }
}
