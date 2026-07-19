//! linux-parity: partial
//! linux-source: vendor/linux/net/ethernet, vendor/linux/net/ipv4
//! test-origin: linux:vendor/linux/net/ipv4/af_inet.c
//! Ethernet/ARP/IPv4 transport path used by module-backed net devices.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::{EADDRNOTAVAIL, EHOSTUNREACH, EINVAL, ENETDOWN, ENOTCONN};
use crate::net::ip::{IPPROTO_ICMP, IPPROTO_TCP, IPPROTO_UDP, checksum};
use crate::net::socket::{
    AF_INET, KernelSocket, QueuedPacket, RCV_SHUTDOWN, SOCK_DGRAM, SOCK_RAW, SOCK_STREAM, SockAddr,
    SocketCred, SocketRef, SocketState,
};

const ETH_P_IP: u16 = 0x0800;
const ETH_P_ARP: u16 = 0x0806;
const ARPHRD_ETHER: u16 = 1;
const ARPOP_REQUEST: u16 = 1;
const ARPOP_REPLY: u16 = 2;

const GUEST_IPV4: u32 = 0x0a00_020f;
const GATEWAY_IPV4: u32 = 0x0a00_0202;
const GUEST_NETMASK: u32 = 0xffff_ff00;

const TCP_FIN: u8 = 0x01;
const TCP_SYN: u8 = 0x02;
const TCP_RST: u8 = 0x04;
const TCP_PSH: u8 = 0x08;
const TCP_ACK: u8 = 0x10;

#[derive(Clone, Debug)]
pub(crate) struct TcpState {
    pub local_addr: u32,
    pub remote_addr: u32,
    pub local_port: u16,
    pub remote_port: u16,
    pub snd_una: u32,
    pub snd_nxt: u32,
    pub rcv_nxt: u32,
    pub fin_received: bool,
}

struct PendingFrame {
    next_hop: u32,
    frame: Vec<u8>,
}

struct WireState {
    neighbours: BTreeMap<u32, [u8; 6]>,
    arp_pending: BTreeSet<u32>,
    pending_frames: Vec<PendingFrame>,
    ip_identification: u16,
}

static WIRE_STATE: Mutex<WireState> = Mutex::new(WireState {
    neighbours: BTreeMap::new(),
    arp_pending: BTreeSet::new(),
    pending_frames: Vec::new(),
    ip_identification: 1,
});

fn active_device() -> Option<crate::net::device::NetDeviceRef> {
    for dev in crate::net::device::list_netdevices() {
        if dev.linux_dev.is_none() || !dev.is_up() {
            continue;
        }
        return Some(dev);
    }
    None
}

fn active_device_by_ifindex(ifindex: u32) -> Option<crate::net::device::NetDeviceRef> {
    crate::net::device::list_netdevices()
        .into_iter()
        .find(|dev| dev.ifindex == ifindex && dev.linux_dev.is_some() && dev.is_up())
}

fn select_ipv4_source_addr(explicit_source: Option<u32>, bound_source: u32) -> u32 {
    explicit_source
        .filter(|addr| *addr != 0)
        // __inet_bind() retains multicast/broadcast in inet_rcv_saddr but
        // clears inet_saddr so output selects a device address. KernelSocket
        // has one local address, so reproduce that split while selecting the
        // wire source rather than weakening bind semantics.
        .or_else(|| {
            (bound_source != 0 && crate::net::socket::inet_addr_is_local(bound_source, None))
                .then_some(bound_source)
        })
        .unwrap_or(GUEST_IPV4)
}

fn transmit(dev: &crate::net::device::NetDeviceRef, frame: &[u8]) -> Result<(), i32> {
    let Some(raw) = dev.linux_dev.map(|address| address as *mut u8) else {
        return Err(ENETDOWN);
    };
    crate::net::module_abi::transmit_linux_ethernet_frame(raw, frame)
}

fn build_arp(
    source_mac: [u8; 6],
    destination_mac: [u8; 6],
    operation: u16,
    sender_ip: u32,
    target_mac: [u8; 6],
    target_ip: u32,
) -> Vec<u8> {
    let mut frame = alloc::vec![0u8; 42];
    frame[0..6].copy_from_slice(&destination_mac);
    frame[6..12].copy_from_slice(&source_mac);
    frame[12..14].copy_from_slice(&ETH_P_ARP.to_be_bytes());
    frame[14..16].copy_from_slice(&ARPHRD_ETHER.to_be_bytes());
    frame[16..18].copy_from_slice(&ETH_P_IP.to_be_bytes());
    frame[18] = 6;
    frame[19] = 4;
    frame[20..22].copy_from_slice(&operation.to_be_bytes());
    frame[22..28].copy_from_slice(&source_mac);
    frame[28..32].copy_from_slice(&sender_ip.to_be_bytes());
    frame[32..38].copy_from_slice(&target_mac);
    frame[38..42].copy_from_slice(&target_ip.to_be_bytes());
    frame
}

fn next_hop(destination: u32) -> u32 {
    if destination & GUEST_NETMASK == GUEST_IPV4 & GUEST_NETMASK {
        destination
    } else {
        GATEWAY_IPV4
    }
}

fn send_ethernet_ipv4_via(
    dev: &crate::net::device::NetDeviceRef,
    mut frame: Vec<u8>,
    destination: u32,
) -> Result<(), i32> {
    let hop = next_hop(destination);
    let mut pending_frame = Some(frame);
    let (neighbour, send_arp) = {
        let mut state = WIRE_STATE.lock();
        if let Some(mac) = state.neighbours.get(&hop).copied() {
            (Some(mac), false)
        } else {
            state.pending_frames.push(PendingFrame {
                next_hop: hop,
                frame: pending_frame.take().expect("pending IPv4 frame"),
            });
            let send = state.arp_pending.insert(hop);
            (None, send)
        }
    };
    if let Some(mac) = neighbour {
        frame = pending_frame.expect("resolved IPv4 frame");
        frame[0..6].copy_from_slice(&mac);
        return transmit(dev, &frame);
    }
    if send_arp {
        let request = build_arp(
            dev.dev_addr,
            [0xff; 6],
            ARPOP_REQUEST,
            GUEST_IPV4,
            [0; 6],
            hop,
        );
        transmit(dev, &request)?;
    }
    Ok(())
}

fn send_ipv4(destination: u32, protocol: u8, payload: &[u8]) -> Result<(), i32> {
    let Some(dev) = active_device() else {
        return Err(ENETDOWN);
    };
    send_ipv4_via(&dev, GUEST_IPV4, destination, protocol, 64, payload)
}

fn send_ipv4_via(
    dev: &crate::net::device::NetDeviceRef,
    source: u32,
    destination: u32,
    protocol: u8,
    ttl: u8,
    payload: &[u8],
) -> Result<(), i32> {
    let total_length = 20usize.checked_add(payload.len()).ok_or(EINVAL)?;
    if total_length > u16::MAX as usize {
        return Err(EINVAL);
    }
    let identification = {
        let mut state = WIRE_STATE.lock();
        let value = state.ip_identification;
        state.ip_identification = state.ip_identification.wrapping_add(1);
        value
    };
    let mut frame = alloc::vec![0u8; 14 + total_length];
    frame[6..12].copy_from_slice(&dev.dev_addr);
    frame[12..14].copy_from_slice(&ETH_P_IP.to_be_bytes());
    let ip = &mut frame[14..34];
    ip[0] = 0x45;
    ip[2..4].copy_from_slice(&(total_length as u16).to_be_bytes());
    ip[4..6].copy_from_slice(&identification.to_be_bytes());
    ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
    ip[8] = ttl;
    ip[9] = protocol;
    ip[12..16].copy_from_slice(&source.to_be_bytes());
    ip[16..20].copy_from_slice(&destination.to_be_bytes());
    let sum = checksum(ip);
    ip[10..12].copy_from_slice(&sum.to_be_bytes());
    frame[34..].copy_from_slice(payload);
    send_ethernet_ipv4_via(dev, frame, destination)
}

fn transport_checksum(source: u32, destination: u32, protocol: u8, segment: &[u8]) -> u16 {
    let mut pseudo = Vec::with_capacity(12 + segment.len() + (segment.len() & 1));
    pseudo.extend_from_slice(&source.to_be_bytes());
    pseudo.extend_from_slice(&destination.to_be_bytes());
    pseudo.push(0);
    pseudo.push(protocol);
    pseudo.extend_from_slice(&(segment.len() as u16).to_be_bytes());
    pseudo.extend_from_slice(segment);
    checksum(&pseudo)
}

fn build_tcp_segment(
    source: u32,
    destination: u32,
    source_port: u16,
    destination_port: u16,
    sequence: u32,
    acknowledgement: u32,
    flags: u8,
    payload: &[u8],
) -> Vec<u8> {
    let mut segment = alloc::vec![0u8; 20 + payload.len()];
    segment[0..2].copy_from_slice(&source_port.to_be_bytes());
    segment[2..4].copy_from_slice(&destination_port.to_be_bytes());
    segment[4..8].copy_from_slice(&sequence.to_be_bytes());
    segment[8..12].copy_from_slice(&acknowledgement.to_be_bytes());
    segment[12] = 5 << 4;
    segment[13] = flags;
    segment[14..16].copy_from_slice(&64240u16.to_be_bytes());
    segment[20..].copy_from_slice(payload);
    let sum = transport_checksum(source, destination, IPPROTO_TCP, &segment);
    segment[16..18].copy_from_slice(&sum.to_be_bytes());
    segment
}

fn send_tcp_control(sock: &SocketRef, flags: u8) -> Result<(), i32> {
    let tcp = sock.lock().wire_tcp.clone().ok_or(ENOTCONN)?;
    let segment = build_tcp_segment(
        tcp.local_addr,
        tcp.remote_addr,
        tcp.local_port,
        tcp.remote_port,
        tcp.snd_nxt,
        tcp.rcv_nxt,
        flags,
        &[],
    );
    send_ipv4(tcp.remote_addr, IPPROTO_TCP, &segment)
}

pub(crate) fn tcp_connect(sock: &SocketRef, remote_addr: u32, remote_port: u16) -> Result<(), i32> {
    let initial_sequence = (crate::kernel::time::clocksource::read_tsc() as u32)
        .wrapping_add(remote_addr)
        .wrapping_add(remote_port as u32);
    let (local_port, segment) = {
        let mut socket = sock.lock();
        if socket.family != AF_INET || socket.sock_type != SOCK_STREAM {
            return Err(EINVAL);
        }
        crate::net::socket::autobind_inet(&mut socket);
        let Some(SockAddr::Inet {
            addr: local_addr,
            port: local_port,
        }) = socket.local
        else {
            return Err(EINVAL);
        };
        socket.peer = Some(SockAddr::Inet {
            addr: remote_addr,
            port: remote_port,
        });
        socket.state = SocketState::Connecting;
        socket.pending_error = 0;
        socket.wire_tcp = Some(TcpState {
            local_addr,
            remote_addr,
            local_port,
            remote_port,
            snd_una: initial_sequence,
            snd_nxt: initial_sequence.wrapping_add(1),
            rcv_nxt: 0,
            fin_received: false,
        });
        (
            local_port,
            build_tcp_segment(
                local_addr,
                remote_addr,
                local_port,
                remote_port,
                initial_sequence,
                0,
                TCP_SYN,
                &[],
            ),
        )
    };
    let _ = local_port;
    send_ipv4(remote_addr, IPPROTO_TCP, &segment)
}

pub(crate) fn tcp_send(sock: &SocketRef, bytes: &[u8]) -> Result<usize, i32> {
    if bytes.is_empty() {
        return Ok(0);
    }
    let (remote, segment) = {
        let mut socket = sock.lock();
        if socket.state != SocketState::Connected
            || socket.shutdown & crate::net::socket::SEND_SHUTDOWN != 0
        {
            return Err(ENOTCONN);
        }
        let tcp = socket.wire_tcp.as_mut().ok_or(ENOTCONN)?;
        let sequence = tcp.snd_nxt;
        tcp.snd_nxt = tcp.snd_nxt.wrapping_add(bytes.len() as u32);
        (
            tcp.remote_addr,
            build_tcp_segment(
                tcp.local_addr,
                tcp.remote_addr,
                tcp.local_port,
                tcp.remote_port,
                sequence,
                tcp.rcv_nxt,
                TCP_ACK | TCP_PSH,
                bytes,
            ),
        )
    };
    send_ipv4(remote, IPPROTO_TCP, &segment)?;
    Ok(bytes.len())
}

pub(crate) fn send_inet(
    sock: &SocketRef,
    bytes: &[u8],
    destination: &SockAddr,
    send_meta: Option<&crate::net::socket::PacketMeta>,
) -> Option<Result<usize, i32>> {
    let SockAddr::Inet { addr, port } = *destination else {
        return None;
    };
    // 127.0.0.0/8 is delivered by the local inet socket path.  It must never
    // be emitted through a physical device (Linux's `ip_route_output_key_hash`
    // selects `lo` for this prefix).
    if addr >> 24 == 127 {
        return None;
    }
    let (sock_type, protocol, source_port, preferred_ifindex, source_addr, ttl) = {
        let mut socket = sock.lock();
        if socket.family != AF_INET || !matches!(socket.sock_type, SOCK_DGRAM | SOCK_RAW) {
            return None;
        }
        crate::net::socket::autobind_inet(&mut socket);
        let source_port = match socket.local {
            Some(SockAddr::Inet { addr, port }) => (addr, port),
            _ => (0, 0),
        };
        let source_addr = select_ipv4_source_addr(
            send_meta.and_then(|meta| meta.local_inet_addr),
            source_port.0,
        );
        let ttl = send_meta
            .and_then(|meta| meta.ttl)
            .filter(|ttl| *ttl != 0)
            .unwrap_or(64);
        let preferred_ifindex = send_meta
            .map(|meta| meta.ifindex)
            .filter(|ifindex| *ifindex != 0)
            .or((socket.bound_ifindex != 0).then_some(socket.bound_ifindex))
            .or((socket.unicast_ifindex != 0).then_some(socket.unicast_ifindex))
            .unwrap_or(0);
        if !crate::net::socket::inet_addr_is_local(source_addr, Some(preferred_ifindex)) {
            return Some(Err(EADDRNOTAVAIL));
        }
        (
            socket.sock_type,
            socket.protocol,
            source_port.1,
            preferred_ifindex,
            source_addr,
            ttl,
        )
    };
    let dev = if preferred_ifindex != 0 {
        match active_device_by_ifindex(preferred_ifindex) {
            Some(dev) => Some(dev),
            None => return Some(Err(ENETDOWN)),
        }
    } else {
        active_device()
    };
    let Some(dev) = dev else {
        return Some(Err(ENETDOWN));
    };
    let result = if sock_type == SOCK_RAW || protocol == IPPROTO_ICMP as u16 {
        send_ipv4_via(&dev, source_addr, addr, IPPROTO_ICMP, ttl, bytes)
    } else {
        let length = 8usize.checked_add(bytes.len()).ok_or(EINVAL);
        match length {
            Ok(length) if length <= u16::MAX as usize => {
                let mut datagram = alloc::vec![0u8; length];
                datagram[0..2].copy_from_slice(&source_port.to_be_bytes());
                datagram[2..4].copy_from_slice(&port.to_be_bytes());
                datagram[4..6].copy_from_slice(&(length as u16).to_be_bytes());
                datagram[8..].copy_from_slice(bytes);
                let checksum = transport_checksum(source_addr, addr, IPPROTO_UDP, &datagram);
                datagram[6..8].copy_from_slice(
                    &(if checksum == 0 { 0xffff } else { checksum }).to_be_bytes(),
                );
                #[cfg(not(test))]
                if crate::kernel::debug_trace::netlink_enabled() && port == 53 {
                    crate::linux_driver_abi::tty::serial_println!(
                        "trace-udp-send src={}.{}.{}.{}:{} dst={}.{}.{}.{}:{} len={} checksum=0x{:04x}",
                        (source_addr >> 24) & 0xff,
                        (source_addr >> 16) & 0xff,
                        (source_addr >> 8) & 0xff,
                        source_addr & 0xff,
                        source_port,
                        (addr >> 24) & 0xff,
                        (addr >> 16) & 0xff,
                        (addr >> 8) & 0xff,
                        addr & 0xff,
                        port,
                        bytes.len(),
                        if checksum == 0 { 0xffff } else { checksum }
                    );
                }
                send_ipv4_via(&dev, source_addr, addr, IPPROTO_UDP, ttl, &datagram)
            }
            _ => Err(EINVAL),
        }
    };
    Some(result.map(|_| bytes.len()))
}

fn handle_arp(linux_dev: *mut u8, frame: &[u8]) {
    if frame.len() < 42
        || u16::from_be_bytes([frame[14], frame[15]]) != ARPHRD_ETHER
        || u16::from_be_bytes([frame[16], frame[17]]) != ETH_P_IP
        || frame[18] != 6
        || frame[19] != 4
    {
        return;
    }
    let operation = u16::from_be_bytes([frame[20], frame[21]]);
    let mut sender_mac = [0u8; 6];
    sender_mac.copy_from_slice(&frame[22..28]);
    let sender_ip = u32::from_be_bytes([frame[28], frame[29], frame[30], frame[31]]);
    let target_ip = u32::from_be_bytes([frame[38], frame[39], frame[40], frame[41]]);
    if operation == ARPOP_REPLY && target_ip == GUEST_IPV4 {
        let pending = {
            let mut state = WIRE_STATE.lock();
            state.neighbours.insert(sender_ip, sender_mac);
            state.arp_pending.remove(&sender_ip);
            let mut ready = Vec::new();
            let mut retained = Vec::new();
            for pending in state.pending_frames.drain(..) {
                if pending.next_hop == sender_ip {
                    ready.push(pending.frame);
                } else {
                    retained.push(pending);
                }
            }
            state.pending_frames = retained;
            ready
        };
        if let Some(dev) = crate::net::device::lookup_linux_netdevice(linux_dev) {
            for mut pending in pending {
                pending[0..6].copy_from_slice(&sender_mac);
                let _ = transmit(&dev, &pending);
            }
        }
    } else if operation == ARPOP_REQUEST && target_ip == GUEST_IPV4 {
        if let Some(dev) = crate::net::device::lookup_linux_netdevice(linux_dev) {
            let reply = build_arp(
                dev.dev_addr,
                sender_mac,
                ARPOP_REPLY,
                GUEST_IPV4,
                sender_mac,
                sender_ip,
            );
            let _ = transmit(&dev, &reply);
        }
    }
}

fn matching_socket(
    protocol: u8,
    local_port: u16,
    remote_addr: u32,
    remote_port: u16,
) -> Option<SocketRef> {
    crate::net::socket::inet_socket_snapshot()
        .into_iter()
        .find(|sock| {
            let socket = sock.lock();
            if socket.family != AF_INET {
                return false;
            }
            match protocol {
                IPPROTO_TCP => socket.wire_tcp.as_ref().is_some_and(|tcp| {
                    tcp.local_port == local_port
                        && tcp.remote_addr == remote_addr
                        && tcp.remote_port == remote_port
                }),
                IPPROTO_UDP => {
                    socket.sock_type == SOCK_DGRAM
                        && matches!(socket.local, Some(SockAddr::Inet { port, .. }) if port == local_port)
                }
                IPPROTO_ICMP => {
                    matches!(socket.sock_type, SOCK_DGRAM | SOCK_RAW)
                        && socket.protocol == IPPROTO_ICMP as u16
                }
                _ => false,
            }
        })
}

fn queue_packet(sock: &SocketRef, bytes: Vec<u8>, peer: SockAddr) {
    sock.lock().recvq.push_back(QueuedPacket {
        bytes,
        peer: Some(peer),
        fds: Vec::new(),
        cred: SocketCred::default(),
        meta: crate::net::socket::PacketMeta::default(),
    });
    crate::net::socket::wake_socket_recv(sock);
}

fn handle_tcp(source: u32, destination: u32, segment: &[u8]) {
    if segment.len() < 20 || transport_checksum(source, destination, IPPROTO_TCP, segment) != 0 {
        return;
    }
    let source_port = u16::from_be_bytes([segment[0], segment[1]]);
    let destination_port = u16::from_be_bytes([segment[2], segment[3]]);
    let sequence = u32::from_be_bytes([segment[4], segment[5], segment[6], segment[7]]);
    let acknowledgement = u32::from_be_bytes([segment[8], segment[9], segment[10], segment[11]]);
    let header_length = ((segment[12] >> 4) as usize) * 4;
    if header_length < 20 || header_length > segment.len() {
        return;
    }
    let flags = segment[13];
    let payload = &segment[header_length..];
    let Some(sock) = matching_socket(IPPROTO_TCP, destination_port, source, source_port) else {
        return;
    };
    let mut send_ack = false;
    let mut deliver = None;
    let mut wake = false;
    {
        let mut socket = sock.lock();
        let state = socket.state;
        let Some(tcp) = socket.wire_tcp.as_mut() else {
            return;
        };
        if flags & TCP_RST != 0 {
            socket.pending_error = crate::include::uapi::errno::ECONNREFUSED;
            socket.state = SocketState::Created;
            wake = true;
        } else if state == SocketState::Connecting
            && flags & (TCP_SYN | TCP_ACK) == (TCP_SYN | TCP_ACK)
            && acknowledgement == tcp.snd_nxt
        {
            tcp.snd_una = acknowledgement;
            tcp.rcv_nxt = sequence.wrapping_add(1);
            socket.state = SocketState::Connected;
            send_ack = true;
            wake = true;
        } else if state == SocketState::Connected {
            if flags & TCP_ACK != 0 && acknowledgement.wrapping_sub(tcp.snd_una) < 0x8000_0000 {
                tcp.snd_una = acknowledgement.min(tcp.snd_nxt);
            }
            if !payload.is_empty() && sequence == tcp.rcv_nxt {
                tcp.rcv_nxt = tcp.rcv_nxt.wrapping_add(payload.len() as u32);
                deliver = Some(payload.to_vec());
                send_ack = true;
            } else if !payload.is_empty() {
                send_ack = true;
            }
            let fin_sequence = sequence.wrapping_add(payload.len() as u32);
            if flags & TCP_FIN != 0 && fin_sequence == tcp.rcv_nxt {
                tcp.rcv_nxt = tcp.rcv_nxt.wrapping_add(1);
                tcp.fin_received = true;
                socket.shutdown |= RCV_SHUTDOWN;
                send_ack = true;
                wake = true;
            }
        }
    }
    if let Some(bytes) = deliver {
        queue_packet(
            &sock,
            bytes,
            SockAddr::Inet {
                addr: source,
                port: source_port,
            },
        );
    }
    if send_ack {
        let _ = send_tcp_control(&sock, TCP_ACK);
    }
    if wake {
        crate::net::socket::wake_socket_recv(&sock);
    }
}

fn handle_udp(source: u32, destination: u32, datagram: &[u8], ifindex: u32, ttl: u8) {
    if datagram.len() < 8 {
        return;
    }
    let source_port = u16::from_be_bytes([datagram[0], datagram[1]]);
    let destination_port = u16::from_be_bytes([datagram[2], datagram[3]]);
    let length = u16::from_be_bytes([datagram[4], datagram[5]]) as usize;
    if length < 8 || length > datagram.len() {
        return;
    }
    let udp_checksum = u16::from_be_bytes([datagram[6], datagram[7]]);
    if udp_checksum != 0
        && transport_checksum(source, destination, IPPROTO_UDP, &datagram[..length]) != 0
    {
        #[cfg(not(test))]
        if crate::kernel::debug_trace::netlink_enabled()
            && (source_port == 53 || destination_port == 53)
        {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-udp-recv-drop src={}.{}.{}.{}:{} dst={}.{}.{}.{}:{} len={} checksum=0x{:04x} computed=0x{:04x}",
                (source >> 24) & 0xff,
                (source >> 16) & 0xff,
                (source >> 8) & 0xff,
                source & 0xff,
                source_port,
                (destination >> 24) & 0xff,
                (destination >> 16) & 0xff,
                (destination >> 8) & 0xff,
                destination & 0xff,
                destination_port,
                length,
                udp_checksum,
                transport_checksum(source, destination, IPPROTO_UDP, &datagram[..length]),
            );
        }
        return;
    }
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled()
        && (source_port == 53 || destination_port == 53)
    {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-udp-recv src={}.{}.{}.{}:{} dst={}.{}.{}.{}:{} len={} checksum=0x{:04x}",
            (source >> 24) & 0xff,
            (source >> 16) & 0xff,
            (source >> 8) & 0xff,
            source & 0xff,
            source_port,
            (destination >> 24) & 0xff,
            (destination >> 16) & 0xff,
            (destination >> 8) & 0xff,
            destination & 0xff,
            destination_port,
            length,
            udp_checksum
        );
    }
    let matched = matching_socket(IPPROTO_UDP, destination_port, source, source_port);
    if let Some(sock) = matched {
        sock.lock().recvq.push_back(QueuedPacket {
            bytes: datagram[8..length].to_vec(),
            peer: Some(SockAddr::Inet {
                addr: source,
                port: source_port,
            }),
            fds: Vec::new(),
            cred: SocketCred::default(),
            meta: crate::net::socket::PacketMeta {
                ifindex,
                local_inet_addr: Some(destination),
                ttl: Some(ttl),
                netlink_group: 0,
            },
        });
        crate::net::socket::wake_socket_recv(&sock);
    }
}

fn handle_ipv4(linux_dev: *mut u8, frame: &[u8]) {
    if frame.len() < 34 {
        return;
    }
    let ip = &frame[14..];
    let header_length = ((ip[0] & 0x0f) as usize) * 4;
    if ip[0] >> 4 != 4 || header_length < 20 || header_length > ip.len() {
        return;
    }
    let total_length = u16::from_be_bytes([ip[2], ip[3]]) as usize;
    if total_length < header_length
        || total_length > ip.len()
        || checksum(&ip[..header_length]) != 0
    {
        return;
    }
    let fragment = u16::from_be_bytes([ip[6], ip[7]]);
    if fragment & 0x3fff != 0 {
        return;
    }
    let source = u32::from_be_bytes([ip[12], ip[13], ip[14], ip[15]]);
    let destination = u32::from_be_bytes([ip[16], ip[17], ip[18], ip[19]]);
    let ttl = ip[8];
    if destination != GUEST_IPV4 {
        return;
    }
    let ifindex = crate::net::device::lookup_linux_netdevice(linux_dev)
        .map(|dev| dev.ifindex)
        .unwrap_or(0);
    let payload = &ip[header_length..total_length];
    match ip[9] {
        IPPROTO_TCP => handle_tcp(source, destination, payload),
        IPPROTO_UDP => handle_udp(source, destination, payload, ifindex, ttl),
        IPPROTO_ICMP => {
            if let Some(sock) = matching_socket(IPPROTO_ICMP, 0, source, 0) {
                sock.lock().recvq.push_back(QueuedPacket {
                    bytes: payload.to_vec(),
                    peer: Some(SockAddr::Inet {
                        addr: source,
                        port: 0,
                    }),
                    fds: Vec::new(),
                    cred: SocketCred::default(),
                    meta: crate::net::socket::PacketMeta {
                        ifindex,
                        local_inet_addr: Some(destination),
                        ttl: Some(ttl),
                        netlink_group: 0,
                    },
                });
                crate::net::socket::wake_socket_recv(&sock);
            }
        }
        _ => {}
    }
}

pub(crate) fn receive_frame(linux_dev: *mut u8, frame: &[u8]) {
    if frame.len() < 14 {
        return;
    }
    match u16::from_be_bytes([frame[12], frame[13]]) {
        ETH_P_ARP => handle_arp(linux_dev, frame),
        ETH_P_IP => handle_ipv4(linux_dev, frame),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fib::ipv4;

    #[test]
    fn bound_receive_only_ipv4_addresses_do_not_become_wire_sources() {
        assert_eq!(
            select_ipv4_source_addr(None, ipv4(239, 1, 2, 3)),
            GUEST_IPV4
        );
        assert_eq!(select_ipv4_source_addr(None, u32::MAX), GUEST_IPV4);
        assert_eq!(
            select_ipv4_source_addr(None, ipv4(127, 255, 255, 255)),
            GUEST_IPV4
        );

        // A source covered by an RTN_LOCAL prefix remains usable.
        assert_eq!(
            select_ipv4_source_addr(None, ipv4(127, 0, 0, 53)),
            ipv4(127, 0, 0, 53)
        );
    }
}
