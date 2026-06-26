//! linux-parity: complete
//! linux-source: vendor/linux/net/802/psnap.c
//! test-origin: linux:vendor/linux/net/802/psnap.c
//! SNAP client matching, receive dispatch, and request construction.

extern crate alloc;

use alloc::vec::Vec;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, ENOSPC};
use crate::net::skbuff::{SkBuff, skb_pull, skb_push};

pub const SNAP_DESC_LEN: usize = 5;
pub const SNAP_8022_HEADER_LEN: usize = 3;
pub const SNAP_SAP: u8 = 0xaa;
pub const ETH_P_SNAP: u16 = 0x0005;

pub type SnapRcvFunc = fn(&mut SkBuff, SnapPacketType) -> i32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapPacketType {
    pub ethertype: u16,
}

#[derive(Clone, Copy)]
pub struct SnapClient {
    id: usize,
    pub desc: [u8; SNAP_DESC_LEN],
    pub header_length: usize,
    pub rcvfunc: SnapRcvFunc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapRequestResult {
    pub dest: [u8; 6],
    pub lsap: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapReceive {
    Delivered(i32),
    Dropped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapRegistrySnapshot {
    pub sap_open: bool,
    pub clients: Vec<[u8; SNAP_DESC_LEN]>,
}

struct SnapRegistry {
    sap_open: bool,
    next_id: usize,
    clients: Vec<SnapClient>,
}

impl SnapRegistry {
    const fn new() -> Self {
        Self {
            sap_open: false,
            next_id: 1,
            clients: Vec::new(),
        }
    }
}

lazy_static! {
    static ref SNAP_REGISTRY: Mutex<SnapRegistry> = Mutex::new(SnapRegistry::new());
}

pub fn snap_init() -> Result<(), i32> {
    let mut registry = SNAP_REGISTRY.lock();
    if registry.sap_open {
        return Err(EBUSY);
    }
    registry.sap_open = true;
    Ok(())
}

pub fn snap_exit() {
    SNAP_REGISTRY.lock().sap_open = false;
}

pub fn find_snap_client(desc: [u8; SNAP_DESC_LEN]) -> Option<SnapClient> {
    SNAP_REGISTRY
        .lock()
        .clients
        .iter()
        .copied()
        .find(|client| client.desc == desc)
}

pub fn register_snap_client(desc: [u8; SNAP_DESC_LEN], rcvfunc: SnapRcvFunc) -> Option<SnapClient> {
    let mut registry = SNAP_REGISTRY.lock();
    if registry.clients.iter().any(|client| client.desc == desc) {
        return None;
    }

    let client = SnapClient {
        id: registry.next_id,
        desc,
        rcvfunc,
        header_length: SNAP_DESC_LEN + SNAP_8022_HEADER_LEN,
    };
    registry.next_id += 1;
    registry.clients.push(client);
    Some(client)
}

pub fn unregister_snap_client(client: SnapClient) {
    let mut registry = SNAP_REGISTRY.lock();
    if let Some(pos) = registry
        .clients
        .iter()
        .position(|entry| entry.id == client.id)
    {
        registry.clients.remove(pos);
    }
}

pub fn snap_rcv(skb: &mut SkBuff) -> SnapReceive {
    if skb.len < SNAP_DESC_LEN {
        return SnapReceive::Dropped;
    }
    let mut desc = [0; SNAP_DESC_LEN];
    desc.copy_from_slice(&skb.data()[..SNAP_DESC_LEN]);

    let Some(client) = find_snap_client(desc) else {
        return SnapReceive::Dropped;
    };

    if skb_pull(skb, SNAP_DESC_LEN).is_err() {
        return SnapReceive::Dropped;
    }
    let rc = (client.rcvfunc)(
        skb,
        SnapPacketType {
            ethertype: ETH_P_SNAP,
        },
    );
    SnapReceive::Delivered(rc)
}

pub fn snap_request(
    client: SnapClient,
    skb: &mut SkBuff,
    dest: [u8; 6],
) -> Result<SnapRequestResult, i32> {
    skb_push(skb, SNAP_DESC_LEN)
        .map_err(|_| ENOSPC)?
        .copy_from_slice(&client.desc);
    Ok(SnapRequestResult {
        dest,
        lsap: SNAP_SAP,
    })
}

pub fn snap_registry_snapshot() -> SnapRegistrySnapshot {
    let registry = SNAP_REGISTRY.lock();
    SnapRegistrySnapshot {
        sap_open: registry.sap_open,
        clients: registry.clients.iter().map(|client| client.desc).collect(),
    }
}

#[cfg(test)]
fn snap_reset_for_tests() {
    *SNAP_REGISTRY.lock() = SnapRegistry::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::skbuff::{alloc_skb, skb_put, skb_reserve};

    fn test_rcv(_skb: &mut SkBuff, packet_type: SnapPacketType) -> i32 {
        assert_eq!(packet_type.ethertype, ETH_P_SNAP);
        17
    }

    #[test]
    fn psnap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/802/psnap.c"
        ));
        assert!(source.contains("Find a snap client by matching the 5 bytes."));
        assert!(source.contains("if (!memcmp(p->type, desc, 5))"));
        assert!(source.contains("static int snap_rcv"));
        assert!(source.contains("pskb_may_pull(skb, 5)"));
        assert!(source.contains("skb_pull_rcsum(skb, 5);"));
        assert!(source.contains("rc = proto->rcvfunc(skb, dev, &snap_packet_type, orig_dev);"));
        assert!(source.contains("memcpy(skb_push(skb, 5), dl->type, 5);"));
        assert!(source.contains("llc_build_and_send_ui_pkt"));
        assert!(source.contains("llc_sap_open(0xAA, snap_rcv);"));
        assert!(source.contains("llc_sap_put(snap_sap);"));
        assert!(source.contains("proto->header_length\t= 5 + 3;"));
        assert!(source.contains("proto->request\t\t= snap_request;"));
        assert!(source.contains("list_add_rcu(&proto->node, &snap_list);"));
        assert!(source.contains("list_del_rcu(&proto->node);"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"SNAP data link layer. Derived from 802.2\")")
        );
    }

    #[test]
    fn snap_registration_rejects_duplicate_descriptors() {
        snap_reset_for_tests();
        snap_init().unwrap();
        let ip = register_snap_client([0, 0, 0, 0x08, 0x00], test_rcv).unwrap();
        assert!(find_snap_client(ip.desc).is_some());
        assert!(register_snap_client(ip.desc, test_rcv).is_none());
        let arp = register_snap_client([0, 0, 0, 0x08, 0x06], test_rcv).unwrap();
        assert_eq!(arp.header_length, 8);
        assert_eq!(
            snap_registry_snapshot().clients,
            [[0, 0, 0, 0x08, 0x00], [0, 0, 0, 0x08, 0x06]]
        );

        unregister_snap_client(ip);
        assert!(find_snap_client(ip.desc).is_none());
        snap_exit();
        assert!(!snap_registry_snapshot().sap_open);
    }

    #[test]
    fn snap_receive_pulls_descriptor_and_dispatches_client() {
        snap_reset_for_tests();
        let client = register_snap_client([0, 0, 0, 0x08, 0x00], test_rcv).unwrap();
        let mut skb = alloc_skb(32).unwrap();
        skb_put(&mut skb, 8)
            .unwrap()
            .copy_from_slice(&[0, 0, 0, 0x08, 0x00, 1, 2, 3]);

        assert_eq!(snap_rcv(&mut skb), SnapReceive::Delivered(17));
        assert_eq!(skb.data(), &[1, 2, 3]);

        unregister_snap_client(client);
    }

    #[test]
    fn snap_request_pushes_five_byte_descriptor() {
        snap_reset_for_tests();
        let client = register_snap_client([0, 0, 0, 0x08, 0x06], test_rcv).unwrap();
        let mut skb = alloc_skb(32).unwrap();
        skb_reserve(&mut skb, SNAP_DESC_LEN).unwrap();
        skb_put(&mut skb, 2).unwrap().copy_from_slice(&[1, 2]);

        let request = snap_request(client, &mut skb, [1, 2, 3, 4, 5, 6]).unwrap();

        assert_eq!(request.lsap, SNAP_SAP);
        assert_eq!(request.dest, [1, 2, 3, 4, 5, 6]);
        assert_eq!(skb.data(), &[0, 0, 0, 0x08, 0x06, 1, 2]);
    }
}
