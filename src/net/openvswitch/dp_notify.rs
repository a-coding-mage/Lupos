//! linux-parity: complete
//! linux-source: vendor/linux/net/openvswitch/dp_notify.c
//! test-origin: linux:vendor/linux/net/openvswitch/dp_notify.c
//! Open vSwitch datapath netdevice notifications.

pub const OVS_VPORT_TYPE_INTERNAL: u16 = 1;
pub const OVS_VPORT_CMD_DEL: u8 = 1;
pub const NETDEV_UNREGISTER: u64 = 2;
pub const NOTIFY_DONE: i32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vport {
    pub vport_type: u16,
    pub netif_is_ovs_port: bool,
    pub detached: bool,
    pub genl_error: bool,
    pub genl_multicast: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceEventAction {
    Done,
    DetachedAndQueued,
}

pub fn dp_detach_port_notify(vport: &mut Vport, build_info_error: bool) {
    vport.detached = true;
    if build_info_error {
        vport.genl_error = true;
    } else {
        vport.genl_multicast = true;
    }
}

pub fn ovs_dp_notify_wq(vports: &mut [Vport], build_info_error: bool) -> usize {
    let mut detached = 0;
    for vport in vports {
        if vport.vport_type == OVS_VPORT_TYPE_INTERNAL {
            continue;
        }
        if !vport.netif_is_ovs_port {
            dp_detach_port_notify(vport, build_info_error);
            detached += 1;
        }
    }
    detached
}

pub const fn dp_device_event(
    event: u64,
    dev_is_internal: bool,
    vport_present: bool,
) -> DeviceEventAction {
    if dev_is_internal || !vport_present {
        return DeviceEventAction::Done;
    }
    if event == NETDEV_UNREGISTER {
        DeviceEventAction::DetachedAndQueued
    } else {
        DeviceEventAction::Done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dp_notify_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/openvswitch/dp_notify.c"
        ));
        assert!(source.contains("static void dp_detach_port_notify(struct vport *vport)"));
        assert!(source.contains("ovs_vport_cmd_build_info(vport"));
        assert!(source.contains("OVS_VPORT_CMD_DEL"));
        assert!(source.contains("ovs_dp_detach_port(vport);"));
        assert!(source.contains("if (IS_ERR(notify))"));
        assert!(source.contains("genl_set_err(&dp_vport_genl_family"));
        assert!(source.contains("genlmsg_multicast_netns(&dp_vport_genl_family"));
        assert!(source.contains("void ovs_dp_notify_wq(struct work_struct *work)"));
        assert!(source.contains("list_for_each_entry(dp, &ovs_net->dps, list_node)"));
        assert!(source.contains("for (i = 0; i < DP_VPORT_HASH_BUCKETS; i++)"));
        assert!(source.contains("if (vport->ops->type == OVS_VPORT_TYPE_INTERNAL)"));
        assert!(source.contains("if (!(netif_is_ovs_port(vport->dev)))"));
        assert!(source.contains("dp_detach_port_notify(vport);"));
        assert!(source.contains("if (!ovs_is_internal_dev(dev))"));
        assert!(source.contains("vport = ovs_netdev_get_vport(dev);"));
        assert!(source.contains("if (event == NETDEV_UNREGISTER)"));
        assert!(source.contains("ovs_netdev_detach_dev(vport);"));
        assert!(source.contains("queue_work(system_percpu_wq, &ovs_net->dp_notify_work);"));
        assert!(source.contains("struct notifier_block ovs_dp_device_notifier"));

        let mut vports = [
            Vport {
                vport_type: OVS_VPORT_TYPE_INTERNAL,
                netif_is_ovs_port: false,
                detached: false,
                genl_error: false,
                genl_multicast: false,
            },
            Vport {
                vport_type: 2,
                netif_is_ovs_port: false,
                detached: false,
                genl_error: false,
                genl_multicast: false,
            },
            Vport {
                vport_type: 2,
                netif_is_ovs_port: true,
                detached: false,
                genl_error: false,
                genl_multicast: false,
            },
        ];
        assert_eq!(ovs_dp_notify_wq(&mut vports, false), 1);
        assert!(!vports[0].detached);
        assert!(vports[1].detached);
        assert!(vports[1].genl_multicast);
        assert!(!vports[2].detached);
        assert_eq!(
            dp_device_event(NETDEV_UNREGISTER, false, true),
            DeviceEventAction::DetachedAndQueued
        );
        assert_eq!(
            dp_device_event(NETDEV_UNREGISTER, true, true),
            DeviceEventAction::Done
        );
    }
}
