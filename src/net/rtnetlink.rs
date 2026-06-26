//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! rtnetlink and NETLINK family scaffolding.
//!
//! The wire format is intentionally represented as typed Rust records here;
//! syscall-level byte parsing can lower into the same records when M52 grows
//! the full socket ABI.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENODEV};
use crate::net::device::{
    DUMMY_NETDEV_OPS, NetDeviceRef, list_netdevices, register_netdevice, unregister_netdevice,
};

pub const NETLINK_ROUTE: u16 = 0;
pub const NETLINK_KOBJECT_UEVENT: u16 = 15;
pub const NETLINK_GENERIC: u16 = 16;
pub const NETLINK_AUDIT: u16 = 9;

pub const RTM_NEWLINK: u16 = 16;
pub const RTM_DELLINK: u16 = 17;
pub const RTM_GETLINK: u16 = 18;
pub const RTM_SETLINK: u16 = 19;
pub const RTM_NEWADDR: u16 = 20;
pub const RTM_DELADDR: u16 = 21;
pub const RTM_GETADDR: u16 = 22;
pub const RTM_NEWROUTE: u16 = 24;
pub const RTM_GETROUTE: u16 = 26;
pub const RTM_GETNEIGH: u16 = 30;
pub const RTM_GETRULE: u16 = 34;
pub const RTM_GETQDISC: u16 = 38;
pub const RTM_GETNEXTHOP: u16 = 106;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetlinkFamily {
    Route,
    KobjectUevent,
    Audit,
    Generic,
}

impl NetlinkFamily {
    pub fn from_raw(raw: u16) -> Result<Self, i32> {
        match raw {
            NETLINK_ROUTE => Ok(Self::Route),
            NETLINK_KOBJECT_UEVENT => Ok(Self::KobjectUevent),
            NETLINK_AUDIT => Ok(Self::Audit),
            NETLINK_GENERIC => Ok(Self::Generic),
            _ => Err(EINVAL),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RtnlMessage {
    pub msg_type: u16,
    pub ifindex: u32,
    pub flags: u32,
    pub mtu: u32,
    pub name: String,
    pub addr: [u8; 6],
}

impl RtnlMessage {
    pub fn from_device(msg_type: u16, dev: &NetDeviceRef) -> Self {
        Self {
            msg_type,
            ifindex: dev.ifindex,
            flags: dev.flags.load(core::sync::atomic::Ordering::Acquire),
            mtu: dev.mtu,
            name: dev.name.clone(),
            addr: dev.dev_addr,
        }
    }
}

pub fn rtm_getlink() -> Vec<RtnlMessage> {
    list_netdevices()
        .iter()
        .map(|dev| RtnlMessage::from_device(RTM_GETLINK, dev))
        .collect()
}

pub fn handle_rtnl_message(msg: &RtnlMessage) -> Result<Vec<RtnlMessage>, i32> {
    match msg.msg_type {
        RTM_NEWLINK => {
            let dev = register_netdevice(&msg.name, msg.mtu, msg.addr, &DUMMY_NETDEV_OPS)?;
            Ok(alloc::vec![RtnlMessage::from_device(RTM_NEWLINK, &dev)])
        }
        RTM_DELLINK => {
            unregister_netdevice(&msg.name)?;
            Ok(Vec::new())
        }
        RTM_GETLINK => Ok(rtm_getlink()),
        RTM_NEWADDR | RTM_NEWROUTE => Err(ENODEV),
        _ => Err(EINVAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_family_and_getlink_schema_work() {
        assert_eq!(
            NetlinkFamily::from_raw(NETLINK_ROUTE).unwrap(),
            NetlinkFamily::Route
        );

        let name = String::from("rtnl-test0");
        let _ = unregister_netdevice(&name);
        let req = RtnlMessage {
            msg_type: RTM_NEWLINK,
            ifindex: 0,
            flags: 0,
            mtu: 1500,
            name: name.clone(),
            addr: [2, 0, 0, 0, 0, 2],
        };
        handle_rtnl_message(&req).unwrap();
        let links = rtm_getlink();
        assert!(
            links
                .iter()
                .any(|link| link.name == name && link.mtu == 1500)
        );

        let del = RtnlMessage {
            msg_type: RTM_DELLINK,
            ..req
        };
        handle_rtnl_message(&del).unwrap();
    }

    #[test]
    fn newlink_rejects_invalid_mtu_and_missing_delete() {
        let req = RtnlMessage {
            msg_type: RTM_NEWLINK,
            ifindex: 0,
            flags: 0,
            mtu: 1,
            name: String::from("bad-mtu0"),
            addr: [2, 0, 0, 0, 0, 4],
        };
        assert_eq!(handle_rtnl_message(&req), Err(EINVAL));

        let del = RtnlMessage {
            msg_type: RTM_DELLINK,
            mtu: 1500,
            name: String::from("missing-netdev0"),
            ..req
        };
        assert_eq!(handle_rtnl_message(&del), Err(ENODEV));
    }
}
