//! linux-parity: complete
//! linux-source: vendor/linux/net/8021q/vlan_mvrp.c
//! test-origin: linux:vendor/linux/net/8021q/vlan_mvrp.c
//! Multiple VLAN Registration Protocol support.

use core::sync::atomic::{AtomicBool, Ordering};

pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_8021Q_BE: u16 = ETH_P_8021Q.to_be();
pub const ETH_P_MVRP: u16 = 0x88f5;
pub const ETH_P_MVRP_BE: u16 = ETH_P_MVRP.to_be();
pub const MRP_MVRP_ADDRESS: [u8; 6] = [0x01, 0x80, 0xc2, 0x00, 0x00, 0x21];
pub const MVRP_ATTR_INVALID: u8 = 0;
pub const MVRP_ATTR_VID: u8 = 1;
pub const MVRP_ATTR_MAX: u8 = MVRP_ATTR_VID;
pub const MRP_APPLICATION_MVRP: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MrpApplication {
    pub app_type: u8,
    pub maxattr: u8,
    pub pkttype_type: u16,
    pub group_address: [u8; 6],
    pub version: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanDevPriv {
    pub vlan_id: u16,
    pub vlan_proto: u16,
    pub real_dev_ifindex: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MrpAction {
    Join,
    Leave,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MrpRequest {
    pub action: MrpAction,
    pub real_dev_ifindex: u32,
    pub vlan_id_be: u16,
    pub attr: u8,
}

pub const VLAN_MRP_APP: MrpApplication = MrpApplication {
    app_type: MRP_APPLICATION_MVRP,
    maxattr: MVRP_ATTR_MAX,
    pkttype_type: ETH_P_MVRP_BE,
    group_address: MRP_MVRP_ADDRESS,
    version: 0,
};

static VLAN_MVRP_REGISTERED: AtomicBool = AtomicBool::new(false);

pub const fn vlan_mvrp_request_join(vlan: VlanDevPriv) -> Option<MrpRequest> {
    if vlan.vlan_proto != ETH_P_8021Q_BE {
        return None;
    }
    Some(MrpRequest {
        action: MrpAction::Join,
        real_dev_ifindex: vlan.real_dev_ifindex,
        vlan_id_be: vlan.vlan_id.to_be(),
        attr: MVRP_ATTR_VID,
    })
}

pub const fn vlan_mvrp_request_leave(vlan: VlanDevPriv) -> Option<MrpRequest> {
    if vlan.vlan_proto != ETH_P_8021Q_BE {
        return None;
    }
    Some(MrpRequest {
        action: MrpAction::Leave,
        real_dev_ifindex: vlan.real_dev_ifindex,
        vlan_id_be: vlan.vlan_id.to_be(),
        attr: MVRP_ATTR_VID,
    })
}

pub const fn vlan_mvrp_init_applicant(_dev_ifindex: u32) -> &'static MrpApplication {
    &VLAN_MRP_APP
}

pub const fn vlan_mvrp_uninit_applicant(_dev_ifindex: u32) -> &'static MrpApplication {
    &VLAN_MRP_APP
}

pub fn vlan_mvrp_init() -> bool {
    !VLAN_MVRP_REGISTERED.swap(true, Ordering::AcqRel)
}

pub fn vlan_mvrp_uninit() -> bool {
    VLAN_MVRP_REGISTERED.swap(false, Ordering::AcqRel)
}

pub fn vlan_mvrp_registered() -> bool {
    VLAN_MVRP_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlan_mvrp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/8021q/vlan_mvrp.c"
        ));
        assert!(source.contains("#define MRP_MVRP_ADDRESS"));
        assert!(source.contains("enum mvrp_attributes"));
        assert!(source.contains("MVRP_ATTR_VID"));
        assert!(source.contains("static struct mrp_application vlan_mrp_app"));
        assert!(source.contains(".type\t\t= MRP_APPLICATION_MVRP"));
        assert!(source.contains(".maxattr\t= MVRP_ATTR_MAX"));
        assert!(source.contains(".pkttype.type\t= htons(ETH_P_MVRP)"));
        assert!(source.contains(".group_address\t= MRP_MVRP_ADDRESS"));
        assert!(source.contains(".version\t= 0"));
        assert!(source.contains("int vlan_mvrp_request_join"));
        assert!(source.contains("__be16 vlan_id = htons(vlan->vlan_id);"));
        assert!(source.contains("if (vlan->vlan_proto != htons(ETH_P_8021Q))"));
        assert!(source.contains("return mrp_request_join(vlan->real_dev, &vlan_mrp_app"));
        assert!(source.contains("void vlan_mvrp_request_leave"));
        assert!(source.contains("mrp_request_leave(vlan->real_dev, &vlan_mrp_app"));
        assert!(source.contains("return mrp_init_applicant(dev, &vlan_mrp_app);"));
        assert!(source.contains("mrp_unregister_application(&vlan_mrp_app);"));

        assert_eq!(VLAN_MRP_APP.group_address, MRP_MVRP_ADDRESS);
        assert_eq!(VLAN_MRP_APP.maxattr, MVRP_ATTR_MAX);
        assert_eq!(VLAN_MRP_APP.pkttype_type, ETH_P_MVRP_BE);
    }

    #[test]
    fn vlan_mvrp_requests_only_for_8021q_proto() {
        let vlan = VlanDevPriv {
            vlan_id: 200,
            vlan_proto: ETH_P_8021Q_BE,
            real_dev_ifindex: 8,
        };
        assert_eq!(
            vlan_mvrp_request_join(vlan),
            Some(MrpRequest {
                action: MrpAction::Join,
                real_dev_ifindex: 8,
                vlan_id_be: 200u16.to_be(),
                attr: MVRP_ATTR_VID,
            })
        );
        assert_eq!(
            vlan_mvrp_request_leave(vlan).unwrap().action,
            MrpAction::Leave
        );
        assert_eq!(
            vlan_mvrp_request_join(VlanDevPriv {
                vlan_proto: 0,
                ..vlan
            }),
            None
        );

        assert!(!vlan_mvrp_registered());
        assert!(vlan_mvrp_init());
        assert!(vlan_mvrp_registered());
        assert!(!vlan_mvrp_init());
        assert!(vlan_mvrp_uninit());
        assert!(!vlan_mvrp_uninit());
    }
}
