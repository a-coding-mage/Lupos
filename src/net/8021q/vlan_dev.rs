//! linux-parity: partial
//! linux-source: vendor/linux/net/8021q/vlan_dev.c
//! test-origin: linux:vendor/linux/net/8021q/vlan_dev.c
//! VLAN device MTU, priority mapping, flag, and header helper behavior.

pub const VLAN_HLEN: u32 = 4;
pub const VLAN_PRIO_SHIFT: u16 = 13;
pub const VLAN_PRIO_MASK: u16 = 0xe000;
pub const VLAN_FLAG_REORDER_HDR: u32 = 0x1;
pub const VLAN_FLAG_GVRP: u32 = 0x2;
pub const VLAN_FLAG_LOOSE_BINDING: u32 = 0x4;
pub const VLAN_FLAG_MVRP: u32 = 0x8;
pub const VLAN_FLAG_BRIDGE_BINDING: u32 = 0x10;
pub const VLAN_ALLOWED_FLAGS: u32 = VLAN_FLAG_REORDER_HDR
    | VLAN_FLAG_GVRP
    | VLAN_FLAG_LOOSE_BINDING
    | VLAN_FLAG_MVRP
    | VLAN_FLAG_BRIDGE_BINDING;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VlanDevError {
    Range,
    InvalidFlags,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanPriorityMap {
    pub ingress: [u32; 8],
    pub nr_ingress_mappings: u32,
}

impl VlanPriorityMap {
    pub const fn new() -> Self {
        Self {
            ingress: [0; 8],
            nr_ingress_mappings: 0,
        }
    }
}

pub const fn vlan_dev_change_mtu(
    real_dev_mtu: u32,
    reduces_vlan_mtu: bool,
    new_mtu: u32,
) -> Result<u32, VlanDevError> {
    let max_mtu = if reduces_vlan_mtu {
        real_dev_mtu - VLAN_HLEN
    } else {
        real_dev_mtu
    };
    if max_mtu < new_mtu {
        Err(VlanDevError::Range)
    } else {
        Ok(new_mtu)
    }
}

pub fn vlan_dev_set_ingress_priority(map: &mut VlanPriorityMap, skb_prio: u32, vlan_prio: u16) {
    let index = (vlan_prio & 0x7) as usize;
    if map.ingress[index] != 0 && skb_prio == 0 {
        map.nr_ingress_mappings -= 1;
    } else if map.ingress[index] == 0 && skb_prio != 0 {
        map.nr_ingress_mappings += 1;
    }
    map.ingress[index] = skb_prio;
}

pub const fn vlan_egress_bucket(skb_prio: u32) -> u32 {
    skb_prio & 0x0f
}

pub const fn vlan_egress_qos(vlan_prio: u16) -> u16 {
    (vlan_prio << VLAN_PRIO_SHIFT) & VLAN_PRIO_MASK
}

pub const fn vlan_dev_change_flags(
    old_flags: u32,
    flags: u32,
    mask: u32,
) -> Result<u32, VlanDevError> {
    if mask & !VLAN_ALLOWED_FLAGS != 0 {
        return Err(VlanDevError::InvalidFlags);
    }
    Ok((old_flags & !mask) | (flags & mask))
}

pub const fn vlan_parse_protocol(first_vlan_proto: u16, encapsulated_proto: u16) -> u16 {
    if first_vlan_proto != 0 {
        first_vlan_proto
    } else {
        encapsulated_proto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlan_dev_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/8021q/vlan_dev.c"
        ));
        assert!(source.contains("static int vlan_dev_change_mtu"));
        assert!(source.contains("max_mtu -= VLAN_HLEN;"));
        assert!(source.contains("return -ERANGE;"));
        assert!(source.contains("vlan->ingress_priority_map[vlan_prio & 0x7]"));
        assert!(source.contains("u32 bucket = skb_prio & 0xF;"));
        assert!(source.contains("u32 vlan_qos = (vlan_prio << VLAN_PRIO_SHIFT) & VLAN_PRIO_MASK;"));
        assert!(source.contains("VLAN_FLAG_REORDER_HDR | VLAN_FLAG_GVRP"));
        assert!(source.contains("static __be16 vlan_parse_protocol"));
        assert!(source.contains("static const struct header_ops vlan_header_ops"));
        assert!(source.contains("static const struct net_device_ops vlan_netdev_ops"));
        assert!(source.contains("void vlan_setup(struct net_device *dev)"));
    }

    #[test]
    fn vlan_device_helpers_follow_linux_masks() {
        assert_eq!(vlan_dev_change_mtu(1500, true, 1496), Ok(1496));
        assert_eq!(
            vlan_dev_change_mtu(1500, true, 1497),
            Err(VlanDevError::Range)
        );
        let mut map = VlanPriorityMap::new();
        vlan_dev_set_ingress_priority(&mut map, 42, 9);
        assert_eq!(map.ingress[1], 42);
        assert_eq!(map.nr_ingress_mappings, 1);
        vlan_dev_set_ingress_priority(&mut map, 0, 1);
        assert_eq!(map.nr_ingress_mappings, 0);
        assert_eq!(vlan_egress_bucket(0x31), 1);
        assert_eq!(vlan_egress_qos(5), 0xa000);
        assert_eq!(
            vlan_dev_change_flags(0, VLAN_FLAG_GVRP, VLAN_FLAG_GVRP),
            Ok(VLAN_FLAG_GVRP)
        );
        assert_eq!(
            vlan_dev_change_flags(0, 0x20, 0x20),
            Err(VlanDevError::InvalidFlags)
        );
        assert_eq!(vlan_parse_protocol(0x8100, 0x0800), 0x8100);
        assert_eq!(vlan_parse_protocol(0, 0x0800), 0x0800);
    }

    #[test]
    fn linux_vlan_bridge_binding_selftest_exercises_bridge_flag() {
        let selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/net/vlan_bridge_binding.sh"
        ));
        assert!(selftest.contains("test_binding_on"));
        assert!(selftest.contains("test_binding_off"));
        assert!(selftest.contains("bridge_binding on"));
        assert!(selftest.contains("bridge_binding off"));
        assert!(selftest.contains("set_vlans type vlan bridge_binding on"));

        assert_eq!(
            vlan_dev_change_flags(0, VLAN_FLAG_BRIDGE_BINDING, VLAN_FLAG_BRIDGE_BINDING,),
            Ok(VLAN_FLAG_BRIDGE_BINDING)
        );
        assert_eq!(
            vlan_dev_change_flags(VLAN_FLAG_BRIDGE_BINDING, 0, VLAN_FLAG_BRIDGE_BINDING,),
            Ok(0)
        );
    }
}
