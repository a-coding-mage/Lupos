//! linux-parity: partial
//! linux-source: vendor/linux/net/socket.c
//! test-origin: linux:vendor/linux/net/socket.c
//! Socket layer scaffolding for AF_INET, AF_INET6, AF_UNIX, AF_PACKET, AF_NETLINK.

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::file::fput;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{
    EADDRINUSE, EADDRNOTAVAIL, EAFNOSUPPORT, EAGAIN, ECONNREFUSED, ECONNRESET, EEXIST, EINPROGRESS,
    EINVAL, ENETDOWN, ENETUNREACH, ENODEV, ENOPROTOOPT, ENOTCONN, EOPNOTSUPP, EPERM, EPIPE,
    EPROTONOSUPPORT, ESRCH,
};
use crate::kernel::capability::{
    CAP_AUDIT_CONTROL, CAP_AUDIT_READ, CAP_AUDIT_WRITE, CAP_NET_ADMIN, capable,
};
use crate::kernel::cred::GroupInfo;
use crate::kernel::pid::{KPid, get_pid, put_pid};
use crate::kernel::sched;
use crate::kernel::sched::wait::WaitQueueHead;
use crate::kernel::task::TaskStruct;
use crate::kernel::task::task_state::TASK_INTERRUPTIBLE;
use crate::net::fib::ipv4;
use crate::net::ip::{IPPROTO_ICMP, IPPROTO_TCP, IPPROTO_UDP, checksum};
use crate::net::rtnetlink::{
    NETLINK_AUDIT, NETLINK_GENERIC, NETLINK_KOBJECT_UEVENT, NETLINK_ROUTE, RTM_DELADDR,
    RTM_DELLINK, RTM_DELROUTE, RTM_GETADDR, RTM_GETLINK, RTM_GETNEIGH, RTM_GETNEXTHOP,
    RTM_GETQDISC, RTM_GETROUTE, RTM_GETRULE, RTM_NEWADDR, RTM_NEWLINK, RTM_NEWROUTE, RTM_SETLINK,
};

pub const AF_UNIX: u16 = 1;
pub const AF_INET: u16 = 2;
pub const AF_INET6: u16 = 10;
pub const AF_NETLINK: u16 = 16;
pub const AF_PACKET: u16 = 17;
pub const AF_MAX: u16 = 46;

pub const SOCK_STREAM: u16 = 1;
pub const SOCK_DGRAM: u16 = 2;
pub const SOCK_RAW: u16 = 3;
pub const SOCK_SEQPACKET: u16 = 5;
pub const SOCK_TYPE_MASK: u32 = 0xf;
pub const SOCK_CLOEXEC: u32 = crate::include::uapi::fcntl::O_CLOEXEC;
pub const SOCK_NONBLOCK: u32 = crate::include::uapi::fcntl::O_NONBLOCK;

pub const SO_REUSEADDR: u32 = 2;
pub const SO_TYPE: u32 = 3;
pub const SO_ERROR: u32 = 4;
pub const SO_SNDBUF: u32 = 7;
pub const SO_RCVBUF: u32 = 8;
pub const SO_PASSCRED: u32 = 16;
pub const SO_PEERCRED: u32 = 17;
pub const SO_RCVTIMEO_OLD: u32 = 20;
pub const SO_SNDTIMEO_OLD: u32 = 21;
pub const SO_TIMESTAMP_OLD: u32 = 29;
pub const SO_ACCEPTCONN: u32 = 30;
pub const SO_PEERSEC: u32 = 31;
pub const SO_SNDBUFFORCE: u32 = 32;
pub const SO_RCVBUFFORCE: u32 = 33;
pub const SO_PASSSEC: u32 = 34;
pub const SO_PROTOCOL: u32 = 38;
pub const SO_DOMAIN: u32 = 39;
pub const SO_TIMESTAMP_NEW: u32 = 63;
pub const SO_RCVTIMEO_NEW: u32 = 66;
pub const SO_SNDTIMEO_NEW: u32 = 67;
pub const SO_PASSPIDFD: u32 = 76;
pub const SO_PEERPIDFD: u32 = 77;
pub const SO_PASSRIGHTS: u32 = 83;
pub const SO_PEERGROUPS: u32 = 59;
pub const IP_RECVTTL: u32 = 12;

pub const RCV_SHUTDOWN: u8 = 1;
pub const SEND_SHUTDOWN: u8 = 2;
pub const SHUTDOWN_MASK: u8 = RCV_SHUTDOWN | SEND_SHUTDOWN;

pub type SocketRef = Arc<Mutex<KernelSocket>>;

#[derive(Debug)]
pub struct SocketPidRef {
    pub pid: i32,
    pub task: *mut TaskStruct,
    pub kpid: *mut KPid,
}

unsafe impl Send for SocketPidRef {}
unsafe impl Sync for SocketPidRef {}

impl Clone for SocketPidRef {
    fn clone(&self) -> Self {
        if !self.kpid.is_null() {
            unsafe {
                get_pid(&*self.kpid);
            }
        }
        Self {
            pid: self.pid,
            task: self.task,
            kpid: self.kpid,
        }
    }
}

impl Drop for SocketPidRef {
    fn drop(&mut self) {
        if !self.kpid.is_null() {
            unsafe {
                put_pid(self.kpid);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SocketCred {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
    pub groups: GroupInfo,
    pub pid_ref: Option<SocketPidRef>,
}

impl PartialEq for SocketCred {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid && self.uid == other.uid && self.gid == other.gid
    }
}

impl Eq for SocketCred {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SockAddr {
    Inet { addr: u32, port: u16 },
    Inet6 { addr: [u8; 16], port: u16 },
    Unix(String),
    Netlink { pid: u32, groups: u32 },
    Packet { ifindex: u32, protocol: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketState {
    Created,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closed,
}

pub struct KernelSocket {
    pub family: u16,
    pub sock_type: u16,
    pub protocol: u16,
    /// Owning network namespace. AF_INET/AF_INET6/AF_NETLINK lookup and
    /// notifications never cross this boundary.
    pub net_ns: usize,
    pub state: SocketState,
    /// Linux `sk_shutdown` (`RCV_SHUTDOWN` / `SEND_SHUTDOWN`).
    pub shutdown: u8,
    /// Pending `sk_err`, consumed by the next receive / SO_ERROR query.
    pub pending_error: i32,
    pub local: Option<SockAddr>,
    pub peer: Option<SockAddr>,
    pub recvq: VecDeque<QueuedPacket>,
    pub backlog: VecDeque<SocketRef>,
    /// `sk->sk_wq->wait`: receive/accept/poll sleepers for this socket.
    pub recv_wait: Arc<WaitQueueHead>,
    pub peer_socket: Option<SocketRef>,
    pub cred: SocketCred,
    pub peer_cred: Option<SocketCred>,
    pub reuseaddr: bool,
    pub passcred: bool,
    pub passpidfd: bool,
    pub passrights: bool,
    pub timestamp_old: bool,
    pub timestamp_new: bool,
    pub recv_ttl: bool,
    pub recv_pktinfo: bool,
    pub unicast_ifindex: u32,
    pub bound_ifindex: u32,
    pub recv_timeout_ns: u64,
    pub send_timeout_ns: u64,
    pub last_rx_meta: PacketMeta,
    pub(crate) wire_tcp: Option<crate::net::wire::TcpState>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PacketMeta {
    pub ifindex: u32,
    pub local_inet_addr: Option<u32>,
    pub ttl: Option<u8>,
    pub netlink_group: u32,
}

/// Packet sitting in a socket's `recvq`.
///
/// `fds` carries the SCM_RIGHTS attachment for AF_UNIX socket fd-passing.
/// On send, the kernel records the cloned `FileRef`s here; on recv, they are
/// installed into the receiving task's fdtable.  Linux reference:
/// `vendor/linux/net/unix/scm.c::unix_attach_fds`.
#[derive(Clone)]
pub struct QueuedPacket {
    pub bytes: Vec<u8>,
    pub peer: Option<SockAddr>,
    pub fds: Vec<FileRef>,
    pub cred: SocketCred,
    pub meta: PacketMeta,
}

lazy_static! {
    static ref BOUND: Mutex<BTreeMap<SockAddr, Vec<SocketRef>>> = Mutex::new(BTreeMap::new());
    static ref IFADDRS: Mutex<Vec<IfAddrRecord>> = Mutex::new(Vec::new());
    static ref ROUTE4S: Mutex<Vec<Route4Record>> = Mutex::new(Vec::new());
    static ref IFADDRS_BY_NAMESPACE: Mutex<BTreeMap<usize, Vec<IfAddrRecord>>> =
        Mutex::new(BTreeMap::new());
    static ref ROUTE4S_BY_NAMESPACE: Mutex<BTreeMap<usize, Vec<Route4Record>>> =
        Mutex::new(BTreeMap::new());
    static ref INET_SOCKETS: Mutex<Vec<Weak<Mutex<KernelSocket>>>> = Mutex::new(Vec::new());
}

fn current_net_namespace_key() -> usize {
    crate::net::core::net_namespace::current_net_namespace_key()
}

fn namespace_scoped_family(family: u16) -> bool {
    matches!(family, AF_INET | AF_INET6 | AF_NETLINK | AF_PACKET)
}

fn socket_in_net_namespace(sock: &SocketRef, namespace_key: usize, family: u16) -> bool {
    !namespace_scoped_family(family) || sock.lock().net_ns == namespace_key
}

fn first_bound_in_namespace(
    sockets: &[SocketRef],
    namespace_key: usize,
    family: u16,
) -> Option<SocketRef> {
    sockets
        .iter()
        .find(|socket| socket_in_net_namespace(socket, namespace_key, family))
        .cloned()
}

fn current_ifaddrs_snapshot() -> Vec<IfAddrRecord> {
    let key = current_net_namespace_key();
    if key == 0 {
        IFADDRS.lock().clone()
    } else {
        IFADDRS_BY_NAMESPACE
            .lock()
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }
}

fn with_current_ifaddrs<R>(f: impl FnOnce(&mut Vec<IfAddrRecord>) -> R) -> R {
    let key = current_net_namespace_key();
    if key == 0 {
        f(&mut IFADDRS.lock())
    } else {
        let mut namespaces = IFADDRS_BY_NAMESPACE.lock();
        f(namespaces.entry(key).or_default())
    }
}

fn current_routes_snapshot() -> Vec<Route4Record> {
    let key = current_net_namespace_key();
    if key == 0 {
        ROUTE4S.lock().clone()
    } else {
        ROUTE4S_BY_NAMESPACE
            .lock()
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }
}

fn with_current_routes<R>(f: impl FnOnce(&mut Vec<Route4Record>) -> R) -> R {
    let key = current_net_namespace_key();
    if key == 0 {
        f(&mut ROUTE4S.lock())
    } else {
        let mut namespaces = ROUTE4S_BY_NAMESPACE.lock();
        f(namespaces.entry(key).or_default())
    }
}

pub fn unregister_net_namespace(namespace_key: usize) {
    IFADDRS_BY_NAMESPACE.lock().remove(&namespace_key);
    ROUTE4S_BY_NAMESPACE.lock().remove(&namespace_key);
    INET_SOCKETS.lock().retain(|entry| {
        entry
            .upgrade()
            .is_some_and(|socket| socket.lock().net_ns != namespace_key)
    });
    BOUND.lock().retain(|_, sockets| {
        sockets.retain(|socket| socket.lock().net_ns != namespace_key);
        !sockets.is_empty()
    });
}

static NEXT_EPHEMERAL_PORT: AtomicU32 = AtomicU32::new(0);
const EPHEMERAL_PORT_FIRST: u16 = 32768;
const EPHEMERAL_PORT_COUNT: u32 = 28232;

static NEXT_NETLINK_AUTOBIND_PORTID: AtomicU32 = AtomicU32::new(u32::MAX);
/// Ethernet receive handoff from a vendor-built Linux net driver after its
/// NAPI/GRO path selects `GRO_NORMAL`.
pub fn receive_linux_ethernet_frame(linux_dev: *mut u8, frame: &[u8]) {
    if frame.len() < 14 {
        return;
    }
    if let Some(dev) = crate::net::device::lookup_linux_netdevice(linux_dev) {
        crate::net::device::record_rx(&dev);
        crate::net::wire::receive_frame(linux_dev, frame);
    }
}

pub fn unbind_unix_path(path: &str) {
    BOUND.lock().remove(&SockAddr::Unix(String::from(path)));
}

pub fn release_bound_socket(sock: &SocketRef) {
    BOUND.lock().retain(|_, bound| {
        bound.retain(|entry| !Arc::ptr_eq(entry, sock));
        !bound.is_empty()
    });
}

/// Test the receive-side condition while the socket lock is held.
///
/// This is the condition behind Linux's `sk_sleep(sk)` wait queue: listeners
/// become readable when an accept child is queued, data sockets when a packet
/// is queued, and streams when shutdown/disconnect makes the next read return
/// EOF.
pub fn socket_recv_ready_locked(socket: &KernelSocket) -> bool {
    if socket.state == SocketState::Listening {
        !socket.backlog.is_empty()
            || socket.shutdown & RCV_SHUTDOWN != 0
            || socket.pending_error != 0
    } else {
        !socket.recvq.is_empty()
            || socket.shutdown & RCV_SHUTDOWN != 0
            || socket.pending_error != 0
            || socket.state == SocketState::Closed
            || stream_hangup_locked(socket)
    }
}

/// Prepare an interruptible receive wait without a condition/waitqueue race.
///
/// Like `prepare_to_wait()` followed by the condition check in Linux
/// `wait_event_interruptible()`, both registration and the readiness recheck
/// happen under the socket lock used by every receive-side producer. `false`
/// means the condition won the race and the caller must retry without
/// scheduling.
///
/// # Safety
/// `task` must be the live current task for the duration of the wait.
pub unsafe fn prepare_socket_recv_wait(sock: &SocketRef, task: *mut TaskStruct) -> bool {
    let socket = sock.lock();
    if socket_recv_ready_locked(&socket) {
        return false;
    }
    unsafe {
        socket.recv_wait.prepare_to_wait(task, TASK_INTERRUPTIBLE);
    }
    true
}

/// Remove a receive waiter and restore `TASK_RUNNING`, matching
/// `finish_wait()` in the Linux wait-event loop.
///
/// # Safety
/// `task` must still identify the task passed to
/// [`prepare_socket_recv_wait`].
pub unsafe fn finish_socket_recv_wait(sock: &SocketRef, task: *mut TaskStruct) {
    let wait = sock.lock().recv_wait.clone();
    unsafe {
        wait.finish_wait(task);
    }
}

/// Wake all receive, accept, poll, and epoll waiters after a readiness change.
pub fn wake_socket_recv(sock: &SocketRef) -> usize {
    // Do not invoke persistent poll callbacks under the socket lock. epoll_ctl
    // and ready collection acquire eventpoll state before polling the socket;
    // retaining the socket lock here would create the inverse lock order.
    let wait = sock.lock().recv_wait.clone();
    wait.wake_up_all()
}

pub fn release_socket(sock: &SocketRef) {
    release_socket_inner(sock, false);
}

fn release_socket_inner(sock: &SocketRef, embrion: bool) {
    release_bound_socket(sock);

    let (peer, backlog, packets, reset_peer) = {
        let mut socket = sock.lock();
        // Linux unix_release_sock() removes the endpoint from lookup, marks it
        // closed/shutdown, clears the held peer pointer, and releases unaccepted
        // children queued on a listening socket. unix_stream_connect() takes
        // peer sock references with sock_hold(); Lupos mirrors that with Arc
        // links and breaks the opposite link here to avoid an Arc cycle.
        socket.state = SocketState::Closed;
        socket.shutdown = SHUTDOWN_MASK;
        let peer = socket.peer_socket.take();
        let reset_peer = (embrion || !socket.recvq.is_empty())
            && matches!(socket.sock_type, SOCK_STREAM | SOCK_SEQPACKET);
        let packets = socket.recvq.drain(..).collect::<Vec<_>>();
        (
            peer,
            socket.backlog.drain(..).collect::<Vec<_>>(),
            packets,
            reset_peer,
        )
    };
    wake_socket_recv(sock);

    if let Some(peer) = peer {
        {
            let mut peer_locked = peer.lock();
            if reset_peer {
                peer_locked.pending_error = ECONNRESET;
            }
            if matches!(peer_locked.sock_type, SOCK_STREAM | SOCK_SEQPACKET) {
                peer_locked.shutdown = SHUTDOWN_MASK;
            }
            if peer_locked
                .peer_socket
                .as_ref()
                .is_some_and(|linked| Arc::ptr_eq(linked, sock))
            {
                peer_locked.peer_socket = None;
            }
        }
        // unix_release_sock()/tcp close wakes the peer so a blocked read can
        // recheck the now-observable EOF condition.
        wake_socket_recv(&peer);
    }

    for packet in packets {
        drop_file_refs(packet.fds);
    }

    for queued in backlog {
        release_socket_inner(&queued, true);
    }
}

pub fn rollback_bound_socket_addr(sock: &SocketRef, addr: &SockAddr) {
    {
        let mut bound = BOUND.lock();
        let remove = if let Some(entries) = bound.get_mut(addr) {
            entries.retain(|entry| !Arc::ptr_eq(entry, sock));
            entries.is_empty()
        } else {
            false
        };
        if remove {
            bound.remove(addr);
        }
    }

    let mut socket = sock.lock();
    if socket.local.as_ref() == Some(addr) {
        socket.local = None;
        socket.state = SocketState::Created;
    }
}

pub fn socket(family: u16, sock_type: u16, protocol: u16) -> Result<SocketRef, i32> {
    match family {
        AF_INET | AF_INET6 => validate_inet_socket(sock_type, protocol)?,
        AF_UNIX => validate_unix_socket(sock_type, protocol)?,
        AF_PACKET | AF_NETLINK => {}
        _ => return Err(EAFNOSUPPORT),
    }
    let local = if family == AF_NETLINK {
        Some(SockAddr::Netlink { pid: 0, groups: 0 })
    } else {
        None
    };
    let socket = Arc::new(Mutex::new(KernelSocket {
        family,
        sock_type,
        protocol,
        net_ns: current_net_namespace_key(),
        state: SocketState::Created,
        shutdown: 0,
        pending_error: 0,
        local,
        peer: None,
        recvq: VecDeque::new(),
        backlog: VecDeque::new(),
        recv_wait: Arc::new(WaitQueueHead::new()),
        peer_socket: None,
        cred: current_peer_cred(),
        peer_cred: None,
        reuseaddr: false,
        passcred: false,
        passpidfd: false,
        passrights: false,
        timestamp_old: false,
        timestamp_new: false,
        recv_ttl: false,
        recv_pktinfo: false,
        unicast_ifindex: 0,
        bound_ifindex: 0,
        recv_timeout_ns: 0,
        send_timeout_ns: 0,
        last_rx_meta: PacketMeta::default(),
        wire_tcp: None,
    }));
    if family == AF_INET {
        let mut sockets = INET_SOCKETS.lock();
        sockets.retain(|entry| entry.strong_count() != 0);
        sockets.push(Arc::downgrade(&socket));
    }
    Ok(socket)
}

pub(crate) fn inet_socket_snapshot() -> Vec<SocketRef> {
    let namespace_key = current_net_namespace_key();
    let mut sockets = INET_SOCKETS.lock();
    let live = sockets
        .iter()
        .filter_map(Weak::upgrade)
        .filter(|socket| socket.lock().net_ns == namespace_key)
        .collect::<Vec<_>>();
    sockets.retain(|entry| entry.strong_count() != 0);
    live
}

fn validate_inet_socket(sock_type: u16, protocol: u16) -> Result<(), i32> {
    match sock_type {
        SOCK_STREAM if protocol == 0 || protocol == IPPROTO_TCP as u16 => Ok(()),
        SOCK_STREAM if protocol == IPPROTO_UDP as u16 => Err(EPROTONOSUPPORT),
        SOCK_DGRAM if protocol == 0 || protocol == IPPROTO_UDP as u16 => Ok(()),
        SOCK_DGRAM if protocol == IPPROTO_ICMP as u16 => Ok(()),
        SOCK_DGRAM if protocol == IPPROTO_TCP as u16 => Err(EPROTONOSUPPORT),
        SOCK_RAW if protocol == IPPROTO_ICMP as u16 => Ok(()),
        SOCK_STREAM | SOCK_DGRAM | SOCK_RAW => Err(EPROTONOSUPPORT),
        _ => Err(EINVAL),
    }
}

pub(crate) fn qemu_guest_ipv4() -> u32 {
    ipv4(10, 0, 2, 15)
}

pub(crate) fn qemu_dns_ipv4() -> u32 {
    ipv4(10, 0, 2, 3)
}

fn next_ephemeral_port() -> u16 {
    let next = NEXT_EPHEMERAL_PORT.fetch_add(1, Ordering::AcqRel);
    EPHEMERAL_PORT_FIRST + (next % EPHEMERAL_PORT_COUNT) as u16
}

fn inet_bind_addresses_conflict(candidate: &SockAddr, existing: &SockAddr) -> bool {
    match (candidate, existing) {
        (
            SockAddr::Inet {
                addr: candidate_addr,
                port: candidate_port,
            },
            SockAddr::Inet {
                addr: existing_addr,
                port: existing_port,
            },
        ) => {
            candidate_port == existing_port
                && (*candidate_addr == 0 || *existing_addr == 0 || candidate_addr == existing_addr)
        }
        (
            SockAddr::Inet6 {
                addr: candidate_addr,
                port: candidate_port,
            },
            SockAddr::Inet6 {
                addr: existing_addr,
                port: existing_port,
            },
        ) => {
            candidate_port == existing_port
                && (*candidate_addr == [0; 16]
                    || *existing_addr == [0; 16]
                    || candidate_addr == existing_addr)
        }
        _ => false,
    }
}

fn inet_transport_protocol(sock_type: u16, protocol: u16) -> u16 {
    match (sock_type, protocol) {
        (SOCK_STREAM, 0) => IPPROTO_TCP as u16,
        (SOCK_DGRAM, 0) => IPPROTO_UDP as u16,
        _ => protocol,
    }
}

fn inet_uses_ephemeral_port_allocator(family: u16, sock_type: u16, protocol: u16) -> bool {
    matches!(family, AF_INET | AF_INET6)
        && ((sock_type == SOCK_STREAM && (protocol == 0 || protocol == IPPROTO_TCP as u16))
            || (sock_type == SOCK_DGRAM && (protocol == 0 || protocol == IPPROTO_UDP as u16)))
}

fn bound_inet_addr_conflicts(
    bound: &BTreeMap<SockAddr, Vec<SocketRef>>,
    sock: &SocketRef,
    candidate: &SockAddr,
    reuseaddr: bool,
    family: u16,
    sock_type: u16,
    protocol: u16,
    namespace_key: usize,
) -> bool {
    let protocol = inet_transport_protocol(sock_type, protocol);
    bound.iter().any(|(bound_addr, entries)| {
        inet_bind_addresses_conflict(candidate, bound_addr)
            && entries.iter().any(|entry| {
                if Arc::ptr_eq(entry, sock) {
                    return false;
                }
                let entry = entry.lock();
                (!namespace_scoped_family(family) || entry.net_ns == namespace_key)
                    && entry.family == family
                    && entry.sock_type == sock_type
                    && inet_transport_protocol(entry.sock_type, entry.protocol) == protocol
                    && (!reuseaddr || !entry.reuseaddr)
            })
    })
}

/// Resolve an explicit inet `bind(..., port 0)` before inserting the socket
/// into the bind table.
///
/// Linux `__inet_bind()` calls the protocol's `get_port(sk, 0)` unless
/// `IP_BIND_ADDRESS_NO_PORT` was requested.  The selected port is immediately
/// visible through `getsockname()` and is retained for later sends.  In
/// particular, Firefox's HTTP/3 transport binds `0.0.0.0:0`, reads the chosen
/// address back, and then hands the descriptor to Neqo.
fn resolve_inet_ephemeral_bind(
    bound: &BTreeMap<SockAddr, Vec<SocketRef>>,
    sock: &SocketRef,
    addr: SockAddr,
    reuseaddr: bool,
    family: u16,
    sock_type: u16,
    protocol: u16,
    namespace_key: usize,
) -> Result<SockAddr, i32> {
    let needs_port = matches!(
        &addr,
        SockAddr::Inet { port: 0, .. } | SockAddr::Inet6 { port: 0, .. }
    ) && inet_uses_ephemeral_port_allocator(family, sock_type, protocol);
    if !needs_port {
        return Ok(addr);
    }

    for _ in 0..EPHEMERAL_PORT_COUNT {
        let port = next_ephemeral_port();
        let candidate = match &addr {
            SockAddr::Inet { addr, .. } => SockAddr::Inet { addr: *addr, port },
            SockAddr::Inet6 { addr, .. } => SockAddr::Inet6 { addr: *addr, port },
            _ => unreachable!(),
        };
        if !bound_inet_addr_conflicts(
            bound,
            sock,
            &candidate,
            reuseaddr,
            family,
            sock_type,
            protocol,
            namespace_key,
        ) {
            return Ok(candidate);
        }
    }
    Err(EADDRINUSE)
}

pub(crate) fn autobind_inet(socket: &mut KernelSocket) {
    if socket.family == AF_INET && socket.local.is_none() {
        socket.local = Some(SockAddr::Inet {
            addr: qemu_guest_ipv4(),
            port: next_ephemeral_port(),
        });
    }
}

fn current_tgid_vnr() -> i32 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return 0;
    }
    let tgid = unsafe { (*task).tgid };
    if tgid > 0 {
        tgid
    } else {
        unsafe { (*task).pid }
    }
}

fn current_pid_ref(pid: i32) -> Option<SocketPidRef> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return None;
    }
    let kpid = unsafe { (*task).m26.thread_pid };
    if kpid.is_null() {
        return None;
    }
    unsafe {
        get_pid(&*kpid);
    }
    Some(SocketPidRef { pid, task, kpid })
}

fn pid_ref_by_vnr(pid: i32) -> Option<SocketPidRef> {
    let current = unsafe { sched::get_current() };
    if !current.is_null() {
        let current_pid = unsafe { (*current).pid };
        let current_tgid = unsafe { (*current).tgid };
        if pid == current_pid || pid == current_tgid {
            return current_pid_ref(pid);
        }
    }

    let task = {
        let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
        if !heap.is_null() {
            heap
        } else {
            sched::find_pool_task_by_pid(pid)
        }
    };
    if task.is_null() {
        return None;
    }
    let kpid = unsafe { (*task).m26.thread_pid };
    if kpid.is_null() {
        return None;
    }
    unsafe {
        get_pid(&*kpid);
    }
    Some(SocketPidRef { pid, task, kpid })
}

fn current_peer_cred() -> SocketCred {
    let pid = current_tgid_vnr();
    let cred = crate::kernel::cred::current_cred();
    if cred.is_null() {
        return SocketCred {
            pid,
            uid: 0,
            gid: 0,
            groups: GroupInfo::default(),
            pid_ref: current_pid_ref(pid),
        };
    }
    SocketCred {
        pid,
        uid: unsafe { (*cred).euid.0 },
        gid: unsafe { (*cred).egid.0 },
        groups: unsafe { (*cred).group_info },
        pid_ref: current_pid_ref(pid),
    }
}

fn current_scm_cred() -> SocketCred {
    let pid = current_tgid_vnr();
    let cred = crate::kernel::cred::current_cred();
    if cred.is_null() {
        return SocketCred {
            pid,
            uid: 0,
            gid: 0,
            groups: GroupInfo::default(),
            pid_ref: current_pid_ref(pid),
        };
    }
    SocketCred {
        pid,
        uid: unsafe { (*cred).uid.0 },
        gid: unsafe { (*cred).gid.0 },
        groups: unsafe { (*cred).group_info },
        pid_ref: current_pid_ref(pid),
    }
}

pub fn validate_unix_scm_credentials(pid: i32, uid: u32, gid: u32) -> Result<SocketCred, i32> {
    let current_pid = current_tgid_vnr();
    let cred = crate::kernel::cred::current_cred();
    if cred.is_null() {
        return Err(EINVAL);
    }
    let pid_ok = pid == current_pid || capable(crate::kernel::capability::CAP_SYS_ADMIN);
    let uid_ok = uid == unsafe { (*cred).uid.0 }
        || uid == unsafe { (*cred).euid.0 }
        || uid == unsafe { (*cred).suid.0 }
        || capable(crate::kernel::capability::CAP_SETUID);
    let gid_ok = gid == unsafe { (*cred).gid.0 }
        || gid == unsafe { (*cred).egid.0 }
        || gid == unsafe { (*cred).sgid.0 }
        || capable(crate::kernel::capability::CAP_SETGID);
    if !pid_ok || !uid_ok || !gid_ok {
        return Err(EPERM);
    }
    let pid_ref = pid_ref_by_vnr(pid).ok_or(ESRCH)?;
    Ok(SocketCred {
        pid,
        uid,
        gid,
        groups: unsafe { (*cred).group_info },
        pid_ref: Some(pid_ref),
    })
}

fn validate_unix_socket(sock_type: u16, protocol: u16) -> Result<(), i32> {
    if protocol != 0 {
        return Err(EPROTONOSUPPORT);
    }
    match sock_type {
        SOCK_STREAM | SOCK_DGRAM | SOCK_SEQPACKET => Ok(()),
        _ => Err(EINVAL),
    }
}

pub fn bind(sock: &SocketRef, addr: SockAddr) -> Result<(), i32> {
    let addr = netlink_autobind_addr(sock, addr);
    if audit_netlink_readlog_addr_requires_cap(sock, &addr) && !capable(CAP_AUDIT_READ) {
        return Err(EPERM);
    }

    let (reuseaddr, family, sock_type, protocol, namespace_key, bound_ifindex, inet_already_bound) = {
        let socket = sock.lock();
        (
            socket.reuseaddr,
            socket.family,
            socket.sock_type,
            socket.protocol,
            socket.net_ns,
            socket.bound_ifindex,
            matches!(socket.family, AF_INET | AF_INET6) && socket.local.is_some(),
        )
    };
    if inet_already_bound {
        return Err(EINVAL);
    }
    if let SockAddr::Inet { addr, .. } = &addr
        && !inet_addr_is_bindable(*addr, bound_ifindex)
    {
        return Err(EADDRNOTAVAIL);
    }
    {
        let mut bound = BOUND.lock();
        let addr = resolve_inet_ephemeral_bind(
            &bound,
            sock,
            addr,
            reuseaddr,
            family,
            sock_type,
            protocol,
            namespace_key,
        )?;
        if matches!(addr, SockAddr::Inet { .. } | SockAddr::Inet6 { .. }) {
            if bound_inet_addr_conflicts(
                &bound,
                sock,
                &addr,
                reuseaddr,
                family,
                sock_type,
                protocol,
                namespace_key,
            ) {
                return Err(EADDRINUSE);
            }
        } else if let Some(existing) = bound.get(&addr) {
            for entry in existing.iter().filter(|entry| !Arc::ptr_eq(entry, sock)) {
                let entry = entry.lock();
                if namespace_scoped_family(family) && entry.net_ns != namespace_key {
                    continue;
                }
                if !reuseaddr
                    || !entry.reuseaddr
                    || entry.family != family
                    || entry.sock_type != sock_type
                    || entry.protocol != protocol
                {
                    return Err(EADDRINUSE);
                }
            }
        }

        {
            let mut socket = sock.lock();
            socket.local = Some(addr.clone());
            socket.state = SocketState::Bound;
        }

        let entry = bound.entry(addr).or_insert_with(Vec::new);
        if !entry.iter().any(|bound| Arc::ptr_eq(bound, sock)) {
            entry.push(sock.clone());
        }
    }
    replay_pending_kobject_uevents(sock);
    replay_pending_audit_records(sock);
    Ok(())
}

fn netlink_autobind_addr(sock: &SocketRef, addr: SockAddr) -> SockAddr {
    let SockAddr::Netlink { pid: 0, groups } = addr else {
        return addr;
    };
    let namespace_key = sock.lock().net_ns;
    if sock.lock().family != AF_NETLINK {
        return SockAddr::Netlink { pid: 0, groups };
    }
    let preferred = current_netlink_portid();
    let preferred_in_use = BOUND
        .lock()
        .get(&SockAddr::Netlink {
            pid: preferred,
            groups,
        })
        .is_some_and(|sockets| {
            first_bound_in_namespace(sockets, namespace_key, AF_NETLINK).is_some()
        });
    if preferred != 0 && !preferred_in_use {
        return SockAddr::Netlink {
            pid: preferred,
            groups,
        };
    }
    SockAddr::Netlink {
        pid: NEXT_NETLINK_AUTOBIND_PORTID.fetch_sub(1, Ordering::AcqRel),
        groups,
    }
}

fn current_netlink_portid() -> u32 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return 0;
    }
    let tgid = unsafe { (*task).tgid };
    let pid = if tgid > 0 {
        tgid
    } else {
        unsafe { (*task).pid }
    };
    pid.max(0) as u32
}

pub fn listen(sock: &SocketRef) -> Result<(), i32> {
    let mut socket = sock.lock();
    if socket.family == AF_NETLINK {
        socket.state = SocketState::Listening;
        return Ok(());
    }
    if socket.state != SocketState::Bound || socket.sock_type == SOCK_DGRAM {
        return Err(EINVAL);
    }
    socket.state = SocketState::Listening;
    Ok(())
}

/// True when an inet destination targets the loopback network.
///
/// `ipv4()` packs big-endian, so 127.0.0.0/8 means the top byte is 127.
/// IPv6 loopback is the fixed `::1` address.
fn inet_loopback_dest(peer: &SockAddr) -> bool {
    match peer {
        SockAddr::Inet { addr, .. } => (addr >> 24) as u8 == 127,
        SockAddr::Inet6 { addr, .. } => {
            let mut loopback = [0u8; 16];
            loopback[15] = 1;
            *addr == loopback
        }
        _ => false,
    }
}

fn inet6_unscoped_link_local(peer: &SockAddr) -> bool {
    matches!(
        peer,
        SockAddr::Inet6 { addr, .. } if addr[0] == 0xfe && addr[1] & 0xc0 == 0x80
    )
}

pub fn connect(sock: &SocketRef, peer: SockAddr) -> Result<(), i32> {
    let (family, sock_type, namespace_key) = {
        let socket = sock.lock();
        (socket.family, socket.sock_type, socket.net_ns)
    };
    // connect() to the IPv6 any-address means loopback: the BSD'ism kept by
    // vendor/linux/net/ipv6/tcp_ipv6.c::tcp_v6_connect() and
    // vendor/linux/net/ipv6/datagram.c::__ip6_datagram_connect(). Do this only
    // after matching the socket family; an AF_INET socket given sockaddr_in6
    // must fail EAFNOSUPPORT rather than accidentally reaching IPv6 loopback.
    let peer = match (family, peer) {
        (AF_INET6, SockAddr::Inet6 { addr, port }) if addr == [0u8; 16] => {
            let mut loopback = [0u8; 16];
            loopback[15] = 1;
            SockAddr::Inet6 {
                addr: loopback,
                port,
            }
        }
        (AF_INET6, peer @ SockAddr::Inet6 { .. }) | (AF_INET, peer @ SockAddr::Inet { .. }) => peer,
        (AF_INET | AF_INET6, _) => return Err(EAFNOSUPPORT),
        (_, peer) => peer,
    };
    let mut listener = BOUND
        .lock()
        .get(&peer)
        .and_then(|sockets| first_bound_in_namespace(sockets, namespace_key, family));
    // Linux __inet_lookup_listener(): when no exact-address listener matches,
    // a stream connect still reaches the INADDR_ANY listener on that port.
    if listener.is_none()
        && matches!(family, AF_INET | AF_INET6)
        && sock_type == SOCK_STREAM
        && let SockAddr::Inet { port, .. } = peer
    {
        listener = BOUND
            .lock()
            .get(&SockAddr::Inet { addr: 0, port })
            .and_then(|sockets| first_bound_in_namespace(sockets, namespace_key, family));
    }
    if matches!(peer, SockAddr::Unix(_)) && listener.is_none() {
        return Err(ECONNREFUSED);
    }
    // A SYN to a closed loopback port is answered with RST in Linux
    // (vendor/linux/net/ipv4/tcp_ipv4.c::tcp_v4_send_reset), so connect()
    // fails with ECONNREFUSED.  Non-loopback destinations use the real
    // device-backed IPv4 path below.
    if matches!(family, AF_INET | AF_INET6)
        && sock_type == SOCK_STREAM
        && listener.is_none()
        && inet_loopback_dest(&peer)
    {
        return Err(ECONNREFUSED);
    }
    let stream_rendezvous = match family {
        AF_UNIX => matches!(sock_type, SOCK_STREAM | SOCK_SEQPACKET),
        // Linux inet_stream_connect() -> tcp_v4_connect(): a loopback TCP
        // connect rendezvouses with the local listener; sshd/ssh/scp over
        // 127.0.0.1 depend on this.
        AF_INET | AF_INET6 => sock_type == SOCK_STREAM,
        _ => false,
    };
    if stream_rendezvous && let Some(listener) = listener {
        let (
            listener_state,
            listener_family,
            listener_type,
            listener_protocol,
            listener_local,
            listener_passcred,
            listener_passpidfd,
            listener_passrights,
            listener_timestamp_old,
            listener_timestamp_new,
            listener_cred,
        ) = {
            let socket = listener.lock();
            (
                socket.state,
                socket.family,
                socket.sock_type,
                socket.protocol,
                socket.local.clone(),
                socket.passcred,
                socket.passpidfd,
                socket.passrights,
                socket.timestamp_old,
                socket.timestamp_new,
                socket.cred.clone(),
            )
        };
        if listener_state != SocketState::Listening {
            return Err(ECONNREFUSED);
        }

        // Linux unix_stream_connect() creates a fresh connected server-side
        // socket and queues that object on the listening socket for accept().
        // The accepted inet socket reports the address the client dialed as
        // its local address, even for an INADDR_ANY listener (Linux
        // inet_csk_accept() inherits the request-socket's destination).
        let accepted_local = if matches!(family, AF_INET | AF_INET6) {
            Some(peer.clone())
        } else {
            listener_local.clone()
        };
        let accepted = socket(listener_family, listener_type, listener_protocol)?;
        let (client_local, client_cred) = {
            let mut socket = sock.lock();
            // Linux tcp_v4_connect(): an unbound client is given an
            // ephemeral source port before the handshake.
            autobind_inet(&mut socket);
            let client_cred = current_peer_cred();
            socket.peer = Some(peer);
            socket.state = SocketState::Connected;
            socket.peer_socket = Some(accepted.clone());
            socket.peer_cred = Some(listener_cred.clone());
            (socket.local.clone(), client_cred)
        };
        #[cfg(not(test))]
        trace_proc_unix_connect(&listener_local, client_cred.clone(), listener_cred);
        {
            let mut socket = accepted.lock();
            socket.state = SocketState::Connected;
            socket.local = accepted_local;
            socket.peer = client_local;
            socket.peer_socket = Some(sock.clone());
            socket.peer_cred = Some(client_cred.clone());
            socket.passcred = listener_passcred;
            socket.passpidfd = listener_passpidfd;
            socket.passrights = listener_passrights;
            socket.timestamp_old = listener_timestamp_old;
            socket.timestamp_new = listener_timestamp_new;
        }
        {
            let mut listener_guard = listener.lock();
            if listener_guard.state != SocketState::Listening
                || listener_guard.shutdown & RCV_SHUTDOWN != 0
            {
                drop(listener_guard);
                {
                    let mut client = sock.lock();
                    client.state = SocketState::Created;
                    client.peer = None;
                    client.peer_socket = None;
                    client.peer_cred = None;
                }
                accepted.lock().peer_socket = None;
                release_socket(&accepted);
                return Err(ECONNREFUSED);
            }
            listener_guard.backlog.push_back(accepted);
        }
        wake_socket_recv(&listener);
        wake_socket_recv(sock);
        return Ok(());
    }
    // No local rendezvous matched.  Linux resolves external IPv6 destinations
    // through ip6_dst_lookup_flow()
    // (vendor/linux/net/ipv6/tcp_ipv6.c::tcp_v6_connect(),
    // vendor/linux/net/ipv6/datagram.c::__ip6_datagram_connect()); Lupos
    // carries no IPv6 routes (the QEMU network is IPv4-only), so the lookup
    // fails with ENETUNREACH before any packet is emitted.  glibc's
    // getaddrinfo() RFC 6724 probe and curl's happy-eyeballs fallback depend
    // on this errno to drop AAAA candidates and connect over IPv4 instead of
    // trusting a transport-less fake "success" that EPIPEs on the first send.
    // (Linux routes v4-mapped ::ffff:a.b.c.d through tcp_v4_connect(); no
    // Lupos userspace produces mapped addresses, so they take the same
    // unreachable path.)
    if family == AF_INET6
        && let SockAddr::Inet6 { addr, .. } = &peer
        && !inet_loopback_dest(&peer)
    {
        // With no scope-id field in SockAddr, every link-local destination is
        // unscoped. Linux ip6_dst_lookup_flow() rejects that with EINVAL.
        if addr[0] == 0xfe && addr[1] & 0xc0 == 0x80 {
            return Err(EINVAL);
        }
        return Err(ENETUNREACH);
    }
    if family == AF_INET && sock_type == SOCK_STREAM {
        let SockAddr::Inet { addr, port } = peer else {
            return Err(EAFNOSUPPORT);
        };
        crate::net::wire::tcp_connect(sock, addr, port)?;
        return Err(EINPROGRESS);
    }
    {
        let mut socket = sock.lock();
        if matches!(peer, SockAddr::Inet { .. }) {
            autobind_inet(&mut socket);
        }
        if socket.family == AF_UNIX {
            socket.peer_cred = listener.as_ref().map(|peer| peer.lock().cred.clone());
        }
        socket.peer = Some(peer);
        socket.state = SocketState::Connected;
    }
    wake_socket_recv(sock);
    Ok(())
}

#[cfg(not(test))]
fn trace_proc_unix_connect(
    listener_local: &Option<SockAddr>,
    client_cred: SocketCred,
    listener_cred: SocketCred,
) {
    if !crate::kernel::debug_trace::proc_enabled() {
        return;
    }
    let Some(SockAddr::Unix(path)) = listener_local else {
        return;
    };
    if !path.contains("resolve.hook")
        && !path.contains("/run/systemd/resolve")
        && !path.contains("dbus-system-bus")
        && !path.contains("/run/dbus/system_bus_socket")
    {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-proc-unix-connect path={} client_pid={} client_uid={} client_gid={} listener_pid={} listener_uid={} listener_gid={}",
        path,
        client_cred.pid,
        client_cred.uid,
        client_cred.gid,
        listener_cred.pid,
        listener_cred.uid,
        listener_cred.gid
    );
}

#[cfg(not(test))]
fn trace_rtnl_route(
    action: &str,
    header: &NetlinkHeader,
    route: Option<&Route4Record>,
    error: i32,
) {
    if !crate::kernel::debug_trace::netlink_enabled() {
        return;
    }
    let current = unsafe { sched::get_current() };
    let pid = if current.is_null() {
        0
    } else {
        unsafe { (*current).pid }
    };
    if let Some(route) = route {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-route action={} pid={} nlmsg_type={} seq={} flags=0x{:x} err={} dst={}.{}.{}.{}/{} table={} proto={} scope={} type={} oif={} gateway={}.{}.{}.{} prefsrc={}.{}.{}.{} priority={}",
            action,
            pid,
            header.msg_type,
            header.seq,
            header.flags,
            error,
            (route.dst >> 24) & 0xff,
            (route.dst >> 16) & 0xff,
            (route.dst >> 8) & 0xff,
            route.dst & 0xff,
            route.dst_len,
            route.table,
            route.protocol,
            route.scope,
            route.route_type,
            route.oif,
            route.gateway.unwrap_or(0) >> 24,
            (route.gateway.unwrap_or(0) >> 16) & 0xff,
            (route.gateway.unwrap_or(0) >> 8) & 0xff,
            route.gateway.unwrap_or(0) & 0xff,
            route.prefsrc.unwrap_or(0) >> 24,
            (route.prefsrc.unwrap_or(0) >> 16) & 0xff,
            (route.prefsrc.unwrap_or(0) >> 8) & 0xff,
            route.prefsrc.unwrap_or(0) & 0xff,
            route.priority.unwrap_or(0),
        );
    } else {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-route action={} pid={} nlmsg_type={} seq={} flags=0x{:x} err={}",
            action,
            pid,
            header.msg_type,
            header.seq,
            header.flags,
            error
        );
    }
}

pub fn accept4(sock: &SocketRef) -> Result<SocketRef, i32> {
    let mut socket = sock.lock();
    if socket.state != SocketState::Listening {
        return Err(EINVAL);
    }
    if socket.pending_error != 0 {
        let error = socket.pending_error;
        socket.pending_error = 0;
        return Err(error);
    }
    if socket.shutdown & RCV_SHUTDOWN != 0 {
        return Err(EINVAL);
    }
    socket.backlog.pop_front().ok_or(EAGAIN)
}

pub fn socketpair(
    family: u16,
    sock_type: u16,
    protocol: u16,
) -> Result<(SocketRef, SocketRef), i32> {
    if family != AF_UNIX {
        return Err(EAFNOSUPPORT);
    }
    let left = socket(family, sock_type, protocol)?;
    let right = socket(family, sock_type, protocol)?;
    let left_cred = left.lock().cred.clone();
    let right_cred = right.lock().cred.clone();
    {
        let mut locked = left.lock();
        locked.state = SocketState::Connected;
        locked.peer_socket = Some(right.clone());
        locked.peer_cred = Some(right_cred);
    }
    {
        let mut locked = right.lock();
        locked.state = SocketState::Connected;
        locked.peer_socket = Some(left.clone());
        locked.peer_cred = Some(left_cred);
    }
    Ok((left, right))
}

pub fn sendmsg(sock: &SocketRef, bytes: &[u8]) -> Result<usize, i32> {
    sendmsg_with_fds_meta(sock, bytes, Vec::new(), None)
}

fn drop_file_refs(fds: Vec<FileRef>) {
    for file in fds {
        fput(file);
    }
}

/// Send `bytes` plus an optional SCM_RIGHTS attachment of `fds` to whatever
/// the socket is connected to.  Mirrors Linux's `unix_dgram_sendmsg` +
/// `unix_attach_fds` shape: the file references travel with the packet and
/// are installed into the receiver on `recvmsg`.
pub fn sendmsg_with_fds(sock: &SocketRef, bytes: &[u8], fds: Vec<FileRef>) -> Result<usize, i32> {
    sendmsg_with_fds_meta(sock, bytes, fds, None)
}

pub fn sendmsg_with_fds_meta(
    sock: &SocketRef,
    bytes: &[u8],
    fds: Vec<FileRef>,
    send_meta: Option<PacketMeta>,
) -> Result<usize, i32> {
    sendmsg_with_fds_meta_cred(sock, bytes, fds, send_meta, None)
}

pub fn sendmsg_with_fds_meta_cred(
    sock: &SocketRef,
    bytes: &[u8],
    mut fds: Vec<FileRef>,
    send_meta: Option<PacketMeta>,
    send_cred: Option<SocketCred>,
) -> Result<usize, i32> {
    let cred = send_cred.unwrap_or_else(current_scm_cred);
    let (peer_socket, peer_addr, local_addr, family, sock_type, state, shutdown, namespace_key) = {
        let socket = sock.lock();
        (
            socket.peer_socket.clone(),
            socket.peer.clone(),
            socket.local.clone(),
            socket.family,
            socket.sock_type,
            socket.state,
            socket.shutdown,
            socket.net_ns,
        )
    };
    if family != AF_UNIX && !fds.is_empty() {
        drop_file_refs(core::mem::take(&mut fds));
    }
    let disconnected = if matches!(sock_type, SOCK_STREAM | SOCK_SEQPACKET) {
        EPIPE
    } else {
        ENOTCONN
    };
    if state == SocketState::Closed || shutdown & SEND_SHUTDOWN != 0 {
        drop_file_refs(fds);
        return Err(disconnected);
    }
    if let Some(peer_socket) = peer_socket {
        {
            let mut target = peer_socket.lock();
            if target.state == SocketState::Closed || target.shutdown & RCV_SHUTDOWN != 0 {
                drop_file_refs(fds);
                return Err(disconnected);
            }
            target.recvq.push_back(QueuedPacket {
                bytes: bytes.to_vec(),
                peer: local_addr,
                fds,
                cred,
                meta: PacketMeta::default(),
            });
        }
        wake_socket_recv(&peer_socket);
        return Ok(bytes.len());
    }
    if family == AF_INET && sock_type == SOCK_STREAM && state == SocketState::Connected {
        drop_file_refs(fds);
        return crate::net::wire::tcp_send(sock, bytes);
    }
    if matches!(sock_type, SOCK_STREAM | SOCK_SEQPACKET) {
        drop_file_refs(fds);
        return Err(disconnected);
    }

    let Some(peer) = peer_addr else {
        if let Some(n) = synthesize_netlink_send(sock, bytes, None, &cred) {
            drop_file_refs(fds);
            return Ok(n);
        }
        drop_file_refs(fds);
        return Err(ENOTCONN);
    };
    if let Some(n) = synthesize_netlink_send(sock, bytes, Some(&peer), &cred) {
        drop_file_refs(fds);
        return Ok(n);
    }
    // Keep the old deterministic DNS/ICMP responder solely as unit-test
    // scaffolding.  Production traffic must always reach the device-backed
    // path, which is the behavior exercised by the QEMU/vendor selftests.
    #[cfg(test)]
    if let Some(n) = synthesize_external_inet_response(sock, bytes, &peer) {
        drop_file_refs(fds);
        return Ok(n);
    }
    if let Some(result) = crate::net::wire::send_inet(sock, bytes, &peer, send_meta.as_ref()) {
        drop_file_refs(fds);
        return result;
    }
    // vendor/linux/net/ipv6/udp.c::udpv6_sendmsg() fails unreachable IPv6
    // destinations with ENETUNREACH (Lupos has no IPv6 routes); only the
    // loopback rendezvous below stays local.
    if family == AF_INET6 && matches!(peer, SockAddr::Inet6 { .. }) && !inet_loopback_dest(&peer) {
        drop_file_refs(fds);
        return Err(ENETUNREACH);
    }
    if !matches!(peer, SockAddr::Unix(_)) && !fds.is_empty() {
        drop_file_refs(core::mem::take(&mut fds));
    }
    let target = match BOUND
        .lock()
        .get(&peer)
        .and_then(|sockets| first_bound_in_namespace(sockets, namespace_key, family))
    {
        Some(target) => target,
        None => {
            drop_file_refs(fds);
            return Err(ENOTCONN);
        }
    };
    {
        let mut target_locked = target.lock();
        if target_locked.state == SocketState::Closed || target_locked.shutdown & RCV_SHUTDOWN != 0
        {
            drop_file_refs(fds);
            return Err(ENOTCONN);
        }
        target_locked.recvq.push_back(QueuedPacket {
            bytes: bytes.to_vec(),
            peer: local_addr,
            fds,
            cred,
            meta: inet_packet_meta_for_dest(&peer),
        });
    }
    wake_socket_recv(&target);
    Ok(bytes.len())
}

pub fn sendto(sock: &SocketRef, bytes: &[u8], dest: SockAddr) -> Result<usize, i32> {
    sendto_with_fds_meta(sock, bytes, dest, Vec::new(), None)
}

pub fn sendto_with_fds(
    sock: &SocketRef,
    bytes: &[u8],
    dest: SockAddr,
    fds: Vec<FileRef>,
) -> Result<usize, i32> {
    sendto_with_fds_meta(sock, bytes, dest, fds, None)
}

pub fn sendto_with_fds_meta(
    sock: &SocketRef,
    bytes: &[u8],
    dest: SockAddr,
    fds: Vec<FileRef>,
    send_meta: Option<PacketMeta>,
) -> Result<usize, i32> {
    sendto_with_fds_meta_cred(sock, bytes, dest, fds, send_meta, None)
}

pub fn sendto_with_fds_meta_cred(
    sock: &SocketRef,
    bytes: &[u8],
    dest: SockAddr,
    mut fds: Vec<FileRef>,
    send_meta: Option<PacketMeta>,
    send_cred: Option<SocketCred>,
) -> Result<usize, i32> {
    let cred = send_cred.unwrap_or_else(current_scm_cred);
    let (family, state, shutdown, namespace_key) = {
        let socket = sock.lock();
        (socket.family, socket.state, socket.shutdown, socket.net_ns)
    };
    if state == SocketState::Closed || shutdown & SEND_SHUTDOWN != 0 {
        drop_file_refs(fds);
        return Err(ENOTCONN);
    }
    if family != AF_UNIX && !fds.is_empty() {
        drop_file_refs(core::mem::take(&mut fds));
    }
    // udpv6_sendmsg() applies the same BSD `::` -> `::1` normalization as
    // connect(), but only for an AF_INET6 socket. Preserve Linux's
    // EAFNOSUPPORT for a sockaddr whose family does not match the socket.
    let dest = match (family, dest) {
        (AF_INET6, SockAddr::Inet6 { addr, port }) if addr == [0u8; 16] => {
            let mut loopback = [0u8; 16];
            loopback[15] = 1;
            SockAddr::Inet6 {
                addr: loopback,
                port,
            }
        }
        (AF_INET6, dest @ SockAddr::Inet6 { .. }) | (AF_INET, dest @ SockAddr::Inet { .. }) => dest,
        (AF_INET | AF_INET6, _) => {
            drop_file_refs(fds);
            return Err(EAFNOSUPPORT);
        }
        (_, dest) => dest,
    };
    if let Some(n) = synthesize_netlink_send(sock, bytes, Some(&dest), &cred) {
        drop_file_refs(fds);
        return Ok(n);
    }
    #[cfg(test)]
    if let Some(n) = synthesize_external_inet_response(sock, bytes, &dest) {
        drop_file_refs(fds);
        return Ok(n);
    }
    if let Some(result) = crate::net::wire::send_inet(sock, bytes, &dest, send_meta.as_ref()) {
        drop_file_refs(fds);
        return result;
    }
    // vendor/linux/net/ipv6/udp.c::udpv6_sendmsg() fails unreachable IPv6
    // destinations with ENETUNREACH (Lupos has no IPv6 routes); only the
    // loopback rendezvous below stays local.
    if family == AF_INET6 && matches!(dest, SockAddr::Inet6 { .. }) && !inet_loopback_dest(&dest) {
        drop_file_refs(fds);
        if inet6_unscoped_link_local(&dest) {
            return Err(EINVAL);
        }
        return Err(ENETUNREACH);
    }
    let target = match BOUND
        .lock()
        .get(&dest)
        .and_then(|sockets| first_bound_in_namespace(sockets, namespace_key, family))
    {
        Some(target) => target,
        None => {
            drop_file_refs(fds);
            return Err(ENOTCONN);
        }
    };
    let local = {
        let socket = sock.lock();
        if socket.state == SocketState::Closed || socket.shutdown & SEND_SHUTDOWN != 0 {
            drop_file_refs(fds);
            return Err(ENOTCONN);
        }
        socket.local.clone()
    };
    let fds = if matches!(dest, SockAddr::Unix(_)) {
        fds
    } else {
        drop_file_refs(fds);
        Vec::new()
    };
    {
        let mut target_locked = target.lock();
        if target_locked.state == SocketState::Closed || target_locked.shutdown & RCV_SHUTDOWN != 0
        {
            drop_file_refs(fds);
            return Err(ENOTCONN);
        }
        target_locked.recvq.push_back(QueuedPacket {
            bytes: bytes.to_vec(),
            peer: local,
            fds,
            cred,
            meta: inet_packet_meta_for_dest(&dest),
        });
    }
    wake_socket_recv(&target);
    Ok(bytes.len())
}

fn synthesize_netlink_send(
    sock: &SocketRef,
    bytes: &[u8],
    dest: Option<&SockAddr>,
    cred: &SocketCred,
) -> Option<usize> {
    let (family, protocol, namespace_key, sender_pid, accepts_kernel_dest) = {
        let socket = sock.lock();
        (
            socket.family,
            socket.protocol,
            socket.net_ns,
            match socket.local {
                Some(SockAddr::Netlink { pid, .. }) => pid,
                _ => 0,
            },
            dest.is_none_or(|addr| matches!(addr, SockAddr::Netlink { pid: 0, .. })),
        )
    };
    if family != AF_NETLINK {
        return None;
    }

    // A netlink sender's multicast subscriptions are not part of the source
    // address reported for a unicast datagram. Linux netlink_recvmsg() reports
    // nl_pid=NETLINK_CB(skb).portid and nl_groups=0 here. In particular,
    // systemd-udevd's manager socket subscribes to the kernel uevent group,
    // then unicasts processed devices to idle worker sockets. Propagating the
    // manager's subscription mask as the packet source makes libsystemd reject
    // the otherwise valid worker message and leaves the event running until
    // udev's three-minute timeout.
    if let Some(SockAddr::Netlink { pid, .. }) = dest
        && *pid != 0
    {
        let target = {
            let bound = BOUND.lock();
            bound
                .values()
                .flat_map(|sockets| sockets.iter())
                .find(|candidate| {
                    let socket = candidate.lock();
                    socket.net_ns == namespace_key
                        && socket.family == AF_NETLINK
                        && socket.protocol == protocol
                        && matches!(socket.local, Some(SockAddr::Netlink { pid: local, .. }) if local == *pid)
                })
                .cloned()
        }?;
        {
            target.lock().recvq.push_back(QueuedPacket {
                bytes: bytes.to_vec(),
                peer: Some(SockAddr::Netlink {
                    pid: sender_pid,
                    groups: 0,
                }),
                fds: Vec::new(),
                cred: cred.clone(),
                meta: PacketMeta::default(),
            });
        }
        wake_socket_recv(&target);
        return Some(bytes.len());
    }

    if !accepts_kernel_dest {
        return None;
    }

    match protocol {
        NETLINK_KOBJECT_UEVENT => {
            let groups = match dest {
                Some(SockAddr::Netlink { pid: 0, groups }) if *groups != 0 => *groups,
                _ => {
                    queue_netlink_error(sock, bytes, -(EOPNOTSUPP as i32));
                    return Some(bytes.len());
                }
            };
            broadcast_userspace_kobject_uevent(sock, bytes, groups, cred);
            Some(bytes.len())
        }
        NETLINK_AUDIT => {
            synthesize_audit_netlink(sock, bytes);
            Some(bytes.len())
        }
        NETLINK_ROUTE => {
            synthesize_route_netlink(sock, bytes);
            Some(bytes.len())
        }
        NETLINK_GENERIC => {
            queue_netlink_error(sock, bytes, -(EOPNOTSUPP as i32));
            Some(bytes.len())
        }
        _ => {
            queue_netlink_error(sock, bytes, -(EOPNOTSUPP as i32));
            Some(bytes.len())
        }
    }
}

fn synthesize_route_netlink(sock: &SocketRef, bytes: &[u8]) {
    let Some(header) = NetlinkHeader::parse(bytes) else {
        queue_netlink_error(sock, bytes, -(EINVAL as i32));
        return;
    };

    match header.msg_type {
        RTM_GETLINK => queue_rtnl_getlink_dump(sock, bytes, &header),
        RTM_GETADDR => queue_rtnl_getaddr_dump(sock, bytes, &header),
        RTM_GETROUTE => queue_rtnl_getroute_dump(sock, bytes, &header),
        RTM_GETNEIGH | RTM_GETNEXTHOP => queue_netlink_done(sock, &header),
        // Linux applies IFF_UP transitions under RTNL before acknowledging
        // RTM_SETLINK (and RTM_NEWLINK when it identifies an existing link).
        RTM_SETLINK => {
            let error = apply_rtnl_setlink(sock, bytes, &header).map_or_else(|errno| -errno, |_| 0);
            queue_netlink_error(sock, bytes, error);
        }
        RTM_NEWLINK => {
            let ifindex = rtnl_link_ifindex(bytes, &header).unwrap_or(0);
            let error = if ifindex == 0 {
                0
            } else {
                apply_rtnl_setlink(sock, bytes, &header).map_or_else(|errno| -errno, |_| 0)
            };
            queue_netlink_error(sock, bytes, error);
        }
        RTM_NEWADDR => {
            let error = require_rtnl_net_admin()
                .and_then(|_| apply_rtnl_newaddr(sock, bytes, &header))
                .map_or_else(|errno| -errno, |_| 0);
            queue_netlink_error(sock, bytes, error);
        }
        RTM_DELADDR => {
            let error = require_rtnl_net_admin()
                .and_then(|_| apply_rtnl_deladdr(sock, bytes, &header))
                .map_or_else(|errno| -errno, |_| 0);
            queue_netlink_error(sock, bytes, error);
        }
        RTM_NEWROUTE => {
            let error = require_rtnl_net_admin()
                .and_then(|_| apply_rtnl_newroute(sock, bytes, &header))
                .map_or_else(|errno| -errno, |_| 0);
            queue_netlink_error(sock, bytes, error);
        }
        RTM_DELROUTE => {
            let error = require_rtnl_net_admin()
                .and_then(|_| apply_rtnl_delroute(sock, bytes, &header))
                .map_or_else(|errno| -errno, |_| 0);
            queue_netlink_error(sock, bytes, error);
        }
        RTM_DELLINK => queue_netlink_error(sock, bytes, 0),
        // systemd treats qdisc and fib-rule EOPNOTSUPP as optional-kernel
        // features, matching Linux when CONFIG_NET_SCHED/CONFIG_FIB_RULES are
        // unavailable.
        RTM_GETQDISC | RTM_GETRULE => queue_netlink_error(sock, bytes, -(EOPNOTSUPP as i32)),
        _ => queue_netlink_error(sock, bytes, -(EOPNOTSUPP as i32)),
    }
}

fn require_rtnl_net_admin() -> Result<(), i32> {
    if capable(CAP_NET_ADMIN) {
        Ok(())
    } else {
        Err(EPERM)
    }
}

fn inet_packet_meta_for_dest(dest: &SockAddr) -> PacketMeta {
    match dest {
        SockAddr::Inet { addr, .. } => PacketMeta {
            local_inet_addr: Some(*addr),
            ttl: Some(64),
            ..PacketMeta::default()
        },
        _ => PacketMeta::default(),
    }
}

pub fn get_inet_mtu(sock: &SocketRef) -> Result<u32, i32> {
    let (family, peer, preferred_ifindex) = {
        let socket = sock.lock();
        (
            socket.family,
            socket.peer.clone(),
            if socket.bound_ifindex != 0 {
                socket.bound_ifindex
            } else {
                socket.unicast_ifindex
            },
        )
    };
    let dev = if preferred_ifindex != 0 {
        crate::net::device::list_netdevices()
            .into_iter()
            .find(|dev| dev.ifindex == preferred_ifindex && dev.is_up())
    } else {
        match peer {
            Some(SockAddr::Inet { addr, .. }) if addr >> 24 == 127 => {
                crate::net::device::lookup_netdevice("lo").filter(|dev| dev.is_up())
            }
            Some(SockAddr::Inet { .. } | SockAddr::Inet6 { .. }) => {
                crate::net::device::list_netdevices()
                    .into_iter()
                    .find(|dev| dev.linux_dev.is_some() && dev.is_up())
            }
            _ => None,
        }
    };
    let Some(dev) = dev else {
        return if matches!(family, AF_INET | AF_INET6) {
            if matches!(peer, Some(SockAddr::Inet { .. } | SockAddr::Inet6 { .. })) {
                Err(ENETDOWN)
            } else {
                Err(ENOTCONN)
            }
        } else {
            Err(EINVAL)
        };
    };
    match (family, peer.as_ref()) {
        (AF_INET, Some(SockAddr::Inet { .. })) | (AF_INET6, Some(SockAddr::Inet6 { .. })) => {
            Ok(dev.mtu)
        }
        (AF_INET | AF_INET6, _) => Err(ENOTCONN),
        _ => Err(EINVAL),
    }
}

fn rtnl_link_ifindex(bytes: &[u8], header: &NetlinkHeader) -> Result<u32, i32> {
    if header.len < NLMSG_HDRLEN + 16 || bytes.len() < NLMSG_HDRLEN + 16 {
        return Err(EINVAL);
    }
    let ifindex = i32::from_ne_bytes(bytes[20..24].try_into().unwrap());
    u32::try_from(ifindex).map_err(|_| EINVAL)
}

fn apply_rtnl_setlink(sock: &SocketRef, bytes: &[u8], header: &NetlinkHeader) -> Result<(), i32> {
    if !capable(CAP_NET_ADMIN) {
        return Err(EPERM);
    }
    let ifindex = rtnl_link_ifindex(bytes, header)?;
    if ifindex == 0 {
        return Err(ENODEV);
    }
    let dev = crate::net::device::list_netdevices()
        .into_iter()
        .find(|dev| dev.ifindex == ifindex)
        .ok_or(ENODEV)?;
    let previous_flags = dev.flags.load(Ordering::Acquire);
    let flags = u32::from_ne_bytes(bytes[24..28].try_into().unwrap());
    let change = u32::from_ne_bytes(bytes[28..32].try_into().unwrap());
    let operstate_attr = rtattr_u8(bytes, NLMSG_HDRLEN + 16, IFLA_OPERSTATE);
    let linkmode_attr = rtattr_u8(bytes, NLMSG_HDRLEN + 16, IFLA_LINKMODE);
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        let current = unsafe { sched::get_current() };
        let pid = if current.is_null() {
            0
        } else {
            unsafe { (*current).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-link action=setlink-request pid={} nlmsg_type={} seq={} flags=0x{:x} ifindex={} ifi_flags=0x{:x} ifi_change=0x{:x} prev_flags=0x{:x}",
            pid,
            header.msg_type,
            header.seq,
            header.flags,
            ifindex,
            flags,
            change,
            previous_flags,
        );
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-link action=setlink-attrs seq={} ifindex={} operstate_attr={} linkmode_attr={}",
            header.seq,
            ifindex,
            operstate_attr.map_or(-1, i32::from),
            linkmode_attr.map_or(-1, i32::from)
        );
    }
    // Linux `rtnl_dev_combine_flags()` keeps bugwards compatibility here:
    // `ifi_change == 0` means "treat every flag bit as specified".
    let effective_change = if change == 0 { u32::MAX } else { change };
    if effective_change & crate::net::device::IFF_UP != 0 {
        if flags & crate::net::device::IFF_UP != 0 {
            if !dev.is_up() {
                crate::net::device::set_device_up(&dev)?;
            }
        } else if dev.is_up() {
            crate::net::device::set_device_down(&dev)?;
        }
    }
    let report = rtnl_report_requested(header);
    let mut state_changed = false;
    if let Some(link_mode) = linkmode_attr {
        if dev.set_link_mode(link_mode) {
            state_changed = true;
        }
    }
    if let Some(transition) = operstate_attr {
        if dev.set_operstate_from_user(transition) {
            state_changed = true;
        }
    }
    let flags_changed = dev.flags.load(Ordering::Acquire) != previous_flags;
    // Linux notifies RTNLGRP_LINK on every effective change.  IFF_UP flag
    // transitions already multicast the kernel-origin RTM_NEWLINK from
    // set_device_up()/set_device_down() — Lupos' dev_open()/dev_close()
    // equivalents of __dev_notify_flags() -> rtmsg_ifinfo()
    // (vendor/linux/net/core/dev.c); systemd-networkd's link engine depends
    // on that multicast to observe its own RTM_SETLINK IFF_UP take effect
    // before it configures addresses, routes, and per-link DNS.  Operstate/
    // linkmode updates notify here, mirroring netdev_state_change()
    // (vendor/linux/net/core/rtnetlink.c::do_setlink); notifying for
    // flags_changed here as well would double-send what set_device_up()
    // already emitted.
    if state_changed {
        if report {
            broadcast_rtnl_newlink_for_request(sock, header, &dev);
        } else {
            broadcast_rtnl_newlink(&dev);
        }
    }
    if report && (flags_changed || state_changed) {
        enqueue_rtnl_newlink_requester(sock, header, &dev);
    }
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-link action=setlink-result seq={} ifindex={} new_flags=0x{:x}",
            header.seq,
            ifindex,
            dev.flags.load(Ordering::Acquire)
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct NetlinkHeader {
    len: usize,
    msg_type: u16,
    flags: u16,
    seq: u32,
    pid: u32,
}

impl NetlinkHeader {
    fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < NLMSG_HDRLEN {
            return None;
        }
        let len = u32::from_ne_bytes(bytes[0..4].try_into().ok()?) as usize;
        if len < NLMSG_HDRLEN || len > bytes.len() {
            return None;
        }
        Some(Self {
            len,
            msg_type: u16::from_ne_bytes(bytes[4..6].try_into().ok()?),
            flags: u16::from_ne_bytes(bytes[6..8].try_into().ok()?),
            seq: u32::from_ne_bytes(bytes[8..12].try_into().ok()?),
            pid: u32::from_ne_bytes(bytes[12..16].try_into().ok()?),
        })
    }
}

const NLMSG_HDRLEN: usize = 16;
const NLMSG_ERROR: u16 = 2;
const NLMSG_DONE: u16 = 3;
const NLM_F_MULTI: u16 = 0x2;
const NLM_F_ACK: u16 = 0x4;
const NLM_F_ECHO: u16 = 0x8;
const NLM_F_DUMP: u16 = 0x300;
const NLM_F_REPLACE: u16 = 0x100;
const NLM_F_EXCL: u16 = 0x200;
const NLM_F_CREATE: u16 = 0x400;
const RTNLGRP_LINK: u32 = 1;
const RTNLGRP_IPV4_IFADDR: u32 = 5;
const RTNLGRP_IPV4_ROUTE: u32 = 7;
const AUDIT_GET: u16 = 1000;
const AUDIT_SET: u16 = 1001;
const AUDIT_USER: u16 = 1005;
const AUDIT_SIGNAL_INFO: u16 = 1010;
const AUDIT_ADD_RULE: u16 = 1011;
const AUDIT_DEL_RULE: u16 = 1012;
const AUDIT_LIST_RULES: u16 = 1013;
const AUDIT_SET_FEATURE: u16 = 1018;
const AUDIT_GET_FEATURE: u16 = 1019;
const AUDIT_KERNEL: u16 = 2000;
const AUDIT_FIRST_USER_MSG: u16 = 1100;
const AUDIT_LAST_USER_MSG: u16 = 1199;
const AUDIT_FIRST_USER_MSG2: u16 = 2100;
const AUDIT_LAST_USER_MSG2: u16 = 2999;
const AUDIT_NLGRP_READLOG: u32 = 1;
const AF_UNSPEC: u8 = 0;
const ARPHRD_ETHER: u16 = 1;
const ARPHRD_LOOPBACK: u16 = 772;
const IFLA_ADDRESS: u16 = 1;
const IFLA_BROADCAST: u16 = 2;
const IFLA_IFNAME: u16 = 3;
const IFLA_MTU: u16 = 4;
const IFLA_TXQLEN: u16 = 13;
const IFLA_OPERSTATE: u16 = 16;
const IFLA_LINKMODE: u16 = 17;
const IFLA_GROUP: u16 = 27;
const IFLA_NUM_TX_QUEUES: u16 = 31;
const IFLA_NUM_RX_QUEUES: u16 = 32;
const IFLA_CARRIER: u16 = 33;
const IFLA_MIN_MTU: u16 = 50;
const IFLA_MAX_MTU: u16 = 51;
const IFLA_PERM_ADDRESS: u16 = 54;
const IFA_ADDRESS: u16 = 1;
const IFA_LOCAL: u16 = 2;
const IFA_LABEL: u16 = 3;
const IFA_BROADCAST: u16 = 4;
const IFA_CACHEINFO: u16 = 6;
const IFA_FLAGS: u16 = 8;
const IFA_RT_PRIORITY: u16 = 9;
const IFA_PROTO: u16 = 11;
const RTA_DST: u16 = 1;
const RTA_OIF: u16 = 4;
const RTA_GATEWAY: u16 = 5;
const RTA_PRIORITY: u16 = 6;
const RTA_PREFSRC: u16 = 7;
const RTA_TABLE: u16 = 15;
const IF_OPER_UNKNOWN: u8 = crate::net::device::IF_OPER_UNKNOWN;
const IF_OPER_DOWN: u8 = crate::net::device::IF_OPER_DOWN;
const IF_OPER_DORMANT: u8 = crate::net::device::IF_OPER_DORMANT;
const IF_OPER_UP: u8 = crate::net::device::IF_OPER_UP;
const IF_LINK_MODE_DEFAULT: u8 = crate::net::device::IF_LINK_MODE_DEFAULT;
const IF_LINK_MODE_DORMANT: u8 = crate::net::device::IF_LINK_MODE_DORMANT;
const IFA_F_PERMANENT: u32 = 0x80;
const IFA_F_NOPREFIXROUTE: u32 = 0x200;
const IFA_F_DEPRECATED: u32 = 0x20;
const IFAPROT_KERNEL_LO: u8 = 1;
const RTPROT_UNSPEC: u8 = 0;
const RTPROT_KERNEL: u8 = 2;
const RTPROT_BOOT: u8 = 3;
const RT_SCOPE_UNIVERSE: u8 = 0;
const RT_SCOPE_LINK: u8 = 253;
const RT_SCOPE_HOST: u8 = 254;
const RTN_UNICAST: u8 = 1;
const RTN_LOCAL: u8 = 2;
const RTN_BROADCAST: u8 = 3;
const RTN_MULTICAST: u8 = 5;
const RTM_F_CLONED: u32 = 0x200;
const RT_TABLE_UNSPEC: u8 = 0;
const RT_TABLE_MAIN: u8 = 254;
const RT_TABLE_LOCAL: u8 = 255;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct IfaCacheInfo {
    preferred: u32,
    valid: u32,
    cstamp: u32,
    tstamp: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IfAddrRecord {
    family: u8,
    prefixlen: u8,
    flags: u32,
    scope: u8,
    ifindex: u32,
    address: u32,
    local: u32,
    broadcast: Option<u32>,
    label: String,
    proto: u8,
    rt_priority: u32,
    cacheinfo: IfaCacheInfo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Route4Record {
    dst: u32,
    dst_len: u8,
    tos: u8,
    table: u8,
    protocol: u8,
    scope: u8,
    route_type: u8,
    flags: u32,
    oif: u32,
    gateway: Option<u32>,
    priority: Option<u32>,
    prefsrc: Option<u32>,
}

fn nlmsg_align(len: usize) -> usize {
    (len + 3) & !3
}

fn queue_netlink_error(sock: &SocketRef, bytes: &[u8], error: i32) {
    const ACK_LEN: usize = NLMSG_HDRLEN + 4 + NLMSG_HDRLEN;

    let mut ack = alloc::vec![0u8; ACK_LEN];
    ack[0..4].copy_from_slice(&(ACK_LEN as u32).to_ne_bytes());
    ack[4..6].copy_from_slice(&NLMSG_ERROR.to_ne_bytes());
    ack[16..20].copy_from_slice(&error.to_ne_bytes());
    if bytes.len() >= NLMSG_HDRLEN {
        ack[8..12].copy_from_slice(&bytes[8..12]);
        // Mirror the receiver's bound portid into the ACK header.  Linux's
        // `netlink_ack` calls `__nlmsg_put(skb, NETLINK_CB(in_skb).portid,
        // ...)` — i.e. the ACK is addressed back to the sender (systemd's
        // bound netlink portid), not to the kernel.  Without this, sd_netlink_
        // process() can't match the ACK to the callback that issued the
        // request.  Ref: vendor/linux/net/netlink/af_netlink.c.
        ack[12..16].copy_from_slice(&netlink_reply_portid(sock).to_ne_bytes());
        ack[20..36].copy_from_slice(&bytes[..NLMSG_HDRLEN]);
    }

    {
        sock.lock().recvq.push_back(QueuedPacket {
            bytes: ack,
            peer: Some(SockAddr::Netlink { pid: 0, groups: 0 }),
            fds: Vec::new(),
            cred: SocketCred {
                pid: 0,
                uid: 0,
                gid: 0,
                groups: GroupInfo::default(),
                pid_ref: None,
            },
            meta: PacketMeta::default(),
        });
    }
    wake_socket_recv(sock);
}

fn rtnl_report_requested(header: &NetlinkHeader) -> bool {
    header.flags & NLM_F_ECHO != 0
}

fn queue_netlink_done(sock: &SocketRef, req: &NetlinkHeader) {
    let done_len = NLMSG_HDRLEN + 4;
    let mut done = alloc::vec![0u8; done_len];
    done[0..4].copy_from_slice(&(done_len as u32).to_ne_bytes());
    done[4..6].copy_from_slice(&NLMSG_DONE.to_ne_bytes());
    done[6..8].copy_from_slice(&NLM_F_MULTI.to_ne_bytes());
    done[8..12].copy_from_slice(&req.seq.to_ne_bytes());
    done[12..16].copy_from_slice(&netlink_reply_portid(sock).to_ne_bytes());
    done[16..20].copy_from_slice(&0i32.to_ne_bytes());
    enqueue_netlink_packet(sock, done);
}

fn queue_netlink_payload(
    sock: &SocketRef,
    msg_type: u16,
    seq: u32,
    pid: u32,
    payload: &[u8],
    flags: u16,
) {
    let len = NLMSG_HDRLEN + payload.len();
    let mut msg = alloc::vec![0u8; nlmsg_align(len)];
    msg[0..4].copy_from_slice(&(len as u32).to_ne_bytes());
    msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
    msg[6..8].copy_from_slice(&flags.to_ne_bytes());
    msg[8..12].copy_from_slice(&seq.to_ne_bytes());
    msg[12..16].copy_from_slice(&pid.to_ne_bytes());
    msg[NLMSG_HDRLEN..NLMSG_HDRLEN + payload.len()].copy_from_slice(payload);
    enqueue_netlink_packet(sock, msg);
}

fn audit_status_to_bytes(status: crate::kernel::audit::AuditStatus) -> [u8; 44] {
    let fields = [
        status.mask,
        status.enabled,
        status.failure,
        status.pid,
        status.rate_limit,
        status.backlog_limit,
        status.lost,
        status.backlog,
        status.feature_bitmap,
        status.backlog_wait_time,
        status.backlog_wait_time_actual,
    ];
    let mut out = [0u8; 44];
    for (idx, field) in fields.iter().enumerate() {
        out[idx * 4..idx * 4 + 4].copy_from_slice(&field.to_ne_bytes());
    }
    out
}

fn audit_status_from_payload(
    payload: &[u8],
    fallback_pid: u32,
) -> crate::kernel::audit::AuditStatus {
    fn field(payload: &[u8], idx: usize) -> u32 {
        let off = idx * 4;
        if payload.len() < off + 4 {
            return 0;
        }
        u32::from_ne_bytes(payload[off..off + 4].try_into().unwrap())
    }

    let mut status = crate::kernel::audit::status();
    status.mask = field(payload, 0);
    status.enabled = field(payload, 1);
    status.failure = field(payload, 2);
    status.pid = field(payload, 3);
    status.rate_limit = field(payload, 4);
    status.backlog_limit = field(payload, 5);
    status.lost = field(payload, 6);
    status.backlog = field(payload, 7);
    status.feature_bitmap = field(payload, 8);
    status.backlog_wait_time = field(payload, 9);
    status.backlog_wait_time_actual = field(payload, 10);
    if status.mask & crate::kernel::audit::AUDIT_STATUS_PID != 0 && status.pid == 0 {
        status.pid = fallback_pid;
    }
    status
}

fn audit_features_to_bytes() -> [u8; 16] {
    let fields = [1u32, 0u32, 0u32, 0u32];
    let mut out = [0u8; 16];
    for (idx, field) in fields.iter().enumerate() {
        out[idx * 4..idx * 4 + 4].copy_from_slice(&field.to_ne_bytes());
    }
    out
}

fn audit_user_payload(bytes: &[u8], header: &NetlinkHeader) -> alloc::string::String {
    let payload = &bytes[NLMSG_HDRLEN..header.len];
    let end = payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(payload.len());
    let text = core::str::from_utf8(&payload[..end]).unwrap_or("netlink-audit");
    alloc::format!("type=USER msg={}", text)
}

fn audit_ack_requested(header: &NetlinkHeader) -> bool {
    header.flags & NLM_F_ACK != 0
}

fn queue_audit_ack_if_requested(sock: &SocketRef, bytes: &[u8], header: &NetlinkHeader) {
    if audit_ack_requested(header) {
        queue_netlink_error(sock, bytes, 0);
    }
}

fn synthesize_audit_netlink(sock: &SocketRef, bytes: &[u8]) {
    let Some(header) = NetlinkHeader::parse(bytes) else {
        if capable(CAP_AUDIT_WRITE) {
            crate::kernel::audit::audit_log("type=USER msg=netlink-audit");
            queue_netlink_error(sock, bytes, 0);
        } else {
            queue_netlink_error(sock, bytes, -(EPERM as i32));
        }
        return;
    };

    if let Some(cap) = audit_netlink_required_cap(header.msg_type)
        && !capable(cap)
    {
        queue_netlink_error(sock, bytes, -(EPERM as i32));
        return;
    }

    match header.msg_type {
        AUDIT_GET => {
            let status = audit_status_to_bytes(crate::kernel::audit::status());
            queue_audit_ack_if_requested(sock, bytes, &header);
            queue_netlink_payload(
                sock,
                AUDIT_GET,
                header.seq,
                netlink_reply_portid(sock),
                &status,
                0,
            );
        }
        AUDIT_SET => {
            let payload = &bytes[NLMSG_HDRLEN..header.len];
            let fallback_pid = if header.pid != 0 {
                header.pid
            } else {
                netlink_reply_portid(sock)
            };
            crate::kernel::audit::apply_status(audit_status_from_payload(payload, fallback_pid));
            queue_audit_ack_if_requested(sock, bytes, &header);
        }
        AUDIT_GET_FEATURE => {
            let features = audit_features_to_bytes();
            queue_audit_ack_if_requested(sock, bytes, &header);
            queue_netlink_payload(
                sock,
                AUDIT_GET_FEATURE,
                header.seq,
                netlink_reply_portid(sock),
                &features,
                0,
            );
        }
        AUDIT_LIST_RULES => {
            queue_audit_ack_if_requested(sock, bytes, &header);
            queue_netlink_done(sock, &header);
        }
        AUDIT_ADD_RULE | AUDIT_DEL_RULE | AUDIT_SET_FEATURE | AUDIT_SIGNAL_INFO => {
            queue_audit_ack_if_requested(sock, bytes, &header);
        }
        AUDIT_USER
        | AUDIT_FIRST_USER_MSG..=AUDIT_LAST_USER_MSG
        | AUDIT_FIRST_USER_MSG2..=AUDIT_LAST_USER_MSG2 => {
            let text = audit_user_payload(bytes, &header);
            crate::kernel::audit::audit_log(&text);
            queue_audit_ack_if_requested(sock, bytes, &header);
        }
        _ => queue_netlink_error(sock, bytes, -(EOPNOTSUPP as i32)),
    }
}

fn queue_rtnl_getlink_dump(sock: &SocketRef, bytes: &[u8], req: &NetlinkHeader) {
    let reply_portid = netlink_reply_portid(sock);
    let ifindex = if req.len >= NLMSG_HDRLEN + 16 {
        u32::try_from(i32::from_ne_bytes(bytes[20..24].try_into().unwrap()))
            .ok()
            .unwrap_or(0)
    } else {
        0
    };
    let ifname = if req.len >= NLMSG_HDRLEN + 16 {
        rtattr_string(&bytes[..req.len], NLMSG_HDRLEN + 16, IFLA_IFNAME)
    } else {
        None
    };
    for dev in crate::net::device::list_netdevices() {
        if ifindex != 0 && ifindex != dev.ifindex {
            continue;
        }
        if ifname.as_ref().is_some_and(|name| name != &dev.name) {
            continue;
        }
        enqueue_netlink_packet(sock, build_rtnl_link_message(req, &dev, reply_portid));
    }
    queue_netlink_done(sock, req);
}

fn build_rtnl_link_message(
    req: &NetlinkHeader,
    dev: &crate::net::device::NetDeviceRef,
    reply_portid: u32,
) -> Vec<u8> {
    let flags = dev.userspace_flags();
    let operstate = dev.userspace_operstate();
    let ifi_type = if flags & crate::net::device::IFF_LOOPBACK != 0 {
        ARPHRD_LOOPBACK
    } else {
        ARPHRD_ETHER
    };
    let mut msg = alloc::vec![0u8; NLMSG_HDRLEN + 16];
    msg[4..6].copy_from_slice(&RTM_NEWLINK.to_ne_bytes());
    msg[6..8].copy_from_slice(&NLM_F_MULTI.to_ne_bytes());
    msg[8..12].copy_from_slice(&req.seq.to_ne_bytes());
    msg[12..16].copy_from_slice(&reply_portid.to_ne_bytes());
    msg[16] = AF_UNSPEC;
    msg[18..20].copy_from_slice(&ifi_type.to_ne_bytes());
    msg[20..24].copy_from_slice(&(dev.ifindex as i32).to_ne_bytes());
    msg[24..28].copy_from_slice(&flags.to_ne_bytes());
    msg[28..32].copy_from_slice(&0u32.to_ne_bytes());

    push_rta_bytes(&mut msg, IFLA_IFNAME, dev.name.as_bytes(), true);
    push_rta_u32(&mut msg, IFLA_MTU, dev.mtu);
    push_rta_u32(&mut msg, IFLA_TXQLEN, 1000);
    push_rta_u32(&mut msg, IFLA_GROUP, 0);
    push_rta_u32(&mut msg, IFLA_NUM_TX_QUEUES, 1);
    push_rta_u32(&mut msg, IFLA_NUM_RX_QUEUES, 1);
    push_rta_u8(&mut msg, IFLA_CARRIER, u8::from(dev.carrier_ok()));
    push_rta_u8(&mut msg, IFLA_OPERSTATE, operstate);
    push_rta_u8(&mut msg, IFLA_LINKMODE, dev.link_mode());
    push_rta_bytes(&mut msg, IFLA_ADDRESS, &dev.dev_addr, false);
    push_rta_bytes(&mut msg, IFLA_BROADCAST, &[0xff; 6], false);
    push_rta_bytes(&mut msg, IFLA_PERM_ADDRESS, &dev.dev_addr, false);
    push_rta_u32(
        &mut msg,
        IFLA_MIN_MTU,
        if ifi_type == ARPHRD_LOOPBACK {
            0
        } else {
            crate::net::device::ETH_MIN_MTU
        },
    );
    push_rta_u32(
        &mut msg,
        IFLA_MAX_MTU,
        if ifi_type == ARPHRD_LOOPBACK {
            0
        } else {
            crate::net::device::ETH_MAX_MTU
        },
    );

    let len = msg.len() as u32;
    msg[0..4].copy_from_slice(&len.to_ne_bytes());
    msg
}

fn build_rtnl_link_notification(dev: &crate::net::device::NetDeviceRef) -> Vec<u8> {
    let header = NetlinkHeader {
        len: NLMSG_HDRLEN + 16,
        msg_type: RTM_NEWLINK,
        flags: 0,
        seq: 0,
        pid: 0,
    };
    let mut msg = build_rtnl_link_message(&header, dev, 0);
    msg[6..8].copy_from_slice(&0u16.to_ne_bytes());
    msg
}

fn build_rtnl_link_notification_for_request(
    sock: &SocketRef,
    header: &NetlinkHeader,
    dev: &crate::net::device::NetDeviceRef,
) -> Vec<u8> {
    let mut msg = build_rtnl_link_message(header, dev, netlink_reply_portid(sock));
    msg[6..8].copy_from_slice(&0u16.to_ne_bytes());
    msg
}

fn ifaddr_ipv4(prefixlen: u8, host: [u8; 4]) -> IfAddrRecord {
    let local = u32::from_be_bytes(host);
    IfAddrRecord {
        family: AF_INET as u8,
        prefixlen,
        flags: IFA_F_PERMANENT,
        scope: RT_SCOPE_HOST,
        ifindex: 0,
        address: local,
        local,
        broadcast: None,
        label: String::new(),
        proto: IFAPROT_KERNEL_LO,
        rt_priority: 0,
        cacheinfo: IfaCacheInfo {
            preferred: u32::MAX,
            valid: u32::MAX,
            cstamp: 0,
            tstamp: 0,
        },
    }
}

fn ensure_default_ifaddrs() {
    crate::net::device::init();
    let Some(lo) = crate::net::device::lookup_netdevice("lo") else {
        return;
    };
    // Linux creates a bare, down `lo` in a new network namespace.  The
    // namespace creator (bubblewrap here) adds 127.0.0.1 and raises the link;
    // pre-seeding it would make NLM_F_EXCL RTM_NEWADDR incorrectly fail.
    if current_net_namespace_key() != 0 {
        return;
    }
    with_current_ifaddrs(|ifaddrs| {
        if ifaddrs.iter().any(|ifa| {
            ifa.family == AF_INET as u8
                && ifa.ifindex == lo.ifindex
                && ifa.local == ipv4(127, 0, 0, 1)
        }) {
            return;
        }
        let mut loopback = ifaddr_ipv4(8, [127, 0, 0, 1]);
        loopback.ifindex = lo.ifindex;
        loopback.label = String::from("lo");
        ifaddrs.push(loopback);
    });
}

fn rtattr_payload(packet: &[u8], offset: usize, attr_type: u16) -> Option<&[u8]> {
    let mut offset = offset;
    while offset + 4 <= packet.len() {
        let rta_len = u16::from_ne_bytes(packet[offset..offset + 2].try_into().ok()?) as usize;
        let rta_type = u16::from_ne_bytes(packet[offset + 2..offset + 4].try_into().ok()?);
        if rta_len < 4 || offset.checked_add(rta_len)? > packet.len() {
            return None;
        }
        if rta_type == attr_type {
            return Some(&packet[offset + 4..offset + rta_len]);
        }
        offset = offset.checked_add(nlmsg_align(rta_len))?;
    }
    None
}

fn rtattr_u8(packet: &[u8], offset: usize, attr_type: u16) -> Option<u8> {
    let payload = rtattr_payload(packet, offset, attr_type)?;
    (payload.len() == 1).then_some(payload[0])
}

fn rtattr_u32(packet: &[u8], offset: usize, attr_type: u16) -> Option<u32> {
    let payload = rtattr_payload(packet, offset, attr_type)?;
    if payload.len() != 4 {
        return None;
    }
    Some(u32::from_ne_bytes(payload.try_into().ok()?))
}

fn rtattr_ipv4(packet: &[u8], offset: usize, attr_type: u16) -> Option<u32> {
    let payload = rtattr_payload(packet, offset, attr_type)?;
    if payload.len() != 4 {
        return None;
    }
    Some(u32::from_be_bytes(payload.try_into().ok()?))
}

fn rtattr_string(packet: &[u8], offset: usize, attr_type: u16) -> Option<String> {
    let payload = rtattr_payload(packet, offset, attr_type)?;
    let end = payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(payload.len());
    Some(String::from(core::str::from_utf8(&payload[..end]).ok()?))
}

fn rtattr_ifa_cacheinfo(packet: &[u8], offset: usize) -> Result<Option<IfaCacheInfo>, i32> {
    let Some(payload) = rtattr_payload(packet, offset, IFA_CACHEINFO) else {
        return Ok(None);
    };
    if payload.len() != 16 {
        return Err(EINVAL);
    }
    let cacheinfo = IfaCacheInfo {
        preferred: u32::from_ne_bytes(payload[0..4].try_into().unwrap()),
        valid: u32::from_ne_bytes(payload[4..8].try_into().unwrap()),
        cstamp: u32::from_ne_bytes(payload[8..12].try_into().unwrap()),
        tstamp: u32::from_ne_bytes(payload[12..16].try_into().unwrap()),
    };
    if cacheinfo.valid == 0 || cacheinfo.preferred > cacheinfo.valid {
        return Err(EINVAL);
    }
    Ok(Some(cacheinfo))
}

fn push_rta_ipv4(msg: &mut Vec<u8>, rta_type: u16, value: u32) {
    push_rta_bytes(msg, rta_type, &value.to_be_bytes(), false);
}

fn ifa_cacheinfo_bytes(cacheinfo: IfaCacheInfo) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&cacheinfo.preferred.to_ne_bytes());
    out[4..8].copy_from_slice(&cacheinfo.valid.to_ne_bytes());
    out[8..12].copy_from_slice(&cacheinfo.cstamp.to_ne_bytes());
    out[12..16].copy_from_slice(&cacheinfo.tstamp.to_ne_bytes());
    out
}

fn ipv4_broadcast(local: u32, prefixlen: u8) -> Option<u32> {
    if prefixlen >= 32 {
        return Some(local);
    }
    let shift = 32u32.checked_sub(prefixlen as u32)?;
    let host_mask = if shift == 32 {
        u32::MAX
    } else {
        (1u32 << shift) - 1
    };
    Some(local | host_mask)
}

fn build_rtnl_addr_message(
    msg_type: u16,
    seq: u32,
    pid: u32,
    nl_flags: u16,
    ifaddr: &IfAddrRecord,
) -> Vec<u8> {
    let mut msg = alloc::vec![0u8; NLMSG_HDRLEN + 8];
    msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
    msg[6..8].copy_from_slice(&nl_flags.to_ne_bytes());
    msg[8..12].copy_from_slice(&seq.to_ne_bytes());
    msg[12..16].copy_from_slice(&pid.to_ne_bytes());
    msg[16] = ifaddr.family;
    msg[17] = ifaddr.prefixlen;
    msg[18] = ifaddr.flags as u8;
    msg[19] = ifaddr.scope;
    msg[20..24].copy_from_slice(&ifaddr.ifindex.to_ne_bytes());
    push_rta_ipv4(&mut msg, IFA_ADDRESS, ifaddr.address);
    push_rta_ipv4(&mut msg, IFA_LOCAL, ifaddr.local);
    if let Some(broadcast) = ifaddr.broadcast {
        push_rta_ipv4(&mut msg, IFA_BROADCAST, broadcast);
    }
    if !ifaddr.label.is_empty() {
        push_rta_bytes(&mut msg, IFA_LABEL, ifaddr.label.as_bytes(), true);
    }
    if ifaddr.proto != 0 {
        push_rta_u8(&mut msg, IFA_PROTO, ifaddr.proto);
    }
    push_rta_u32(&mut msg, IFA_FLAGS, ifaddr.flags);
    if ifaddr.rt_priority != 0 {
        push_rta_u32(&mut msg, IFA_RT_PRIORITY, ifaddr.rt_priority);
    }
    push_rta_bytes(
        &mut msg,
        IFA_CACHEINFO,
        &ifa_cacheinfo_bytes(ifaddr.cacheinfo),
        false,
    );
    let len = msg.len() as u32;
    msg[0..4].copy_from_slice(&len.to_ne_bytes());
    msg
}

fn queue_rtnl_getaddr_dump(sock: &SocketRef, bytes: &[u8], req: &NetlinkHeader) {
    ensure_default_ifaddrs();
    let family = if req.len >= NLMSG_HDRLEN + 8 {
        bytes[16]
    } else {
        AF_UNSPEC
    };
    let ifindex = if req.len >= NLMSG_HDRLEN + 8 {
        u32::from_ne_bytes(bytes[20..24].try_into().unwrap())
    } else {
        0
    };
    let reply_portid = netlink_reply_portid(sock);
    let mut ifaddrs = current_ifaddrs_snapshot();
    ifaddrs.sort_by_key(|ifa| (ifa.ifindex, ifa.local));
    for ifaddr in ifaddrs {
        if family != AF_UNSPEC && family != ifaddr.family {
            continue;
        }
        if ifindex != 0 && ifindex != ifaddr.ifindex {
            continue;
        }
        enqueue_netlink_packet(
            sock,
            build_rtnl_addr_message(RTM_NEWADDR, req.seq, reply_portid, NLM_F_MULTI, &ifaddr),
        );
    }
    queue_netlink_done(sock, req);
}

fn parse_ifaddr_request(bytes: &[u8], header: &NetlinkHeader) -> Result<IfAddrRecord, i32> {
    if header.len < NLMSG_HDRLEN + 8 || bytes.len() < NLMSG_HDRLEN + 8 {
        return Err(EINVAL);
    }
    let family = bytes[16];
    if family != AF_INET as u8 {
        return Err(EOPNOTSUPP);
    }
    let prefixlen = bytes[17];
    let flags8 = bytes[18] as u32;
    let scope = bytes[19];
    let ifindex = u32::from_ne_bytes(bytes[20..24].try_into().unwrap());
    let dev = crate::net::device::list_netdevices()
        .into_iter()
        .find(|dev| dev.ifindex == ifindex)
        .ok_or(ENODEV)?;
    let payload = &bytes[..header.len];
    let attr_offset = NLMSG_HDRLEN + 8;
    let local = rtattr_ipv4(payload, attr_offset, IFA_LOCAL)
        .or_else(|| rtattr_ipv4(payload, attr_offset, IFA_ADDRESS))
        .ok_or(EADDRNOTAVAIL)?;
    let address = rtattr_ipv4(payload, attr_offset, IFA_ADDRESS).unwrap_or(local);
    let mut flags = rtattr_u32(payload, attr_offset, IFA_FLAGS).unwrap_or(flags8);
    let cacheinfo = match rtattr_ifa_cacheinfo(payload, attr_offset)? {
        Some(cacheinfo) => {
            if cacheinfo.valid == u32::MAX && cacheinfo.preferred == u32::MAX {
                flags |= IFA_F_PERMANENT;
            } else {
                flags &= !(IFA_F_PERMANENT | IFA_F_DEPRECATED);
            }
            cacheinfo
        }
        None => {
            flags |= IFA_F_PERMANENT;
            IfaCacheInfo {
                preferred: u32::MAX,
                valid: u32::MAX,
                cstamp: 0,
                tstamp: 0,
            }
        }
    };
    let broadcast = rtattr_ipv4(payload, attr_offset, IFA_BROADCAST).or_else(|| {
        if scope == RT_SCOPE_HOST {
            None
        } else {
            ipv4_broadcast(local, prefixlen)
        }
    });
    Ok(IfAddrRecord {
        family,
        prefixlen,
        flags,
        scope,
        ifindex,
        address,
        local,
        broadcast,
        label: rtattr_string(payload, attr_offset, IFA_LABEL).unwrap_or_else(|| dev.name.clone()),
        proto: rtattr_u8(payload, attr_offset, IFA_PROTO).unwrap_or(0),
        rt_priority: rtattr_u32(payload, attr_offset, IFA_RT_PRIORITY).unwrap_or(0),
        cacheinfo,
    })
}

fn apply_rtnl_newaddr(sock: &SocketRef, bytes: &[u8], header: &NetlinkHeader) -> Result<(), i32> {
    ensure_default_ifaddrs();
    let ifaddr = parse_ifaddr_request(bytes, header)?;
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        let current = unsafe { sched::get_current() };
        let pid = if current.is_null() {
            0
        } else {
            unsafe { (*current).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-addr action=newaddr-request pid={} nlmsg_type={} seq={} flags=0x{:x} ifindex={} local={}.{}.{}.{}/{} scope={} flags_attr=0x{:x}",
            pid,
            header.msg_type,
            header.seq,
            header.flags,
            ifaddr.ifindex,
            (ifaddr.local >> 24) & 0xff,
            (ifaddr.local >> 16) & 0xff,
            (ifaddr.local >> 8) & 0xff,
            ifaddr.local & 0xff,
            ifaddr.prefixlen,
            ifaddr.scope,
            ifaddr.flags,
        );
    }
    let duplicate = with_current_ifaddrs(|ifaddrs| {
        if ifaddrs.iter().any(|current| {
            current.family == ifaddr.family
                && current.ifindex == ifaddr.ifindex
                && current.prefixlen == ifaddr.prefixlen
                && current.local == ifaddr.local
                && current.address == ifaddr.address
        }) {
            true
        } else {
            ifaddrs.push(ifaddr.clone());
            false
        }
    });
    if duplicate {
        #[cfg(not(test))]
        if crate::kernel::debug_trace::netlink_enabled() {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-netlink-addr action=newaddr-result seq={} err={}",
                header.seq,
                -EEXIST
            );
        }
        return Err(EEXIST);
    }
    if rtnl_report_requested(header) {
        enqueue_rtnl_ifaddr_requester(sock, header, RTM_NEWADDR, &ifaddr);
    }
    broadcast_rtnl_ifaddr(sock, header, RTM_NEWADDR, &ifaddr);
    // Linux's `fib_add_ifaddr()` path synthesizes these IPv4 routes from the
    // kernel, without carrying the original RTM_NEWADDR request header into
    // `rtmsg_fib()`. Mirror that shape: multicast notifications are
    // kernel-origin (`nlmsg_seq = nlmsg_pid = 0`) rather than requester echoes.
    for route in derived_route4s_for_ifaddr(&ifaddr) {
        broadcast_rtnl_route_kernel(RTM_NEWROUTE, &route);
    }
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-addr action=newaddr-result seq={} err=0",
            header.seq
        );
    }
    Ok(())
}

fn apply_rtnl_deladdr(sock: &SocketRef, bytes: &[u8], header: &NetlinkHeader) -> Result<(), i32> {
    ensure_default_ifaddrs();
    let needle = parse_ifaddr_request(bytes, header)?;
    let removed = with_current_ifaddrs(|ifaddrs| {
        let Some(index) = ifaddrs.iter().position(|current| {
            current.family == needle.family
                && current.ifindex == needle.ifindex
                && current.local == needle.local
                && current.address == needle.address
        }) else {
            return None;
        };
        Some(ifaddrs.remove(index))
    })
    .ok_or(EADDRNOTAVAIL)?;
    if rtnl_report_requested(header) {
        enqueue_rtnl_ifaddr_requester(sock, header, RTM_DELADDR, &removed);
    }
    broadcast_rtnl_ifaddr(sock, header, RTM_DELADDR, &removed);
    for route in derived_route4s_for_ifaddr(&removed) {
        broadcast_rtnl_route_kernel(RTM_DELROUTE, &route);
    }
    Ok(())
}

fn build_rtnl_addr_notification(
    sock: &SocketRef,
    header: &NetlinkHeader,
    msg_type: u16,
    ifaddr: &IfAddrRecord,
) -> Vec<u8> {
    build_rtnl_addr_message(msg_type, header.seq, netlink_reply_portid(sock), 0, ifaddr)
}

fn build_rtnl_addr_kernel_notification(msg_type: u16, ifaddr: &IfAddrRecord) -> Vec<u8> {
    build_rtnl_addr_message(msg_type, 0, 0, 0, ifaddr)
}

fn enqueue_rtnl_ifaddr_requester(
    sock: &SocketRef,
    header: &NetlinkHeader,
    msg_type: u16,
    ifaddr: &IfAddrRecord,
) {
    enqueue_netlink_packet(
        sock,
        build_rtnl_addr_notification(sock, header, msg_type, ifaddr),
    );
}

fn route_ifaddr_subscribed(socket: &KernelSocket) -> bool {
    if socket.family != AF_NETLINK || socket.protocol != NETLINK_ROUTE {
        return false;
    }
    matches!(
        socket.local,
        Some(SockAddr::Netlink { groups, .. })
            if groups & (1u32 << (RTNLGRP_IPV4_IFADDR - 1)) != 0
    )
}

fn broadcast_rtnl_ifaddr(
    sock: &SocketRef,
    _header: &NetlinkHeader,
    msg_type: u16,
    ifaddr: &IfAddrRecord,
) {
    let namespace_key = sock.lock().net_ns;
    let listeners = {
        let bound = BOUND.lock();
        bound
            .values()
            .flat_map(|sockets| sockets.iter())
            .filter(|listener| {
                let socket = listener.lock();
                socket.net_ns == namespace_key && route_ifaddr_subscribed(&socket)
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let packet = build_rtnl_addr_kernel_notification(msg_type, ifaddr);
    for listener in listeners {
        enqueue_netlink_packet_with_groups(
            &listener,
            packet.clone(),
            1u32 << (RTNLGRP_IPV4_IFADDR - 1),
        );
    }
}

pub fn drop_rtnl_ifaddrs_for_device(ifindex: u32) {
    with_current_ifaddrs(|ifaddrs| {
        ifaddrs.retain(|ifa| ifa.ifindex != ifindex || ifa.local == ipv4(127, 0, 0, 1));
    });
}

fn ipv4_prefix(local: u32, prefixlen: u8) -> u32 {
    if prefixlen == 0 {
        return 0;
    }
    let mask = u32::MAX << (32 - prefixlen as u32);
    local & mask
}

fn push_route4_unique(routes: &mut Vec<Route4Record>, route: Route4Record) {
    if routes.iter().any(|current| {
        current.dst == route.dst
            && current.dst_len == route.dst_len
            && current.table == route.table
            && current.protocol == route.protocol
            && current.scope == route.scope
            && current.route_type == route.route_type
            && current.flags == route.flags
            && current.oif == route.oif
            && current.gateway == route.gateway
            && current.priority == route.priority
            && current.prefsrc == route.prefsrc
    }) {
        return;
    }
    routes.push(route);
}

fn derived_route4s_for_ifaddr(ifaddr: &IfAddrRecord) -> Vec<Route4Record> {
    if ifaddr.family != AF_INET as u8 {
        return Vec::new();
    }
    let mut routes = Vec::new();
    let Some(dev) = crate::net::device::list_netdevices()
        .into_iter()
        .find(|dev| dev.ifindex == ifaddr.ifindex)
    else {
        return routes;
    };
    let prefix = ipv4_prefix(ifaddr.address, ifaddr.prefixlen);
    push_route4_unique(
        &mut routes,
        Route4Record {
            dst: ifaddr.local,
            dst_len: 32,
            tos: 0,
            table: RT_TABLE_LOCAL,
            protocol: RTPROT_KERNEL,
            scope: RT_SCOPE_HOST,
            route_type: RTN_LOCAL,
            flags: 0,
            oif: ifaddr.ifindex,
            gateway: None,
            priority: Some(ifaddr.rt_priority),
            prefsrc: Some(ifaddr.local),
        },
    );
    if !dev.is_up() {
        return routes;
    }
    if let Some(broadcast) = ifaddr.broadcast
        && broadcast != u32::MAX
    {
        push_route4_unique(
            &mut routes,
            Route4Record {
                dst: broadcast,
                dst_len: 32,
                tos: 0,
                table: RT_TABLE_LOCAL,
                protocol: RTPROT_KERNEL,
                scope: RT_SCOPE_LINK,
                route_type: RTN_BROADCAST,
                flags: 0,
                oif: ifaddr.ifindex,
                gateway: None,
                priority: Some(0),
                prefsrc: Some(ifaddr.local),
            },
        );
    }
    if prefix != 0 && (prefix != ifaddr.local || ifaddr.prefixlen < 32) {
        if ifaddr.flags & IFA_F_NOPREFIXROUTE == 0 {
            push_route4_unique(
                &mut routes,
                Route4Record {
                    dst: prefix,
                    dst_len: ifaddr.prefixlen,
                    tos: 0,
                    table: if dev.flags.load(Ordering::Acquire) & crate::net::device::IFF_LOOPBACK
                        != 0
                    {
                        RT_TABLE_LOCAL
                    } else {
                        RT_TABLE_MAIN
                    },
                    protocol: RTPROT_KERNEL,
                    scope: if dev.flags.load(Ordering::Acquire) & crate::net::device::IFF_LOOPBACK
                        != 0
                    {
                        RT_SCOPE_HOST
                    } else {
                        RT_SCOPE_LINK
                    },
                    route_type: if dev.flags.load(Ordering::Acquire)
                        & crate::net::device::IFF_LOOPBACK
                        != 0
                    {
                        RTN_LOCAL
                    } else {
                        RTN_UNICAST
                    },
                    flags: 0,
                    oif: ifaddr.ifindex,
                    gateway: None,
                    priority: Some(ifaddr.rt_priority),
                    prefsrc: Some(ifaddr.local),
                },
            );
        }
        if ifaddr.prefixlen < 31 {
            push_route4_unique(
                &mut routes,
                Route4Record {
                    dst: prefix | !((u32::MAX) << (32 - ifaddr.prefixlen as u32)),
                    dst_len: 32,
                    tos: 0,
                    table: RT_TABLE_LOCAL,
                    protocol: RTPROT_KERNEL,
                    scope: RT_SCOPE_LINK,
                    route_type: RTN_BROADCAST,
                    flags: 0,
                    oif: ifaddr.ifindex,
                    gateway: None,
                    priority: Some(0),
                    prefsrc: Some(ifaddr.local),
                },
            );
        }
    }
    routes
}

fn derived_route4s() -> Vec<Route4Record> {
    ensure_default_ifaddrs();
    current_ifaddrs_snapshot()
        .iter()
        .flat_map(derived_route4s_for_ifaddr)
        .collect()
}

fn current_route4s() -> Vec<Route4Record> {
    let mut routes = derived_route4s();
    routes.extend(current_routes_snapshot());
    routes.sort_by_key(|route| {
        (
            route.table,
            route.dst_len,
            route.dst,
            route.oif,
            route.gateway.unwrap_or(0),
            route.priority.unwrap_or(0),
        )
    });
    routes
}

fn route4_matches(route: &Route4Record, needle: &Route4Record) -> bool {
    route.dst == needle.dst
        && route.dst_len == needle.dst_len
        && route.table == needle.table
        && route.route_type == needle.route_type
        && route.oif == needle.oif
        && route.gateway == needle.gateway
}

fn infer_route4_oif(gateway: Option<u32>, dst: u32, dst_len: u8) -> Option<u32> {
    let probe = gateway.unwrap_or(dst);
    current_ifaddrs_snapshot()
        .iter()
        .filter(|ifa| ifa.family == AF_INET as u8)
        .find(|ifa| {
            if ifa.prefixlen == 0 {
                return false;
            }
            ipv4_prefix(probe, ifa.prefixlen) == ipv4_prefix(ifa.local, ifa.prefixlen)
                || (dst_len != 0
                    && ipv4_prefix(dst, ifa.prefixlen) == ipv4_prefix(ifa.local, ifa.prefixlen))
        })
        .map(|ifa| ifa.ifindex)
}

fn qemu_guest_addr_is_builtin_local(addr: u32, namespace_key: usize) -> bool {
    namespace_key == 0 && addr == qemu_guest_ipv4()
}

fn inet_addr_type_local_table(addr: u32) -> u8 {
    // Match vendor/linux/net/ipv4/fib_frontend.c::__inet_dev_addr_type().
    // INADDR_ANY and limited broadcast are broadcast-class addresses, while
    // multicast does not require a FIB entry.
    if addr == 0 || addr == u32::MAX {
        return RTN_BROADCAST;
    }
    if addr & 0xf000_0000 == 0xe000_0000 {
        return RTN_MULTICAST;
    }

    // The slirp address is Lupos's built-in IPv4 assignment in the init
    // network namespace even before userspace has materialized it through
    // rtnetlink. It must not leak into a newly-created namespace whose local
    // FIB and interface-address set start empty.
    if qemu_guest_addr_is_builtin_local(addr, current_net_namespace_key()) {
        return RTN_LOCAL;
    }

    // fib_table_lookup() performs a longest-prefix lookup. This matters for
    // loopback: fib_add_ifaddr() installs both 127.0.0.0/8 as RTN_LOCAL and
    // 127.255.255.255/32 as RTN_BROADCAST. An `any(RTN_LOCAL)` check would
    // incorrectly permit the latter as a packet source.
    current_route4s()
        .into_iter()
        .filter(|route| {
            route.table == RT_TABLE_LOCAL
                && route.dst_len <= 32
                && ipv4_prefix(route.dst, route.dst_len) == ipv4_prefix(addr, route.dst_len)
        })
        .max_by_key(|route| route.dst_len)
        .map_or(RTN_UNICAST, |route| route.route_type)
}

fn inet_addr_is_bindable(addr: u32, _bound_ifindex: u32) -> bool {
    // vendor/linux/include/net/inet_sock.h::inet_addr_valid_or_nonlocal().
    // Lupos does not model l3mdev-specific FIB tables, so an ordinary
    // SO_BINDTODEVICE does not narrow the local table used for this check.
    addr == 0
        || matches!(
            inet_addr_type_local_table(addr),
            RTN_LOCAL | RTN_MULTICAST | RTN_BROADCAST
        )
}

pub(crate) fn inet_addr_is_local(addr: u32, _ifindex: Option<u32>) -> bool {
    // vendor/linux/net/ipv4/route.c::ip_route_output_key_hash_rcu() validates
    // a nonzero source through __ip_dev_find(), whose fallback is an
    // RTN_LOCAL lookup in the local FIB. Linux deliberately permits a source
    // owned by another interface, so the selected output interface does not
    // narrow this lookup.
    inet_addr_type_local_table(addr) == RTN_LOCAL
}

fn route4_prefsrc_for_oif(oif: u32) -> Option<u32> {
    current_ifaddrs_snapshot()
        .iter()
        .find(|ifa| ifa.family == AF_INET as u8 && ifa.ifindex == oif)
        .map(|ifa| ifa.local)
}

fn parse_route4_request(bytes: &[u8], header: &NetlinkHeader) -> Result<Route4Record, i32> {
    if header.len < NLMSG_HDRLEN + 12 || bytes.len() < NLMSG_HDRLEN + 12 {
        return Err(EINVAL);
    }
    let family = bytes[16];
    if family != AF_INET as u8 {
        return Err(EOPNOTSUPP);
    }
    let dst_len = bytes[17];
    let table = rtattr_u32(&bytes[..header.len], NLMSG_HDRLEN + 12, RTA_TABLE)
        .and_then(|table| u8::try_from(table).ok())
        .unwrap_or(bytes[20]);
    let table = if table == RT_TABLE_UNSPEC {
        RT_TABLE_MAIN
    } else {
        table
    };
    let gateway = rtattr_ipv4(&bytes[..header.len], NLMSG_HDRLEN + 12, RTA_GATEWAY);
    let dst = rtattr_ipv4(&bytes[..header.len], NLMSG_HDRLEN + 12, RTA_DST).unwrap_or(0);
    let mut oif = rtattr_u32(&bytes[..header.len], NLMSG_HDRLEN + 12, RTA_OIF).unwrap_or(0);
    if oif == 0 {
        oif = infer_route4_oif(gateway, dst, dst_len).ok_or(EADDRNOTAVAIL)?;
    }
    let route_type = if bytes[23] == 0 {
        RTN_UNICAST
    } else {
        bytes[23]
    };
    Ok(Route4Record {
        dst,
        dst_len,
        tos: bytes[19],
        table,
        protocol: if bytes[21] == 0 {
            RTPROT_BOOT
        } else {
            bytes[21]
        },
        scope: if bytes[22] == 0 {
            RT_SCOPE_UNIVERSE
        } else {
            bytes[22]
        },
        route_type,
        flags: u32::from_ne_bytes(bytes[24..28].try_into().unwrap()),
        oif,
        gateway,
        priority: rtattr_u32(&bytes[..header.len], NLMSG_HDRLEN + 12, RTA_PRIORITY),
        prefsrc: rtattr_ipv4(&bytes[..header.len], NLMSG_HDRLEN + 12, RTA_PREFSRC)
            .or_else(|| route4_prefsrc_for_oif(oif)),
    })
}

fn build_rtnl_route_message(
    msg_type: u16,
    seq: u32,
    pid: u32,
    nl_flags: u16,
    route: &Route4Record,
) -> Vec<u8> {
    let mut msg = alloc::vec![0u8; NLMSG_HDRLEN + 12];
    msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
    msg[6..8].copy_from_slice(&nl_flags.to_ne_bytes());
    msg[8..12].copy_from_slice(&seq.to_ne_bytes());
    msg[12..16].copy_from_slice(&pid.to_ne_bytes());
    msg[16] = AF_INET as u8;
    msg[17] = route.dst_len;
    msg[18] = 0;
    msg[19] = route.tos;
    msg[20] = route.table;
    msg[21] = route.protocol;
    msg[22] = route.scope;
    msg[23] = route.route_type;
    msg[24..28].copy_from_slice(&route.flags.to_ne_bytes());
    push_rta_u32(&mut msg, RTA_TABLE, route.table.into());
    if route.dst_len != 0 {
        push_rta_ipv4(&mut msg, RTA_DST, route.dst);
    }
    push_rta_u32(&mut msg, RTA_OIF, route.oif);
    if let Some(gateway) = route.gateway {
        push_rta_ipv4(&mut msg, RTA_GATEWAY, gateway);
    }
    if let Some(priority) = route.priority
        && priority != 0
    {
        push_rta_u32(&mut msg, RTA_PRIORITY, priority);
    }
    if let Some(prefsrc) = route.prefsrc {
        push_rta_ipv4(&mut msg, RTA_PREFSRC, prefsrc);
    }
    let len = msg.len() as u32;
    msg[0..4].copy_from_slice(&len.to_ne_bytes());
    msg
}

fn queue_rtnl_getroute_lookup(sock: &SocketRef, bytes: &[u8], req: &NetlinkHeader) {
    // Linux dispatches RTM_GETROUTE without NLM_F_DUMP to
    // net/ipv4/route.c::inet_rtm_getroute(), which parses RTA_DST, performs an
    // output-route lookup, and unicasts one RTM_NEWROUTE carrying RTA_OIF.
    // Mozilla's QUIC PMTU discovery uses exactly this form and blocks in
    // read(2) until that reply arrives.
    if req.len < NLMSG_HDRLEN + 12 {
        queue_netlink_error(sock, bytes, -(EINVAL as i32));
        return;
    }
    if bytes[16] != AF_INET as u8 {
        queue_netlink_error(sock, bytes, -(EOPNOTSUPP as i32));
        return;
    }
    let dst = rtattr_ipv4(&bytes[..req.len], NLMSG_HDRLEN + 12, RTA_DST).unwrap_or(0);
    let requested_oif = rtattr_u32(&bytes[..req.len], NLMSG_HDRLEN + 12, RTA_OIF);
    let route = current_route4s()
        .into_iter()
        .filter(|route| requested_oif.is_none_or(|oif| route.oif == oif))
        .filter(|route| {
            route.dst_len <= 32
                && ipv4_prefix(dst, route.dst_len) == ipv4_prefix(route.dst, route.dst_len)
        })
        .max_by_key(|route| route.dst_len);

    let Some(route) = route else {
        queue_netlink_error(sock, bytes, -(ENETUNREACH as i32));
        return;
    };
    // inet_rtm_getroute() reports the resolved destination rather than
    // replaying the selected FIB entry. In particular, a default-route match
    // is returned as a cloned /32 route for the queried address.
    let reply = Route4Record {
        dst,
        dst_len: 32,
        protocol: RTPROT_UNSPEC,
        scope: RT_SCOPE_UNIVERSE,
        flags: RTM_F_CLONED,
        priority: None,
        ..route
    };
    enqueue_netlink_packet(
        sock,
        build_rtnl_route_message(RTM_NEWROUTE, req.seq, netlink_reply_portid(sock), 0, &reply),
    );
    if req.flags & NLM_F_ACK != 0 {
        queue_netlink_error(sock, bytes, 0);
    }
}

fn queue_rtnl_getroute_dump(sock: &SocketRef, bytes: &[u8], req: &NetlinkHeader) {
    // vendor/linux/net/core/rtnetlink.c::rtnetlink_rcv_msg selects dumpit
    // only for requests carrying NLM_F_DUMP; all other GET requests use doit.
    if req.flags & NLM_F_DUMP == 0 {
        queue_rtnl_getroute_lookup(sock, bytes, req);
        return;
    }
    let family = if req.len >= NLMSG_HDRLEN + 12 {
        bytes[16]
    } else {
        AF_UNSPEC
    };
    let table = if req.len >= NLMSG_HDRLEN + 12 {
        rtattr_u32(&bytes[..req.len], NLMSG_HDRLEN + 12, RTA_TABLE)
            .and_then(|table| u8::try_from(table).ok())
            .unwrap_or(bytes[20])
    } else {
        RT_TABLE_UNSPEC
    };
    let reply_portid = netlink_reply_portid(sock);
    for route in current_route4s() {
        if family != AF_UNSPEC && family != AF_INET as u8 {
            continue;
        }
        if table != RT_TABLE_UNSPEC && table != route.table {
            continue;
        }
        enqueue_netlink_packet(
            sock,
            build_rtnl_route_message(RTM_NEWROUTE, req.seq, reply_portid, NLM_F_MULTI, &route),
        );
    }
    queue_netlink_done(sock, req);
}

fn apply_rtnl_newroute(sock: &SocketRef, bytes: &[u8], header: &NetlinkHeader) -> Result<(), i32> {
    let route = match parse_route4_request(bytes, header) {
        Ok(route) => route,
        Err(errno) => {
            #[cfg(not(test))]
            trace_rtnl_route("newroute-parse", header, None, -errno);
            return Err(errno);
        }
    };
    let replace = header.flags & NLM_F_REPLACE != 0;
    let create = header.flags & NLM_F_CREATE != 0;
    let excl = header.flags & NLM_F_EXCL != 0;
    #[cfg(not(test))]
    trace_rtnl_route("newroute-request", header, Some(&route), 0);
    let result = with_current_routes(|routes| {
        if let Some(existing) = routes
            .iter_mut()
            .find(|current| route4_matches(current, &route))
        {
            if excl || (!replace && !create) {
                return Err(EEXIST);
            }
            *existing = route.clone();
        } else {
            routes.push(route.clone());
        }
        Ok(())
    });
    if let Err(errno) = result {
        #[cfg(not(test))]
        trace_rtnl_route("newroute-result", header, Some(&route), -errno);
        return Err(errno);
    }
    if rtnl_report_requested(header) {
        enqueue_rtnl_route_requester(sock, header, RTM_NEWROUTE, &route);
    }
    broadcast_rtnl_route(sock, header, RTM_NEWROUTE, &route);
    #[cfg(not(test))]
    trace_rtnl_route("newroute-result", header, Some(&route), 0);
    Ok(())
}

fn apply_rtnl_delroute(sock: &SocketRef, bytes: &[u8], header: &NetlinkHeader) -> Result<(), i32> {
    let route = parse_route4_request(bytes, header)?;
    let removed = with_current_routes(|routes| {
        let Some(index) = routes
            .iter()
            .position(|current| route4_matches(current, &route))
        else {
            return None;
        };
        Some(routes.remove(index))
    })
    .ok_or(EADDRNOTAVAIL)?;
    if rtnl_report_requested(header) {
        enqueue_rtnl_route_requester(sock, header, RTM_DELROUTE, &removed);
    }
    broadcast_rtnl_route(sock, header, RTM_DELROUTE, &removed);
    Ok(())
}

fn route_multicast_subscribed(socket: &KernelSocket, group: u32) -> bool {
    if socket.family != AF_NETLINK || socket.protocol != NETLINK_ROUTE {
        return false;
    }
    matches!(
        socket.local,
        Some(SockAddr::Netlink { groups, .. }) if groups & (1u32 << (group - 1)) != 0
    )
}

fn build_rtnl_route_notification(
    sock: &SocketRef,
    header: &NetlinkHeader,
    msg_type: u16,
    route: &Route4Record,
) -> Vec<u8> {
    build_rtnl_route_message(msg_type, header.seq, netlink_reply_portid(sock), 0, route)
}

fn build_rtnl_route_kernel_notification(msg_type: u16, route: &Route4Record) -> Vec<u8> {
    build_rtnl_route_message(msg_type, 0, 0, 0, route)
}

fn enqueue_rtnl_route_requester(
    sock: &SocketRef,
    header: &NetlinkHeader,
    msg_type: u16,
    route: &Route4Record,
) {
    enqueue_netlink_packet(
        sock,
        build_rtnl_route_notification(sock, header, msg_type, route),
    );
}

fn broadcast_rtnl_route(
    sock: &SocketRef,
    _header: &NetlinkHeader,
    msg_type: u16,
    route: &Route4Record,
) {
    let namespace_key = sock.lock().net_ns;
    let listeners = {
        let bound = BOUND.lock();
        bound
            .values()
            .flat_map(|sockets| sockets.iter())
            .filter(|listener| {
                let socket = listener.lock();
                socket.net_ns == namespace_key
                    && route_multicast_subscribed(&socket, RTNLGRP_IPV4_ROUTE)
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let packet = build_rtnl_route_kernel_notification(msg_type, route);
    for listener in listeners {
        enqueue_netlink_packet_with_groups(
            &listener,
            packet.clone(),
            1u32 << (RTNLGRP_IPV4_ROUTE - 1),
        );
    }
}

fn broadcast_rtnl_route_kernel(msg_type: u16, route: &Route4Record) {
    let namespace_key = current_net_namespace_key();
    let listeners = {
        let bound = BOUND.lock();
        bound
            .values()
            .flat_map(|sockets| sockets.iter())
            .filter(|listener| {
                let socket = listener.lock();
                socket.net_ns == namespace_key
                    && route_multicast_subscribed(&socket, RTNLGRP_IPV4_ROUTE)
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        let header = NetlinkHeader {
            len: NLMSG_HDRLEN + 12,
            msg_type,
            flags: 0,
            seq: 0,
            pid: 0,
        };
        trace_rtnl_route(
            "broadcast-kernel",
            &header,
            Some(route),
            listeners.len() as i32,
        );
    }
    let packet = build_rtnl_route_kernel_notification(msg_type, route);
    for listener in listeners {
        enqueue_netlink_packet_with_groups(
            &listener,
            packet.clone(),
            1u32 << (RTNLGRP_IPV4_ROUTE - 1),
        );
    }
}

pub fn drop_rtnl_routes_for_device(ifindex: u32) {
    with_current_routes(|routes| routes.retain(|route| route.oif != ifindex));
}

pub fn broadcast_rtnl_newlink(dev: &crate::net::device::NetDeviceRef) {
    let namespace_key = current_net_namespace_key();
    let listeners = {
        let bound = BOUND.lock();
        bound
            .values()
            .flat_map(|sockets| sockets.iter())
            .filter(|sock| {
                let socket = sock.lock();
                socket.net_ns == namespace_key
                    && socket.family == AF_NETLINK
                    && socket.protocol == NETLINK_ROUTE
                    && matches!(
                        socket.local,
                        Some(SockAddr::Netlink { groups, .. }) if groups & (1u32 << (RTNLGRP_LINK - 1)) != 0
                    )
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-link action=broadcast-newlink ifindex={} listeners={} flags=0x{:x} carrier={} operstate={}",
            dev.ifindex,
            listeners.len(),
            dev.userspace_flags(),
            u8::from(dev.carrier_ok()),
            dev.userspace_operstate()
        );
    }
    if listeners.is_empty() {
        return;
    }
    let packet = build_rtnl_link_notification(dev);
    for listener in listeners {
        enqueue_netlink_packet_with_groups(&listener, packet.clone(), 1u32 << (RTNLGRP_LINK - 1));
    }
}

fn link_multicast_subscribed(socket: &KernelSocket) -> bool {
    if socket.family != AF_NETLINK || socket.protocol != NETLINK_ROUTE {
        return false;
    }
    matches!(
        socket.local,
        Some(SockAddr::Netlink { groups, .. }) if groups & (1u32 << (RTNLGRP_LINK - 1)) != 0
    )
}

fn broadcast_rtnl_newlink_for_request(
    sock: &SocketRef,
    header: &NetlinkHeader,
    dev: &crate::net::device::NetDeviceRef,
) {
    let namespace_key = sock.lock().net_ns;
    let listeners = {
        let bound = BOUND.lock();
        bound
            .values()
            .flat_map(|sockets| sockets.iter())
            .filter(|listener| {
                let socket = listener.lock();
                socket.net_ns == namespace_key && link_multicast_subscribed(&socket)
            })
            .filter(|listener| !Arc::ptr_eq(listener, sock))
            .cloned()
            .collect::<Vec<_>>()
    };
    let packet = build_rtnl_link_notification_for_request(sock, header, dev);
    for listener in listeners {
        enqueue_netlink_packet_with_groups(&listener, packet.clone(), 1u32 << (RTNLGRP_LINK - 1));
    }
}

fn enqueue_rtnl_newlink_requester(
    sock: &SocketRef,
    header: &NetlinkHeader,
    dev: &crate::net::device::NetDeviceRef,
) {
    enqueue_netlink_packet(
        sock,
        build_rtnl_link_notification_for_request(sock, header, dev),
    );
}

fn netlink_reply_portid(sock: &SocketRef) -> u32 {
    match sock.lock().local.as_ref() {
        Some(SockAddr::Netlink { pid, .. }) => *pid,
        _ => 0,
    }
}

fn push_rta_u8(msg: &mut Vec<u8>, rta_type: u16, value: u8) {
    push_rta_bytes(msg, rta_type, &[value], false);
}

fn push_rta_u32(msg: &mut Vec<u8>, rta_type: u16, value: u32) {
    push_rta_bytes(msg, rta_type, &value.to_ne_bytes(), false);
}

fn push_rta_bytes(msg: &mut Vec<u8>, rta_type: u16, value: &[u8], nul: bool) {
    let payload_len = value.len() + usize::from(nul);
    let rta_len = 4 + payload_len;
    let start = msg.len();
    let aligned_len = nlmsg_align(rta_len);
    msg.resize(start + aligned_len, 0);
    msg[start..start + 2].copy_from_slice(&(rta_len as u16).to_ne_bytes());
    msg[start + 2..start + 4].copy_from_slice(&rta_type.to_ne_bytes());
    msg[start + 4..start + 4 + value.len()].copy_from_slice(value);
}

fn enqueue_netlink_packet(sock: &SocketRef, bytes: Vec<u8>) {
    enqueue_netlink_packet_with_groups(sock, bytes, 0);
}

fn netlink_dst_group(groups: u32) -> u32 {
    if groups == 0 {
        0
    } else {
        groups.trailing_zeros() + 1
    }
}

fn enqueue_netlink_packet_with_groups(sock: &SocketRef, bytes: Vec<u8>, groups: u32) {
    {
        sock.lock().recvq.push_back(QueuedPacket {
            bytes,
            peer: Some(SockAddr::Netlink { pid: 0, groups }),
            fds: Vec::new(),
            cred: SocketCred {
                pid: 0,
                uid: 0,
                gid: 0,
                groups: GroupInfo::default(),
                pid_ref: None,
            },
            meta: PacketMeta {
                netlink_group: netlink_dst_group(groups),
                ..PacketMeta::default()
            },
        });
    }
    wake_socket_recv(sock);
}

fn kobject_uevent_subscribed(socket: &KernelSocket) -> bool {
    if socket.family != AF_NETLINK || socket.protocol != NETLINK_KOBJECT_UEVENT {
        return false;
    }
    matches!(socket.local, Some(SockAddr::Netlink { groups, .. }) if groups & 1 != 0)
}

fn broadcast_userspace_kobject_uevent(
    sender: &SocketRef,
    payload: &[u8],
    groups: u32,
    cred: &SocketCred,
) {
    let (namespace_key, sender_pid) = {
        let socket = sender.lock();
        let pid = match socket.local {
            Some(SockAddr::Netlink { pid, .. }) => pid,
            _ => 0,
        };
        (socket.net_ns, pid)
    };
    let listeners = {
        let bound = BOUND.lock();
        bound
            .values()
            .flat_map(|sockets| sockets.iter())
            .filter(|listener| !Arc::ptr_eq(listener, sender))
            .filter(|listener| {
                let socket = listener.lock();
                socket.net_ns == namespace_key
                    && socket.family == AF_NETLINK
                    && socket.protocol == NETLINK_KOBJECT_UEVENT
                    && matches!(
                        socket.local,
                        Some(SockAddr::Netlink {
                            groups: subscribed,
                            ..
                        }) if subscribed & groups != 0
                    )
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    for listener in listeners {
        {
            listener.lock().recvq.push_back(QueuedPacket {
                bytes: payload.to_vec(),
                peer: Some(SockAddr::Netlink {
                    pid: sender_pid,
                    groups,
                }),
                fds: Vec::new(),
                cred: cred.clone(),
                meta: PacketMeta {
                    netlink_group: netlink_dst_group(groups),
                    ..PacketMeta::default()
                },
            });
        }
        wake_socket_recv(&listener);
    }
}

fn audit_netlink_required_cap(msg_type: u16) -> Option<u32> {
    match msg_type {
        AUDIT_SET | AUDIT_ADD_RULE | AUDIT_DEL_RULE | AUDIT_SET_FEATURE | AUDIT_SIGNAL_INFO => {
            Some(CAP_AUDIT_CONTROL)
        }
        AUDIT_USER
        | AUDIT_FIRST_USER_MSG..=AUDIT_LAST_USER_MSG
        | AUDIT_FIRST_USER_MSG2..=AUDIT_LAST_USER_MSG2 => Some(CAP_AUDIT_WRITE),
        _ => None,
    }
}

fn audit_readlog_group_mask() -> u32 {
    1u32 << (AUDIT_NLGRP_READLOG - 1)
}

fn audit_netlink_readlog_groups(groups: u32) -> bool {
    groups & audit_readlog_group_mask() != 0
}

fn audit_netlink_readlog_addr_requires_cap(sock: &SocketRef, addr: &SockAddr) -> bool {
    let SockAddr::Netlink { groups, .. } = addr else {
        return false;
    };
    if !audit_netlink_readlog_groups(*groups) {
        return false;
    }
    let socket = sock.lock();
    socket.family == AF_NETLINK && socket.protocol == NETLINK_AUDIT
}

fn audit_netlink_readlog_subscribed(socket: &KernelSocket) -> bool {
    if socket.family != AF_NETLINK || socket.protocol != NETLINK_AUDIT {
        return false;
    }
    matches!(socket.local, Some(SockAddr::Netlink { groups, .. }) if audit_netlink_readlog_groups(groups))
}

fn enqueue_kobject_uevent(sock: &SocketRef, payload: &[u8]) {
    let mut socket = sock.lock();
    socket.recvq.push_back(QueuedPacket {
        bytes: payload.to_vec(),
        peer: Some(SockAddr::Netlink { pid: 0, groups: 1 }),
        fds: Vec::new(),
        cred: SocketCred {
            pid: 0,
            uid: 0,
            gid: 0,
            groups: GroupInfo::default(),
            pid_ref: None,
        },
        // NETLINK_KOBJECT_UEVENT's kernel multicast channel is group 1.
        // systemd-udevd enables NETLINK_PKTINFO and rejects datagrams whose
        // ancillary nl_pktinfo group does not identify that channel, even
        // when sockaddr_nl.nl_groups is correct. `udevadm monitor --kernel`
        // is less strict, which can otherwise hide this mismatch.
        meta: PacketMeta {
            netlink_group: 1,
            ..PacketMeta::default()
        },
    });
    drop(socket);
    wake_socket_recv(sock);
}

fn replay_pending_kobject_uevents(sock: &SocketRef) {
    if !kobject_uevent_subscribed(&sock.lock()) {
        return;
    }
    for msg in crate::net::uevent::pending_snapshot() {
        enqueue_kobject_uevent(sock, &msg.payload);
    }
}

fn queue_audit_record(sock: &SocketRef, record: &crate::kernel::audit::AuditRecord) {
    let payload = alloc::format!("audit({}): {}\0", record.seq, record.text);
    queue_netlink_payload(
        sock,
        AUDIT_KERNEL,
        record.seq as u32,
        0,
        payload.as_bytes(),
        0,
    );
}

fn replay_pending_audit_records(sock: &SocketRef) {
    if !audit_netlink_readlog_subscribed(&sock.lock()) {
        return;
    }
    for record in crate::kernel::audit::record_snapshot() {
        queue_audit_record(sock, &record);
    }
}

pub fn broadcast_kobject_uevent(payload: &[u8]) {
    let listeners = {
        let bound = BOUND.lock();
        bound
            .values()
            .flat_map(|sockets| sockets.iter())
            .filter(|sock| kobject_uevent_subscribed(&sock.lock()))
            .cloned()
            .collect::<Vec<_>>()
    };
    for listener in listeners {
        enqueue_kobject_uevent(&listener, payload);
    }
}

pub fn broadcast_audit_record(record: &crate::kernel::audit::AuditRecord) {
    let auditd_pid = crate::kernel::audit::auditd_pid();
    let listeners = {
        let bound = BOUND.lock();
        let mut out: Vec<SocketRef> = Vec::new();
        for sock in bound.values().flat_map(|sockets| sockets.iter()) {
            let socket = sock.lock();
            let readlog = audit_netlink_readlog_subscribed(&socket);
            let auditd = auditd_pid != 0
                && socket.family == AF_NETLINK
                && socket.protocol == NETLINK_AUDIT
                && matches!(socket.local, Some(SockAddr::Netlink { pid, .. }) if pid == auditd_pid);
            drop(socket);
            if (readlog || auditd) && !out.iter().any(|existing| Arc::ptr_eq(existing, sock)) {
                out.push(sock.clone());
            }
        }
        out
    };
    for listener in listeners {
        queue_audit_record(&listener, record);
    }
}

pub fn set_netlink_membership(sock: &SocketRef, group: u32, add: bool) -> Result<(), i32> {
    if group == 0 || group > 32 {
        return Err(EINVAL);
    }
    let mask = 1u32 << (group - 1);
    if add && group == AUDIT_NLGRP_READLOG {
        let socket = sock.lock();
        if socket.family == AF_NETLINK
            && socket.protocol == NETLINK_AUDIT
            && !capable(CAP_AUDIT_READ)
        {
            return Err(EPERM);
        }
    }
    if add {
        let needs_autobind = {
            let socket = sock.lock();
            socket.family == AF_NETLINK && socket.state == SocketState::Created
        };
        if needs_autobind {
            bind(sock, SockAddr::Netlink { pid: 0, groups: 0 })?;
        }
    }
    {
        let mut socket = sock.lock();
        if socket.family != AF_NETLINK {
            return Err(EINVAL);
        }
        match socket.local {
            Some(SockAddr::Netlink { pid, groups }) => {
                let next = if add { groups | mask } else { groups & !mask };
                socket.local = Some(SockAddr::Netlink { pid, groups: next });
            }
            _ => return Err(EINVAL),
        }
    }
    if add {
        replay_pending_kobject_uevents(sock);
        replay_pending_audit_records(sock);
    }
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        let (pid, groups, protocol) = {
            let socket = sock.lock();
            match socket.local {
                Some(SockAddr::Netlink { pid, groups }) => (pid, groups, socket.protocol),
                _ => (0, 0, socket.protocol),
            }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-membership pid={} protocol={} group={} add={} groups=0x{:x}",
            pid,
            protocol,
            group,
            u8::from(add),
            groups
        );
    }
    Ok(())
}

#[cfg(test)]
fn synthesize_external_inet_response(
    sock: &SocketRef,
    bytes: &[u8],
    dest: &SockAddr,
) -> Option<usize> {
    let mut socket = sock.lock();
    if socket.family != AF_INET {
        return None;
    }

    let SockAddr::Inet { addr, port } = dest else {
        return None;
    };
    if socket.sock_type == SOCK_DGRAM && socket.protocol != IPPROTO_ICMP as u16 && *port == 53 {
        if *addr == qemu_dns_ipv4()
            && let Some(response) = build_dns_a_response(bytes)
        {
            autobind_inet(&mut socket);
            socket.recvq.push_back(QueuedPacket {
                bytes: response,
                peer: Some(dest.clone()),
                fds: Vec::new(),
                cred: SocketCred {
                    pid: 0,
                    uid: 0,
                    gid: 0,
                    groups: GroupInfo::default(),
                    pid_ref: None,
                },
                meta: inet_packet_meta_for_dest(dest),
            });
            drop(socket);
            wake_socket_recv(sock);
            return Some(bytes.len());
        }
        return None;
    }

    if matches!(socket.sock_type, SOCK_DGRAM | SOCK_RAW)
        && socket.protocol == IPPROTO_ICMP as u16
        && let Some(response) = build_icmp_echo_reply(bytes)
    {
        autobind_inet(&mut socket);
        socket.recvq.push_back(QueuedPacket {
            bytes: response,
            peer: Some(dest.clone()),
            fds: Vec::new(),
            cred: SocketCred {
                pid: 0,
                uid: 0,
                gid: 0,
                groups: GroupInfo::default(),
                pid_ref: None,
            },
            meta: inet_packet_meta_for_dest(dest),
        });
        drop(socket);
        wake_socket_recv(sock);
        return Some(bytes.len());
    }
    None
}

#[cfg(test)]
fn build_dns_a_response(query: &[u8]) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let qdcount = u16::from_be_bytes([query[4], query[5]]);
    if qdcount != 1 {
        return None;
    }
    let mut pos = 12usize;
    while pos < query.len() {
        let label_len = query[pos] as usize;
        pos += 1;
        if label_len == 0 {
            break;
        }
        if (label_len & 0xc0) != 0 || pos.checked_add(label_len)? > query.len() {
            return None;
        }
        pos += label_len;
    }
    if pos.checked_add(4)? > query.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([query[pos], query[pos + 1]]);
    let question_end = pos + 4;
    let answer_count = if qtype == 1 { 1u16 } else { 0u16 };

    let mut response = Vec::new();
    response.extend_from_slice(&query[0..2]);
    response.extend_from_slice(&0x8180u16.to_be_bytes());
    response.extend_from_slice(&qdcount.to_be_bytes());
    response.extend_from_slice(&answer_count.to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes());
    response.extend_from_slice(&query[12..question_end]);
    if answer_count != 0 {
        response.extend_from_slice(&[0xc0, 0x0c]);
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&60u32.to_be_bytes());
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&[93, 184, 216, 34]);
    }
    Some(response)
}

#[cfg(test)]
fn build_icmp_echo_reply(packet: &[u8]) -> Option<Vec<u8>> {
    if packet.len() < 8 || packet[0] != 8 {
        return None;
    }
    let mut reply = packet.to_vec();
    reply[0] = 0;
    reply[2] = 0;
    reply[3] = 0;
    let csum = checksum(&reply);
    reply[2..4].copy_from_slice(&csum.to_be_bytes());
    Some(reply)
}

pub fn recvmsg(sock: &SocketRef, out: &mut [u8]) -> Result<usize, i32> {
    recvfrom(sock, out).map(|(len, _)| len)
}

pub fn recvfrom(sock: &SocketRef, out: &mut [u8]) -> Result<(usize, Option<SockAddr>), i32> {
    let (len, peer, fds, _, _, _) = recvmsg_full(sock, out, 0)?;
    drop_file_refs(fds);
    Ok((len, peer))
}

/// Pop the next queued packet AND its SCM_RIGHTS attachment.
///
/// Returns `(bytes_copied, peer, fds)`.  The caller installs the `FileRef`s
/// into the receiving task's fdtable and serializes the fd numbers into the
/// user `msghdr.control` buffer.  Linux reference:
/// `vendor/linux/net/unix/scm.c::unix_detach_fds`.
pub fn recvmsg_with_fds(
    sock: &SocketRef,
    out: &mut [u8],
) -> Result<(usize, Option<SockAddr>, Vec<FileRef>, SocketCred), i32> {
    let (len, peer, fds, cred, _, _) = recvmsg_full(sock, out, 0)?;
    Ok((len, peer, fds, cred))
}

// recvmsg(2) flag bits per `vendor/linux/include/uapi/asm-generic/socket.h`
// and `vendor/linux/net/socket.c::sock_recvmsg`.
pub const MSG_PEEK: i32 = 0x0002;
pub const MSG_TRUNC: i32 = 0x0020;

/// Full recvmsg primitive that honours `MSG_PEEK` and `MSG_TRUNC`.
///
/// Returns `(bytes_copied, peer, fds, cred, real_packet_len)`.
///
/// * If `flags & MSG_PEEK`, the front packet is NOT removed from the recvq.
///   File-descriptor attachments are not delivered on a peek (Linux does not
///   re-deliver them on the consuming `recvmsg` either, but the safe choice
///   for our subset is to defer until the message is actually consumed).
/// * If the user buffer is shorter than a datagram, the missing bytes are
///   discarded and `real_packet_len` reports the datagram length for
///   `MSG_TRUNC`.
/// * SOCK_STREAM follows Linux `unix_stream_read_generic()`: unread bytes from
///   the front queued chunk stay available, and a recv can glue bytes from
///   later queued chunks until the caller buffer fills, the queue runs dry, a
///   credential boundary is reached, or an SCM_RIGHTS attachment is consumed.
///
/// Ref: `vendor/linux/net/socket.c::sock_recvmsg` and
/// `vendor/linux/net/netlink/af_netlink.c::netlink_recvmsg` (the MSG_PEEK
/// branch that systemd's `sd-netlink::socket_read_message` depends on).
/// True when a connected stream socket has hung up: the socket itself was
/// shut down, or the peer endpoint was closed or dropped.
///
/// Linux `tcp_recvmsg()` / `unix_stream_read_generic()` return 0 (EOF) in
/// this state instead of blocking; sshd, scp, and shell pipelines depend on
/// reads draining to EOF when the remote side closes.
///
/// Uses `try_lock` on the peer: the caller already holds this socket's lock,
/// and two concurrent receivers locking each other's peer would deadlock.  A
/// contended peer lock means the peer is alive, which is the "no hangup"
/// answer anyway.
pub fn stream_hangup_locked(socket: &KernelSocket) -> bool {
    if socket.sock_type != SOCK_STREAM && socket.sock_type != SOCK_SEQPACKET {
        return false;
    }
    if socket.state == SocketState::Closed {
        return true;
    }
    if socket.shutdown & RCV_SHUTDOWN != 0 {
        return true;
    }
    if socket.state != SocketState::Connected {
        return false;
    }
    if let Some(tcp) = &socket.wire_tcp {
        return tcp.fin_received;
    }
    match &socket.peer_socket {
        // In the in-kernel SocketRef API, the peer link can be the final Arc
        // after its caller drops the endpoint without a VFS file wrapper.
        // That is the same lifetime boundary as unix_release_sock().
        Some(peer) if Arc::strong_count(peer) == 1 => true,
        Some(peer) => peer
            .try_lock()
            .map(|peer| peer.state == SocketState::Closed)
            .unwrap_or(false),
        None => true,
    }
}

pub fn recvmsg_full(
    sock: &SocketRef,
    out: &mut [u8],
    flags: i32,
) -> Result<
    (
        usize,
        Option<SockAddr>,
        Vec<FileRef>,
        SocketCred,
        usize,
        bool,
    ),
    i32,
> {
    let mut socket = sock.lock();
    let peek = flags & MSG_PEEK != 0;
    let is_stream = socket.sock_type == SOCK_STREAM;
    if matches!(socket.sock_type, SOCK_STREAM | SOCK_SEQPACKET)
        && !matches!(socket.state, SocketState::Connected | SocketState::Closed)
    {
        return Err(ENOTCONN);
    }
    if socket.recvq.is_empty() && socket.pending_error != 0 {
        let error = socket.pending_error;
        socket.pending_error = 0;
        return Err(error);
    }
    if socket.recvq.is_empty()
        && (socket.shutdown & RCV_SHUTDOWN != 0 || stream_hangup_locked(&socket))
    {
        // EOF: queued bytes were already drained and the peer is gone.
        return Ok((0, None, Vec::new(), SocketCred::default(), 0, false));
    }
    if is_stream && out.is_empty() {
        return Ok((0, None, Vec::new(), SocketCred::default(), 0, false));
    }
    if is_stream {
        let passcred = socket.passcred;
        let mut copied = 0usize;
        let mut first_peer = None;
        let mut first_cred = SocketCred::default();
        let mut first_seen = false;
        let mut first_fds = Vec::new();
        let mut first_meta = PacketMeta::default();

        if peek {
            for msg in socket.recvq.iter() {
                if copied == out.len() {
                    break;
                }
                if passcred && first_seen && msg.cred != first_cred {
                    break;
                }
                let chunk = (out.len() - copied).min(msg.bytes.len());
                if chunk == 0 {
                    continue;
                }
                if !first_seen {
                    first_peer = msg.peer.clone();
                    first_cred = msg.cred.clone();
                    first_meta = msg.meta.clone();
                    first_seen = true;
                }
                out[copied..copied + chunk].copy_from_slice(&msg.bytes[..chunk]);
                copied += chunk;
            }
            if copied == 0 {
                return Err(EAGAIN);
            }
            socket.last_rx_meta = first_meta;
            return Ok((copied, first_peer, Vec::new(), first_cred, copied, true));
        }

        while copied < out.len() {
            let Some(mut msg) = socket.recvq.pop_front() else {
                break;
            };
            if passcred && first_seen && msg.cred != first_cred {
                socket.recvq.push_front(msg);
                break;
            }
            if first_seen && !msg.fds.is_empty() {
                socket.recvq.push_front(msg);
                break;
            }
            let chunk = (out.len() - copied).min(msg.bytes.len());
            if chunk == 0 {
                socket.recvq.push_front(msg);
                break;
            }
            if !first_seen {
                first_peer = msg.peer.clone();
                first_cred = msg.cred.clone();
                first_fds = core::mem::take(&mut msg.fds);
                first_meta = msg.meta.clone();
                first_seen = true;
            }
            out[copied..copied + chunk].copy_from_slice(&msg.bytes[..chunk]);
            copied += chunk;
            let had_fds = !first_fds.is_empty() && copied == chunk;
            if chunk < msg.bytes.len() {
                msg.bytes = msg.bytes[chunk..].to_vec();
                socket.recvq.push_front(msg);
                break;
            }
            if had_fds {
                break;
            }
        }
        if copied == 0 {
            return Err(EAGAIN);
        }
        socket.last_rx_meta = first_meta;
        return Ok((copied, first_peer, first_fds, first_cred, copied, true));
    }
    if peek {
        let msg = socket.recvq.front().ok_or(EAGAIN)?;
        let real_len = msg.bytes.len();
        let len = out.len().min(real_len);
        let meta = msg.meta.clone();
        let peer = msg.peer.clone();
        let cred = msg.cred.clone();
        out[..len].copy_from_slice(&msg.bytes[..len]);
        socket.last_rx_meta = meta;
        // On MSG_PEEK we surface the peer + creds but never duplicate the
        // SCM_RIGHTS attachment — the consuming recvmsg installs the fds.
        let reported_len = if is_stream { len } else { real_len };
        Ok((len, peer, Vec::new(), cred, reported_len, true))
    } else {
        let mut msg = socket.recvq.pop_front().ok_or(EAGAIN)?;
        let real_len = msg.bytes.len();
        let len = out.len().min(real_len);
        socket.last_rx_meta = msg.meta.clone();
        out[..len].copy_from_slice(&msg.bytes[..len]);
        let peer = msg.peer.clone();
        let cred = msg.cred.clone();
        let fds = core::mem::take(&mut msg.fds);
        if is_stream && len < real_len {
            msg.bytes = msg.bytes[len..].to_vec();
            socket.recvq.push_front(msg);
        }
        let reported_len = if is_stream { len } else { real_len };
        Ok((len, peer, fds, cred, reported_len, true))
    }
}

pub fn setsockopt(sock: &SocketRef, opt: u32, value: u32) -> Result<(), i32> {
    let mut socket = sock.lock();
    match opt {
        SO_REUSEADDR => {
            socket.reuseaddr = value != 0;
            Ok(())
        }
        SO_SNDBUF | SO_RCVBUF | SO_SNDBUFFORCE | SO_RCVBUFFORCE => Ok(()),
        SO_RCVTIMEO_OLD | SO_RCVTIMEO_NEW | SO_SNDTIMEO_OLD | SO_SNDTIMEO_NEW => Ok(()),
        SO_PASSCRED => {
            socket.passcred = value != 0;
            Ok(())
        }
        SO_PASSPIDFD => {
            if socket.family != AF_UNIX {
                return Err(EOPNOTSUPP);
            }
            socket.passpidfd = value != 0;
            Ok(())
        }
        SO_PASSRIGHTS => {
            if socket.family != AF_UNIX {
                return Err(EOPNOTSUPP);
            }
            socket.passrights = value != 0;
            Ok(())
        }
        SO_PASSSEC => Err(EOPNOTSUPP),
        SO_TIMESTAMP_OLD => {
            socket.timestamp_old = value != 0;
            if value != 0 {
                socket.timestamp_new = false;
            }
            Ok(())
        }
        SO_TIMESTAMP_NEW => {
            socket.timestamp_new = value != 0;
            if value != 0 {
                socket.timestamp_old = false;
            }
            Ok(())
        }
        // Many distro tools set socket options such as IP_RECVERR,
        // IP_TTL, ICMP_FILTER, buffer sizing, or timestamping before a simple
        // ping. The minimal Lupos socket layer does not model their behavior
        // yet, but accepting them keeps the syscall path compatible.
        _ if matches!(socket.family, AF_INET | AF_INET6 | AF_PACKET) => Ok(()),
        _ if socket.family == AF_NETLINK => Ok(()),
        _ => Err(EINVAL),
    }
}

pub fn set_recv_ttl(sock: &SocketRef, value: u32) -> Result<(), i32> {
    let mut socket = sock.lock();
    if socket.family != AF_INET {
        return Err(EINVAL);
    }
    socket.recv_ttl = value != 0;
    Ok(())
}

pub fn set_recv_pktinfo(sock: &SocketRef, value: u32) -> Result<(), i32> {
    let mut socket = sock.lock();
    if !matches!(socket.family, AF_INET | AF_INET6 | AF_NETLINK) {
        return Err(EINVAL);
    }
    socket.recv_pktinfo = value != 0;
    Ok(())
}

pub fn get_recv_pktinfo(sock: &SocketRef) -> Result<u32, i32> {
    let socket = sock.lock();
    if !matches!(socket.family, AF_INET | AF_INET6 | AF_NETLINK) {
        return Err(EINVAL);
    }
    Ok(socket.recv_pktinfo as u32)
}

pub fn set_unicast_if(sock: &SocketRef, ifindex: u32) -> Result<(), i32> {
    let mut socket = sock.lock();
    if !matches!(socket.family, AF_INET | AF_INET6) {
        return Err(EINVAL);
    }
    if ifindex != 0 {
        let dev_exists = crate::net::device::list_netdevices()
            .into_iter()
            .any(|dev| dev.ifindex == ifindex);
        if !dev_exists {
            return Err(EADDRNOTAVAIL);
        }
        if socket.bound_ifindex != 0 && socket.bound_ifindex != ifindex {
            return Err(EINVAL);
        }
    }
    socket.unicast_ifindex = ifindex;
    Ok(())
}

pub fn get_unicast_if(sock: &SocketRef) -> Result<u32, i32> {
    let socket = sock.lock();
    if !matches!(socket.family, AF_INET | AF_INET6) {
        return Err(EINVAL);
    }
    Ok(socket.unicast_ifindex)
}

pub fn set_bound_ifindex(sock: &SocketRef, ifindex: u32) -> Result<(), i32> {
    let mut socket = sock.lock();
    if !matches!(socket.family, AF_INET | AF_INET6) {
        return Err(EINVAL);
    }
    socket.bound_ifindex = ifindex;
    Ok(())
}

pub fn get_bound_ifindex(sock: &SocketRef) -> Result<u32, i32> {
    let socket = sock.lock();
    if !matches!(socket.family, AF_INET | AF_INET6) {
        return Err(EINVAL);
    }
    Ok(socket.bound_ifindex)
}

pub fn last_rx_meta(sock: &SocketRef) -> PacketMeta {
    sock.lock().last_rx_meta.clone()
}

pub fn get_recv_ttl(sock: &SocketRef) -> Result<u32, i32> {
    let socket = sock.lock();
    if socket.family != AF_INET {
        return Err(EINVAL);
    }
    Ok(socket.recv_ttl as u32)
}

pub fn getsockopt(sock: &SocketRef, opt: u32) -> Result<u32, i32> {
    let mut socket = sock.lock();
    match opt {
        SO_REUSEADDR => Ok(socket.reuseaddr as u32),
        SO_TYPE => Ok(socket.sock_type as u32),
        SO_ERROR => {
            let error = socket.pending_error;
            socket.pending_error = 0;
            Ok(error as u32)
        }
        SO_SNDBUF | SO_RCVBUF | SO_SNDBUFFORCE | SO_RCVBUFFORCE => Ok(212_992),
        SO_PASSCRED => Ok(socket.passcred as u32),
        SO_PASSPIDFD => Ok(socket.passpidfd as u32),
        SO_PASSRIGHTS if socket.family == AF_UNIX => Ok(socket.passrights as u32),
        SO_PASSRIGHTS | SO_PASSSEC => Err(EOPNOTSUPP),
        SO_TIMESTAMP_OLD => Ok(socket.timestamp_old as u32),
        SO_TIMESTAMP_NEW => Ok(socket.timestamp_new as u32),
        SO_ACCEPTCONN => Ok((socket.state == SocketState::Listening) as u32),
        SO_PEERSEC => Err(ENOPROTOOPT),
        SO_PROTOCOL => Ok(socket.protocol as u32),
        SO_DOMAIN => Ok(socket.family as u32),
        _ if socket.family == AF_PACKET => Ok(0),
        _ => Err(EINVAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use core::sync::atomic::AtomicUsize;

    use crate::include::uapi::errno::{EAFNOSUPPORT, EPERM, EPROTONOSUPPORT};
    use crate::kernel::capability::KernelCapT;
    use crate::kernel::cred::{Cred, GroupInfo, KGid, KUid, NGROUPS_MAX_INLINE};
    use crate::kernel::{sched, task::TaskStruct};
    use crate::net::fib::ipv4;
    use crate::net::ip::{IPPROTO_ICMP, IPPROTO_TCP, IPPROTO_UDP};

    fn unprivileged_cred() -> Box<Cred> {
        Box::new(Cred {
            usage: AtomicUsize::new(1),
            uid: KUid(1000),
            gid: KGid(1000),
            suid: KUid(1000),
            sgid: KGid(1000),
            euid: KUid(1000),
            egid: KGid(1000),
            fsuid: KUid(1000),
            fsgid: KGid(1000),
            cap_inheritable: KernelCapT::empty(),
            cap_permitted: KernelCapT::empty(),
            cap_effective: KernelCapT::empty(),
            cap_bset: KernelCapT::empty(),
            cap_ambient: KernelCapT::empty(),
            securebits: 0,
            group_info: GroupInfo {
                usage: 1,
                ngroups: 0,
                gid: [KGid(0); NGROUPS_MAX_INLINE],
            },
            user_ns: core::ptr::null(),
        })
    }

    #[test]
    fn linux_socket_selftest_protocol_matrix() {
        assert_eq!(socket(AF_MAX, 0, 0).err(), Some(EAFNOSUPPORT));
        assert!(socket(AF_INET, SOCK_STREAM, IPPROTO_TCP as u16).is_ok());
        assert_eq!(
            socket(AF_INET, SOCK_DGRAM, IPPROTO_TCP as u16).err(),
            Some(EPROTONOSUPPORT)
        );
        assert!(socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP as u16).is_ok());
        assert_eq!(
            socket(AF_INET, SOCK_STREAM, IPPROTO_UDP as u16).err(),
            Some(EPROTONOSUPPORT)
        );
    }

    #[test]
    fn netlink_route_getlink_dumps_loopback_then_done() {
        crate::net::device::init();
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 16];
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_GETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        req[8..12].copy_from_slice(&77u32.to_ne_bytes());

        assert_eq!(
            sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            req.len()
        );

        let mut saw_loopback = false;
        let mut saw_done = false;
        for _ in 0..16 {
            let mut out = [0u8; 512];
            let n = recvmsg(&sock, &mut out).unwrap();
            let msg_type = u16::from_ne_bytes([out[4], out[5]]);
            if msg_type == RTM_NEWLINK {
                saw_loopback |= out[..n].windows(3).any(|w| w == b"lo\0");
            } else if msg_type == NLMSG_DONE {
                saw_done = true;
                break;
            }
        }

        assert!(
            saw_loopback,
            "RTM_GETLINK dump should include Linux loopback"
        );
        assert!(saw_done, "RTM_GETLINK dump should end with NLMSG_DONE");
    }

    #[test]
    fn rtnetlink_getlink_replies_match_systemd_sd_netlink_shape() {
        crate::net::device::init();
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        let reply_portid = 0x4c55_504f;
        bind(
            &sock,
            SockAddr::Netlink {
                pid: reply_portid,
                groups: 0,
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 16];
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_GETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        req[8..12].copy_from_slice(&0x77u32.to_ne_bytes());

        assert_eq!(
            sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            req.len()
        );

        let mut saw_loopback = false;
        let mut saw_done = false;
        for _ in 0..16 {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).unwrap();
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            let msg_pid = u32::from_ne_bytes(packet[12..16].try_into().unwrap());
            assert_eq!(
                msg_pid, reply_portid,
                "sd-netlink drops unicast replies not addressed to its portid"
            );

            if msg_type == RTM_NEWLINK && attr_payload(packet, IFLA_IFNAME) == Some(&b"lo\0"[..]) {
                saw_loopback = true;
                let flags = u32::from_ne_bytes(packet[24..28].try_into().unwrap());
                assert_ne!(flags & crate::net::device::IFF_RUNNING, 0);
                assert_ne!(flags & crate::net::device::IFF_LOWER_UP, 0);
                assert_eq!(attr_u8(packet, IFLA_CARRIER), Some(1));
                assert_eq!(attr_u8(packet, IFLA_OPERSTATE), Some(IF_OPER_UNKNOWN));
                assert_eq!(attr_u8(packet, IFLA_LINKMODE), Some(IF_LINK_MODE_DEFAULT));
                assert_eq!(
                    attr_u32(packet, IFLA_MTU),
                    Some(crate::net::device::LOOPBACK_MTU)
                );
                assert_eq!(attr_u32(packet, IFLA_GROUP), Some(0));
                assert_eq!(attr_u32(packet, IFLA_TXQLEN), Some(1000));
                assert_eq!(attr_u32(packet, IFLA_NUM_TX_QUEUES), Some(1));
                assert_eq!(attr_u32(packet, IFLA_NUM_RX_QUEUES), Some(1));
                assert_eq!(attr_payload(packet, IFLA_ADDRESS), Some(&[0u8; 6][..]));
            } else if msg_type == NLMSG_DONE {
                saw_done = true;
                break;
            }
        }

        assert!(saw_loopback, "RTM_GETLINK dump should include lo");
        assert!(saw_done, "RTM_GETLINK dump should complete");
    }

    #[test]
    fn rtnetlink_getlink_honors_ifindex_filter() {
        crate::net::device::init();
        let lo = crate::net::device::lookup_netdevice("lo").expect("loopback");
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 16];
        req[0..4].copy_from_slice(&((NLMSG_HDRLEN + 16) as u32).to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_GETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        req[8..12].copy_from_slice(&78u32.to_ne_bytes());
        req[20..24].copy_from_slice(&(lo.ifindex as i32).to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send");

        let mut saw_loopback = false;
        let mut saw_other = false;
        let mut saw_done = false;
        for _ in 0..16 {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).expect("recv");
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            if msg_type == RTM_NEWLINK {
                let ifindex = u32::try_from(i32::from_ne_bytes(packet[20..24].try_into().unwrap()))
                    .expect("ifindex");
                if ifindex == lo.ifindex {
                    saw_loopback = true;
                } else {
                    saw_other = true;
                }
            } else if msg_type == NLMSG_DONE {
                saw_done = true;
                break;
            }
        }

        assert!(
            saw_loopback,
            "filtered RTM_GETLINK should include requested ifindex"
        );
        assert!(
            !saw_other,
            "filtered RTM_GETLINK should exclude other interfaces"
        );
        assert!(
            saw_done,
            "filtered RTM_GETLINK should still terminate with NLMSG_DONE"
        );
    }

    #[test]
    fn rtnetlink_getlink_honors_ifname_filter() {
        crate::net::device::init();
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 16];
        req[4..6].copy_from_slice(&RTM_GETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        req[8..12].copy_from_slice(&79u32.to_ne_bytes());
        push_rta_bytes(&mut req, IFLA_IFNAME, b"lo", true);
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send");

        let mut saw_loopback = false;
        let mut saw_other = false;
        let mut saw_done = false;
        for _ in 0..16 {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).expect("recv");
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            if msg_type == RTM_NEWLINK {
                match attr_payload(packet, IFLA_IFNAME) {
                    Some(b"lo\0") => saw_loopback = true,
                    Some(_) => saw_other = true,
                    None => {}
                }
            } else if msg_type == NLMSG_DONE {
                saw_done = true;
                break;
            }
        }

        assert!(
            saw_loopback,
            "filtered RTM_GETLINK should include requested ifname"
        );
        assert!(
            !saw_other,
            "filtered RTM_GETLINK should exclude other interface names"
        );
        assert!(
            saw_done,
            "filtered RTM_GETLINK should terminate with NLMSG_DONE"
        );
    }

    /// Source-backed parity check for the NLMSG_ERROR ACK layout that
    /// systemd's `sd_netlink_process` matches against per-call callbacks.
    /// Linux reference: `vendor/linux/net/netlink/af_netlink.c::netlink_ack`
    /// and `vendor/linux/include/uapi/linux/netlink.h::struct nlmsgerr`.
    /// The on-wire layout is:
    ///   bytes[0..4]   = nlmsg_len   (36)
    ///   bytes[4..6]   = nlmsg_type  (NLMSG_ERROR = 2)
    ///   bytes[6..8]   = nlmsg_flags (NLM_F_ACK_TLVS=0x200 when EXT_ACK)
    ///   bytes[8..12]  = nlmsg_seq   (from request)
    ///   bytes[12..16] = nlmsg_pid   (receiver's portid; mirrors Linux's
    ///                                __nlmsg_put portid argument)
    ///   bytes[16..20] = nlmsgerr.error
    ///   bytes[20..36] = nlmsgerr.msg (orig request nlmsghdr)
    #[test]
    fn rtnetlink_ack_layout_matches_linux_netlink_ack() {
        crate::net::device::init();
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();

        // Forge an RTM_SETLINK request (32 bytes: nlmsghdr + ifinfomsg).
        // Use a non-trivial seq and flags so we can verify they round-trip
        // into the ACK.
        let mut req = alloc::vec![0u8; 32];
        req[0..4].copy_from_slice(&32u32.to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_SETLINK.to_ne_bytes());
        // NLM_F_REQUEST | NLM_F_ACK = 0x05.
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&0xCAFEu32.to_ne_bytes());
        let lo = crate::net::device::lookup_netdevice("lo").expect("loopback device");
        req[20..24].copy_from_slice(&(lo.ifindex as i32).to_ne_bytes());
        req[24..28].copy_from_slice(&crate::net::device::IFF_UP.to_ne_bytes());
        req[28..32].copy_from_slice(&crate::net::device::IFF_UP.to_ne_bytes());

        assert_eq!(
            sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            req.len()
        );

        // Drain the ACK that synthesize_route_netlink enqueued.
        let mut out = [0u8; 64];
        let (n, _, _, _, _, _) = recvmsg_full(&sock, &mut out, 0).expect("ack");
        assert_eq!(n, 36, "NLMSG_ERROR ACK must be 36 bytes");

        let len = u32::from_ne_bytes(out[0..4].try_into().unwrap());
        let ty = u16::from_ne_bytes(out[4..6].try_into().unwrap());
        let seq = u32::from_ne_bytes(out[8..12].try_into().unwrap());
        let err = i32::from_ne_bytes(out[16..20].try_into().unwrap());
        assert_eq!(len, 36);
        assert_eq!(ty, NLMSG_ERROR);
        assert_eq!(seq, 0xCAFE, "ACK must echo the request's nlmsg_seq");
        assert_eq!(err, 0, "RTM_SETLINK must ACK with success");
        // The original 16-byte nlmsghdr is mirrored into the trailing payload.
        assert_eq!(&out[20..36], &req[..16]);
    }

    #[test]
    fn unprivileged_rtnetlink_setlink_requires_net_admin() {
        let name = "rtnl-setlink-unpriv0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 7],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        assert!(!dev.is_up(), "fresh dummy device should start down");

        let previous = unsafe { sched::get_current() };
        let cred = unprivileged_cred();
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 0x4e45_5441;
        current.tgid = 0x4e45_5441;
        current.cred = &*cred as *const Cred;
        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
        }

        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        let mut req = alloc::vec![0u8; 32];
        req[0..4].copy_from_slice(&32u32.to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_SETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&0x4eadu32.to_ne_bytes());
        req[20..24].copy_from_slice(&(dev.ifindex as i32).to_ne_bytes());
        req[24..28].copy_from_slice(&crate::net::device::IFF_UP.to_ne_bytes());
        req[28..32].copy_from_slice(&crate::net::device::IFF_UP.to_ne_bytes());

        assert_eq!(
            sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            req.len()
        );

        let mut out = [0u8; 64];
        let (n, _, _, _, _, _) = recvmsg_full(&sock, &mut out, 0).expect("ack");
        assert_eq!(n, 36, "NLMSG_ERROR ACK must be 36 bytes");
        assert_eq!(
            u16::from_ne_bytes(out[4..6].try_into().unwrap()),
            NLMSG_ERROR
        );
        assert_eq!(
            i32::from_ne_bytes(out[16..20].try_into().unwrap()),
            -(EPERM as i32)
        );
        assert!(
            !dev.is_up(),
            "unprivileged RTM_SETLINK must not raise IFF_UP"
        );

        unsafe {
            sched::set_current(previous);
        }
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_setlink_zero_change_still_applies_iff_up() {
        let name = "rtnl-setlink0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 5],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        assert!(!dev.is_up(), "fresh dummy device should start down");

        // systemd-networkd subscribes to RTNLGRP_LINK and sends RTM_SETLINK
        // without NLM_F_ECHO: Linux answers with an ACK plus a kernel-origin
        // (nlmsg_pid=0, nlmsg_seq=0) RTM_NEWLINK multicast that reaches every
        // group member, including the requester
        // (vendor/linux/net/core/dev.c::__dev_notify_flags -> rtmsg_ifinfo,
        // vendor/linux/net/core/rtnetlink.c::rtmsg_ifinfo_build_skb,
        // vendor/linux/net/netlink/af_netlink.c::nlmsg_notify).
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x6041,
                groups: 1u32 << (RTNLGRP_LINK - 1),
            },
        )
        .unwrap();
        let mut req = alloc::vec![0u8; 32];
        req[0..4].copy_from_slice(&32u32.to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_SETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[20..24].copy_from_slice(&(dev.ifindex as i32).to_ne_bytes());
        req[24..28].copy_from_slice(&crate::net::device::IFF_UP.to_ne_bytes());
        req[28..32].copy_from_slice(&0u32.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send");
        let mut out = [0u8; 256];
        let mut saw_notify = false;
        let _ = recv_until_ack(&sock, &mut out, |packet| {
            if u16::from_ne_bytes(packet[4..6].try_into().unwrap()) == RTM_NEWLINK
                && u32::from_ne_bytes(packet[20..24].try_into().unwrap()) == dev.ifindex
            {
                assert_eq!(
                    u32::from_ne_bytes(packet[8..12].try_into().unwrap()),
                    0,
                    "link-up notification must be kernel-origin (seq=0)"
                );
                assert_eq!(
                    u32::from_ne_bytes(packet[12..16].try_into().unwrap()),
                    0,
                    "link-up notification must be kernel-origin (pid=0)"
                );
                saw_notify = true;
            }
        });
        assert!(
            saw_notify,
            "RTM_SETLINK link-up must multicast RTM_NEWLINK to RTNLGRP_LINK"
        );
        assert!(dev.is_up(), "RTM_SETLINK change=0 must still honor IFF_UP");
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_setlink_linkmode_and_operstate_follow_linux_reporting() {
        let name = "rtnl-setlink1";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 6],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        crate::net::device::set_device_up(&dev).expect("device up");

        // Like networkd: subscribed to RTNLGRP_LINK, no NLM_F_ECHO.  The
        // operstate/linkmode change must arrive as the kernel-origin
        // RTM_NEWLINK multicast (vendor/linux/net/core/rtnetlink.c::
        // do_setlink -> netdev_state_change -> rtmsg_ifinfo_event).
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x603e,
                groups: 1u32 << (RTNLGRP_LINK - 1),
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 16];
        req[4..6].copy_from_slice(&RTM_SETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&57u32.to_ne_bytes());
        req[20..24].copy_from_slice(&(dev.ifindex as i32).to_ne_bytes());
        req[24..28].copy_from_slice(&crate::net::device::IFF_UP.to_ne_bytes());
        req[28..32].copy_from_slice(&crate::net::device::IFF_UP.to_ne_bytes());
        push_rta_u8(&mut req, IFLA_LINKMODE, IF_LINK_MODE_DORMANT);
        push_rta_u8(&mut req, IFLA_OPERSTATE, IF_OPER_DORMANT);
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send");
        let mut ack = [0u8; 256];
        let mut saw_notify = false;
        let _ = recv_until_ack(&sock, &mut ack, |packet| {
            if u16::from_ne_bytes(packet[4..6].try_into().unwrap()) == RTM_NEWLINK
                && u32::from_ne_bytes(packet[20..24].try_into().unwrap()) == dev.ifindex
            {
                saw_notify = true;
                assert_eq!(
                    u32::from_ne_bytes(packet[8..12].try_into().unwrap()),
                    0,
                    "state-change notification must be kernel-origin (seq=0)"
                );
                assert_eq!(
                    u32::from_ne_bytes(packet[12..16].try_into().unwrap()),
                    0,
                    "state-change notification must be kernel-origin (pid=0)"
                );
                let flags = u32::from_ne_bytes(packet[24..28].try_into().unwrap());
                assert_eq!(attr_u8(packet, IFLA_LINKMODE), Some(IF_LINK_MODE_DORMANT));
                assert_eq!(attr_u8(packet, IFLA_OPERSTATE), Some(IF_OPER_DORMANT));
                assert_eq!(attr_u8(packet, IFLA_CARRIER), Some(1));
                assert_eq!(
                    flags & crate::net::device::IFF_LOWER_UP,
                    crate::net::device::IFF_LOWER_UP
                );
                assert_eq!(
                    flags & crate::net::device::IFF_DORMANT,
                    crate::net::device::IFF_DORMANT
                );
                assert_eq!(flags & crate::net::device::IFF_RUNNING, 0);
            }
        });
        assert!(
            saw_notify,
            "RTM_SETLINK must multicast operstate/linkmode updates to RTNLGRP_LINK"
        );
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    /// Source-backed parity check for `recvmsg(MSG_PEEK | MSG_TRUNC)` — the
    /// contract `vendor/systemd/systemd-260.1/src/libsystemd/sd-netlink/
    /// netlink-message.c` uses to size netlink datagrams before draining
    /// them.  Reference: `vendor/linux/net/socket.c::sock_recvmsg` and
    /// `vendor/linux/net/netlink/af_netlink.c::netlink_recvmsg`.
    #[test]
    fn audit_netlink_status_and_readlog_delivery_match_linux_uapi() {
        let _guard = crate::kernel::audit::test_lock();

        fn audit_req_with_flags(
            msg_type: u16,
            seq: u32,
            pid: u32,
            flags: u16,
            payload: &[u8],
        ) -> Vec<u8> {
            let len = NLMSG_HDRLEN + payload.len();
            let mut msg = alloc::vec![0u8; len];
            msg[0..4].copy_from_slice(&(len as u32).to_ne_bytes());
            msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
            msg[6..8].copy_from_slice(&flags.to_ne_bytes());
            msg[8..12].copy_from_slice(&seq.to_ne_bytes());
            msg[12..16].copy_from_slice(&pid.to_ne_bytes());
            msg[NLMSG_HDRLEN..].copy_from_slice(payload);
            msg
        }
        fn audit_req(msg_type: u16, seq: u32, pid: u32, payload: &[u8]) -> Vec<u8> {
            audit_req_with_flags(msg_type, seq, pid, 1, payload)
        }
        fn nl_type(packet: &[u8]) -> u16 {
            u16::from_ne_bytes(packet[4..6].try_into().unwrap())
        }
        fn nl_flags(packet: &[u8]) -> u16 {
            u16::from_ne_bytes(packet[6..8].try_into().unwrap())
        }
        fn nl_seq(packet: &[u8]) -> u32 {
            u32::from_ne_bytes(packet[8..12].try_into().unwrap())
        }
        fn nl_pid(packet: &[u8]) -> u32 {
            u32::from_ne_bytes(packet[12..16].try_into().unwrap())
        }
        fn status_field(packet: &[u8], idx: usize) -> u32 {
            let off = NLMSG_HDRLEN + idx * 4;
            u32::from_ne_bytes(packet[off..off + 4].try_into().unwrap())
        }

        crate::kernel::audit::reset_for_test();
        let readlog = socket(AF_NETLINK, SOCK_RAW, NETLINK_AUDIT).unwrap();
        let auditd = socket(AF_NETLINK, SOCK_RAW, NETLINK_AUDIT).unwrap();
        let readlog_pid = 0x0a11_d001;
        let auditd_pid = 0x0a11_d002;
        bind(
            &readlog,
            SockAddr::Netlink {
                pid: readlog_pid,
                groups: 1,
            },
        )
        .unwrap();
        bind(
            &auditd,
            SockAddr::Netlink {
                pid: auditd_pid,
                groups: 0,
            },
        )
        .unwrap();

        let get = audit_req(AUDIT_GET, 7, auditd_pid, &[]);
        assert_eq!(
            sendto(&auditd, &get, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            get.len()
        );
        let mut out = [0u8; 128];
        let n = recvmsg(&auditd, &mut out).expect("AUDIT_GET reply");
        assert_eq!(nl_type(&out[..n]), AUDIT_GET);
        assert_eq!(nl_seq(&out[..n]), 7);
        assert_eq!(nl_pid(&out[..n]), auditd_pid);
        assert_eq!(status_field(&out[..n], 3), 0, "auditd pid starts unset");

        let mut status = [0u8; 44];
        status[0..4].copy_from_slice(
            &(crate::kernel::audit::AUDIT_STATUS_PID | crate::kernel::audit::AUDIT_STATUS_ENABLED)
                .to_ne_bytes(),
        );
        status[4..8].copy_from_slice(&1u32.to_ne_bytes());
        status[12..16].copy_from_slice(&auditd_pid.to_ne_bytes());
        let set = audit_req_with_flags(AUDIT_SET, 8, auditd_pid, 1 | NLM_F_ACK, &status);
        assert_eq!(
            sendto(&auditd, &set, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            set.len()
        );
        let n = recvmsg(&auditd, &mut out).expect("AUDIT_SET ACK");
        assert_eq!(nl_type(&out[..n]), NLMSG_ERROR);
        assert_eq!(i32::from_ne_bytes(out[16..20].try_into().unwrap()), 0);
        assert_eq!(crate::kernel::audit::auditd_pid(), auditd_pid);

        let list = audit_req_with_flags(AUDIT_LIST_RULES, 9, auditd_pid, 1 | NLM_F_ACK, &[]);
        assert_eq!(
            sendto(&auditd, &list, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            list.len()
        );
        let n = recvmsg(&auditd, &mut out).expect("AUDIT_LIST_RULES ACK");
        assert_eq!(nl_type(&out[..n]), NLMSG_ERROR);
        assert_eq!(nl_seq(&out[..n]), 9);
        assert_eq!(i32::from_ne_bytes(out[16..20].try_into().unwrap()), 0);
        let n = recvmsg(&auditd, &mut out).expect("AUDIT_LIST_RULES done");
        assert_eq!(n, NLMSG_HDRLEN + core::mem::size_of::<i32>());
        assert_eq!(nl_type(&out[..n]), NLMSG_DONE);
        assert_eq!(nl_flags(&out[..n]), NLM_F_MULTI);
        assert_eq!(nl_seq(&out[..n]), 9);
        assert_eq!(nl_pid(&out[..n]), auditd_pid);

        crate::kernel::audit::audit_log("type=DAEMON_START msg=auditd started");
        let n = recvmsg(&auditd, &mut out).expect("auditd unicast record");
        assert_eq!(nl_type(&out[..n]), AUDIT_KERNEL);
        let needle = b"type=DAEMON_START";
        assert!(out[..n].windows(needle.len()).any(|w| w == needle));

        let n = recvmsg(&readlog, &mut out).expect("readlog multicast record");
        assert_eq!(nl_type(&out[..n]), AUDIT_KERNEL);
        assert!(out[..n].windows(needle.len()).any(|w| w == needle));

        release_bound_socket(&readlog);
        release_bound_socket(&auditd);
        crate::kernel::audit::reset_for_test();
    }

    #[test]
    fn unprivileged_audit_netlink_requires_audit_capabilities() {
        let _guard = crate::kernel::audit::test_lock();
        crate::kernel::audit::reset_for_test();

        fn audit_req(msg_type: u16, seq: u32, pid: u32, payload: &[u8]) -> Vec<u8> {
            let len = NLMSG_HDRLEN + payload.len();
            let mut msg = alloc::vec![0u8; len];
            msg[0..4].copy_from_slice(&(len as u32).to_ne_bytes());
            msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
            msg[6..8].copy_from_slice(&1u16.to_ne_bytes());
            msg[8..12].copy_from_slice(&seq.to_ne_bytes());
            msg[12..16].copy_from_slice(&pid.to_ne_bytes());
            msg[NLMSG_HDRLEN..].copy_from_slice(payload);
            msg
        }

        let previous = unsafe { sched::get_current() };
        let cred = unprivileged_cred();
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 0x0bad_c0de;
        current.tgid = 0x0bad_c0de;
        current.cred = &*cred as *const Cred;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
        }

        let auditd = socket(AF_NETLINK, SOCK_RAW, NETLINK_AUDIT).unwrap();
        let readlog = socket(AF_NETLINK, SOCK_RAW, NETLINK_AUDIT).unwrap();
        let auditd_pid = 0x0a11_d002;
        bind(
            &auditd,
            SockAddr::Netlink {
                pid: auditd_pid,
                groups: 0,
            },
        )
        .unwrap();

        let mut status = [0u8; 44];
        status[0..4].copy_from_slice(&crate::kernel::audit::AUDIT_STATUS_PID.to_ne_bytes());
        status[12..16].copy_from_slice(&auditd_pid.to_ne_bytes());
        let set = audit_req(AUDIT_SET, 99, auditd_pid, &status);
        assert_eq!(
            sendto(&auditd, &set, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            set.len()
        );

        let mut out = [0u8; 128];
        let n = recvmsg(&auditd, &mut out).expect("AUDIT_SET denial ACK");
        assert_eq!(
            u16::from_ne_bytes(out[4..6].try_into().unwrap()),
            NLMSG_ERROR
        );
        assert_eq!(
            i32::from_ne_bytes(out[16..20].try_into().unwrap()),
            -(EPERM as i32)
        );
        assert_eq!(crate::kernel::audit::auditd_pid(), 0);

        assert_eq!(
            bind(
                &readlog,
                SockAddr::Netlink {
                    pid: 0x0a11_d001,
                    groups: audit_readlog_group_mask(),
                },
            ),
            Err(EPERM)
        );
        bind(
            &readlog,
            SockAddr::Netlink {
                pid: 0x0a11_d001,
                groups: 0,
            },
        )
        .unwrap();
        assert_eq!(
            set_netlink_membership(&readlog, AUDIT_NLGRP_READLOG, true),
            Err(EPERM)
        );

        release_bound_socket(&readlog);
        release_bound_socket(&auditd);
        unsafe {
            sched::set_current(previous);
        }
        crate::kernel::audit::reset_for_test();
    }

    #[test]
    fn unprivileged_route_netlink_mutations_require_net_admin() {
        let previous = unsafe { sched::get_current() };
        let cred = unprivileged_cred();
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 0x0bad_8000;
        current.tgid = 0x0bad_8000;
        current.cred = &*cred as *const Cred;
        current.m27.real_cred = &*cred as *const Cred;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
        }

        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        let mut req = [0u8; NLMSG_HDRLEN + 8];
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&1u16.to_ne_bytes());
        req[8..12].copy_from_slice(&0xaced_u32.to_ne_bytes());

        assert_eq!(
            sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            req.len()
        );

        let mut out = [0u8; 128];
        let n = recvmsg(&sock, &mut out).expect("RTM_NEWADDR denial ACK");
        assert_eq!(
            u16::from_ne_bytes(out[4..6].try_into().unwrap()),
            NLMSG_ERROR
        );
        assert_eq!(u32::from_ne_bytes(out[8..12].try_into().unwrap()), 0xaced);
        assert_eq!(
            i32::from_ne_bytes(out[16..20].try_into().unwrap()),
            -(EPERM as i32)
        );
        assert_eq!(n, 36);

        release_bound_socket(&sock);
        unsafe {
            sched::set_current(previous);
        }
    }

    #[test]
    fn kobject_uevent_broadcast_reaches_bound_netlink_listener() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        let sock = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x5545_0001,
                groups: 1,
            },
        )
        .unwrap();

        crate::net::uevent::announce_class_device(
            "input",
            "event-test0",
            "input",
            "input/event-test0",
        );

        let mut out = [0u8; 256];
        let n = recvmsg(&sock, &mut out).expect("uevent payload");
        assert!(out[..n].starts_with(b"add@/devices/virtual/input/event-test0\0"));
        assert!(out[..n].windows(16).any(|w| w == b"SUBSYSTEM=input\0"));
        release_bound_socket(&sock);
        let _ = crate::net::uevent::drain_pending();
    }

    #[test]
    fn kernel_kobject_uevent_does_not_reach_udev_userspace_group() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        let udev_listener = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &udev_listener,
            SockAddr::Netlink {
                pid: 0x5545_0011,
                groups: 2,
            },
        )
        .unwrap();

        crate::net::uevent::announce_class_device(
            "graphics",
            "fb-group-test0",
            "graphics",
            "fb-group-test0",
        );

        let mut out = [0u8; 256];
        assert_eq!(recvmsg(&udev_listener, &mut out), Err(EAGAIN));
        release_bound_socket(&udev_listener);
        let _ = crate::net::uevent::drain_pending();
    }

    #[test]
    fn userspace_kobject_multicast_reaches_only_requested_group() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        let sender = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &sender,
            SockAddr::Netlink {
                pid: 0x5545_0020,
                groups: 0,
            },
        )
        .unwrap();
        let udev_listener = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &udev_listener,
            SockAddr::Netlink {
                pid: 0x5545_0021,
                groups: 2,
            },
        )
        .unwrap();
        let kernel_listener = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &kernel_listener,
            SockAddr::Netlink {
                pid: 0x5545_0022,
                groups: 1,
            },
        )
        .unwrap();

        let payload = b"libudev\0processed-device";
        assert_eq!(
            sendto(&sender, payload, SockAddr::Netlink { pid: 0, groups: 2 },),
            Ok(payload.len())
        );

        let mut out = [0u8; 64];
        let (n, peer) = recvfrom(&udev_listener, &mut out).expect("userspace multicast");
        assert_eq!(&out[..n], payload);
        assert_eq!(
            peer,
            Some(SockAddr::Netlink {
                pid: 0x5545_0020,
                groups: 2,
            })
        );
        assert_eq!(last_rx_meta(&udev_listener).netlink_group, 2);
        assert_eq!(recvmsg(&kernel_listener, &mut out), Err(EAGAIN));

        release_bound_socket(&sender);
        release_bound_socket(&udev_listener);
        release_bound_socket(&kernel_listener);
        let _ = crate::net::uevent::drain_pending();
    }

    #[test]
    fn userspace_kobject_unicast_reports_sender_portid_with_zero_groups() {
        let manager = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &manager,
            SockAddr::Netlink {
                pid: 0x5545_0030,
                // The udev manager listens to kernel multicast while using
                // this same socket for unicast messages to its workers.
                groups: 1,
            },
        )
        .unwrap();
        let worker = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &worker,
            SockAddr::Netlink {
                pid: 0x5545_0031,
                groups: 0,
            },
        )
        .unwrap();

        let payload = b"libudev\0worker-device";
        assert_eq!(
            sendto(
                &manager,
                payload,
                SockAddr::Netlink {
                    pid: 0x5545_0031,
                    groups: 0,
                },
            ),
            Ok(payload.len())
        );

        let mut out = [0u8; 64];
        let (n, peer) = recvfrom(&worker, &mut out).expect("userspace unicast");
        assert_eq!(&out[..n], payload);
        assert_eq!(
            peer,
            Some(SockAddr::Netlink {
                pid: 0x5545_0030,
                groups: 0,
            })
        );
        assert_eq!(last_rx_meta(&worker).netlink_group, 0);

        release_bound_socket(&manager);
        release_bound_socket(&worker);
    }

    #[test]
    fn netlink_membership_replays_pending_kobject_uevents() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        crate::net::uevent::announce_class_device("graphics", "fb-test0", "graphics", "fb-test0");

        let sock = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x5545_0002,
                groups: 0,
            },
        )
        .unwrap();
        set_netlink_membership(&sock, 1, true).expect("join kobject multicast group");

        let mut out = [0u8; 256];
        let n = recvmsg(&sock, &mut out).expect("replayed uevent payload");
        assert!(out[..n].starts_with(b"add@/devices/virtual/graphics/fb-test0\0"));
        assert!(out[..n].windows(19).any(|w| w == b"SUBSYSTEM=graphics\0"));
        release_bound_socket(&sock);
        let _ = crate::net::uevent::drain_pending();
    }

    #[test]
    fn netlink_membership_autobinds_created_kobject_listener() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        crate::net::uevent::announce_class_device(
            "net",
            "eth-autobind0",
            "net",
            "net/eth-autobind0",
        );

        let sock = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        assert_eq!(sock.lock().state, SocketState::Created);

        set_netlink_membership(&sock, 1, true).expect("join kobject multicast group");

        {
            let socket = sock.lock();
            assert_eq!(socket.state, SocketState::Bound);
            match socket.local {
                Some(SockAddr::Netlink { pid, groups }) => {
                    assert_ne!(pid, 0, "autobind should assign a portid");
                    assert_eq!(groups, 1);
                }
                _ => panic!("expected autobound netlink address"),
            }
        }

        let mut out = [0u8; 256];
        let n = recvmsg(&sock, &mut out).expect("replayed uevent payload");
        assert!(out[..n].starts_with(b"add@/devices/virtual/net/eth-autobind0\0"));
        assert!(out[..n].windows(14).any(|w| w == b"SUBSYSTEM=net\0"));
        release_bound_socket(&sock);
        let _ = crate::net::uevent::drain_pending();
    }

    #[test]
    fn kobject_uevent_recvfrom_reports_kernel_multicast_group() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        let sock = socket(AF_NETLINK, SOCK_DGRAM, NETLINK_KOBJECT_UEVENT).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x5545_0003,
                groups: 1,
            },
        )
        .unwrap();

        crate::net::uevent::announce_class_device(
            "input",
            "event-test1",
            "input",
            "input/event-test1",
        );

        let mut out = [0u8; 256];
        let (n, peer) = recvfrom(&sock, &mut out).expect("uevent payload");
        assert!(out[..n].starts_with(b"add@/devices/virtual/input/event-test1\0"));
        assert_eq!(peer, Some(SockAddr::Netlink { pid: 0, groups: 1 }));
        assert_eq!(last_rx_meta(&sock).netlink_group, 1);
        release_bound_socket(&sock);
        let _ = crate::net::uevent::drain_pending();
    }

    #[test]
    fn recvmsg_peek_and_trunc_preserve_message_and_report_real_length() {
        crate::net::device::init();
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        // Drive an RTM_GETLINK so the recvq carries at least one real
        // netlink datagram of known shape.
        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 16];
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_GETLINK.to_ne_bytes());
        req[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        req[8..12].copy_from_slice(&101u32.to_ne_bytes());
        assert_eq!(
            sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).unwrap(),
            req.len()
        );

        // Empty buffer + MSG_PEEK|MSG_TRUNC: real_len must report the full
        // datagram size; bytes_copied is 0; the message stays on the queue.
        let mut empty: [u8; 0] = [];
        let (copied, _, _, _, real_len, _) =
            recvmsg_full(&sock, &mut empty, MSG_PEEK | MSG_TRUNC).expect("peek");
        assert_eq!(copied, 0);
        assert!(
            real_len >= NLMSG_HDRLEN,
            "peek must surface at least a netlink header"
        );

        // Buffer smaller than packet, no PEEK: pop, fill what fits, advertise
        // the real length via the fifth tuple element so the syscall layer
        // can set MSG_TRUNC + return real_len.
        let mut tiny = [0u8; NLMSG_HDRLEN];
        let (copied, _, _, _, real_len_after, _) =
            recvmsg_full(&sock, &mut tiny, 0).expect("consume");
        assert_eq!(copied, tiny.len());
        assert_eq!(real_len_after, real_len);

        // After the consuming recvmsg the head of the queue advanced — peek
        // returns a *different* message length (NLMSG_DONE, 16 bytes) or
        // EAGAIN if the dump only had one entry.  Either way the previous
        // packet is gone, proving MSG_PEEK didn't double-consume.
        let mut probe = [0u8; 256];
        match recvmsg_full(&sock, &mut probe, MSG_PEEK | MSG_TRUNC) {
            Ok((_, _, _, _, next_len, _)) => assert_ne!(
                next_len, real_len,
                "MSG_PEEK must not re-deliver the just-consumed packet"
            ),
            Err(EAGAIN) => {}
            Err(other) => panic!("unexpected recvmsg error after peek: {other}"),
        }
    }

    fn attr_payload(packet: &[u8], attr_type: u16) -> Option<&[u8]> {
        let mut offset = NLMSG_HDRLEN + 16;
        while offset + 4 <= packet.len() {
            let rta_len = u16::from_ne_bytes(packet[offset..offset + 2].try_into().ok()?) as usize;
            let rta_type = u16::from_ne_bytes(packet[offset + 2..offset + 4].try_into().ok()?);
            if rta_len < 4 || offset.checked_add(rta_len)? > packet.len() {
                return None;
            }
            if rta_type == attr_type {
                return Some(&packet[offset + 4..offset + rta_len]);
            }
            offset = offset.checked_add(nlmsg_align(rta_len))?;
        }
        None
    }

    fn attr_u8(packet: &[u8], attr_type: u16) -> Option<u8> {
        let payload = attr_payload(packet, attr_type)?;
        (payload.len() == 1).then_some(payload[0])
    }

    fn attr_u32(packet: &[u8], attr_type: u16) -> Option<u32> {
        let payload = attr_payload(packet, attr_type)?;
        if payload.len() != 4 {
            return None;
        }
        Some(u32::from_ne_bytes(payload.try_into().ok()?))
    }

    fn addr_attr_payload(packet: &[u8], attr_type: u16) -> Option<&[u8]> {
        rtattr_payload(packet, NLMSG_HDRLEN + 8, attr_type)
    }

    fn addr_attr_ipv4(packet: &[u8], attr_type: u16) -> Option<[u8; 4]> {
        let payload = addr_attr_payload(packet, attr_type)?;
        (payload.len() == 4).then_some(payload.try_into().ok()?)
    }

    fn addr_attr_u32(packet: &[u8], attr_type: u16) -> Option<u32> {
        let payload = addr_attr_payload(packet, attr_type)?;
        (payload.len() == 4).then_some(u32::from_ne_bytes(payload.try_into().ok()?))
    }

    fn addr_attr_cacheinfo(packet: &[u8]) -> Option<IfaCacheInfo> {
        let payload = addr_attr_payload(packet, IFA_CACHEINFO)?;
        if payload.len() != 16 {
            return None;
        }
        Some(IfaCacheInfo {
            preferred: u32::from_ne_bytes(payload[0..4].try_into().ok()?),
            valid: u32::from_ne_bytes(payload[4..8].try_into().ok()?),
            cstamp: u32::from_ne_bytes(payload[8..12].try_into().ok()?),
            tstamp: u32::from_ne_bytes(payload[12..16].try_into().ok()?),
        })
    }

    fn route_attr_payload(packet: &[u8], attr_type: u16) -> Option<&[u8]> {
        rtattr_payload(packet, NLMSG_HDRLEN + 12, attr_type)
    }

    fn route_attr_ipv4(packet: &[u8], attr_type: u16) -> Option<[u8; 4]> {
        let payload = route_attr_payload(packet, attr_type)?;
        (payload.len() == 4).then_some(payload.try_into().ok()?)
    }

    fn route_attr_u32(packet: &[u8], attr_type: u16) -> Option<u32> {
        let payload = route_attr_payload(packet, attr_type)?;
        (payload.len() == 4).then_some(u32::from_ne_bytes(payload.try_into().ok()?))
    }

    fn recv_until_ack<F: FnMut(&[u8])>(
        sock: &SocketRef,
        out: &mut [u8],
        mut on_packet: F,
    ) -> usize {
        for _ in 0..16 {
            let (n, _, _, _, _, _) = recvmsg_full(sock, out, 0).expect("recv");
            let packet = &out[..n];
            if u16::from_ne_bytes(packet[4..6].try_into().unwrap()) == NLMSG_ERROR {
                return n;
            }
            on_packet(packet);
        }
        panic!("expected netlink ACK");
    }

    #[test]
    fn rtnetlink_getaddr_dump_reports_loopback_ipv4() {
        crate::net::device::init();
        let lo = crate::net::device::lookup_netdevice("lo").expect("loopback device");
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x600d,
                groups: 0,
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[0..4].copy_from_slice(&((NLMSG_HDRLEN + 8) as u32).to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_GETADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        req[8..12].copy_from_slice(&41u32.to_ne_bytes());
        req[16] = AF_INET as u8;

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send");

        let mut saw_loopback = false;
        let mut saw_done = false;
        for _ in 0..16 {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).expect("recv");
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            if msg_type == RTM_NEWADDR
                && u32::from_ne_bytes(packet[20..24].try_into().unwrap()) == lo.ifindex
            {
                saw_loopback = true;
                assert_eq!(packet[16], AF_INET as u8);
                assert_eq!(packet[17], 8);
                assert_eq!(packet[19], RT_SCOPE_HOST);
                assert_eq!(addr_attr_ipv4(packet, IFA_LOCAL), Some([127, 0, 0, 1]));
                assert_eq!(addr_attr_ipv4(packet, IFA_ADDRESS), Some([127, 0, 0, 1]));
                assert_eq!(addr_attr_payload(packet, IFA_LABEL), Some(&b"lo\0"[..]));
            } else if msg_type == NLMSG_DONE {
                saw_done = true;
                break;
            }
        }

        assert!(saw_loopback, "RTM_GETADDR dump should include 127.0.0.1/8");
        assert!(saw_done, "RTM_GETADDR dump should end with NLMSG_DONE");
    }

    #[test]
    fn rtnetlink_done_carries_linux_status_payload() {
        crate::net::device::init();
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x600e,
                groups: 0,
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[0..4].copy_from_slice(&((NLMSG_HDRLEN + 8) as u32).to_ne_bytes());
        req[4..6].copy_from_slice(&RTM_GETADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        req[8..12].copy_from_slice(&44u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[20..24].copy_from_slice(&u32::MAX.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send");

        loop {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).expect("recv");
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            if msg_type == NLMSG_DONE {
                assert_eq!(n, NLMSG_HDRLEN + 4);
                assert_eq!(u32::from_ne_bytes(packet[0..4].try_into().unwrap()), 20);
                assert_eq!(i32::from_ne_bytes(packet[16..20].try_into().unwrap()), 0);
                break;
            }
        }
    }

    #[test]
    fn rtnetlink_newaddr_persists_and_replays_via_getaddr() {
        let name = "rtaddr0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 9],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x601d,
                groups: 0,
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&42u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[17] = 24;
        req[18] = IFA_F_PERMANENT as u8;
        req[19] = RT_SCOPE_UNIVERSE;
        req[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut req, IFA_LOCAL, ipv4(10, 0, 2, 15));
        push_rta_ipv4(&mut req, IFA_ADDRESS, ipv4(10, 0, 2, 15));
        push_rta_ipv4(&mut req, IFA_BROADCAST, ipv4(10, 0, 2, 255));
        push_rta_bytes(&mut req, IFA_LABEL, name.as_bytes(), true);
        push_rta_u32(&mut req, IFA_FLAGS, IFA_F_PERMANENT);
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send newaddr");
        let mut ack = [0u8; 128];
        let n = recv_until_ack(&sock, &mut ack, |_| {});
        assert_eq!(
            u16::from_ne_bytes(ack[4..6].try_into().unwrap()),
            NLMSG_ERROR
        );
        assert_eq!(i32::from_ne_bytes(ack[16..20].try_into().unwrap()), 0);
        assert_eq!(n, 36);

        let mut dump = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        dump[0..4].copy_from_slice(&((NLMSG_HDRLEN + 8) as u32).to_ne_bytes());
        dump[4..6].copy_from_slice(&RTM_GETADDR.to_ne_bytes());
        dump[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        dump[8..12].copy_from_slice(&43u32.to_ne_bytes());
        dump[16] = AF_INET as u8;
        dump[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        sendto(&sock, &dump, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send dump");

        let mut saw_addr = false;
        let mut saw_done = false;
        for _ in 0..16 {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).expect("recv dump");
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            if msg_type == RTM_NEWADDR
                && u32::from_ne_bytes(packet[20..24].try_into().unwrap()) == dev.ifindex
            {
                saw_addr = true;
                assert_eq!(packet[17], 24);
                assert_eq!(packet[19], RT_SCOPE_UNIVERSE);
                assert_eq!(addr_attr_ipv4(packet, IFA_LOCAL), Some([10, 0, 2, 15]));
                assert_eq!(addr_attr_ipv4(packet, IFA_ADDRESS), Some([10, 0, 2, 15]));
                assert_eq!(addr_attr_ipv4(packet, IFA_BROADCAST), Some([10, 0, 2, 255]));
                assert_eq!(addr_attr_u32(packet, IFA_FLAGS), Some(IFA_F_PERMANENT));
                assert_eq!(
                    addr_attr_cacheinfo(packet),
                    Some(IfaCacheInfo {
                        preferred: u32::MAX,
                        valid: u32::MAX,
                        cstamp: 0,
                        tstamp: 0,
                    })
                );
                assert_eq!(
                    addr_attr_payload(packet, IFA_LABEL),
                    Some(&b"rtaddr0\0"[..])
                );
            } else if msg_type == NLMSG_DONE {
                saw_done = true;
                break;
            }
        }

        assert!(
            saw_addr,
            "RTM_GETADDR should replay the configured IPv4 address"
        );
        assert!(saw_done, "RTM_GETADDR dump should complete");

        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_newaddr_without_cacheinfo_defaults_to_permanent_lifetimes() {
        let name = "rtaddrperm0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x0b],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x603d,
                groups: 0,
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&55u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[17] = 24;
        req[19] = RT_SCOPE_UNIVERSE;
        req[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut req, IFA_LOCAL, ipv4(10, 0, 2, 19));
        push_rta_ipv4(&mut req, IFA_ADDRESS, ipv4(10, 0, 2, 19));
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send newaddr");
        let mut ack = [0u8; 128];
        let n = recv_until_ack(&sock, &mut ack, |_| {});
        assert_eq!(n, 36);
        assert_eq!(i32::from_ne_bytes(ack[16..20].try_into().unwrap()), 0);

        let mut dump = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        dump[0..4].copy_from_slice(&((NLMSG_HDRLEN + 8) as u32).to_ne_bytes());
        dump[4..6].copy_from_slice(&RTM_GETADDR.to_ne_bytes());
        dump[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        dump[8..12].copy_from_slice(&56u32.to_ne_bytes());
        dump[16] = AF_INET as u8;
        dump[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        sendto(&sock, &dump, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send dump");

        let mut saw_addr = false;
        for _ in 0..16 {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).expect("recv dump");
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            if msg_type == RTM_NEWADDR
                && u32::from_ne_bytes(packet[20..24].try_into().unwrap()) == dev.ifindex
                && addr_attr_ipv4(packet, IFA_LOCAL) == Some([10, 0, 2, 19])
            {
                saw_addr = true;
                assert_eq!(addr_attr_u32(packet, IFA_FLAGS), Some(IFA_F_PERMANENT));
                assert_eq!(
                    addr_attr_cacheinfo(packet),
                    Some(IfaCacheInfo {
                        preferred: u32::MAX,
                        valid: u32::MAX,
                        cstamp: 0,
                        tstamp: 0,
                    })
                );
                break;
            }
        }

        assert!(
            saw_addr,
            "RTM_GETADDR should replay addresses without cacheinfo as permanent"
        );

        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        release_bound_socket(&sock);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_newaddr_echo_notifies_requester() {
        let name = "rtaddrecho0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x1e],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x603e,
                groups: 0,
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&(0x0005u16 | NLM_F_ECHO).to_ne_bytes());
        req[8..12].copy_from_slice(&57u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[17] = 24;
        req[19] = RT_SCOPE_UNIVERSE;
        req[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut req, IFA_LOCAL, ipv4(10, 0, 2, 20));
        push_rta_ipv4(&mut req, IFA_ADDRESS, ipv4(10, 0, 2, 20));
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send newaddr");
        let mut ack = [0u8; 128];
        let mut saw_notify = false;
        let n = recv_until_ack(&sock, &mut ack, |packet| {
            if u16::from_ne_bytes(packet[4..6].try_into().unwrap()) == RTM_NEWADDR
                && u32::from_ne_bytes(packet[20..24].try_into().unwrap()) == dev.ifindex
                && addr_attr_ipv4(packet, IFA_LOCAL) == Some([10, 0, 2, 20])
            {
                saw_notify = true;
                assert_eq!(u32::from_ne_bytes(packet[8..12].try_into().unwrap()), 57);
                assert_eq!(
                    u32::from_ne_bytes(packet[12..16].try_into().unwrap()),
                    0x603e
                );
            }
        });
        assert_eq!(n, 36);
        assert_eq!(i32::from_ne_bytes(ack[16..20].try_into().unwrap()), 0);
        assert!(
            saw_notify,
            "NLM_F_ECHO should report RTM_NEWADDR to requester"
        );

        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        release_bound_socket(&sock);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_newaddr_multicast_is_kernel_origin() {
        let name = "rtaddrgrp1";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x21],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        let requester = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &requester,
            SockAddr::Netlink {
                pid: 0x603d,
                groups: 0,
            },
        )
        .unwrap();
        let subscriber = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &subscriber,
            SockAddr::Netlink {
                pid: 0x603c,
                groups: 1u32 << (RTNLGRP_IPV4_IFADDR - 1),
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&56u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[17] = 24;
        req[19] = RT_SCOPE_UNIVERSE;
        req[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut req, IFA_LOCAL, ipv4(10, 0, 2, 18));
        push_rta_ipv4(&mut req, IFA_ADDRESS, ipv4(10, 0, 2, 18));
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&requester, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send newaddr");
        let mut ack = [0u8; 128];
        let _ = recv_until_ack(&requester, &mut ack, |_| {});

        let mut out = [0u8; 1024];
        loop {
            let (n, peer) = recvfrom(&subscriber, &mut out).expect("multicast addr");
            if u16::from_ne_bytes(out[4..6].try_into().unwrap()) == RTM_NEWADDR
                && u32::from_ne_bytes(out[20..24].try_into().unwrap()) == dev.ifindex
                && addr_attr_ipv4(&out[..n], IFA_LOCAL) == Some([10, 0, 2, 18])
            {
                assert_eq!(u32::from_ne_bytes(out[8..12].try_into().unwrap()), 0);
                assert_eq!(u32::from_ne_bytes(out[12..16].try_into().unwrap()), 0);
                assert_eq!(
                    peer,
                    Some(SockAddr::Netlink {
                        pid: 0,
                        groups: 1u32 << (RTNLGRP_IPV4_IFADDR - 1),
                    })
                );
                break;
            }
        }

        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        release_bound_socket(&requester);
        release_bound_socket(&subscriber);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_route_multicast_reports_kernel_group() {
        let name = "rtroutegrp0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x0c],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        crate::net::device::set_device_up(&dev).expect("device up");
        let requester = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &requester,
            SockAddr::Netlink {
                pid: 0x603f,
                groups: 0,
            },
        )
        .unwrap();
        let subscriber = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &subscriber,
            SockAddr::Netlink {
                pid: 0x6040,
                groups: 1u32 << (RTNLGRP_IPV4_ROUTE - 1),
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&58u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[17] = 24;
        req[19] = RT_SCOPE_UNIVERSE;
        req[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut req, IFA_LOCAL, ipv4(10, 0, 2, 21));
        push_rta_ipv4(&mut req, IFA_ADDRESS, ipv4(10, 0, 2, 21));
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&requester, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send newaddr");
        let mut ack = [0u8; 128];
        let _ = recv_until_ack(&requester, &mut ack, |_| {});

        let mut out = [0u8; 1024];
        loop {
            let (n, peer) = recvfrom(&subscriber, &mut out).expect("multicast route");
            if u16::from_ne_bytes(out[4..6].try_into().unwrap()) == RTM_NEWROUTE
                && route_attr_ipv4(&out[..n], RTA_DST) == Some([10, 0, 2, 0])
            {
                assert_eq!(u32::from_ne_bytes(out[8..12].try_into().unwrap()), 0);
                assert_eq!(u32::from_ne_bytes(out[12..16].try_into().unwrap()), 0);
                assert_eq!(
                    route_attr_u32(&out[..n], RTA_TABLE),
                    Some(RT_TABLE_MAIN.into())
                );
                assert_eq!(route_attr_u32(&out[..n], RTA_PRIORITY), None);
                assert_eq!(
                    peer,
                    Some(SockAddr::Netlink {
                        pid: 0,
                        groups: 1u32 << (RTNLGRP_IPV4_ROUTE - 1),
                    })
                );
                break;
            }
        }

        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_route_multicast_tracks_numeric_dst_group_in_packet_meta() {
        let name = "rtroutegrpmeta0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x0f],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        crate::net::device::set_device_up(&dev).expect("device up");
        let requester = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &requester,
            SockAddr::Netlink {
                pid: 0x6042,
                groups: 0,
            },
        )
        .unwrap();
        let subscriber = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &subscriber,
            SockAddr::Netlink {
                pid: 0x6043,
                groups: 1u32 << (RTNLGRP_IPV4_ROUTE - 1),
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&60u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[17] = 24;
        req[19] = RT_SCOPE_UNIVERSE;
        req[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut req, IFA_LOCAL, ipv4(10, 0, 2, 23));
        push_rta_ipv4(&mut req, IFA_ADDRESS, ipv4(10, 0, 2, 23));
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&requester, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send newaddr");
        let mut ack = [0u8; 128];
        let _ = recv_until_ack(&requester, &mut ack, |_| {});

        let mut out = [0u8; 1024];
        loop {
            let (n, peer, _, _, _, _) = recvmsg_full(&subscriber, &mut out, 0).expect("recv");
            if u16::from_ne_bytes(out[4..6].try_into().unwrap()) == RTM_NEWROUTE
                && route_attr_ipv4(&out[..n], RTA_DST) == Some([10, 0, 2, 0])
            {
                assert_eq!(
                    peer,
                    Some(SockAddr::Netlink {
                        pid: 0,
                        groups: 1u32 << (RTNLGRP_IPV4_ROUTE - 1),
                    })
                );
                assert_eq!(last_rx_meta(&subscriber).netlink_group, RTNLGRP_IPV4_ROUTE);
                break;
            }
        }

        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_requester_subscribed_to_route_group_gets_kernel_multicast_only() {
        let name = "rtroutegrp1";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x0d],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        crate::net::device::set_device_up(&dev).expect("device up");
        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x6041,
                groups: 1u32 << (RTNLGRP_IPV4_ROUTE - 1),
            },
        )
        .unwrap();

        let mut req = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        req[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        req[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        req[8..12].copy_from_slice(&59u32.to_ne_bytes());
        req[16] = AF_INET as u8;
        req[17] = 24;
        req[19] = RT_SCOPE_UNIVERSE;
        req[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut req, IFA_LOCAL, ipv4(10, 0, 2, 22));
        push_rta_ipv4(&mut req, IFA_ADDRESS, ipv4(10, 0, 2, 22));
        let req_len = req.len() as u32;
        req[0..4].copy_from_slice(&req_len.to_ne_bytes());

        sendto(&sock, &req, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send newaddr");

        let mut saw_multicast = false;
        let mut route_keys = Vec::new();
        loop {
            let mut out = [0u8; 1024];
            let (n, peer) = recvfrom(&sock, &mut out).expect("recv");
            let msg_type = u16::from_ne_bytes(out[4..6].try_into().unwrap());
            if msg_type == NLMSG_ERROR {
                assert_eq!(i32::from_ne_bytes(out[16..20].try_into().unwrap()), 0);
                break;
            }
            if msg_type == RTM_NEWROUTE && route_attr_u32(&out[..n], RTA_OIF) == Some(dev.ifindex) {
                match peer {
                    Some(SockAddr::Netlink { pid: 0, groups })
                        if groups == 1u32 << (RTNLGRP_IPV4_ROUTE - 1) =>
                    {
                        assert_eq!(u32::from_ne_bytes(out[8..12].try_into().unwrap()), 0);
                        assert_eq!(u32::from_ne_bytes(out[12..16].try_into().unwrap()), 0);
                        saw_multicast = true;
                        route_keys.push((route_attr_ipv4(&out[..n], RTA_DST), out[17]));
                    }
                    other => panic!("unexpected peer for route notification: {:?}", other),
                }
            }
        }

        assert!(
            saw_multicast,
            "subscribed requester should receive the kernel multicast route notification"
        );
        route_keys.sort_unstable();
        route_keys.dedup();
        assert_eq!(
            route_keys,
            alloc::vec![
                (Some([10, 0, 2, 0]), 24),
                (Some([10, 0, 2, 22]), 32),
                (Some([10, 0, 2, 255]), 32),
            ]
        );

        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_newroute_with_gateway_is_replayed_via_getroute() {
        let name = "rtroute0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x0a],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        ROUTE4S.lock().clear();

        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x602d,
                groups: 0,
            },
        )
        .unwrap();

        let mut addr = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        addr[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        addr[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        addr[8..12].copy_from_slice(&52u32.to_ne_bytes());
        addr[16] = AF_INET as u8;
        addr[17] = 24;
        addr[18] = IFA_F_PERMANENT as u8;
        addr[19] = RT_SCOPE_UNIVERSE;
        addr[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut addr, IFA_LOCAL, ipv4(10, 0, 2, 15));
        push_rta_ipv4(&mut addr, IFA_ADDRESS, ipv4(10, 0, 2, 15));
        push_rta_u32(&mut addr, IFA_FLAGS, IFA_F_PERMANENT);
        let addr_len = addr.len() as u32;
        addr[0..4].copy_from_slice(&addr_len.to_ne_bytes());
        sendto(&sock, &addr, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send addr");
        let mut ack = [0u8; 128];
        let _ = recv_until_ack(&sock, &mut ack, |_| {});

        let mut route = alloc::vec![0u8; NLMSG_HDRLEN + 12];
        route[4..6].copy_from_slice(&RTM_NEWROUTE.to_ne_bytes());
        route[6..8].copy_from_slice(&0x0605u16.to_ne_bytes());
        route[8..12].copy_from_slice(&53u32.to_ne_bytes());
        route[16] = AF_INET as u8;
        route[20] = RT_TABLE_MAIN;
        route[21] = RTPROT_BOOT;
        route[22] = RT_SCOPE_UNIVERSE;
        route[23] = RTN_UNICAST;
        push_rta_ipv4(&mut route, RTA_GATEWAY, ipv4(10, 0, 2, 2));
        let route_len = route.len() as u32;
        route[0..4].copy_from_slice(&route_len.to_ne_bytes());
        sendto(&sock, &route, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send route");
        let n = recv_until_ack(&sock, &mut ack, |_| {});
        assert_eq!(n, 36);
        assert_eq!(
            i32::from_ne_bytes(ack[16..20].try_into().unwrap()),
            0,
            "RTM_NEWROUTE should ACK success"
        );

        let mut dump = alloc::vec![0u8; NLMSG_HDRLEN + 12];
        dump[0..4].copy_from_slice(&((NLMSG_HDRLEN + 12) as u32).to_ne_bytes());
        dump[4..6].copy_from_slice(&RTM_GETROUTE.to_ne_bytes());
        dump[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        dump[8..12].copy_from_slice(&54u32.to_ne_bytes());
        dump[16] = AF_INET as u8;
        sendto(&sock, &dump, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send dump");

        let mut saw_default = false;
        let mut saw_done = false;
        for _ in 0..32 {
            let mut out = [0u8; 1024];
            let n = recvmsg(&sock, &mut out).expect("recv dump");
            let packet = &out[..n];
            let msg_type = u16::from_ne_bytes(packet[4..6].try_into().unwrap());
            if msg_type == RTM_NEWROUTE
                && packet[17] == 0
                && route_attr_ipv4(packet, RTA_GATEWAY) == Some([10, 0, 2, 2])
            {
                saw_default = true;
                assert_eq!(packet[20], RT_TABLE_MAIN);
                assert_eq!(packet[21], RTPROT_BOOT);
                assert_eq!(packet[22], RT_SCOPE_UNIVERSE);
                assert_eq!(packet[23], RTN_UNICAST);
                assert_eq!(route_attr_u32(packet, RTA_OIF), Some(dev.ifindex));
                assert_eq!(route_attr_ipv4(packet, RTA_PREFSRC), Some([10, 0, 2, 15]));
            } else if msg_type == NLMSG_DONE {
                saw_done = true;
                break;
            }
        }

        assert!(
            saw_default,
            "RTM_GETROUTE should replay the default gateway"
        );
        assert!(saw_done, "RTM_GETROUTE dump should complete");

        ROUTE4S.lock().clear();
        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_newroute_echo_notifies_requester() {
        let name = "rtrouteecho0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x1f],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        ROUTE4S.lock().clear();

        let sock = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &sock,
            SockAddr::Netlink {
                pid: 0x602e,
                groups: 0,
            },
        )
        .unwrap();

        let mut addr = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        addr[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        addr[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        addr[8..12].copy_from_slice(&60u32.to_ne_bytes());
        addr[16] = AF_INET as u8;
        addr[17] = 24;
        addr[18] = IFA_F_PERMANENT as u8;
        addr[19] = RT_SCOPE_UNIVERSE;
        addr[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut addr, IFA_LOCAL, ipv4(10, 0, 2, 25));
        push_rta_ipv4(&mut addr, IFA_ADDRESS, ipv4(10, 0, 2, 25));
        push_rta_u32(&mut addr, IFA_FLAGS, IFA_F_PERMANENT);
        let addr_len = addr.len() as u32;
        addr[0..4].copy_from_slice(&addr_len.to_ne_bytes());
        sendto(&sock, &addr, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send addr");
        let mut ack = [0u8; 128];
        let _ = recv_until_ack(&sock, &mut ack, |_| {});

        let mut route = alloc::vec![0u8; NLMSG_HDRLEN + 12];
        route[4..6].copy_from_slice(&RTM_NEWROUTE.to_ne_bytes());
        route[6..8].copy_from_slice(&(0x0605u16 | NLM_F_ECHO).to_ne_bytes());
        route[8..12].copy_from_slice(&61u32.to_ne_bytes());
        route[16] = AF_INET as u8;
        route[20] = RT_TABLE_MAIN;
        route[21] = RTPROT_BOOT;
        route[22] = RT_SCOPE_UNIVERSE;
        route[23] = RTN_UNICAST;
        push_rta_ipv4(&mut route, RTA_GATEWAY, ipv4(10, 0, 2, 2));
        let route_len = route.len() as u32;
        route[0..4].copy_from_slice(&route_len.to_ne_bytes());
        sendto(&sock, &route, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send route");
        let mut saw_notify = false;
        let n = recv_until_ack(&sock, &mut ack, |packet| {
            if u16::from_ne_bytes(packet[4..6].try_into().unwrap()) == RTM_NEWROUTE
                && packet[17] == 0
                && route_attr_ipv4(packet, RTA_GATEWAY) == Some([10, 0, 2, 2])
            {
                saw_notify = true;
                assert_eq!(u32::from_ne_bytes(packet[8..12].try_into().unwrap()), 61);
                assert_eq!(
                    u32::from_ne_bytes(packet[12..16].try_into().unwrap()),
                    0x602e
                );
            }
        });
        assert_eq!(n, 36);
        assert_eq!(i32::from_ne_bytes(ack[16..20].try_into().unwrap()), 0);
        assert!(
            saw_notify,
            "NLM_F_ECHO should report RTM_NEWROUTE to requester"
        );

        ROUTE4S.lock().clear();
        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn rtnetlink_newroute_multicast_is_kernel_origin() {
        let name = "rtroutegrp2";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 0x20],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("device");
        ROUTE4S.lock().clear();

        let requester = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &requester,
            SockAddr::Netlink {
                pid: 0x602f,
                groups: 0,
            },
        )
        .unwrap();
        let subscriber = socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE).unwrap();
        bind(
            &subscriber,
            SockAddr::Netlink {
                pid: 0x6030,
                groups: 1u32 << (RTNLGRP_IPV4_ROUTE - 1),
            },
        )
        .unwrap();

        let mut addr = alloc::vec![0u8; NLMSG_HDRLEN + 8];
        addr[4..6].copy_from_slice(&RTM_NEWADDR.to_ne_bytes());
        addr[6..8].copy_from_slice(&0x0005u16.to_ne_bytes());
        addr[8..12].copy_from_slice(&62u32.to_ne_bytes());
        addr[16] = AF_INET as u8;
        addr[17] = 24;
        addr[18] = IFA_F_PERMANENT as u8;
        addr[19] = RT_SCOPE_UNIVERSE;
        addr[20..24].copy_from_slice(&dev.ifindex.to_ne_bytes());
        push_rta_ipv4(&mut addr, IFA_LOCAL, ipv4(10, 0, 2, 26));
        push_rta_ipv4(&mut addr, IFA_ADDRESS, ipv4(10, 0, 2, 26));
        push_rta_u32(&mut addr, IFA_FLAGS, IFA_F_PERMANENT);
        let addr_len = addr.len() as u32;
        addr[0..4].copy_from_slice(&addr_len.to_ne_bytes());
        sendto(&requester, &addr, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send addr");
        let mut ack = [0u8; 128];
        let _ = recv_until_ack(&requester, &mut ack, |_| {});

        let mut route = alloc::vec![0u8; NLMSG_HDRLEN + 12];
        route[4..6].copy_from_slice(&RTM_NEWROUTE.to_ne_bytes());
        route[6..8].copy_from_slice(&0x0605u16.to_ne_bytes());
        route[8..12].copy_from_slice(&63u32.to_ne_bytes());
        route[16] = AF_INET as u8;
        route[20] = RT_TABLE_MAIN;
        route[21] = RTPROT_BOOT;
        route[22] = RT_SCOPE_UNIVERSE;
        route[23] = RTN_UNICAST;
        push_rta_ipv4(&mut route, RTA_GATEWAY, ipv4(10, 0, 2, 2));
        let route_len = route.len() as u32;
        route[0..4].copy_from_slice(&route_len.to_ne_bytes());
        sendto(&requester, &route, SockAddr::Netlink { pid: 0, groups: 0 }).expect("send route");
        let _ = recv_until_ack(&requester, &mut ack, |_| {});

        let mut out = [0u8; 1024];
        loop {
            let (n, peer) = recvfrom(&subscriber, &mut out).expect("multicast route");
            if u16::from_ne_bytes(out[4..6].try_into().unwrap()) == RTM_NEWROUTE
                && out[17] == 0
                && route_attr_ipv4(&out[..n], RTA_GATEWAY) == Some([10, 0, 2, 2])
            {
                assert_eq!(u32::from_ne_bytes(out[8..12].try_into().unwrap()), 0);
                assert_eq!(u32::from_ne_bytes(out[12..16].try_into().unwrap()), 0);
                assert_eq!(
                    peer,
                    Some(SockAddr::Netlink {
                        pid: 0,
                        groups: 1u32 << (RTNLGRP_IPV4_ROUTE - 1),
                    })
                );
                break;
            }
        }

        ROUTE4S.lock().clear();
        drop_rtnl_ifaddrs_for_device(dev.ifindex);
        crate::net::device::unregister_netdevice(name).expect("cleanup");
    }

    #[test]
    fn inet_dgram_send_recv() {
        let server_addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 5555,
        };
        let server = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        let _ = setsockopt(&server, SO_REUSEADDR, 1);
        bind(&server, server_addr.clone()).unwrap();
        connect(&client, server_addr).unwrap();

        assert_eq!(sendmsg(&client, b"ping").unwrap(), 4);
        let mut buf = [0u8; 8];
        assert_eq!(recvmsg(&server, &mut buf).unwrap(), 4);
        assert_eq!(&buf[..4], b"ping");
    }

    #[test]
    fn inet_dgram_sendto_recvfrom_without_connect() {
        let server_addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 5556,
        };
        let server = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        let _ = setsockopt(&server, SO_REUSEADDR, 1);
        bind(&server, server_addr.clone()).unwrap();

        assert_eq!(sendto(&client, b"ping", server_addr).unwrap(), 4);
        let mut buf = [0u8; 8];
        let (len, peer) = recvfrom(&server, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(peer, None);
        assert_eq!(&buf[..4], b"ping");
    }

    #[test]
    fn inet_reuseaddr_requires_existing_socket_opt_in() {
        let addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 45678,
        };
        let victim = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        let attacker = socket(AF_INET, SOCK_DGRAM, 0).unwrap();

        bind(&victim, addr.clone()).unwrap();
        assert_eq!(setsockopt(&attacker, SO_REUSEADDR, 1), Ok(()));

        assert_eq!(bind(&attacker, addr), Err(EADDRINUSE));
    }

    #[test]
    fn inet_reuseaddr_does_not_replace_existing_binding() {
        let addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 45679,
        };
        let victim = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        let attacker = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        assert_eq!(setsockopt(&victim, SO_REUSEADDR, 1), Ok(()));
        assert_eq!(setsockopt(&attacker, SO_REUSEADDR, 1), Ok(()));

        bind(&victim, addr.clone()).unwrap();
        bind(&attacker, addr.clone()).unwrap();

        assert_eq!(sendto(&client, b"secret", addr).unwrap(), 6);
        let mut buf = [0u8; 8];
        assert_eq!(recvmsg(&victim, &mut buf).unwrap(), 6);
        assert_eq!(&buf[..6], b"secret");
        assert_eq!(recvmsg(&attacker, &mut buf), Err(EAGAIN));
    }

    // AF_INET loopback stream rendezvous mirrors Linux inet_stream_connect()
    // + inet_csk_accept(): a connect to a bound listener queues a fresh
    // connected socket on the listener backlog (vendor/linux/net/ipv4/af_inet.c,
    // vendor/linux/net/ipv4/inet_connection_sock.c). sshd over 127.0.0.1
    // depends on this whole group.

    #[test]
    fn inet_stream_connect_queues_accepted_socket_on_loopback_listener() {
        let addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 2201,
        };
        let listener = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();

        connect(&client, addr.clone()).unwrap();
        let accepted = accept4(&listener).unwrap();
        assert_eq!(
            accept4(&listener).err(),
            Some(EAGAIN),
            "only one pending connection was queued"
        );

        let client_local = client.lock().local.clone();
        assert!(
            matches!(client_local, Some(SockAddr::Inet { .. })),
            "client must be autobound to an ephemeral inet address before accept"
        );
        let accepted = accepted.lock();
        assert_eq!(accepted.state, SocketState::Connected);
        assert_eq!(accepted.local, Some(addr));
        assert_eq!(accepted.peer, client_local);
        assert_eq!(client.lock().state, SocketState::Connected);
    }

    #[test]
    fn inet_stream_loopback_round_trip_delivers_bytes_both_directions() {
        let addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 2202,
        };
        let listener = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();
        connect(&client, addr).unwrap();
        let accepted = accept4(&listener).unwrap();

        assert_eq!(sendmsg(&client, b"syn").unwrap(), 3);
        let mut buf = [0u8; 8];
        assert_eq!(recvmsg(&accepted, &mut buf).unwrap(), 3);
        assert_eq!(&buf[..3], b"syn");

        assert_eq!(sendmsg(&accepted, b"ack!").unwrap(), 4);
        assert_eq!(recvmsg(&client, &mut buf).unwrap(), 4);
        assert_eq!(&buf[..4], b"ack!");

        assert!(
            listener.lock().recvq.is_empty(),
            "stream data must not make the listening socket readable"
        );
    }

    #[test]
    fn inet_stream_connect_to_wildcard_listener_matches_bound_port() {
        // Linux __inet_lookup_listener() falls back to the INADDR_ANY
        // listener when no exact-address listener matches.
        let listener = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        bind(
            &listener,
            SockAddr::Inet {
                addr: 0,
                port: 2203,
            },
        )
        .unwrap();
        listen(&listener).unwrap();

        let dialed = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 2203,
        };
        connect(&client, dialed.clone()).unwrap();
        let accepted = accept4(&listener).unwrap();
        assert_eq!(
            accepted.lock().local,
            Some(dialed),
            "accepted socket reports the address the client dialed"
        );
    }

    #[test]
    fn inet_stream_connect_to_loopback_without_listener_is_refused() {
        // Linux answers a SYN to a closed loopback port with RST ->
        // ECONNREFUSED (vendor/linux/net/ipv4/tcp_ipv4.c::tcp_v4_send_reset).
        let client = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        assert_eq!(
            connect(
                &client,
                SockAddr::Inet {
                    addr: ipv4(127, 0, 0, 1),
                    port: 2204,
                },
            ),
            Err(ECONNREFUSED)
        );

        // With no device registered in this unit-test process, an external
        // connect reports the same ENETDOWN error as Linux.
        let external = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        assert_eq!(
            connect(
                &external,
                SockAddr::Inet {
                    addr: ipv4(10, 0, 2, 2),
                    port: 80,
                },
            ),
            Err(ENETDOWN)
        );
    }

    #[test]
    fn inet6_connect_without_route_is_network_unreachable() {
        // Linux resolves IPv6 destinations through ip6_dst_lookup_flow()
        // (vendor/linux/net/ipv6/tcp_ipv6.c::tcp_v6_connect,
        // vendor/linux/net/ipv6/datagram.c::__ip6_datagram_connect).  With no
        // IPv6 route configured both stream and datagram connects fail with
        // ENETUNREACH; glibc's getaddrinfo() RFC 6724 probe and curl's
        // happy-eyeballs IPv4 fallback depend on that errno instead of a fake
        // instant "success" that EPIPEs on the first send.
        let mut global = [0u8; 16];
        global[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
        global[15] = 1;
        for sock_type in [SOCK_STREAM, SOCK_DGRAM] {
            let sock = socket(AF_INET6, sock_type, 0).unwrap();
            assert_eq!(
                connect(
                    &sock,
                    SockAddr::Inet6 {
                        addr: global,
                        port: 80,
                    },
                ),
                Err(ENETUNREACH)
            );
        }

        // sendto() to an unreachable IPv6 destination reports the same errno
        // (vendor/linux/net/ipv6/udp.c::udpv6_sendmsg).
        let udp = socket(AF_INET6, SOCK_DGRAM, 0).unwrap();
        assert_eq!(
            sendto(
                &udp,
                b"probe",
                SockAddr::Inet6 {
                    addr: global,
                    port: 53,
                },
            ),
            Err(ENETUNREACH)
        );

        // Loopback keeps its Linux semantics: a SYN to a closed [::1] port is
        // refused, not unreachable, and `::` means `::1` (the BSD'ism in
        // tcp_v6_connect()).
        let mut loopback = [0u8; 16];
        loopback[15] = 1;
        let stream = socket(AF_INET6, SOCK_STREAM, 0).unwrap();
        assert_eq!(
            connect(
                &stream,
                SockAddr::Inet6 {
                    addr: loopback,
                    port: 2206,
                },
            ),
            Err(ECONNREFUSED)
        );
        let any = socket(AF_INET6, SOCK_STREAM, 0).unwrap();
        assert_eq!(
            connect(
                &any,
                SockAddr::Inet6 {
                    addr: [0u8; 16],
                    port: 2206,
                },
            ),
            Err(ECONNREFUSED)
        );

        // udpv6_sendmsg() applies the same any-address normalization. Prove
        // the datagram reaches a local ::1 binding instead of being rejected
        // as an unreachable all-zero destination.
        let receiver = socket(AF_INET6, SOCK_DGRAM, 0).unwrap();
        bind(
            &receiver,
            SockAddr::Inet6 {
                addr: loopback,
                port: 2207,
            },
        )
        .unwrap();
        let sender = socket(AF_INET6, SOCK_DGRAM, 0).unwrap();
        assert_eq!(
            sendto(
                &sender,
                b"any",
                SockAddr::Inet6 {
                    addr: [0u8; 16],
                    port: 2207,
                },
            ),
            Ok(3)
        );
        let mut received = [0u8; 8];
        assert_eq!(recvfrom(&receiver, &mut received).unwrap().0, 3);
        assert_eq!(&received[..3], b"any");

        // Address-family validation happens before :: normalization. Linux
        // returns EAFNOSUPPORT for sockaddr_in6 on an AF_INET socket.
        let inet_stream = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        assert_eq!(
            connect(
                &inet_stream,
                SockAddr::Inet6 {
                    addr: [0u8; 16],
                    port: 2208,
                },
            ),
            Err(EAFNOSUPPORT)
        );
        let inet_udp = socket(AF_INET, SOCK_DGRAM, 0).unwrap();
        assert_eq!(
            sendto(
                &inet_udp,
                b"wrong-family",
                SockAddr::Inet6 {
                    addr: global,
                    port: 53,
                },
            ),
            Err(EAFNOSUPPORT)
        );

        // SockAddr currently has no scope-id field, so a link-local address
        // is necessarily unscoped and ip6_dst_lookup_flow() returns EINVAL.
        let mut link_local = [0u8; 16];
        link_local[0] = 0xfe;
        link_local[1] = 0x80;
        link_local[15] = 1;
        let link_stream = socket(AF_INET6, SOCK_STREAM, 0).unwrap();
        assert_eq!(
            connect(
                &link_stream,
                SockAddr::Inet6 {
                    addr: link_local,
                    port: 80,
                },
            ),
            Err(EINVAL)
        );
        let link_udp = socket(AF_INET6, SOCK_DGRAM, 0).unwrap();
        assert_eq!(
            sendto(
                &link_udp,
                b"unscoped",
                SockAddr::Inet6 {
                    addr: link_local,
                    port: 53,
                },
            ),
            Err(EINVAL)
        );
    }

    #[test]
    fn inet_stream_peer_close_yields_eof_not_eagain() {
        let addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 2205,
        };
        let listener = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();
        connect(&client, addr).unwrap();
        let accepted = accept4(&listener).unwrap();

        assert_eq!(sendmsg(&accepted, b"bye").unwrap(), 3);
        drop(accepted);

        let mut buf = [0u8; 8];
        assert_eq!(
            recvmsg(&client, &mut buf).unwrap(),
            3,
            "bytes queued before the close are still delivered"
        );
        assert_eq!(
            recvmsg(&client, &mut buf).unwrap(),
            0,
            "peer close must yield EOF (read 0) like Linux tcp_recvmsg, not EAGAIN"
        );
    }

    #[test]
    fn inet_stream_empty_connected_queue_returns_eagain_until_data_or_eof() {
        let addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 2206,
        };
        let listener = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_INET, SOCK_STREAM, 0).unwrap();
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();
        connect(&client, addr).unwrap();
        let _accepted = accept4(&listener).unwrap();

        let mut buf = [0u8; 8];
        assert_eq!(
            recvmsg(&client, &mut buf),
            Err(EAGAIN),
            "connected stream sockets with no queued bytes must report EAGAIN, not EOF"
        );
    }

    #[test]
    fn unix_socketpair_delivers_bytes_between_peers() {
        let (left, right) = socketpair(AF_UNIX, SOCK_STREAM, 0).unwrap();
        assert_eq!(sendmsg(&left, b"pair").unwrap(), 4);
        let mut buf = [0u8; 8];
        assert_eq!(recvmsg(&right, &mut buf).unwrap(), 4);
        assert_eq!(&buf[..4], b"pair");
    }

    #[test]
    fn unix_socketpair_release_breaks_held_peer_links() {
        let (left, right) = socketpair(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let left_weak = Arc::downgrade(&left);
        let right_weak = Arc::downgrade(&right);

        assert_eq!(Arc::strong_count(&left), 2);
        assert_eq!(Arc::strong_count(&right), 2);

        assert_eq!(sendmsg(&left, b"queued").unwrap(), 6);
        release_socket(&left);
        release_socket(&right);
        drop(left);
        drop(right);

        assert!(left_weak.upgrade().is_none());
        assert!(right_weak.upgrade().is_none());
    }

    #[test]
    fn unix_stream_partial_recv_preserves_remaining_bytes() {
        let (left, right) = socketpair(AF_UNIX, SOCK_STREAM, 0).unwrap();
        assert_eq!(
            sendmsg(&left, b"AUTH EXTERNAL 30\r\nNEGOTIATE_UNIX_FD\r\nBEGIN\r\n").unwrap(),
            44
        );

        let mut first = [0u8; 10];
        assert_eq!(recvmsg(&right, &mut first).unwrap(), first.len());
        assert_eq!(&first, b"AUTH EXTER");

        let mut rest = [0u8; 64];
        let n = recvmsg(&right, &mut rest).unwrap();
        assert_eq!(&rest[..n], b"NAL 30\r\nNEGOTIATE_UNIX_FD\r\nBEGIN\r\n");
    }

    #[test]
    fn unix_stream_recv_glues_multiple_queued_chunks_like_linux() {
        let (left, right) = socketpair(AF_UNIX, SOCK_STREAM, 0).unwrap();
        assert_eq!(sendmsg(&left, b"AUTH ").unwrap(), 5);
        assert_eq!(sendmsg(&left, b"EXTERNAL").unwrap(), 8);

        let mut buf = [0u8; 13];
        let n = recvmsg(&right, &mut buf).unwrap();
        assert_eq!(n, 13);
        assert_eq!(&buf[..n], b"AUTH EXTERNAL");
    }

    #[test]
    fn unix_stream_msg_peek_glues_multiple_queued_chunks_without_consuming() {
        let (left, right) = socketpair(AF_UNIX, SOCK_STREAM, 0).unwrap();
        assert_eq!(sendmsg(&left, b"abc").unwrap(), 3);
        assert_eq!(sendmsg(&left, b"def").unwrap(), 3);

        let mut peeked = [0u8; 6];
        let (n, _, _, _, real_len, _) = recvmsg_full(&right, &mut peeked, MSG_PEEK).unwrap();
        assert_eq!(n, 6);
        assert_eq!(real_len, 6);
        assert_eq!(&peeked[..n], b"abcdef");

        let mut consumed = [0u8; 6];
        let n = recvmsg(&right, &mut consumed).unwrap();
        assert_eq!(n, 6);
        assert_eq!(&consumed[..n], b"abcdef");
    }

    #[test]
    fn unix_stream_recv_stops_at_scm_rights_boundary_like_linux() {
        use crate::fs::dcache::d_alloc;
        use crate::fs::file::{alloc_file, fget, fput};
        use crate::fs::ops::NOOP_FILE_OPS;

        let (left, right) = socketpair(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let attached: FileRef = alloc_file(d_alloc("dbus-rights-boundary"), 0, 0, &NOOP_FILE_OPS);

        assert_eq!(
            sendmsg_with_fds(&left, b"A", alloc::vec![fget(&attached)]).unwrap(),
            1
        );
        assert_eq!(sendmsg(&left, b"B").unwrap(), 1);

        let mut buf = [0u8; 2];
        let (n, _peer, fds, _cred) = recvmsg_with_fds(&right, &mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(&buf[..n], b"A");
        assert_eq!(fds.len(), 1);
        assert!(Arc::ptr_eq(&fds[0], &attached));
        for file in fds {
            fput(file);
        }

        let n = recvmsg(&right, &mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(&buf[..n], b"B");
    }

    #[test]
    fn unix_socketpair_carries_scm_rights_fileref_through_sendmsg_with_fds() {
        use crate::fs::dcache::d_alloc;
        use crate::fs::file::{alloc_file, fget, fput};
        use crate::fs::ops::NOOP_FILE_OPS;

        let (left, right) = socketpair(AF_UNIX, SOCK_DGRAM, 0).unwrap();
        let dentry = d_alloc("journal-sock");
        let attached: FileRef = alloc_file(dentry, 0, 0, &NOOP_FILE_OPS);

        // Sender bundles a file reference into the cmsg payload.
        assert_eq!(
            sendmsg_with_fds(&left, b"fd!", alloc::vec![fget(&attached)]).unwrap(),
            3
        );

        // Receiver pops the packet and gets the exact same FileRef back.
        let mut buf = [0u8; 8];
        let (n, _peer, fds, _cred) = recvmsg_with_fds(&right, &mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], b"fd!");
        assert_eq!(fds.len(), 1, "SCM_RIGHTS payload must travel intact");
        assert!(
            Arc::ptr_eq(&fds[0], &attached),
            "receiver gets the same Arc<File>, not a clone of the bytes"
        );
        for file in fds {
            fput(file);
        }
    }

    #[test]
    fn unix_stream_accepts_pending_connection() {
        let addr = SockAddr::Unix(String::from("/sock-test"));
        let listener = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let _ = setsockopt(&listener, SO_REUSEADDR, 1);
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();
        connect(&client, addr).unwrap();
        assert!(accept4(&listener).is_ok());
    }

    #[test]
    fn unix_stream_connect_queues_server_side_socket_for_accept() {
        let addr = SockAddr::Unix(String::from("/sock-accept-data"));
        let listener = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let _ = setsockopt(&listener, SO_REUSEADDR, 1);
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();

        connect(&client, addr).unwrap();
        let accepted = accept4(&listener).unwrap();
        assert_eq!(
            accept4(&listener).err(),
            Some(crate::include::uapi::errno::EAGAIN)
        );

        assert_eq!(sendmsg(&client, b"log").unwrap(), 3);
        assert!(
            listener.lock().recvq.is_empty(),
            "stream data must not make the listening socket readable"
        );

        let mut out = [0u8; 8];
        assert_eq!(recvmsg(&accepted, &mut out).unwrap(), 3);
        assert_eq!(&out[..3], b"log");
    }

    #[test]
    fn unix_release_socket_closes_backlog_and_unbinds_listener() {
        let addr = SockAddr::Unix(String::from("/sock-release-backlog"));
        let listener = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let client = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();
        connect(&client, addr.clone()).unwrap();
        assert_eq!(listener.lock().backlog.len(), 1);

        release_socket(&listener);
        assert_eq!(listener.lock().state, SocketState::Closed);
        assert!(listener.lock().backlog.is_empty());

        let replacement = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        assert_eq!(bind(&replacement, addr), Ok(()));

        let mut out = [0u8; 1];
        assert_eq!(recvmsg(&client, &mut out), Err(ECONNRESET));
        assert_eq!(
            sendmsg(&client, b"x"),
            Err(crate::include::uapi::errno::EPIPE)
        );
    }

    #[test]
    fn unix_stream_send_after_peer_release_returns_epipe() {
        let (left, right) = socketpair(AF_UNIX, SOCK_STREAM, 0).unwrap();
        release_socket(&right);

        assert_eq!(
            sendmsg(&left, b"x"),
            Err(crate::include::uapi::errno::EPIPE)
        );
        let mut out = [0u8; 1];
        assert_eq!(recvmsg(&left, &mut out), Ok(0));
    }

    #[test]
    fn unix_stream_connect_captures_peer_credentials() {
        use crate::kernel::{
            cred::{INIT_CRED, KGid, KUid, commit_creds, prepare_creds},
            sched,
            task::TaskStruct,
        };
        use alloc::boxed::Box;

        let previous = unsafe { sched::get_current() };
        let mut server = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        server.pid = 700;
        server.tgid = 700;
        server.cred = &raw const INIT_CRED;
        let mut client_task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        client_task.pid = 799;
        client_task.tgid = 701;
        client_task.cred = &raw const INIT_CRED;

        unsafe { sched::set_current(&mut *server as *mut TaskStruct) };
        let listener = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        let addr = SockAddr::Unix(String::from("/peercred-capture"));
        bind(&listener, addr.clone()).unwrap();
        listen(&listener).unwrap();

        unsafe { sched::set_current(&mut *client_task as *mut TaskStruct) };
        let new = prepare_creds().expect("prepare creds");
        unsafe {
            (*new).uid = KUid(1000);
            (*new).gid = KGid(1000);
            (*new).euid = KUid(1000);
            (*new).egid = KGid(1000);
        }
        commit_creds(new);
        let client = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();

        let new = prepare_creds().expect("prepare creds");
        unsafe {
            (*new).uid = KUid(1000);
            (*new).gid = KGid(1000);
            (*new).euid = KUid(0);
            (*new).egid = KGid(0);
        }
        commit_creds(new);
        connect(&client, addr.clone()).unwrap();

        unsafe { sched::set_current(&mut *server as *mut TaskStruct) };
        let accepted = accept4(&listener).unwrap();
        assert_eq!(accepted.lock().peer_cred.as_ref().unwrap().pid, 701);
        assert_eq!(
            accepted.lock().peer_cred.as_ref().unwrap().uid,
            0,
            "SO_PEERCRED stores effective uid, not real uid"
        );
        assert_eq!(client.lock().peer_cred.as_ref().unwrap().pid, 700);

        unbind_unix_path("/peercred-capture");
        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn unix_scm_credentials_follow_linux_tgid_and_real_ids() {
        use crate::kernel::{
            cred::{INIT_CRED, KGid, KUid, commit_creds, prepare_creds},
            sched,
            task::TaskStruct,
        };
        use alloc::boxed::Box;

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 812;
        current.tgid = 800;
        current.cred = &raw const INIT_CRED;

        unsafe { sched::set_current(&mut *current as *mut TaskStruct) };
        let new = prepare_creds().expect("prepare creds");
        unsafe {
            (*new).uid = KUid(1000);
            (*new).gid = KGid(1001);
            (*new).euid = KUid(0);
            (*new).egid = KGid(0);
        }
        commit_creds(new);

        let cred = current_scm_cred();
        assert_eq!(cred.pid, 800);
        assert_eq!(cred.uid, 1000);
        assert_eq!(cred.gid, 1001);

        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn unix_release_bound_socket_allows_path_rebind() {
        let addr = SockAddr::Unix(String::from("/sock-release-rebind"));
        let first = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        bind(&first, addr.clone()).unwrap();

        let second = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        assert_eq!(bind(&second, addr.clone()), Err(EADDRINUSE));

        release_bound_socket(&first);
        assert_eq!(bind(&second, addr), Ok(()));
    }

    #[test]
    fn unix_socket_plymouth_probe_shape() {
        let sock = socket(AF_UNIX, SOCK_STREAM, 0).unwrap();
        assert_eq!(setsockopt(&sock, SO_PASSCRED, 1), Ok(()));
        assert_eq!(
            connect(&sock, SockAddr::Unix(String::from("\0plymouth"))),
            Err(crate::include::uapi::errno::ECONNREFUSED)
        );
    }

    #[test]
    fn inet_bind_rejects_nonlocal_source_address() {
        let sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP as u16).unwrap();
        assert_eq!(
            bind(
                &sock,
                SockAddr::Inet {
                    addr: ipv4(203, 0, 113, 66),
                    port: 12345,
                },
            ),
            Err(EADDRNOTAVAIL)
        );
    }

    #[test]
    fn inet_bind_accepts_address_covered_by_loopback_local_route() {
        let addr = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 53),
            port: 45680,
        };
        let sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP as u16).unwrap();

        assert_eq!(bind(&sock, addr), Ok(()));
        assert!(inet_addr_is_local(ipv4(127, 0, 0, 53), None));

        // Keep the global bind table independent of this regression test.
        release_bound_socket(&sock);
    }

    #[test]
    fn inet_bind_and_transmit_source_use_distinct_linux_address_rules() {
        let multicast = ipv4(239, 1, 2, 3);
        let limited_broadcast = u32::MAX;

        // inet_addr_valid_or_nonlocal() permits these receive addresses.
        assert!(inet_addr_is_bindable(multicast, 0));
        assert!(inet_addr_is_bindable(limited_broadcast, 0));

        // ip_route_output_key_hash_rcu() rejects them as packet sources.
        assert!(!inet_addr_is_local(multicast, None));
        assert!(!inet_addr_is_local(limited_broadcast, None));
        assert!(!inet_addr_is_local(ipv4(203, 0, 113, 66), None));

        // This vendor tree's ipv4_is_zeronet() is `addr == 0`, not 0/8.
        assert!(!inet_addr_is_bindable(ipv4(0, 1, 2, 3), 0));

        // Lupos's synthetic slirp assignment belongs only to init_net.
        assert!(qemu_guest_addr_is_builtin_local(qemu_guest_ipv4(), 0));
        assert!(!qemu_guest_addr_is_builtin_local(
            qemu_guest_ipv4(),
            usize::MAX
        ));
    }

    #[test]
    fn journald_unix_socket_options_match_linux_core_sockopt() {
        let sock = socket(AF_UNIX, SOCK_DGRAM, 0).unwrap();

        assert_eq!(setsockopt(&sock, SO_PASSCRED, 1), Ok(()));
        assert_eq!(getsockopt(&sock, SO_PASSCRED), Ok(1));
        assert_eq!(setsockopt(&sock, SO_SNDBUFFORCE, 8 * 1024 * 1024), Ok(()));
        assert_eq!(getsockopt(&sock, SO_SNDBUFFORCE), Ok(212_992));
        assert_eq!(setsockopt(&sock, SO_RCVBUFFORCE, 8 * 1024 * 1024), Ok(()));
        assert_eq!(getsockopt(&sock, SO_RCVBUFFORCE), Ok(212_992));
        assert_eq!(setsockopt(&sock, SO_PASSRIGHTS, 0), Ok(()));
        assert_eq!(getsockopt(&sock, SO_PASSRIGHTS), Ok(0));
        assert_eq!(setsockopt(&sock, SO_TIMESTAMP_OLD, 1), Ok(()));
        assert_eq!(getsockopt(&sock, SO_TIMESTAMP_OLD), Ok(1));
        assert_eq!(setsockopt(&sock, SO_TIMESTAMP_NEW, 1), Ok(()));
        assert_eq!(getsockopt(&sock, SO_TIMESTAMP_NEW), Ok(1));

        let inet = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP as u16).unwrap();
        assert_eq!(get_recv_ttl(&inet), Ok(0));
        assert_eq!(set_recv_ttl(&inet, 1), Ok(()));
        assert_eq!(get_recv_ttl(&inet), Ok(1));
    }

    #[test]
    fn passsec_reports_unsupported_without_security_network() {
        let sock = socket(AF_UNIX, SOCK_DGRAM, 0).unwrap();
        assert_eq!(
            setsockopt(&sock, SO_PASSSEC, 1),
            Err(crate::include::uapi::errno::EOPNOTSUPP)
        );
    }

    #[test]
    fn qemu_dns_query_synthesizes_a_record() {
        let resolver = SockAddr::Inet {
            addr: qemu_dns_ipv4(),
            port: 53,
        };
        let client = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP as u16).unwrap();
        let query = [
            0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 7, b'e', b'x',
            b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0, 0x00, 0x01, 0x00, 0x01,
        ];
        assert_eq!(
            sendto(&client, &query, resolver.clone()).unwrap(),
            query.len()
        );
        let mut buf = [0u8; 96];
        let (len, peer) = recvfrom(&client, &mut buf).unwrap();
        assert_eq!(peer, Some(resolver));
        assert_eq!(&buf[0..2], &[0x12, 0x34]);
        assert_eq!(&buf[6..8], &[0x00, 0x01]);
        assert!(buf[..len].ends_with(&[93, 184, 216, 34]));
    }

    #[test]
    fn icmp_echo_to_external_peer_synthesizes_reply() {
        let peer = SockAddr::Inet {
            addr: ipv4(93, 184, 216, 34),
            port: 0,
        };
        let client = socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP as u16).unwrap();
        let mut echo = Vec::from([8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01, b'p', b'i', b'n', b'g']);
        let csum = checksum(&echo);
        echo[2..4].copy_from_slice(&csum.to_be_bytes());

        assert_eq!(sendto(&client, &echo, peer.clone()).unwrap(), echo.len());
        let mut buf = [0u8; 64];
        let (len, from) = recvfrom(&client, &mut buf).unwrap();
        assert_eq!(from, Some(peer));
        assert_eq!(buf[0], 0);
        assert_eq!(&buf[4..len], &echo[4..]);
    }
}
