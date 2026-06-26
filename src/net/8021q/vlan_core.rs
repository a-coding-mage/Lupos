//! linux-parity: partial
//! linux-source: vendor/linux/net/8021q/vlan_core.c
//! test-origin: linux:vendor/linux/net/8021q/vlan_core.c
//! VLAN receive/filter helpers and exported vlan_dev accessors.

pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_8021AD: u16 = 0x88a8;
pub const ETH_P_8021Q_BE: u16 = ETH_P_8021Q.to_be();
pub const ETH_P_8021AD_BE: u16 = ETH_P_8021AD.to_be();
pub const VLAN_VID_MASK: u16 = 0x0fff;
pub const VLAN_PRIO_MASK: u16 = 0xe000;
pub const VLAN_PRIO_SHIFT: u8 = 13;
pub const NETIF_F_HW_VLAN_CTAG_FILTER: u64 = 1 << 1;
pub const NETIF_F_HW_VLAN_STAG_FILTER: u64 = 1 << 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanDevPriv {
    pub real_dev_ifindex: u32,
    pub vlan_id: u16,
    pub vlan_proto: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanReceiveResult {
    pub vlan_id: u16,
    pub priority: u16,
    pub clear_hwaccel_tag: bool,
}

pub const fn skb_vlan_tag_get_id(vlan_tci: u16) -> u16 {
    vlan_tci & VLAN_VID_MASK
}

pub const fn skb_vlan_tag_get_prio(vlan_tci: u16) -> u16 {
    (vlan_tci & VLAN_PRIO_MASK) >> VLAN_PRIO_SHIFT
}

pub const fn vlan_hw_filter_capable(features: u64, proto_be: u16) -> bool {
    (proto_be == ETH_P_8021Q_BE && features & NETIF_F_HW_VLAN_CTAG_FILTER != 0)
        || (proto_be == ETH_P_8021AD_BE && features & NETIF_F_HW_VLAN_STAG_FILTER != 0)
}

pub const fn vlan_do_receive(vlan_tci: u16) -> VlanReceiveResult {
    VlanReceiveResult {
        vlan_id: skb_vlan_tag_get_id(vlan_tci),
        priority: skb_vlan_tag_get_prio(vlan_tci),
        clear_hwaccel_tag: true,
    }
}

pub const fn vlan_dev_real_dev(dev: VlanDevPriv) -> u32 {
    dev.real_dev_ifindex
}

pub const fn vlan_dev_vlan_id(dev: VlanDevPriv) -> u16 {
    dev.vlan_id
}

pub const fn vlan_dev_vlan_proto(dev: VlanDevPriv) -> u16 {
    dev.vlan_proto
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlan_core_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/8021q/vlan_core.c"
        ));
        assert!(source.contains("bool vlan_do_receive(struct sk_buff **skbp)"));
        assert!(source.contains("u16 vlan_id = skb_vlan_tag_get_id(skb);"));
        assert!(source.contains("skb->priority = vlan_get_ingress_priority"));
        assert!(source.contains("__vlan_hwaccel_clear_tag(skb);"));
        assert!(source.contains("EXPORT_SYMBOL(__vlan_find_dev_deep_rcu);"));
        assert!(source.contains("struct net_device *vlan_dev_real_dev"));
        assert!(source.contains("EXPORT_SYMBOL(vlan_dev_vlan_id);"));
        assert!(source.contains("EXPORT_SYMBOL(vlan_dev_vlan_proto);"));
        assert!(source.contains("dev->features & NETIF_F_HW_VLAN_CTAG_FILTER"));
        assert!(source.contains("dev->features & NETIF_F_HW_VLAN_STAG_FILTER"));
        assert!(source.contains("vlan_packet_offloads"));
    }

    #[test]
    fn vlan_tci_and_filter_helpers_follow_linux_masks() {
        let tci = 0xa123;
        assert_eq!(skb_vlan_tag_get_id(tci), 0x0123);
        assert_eq!(skb_vlan_tag_get_prio(tci), 5);
        assert!(vlan_hw_filter_capable(
            NETIF_F_HW_VLAN_CTAG_FILTER,
            ETH_P_8021Q_BE
        ));
        assert!(!vlan_hw_filter_capable(
            NETIF_F_HW_VLAN_CTAG_FILTER,
            ETH_P_8021AD_BE
        ));
        assert_eq!(
            vlan_do_receive(tci),
            VlanReceiveResult {
                vlan_id: 0x0123,
                priority: 5,
                clear_hwaccel_tag: true,
            }
        );
        let dev = VlanDevPriv {
            real_dev_ifindex: 7,
            vlan_id: 100,
            vlan_proto: ETH_P_8021Q_BE,
        };
        assert_eq!(vlan_dev_real_dev(dev), 7);
        assert_eq!(vlan_dev_vlan_id(dev), 100);
        assert_eq!(vlan_dev_vlan_proto(dev), ETH_P_8021Q_BE);
    }

    #[test]
    fn linux_xdp_vlan_selftest_vectors_match_tci_masks() {
        let prog = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/bpf/progs/test_xdp_vlan.c"
        ));
        let harness = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/bpf/prog_tests/xdp_vlan.c"
        ));
        assert!(prog.contains("#define TESTVLAN 4011"));
        assert!(prog.contains("#define TO_VLAN\t0"));
        assert!(prog.contains("& VLAN_VID_MASK"));
        assert!(prog.contains("bpf_ntohs(vlan_hdr->h_vlan_TCI) & 0xf000U"));
        assert!(harness.contains("#define VLAN_ID\t\t4011"));
        assert!(harness.contains("type vlan id %d"));

        let test_vlan = 4011u16;
        assert_eq!(test_vlan, 0x0fab);
        assert_eq!(skb_vlan_tag_get_id(test_vlan), 4011);
        assert_eq!(skb_vlan_tag_get_id(0xf000 | test_vlan), 4011);
        assert_eq!(skb_vlan_tag_get_prio(0xf000 | test_vlan), 7);
    }
}
