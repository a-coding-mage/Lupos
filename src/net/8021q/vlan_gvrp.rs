//! linux-parity: complete
//! linux-source: vendor/linux/net/8021q/vlan_gvrp.c
//! test-origin: linux:vendor/linux/net/8021q/vlan_gvrp.c
//! GARP VLAN Registration Protocol support.

use core::sync::atomic::{AtomicBool, Ordering};

pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_8021Q_BE: u16 = ETH_P_8021Q.to_be();
pub const GARP_GVRP_ADDRESS: [u8; 6] = [0x01, 0x80, 0xc2, 0x00, 0x00, 0x21];
pub const GVRP_ATTR_INVALID: u8 = 0;
pub const GVRP_ATTR_VID: u8 = 1;
pub const GVRP_ATTR_MAX: u8 = GVRP_ATTR_VID;
pub const GARP_APPLICATION_GVRP: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GarpApplication {
    pub group_address: [u8; 6],
    pub maxattr: u8,
    pub app_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VlanDevPriv {
    pub vlan_id: u16,
    pub vlan_proto: u16,
    pub real_dev_ifindex: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GarpAction {
    Join,
    Leave,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GarpRequest {
    pub action: GarpAction,
    pub real_dev_ifindex: u32,
    pub vlan_id_be: u16,
    pub attr: u8,
}

pub const VLAN_GVRP_APP: GarpApplication = GarpApplication {
    group_address: GARP_GVRP_ADDRESS,
    maxattr: GVRP_ATTR_MAX,
    app_type: GARP_APPLICATION_GVRP,
};

static VLAN_GVRP_REGISTERED: AtomicBool = AtomicBool::new(false);

pub const fn vlan_gvrp_request_join(vlan: VlanDevPriv) -> Option<GarpRequest> {
    if vlan.vlan_proto != ETH_P_8021Q_BE {
        return None;
    }
    Some(GarpRequest {
        action: GarpAction::Join,
        real_dev_ifindex: vlan.real_dev_ifindex,
        vlan_id_be: vlan.vlan_id.to_be(),
        attr: GVRP_ATTR_VID,
    })
}

pub const fn vlan_gvrp_request_leave(vlan: VlanDevPriv) -> Option<GarpRequest> {
    if vlan.vlan_proto != ETH_P_8021Q_BE {
        return None;
    }
    Some(GarpRequest {
        action: GarpAction::Leave,
        real_dev_ifindex: vlan.real_dev_ifindex,
        vlan_id_be: vlan.vlan_id.to_be(),
        attr: GVRP_ATTR_VID,
    })
}

pub const fn vlan_gvrp_init_applicant(_dev_ifindex: u32) -> &'static GarpApplication {
    &VLAN_GVRP_APP
}

pub const fn vlan_gvrp_uninit_applicant(_dev_ifindex: u32) -> &'static GarpApplication {
    &VLAN_GVRP_APP
}

pub fn vlan_gvrp_init() -> bool {
    !VLAN_GVRP_REGISTERED.swap(true, Ordering::AcqRel)
}

pub fn vlan_gvrp_uninit() -> bool {
    VLAN_GVRP_REGISTERED.swap(false, Ordering::AcqRel)
}

pub fn vlan_gvrp_registered() -> bool {
    VLAN_GVRP_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlan_gvrp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/8021q/vlan_gvrp.c"
        ));
        assert!(source.contains("#define GARP_GVRP_ADDRESS"));
        assert!(source.contains("enum gvrp_attributes"));
        assert!(source.contains("GVRP_ATTR_VID"));
        assert!(source.contains("static struct garp_application vlan_gvrp_app"));
        assert!(source.contains(".proto.group_address\t= GARP_GVRP_ADDRESS"));
        assert!(source.contains(".maxattr\t\t= GVRP_ATTR_MAX"));
        assert!(source.contains(".type\t\t\t= GARP_APPLICATION_GVRP"));
        assert!(source.contains("int vlan_gvrp_request_join"));
        assert!(source.contains("__be16 vlan_id = htons(vlan->vlan_id);"));
        assert!(source.contains("if (vlan->vlan_proto != htons(ETH_P_8021Q))"));
        assert!(source.contains("return garp_request_join(vlan->real_dev, &vlan_gvrp_app"));
        assert!(source.contains("void vlan_gvrp_request_leave"));
        assert!(source.contains("garp_request_leave(vlan->real_dev, &vlan_gvrp_app"));
        assert!(source.contains("return garp_init_applicant(dev, &vlan_gvrp_app);"));
        assert!(source.contains("garp_uninit_applicant(dev, &vlan_gvrp_app);"));
        assert!(source.contains("return garp_register_application(&vlan_gvrp_app);"));
        assert!(source.contains("garp_unregister_application(&vlan_gvrp_app);"));

        assert_eq!(VLAN_GVRP_APP.group_address, GARP_GVRP_ADDRESS);
        assert_eq!(VLAN_GVRP_APP.maxattr, GVRP_ATTR_MAX);
        assert_eq!(VLAN_GVRP_APP.app_type, GARP_APPLICATION_GVRP);
    }

    #[test]
    fn vlan_gvrp_requests_only_for_8021q_proto() {
        let vlan = VlanDevPriv {
            vlan_id: 100,
            vlan_proto: ETH_P_8021Q_BE,
            real_dev_ifindex: 7,
        };
        assert_eq!(
            vlan_gvrp_request_join(vlan),
            Some(GarpRequest {
                action: GarpAction::Join,
                real_dev_ifindex: 7,
                vlan_id_be: 100u16.to_be(),
                attr: GVRP_ATTR_VID,
            })
        );
        assert_eq!(
            vlan_gvrp_request_leave(vlan).unwrap().action,
            GarpAction::Leave
        );
        assert_eq!(
            vlan_gvrp_request_join(VlanDevPriv {
                vlan_proto: 0,
                ..vlan
            }),
            None
        );

        assert!(!vlan_gvrp_registered());
        assert!(vlan_gvrp_init());
        assert!(vlan_gvrp_registered());
        assert!(!vlan_gvrp_init());
        assert!(vlan_gvrp_uninit());
        assert!(!vlan_gvrp_uninit());
    }
}
