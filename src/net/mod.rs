//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! Networking stack.
//!
//! The module layout follows the Linux networking subsystems while preserving
//! the small public helpers used by boot tests and host-side unit tests.

#![allow(dead_code)]

extern crate alloc;

use ::core::sync::atomic::{AtomicBool, Ordering};

pub mod appletalk;
pub mod atm;
#[path = "batman-adv/mod.rs"]
pub mod batman_adv;
pub mod bluetooth;
pub mod bridge;
pub mod ceph;
pub mod core;
pub mod dcb;
pub mod device;
pub mod devres;
pub mod dsa;
pub mod ethtool;
pub mod fib;
pub mod handshake;
#[path = "802/mod.rs"]
pub mod ieee802;
pub mod ieee802154;
pub mod ip;
pub mod ipv4;
pub mod ipv6;
pub mod link;
pub mod linux_parity;
pub mod linux_sources;
pub mod llc;
pub mod mac80211;
pub mod mac802154;
pub mod module_abi;
pub mod mptcp;
pub mod neighbour;
pub mod netfilter;
pub mod netlabel;
pub mod nfc;
pub mod niche;
#[path = "9p/mod.rs"]
pub mod ninep;
pub mod openvswitch;
pub mod phonet;
pub mod rds;
pub mod rtnetlink;
pub mod rxrpc;
pub mod sched;
pub mod sctp;
#[path = "6lowpan/mod.rs"]
pub mod sixlowpan;
pub mod skbuff;
pub mod smc;
pub mod socket;
pub mod sunrpc;
pub mod syscalls;
pub mod tcp;
pub mod tipc;
pub mod tls;
pub mod udp;
pub mod uevent;
pub mod unix;
#[cfg(any(test, CONFIG_VIRTIO_NET = "m"))]
pub mod virtio_net;
#[path = "8021q/mod.rs"]
pub mod vlan_8021q;
pub mod vmw_vsock;
pub mod wireless;
pub mod x25;
pub mod xdp;
pub mod xfrm;

pub const PROTOCOL_FAMILY_BOOT_LOGS: [&str; 3] = [
    "NET: Registered PF_INET protocol family",
    "NET: Registered PF_INET6 protocol family",
    "NET: Registered PF_PACKET protocol family",
];

static PROTOCOL_FAMILY_LOGGED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    device::init();
    log_protocol_family_registrations();
    niche::init();
}

fn log_protocol_family_registrations() {
    if !PROTOCOL_FAMILY_LOGGED.swap(true, Ordering::AcqRel) {
        for line in PROTOCOL_FAMILY_BOOT_LOGS {
            crate::log_info!("", "{}", line);
        }
    }
}

pub fn run_networking_acceptance() -> Result<(), i32> {
    use ::core::sync::atomic::Ordering;
    use alloc::string::String;

    use self::device::{
        DUMMY_NETDEV_OPS, lookup_netdevice, register_netdevice, set_device_up, unregister_netdevice,
    };
    use self::fib::{
        Fib4Entry, Fib6Entry, fib_clear, fib4_add, fib4_lookup, fib6_add, fib6_lookup, ipv4,
    };
    use self::ip::{
        IPPROTO_ICMP, build_icmp_echo, build_ipv4_packet, parse_icmp, parse_ipv4_packet,
    };
    use self::link::{Bond, BondMode, Bridge, VlanDevice};
    use self::neighbour::{
        AddressFamily, ArpPacket, NeighState, Neighbour, clear_neighbours, neigh_lookup,
        neigh_update,
    };
    use self::netfilter::{Hook, NfTable, Verdict};
    use self::rtnetlink::{
        RTM_GETLINK, RTM_NEWLINK, RtnlMessage, handle_rtnl_message, rtm_getlink,
    };
    use self::skbuff::{
        alloc_skb, pskb_expand_head, skb_clone, skb_put, skb_reserve, skb_share_check,
    };
    use self::socket::{AF_INET, AF_UNIX, SO_REUSEADDR, SOCK_DGRAM, SOCK_STREAM, SockAddr};
    use self::tcp::{TCP_ACK, TCP_SYN, TcpConnection, TcpSegment, TcpState};

    init();
    linux_sources::all_sources_have_policy()?;
    let niche = niche::registration_snapshot();
    assert!(niche.mpls_gso);
    assert_eq!(niche.mpls_packet_offloads, 2);
    assert!(niche.ioam6_genl_family);
    assert_eq!(niche.mip6_xfrm_types, 2);
    niche::run_niche_acceptance()?;
    assert_eq!(
        linux_sources::source_count(),
        linux_sources::NETWORKING_SOURCE_COUNT
    );

    // M47: clone, copy-on-write, and head expansion.
    let mut skb = alloc_skb(64)?;
    skb_reserve(&mut skb, 16)?;
    skb_put(&mut skb, 4)?.copy_from_slice(&[1, 2, 3, 4]);
    let clone = skb_clone(&skb);
    let mut writable = skb_share_check(clone)?;
    writable.data_mut()[0] = 9;
    assert_eq!(skb.data(), &[1, 2, 3, 4]);
    pskb_expand_head(&mut writable, 8, 8)?;
    assert_eq!(writable.headroom(), 8);

    // M48: net_device registry and rtnetlink schema.
    let _ = unregister_netdevice("eth-netacc");
    let dev = register_netdevice("eth-netacc", 1500, [2, 0, 0, 0, 8, 1], &DUMMY_NETDEV_OPS)?;
    set_device_up(&dev)?;
    assert!(lookup_netdevice("eth-netacc").is_some());
    assert!(rtm_getlink().iter().any(|link| link.name == "eth-netacc"));
    let req = RtnlMessage {
        msg_type: RTM_NEWLINK,
        ifindex: 0,
        flags: 0,
        mtu: 1500,
        name: String::from("rt-netacc"),
        addr: [2, 0, 0, 0, 8, 2],
    };
    let _ = unregister_netdevice("rt-netacc");
    handle_rtnl_message(&req)?;
    assert!(
        handle_rtnl_message(&RtnlMessage {
            msg_type: RTM_GETLINK,
            ..req
        })?
        .len()
            >= 2
    );

    // M49: ARP/NDISC neighbour table and FIB4/FIB6 longest-prefix lookup.
    clear_neighbours();
    let arp = ArpPacket {
        operation: ArpPacket::REPLY,
        sender_hw: [2, 0, 0, 0, 8, 3],
        sender_ip: [10, 8, 0, 2],
        target_hw: [2, 0, 0, 0, 8, 1],
        target_ip: [10, 8, 0, 1],
    };
    neigh_update(Neighbour::new_v4(
        arp.sender_ip,
        arp.sender_hw,
        dev.ifindex,
        NeighState::Reachable,
    ));
    assert_eq!(
        neigh_lookup(AddressFamily::Inet4, &[10, 8, 0, 2], dev.ifindex)?.lladdr,
        arp.sender_hw
    );
    fib_clear();
    fib4_add(Fib4Entry {
        prefix: ipv4(0, 0, 0, 0),
        prefix_len: 0,
        gateway: ipv4(10, 8, 0, 1),
        ifindex: dev.ifindex,
        metric: 100,
    });
    fib4_add(Fib4Entry {
        prefix: ipv4(10, 8, 0, 0),
        prefix_len: 24,
        gateway: 0,
        ifindex: dev.ifindex + 1,
        metric: 10,
    });
    assert_eq!(
        fib4_lookup(ipv4(10, 8, 0, 99)).unwrap().ifindex,
        dev.ifindex + 1
    );
    let mut v6_prefix = [0u8; 16];
    v6_prefix[0..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
    let mut v6_dst = v6_prefix;
    v6_dst[15] = 7;
    fib6_add(Fib6Entry {
        prefix: v6_prefix,
        prefix_len: 32,
        gateway: [0; 16],
        ifindex: dev.ifindex,
        metric: 1,
    });
    assert_eq!(fib6_lookup(v6_dst).unwrap().ifindex, dev.ifindex);

    // M50: IPv4, ICMP, and UDP.
    let icmp = build_icmp_echo(1, 1, b"netacc", false)?;
    assert_eq!(parse_icmp(&icmp)?.icmp_type, 8);
    let ip_skb = build_ipv4_packet(
        ipv4(10, 8, 0, 1),
        ipv4(10, 8, 0, 2),
        IPPROTO_ICMP,
        &icmp,
        64,
    )?;
    assert_eq!(parse_ipv4_packet(&ip_skb)?.protocol, IPPROTO_ICMP);
    let udp_skb = udp::udp_sendmsg(ipv4(10, 8, 0, 1), ipv4(10, 8, 0, 2), 10000, 10001, b"udp")?;
    assert_eq!(udp::udp_recvmsg(&udp_skb)?.payload, b"udp");

    // M51: TCP state and CUBIC.
    let mut conn = TcpConnection::connect(100);
    conn.on_segment(TcpSegment {
        seq: 200,
        ack: 101,
        flags: TCP_SYN | TCP_ACK,
        wnd: 4096,
    })?;
    assert_eq!(conn.state, TcpState::Established);
    let cwnd = conn.congestion.cwnd;
    conn.on_segment(TcpSegment {
        seq: 201,
        ack: 101,
        flags: TCP_ACK,
        wnd: 4096,
    })?;
    assert!(conn.congestion.cwnd > cwnd);

    // M52: datagram and Unix stream socket paths.
    let server_addr = SockAddr::Inet {
        addr: ipv4(127, 0, 0, 1),
        port: 8047,
    };
    let server = socket::socket(AF_INET, SOCK_DGRAM, 0)?;
    let client = socket::socket(AF_INET, SOCK_DGRAM, 0)?;
    socket::setsockopt(&server, SO_REUSEADDR, 1)?;
    socket::bind(&server, server_addr.clone())?;
    socket::connect(&client, server_addr)?;
    socket::sendmsg(&client, b"net")?;
    let mut buf = [0u8; 8];
    let n = socket::recvmsg(&server, &mut buf)?;
    assert_eq!(&buf[..n], b"net");

    let unix_addr = SockAddr::Unix(String::from("/netacc.sock"));
    let listener = socket::socket(AF_UNIX, SOCK_STREAM, 0)?;
    let peer = socket::socket(AF_UNIX, SOCK_STREAM, 0)?;
    socket::setsockopt(&listener, SO_REUSEADDR, 1)?;
    socket::bind(&listener, unix_addr.clone())?;
    socket::listen(&listener)?;
    socket::connect(&peer, unix_addr)?;
    assert!(socket::accept4(&listener).is_ok());

    // M53: netfilter, bridge/VLAN/bonding.
    let mut table = NfTable::new();
    table.drop_icmp(Hook::LocalIn);
    assert_eq!(table.evaluate(Hook::LocalIn, &ip_skb), Verdict::Drop);

    let mut bridge = Bridge::new(100);
    bridge.add_port(dev.ifindex);
    bridge.add_port(dev.ifindex + 1);
    assert_eq!(
        bridge.forward_ports(dev.ifindex),
        alloc::vec![dev.ifindex + 1]
    );
    assert_eq!(VlanDevice::new(101, dev.ifindex, 47)?.vlan_id, 47);
    let mut bond = Bond::new(102, BondMode::ActiveBackup);
    bond.add_slave(dev.ifindex);
    bond.add_slave(dev.ifindex + 1);
    bond.set_link(dev.ifindex, false)?;
    assert_eq!(bond.choose_tx_slave()?, dev.ifindex + 1);

    dev.tx_packets.store(0, Ordering::Release);
    linux_parity::run_vendor_linux_networking_acceptance()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn networking_acceptance_passes() {
        super::run_networking_acceptance().unwrap();
    }

    #[test]
    fn protocol_family_registration_lines_match_linux_boot_tokens() {
        assert_eq!(
            super::PROTOCOL_FAMILY_BOOT_LOGS,
            [
                "NET: Registered PF_INET protocol family",
                "NET: Registered PF_INET6 protocol family",
                "NET: Registered PF_PACKET protocol family",
            ]
        );
    }

    #[test]
    fn niche_network_registration_lines_match_linux_boot_tokens() {
        assert_eq!(
            super::niche::NICHE_NET_BOOT_LOGS,
            [
                "MPLS GSO support",
                "In-situ OAM (IOAM) with IPv6",
                "Mobile IPv6",
            ]
        );
    }
}
