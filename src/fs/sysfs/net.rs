//! linux-parity: partial
//! linux-source: vendor/linux/net/core/net-sysfs.c
//! Sysfs exposure for network devices.

extern crate alloc;

use alloc::format;
use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::kernfs::{KernfsNode, add_child, lookup};
use crate::include::uapi::errno::{EINVAL, ENODEV};
use crate::net::device::{IFF_LOOPBACK, IFF_UP, LOOPBACK_MTU, NetDeviceRef, list_netdevices};
use crate::net::uevent::UeventAction;

lazy_static! {
    static ref NET_CLASS_ROOT: Mutex<Option<Arc<KernfsNode>>> = Mutex::new(None);
    static ref NET_DEVICES_ROOT: Mutex<Option<Arc<KernfsNode>>> = Mutex::new(None);
}

fn copy_text(buf: &mut [u8], text: &str) -> Result<usize, i32> {
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

fn netdev_from_node(node: &Arc<KernfsNode>) -> Result<NetDeviceRef, i32> {
    let ifindex = node.priv_ptr.load(Ordering::Acquire) as u32;
    list_netdevices()
        .into_iter()
        .find(|dev| dev.ifindex == ifindex)
        .ok_or(ENODEV)
}

fn netdev_attr_file(
    ifindex: u32,
    name: &str,
    mode: u32,
    show: fn(&Arc<KernfsNode>, &mut [u8]) -> Result<usize, i32>,
) -> Arc<KernfsNode> {
    let file = KernfsNode::new_file(name, mode, Some(show), None);
    file.priv_ptr.store(ifindex as u64, Ordering::Release);
    file
}

fn netdev_uevent_file(ifindex: u32) -> Arc<KernfsNode> {
    let file = KernfsNode::new_file(
        "uevent",
        0o644,
        Some(|node, buf| {
            let dev = netdev_from_node(node)?;
            copy_text(
                buf,
                &format!("INTERFACE={}\nIFINDEX={}\n", dev.name, dev.ifindex),
            )
        }),
        Some(netdev_uevent_store),
    );
    file.priv_ptr.store(ifindex as u64, Ordering::Release);
    file
}

fn parse_uevent_action(buf: &[u8]) -> Result<UeventAction, i32> {
    let end = buf
        .iter()
        .position(|b| matches!(*b, 0 | b'\n'))
        .unwrap_or(buf.len());
    let text = core::str::from_utf8(&buf[..end])
        .map_err(|_| EINVAL)?
        .trim();
    let action = text.split_ascii_whitespace().next().ok_or(EINVAL)?;
    match action {
        "add" => Ok(UeventAction::Add),
        "remove" => Ok(UeventAction::Remove),
        "change" => Ok(UeventAction::Change),
        "online" => Ok(UeventAction::Online),
        "offline" => Ok(UeventAction::Offline),
        "move" => Ok(UeventAction::Move),
        "bind" => Ok(UeventAction::Bind),
        "unbind" => Ok(UeventAction::Unbind),
        _ => Err(EINVAL),
    }
}

fn netdev_uevent_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let action = parse_uevent_action(buf)?;
    crate::net::uevent::announce_netdevice(action, &dev.name, dev.ifindex);
    Ok(buf.len())
}

fn netdev_ifindex_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    copy_text(buf, &format!("{}\n", dev.ifindex))
}

fn netdev_iflink_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    copy_text(buf, &format!("{}\n", dev.ifindex))
}

fn netdev_flags_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let flags = dev.flags.load(Ordering::Acquire);
    copy_text(buf, &format!("0x{:x}\n", flags))
}

fn netdev_mtu_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    copy_text(buf, &format!("{}\n", dev.mtu))
}

fn netdev_tx_queue_len_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let qlen = if dev.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
        0
    } else {
        1000
    };
    copy_text(buf, &format!("{}\n", qlen))
}

fn netdev_operstate_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let flags = dev.flags.load(Ordering::Acquire);
    let state = if flags & IFF_LOOPBACK != 0 {
        "unknown"
    } else if flags & IFF_UP != 0 && dev.carrier_ok() {
        "up"
    } else {
        "down"
    };
    copy_text(buf, &format!("{state}\n"))
}

fn netdev_carrier_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    copy_text(buf, if dev.carrier_ok() { "1\n" } else { "0\n" })
}

fn netdev_type_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let arphrd = if dev.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
        772
    } else {
        1
    };
    copy_text(buf, &format!("{arphrd}\n"))
}

fn netdev_addr_len_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let len = if dev.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
        6
    } else {
        dev.dev_addr.len()
    };
    copy_text(buf, &format!("{len}\n"))
}

fn fmt_mac(mac: [u8; 6]) -> alloc::string::String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\n",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

fn netdev_address_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    copy_text(buf, &fmt_mac(dev.dev_addr))
}

fn netdev_broadcast_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let broadcast = if dev.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
        [0; 6]
    } else {
        [0xff; 6]
    };
    copy_text(buf, &fmt_mac(broadcast))
}

fn netdev_speed_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let speed = if dev.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
        0
    } else {
        10_000
    };
    copy_text(buf, &format!("{speed}\n"))
}

fn netdev_duplex_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let dev = netdev_from_node(node)?;
    let duplex = if dev.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
        "unknown"
    } else {
        "full"
    };
    copy_text(buf, &format!("{duplex}\n"))
}

fn netdev_zero_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0\n")
}

fn build_virtual_net_dir(dev: &NetDeviceRef) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(&dev.name, 0o555);
    let ifindex = dev.ifindex;
    let mtu = if dev.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
        LOOPBACK_MTU
    } else {
        dev.mtu
    };

    for file in [
        netdev_attr_file(ifindex, "ifindex", 0o444, netdev_ifindex_show),
        netdev_attr_file(ifindex, "iflink", 0o444, netdev_iflink_show),
        netdev_attr_file(ifindex, "flags", 0o444, netdev_flags_show),
        netdev_attr_file(ifindex, "mtu", 0o444, netdev_mtu_show),
        netdev_attr_file(ifindex, "operstate", 0o444, netdev_operstate_show),
        netdev_attr_file(ifindex, "carrier", 0o444, netdev_carrier_show),
        netdev_attr_file(ifindex, "type", 0o444, netdev_type_show),
        netdev_attr_file(ifindex, "addr_len", 0o444, netdev_addr_len_show),
        netdev_attr_file(ifindex, "address", 0o444, netdev_address_show),
        netdev_attr_file(ifindex, "broadcast", 0o444, netdev_broadcast_show),
        netdev_attr_file(ifindex, "tx_queue_len", 0o444, netdev_tx_queue_len_show),
        netdev_attr_file(ifindex, "speed", 0o444, netdev_speed_show),
        netdev_attr_file(ifindex, "duplex", 0o444, netdev_duplex_show),
        netdev_attr_file(ifindex, "dev_id", 0o444, netdev_zero_show),
        netdev_attr_file(ifindex, "dev_port", 0o444, netdev_zero_show),
        netdev_attr_file(ifindex, "dormant", 0o444, netdev_zero_show),
        netdev_attr_file(ifindex, "netdev_group", 0o444, netdev_zero_show),
    ] {
        add_child(&dir, file);
    }
    add_child(
        &dir,
        KernfsNode::new_symlink("subsystem", "../../../class/net"),
    );
    add_child(&dir, netdev_uevent_file(ifindex));
    if let Some(mtu_node) = lookup(&dir, "mtu") {
        mtu_node.priv_ptr.store(ifindex as u64, Ordering::Release);
    }
    let _ = mtu;
    dir
}

pub fn attach_roots(class_root: &Arc<KernfsNode>, devices_root: &Arc<KernfsNode>) {
    let net_class = lookup(class_root, "net").unwrap_or_else(|| {
        let dir = KernfsNode::new_dir("net", 0o555);
        add_child(class_root, dir.clone());
        dir
    });
    let virtual_root = lookup(devices_root, "virtual").unwrap_or_else(|| {
        let dir = KernfsNode::new_dir("virtual", 0o555);
        add_child(devices_root, dir.clone());
        dir
    });
    let virtual_net = lookup(&virtual_root, "net").unwrap_or_else(|| {
        let dir = KernfsNode::new_dir("net", 0o555);
        add_child(&virtual_root, dir.clone());
        dir
    });
    *NET_CLASS_ROOT.lock() = Some(net_class);
    *NET_DEVICES_ROOT.lock() = Some(virtual_net);
    for dev in list_netdevices() {
        register_netdevice(&dev);
    }
}

pub fn register_netdevice(dev: &NetDeviceRef) {
    let class_root = NET_CLASS_ROOT.lock().clone();
    let devices_root = NET_DEVICES_ROOT.lock().clone();
    let Some(class_root) = class_root else {
        return;
    };
    let Some(devices_root) = devices_root else {
        return;
    };

    devices_root.children.lock().remove(&dev.name);
    add_child(&devices_root, build_virtual_net_dir(dev));

    class_root.children.lock().remove(&dev.name);
    add_child(
        &class_root,
        KernfsNode::new_symlink(
            &dev.name,
            &format!("../../devices/virtual/net/{}", dev.name),
        ),
    );
}

pub fn unregister_netdevice(name: &str) {
    if let Some(class_root) = NET_CLASS_ROOT.lock().clone() {
        class_root.children.lock().remove(name);
    }
    if let Some(devices_root) = NET_DEVICES_ROOT.lock().clone() {
        devices_root.children.lock().remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::KernfsKind;
    use crate::net::device::{DUMMY_NETDEV_OPS, register_netdevice as register_device};

    #[test]
    fn netdev_uevent_reports_interface_and_ifindex() {
        let name = "sysfs-net-uevent0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev =
            register_device(name, 1500, [2, 0, 0, 0, 0, 9], &DUMMY_NETDEV_OPS).expect("device");
        let node = netdev_uevent_file(dev.ifindex);
        let KernfsKind::File {
            show: Some(show), ..
        } = &node.kind
        else {
            panic!("uevent node must be a readable file");
        };
        let mut buf = [0u8; 128];
        let n = show(&node, &mut buf).expect("uevent read");
        let text = core::str::from_utf8(&buf[..n]).expect("utf8");
        assert_eq!(
            text,
            format!("INTERFACE={}\nIFINDEX={}\n", dev.name, dev.ifindex)
        );
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn netdev_uevent_store_accepts_add_and_broadcasts_linux_shaped_event() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        let name = "sysfs-net-uevent1";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev =
            register_device(name, 1500, [2, 0, 0, 0, 0, 10], &DUMMY_NETDEV_OPS).expect("device");
        let _ = crate::net::uevent::drain_pending();
        let node = netdev_uevent_file(dev.ifindex);
        let KernfsKind::File {
            store: Some(store), ..
        } = &node.kind
        else {
            panic!("uevent node must be writable");
        };

        assert_eq!(store(&node, b"add\n"), Ok(4));

        let drained = crate::net::uevent::drain_pending();
        assert!(!drained.is_empty());
        assert!(
            drained
                .last()
                .expect("synthetic netdev add uevent")
                .payload
                .starts_with(b"add@/devices/virtual/net/sysfs-net-uevent1\0")
        );
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn netdev_uevent_store_accepts_linux_synthetic_action_args() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        let name = "sysfs-net-uevent2";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev =
            register_device(name, 1500, [2, 0, 0, 0, 0, 11], &DUMMY_NETDEV_OPS).expect("device");
        let _ = crate::net::uevent::drain_pending();
        let node = netdev_uevent_file(dev.ifindex);
        let KernfsKind::File {
            store: Some(store), ..
        } = &node.kind
        else {
            panic!("uevent node must be writable");
        };

        let request = b"add 3d0f6f14-6ef3-4e67-8dd3-36f3f9f7d700 IFINDEX=2\n";
        assert_eq!(store(&node, request), Ok(request.len()));

        let drained = crate::net::uevent::drain_pending();
        assert!(!drained.is_empty());
        assert!(
            drained
                .last()
                .expect("synthetic netdev add uevent with args")
                .payload
                .starts_with(b"add@/devices/virtual/net/sysfs-net-uevent2\0")
        );
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }
}
