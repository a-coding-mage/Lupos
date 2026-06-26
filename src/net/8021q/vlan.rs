//! linux-parity: partial
//! linux-source: vendor/linux/net/8021q/vlan.c
//! test-origin: linux:vendor/linux/net/8021q/vlan.c
//! 802.1Q VLAN module metadata, protocol indexing, VID partitioning, and real-device checks.

pub const DRV_VERSION: &str = "1.8";
pub const VLAN_FULLNAME: &str = "802.1Q VLAN Support";
pub const VLAN_VERSION: &str = DRV_VERSION;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_8021AD: u16 = 0x88a8;
pub const ETH_P_8021Q_BE: u16 = ETH_P_8021Q.to_be();
pub const ETH_P_8021AD_BE: u16 = ETH_P_8021AD.to_be();
pub const VLAN_N_VID: u16 = 4096;
pub const VLAN_VID_MASK: u16 = 0x0fff;
pub const VLAN_GROUP_ARRAY_SPLIT_PARTS: u16 = 8;
pub const VLAN_GROUP_ARRAY_PART_LEN: u16 = VLAN_N_VID / VLAN_GROUP_ARRAY_SPLIT_PARTS;
pub const ARPHRD_ETHER: u16 = 1;
pub const NETIF_F_VLAN_CHALLENGED: u64 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VlanProto {
    Dot1Q,
    Dot1Ad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VlanError {
    InvalidProtocol,
    InvalidVid,
    UnsupportedDevice,
    AlreadyExists,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VlanNameType {
    PlusVid,
    RawPlusVid,
    PlusVidNoPad,
    RawPlusVidNoPad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanGroupSlot {
    pub proto_index: usize,
    pub array_index: usize,
    pub offset: usize,
}

pub const fn vlan_proto_idx(proto_be: u16) -> Result<VlanProto, VlanError> {
    match proto_be {
        ETH_P_8021Q_BE => Ok(VlanProto::Dot1Q),
        ETH_P_8021AD_BE => Ok(VlanProto::Dot1Ad),
        _ => Err(VlanError::InvalidProtocol),
    }
}

pub const fn vlan_id_valid(vlan_id: u16) -> bool {
    vlan_id < VLAN_VID_MASK
}

pub const fn vlan_group_slot(proto_be: u16, vlan_id: u16) -> Result<VlanGroupSlot, VlanError> {
    if !vlan_id_valid(vlan_id) {
        return Err(VlanError::InvalidVid);
    }
    let proto_index = match vlan_proto_idx(proto_be) {
        Ok(VlanProto::Dot1Q) => 0,
        Ok(VlanProto::Dot1Ad) => 1,
        Err(err) => return Err(err),
    };
    Ok(VlanGroupSlot {
        proto_index,
        array_index: (vlan_id / VLAN_GROUP_ARRAY_PART_LEN) as usize,
        offset: (vlan_id % VLAN_GROUP_ARRAY_PART_LEN) as usize,
    })
}

pub const fn vlan_check_real_dev(
    features: u64,
    dev_type: u16,
    existing_vlan: bool,
) -> Result<(), VlanError> {
    if features & NETIF_F_VLAN_CHALLENGED != 0 || dev_type != ARPHRD_ETHER {
        return Err(VlanError::UnsupportedDevice);
    }
    if existing_vlan {
        return Err(VlanError::AlreadyExists);
    }
    Ok(())
}

pub const fn register_vlan_device_defaults(vlan_id: u16) -> Result<(u16, u32), VlanError> {
    if !vlan_id_valid(vlan_id) {
        return Err(VlanError::InvalidVid);
    }
    Ok((ETH_P_8021Q_BE, 0x1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlan_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/8021q/vlan.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/8021q/vlan.h"
        ));
        assert!(source.contains("#define DRV_VERSION \"1.8\""));
        assert!(source.contains("const char vlan_fullname[] = \"802.1Q VLAN Support\";"));
        assert!(source.contains("const char vlan_version[] = DRV_VERSION;"));
        assert!(source.contains("vlan_group_prealloc_vid"));
        assert!(source.contains("vlan_id / VLAN_GROUP_ARRAY_PART_LEN"));
        assert!(source.contains("real_dev->features & NETIF_F_VLAN_CHALLENGED"));
        assert!(source.contains("real_dev->type != ARPHRD_ETHER"));
        assert!(source.contains("vlan_find_dev(real_dev, protocol, vlan_id) != NULL"));
        assert!(source.contains("if (vlan_id >= VLAN_VID_MASK)"));
        assert!(source.contains("vlan->vlan_proto = htons(ETH_P_8021Q);"));
        assert!(source.contains("vlan->flags = VLAN_FLAG_REORDER_HDR;"));
        assert!(source.contains("MODULE_DESCRIPTION(\"802.1Q/802.1ad VLAN Protocol\")"));
        assert!(header.contains("#define VLAN_GROUP_ARRAY_SPLIT_PARTS  8"));
        assert!(header.contains("case htons(ETH_P_8021Q):"));
    }

    #[test]
    fn vlan_indexing_and_real_dev_checks_follow_linux() {
        assert_eq!(VLAN_FULLNAME, "802.1Q VLAN Support");
        assert_eq!(vlan_proto_idx(ETH_P_8021Q_BE), Ok(VlanProto::Dot1Q));
        assert_eq!(vlan_proto_idx(ETH_P_8021AD_BE), Ok(VlanProto::Dot1Ad));
        assert_eq!(vlan_proto_idx(0), Err(VlanError::InvalidProtocol));
        assert_eq!(
            vlan_group_slot(ETH_P_8021Q_BE, 1025),
            Ok(VlanGroupSlot {
                proto_index: 0,
                array_index: 2,
                offset: 1,
            })
        );
        assert_eq!(
            vlan_check_real_dev(NETIF_F_VLAN_CHALLENGED, ARPHRD_ETHER, false),
            Err(VlanError::UnsupportedDevice)
        );
        assert_eq!(
            vlan_check_real_dev(0, ARPHRD_ETHER, true),
            Err(VlanError::AlreadyExists)
        );
        assert_eq!(
            register_vlan_device_defaults(4095),
            Err(VlanError::InvalidVid)
        );
        assert_eq!(register_vlan_device_defaults(5), Ok((ETH_P_8021Q_BE, 0x1)));
    }

    #[test]
    fn linux_vlan_hw_filter_selftest_vid0_vector_is_accepted() {
        let selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/net/vlan_hw_filter.sh"
        ));
        assert!(selftest.contains("test_vlan_filter_check"));
        assert!(selftest.contains("type vlan id 0"));
        assert!(selftest.contains("type vlan id 0 protocol 802.1q"));
        assert!(selftest.contains("rx-vlan-filter on"));

        assert!(vlan_id_valid(0));
        assert_eq!(
            vlan_group_slot(ETH_P_8021Q_BE, 0),
            Ok(VlanGroupSlot {
                proto_index: 0,
                array_index: 0,
                offset: 0,
            })
        );
    }
}
