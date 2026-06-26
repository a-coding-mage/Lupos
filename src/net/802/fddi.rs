//! linux-parity: complete
//! linux-source: vendor/linux/net/802/fddi.c
//! test-origin: linux:vendor/linux/net/802/fddi.c
//! FDDI generic netdevice defaults, header sizing, and receive type classification.

pub const FDDI_K_ALEN: usize = 6;
pub const FDDI_K_8022_HLEN: usize = 16;
pub const FDDI_K_SNAP_HLEN: usize = 21;
pub const FDDI_K_SNAP_DLEN: u32 = 4470;
pub const FDDI_FC_K_ASYNC_LLC_DEF: u8 = 0x54;
pub const FDDI_EXTENDED_SAP: u8 = 0xaa;
pub const FDDI_UI_CMD: u8 = 0x03;
pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_ARP: u16 = 0x0806;
pub const ETH_P_IPV6: u16 = 0x86dd;
pub const ETH_P_802_2: u16 = 0x0004;
pub const ARPHRD_FDDI: u16 = 774;
pub const IFF_BROADCAST: u32 = 0x2;
pub const IFF_MULTICAST: u32 = 0x1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PacketType {
    Host,
    Broadcast,
    Multicast,
    OtherHost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FddiDeviceDefaults {
    pub header_ops_create: bool,
    pub dev_type: u16,
    pub hard_header_len: usize,
    pub mtu: u32,
    pub min_mtu: usize,
    pub max_mtu: u32,
    pub addr_len: usize,
    pub tx_queue_len: u32,
    pub flags: u32,
    pub broadcast: [u8; FDDI_K_ALEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FddiHeader {
    pub fc: u8,
    pub dsap: u8,
    pub ssap: u8,
    pub ctrl: u8,
    pub oui: [u8; 3],
    pub ethertype: u16,
    pub saddr: [u8; FDDI_K_ALEN],
    pub daddr: Option<[u8; FDDI_K_ALEN]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FddiHeaderCreate {
    pub header: FddiHeader,
    pub pushed: usize,
    pub return_len: isize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FddiTypeTrans {
    pub ethertype: u16,
    pub pulled: usize,
    pub packet_type: PacketType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FddiAllocNetdev {
    pub sizeof_priv: usize,
    pub name_pattern: &'static str,
    pub name_assign_type: &'static str,
    pub setup: FddiDeviceDefaults,
}

pub const FDDI_DEVICE_DEFAULTS: FddiDeviceDefaults = FddiDeviceDefaults {
    header_ops_create: true,
    dev_type: ARPHRD_FDDI,
    hard_header_len: FDDI_K_SNAP_HLEN + 3,
    mtu: FDDI_K_SNAP_DLEN,
    min_mtu: FDDI_K_SNAP_HLEN,
    max_mtu: FDDI_K_SNAP_DLEN,
    addr_len: FDDI_K_ALEN,
    tx_queue_len: 100,
    flags: IFF_BROADCAST | IFF_MULTICAST,
    broadcast: [0xff; FDDI_K_ALEN],
};

pub const fn fddi_uses_snap(ethertype: u16) -> bool {
    ethertype == ETH_P_IP || ethertype == ETH_P_IPV6 || ethertype == ETH_P_ARP
}

pub const fn fddi_header_len(ethertype: u16, has_destination: bool) -> isize {
    let len = if fddi_uses_snap(ethertype) {
        FDDI_K_SNAP_HLEN
    } else {
        FDDI_K_8022_HLEN - 3
    } as isize;
    if has_destination { len } else { -len }
}

pub const fn fddi_header(
    ethertype: u16,
    daddr: Option<[u8; FDDI_K_ALEN]>,
    saddr: Option<[u8; FDDI_K_ALEN]>,
    dev_addr: [u8; FDDI_K_ALEN],
) -> FddiHeaderCreate {
    let snap = fddi_uses_snap(ethertype);
    let pushed = if snap {
        FDDI_K_SNAP_HLEN
    } else {
        FDDI_K_8022_HLEN - 3
    };
    let source = match saddr {
        Some(addr) => addr,
        None => dev_addr,
    };
    FddiHeaderCreate {
        header: FddiHeader {
            fc: FDDI_FC_K_ASYNC_LLC_DEF,
            dsap: if snap { FDDI_EXTENDED_SAP } else { 0 },
            ssap: if snap { FDDI_EXTENDED_SAP } else { 0 },
            ctrl: if snap { FDDI_UI_CMD } else { 0 },
            oui: [0; 3],
            ethertype: if snap { ethertype } else { 0 },
            saddr: source,
            daddr,
        },
        pushed,
        return_len: if daddr.is_some() {
            pushed as isize
        } else {
            -(pushed as isize)
        },
    }
}

pub fn fddi_type_trans(
    dsap: u8,
    snap_ethertype: u16,
    daddr: [u8; FDDI_K_ALEN],
    dev_addr: [u8; FDDI_K_ALEN],
    broadcast: [u8; FDDI_K_ALEN],
    promisc: bool,
) -> FddiTypeTrans {
    let (ethertype, pulled) = if dsap == 0xe0 {
        (ETH_P_802_2, FDDI_K_8022_HLEN - 3)
    } else {
        (snap_ethertype, FDDI_K_SNAP_HLEN)
    };
    let packet_type = if daddr[0] & 0x01 != 0 {
        if daddr == broadcast {
            PacketType::Broadcast
        } else {
            PacketType::Multicast
        }
    } else if promisc && daddr != dev_addr {
        PacketType::OtherHost
    } else {
        PacketType::Host
    };
    FddiTypeTrans {
        ethertype,
        pulled,
        packet_type,
    }
}

pub const fn fddi_setup() -> FddiDeviceDefaults {
    FDDI_DEVICE_DEFAULTS
}

pub const fn alloc_fddidev(sizeof_priv: usize) -> FddiAllocNetdev {
    FddiAllocNetdev {
        sizeof_priv,
        name_pattern: "fddi%d",
        name_assign_type: "NET_NAME_UNKNOWN",
        setup: fddi_setup(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fddi_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/802/fddi.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/if_fddi.h"
        ));
        assert!(source.contains("if(type != ETH_P_IP && type != ETH_P_IPV6 && type != ETH_P_ARP)"));
        assert!(source.contains("fddi = skb_push(skb, hl);"));
        assert!(source.contains("FDDI_FC_K_ASYNC_LLC_DEF;"));
        assert!(source.contains("fddi->hdr.llc_snap.dsap"));
        assert!(source.contains("fddi->hdr.llc_snap.ethertype"));
        assert!(source.contains("memcpy(fddi->saddr, saddr, dev->addr_len);"));
        assert!(source.contains("memcpy(fddi->saddr, dev->dev_addr, dev->addr_len);"));
        assert!(source.contains("memcpy(fddi->daddr, daddr, dev->addr_len);"));
        assert!(source.contains("return -hl;"));
        assert!(source.contains("skb_pull(skb, FDDI_K_8022_HLEN-3);"));
        assert!(source.contains("skb_pull(skb, FDDI_K_SNAP_HLEN);"));
        assert!(source.contains(".create\t\t= fddi_header"));
        assert!(source.contains("dev->header_ops\t\t= &fddi_header_ops;"));
        assert!(source.contains("dev->type\t\t= ARPHRD_FDDI;"));
        assert!(source.contains("dev->hard_header_len\t= FDDI_K_SNAP_HLEN+3;"));
        assert!(source.contains("dev->flags\t\t= IFF_BROADCAST | IFF_MULTICAST;"));
        assert!(source.contains("alloc_netdev(sizeof_priv, \"fddi%d\", NET_NAME_UNKNOWN"));
        assert!(source.contains("EXPORT_SYMBOL(alloc_fddidev);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Core routines for FDDI network devices\")"));
        assert!(header.contains("#define FDDI_K_SNAP_HLEN\t21"));
        assert!(header.contains("#define FDDI_FC_K_ASYNC_LLC_DEF\t\t0x54"));
    }

    #[test]
    fn fddi_header_and_packet_type_follow_linux() {
        assert_eq!(FDDI_DEVICE_DEFAULTS.dev_type, ARPHRD_FDDI);
        assert!(FDDI_DEVICE_DEFAULTS.header_ops_create);
        assert_eq!(FDDI_DEVICE_DEFAULTS.flags, IFF_BROADCAST | IFF_MULTICAST);
        assert_eq!(fddi_header_len(ETH_P_IPV6, true), FDDI_K_SNAP_HLEN as isize);
        assert_eq!(
            fddi_header_len(0x1234, false),
            -((FDDI_K_8022_HLEN - 3) as isize)
        );
        let dev = [0x02, 0, 0, 0, 0, 1];
        let dest = [0x02, 0, 0, 0, 0, 2];
        let snap = fddi_header(ETH_P_IP, Some(dest), None, dev);
        assert_eq!(snap.pushed, FDDI_K_SNAP_HLEN);
        assert_eq!(snap.return_len, FDDI_K_SNAP_HLEN as isize);
        assert_eq!(snap.header.fc, FDDI_FC_K_ASYNC_LLC_DEF);
        assert_eq!(snap.header.dsap, FDDI_EXTENDED_SAP);
        assert_eq!(snap.header.ssap, FDDI_EXTENDED_SAP);
        assert_eq!(snap.header.ctrl, FDDI_UI_CMD);
        assert_eq!(snap.header.ethertype, ETH_P_IP);
        assert_eq!(snap.header.saddr, dev);
        assert_eq!(snap.header.daddr, Some(dest));

        let explicit_source = [0x02, 0, 0, 0, 0, 3];
        let unresolved = fddi_header(0x1234, None, Some(explicit_source), dev);
        assert_eq!(unresolved.pushed, FDDI_K_8022_HLEN - 3);
        assert_eq!(unresolved.return_len, -((FDDI_K_8022_HLEN - 3) as isize));
        assert_eq!(unresolved.header.dsap, 0);
        assert_eq!(unresolved.header.ethertype, 0);
        assert_eq!(unresolved.header.saddr, explicit_source);

        let dev = [0x02, 0, 0, 0, 0, 1];
        let bcast = [0xff; 6];
        assert_eq!(
            fddi_type_trans(0xe0, ETH_P_IP, bcast, dev, bcast, false),
            FddiTypeTrans {
                ethertype: ETH_P_802_2,
                pulled: FDDI_K_8022_HLEN - 3,
                packet_type: PacketType::Broadcast,
            }
        );
        assert_eq!(
            fddi_type_trans(0xaa, ETH_P_IPV6, [0x01, 0, 0, 0, 0, 1], dev, bcast, false).packet_type,
            PacketType::Multicast
        );
        assert_eq!(
            fddi_type_trans(0xaa, ETH_P_IPV6, [0x02, 0, 0, 0, 0, 2], dev, bcast, true).packet_type,
            PacketType::OtherHost
        );
        let allocated = alloc_fddidev(128);
        assert_eq!(allocated.sizeof_priv, 128);
        assert_eq!(allocated.name_pattern, "fddi%d");
        assert_eq!(allocated.name_assign_type, "NET_NAME_UNKNOWN");
        assert_eq!(allocated.setup, FDDI_DEVICE_DEFAULTS);
    }
}
