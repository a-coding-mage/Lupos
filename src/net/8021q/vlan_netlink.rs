//! linux-parity: partial
//! linux-source: vendor/linux/net/8021q/vlan_netlink.c
//! test-origin: linux:vendor/linux/net/8021q/vlan_netlink.c
//! VLAN rtnetlink validation and size accounting helpers.

pub const ETH_ALEN: usize = 6;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_8021AD: u16 = 0x88a8;
pub const ETH_P_8021Q_BE: u16 = ETH_P_8021Q.to_be();
pub const ETH_P_8021AD_BE: u16 = ETH_P_8021AD.to_be();
pub const VLAN_VID_MASK: u16 = 0x0fff;
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
pub const NLATTR_HDRLEN: usize = 4;
pub const IFLA_VLAN_QOS_MAPPING_SIZE: usize = 8;
pub const IFLA_VLAN_FLAGS_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VlanNetlinkError {
    InvalidAddressLen,
    MissingProperties,
    InvalidProtocol,
    InvalidId,
    InvalidFlags,
    MissingId,
    MissingLink,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanNetlinkAttrs {
    pub address_len: Option<usize>,
    pub data_present: bool,
    pub protocol_be: Option<u16>,
    pub vlan_id: Option<u16>,
    pub flags: Option<u32>,
    pub flag_mask: Option<u32>,
}

pub const fn vlan_validate(attrs: VlanNetlinkAttrs) -> Result<(), VlanNetlinkError> {
    if let Some(len) = attrs.address_len {
        if len != ETH_ALEN {
            return Err(VlanNetlinkError::InvalidAddressLen);
        }
    }
    if !attrs.data_present {
        return Err(VlanNetlinkError::MissingProperties);
    }
    if let Some(proto) = attrs.protocol_be {
        if proto != ETH_P_8021Q_BE && proto != ETH_P_8021AD_BE {
            return Err(VlanNetlinkError::InvalidProtocol);
        }
    }
    if let Some(id) = attrs.vlan_id {
        if id >= VLAN_VID_MASK {
            return Err(VlanNetlinkError::InvalidId);
        }
    }
    if let (Some(flags), Some(mask)) = (attrs.flags, attrs.flag_mask) {
        if (flags & mask) & !VLAN_ALLOWED_FLAGS != 0 {
            return Err(VlanNetlinkError::InvalidFlags);
        }
    }
    Ok(())
}

pub const fn vlan_newlink_required(
    has_vlan_id: bool,
    has_link: bool,
) -> Result<(), VlanNetlinkError> {
    if !has_vlan_id {
        return Err(VlanNetlinkError::MissingId);
    }
    if !has_link {
        return Err(VlanNetlinkError::MissingLink);
    }
    Ok(())
}

pub const fn nla_align(len: usize) -> usize {
    (len + 3) & !3
}

pub const fn nla_total_size(payload: usize) -> usize {
    nla_align(NLATTR_HDRLEN + payload)
}

pub const fn vlan_qos_map_size(n: usize) -> usize {
    if n == 0 {
        0
    } else {
        nla_total_size(NLATTR_HDRLEN) + nla_total_size(IFLA_VLAN_QOS_MAPPING_SIZE) * n
    }
}

pub const fn vlan_get_size(nr_ingress_mappings: usize, nr_egress_mappings: usize) -> usize {
    nla_total_size(2)
        + nla_total_size(2)
        + nla_total_size(IFLA_VLAN_FLAGS_SIZE)
        + vlan_qos_map_size(nr_ingress_mappings)
        + vlan_qos_map_size(nr_egress_mappings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlan_netlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/8021q/vlan_netlink.c"
        ));
        assert!(source.contains("static const struct nla_policy vlan_policy"));
        assert!(source.contains("[IFLA_VLAN_ID]\t\t= { .type = NLA_U16 }"));
        assert!(source.contains("if (nla_len(tb[IFLA_ADDRESS]) != ETH_ALEN)"));
        assert!(source.contains("VLAN properties not specified"));
        assert!(source.contains("case htons(ETH_P_8021Q):"));
        assert!(source.contains("case htons(ETH_P_8021AD):"));
        assert!(source.contains("if (id >= VLAN_VID_MASK)"));
        assert!(source.contains("Invalid VLAN flags"));
        assert!(source.contains("static inline size_t vlan_qos_map_size"));
        assert!(source.contains("IFLA_VLAN_{EGRESS,INGRESS}_QOS + n * IFLA_VLAN_QOS_MAPPING"));
        assert!(source.contains(".kind\t\t= \"vlan\""));
        assert!(source.contains("MODULE_ALIAS_RTNL_LINK(\"vlan\");"));
    }

    #[test]
    fn netlink_validation_and_sizes_follow_linux() {
        let valid = VlanNetlinkAttrs {
            address_len: Some(ETH_ALEN),
            data_present: true,
            protocol_be: Some(ETH_P_8021Q_BE),
            vlan_id: Some(100),
            flags: Some(VLAN_FLAG_GVRP),
            flag_mask: Some(VLAN_FLAG_GVRP),
        };
        assert_eq!(vlan_validate(valid), Ok(()));
        assert_eq!(
            vlan_validate(VlanNetlinkAttrs {
                address_len: Some(5),
                ..valid
            }),
            Err(VlanNetlinkError::InvalidAddressLen)
        );
        assert_eq!(
            vlan_validate(VlanNetlinkAttrs {
                protocol_be: Some(0),
                ..valid
            }),
            Err(VlanNetlinkError::InvalidProtocol)
        );
        assert_eq!(
            vlan_validate(VlanNetlinkAttrs {
                vlan_id: Some(4095),
                ..valid
            }),
            Err(VlanNetlinkError::InvalidId)
        );
        assert_eq!(
            vlan_validate(VlanNetlinkAttrs {
                flags: Some(0x20),
                flag_mask: Some(0x20),
                ..valid
            }),
            Err(VlanNetlinkError::InvalidFlags)
        );
        assert_eq!(
            vlan_newlink_required(false, true),
            Err(VlanNetlinkError::MissingId)
        );
        assert_eq!(
            vlan_newlink_required(true, false),
            Err(VlanNetlinkError::MissingLink)
        );
        assert_eq!(vlan_qos_map_size(0), 0);
        assert_eq!(
            vlan_qos_map_size(2),
            nla_total_size(4) + nla_total_size(8) * 2
        );
        assert_eq!(
            vlan_get_size(1, 1),
            nla_total_size(2) * 2 + nla_total_size(8) + vlan_qos_map_size(1) * 2
        );
    }
}
