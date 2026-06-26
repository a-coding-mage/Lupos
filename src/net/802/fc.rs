//! linux-parity: complete
//! linux-source: vendor/linux/net/802/fc.c
//! test-origin: linux:vendor/linux/net/802/fc.c
//! Fibre Channel generic netdevice defaults and header construction.

use crate::include::uapi::errno::ENOSPC;
use crate::net::device::IFF_BROADCAST;
use crate::net::skbuff::{SkBuff, skb_push};

pub const FC_ALEN: usize = 6;
pub const FCH_HDR_LEN: usize = FC_ALEN * 2;
pub const FCLLC_LEN: usize = 8;
pub const FC_HLEN: usize = FCH_HDR_LEN + FCLLC_LEN;
pub const EXTENDED_SAP: u8 = 0xaa;
pub const UI_CMD: u8 = 0x03;
pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_ARP: u16 = 0x0806;
pub const ARPHRD_IEEE802: u16 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FcDeviceDefaults {
    pub dev_type: u16,
    pub hard_header_len: usize,
    pub mtu: u32,
    pub addr_len: usize,
    pub tx_queue_len: u32,
    pub flags: u32,
    pub broadcast: [u8; FC_ALEN],
}

pub const FC_DEVICE_DEFAULTS: FcDeviceDefaults = FcDeviceDefaults {
    dev_type: ARPHRD_IEEE802,
    hard_header_len: FC_HLEN,
    mtu: 2024,
    addr_len: FC_ALEN,
    tx_queue_len: 100,
    flags: IFF_BROADCAST,
    broadcast: [0xff; FC_ALEN],
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FcNetDevice {
    pub dev_addr: [u8; FC_ALEN],
    pub defaults: FcDeviceDefaults,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FcHeader {
    pub daddr: [u8; FC_ALEN],
    pub saddr: [u8; FC_ALEN],
    pub snap: Option<[u8; FCLLC_LEN]>,
}

pub const fn fc_uses_snap(ethertype: u16) -> bool {
    ethertype == ETH_P_IP || ethertype == ETH_P_ARP
}

pub const fn fc_header_len(ethertype: u16, has_destination: bool) -> isize {
    let len = if fc_uses_snap(ethertype) {
        FC_HLEN
    } else {
        FCH_HDR_LEN
    } as isize;
    if has_destination { len } else { -len }
}

pub const fn fc_snap_header(ethertype: u16) -> Option<[u8; FCLLC_LEN]> {
    if !fc_uses_snap(ethertype) {
        return None;
    }
    let be = ethertype.to_be_bytes();
    Some([EXTENDED_SAP, EXTENDED_SAP, UI_CMD, 0, 0, 0, be[0], be[1]])
}

pub fn fc_header(
    skb: &mut SkBuff,
    dev: &FcNetDevice,
    ethertype: u16,
    daddr: Option<[u8; FC_ALEN]>,
    saddr: Option<[u8; FC_ALEN]>,
    _len: usize,
) -> Result<isize, i32> {
    let snap = fc_snap_header(ethertype);
    let hdr_len = if snap.is_some() { FC_HLEN } else { FCH_HDR_LEN };
    let header = skb_push(skb, hdr_len).map_err(|_| ENOSPC)?;
    let source = saddr.unwrap_or(dev.dev_addr);
    header[..FC_ALEN].copy_from_slice(&daddr.unwrap_or([0; FC_ALEN]));
    header[FC_ALEN..FCH_HDR_LEN].copy_from_slice(&source);
    if let Some(snap) = snap {
        header[FCH_HDR_LEN..FC_HLEN].copy_from_slice(&snap);
    }

    Ok(fc_header_len(ethertype, daddr.is_some()))
}

pub const fn fc_setup(dev_addr: [u8; FC_ALEN]) -> FcNetDevice {
    FcNetDevice {
        dev_addr,
        defaults: FC_DEVICE_DEFAULTS,
    }
}

pub const fn alloc_fcdev(_sizeof_priv: usize, dev_addr: [u8; FC_ALEN]) -> FcNetDevice {
    fc_setup(dev_addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::skbuff::{alloc_skb, skb_put, skb_reserve};

    #[test]
    fn fc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/802/fc.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/if_fc.h"
        ));
        assert!(source.contains("if (type == ETH_P_IP || type == ETH_P_ARP)"));
        assert!(source.contains("fcllc->dsap = fcllc->ssap = EXTENDED_SAP;"));
        assert!(source.contains("fcllc->llc = UI_CMD;"));
        assert!(source.contains("if(saddr)"));
        assert!(source.contains("memcpy(fch->saddr,saddr,dev->addr_len);"));
        assert!(source.contains("if(daddr)"));
        assert!(source.contains("return -hdr_len;"));
        assert!(source.contains("dev->type\t\t= ARPHRD_IEEE802;"));
        assert!(source.contains("dev->mtu\t\t= 2024;"));
        assert!(source.contains("dev->tx_queue_len\t= 100;"));
        assert!(source.contains("dev->flags\t\t= IFF_BROADCAST;"));
        assert!(source.contains("memset(dev->broadcast, 0xFF, FC_ALEN);"));
        assert!(source.contains("return alloc_netdev(sizeof_priv, \"fc%d\""));
        assert!(header.contains("#define FC_ALEN\t6"));
        assert!(header.contains("#define EXTENDED_SAP 0xAA"));
    }

    #[test]
    fn fc_header_shape_follows_linux() {
        assert_eq!(FC_DEVICE_DEFAULTS.addr_len, FC_ALEN);
        assert_eq!(FC_DEVICE_DEFAULTS.broadcast, [0xff; FC_ALEN]);
        assert_eq!(FC_DEVICE_DEFAULTS.flags, IFF_BROADCAST);
        assert_eq!(fc_header_len(ETH_P_IP, true), FC_HLEN as isize);
        assert_eq!(fc_header_len(0x86dd, true), FCH_HDR_LEN as isize);
        assert_eq!(fc_header_len(ETH_P_ARP, false), -(FC_HLEN as isize));
        assert_eq!(
            fc_snap_header(ETH_P_IP),
            Some([0xaa, 0xaa, 0x03, 0, 0, 0, 0x08, 0x00])
        );
    }

    #[test]
    fn fc_header_pushes_addresses_and_snap_like_linux() {
        let mut skb = alloc_skb(64).unwrap();
        skb_reserve(&mut skb, FC_HLEN).unwrap();
        skb_put(&mut skb, 3).unwrap().copy_from_slice(&[1, 2, 3]);
        let dev = fc_setup([0x10, 0x11, 0x12, 0x13, 0x14, 0x15]);

        let len = fc_header(&mut skb, &dev, ETH_P_IP, Some([0, 1, 2, 3, 4, 5]), None, 3).unwrap();

        assert_eq!(len, FC_HLEN as isize);
        assert_eq!(
            &skb.data()[..FC_HLEN],
            &[
                0, 1, 2, 3, 4, 5, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0xaa, 0xaa, 0x03, 0, 0, 0,
                0x08, 0x00,
            ]
        );
    }

    #[test]
    fn fc_header_returns_negative_length_without_destination() {
        let mut skb = alloc_skb(64).unwrap();
        skb_reserve(&mut skb, FC_HLEN).unwrap();
        let dev = alloc_fcdev(16, [1, 2, 3, 4, 5, 6]);

        let len = fc_header(&mut skb, &dev, ETH_P_ARP, None, Some([6, 5, 4, 3, 2, 1]), 0).unwrap();

        assert_eq!(len, -(FC_HLEN as isize));
        assert_eq!(&skb.data()[FC_ALEN..FCH_HDR_LEN], &[6, 5, 4, 3, 2, 1]);
    }
}
