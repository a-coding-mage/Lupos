//! linux-parity: complete
//! linux-source: vendor/linux/net/openvswitch/vport-gre.c
//! test-origin: linux:vendor/linux/net/openvswitch/vport-gre.c
//! Open vSwitch GRE vport creation and registration helpers.

pub const MODULE_DESCRIPTION: &str = "OVS: GRE switching port";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "vport-type-3";
pub const OVS_VPORT_TYPE_GRE: u16 = 3;
pub const IFF_UP: u32 = 0x1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vport {
    pub name: alloc::string::String,
    pub dev_created: bool,
    pub dev_up: bool,
    pub linked: bool,
    pub held: bool,
}

extern crate alloc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GreCreateError {
    VportAlloc,
    DevCreate,
    ChangeFlags(i32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VportOps {
    pub vport_type: u16,
    pub create: &'static str,
    pub send: &'static str,
    pub destroy: &'static str,
}

pub const OVS_GRE_VPORT_OPS: VportOps = VportOps {
    vport_type: OVS_VPORT_TYPE_GRE,
    create: "gre_create",
    send: "dev_queue_xmit",
    destroy: "ovs_netdev_tunnel_destroy",
};

pub fn gre_tnl_create(
    name: &str,
    vport_alloc_ok: bool,
    dev_create_ok: bool,
    change_flags_ret: i32,
) -> Result<Vport, GreCreateError> {
    if !vport_alloc_ok {
        return Err(GreCreateError::VportAlloc);
    }
    if !dev_create_ok {
        return Err(GreCreateError::DevCreate);
    }
    if change_flags_ret < 0 {
        return Err(GreCreateError::ChangeFlags(change_flags_ret));
    }
    Ok(Vport {
        name: name.into(),
        dev_created: true,
        dev_up: true,
        linked: false,
        held: true,
    })
}

pub fn gre_create(
    name: &str,
    vport_alloc_ok: bool,
    dev_create_ok: bool,
    change_flags_ret: i32,
    link_ok: bool,
) -> Result<Vport, GreCreateError> {
    let mut vport = gre_tnl_create(name, vport_alloc_ok, dev_create_ok, change_flags_ret)?;
    vport.linked = link_ok;
    Ok(vport)
}

pub const fn ovs_gre_tnl_init(register_ret: i32) -> Result<&'static VportOps, i32> {
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(&OVS_GRE_VPORT_OPS)
    }
}

pub const fn ovs_gre_tnl_exit() -> &'static VportOps {
    &OVS_GRE_VPORT_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ovs_vport_gre_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/openvswitch/vport-gre.c"
        ));
        assert!(source.contains("static struct vport_ops ovs_gre_vport_ops;"));
        assert!(source.contains("static struct vport *gre_tnl_create"));
        assert!(source.contains("ovs_vport_alloc(0, &ovs_gre_vport_ops, parms);"));
        assert!(source.contains("rtnl_lock();"));
        assert!(source.contains("gretap_fb_dev_create(net, parms->name, NET_NAME_USER);"));
        assert!(source.contains("dev_change_flags(dev, dev->flags | IFF_UP, NULL);"));
        assert!(source.contains("rtnl_delete_link(dev, 0, NULL);"));
        assert!(source.contains("netdev_hold(vport->dev, &vport->dev_tracker, GFP_KERNEL);"));
        assert!(source.contains("return ovs_netdev_link(vport, true);"));
        assert!(source.contains(".type\t\t= OVS_VPORT_TYPE_GRE"));
        assert!(source.contains(".send\t\t= dev_queue_xmit"));
        assert!(source.contains("ovs_vport_ops_register(&ovs_gre_vport_ops);"));
        assert!(source.contains("ovs_vport_ops_unregister(&ovs_gre_vport_ops);"));
        assert!(source.contains("MODULE_ALIAS(\"vport-type-3\");"));
    }

    #[test]
    fn gre_create_tracks_device_setup_and_error_edges() {
        let vport = gre_create("gre0", true, true, 0, true).unwrap();
        assert_eq!(vport.name, "gre0");
        assert!(vport.dev_created && vport.dev_up && vport.held && vport.linked);
        assert_eq!(
            gre_tnl_create("gre0", false, true, 0),
            Err(GreCreateError::VportAlloc)
        );
        assert_eq!(
            gre_tnl_create("gre0", true, false, 0),
            Err(GreCreateError::DevCreate)
        );
        assert_eq!(
            gre_tnl_create("gre0", true, true, -5),
            Err(GreCreateError::ChangeFlags(-5))
        );
        assert_eq!(ovs_gre_tnl_init(0), Ok(&OVS_GRE_VPORT_OPS));
        assert_eq!(ovs_gre_tnl_init(-2), Err(-2));
        assert_eq!(ovs_gre_tnl_exit(), &OVS_GRE_VPORT_OPS);
    }
}
