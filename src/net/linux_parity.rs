//! linux-parity: partial
//! linux-source: vendor/linux/net
//! Vendor Linux-derived networking acceptance fixtures.
//!
//! These tests intentionally mirror small, deterministic slices of:
//! - `vendor/linux/include/linux/skbuff.h`
//! - `vendor/linux/Documentation/networking/skbuff.rst`
//! - `vendor/linux/tools/testing/selftests/net/socket.c`
//! - `vendor/linux/tools/testing/selftests/net/udpgso.c`
//! - `vendor/linux/tools/testing/selftests/net/fib_tests.sh`
//! - `vendor/linux/tools/testing/selftests/net/rtnetlink.sh`
//! - `vendor/linux/tools/testing/selftests/net/icmp.sh`
//! - `vendor/linux/tools/testing/selftests/net/netfilter/`
//! - `vendor/linux/tools/testing/selftests/net/packetdrill/`

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EAFNOSUPPORT, EINVAL, ENOMEM, EPROTONOSUPPORT};
use crate::net::device::{lookup_netdevice, set_device_up, unregister_netdevice};
use crate::net::fib::{
    Fib4Entry, Fib6Entry, fib_clear, fib4_add, fib4_lookup, fib6_add, fib6_lookup, ipv4,
};
use crate::net::ip::{
    IPPROTO_ICMP, IPPROTO_TCP, IPPROTO_UDP, build_icmp_echo, build_ipv4_packet, checksum,
    parse_icmp, parse_ipv4_packet,
};
use crate::net::link::{Bond, BondMode, Bridge, VlanDevice};
use crate::net::netfilter::{Hook, NfTable, Verdict};
use crate::net::rtnetlink::{
    RTM_DELLINK, RTM_GETLINK, RTM_NEWLINK, RtnlMessage, handle_rtnl_message, rtm_getlink,
};
use crate::net::skbuff::{
    SKB_CB_LEN, alloc_skb, pskb_expand_head, skb_clone, skb_pad, skb_pull, skb_push, skb_put,
    skb_reserve, skb_share_check, skb_trim,
};
use crate::net::socket::{self, AF_INET, AF_MAX, SOCK_DGRAM, SOCK_STREAM};
use crate::net::tcp::{TCP_ACK, TCP_SYN, TcpConnection, TcpSegment, TcpState};
use crate::net::udp::{
    self, CONST_HDRLEN_V4, CONST_HDRLEN_V6, CONST_MAX_SEGS_V4, CONST_MAX_SEGS_V6, CONST_MSS_V4,
    CONST_MSS_V6, ETH_MAX_MTU, IP6_MAX_MTU, UDP_MAX_SEGMENTS, UdpGsoPlan,
};

pub fn run_vendor_linux_networking_acceptance() -> Result<(), i32> {
    accept_skb_doc_and_source_semantics()?;
    accept_socket_selftest_table()?;
    accept_rtnetlink_dummy_bridge_vlan()?;
    accept_fib_tests_fixtures()?;
    accept_icmp_source_fixture()?;
    accept_udp_gso_selftest_table()?;
    accept_tcp_packetdrill_fixture()?;
    accept_netfilter_nft_drop_icmp()?;
    accept_virtio_bridge_vlan_bonding_tail()?;
    Ok(())
}

fn accept_skb_doc_and_source_semantics() -> Result<(), i32> {
    assert_eq!(SKB_CB_LEN, 48);

    let mut skb = alloc_skb(128)?;
    skb_reserve(&mut skb, 32)?;
    assert_eq!(skb.headroom(), 32);
    assert_eq!(skb.tailroom(), 96);

    skb_put(&mut skb, 4)?.copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(skb.data(), &[0xde, 0xad, 0xbe, 0xef]);

    skb_push(&mut skb, 2)?.copy_from_slice(&[0xaa, 0xbb]);
    assert_eq!(skb.data(), &[0xaa, 0xbb, 0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(skb_pull(&mut skb, 2)?, &[0xde, 0xad, 0xbe, 0xef]);

    skb_trim(&mut skb, 3)?;
    assert_eq!(skb.data(), &[0xde, 0xad, 0xbe]);
    skb_pad(&mut skb, 5)?;
    assert_eq!(skb.data(), &[0xde, 0xad, 0xbe, 0, 0, 0, 0, 0]);

    let clone = skb_clone(&skb);
    assert!(skb.cloned());
    let mut writable = skb_share_check(clone)?;
    writable.data_mut()[0] = 0x11;
    assert_eq!(skb.data()[0], 0xde);
    assert_eq!(writable.data()[0], 0x11);

    pskb_expand_head(&mut writable, 64, 16)?;
    assert_eq!(writable.headroom(), 64);
    assert_eq!(writable.tailroom(), 16);
    Ok(())
}

fn accept_socket_selftest_table() -> Result<(), i32> {
    const CASES: &[(u16, u16, u16, Option<i32>)] = &[
        (AF_MAX, 0, 0, Some(EAFNOSUPPORT)),
        (AF_INET, SOCK_STREAM, IPPROTO_TCP as u16, None),
        (
            AF_INET,
            SOCK_DGRAM,
            IPPROTO_TCP as u16,
            Some(EPROTONOSUPPORT),
        ),
        (AF_INET, SOCK_DGRAM, IPPROTO_UDP as u16, None),
        (
            AF_INET,
            SOCK_STREAM,
            IPPROTO_UDP as u16,
            Some(EPROTONOSUPPORT),
        ),
    ];

    for (domain, sock_type, protocol, expected_errno) in CASES {
        match (
            socket::socket(*domain, *sock_type, *protocol),
            expected_errno,
        ) {
            (Ok(_), None) => {}
            (Err(err), Some(expected)) if err == *expected => {}
            _ => return Err(EINVAL),
        }
    }
    Ok(())
}

fn accept_rtnetlink_dummy_bridge_vlan() -> Result<(), i32> {
    let _ = unregister_netdevice("test-dummy0");
    let req = RtnlMessage {
        msg_type: RTM_NEWLINK,
        ifindex: 0,
        flags: 0,
        mtu: 1500,
        name: String::from("test-dummy0"),
        addr: [2, 0, 0, 0, 8, 8],
    };
    let created = handle_rtnl_message(&req)?;
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].name, "test-dummy0");

    let dev = lookup_netdevice("test-dummy0").ok_or(EINVAL)?;
    set_device_up(&dev)?;
    assert!(rtm_getlink().iter().any(|link| {
        link.msg_type == RTM_GETLINK && link.name == "test-dummy0" && link.mtu == 1500
    }));

    let mut bridge = Bridge::new(dev.ifindex + 1000);
    bridge.add_port(dev.ifindex);
    assert!(bridge.forward_ports(dev.ifindex).is_empty());
    let vlan = VlanDevice::new(dev.ifindex + 1001, bridge.ifindex, 1)?;
    assert_eq!(vlan.vlan_id, 1);
    assert_eq!(vlan.lower_ifindex, bridge.ifindex);

    let del = RtnlMessage {
        msg_type: RTM_DELLINK,
        ..req
    };
    handle_rtnl_message(&del)?;
    Ok(())
}

fn accept_fib_tests_fixtures() -> Result<(), i32> {
    fib_clear();
    fib4_add(Fib4Entry {
        prefix: ipv4(0, 0, 0, 0),
        prefix_len: 0,
        gateway: ipv4(192, 0, 2, 1),
        ifindex: 10,
        metric: 500,
    });
    fib4_add(Fib4Entry {
        prefix: ipv4(198, 51, 100, 0),
        prefix_len: 24,
        gateway: 0,
        ifindex: 11,
        metric: 0,
    });
    fib4_add(Fib4Entry {
        prefix: ipv4(203, 0, 113, 0),
        prefix_len: 24,
        gateway: ipv4(198, 51, 100, 254),
        ifindex: 12,
        metric: 200,
    });
    fib4_add(Fib4Entry {
        prefix: ipv4(203, 0, 113, 0),
        prefix_len: 24,
        gateway: ipv4(198, 51, 100, 2),
        ifindex: 11,
        metric: 100,
    });

    let directly_connected = fib4_lookup(ipv4(198, 51, 100, 2)).ok_or(EINVAL)?;
    assert_eq!(directly_connected.gateway, 0);
    assert_eq!(directly_connected.ifindex, 11);

    let via_nexthop = fib4_lookup(ipv4(203, 0, 113, 1)).ok_or(EINVAL)?;
    assert_eq!(via_nexthop.gateway, ipv4(198, 51, 100, 2));
    assert_eq!(via_nexthop.ifindex, 11);

    let defaulted = fib4_lookup(ipv4(8, 8, 8, 8)).ok_or(EINVAL)?;
    assert_eq!(defaulted.gateway, ipv4(192, 0, 2, 1));

    let mut prefix = [0u8; 16];
    prefix[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
    let mut dst = prefix;
    dst[15] = 1;
    fib6_add(Fib6Entry {
        prefix,
        prefix_len: 32,
        gateway: [0; 16],
        ifindex: 13,
        metric: 10,
    });
    assert_eq!(fib6_lookup(dst).ok_or(EINVAL)?.ifindex, 13);
    Ok(())
}

fn accept_icmp_source_fixture() -> Result<(), i32> {
    let quoted = [0u8; 28];
    let icmp = build_icmp_dest_unreach(0, &quoted)?;
    let skb = build_ipv4_packet(
        ipv4(192, 0, 0, 8),
        ipv4(192, 0, 0, 1),
        IPPROTO_ICMP,
        &icmp,
        64,
    )?;
    let ip = parse_ipv4_packet(&skb)?;
    assert_eq!(ip.src, ipv4(192, 0, 0, 8));
    let icmp = parse_icmp(&ip.payload)?;
    assert_eq!(icmp.icmp_type, 3);
    assert_eq!(icmp.code, 0);
    Ok(())
}

fn build_icmp_dest_unreach(code: u8, payload: &[u8]) -> Result<Vec<u8>, i32> {
    let len = 8usize.checked_add(payload.len()).ok_or(EINVAL)?;
    let mut out = Vec::new();
    out.try_reserve_exact(len).map_err(|_| ENOMEM)?;
    out.resize(len, 0);
    out[0] = 3;
    out[1] = code;
    out[8..].copy_from_slice(payload);
    let csum = checksum(&out);
    out[2..4].copy_from_slice(&csum.to_be_bytes());
    Ok(out)
}

fn accept_udp_gso_selftest_table() -> Result<(), i32> {
    const V4_MAX_PAYLOAD: usize = ETH_MAX_MTU - CONST_HDRLEN_V4;
    const V6_MAX_PAYLOAD: usize = IP6_MAX_MTU - CONST_HDRLEN_V6;

    const V4_CASES: &[(usize, Option<usize>, Option<UdpGsoPlan>)] = &[
        (
            1,
            None,
            Some(UdpGsoPlan {
                full_segments: 0,
                last_len: 1,
            }),
        ),
        (
            CONST_MSS_V4,
            None,
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 0,
            }),
        ),
        (CONST_MSS_V4 + 1, None, None),
        (
            CONST_MSS_V4,
            Some(CONST_MSS_V4),
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 0,
            }),
        ),
        (
            CONST_MSS_V4,
            Some(CONST_MSS_V4 + 1),
            Some(UdpGsoPlan {
                full_segments: 0,
                last_len: CONST_MSS_V4,
            }),
        ),
        (CONST_MSS_V4 + 1, Some(CONST_MSS_V4 + 2), None),
        (
            CONST_MSS_V4 + 1,
            Some(CONST_MSS_V4),
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 1,
            }),
        ),
        (
            CONST_MSS_V4 * 2,
            Some(CONST_MSS_V4),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 0,
            }),
        ),
        (
            (CONST_MSS_V4 * 2) + 1,
            Some(CONST_MSS_V4),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 1,
            }),
        ),
        (
            (ETH_MAX_MTU / CONST_MSS_V4) * CONST_MSS_V4,
            Some(CONST_MSS_V4),
            Some(UdpGsoPlan {
                full_segments: ETH_MAX_MTU / CONST_MSS_V4,
                last_len: 0,
            }),
        ),
        (
            V4_MAX_PAYLOAD,
            Some(CONST_MSS_V4),
            Some(UdpGsoPlan {
                full_segments: CONST_MAX_SEGS_V4,
                last_len: V4_MAX_PAYLOAD - (CONST_MAX_SEGS_V4 * CONST_MSS_V4),
            }),
        ),
        (V4_MAX_PAYLOAD + 1, Some(CONST_MSS_V4), None),
        (
            1,
            Some(1),
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 0,
            }),
        ),
        (
            2,
            Some(1),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 0,
            }),
        ),
        (
            5,
            Some(2),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 1,
            }),
        ),
        (
            UDP_MAX_SEGMENTS,
            Some(1),
            Some(UdpGsoPlan {
                full_segments: UDP_MAX_SEGMENTS,
                last_len: 0,
            }),
        ),
        (UDP_MAX_SEGMENTS + 1, Some(1), None),
    ];

    const V6_CASES: &[(usize, Option<usize>, Option<UdpGsoPlan>)] = &[
        (
            1,
            None,
            Some(UdpGsoPlan {
                full_segments: 0,
                last_len: 1,
            }),
        ),
        (
            CONST_MSS_V6,
            None,
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 0,
            }),
        ),
        (CONST_MSS_V6 + 1, None, None),
        (
            CONST_MSS_V6,
            Some(CONST_MSS_V6),
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 0,
            }),
        ),
        (
            CONST_MSS_V6,
            Some(CONST_MSS_V6 + 1),
            Some(UdpGsoPlan {
                full_segments: 0,
                last_len: CONST_MSS_V6,
            }),
        ),
        (CONST_MSS_V6 + 1, Some(CONST_MSS_V6 + 2), None),
        (
            CONST_MSS_V6 + 1,
            Some(CONST_MSS_V6),
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 1,
            }),
        ),
        (
            CONST_MSS_V6 * 2,
            Some(CONST_MSS_V6),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 0,
            }),
        ),
        (
            (CONST_MSS_V6 * 2) + 1,
            Some(CONST_MSS_V6),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 1,
            }),
        ),
        (
            (IP6_MAX_MTU / CONST_MSS_V6) * CONST_MSS_V6,
            Some(CONST_MSS_V6),
            Some(UdpGsoPlan {
                full_segments: IP6_MAX_MTU / CONST_MSS_V6,
                last_len: 0,
            }),
        ),
        (
            V6_MAX_PAYLOAD,
            Some(CONST_MSS_V6),
            Some(UdpGsoPlan {
                full_segments: CONST_MAX_SEGS_V6,
                last_len: V6_MAX_PAYLOAD - (CONST_MAX_SEGS_V6 * CONST_MSS_V6),
            }),
        ),
        (V6_MAX_PAYLOAD + 1, Some(CONST_MSS_V6), None),
        (
            1,
            Some(1),
            Some(UdpGsoPlan {
                full_segments: 1,
                last_len: 0,
            }),
        ),
        (
            2,
            Some(1),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 0,
            }),
        ),
        (
            5,
            Some(2),
            Some(UdpGsoPlan {
                full_segments: 2,
                last_len: 1,
            }),
        ),
        (
            UDP_MAX_SEGMENTS,
            Some(1),
            Some(UdpGsoPlan {
                full_segments: UDP_MAX_SEGMENTS,
                last_len: 0,
            }),
        ),
        (UDP_MAX_SEGMENTS + 1, Some(1), None),
    ];

    for (payload_len, gso_len, expected) in V4_CASES {
        assert_udp_gso_case(*payload_len, *gso_len, false, *expected)?;
    }
    for (payload_len, gso_len, expected) in V6_CASES {
        assert_udp_gso_case(*payload_len, *gso_len, true, *expected)?;
    }
    Ok(())
}

fn assert_udp_gso_case(
    payload_len: usize,
    gso_len: Option<usize>,
    ipv6: bool,
    expected: Option<UdpGsoPlan>,
) -> Result<(), i32> {
    match (udp::udp_gso_plan(payload_len, gso_len, ipv6), expected) {
        (Ok(actual), Some(expected)) if actual == expected => Ok(()),
        (Err(EINVAL), None) => Ok(()),
        _ => Err(EINVAL),
    }
}

fn accept_tcp_packetdrill_fixture() -> Result<(), i32> {
    let mut active = TcpConnection::connect(4000);
    active.on_segment(TcpSegment {
        seq: 9000,
        ack: 4001,
        flags: TCP_SYN | TCP_ACK,
        wnd: 4096,
    })?;
    assert_eq!(active.state, TcpState::Established);

    let mut passive = TcpConnection::listen();
    passive.on_segment(TcpSegment {
        seq: 7000,
        ack: 0,
        flags: TCP_SYN,
        wnd: 4096,
    })?;
    assert_eq!(passive.state, TcpState::SynReceived);
    passive.on_segment(TcpSegment {
        seq: 7001,
        ack: 1,
        flags: TCP_ACK,
        wnd: 4096,
    })?;
    assert_eq!(passive.state, TcpState::Established);
    Ok(())
}

fn accept_netfilter_nft_drop_icmp() -> Result<(), i32> {
    let icmp = build_icmp_echo(9, 1, b"nft", false)?;
    let skb = build_ipv4_packet(
        ipv4(10, 0, 0, 1),
        ipv4(10, 0, 0, 2),
        IPPROTO_ICMP,
        &icmp,
        64,
    )?;
    let mut table = NfTable::new();
    assert_eq!(table.evaluate(Hook::LocalIn, &skb), Verdict::Accept);
    table.drop_icmp(Hook::LocalIn);
    assert_eq!(table.evaluate(Hook::LocalIn, &skb), Verdict::Drop);
    Ok(())
}

fn accept_virtio_bridge_vlan_bonding_tail() -> Result<(), i32> {
    let mut bridge = Bridge::new(200);
    bridge.add_port(201);
    bridge.add_port(202);
    assert_eq!(bridge.forward_ports(201), alloc::vec![202]);
    assert_eq!(VlanDevice::new(203, bridge.ifindex, 1)?.vlan_id, 1);

    let mut bond = Bond::new(300, BondMode::ActiveBackup);
    bond.add_slave(301);
    bond.add_slave(302);
    assert_eq!(bond.choose_tx_slave()?, 301);
    bond.set_link(301, false)?;
    assert_eq!(bond.choose_tx_slave()?, 302);
    Ok(())
}
