//! linux-parity: partial
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! Socket syscall glue for Linux networking syscalls.
//!
//! Ref: `vendor/linux/net/socket.c`.  This layer owns fd lifetime and UAPI
//! copying; `net::socket` owns the in-kernel socket state machine.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::kernel::uaccess;
use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::fdtable::FilesStruct;
use crate::fs::ops::{FileOps, NOOP_FILE_OPS, NOOP_INODE_OPS};
use crate::fs::types::{FileRef, Inode, InodeKind, InodePrivate};
use crate::include::uapi::errno::{
    EADDRINUSE, EAGAIN, EBADF, EFAULT, EINPROGRESS, EINTR, EINVAL, ENODEV, ENOENT, ENOPROTOOPT,
    ENOSYS, ENOTCONN, ENOTDIR, ENOTTY, EOPNOTSUPP, EPERM, ERANGE,
};
use crate::include::uapi::fcntl::{O_NONBLOCK, O_RDWR};
use crate::kernel::capability::{CAP_NET_ADMIN, CAP_NET_RAW, capable};
use crate::kernel::{files, sched};
use crate::security::security_socket_create;

use super::socket::{
    self, AF_INET, AF_INET6, AF_NETLINK, AF_PACKET, AF_UNIX, SockAddr, SocketRef, SocketState,
};

const MAX_RW: usize = 1 << 20;
const UIO_MAXIOV: usize = 1024;

// ── cmsg / SCM_RIGHTS constants ─────────────────────────────────────────────
//
// Linux UAPI: vendor/linux/include/linux/socket.h
//   struct cmsghdr {
//       __kernel_size_t cmsg_len;    // size_t (8B on x86_64)
//       int             cmsg_level;
//       int             cmsg_type;
//   };
// Total = 16 bytes; CMSG alignment is sizeof(size_t) = 8.
const SOL_IP: i32 = 0;
const SOL_SOCKET: i32 = 1;
const SOL_IPV6: i32 = 41;
const SOL_PACKET: i32 = 263;
const SOL_NETLINK: i32 = 270;
const SCM_RIGHTS: i32 = 1;
const SCM_CREDENTIALS: i32 = 2;
const SCM_PIDFD: i32 = 4;
const IP_TTL: i32 = 2;
const IP_PKTINFO: i32 = 8;
const IP_MTU_DISCOVER: i32 = 10;
const IP_RECVERR: i32 = 11;
const IP_MTU: i32 = 14;
const IP_UNICAST_IF: i32 = 50;
const IP_RECVFRAGSIZE: i32 = 25;
const IPV6_MTU: i32 = 24;
const IPV6_RECVERR: i32 = 25;
const IPV6_RECVPKTINFO: i32 = 49;
const IPV6_UNICAST_IF: i32 = 76;
const SO_BINDTODEVICE: i32 = 25;
const CMSG_HDR_LEN: usize = 16;
// Linux caps SCM_RIGHTS payloads at 253 descriptors per message.
const SCM_MAX_FD: usize = 253;
// Bound user-supplied ancillary data to keep sys_sendmsg parsing work finite.
const SCM_MAX_CONTROL_LEN: usize = 64 * 1024;
const MSG_CTRUNC: i32 = 0x0000_0008;
// MSG_PEEK / MSG_TRUNC bits from `vendor/linux/include/uapi/asm-generic/
// socket.h` — used by systemd's `sd-netlink::socket_read_message` to size
// the next datagram before consuming it.  Ref:
// `vendor/linux/net/socket.c::sock_recvmsg`.
const MSG_PEEK: i32 = 0x0000_0002;
const MSG_TRUNC: i32 = 0x0000_0020;
const MSG_DONTWAIT: i32 = 0x0000_0040;
const MSG_CMSG_CLOEXEC: i32 = 0x4000_0000;
const NETLINK_ADD_MEMBERSHIP: i32 = 1;
const NETLINK_DROP_MEMBERSHIP: i32 = 2;
const NETLINK_PKTINFO: i32 = 3;
const NETLINK_LIST_MEMBERSHIPS: i32 = 9;
const RTNLGRP_MAX: usize = 39;
const IFNAMSIZ: usize = 16;
const IFREQ_DATA_LEN: usize = 24;
const SIOCGIFNAME: u32 = 0x8910;
const SIOCGIFFLAGS: u32 = 0x8913;
const SIOCSIFFLAGS: u32 = 0x8914;
const SIOCGIFMTU: u32 = 0x8921;
const SIOCGIFHWADDR: u32 = 0x8927;
const SIOCGIFINDEX: u32 = 0x8933;
const SIOCETHTOOL: u32 = 0x8946;
const ARPHRD_ETHER: u16 = 1;
const ARPHRD_LOOPBACK: u16 = 772;
const ETHTOOL_GDRVINFO: u32 = 0x0000_0003;
const ETHTOOL_GLINK: u32 = 0x0000_000a;
const ETHTOOL_GLINKSETTINGS: u32 = 0x0000_004c;
const DUPLEX_FULL: u8 = 0x01;
const PORT_OTHER: u8 = 0xff;

fn decode_unicast_if_sockopt(value: u32) -> u32 {
    u32::from_be(value)
}

fn encode_unicast_if_sockopt(ifindex: u32) -> u32 {
    ifindex.to_be()
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxIfreq {
    ifr_name: [u8; IFNAMSIZ],
    ifru: [u8; IFREQ_DATA_LEN],
}

impl LinuxIfreq {
    fn ifname(&self) -> String {
        let len = self
            .ifr_name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.ifr_name.len());
        let name = &self.ifr_name[..len];
        let trimmed = match name.iter().position(|byte| *byte == b':') {
            Some(colon) => &name[..colon],
            None => name,
        };
        String::from_utf8_lossy(trimmed).into_owned()
    }

    fn set_ifname(&mut self, name: &str) {
        self.ifr_name.fill(0);
        let bytes = name.as_bytes();
        let len = bytes.len().min(IFNAMSIZ.saturating_sub(1));
        self.ifr_name[..len].copy_from_slice(&bytes[..len]);
    }

    fn ifindex(&self) -> i32 {
        i32::from_ne_bytes(self.ifru[..4].try_into().unwrap())
    }

    fn set_ifindex(&mut self, ifindex: i32) {
        self.ifru[..4].copy_from_slice(&ifindex.to_ne_bytes());
    }

    fn flags(&self) -> u16 {
        u16::from_ne_bytes(self.ifru[..2].try_into().unwrap())
    }

    fn set_flags(&mut self, flags: u16) {
        self.ifru[..2].copy_from_slice(&flags.to_ne_bytes());
    }

    fn set_mtu(&mut self, mtu: i32) {
        self.ifru[..4].copy_from_slice(&mtu.to_ne_bytes());
    }

    fn data_ptr(&self) -> *mut u8 {
        usize::from_ne_bytes(self.ifru[..8].try_into().unwrap()) as *mut u8
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxIfSockaddr {
    sa_family: u16,
    sa_data: [u8; 14],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxEthtoolDrvinfo {
    cmd: u32,
    driver: [u8; 32],
    version: [u8; 32],
    fw_version: [u8; 32],
    bus_info: [u8; 32],
    erom_version: [u8; 32],
    reserved2: [u8; 12],
    n_priv_flags: u32,
    n_stats: u32,
    testinfo_len: u32,
    eedump_len: u32,
    regdump_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxEthtoolValue {
    cmd: u32,
    data: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxEthtoolLinkSettings {
    cmd: u32,
    speed: u32,
    duplex: u8,
    port: u8,
    phy_address: u8,
    autoneg: u8,
    mdio_support: u8,
    eth_tp_mdix: u8,
    eth_tp_mdix_ctrl: u8,
    link_mode_masks_nwords: i8,
    transceiver: u8,
    master_slave_cfg: u8,
    master_slave_state: u8,
    rate_matching: u8,
    reserved: [u32; 7],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct LinuxUcred {
    pid: i32,
    uid: u32,
    gid: u32,
}

#[cfg(not(test))]
fn trace_current_ucred() -> LinuxUcred {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return LinuxUcred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
    }
    let cred = crate::kernel::cred::current_cred();
    if cred.is_null() {
        return LinuxUcred {
            pid: unsafe { (*task).pid },
            uid: 0,
            gid: 0,
        };
    }
    LinuxUcred {
        pid: unsafe { (*task).pid },
        uid: unsafe { (*cred).euid.0 },
        gid: unsafe { (*cred).egid.0 },
    }
}

#[cfg(not(test))]
fn trace_unix_interesting_path(path: &str) -> bool {
    path.contains("resolve.hook")
        || path.contains("/run/systemd/resolve")
        || path.contains("dbus-system-bus")
        || path.contains("/run/dbus/system_bus_socket")
}

#[cfg(not(test))]
fn trace_unix_socket_paths(sock: &SocketRef) -> (Option<String>, Option<String>) {
    let socket = sock.lock();
    let local = match socket.local.as_ref() {
        Some(SockAddr::Unix(path)) if trace_unix_interesting_path(path) => Some(path.clone()),
        _ => None,
    };
    let peer = match socket.peer.as_ref() {
        Some(SockAddr::Unix(path)) if trace_unix_interesting_path(path) => Some(path.clone()),
        _ => None,
    };
    (local, peer)
}

#[cfg(not(test))]
fn trace_payload_prefix(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes.iter().take(192) {
        match b {
            b'\n' => out.push('|'),
            b'\r' => out.push('~'),
            0x20..=0x7e => out.push(b as char),
            _ => out.push('.'),
        }
    }
    out
}

#[cfg(not(test))]
fn trace_unix_sendmsg(
    fd: i32,
    sock: &SocketRef,
    path: Option<&str>,
    bytes: &[u8],
    result: &Result<usize, i32>,
) {
    if !crate::kernel::debug_trace::proc_enabled() {
        return;
    }
    let (local, peer) = trace_unix_socket_paths(sock);
    let local = local.as_deref().or(path);
    let peer = peer.as_deref().or(path);
    if local.is_none() && peer.is_none() {
        return;
    }
    let cred = trace_current_ucred();
    let ret = match result {
        Ok(n) => *n as i64,
        Err(errno) => -(*errno as i64),
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-unix-sendmsg fd={} local={} peer={} bytes={} ret={} pid={} uid={} gid={} payload=\"{}\"",
        fd,
        local.unwrap_or("-"),
        peer.unwrap_or("-"),
        bytes.len(),
        ret,
        cred.pid,
        cred.uid,
        cred.gid,
        trace_payload_prefix(bytes)
    );
}

#[cfg(not(test))]
fn trace_notify_sendmsg(
    fd: i32,
    path: Option<&str>,
    bytes: &[u8],
    fd_count: usize,
    result: &Result<usize, i32>,
) {
    if !crate::kernel::debug_trace::cgroup_enabled() {
        return;
    }
    let Some(path) = path else {
        return;
    };
    if path != "/run/systemd/notify" {
        return;
    }
    let cred = trace_current_ucred();
    let ret = match result {
        Ok(n) => *n as i64,
        Err(errno) => -(*errno as i64),
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-notify-send fd={} bytes={} fds={} ret={} pid={} uid={} gid={} payload=\"{}\"",
        fd,
        bytes.len(),
        fd_count,
        ret,
        cred.pid,
        cred.uid,
        cred.gid,
        trace_payload_prefix(bytes)
    );
}

#[cfg(not(test))]
fn trace_unix_recvmsg(
    fd: i32,
    sock: &SocketRef,
    bytes: &[u8],
    packet_cred: &socket::SocketCred,
    passcred: bool,
    passpidfd: bool,
    fd_count: usize,
    controllen: usize,
    flags: i32,
) {
    if !crate::kernel::debug_trace::proc_enabled() {
        return;
    }
    let (local, peer) = trace_unix_socket_paths(sock);
    let payload = trace_payload_prefix(bytes);
    if local.is_none()
        && peer.is_none()
        && !payload.contains("READY=")
        && !payload.contains("STOPPING=")
    {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-unix-recvmsg fd={} local={} peer={} bytes={} fds={} packet_pid={} packet_uid={} packet_gid={} passcred={} passpidfd={} controllen={} flags={:#x} payload=\"{}\"",
        fd,
        local.as_deref().unwrap_or("-"),
        peer.as_deref().unwrap_or("-"),
        bytes.len(),
        fd_count,
        packet_cred.pid,
        packet_cred.uid,
        packet_cred.gid,
        passcred,
        passpidfd,
        controllen,
        flags,
        payload
    );
}

#[cfg(not(test))]
fn trace_notify_recvmsg(
    fd: i32,
    sock: &SocketRef,
    bytes: &[u8],
    packet_cred: &socket::SocketCred,
    passcred: bool,
    passpidfd: bool,
    fd_count: usize,
    controllen: usize,
    flags: i32,
) {
    if !crate::kernel::debug_trace::cgroup_enabled() {
        return;
    }
    let local = {
        let socket = sock.lock();
        match socket.local.as_ref() {
            Some(SockAddr::Unix(path)) if path == "/run/systemd/notify" => Some(path.clone()),
            _ => None,
        }
    };
    let Some(local) = local else {
        return;
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-notify-recv fd={} local={} bytes={} fds={} packet_pid={} packet_uid={} packet_gid={} passcred={} passpidfd={} controllen={} flags={:#x} payload=\"{}\"",
        fd,
        local,
        bytes.len(),
        fd_count,
        packet_cred.pid,
        packet_cred.uid,
        packet_cred.gid,
        passcred,
        passpidfd,
        controllen,
        flags,
        trace_payload_prefix(bytes)
    );
}

fn cmsg_align(n: usize) -> usize {
    n.checked_add(7).map(|n| n & !7).unwrap_or(usize::MAX)
}

unsafe fn write_cmsg_bytes(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    cmsg_type: i32,
    data: &[u8],
) -> Result<(usize, bool), i32> {
    unsafe { write_cmsg_bytes_level(control, controllen, offset, SOL_SOCKET, cmsg_type, data) }
}

unsafe fn write_cmsg_bytes_level(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    cmsg_level: i32,
    cmsg_type: i32,
    data: &[u8],
) -> Result<(usize, bool), i32> {
    if control.is_null() {
        return Ok((offset, true));
    }
    let total = CMSG_HDR_LEN + data.len();
    if offset.saturating_add(total) > controllen {
        return Ok((offset, true));
    }

    let mut cmsg_buf = alloc::vec![0u8; total];
    cmsg_buf[..8].copy_from_slice(&total.to_ne_bytes());
    cmsg_buf[8..12].copy_from_slice(&cmsg_level.to_ne_bytes());
    cmsg_buf[12..16].copy_from_slice(&cmsg_type.to_ne_bytes());
    cmsg_buf[CMSG_HDR_LEN..].copy_from_slice(data);

    let cmsg = control.wrapping_add(offset);
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(cmsg, cmsg_buf.as_ptr(), cmsg_buf.len())
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok((offset + cmsg_align(total), false))
}

#[derive(Default)]
struct SendMsgControl {
    scm_fds: alloc::vec::Vec<i32>,
    scm_cred: Option<socket::SocketCred>,
    inet_meta: Option<socket::PacketMeta>,
}

/// Walk a sender's `msghdr.control` buffer and harvest the send-side
/// control messages Lupos currently honors:
/// * `SOL_SOCKET / SCM_RIGHTS`
/// * `SOL_IP / IP_PKTINFO`
/// * `SOL_IP / IP_TTL`
///
/// # Safety
/// `control` is a userspace pointer; caller must ensure the page is
/// readable for `controllen` bytes.  Lupos's net layer does direct
/// user-pointer dereferences (no uaccess wrapper) — see
/// `copy_iov_bytes` at line ~242 for the established precedent.
unsafe fn parse_sendmsg_control(
    control: *const u8,
    controllen: usize,
) -> Result<SendMsgControl, i32> {
    if control.is_null() || controllen == 0 {
        return Ok(SendMsgControl::default());
    }
    if controllen > SCM_MAX_CONTROL_LEN {
        return Err(EINVAL);
    }

    let mut parsed = SendMsgControl::default();
    let mut off = 0usize;
    while off <= controllen && controllen - off >= CMSG_HDR_LEN {
        // Read cmsg_len (u64), cmsg_level (i32), cmsg_type (i32).
        let hdr_ptr = unsafe { control.add(off) };
        let cmsg_len = unsafe { core::ptr::read_unaligned(hdr_ptr as *const usize) };
        let cmsg_level = unsafe { core::ptr::read_unaligned(hdr_ptr.add(8) as *const i32) };
        let cmsg_type = unsafe { core::ptr::read_unaligned(hdr_ptr.add(12) as *const i32) };

        let cmsg_end = off.checked_add(cmsg_len).ok_or(EINVAL)?;
        if cmsg_len < CMSG_HDR_LEN || cmsg_end > controllen {
            return Err(EINVAL);
        }
        if cmsg_level == SOL_SOCKET && cmsg_type == SCM_RIGHTS {
            let data_len = cmsg_len - CMSG_HDR_LEN;
            if data_len % 4 != 0 {
                return Err(EINVAL);
            }
            let nfds = data_len / 4;
            if parsed.scm_fds.len().checked_add(nfds).ok_or(EINVAL)? > SCM_MAX_FD {
                return Err(EINVAL);
            }
            let data_ptr = unsafe { hdr_ptr.add(CMSG_HDR_LEN) as *const i32 };
            for i in 0..nfds {
                let fd = unsafe { core::ptr::read_unaligned(data_ptr.add(i)) };
                parsed.scm_fds.push(fd);
            }
        } else if cmsg_level == SOL_SOCKET && cmsg_type == SCM_CREDENTIALS {
            if cmsg_len != CMSG_HDR_LEN + core::mem::size_of::<LinuxUcred>() {
                return Err(EINVAL);
            }
            let ucred = unsafe {
                core::ptr::read_unaligned(hdr_ptr.add(CMSG_HDR_LEN) as *const LinuxUcred)
            };
            parsed.scm_cred = Some(socket::validate_unix_scm_credentials(
                ucred.pid, ucred.uid, ucred.gid,
            )?);
        } else if cmsg_level == SOL_IP {
            match cmsg_type {
                IP_PKTINFO => {
                    if cmsg_len != CMSG_HDR_LEN + core::mem::size_of::<LinuxInPktinfo>() {
                        return Err(EINVAL);
                    }
                    let info = unsafe {
                        core::ptr::read_unaligned(hdr_ptr.add(CMSG_HDR_LEN) as *const LinuxInPktinfo)
                    };
                    let meta = parsed
                        .inet_meta
                        .get_or_insert_with(socket::PacketMeta::default);
                    if info.ipi_ifindex != 0 {
                        meta.ifindex = info.ipi_ifindex;
                    }
                    if info.ipi_spec_dst != 0 {
                        meta.local_inet_addr = Some(u32::from_be(info.ipi_spec_dst));
                    }
                }
                IP_TTL => {
                    if cmsg_len != CMSG_HDR_LEN + core::mem::size_of::<i32>() {
                        return Err(EINVAL);
                    }
                    let ttl = unsafe {
                        core::ptr::read_unaligned(hdr_ptr.add(CMSG_HDR_LEN) as *const i32)
                    };
                    if !(1..=255).contains(&ttl) {
                        return Err(EINVAL);
                    }
                    parsed
                        .inet_meta
                        .get_or_insert_with(socket::PacketMeta::default)
                        .ttl = Some(ttl as u8);
                }
                _ => {}
            }
        }
        let step = cmsg_align(cmsg_len);
        if step == usize::MAX {
            return Err(EINVAL);
        }
        off = off.checked_add(step).ok_or(EINVAL)?;
    }
    Ok(parsed)
}

/// Serialize an `SCM_RIGHTS` cmsg into the receiver's `msghdr.control`
/// buffer.  Returns `(bytes_written, truncated)`.  If `controllen` is
/// too small the cmsg is dropped entirely and `truncated = true` so
/// the caller can OR `MSG_CTRUNC` into `msg.flags`.  No partial cmsg
/// writes — matches Linux's `put_cmsg(SCM_RIGHTS)` semantics.
///
/// # Safety
/// `control` is a userspace pointer; caller ensures writability for
/// `controllen` bytes.
unsafe fn write_scm_rights(
    control: *mut u8,
    controllen: usize,
    fds: &[i32],
) -> Result<(usize, bool), i32> {
    if fds.is_empty() {
        return Ok((0, false));
    }
    let payload_len = fds.len() * 4;
    let mut payload = alloc::vec![0u8; payload_len];
    for (i, fd) in fds.iter().enumerate() {
        payload[i * 4..i * 4 + 4].copy_from_slice(&fd.to_ne_bytes());
    }
    unsafe { write_cmsg_bytes(control, controllen, 0, SCM_RIGHTS, &payload) }
}

unsafe fn write_scm_rights_at(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    fds: &[i32],
) -> Result<(usize, bool), i32> {
    if fds.is_empty() {
        return Ok((offset, false));
    }
    let payload_len = fds.len() * 4;
    let mut payload = alloc::vec![0u8; payload_len];
    for (i, fd) in fds.iter().enumerate() {
        payload[i * 4..i * 4 + 4].copy_from_slice(&fd.to_ne_bytes());
    }
    unsafe { write_cmsg_bytes(control, controllen, offset, SCM_RIGHTS, &payload) }
}

unsafe fn write_scm_credentials_at(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    cred: &socket::SocketCred,
) -> Result<(usize, bool), i32> {
    let ucred = LinuxUcred {
        pid: cred.pid,
        uid: cred.uid,
        gid: cred.gid,
    };
    let data = unsafe {
        core::slice::from_raw_parts(
            &ucred as *const LinuxUcred as *const u8,
            core::mem::size_of::<LinuxUcred>(),
        )
    };
    unsafe { write_cmsg_bytes(control, controllen, offset, SCM_CREDENTIALS, data) }
}

unsafe fn write_scm_pidfd_at(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    pidfd: i32,
) -> Result<(usize, bool), i32> {
    unsafe { write_cmsg_bytes(control, controllen, offset, SCM_PIDFD, &pidfd.to_ne_bytes()) }
}

unsafe fn write_ipv4_ttl_at(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    ttl: u8,
) -> Result<(usize, bool), i32> {
    let ttl = i32::from(ttl).to_ne_bytes();
    unsafe { write_cmsg_bytes_level(control, controllen, offset, SOL_IP, IP_TTL, &ttl) }
}

unsafe fn write_ipv4_pktinfo_at(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    meta: &socket::PacketMeta,
) -> Result<(usize, bool), i32> {
    let pktinfo = LinuxInPktinfo {
        ipi_ifindex: meta.ifindex,
        ipi_spec_dst: meta
            .local_inet_addr
            .map(|addr| u32::from_ne_bytes(addr.to_be_bytes()))
            .unwrap_or(0),
        ipi_addr: meta
            .local_inet_addr
            .map(|addr| u32::from_ne_bytes(addr.to_be_bytes()))
            .unwrap_or(0),
    };
    let data = unsafe {
        core::slice::from_raw_parts(
            &pktinfo as *const LinuxInPktinfo as *const u8,
            core::mem::size_of::<LinuxInPktinfo>(),
        )
    };
    unsafe { write_cmsg_bytes_level(control, controllen, offset, SOL_IP, IP_PKTINFO, data) }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxNlPktinfo {
    group: u32,
}

unsafe fn write_netlink_pktinfo_at(
    control: *mut u8,
    controllen: usize,
    offset: usize,
    group: u32,
) -> Result<(usize, bool), i32> {
    let pktinfo = LinuxNlPktinfo { group };
    let data = unsafe {
        core::slice::from_raw_parts(
            &pktinfo as *const LinuxNlPktinfo as *const u8,
            core::mem::size_of::<LinuxNlPktinfo>(),
        )
    };
    unsafe {
        write_cmsg_bytes_level(
            control,
            controllen,
            offset,
            SOL_NETLINK,
            NETLINK_PKTINFO,
            data,
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxSockAddr {
    pub family: u16,
    pub data: [u8; 14],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxSockAddrIn {
    pub family: u16,
    pub port: u16,
    pub addr: u32,
    pub zero: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxSockAddrIn6 {
    pub family: u16,
    pub port: u16,
    pub flowinfo: u32,
    pub addr: [u8; 16],
    pub scope_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxInPktinfo {
    ipi_ifindex: u32,
    ipi_spec_dst: u32,
    ipi_addr: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxSockAddrNetlink {
    pub family: u16,
    pub pad: u16,
    pub pid: u32,
    pub groups: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxIovec {
    pub base: *mut u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxMsghdr {
    pub name: *mut u8,
    pub namelen: u32,
    pub iov: *mut LinuxIovec,
    pub iovlen: usize,
    pub control: *mut u8,
    pub controllen: usize,
    pub flags: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxMmsghdr {
    pub msg_hdr: LinuxMsghdr,
    pub msg_len: u32,
}

static SOCKET_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref SOCKETS: Mutex<BTreeMap<usize, SocketRef>> = Mutex::new(BTreeMap::new());
}

static SOCKET_FILE_OPS: FileOps = FileOps {
    name: "socket",
    read: Some(socket_file_read),
    write: Some(socket_file_write),
    llseek: None,
    fsync: None,
    poll: Some(socket_file_poll),
    ioctl: Some(socket_file_ioctl),
    mmap: None,
    release: Some(socket_release),
    readdir: None,
};

fn socket_release(file: FileRef) {
    let token = *file.private.lock();
    if token != 0 {
        if let Some(sock) = SOCKETS.lock().remove(&token) {
            socket::release_socket(&sock);
        }
    }
}

fn socket_from_file(file: &FileRef) -> Result<SocketRef, i32> {
    if file.fops.name != SOCKET_FILE_OPS.name {
        return Err(EBADF);
    }
    let token = *file.private.lock();
    SOCKETS.lock().get(&token).cloned().ok_or(EBADF)
}

fn socket_file_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    let sock = socket_from_file(file)?;
    let nonblocking = recvmsg_is_nonblocking(file, 0);
    let recv_timeout_ns = sock.lock().recv_timeout_ns;
    loop {
        match socket::recvmsg(&sock, buf) {
            Ok(n) => return Ok(n),
            Err(EAGAIN) if !nonblocking => {
                if let Err(errno) = wait_for_socket_recv(&sock, recv_timeout_ns) {
                    return Err(errno);
                }
            }
            Err(errno) => return Err(errno),
        }
    }
}

fn socket_file_write(file: &FileRef, buf: &[u8], _pos: &mut u64) -> Result<usize, i32> {
    let sock = socket_from_file(file)?;
    socket::sendmsg(&sock, buf)
}

fn socket_file_poll(file: &FileRef, mut table: Option<&mut crate::fs::select::PollTable>) -> u32 {
    let Ok(sock) = socket_from_file(file) else {
        return crate::fs::select::POLLERR as u32;
    };
    let mask = {
        let socket = sock.lock();
        // Linux sock_poll_wait(): install the caller before sampling state.
        // Queue registration and the readiness check therefore share the
        // socket lock with every producer and cannot lose an intervening wake.
        crate::fs::select::poll_wait(file, &socket.recv_wait, table.as_deref_mut());
        let mut mask = 0u32;
        let hung_up = socket::stream_hangup_locked(&socket);
        let readable = if socket.state == SocketState::Listening {
            !socket.backlog.is_empty()
                || socket.shutdown & socket::RCV_SHUTDOWN != 0
                || socket.pending_error != 0
        } else {
            // A hung-up stream is readable: the pending read drains to EOF
            // (Linux tcp_poll() reports EPOLLIN once the peer closes).
            !socket.recvq.is_empty()
                || socket.shutdown & socket::RCV_SHUTDOWN != 0
                || hung_up
                || socket.pending_error != 0
        };
        if readable {
            // Linux AF_UNIX poll reports normal readable data as both EPOLLIN and
            // EPOLLRDNORM (vendor/linux/net/unix/af_unix.c::unix_poll).
            mask |= (crate::fs::select::POLLIN | crate::fs::select::POLLRDNORM) as u32;
        }
        let datagram_writable = matches!(socket.sock_type, socket::SOCK_DGRAM | socket::SOCK_RAW)
            && socket.state != SocketState::Closed
            && socket.state != SocketState::Listening;
        if socket.state == SocketState::Connected || datagram_writable {
            // unix_poll() likewise reports writable streams with EPOLLOUT and
            // EPOLLWRNORM.
            mask |= (crate::fs::select::POLLOUT | crate::fs::select::POLLWRNORM) as u32;
        }
        if socket.shutdown & socket::RCV_SHUTDOWN != 0 || hung_up {
            mask |= 0x2000; // EPOLLRDHUP
            mask |= (crate::fs::select::POLLIN | crate::fs::select::POLLRDNORM) as u32;
        }
        if socket.shutdown == socket::SHUTDOWN_MASK
            || socket.state == SocketState::Closed
            || hung_up
        {
            mask |= crate::fs::select::POLLHUP as u32;
        }
        if socket.pending_error != 0 {
            mask |= crate::fs::select::POLLERR as u32;
        }
        mask
    };
    mask
}

fn read_user_struct<T: Copy>(ptr: *const T) -> Result<T, i32> {
    if ptr.is_null() {
        return Err(EFAULT);
    }
    let mut out = MaybeUninit::<T>::uninit();
    let left = unsafe {
        uaccess::copy_from_user(
            out.as_mut_ptr().cast::<u8>(),
            ptr.cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if left != 0 {
        return Err(EFAULT);
    }
    Ok(unsafe { out.assume_init() })
}

fn write_user_struct<T: Copy>(ptr: *mut T, value: &T) -> Result<(), i32> {
    if ptr.is_null() {
        return Err(EFAULT);
    }
    let left = unsafe {
        uaccess::copy_to_user(
            ptr.cast::<u8>(),
            (value as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if left != 0 {
        return Err(EFAULT);
    }
    Ok(())
}

fn fill_c_string(dst: &mut [u8], value: &str) {
    dst.fill(0);
    let bytes = value.as_bytes();
    let len = bytes.len().min(dst.len().saturating_sub(1));
    dst[..len].copy_from_slice(&bytes[..len]);
}

fn parse_ifname_optval(val: *const u8, len: u32) -> Result<Option<String>, i32> {
    if len == 0 {
        return Ok(None);
    }
    if val.is_null() {
        return Err(EFAULT);
    }
    let copy_len = (len as usize).min(IFNAMSIZ);
    let mut bytes = alloc::vec![0u8; copy_len];
    let not_copied = unsafe { uaccess::copy_from_user(bytes.as_mut_ptr(), val, copy_len) };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    let used = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if used == 0 {
        return Ok(None);
    }
    let name = core::str::from_utf8(&bytes[..used]).map_err(|_| EINVAL)?;
    Ok(Some(String::from(name)))
}

fn socket_ioctl_dev(
    dev: &crate::net::device::NetDeviceRef,
    cmd: u32,
    ifr: &mut LinuxIfreq,
) -> Result<(), i32> {
    match cmd {
        SIOCGIFFLAGS => {
            ifr.set_flags(dev.flags.load(Ordering::Acquire) as u16);
            Ok(())
        }
        SIOCSIFFLAGS => {
            if !capable(CAP_NET_ADMIN) {
                return Err(EPERM);
            }
            let requested = ifr.flags();
            let wants_up = requested & crate::net::device::IFF_UP as u16 != 0;
            if wants_up && !dev.is_up() {
                crate::net::device::set_device_up(dev)?;
            } else if !wants_up && dev.is_up() {
                crate::net::device::set_device_down(dev)?;
            }
            Ok(())
        }
        SIOCGIFMTU => {
            ifr.set_mtu(dev.mtu as i32);
            Ok(())
        }
        SIOCGIFHWADDR => {
            let mut addr = LinuxIfSockaddr {
                sa_family: if dev.name == "lo" {
                    ARPHRD_LOOPBACK
                } else {
                    ARPHRD_ETHER
                },
                ..Default::default()
            };
            addr.sa_data[..dev.dev_addr.len()].copy_from_slice(&dev.dev_addr);
            ifr.ifru[..core::mem::size_of::<LinuxIfSockaddr>()].copy_from_slice(unsafe {
                core::slice::from_raw_parts(
                    (&addr as *const LinuxIfSockaddr).cast::<u8>(),
                    core::mem::size_of::<LinuxIfSockaddr>(),
                )
            });
            Ok(())
        }
        SIOCGIFINDEX => {
            ifr.set_ifindex(dev.ifindex as i32);
            Ok(())
        }
        _ => Err(ENOTTY),
    }
}

fn socket_ioctl_ethtool(
    dev: &crate::net::device::NetDeviceRef,
    ifr: &LinuxIfreq,
) -> Result<(), i32> {
    let data = ifr.data_ptr();
    if data.is_null() {
        return Err(EFAULT);
    }
    let cmd = read_user_struct::<u32>(data.cast::<u32>())?;
    match cmd {
        ETHTOOL_GDRVINFO => {
            let mut info =
                read_user_struct::<LinuxEthtoolDrvinfo>(data.cast::<LinuxEthtoolDrvinfo>())?;
            info.cmd = ETHTOOL_GDRVINFO;
            fill_c_string(
                &mut info.driver,
                if dev.linux_dev.is_some() {
                    "linux-netdev"
                } else {
                    dev.ops.name
                },
            );
            fill_c_string(&mut info.version, "lupos");
            fill_c_string(&mut info.bus_info, &dev.name);
            write_user_struct(data.cast::<LinuxEthtoolDrvinfo>(), &info)
        }
        ETHTOOL_GLINK => {
            let mut value =
                read_user_struct::<LinuxEthtoolValue>(data.cast::<LinuxEthtoolValue>())?;
            value.cmd = ETHTOOL_GLINK;
            value.data = u32::from(dev.carrier_ok());
            write_user_struct(data.cast::<LinuxEthtoolValue>(), &value)
        }
        ETHTOOL_GLINKSETTINGS => {
            let mut settings = read_user_struct::<LinuxEthtoolLinkSettings>(
                data.cast::<LinuxEthtoolLinkSettings>(),
            )?;
            settings.cmd = ETHTOOL_GLINKSETTINGS;
            settings.speed = if dev.carrier_ok() { 10_000 } else { 0 };
            settings.duplex = DUPLEX_FULL;
            settings.port = PORT_OTHER;
            settings.phy_address = 0;
            settings.autoneg = 0;
            settings.mdio_support = 0;
            settings.eth_tp_mdix = 0;
            settings.eth_tp_mdix_ctrl = 0;
            settings.link_mode_masks_nwords = 0;
            settings.transceiver = 0;
            settings.master_slave_cfg = 0;
            settings.master_slave_state = 0;
            settings.rate_matching = 0;
            write_user_struct(data.cast::<LinuxEthtoolLinkSettings>(), &settings)
        }
        _ => Err(EOPNOTSUPP),
    }
}

fn socket_file_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let _sock = socket_from_file(file)?;
    if cmd == SIOCGIFNAME {
        let mut ifr = read_user_struct::<LinuxIfreq>(arg as *const LinuxIfreq)?;
        let ifindex = ifr.ifindex() as u32;
        let dev = crate::net::device::list_netdevices()
            .into_iter()
            .find(|dev| dev.ifindex == ifindex)
            .ok_or(ENODEV)?;
        ifr.set_ifname(&dev.name);
        write_user_struct(arg as *mut LinuxIfreq, &ifr)?;
        return Ok(0);
    }

    let mut ifr = read_user_struct::<LinuxIfreq>(arg as *const LinuxIfreq)?;
    let name = ifr.ifname();
    let dev = crate::net::device::lookup_netdevice(&name).ok_or(ENODEV)?;
    match cmd {
        SIOCGIFFLAGS | SIOCSIFFLAGS | SIOCGIFMTU | SIOCGIFHWADDR | SIOCGIFINDEX => {
            socket_ioctl_dev(&dev, cmd, &mut ifr)?;
            write_user_struct(arg as *mut LinuxIfreq, &ifr)?;
            Ok(0)
        }
        SIOCETHTOOL => {
            socket_ioctl_ethtool(&dev, &ifr)?;
            Ok(0)
        }
        _ => Err(ENOTTY),
    }
}

fn current_files() -> Result<Arc<FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

struct SocketTypeSpec {
    kind: u16,
    file_flags: u32,
    cloexec: bool,
}

fn parse_socket_type(sock_type: i32) -> Result<SocketTypeSpec, i32> {
    let bits = sock_type as u32;
    let flags = bits & !socket::SOCK_TYPE_MASK;
    if flags & !(socket::SOCK_CLOEXEC | socket::SOCK_NONBLOCK) != 0 {
        return Err(EINVAL);
    }

    Ok(SocketTypeSpec {
        kind: (bits & socket::SOCK_TYPE_MASK) as u16,
        file_flags: O_RDWR | (flags & O_NONBLOCK),
        cloexec: flags & socket::SOCK_CLOEXEC != 0,
    })
}

fn parse_accept_flags(flags: i32) -> Result<(u32, bool), i32> {
    let flags = flags as u32;
    if flags & !(socket::SOCK_CLOEXEC | socket::SOCK_NONBLOCK) != 0 {
        return Err(EINVAL);
    }
    Ok((
        O_RDWR | (flags & O_NONBLOCK),
        flags & socket::SOCK_CLOEXEC != 0,
    ))
}

fn check_socket_create(family: i32, kind: u16, protocol: i32) -> Result<(), i32> {
    let security_rc = security_socket_create(family, kind as i32, protocol);
    if security_rc != 0 {
        return Err(-security_rc);
    }
    if matches!(family as u16, AF_INET | AF_INET6)
        && kind == socket::SOCK_RAW
        && !capable(CAP_NET_RAW)
    {
        return Err(EPERM);
    }
    Ok(())
}

fn install_socket_with(sock: SocketRef, file_flags: u32, cloexec: bool) -> Result<i32, i32> {
    let files = current_files()?;
    let token = SOCKET_TOKEN.fetch_add(1, Ordering::AcqRel);
    SOCKETS.lock().insert(token, sock);
    let file = alloc_anon_file("socket", &SOCKET_FILE_OPS, token);
    file.flags.store(file_flags, Ordering::Release);
    match files.install(file, cloexec) {
        Ok(fd) => Ok(fd),
        Err(errno) => {
            if let Some(sock) = SOCKETS.lock().remove(&token) {
                socket::release_socket(&sock);
            }
            Err(errno)
        }
    }
}

fn socket_from_fd(fd: i32) -> Result<SocketRef, i32> {
    let file = current_files()?.get(fd)?;
    socket_from_file(&file)
}

fn socket_file_from_fd(fd: i32) -> Result<(FileRef, SocketRef), i32> {
    let file = current_files()?.get(fd)?;
    let sock = socket_from_file(&file)?;
    Ok((file, sock))
}

fn socket_file_is_nonblocking(file: &FileRef) -> bool {
    file.flags.load(Ordering::Acquire) & O_NONBLOCK != 0
}

fn drop_file_refs(fds: alloc::vec::Vec<FileRef>) {
    for file in fds {
        crate::fs::file::fput(file);
    }
}

struct FileRefGuard {
    files: alloc::vec::Vec<FileRef>,
}

impl FileRefGuard {
    fn new(files: alloc::vec::Vec<FileRef>) -> Self {
        Self { files }
    }

    fn take(&mut self) -> alloc::vec::Vec<FileRef> {
        core::mem::take(&mut self.files)
    }
}

impl core::ops::Deref for FileRefGuard {
    type Target = alloc::vec::Vec<FileRef>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl Drop for FileRefGuard {
    fn drop(&mut self) {
        drop_file_refs(core::mem::take(&mut self.files));
    }
}

fn recvmsg_is_nonblocking(file: &FileRef, flags: i32) -> bool {
    flags & MSG_DONTWAIT != 0 || socket_file_is_nonblocking(file)
}

fn socket_recv_ready(sock: &SocketRef) -> bool {
    let socket = sock.lock();
    socket::socket_recv_ready_locked(&socket)
}

fn task_by_pid_for_pidfd(pid: i32) -> *mut crate::kernel::task::TaskStruct {
    let current = unsafe { sched::get_current() };
    if !current.is_null() && unsafe { (*current).pid } == pid {
        return current;
    }
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    sched::find_pool_task_by_pid(pid)
}

fn install_scm_pidfd(cred: &socket::SocketCred) -> Result<Option<i32>, i32> {
    if cred.pid <= 0 {
        return Ok(None);
    }
    if let Some(pid_ref) = &cred.pid_ref {
        let task = task_by_pid_for_pidfd(pid_ref.pid);
        if !task.is_null() && unsafe { (*task).m26.thread_pid } == pid_ref.kpid {
            return crate::fs::pidfd::install_pidfd(task, false).map(Some);
        }
        return crate::fs::pidfd::install_pidfd_from_saved_pid(
            pid_ref.pid,
            core::ptr::null_mut(),
            pid_ref.kpid,
            false,
        )
        .map(Some);
    }

    let task = task_by_pid_for_pidfd(cred.pid);
    if task.is_null() {
        return Ok(None);
    }
    crate::fs::pidfd::install_pidfd(task, false).map(Some)
}

#[cfg(not(test))]
fn wait_for_socket_recv(sock: &SocketRef, timeout_ns: u64) -> Result<(), i32> {
    let deadline_ns = if timeout_ns == 0 {
        None
    } else {
        Some(crate::kernel::time::ktime_get().saturating_add(timeout_ns))
    };
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EAGAIN);
    }

    loop {
        let _ = crate::linux_driver_abi::poll_driver_abi_events_for_wait();
        unsafe {
            crate::kernel::signal::exit_if_fatal_signal_pending_current();
        }
        if crate::kernel::signal::current_has_unblocked_pending_signals() {
            return Err(EINTR);
        }
        if let Some(deadline_ns) = deadline_ns {
            if crate::kernel::time::ktime_get() >= deadline_ns {
                return Err(EAGAIN);
            }
        }

        // Linux's wait-event ordering is prepare, recheck, schedule, finish.
        // The prepare helper takes the socket lock, rechecks recv/backlog/EOF,
        // and registers this task before a producer can change that condition.
        // If readiness already won the race, return so the syscall retries the
        // actual recv/accept operation immediately.
        if !unsafe { socket::prepare_socket_recv_wait(sock, task) } {
            return Ok(());
        }

        // Signals and absolute receive timeouts may have become visible while
        // the task was being linked. Always unlink before returning.
        if crate::kernel::signal::current_has_unblocked_pending_signals() {
            unsafe {
                socket::finish_socket_recv_wait(sock, task);
            }
            return Err(EINTR);
        }
        if let Some(deadline_ns) = deadline_ns
            && crate::kernel::time::ktime_get() >= deadline_ns
        {
            unsafe {
                socket::finish_socket_recv_wait(sock, task);
            }
            return Err(EAGAIN);
        }
        if socket_recv_ready(sock) {
            unsafe {
                socket::finish_socket_recv_wait(sock, task);
            }
            return Ok(());
        }

        // Data/EOF producers provide the normal wakeup. Linux leaves an
        // infinite receive wait untimed; a finite SO_RCVTIMEO arms only its
        // actual remaining deadline, not a 250 Hz re-poll timer.
        let task_id = task as usize;
        let timer_armed = deadline_ns.map(|deadline| {
            let remaining = deadline.saturating_sub(crate::kernel::time::ktime_get());
            let timeout = crate::kernel::time::timeconv::nsecs_to_jiffies64(remaining).max(1);
            let wake_at = crate::kernel::time::jiffies::jiffies().saturating_add(timeout);
            crate::kernel::time::sleep_timeout::arm_wakeup(task_id, wake_at);
        });
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        if timer_armed.is_some() {
            crate::kernel::time::sleep_timeout::cancel_wakeup(task_id);
        }
        unsafe {
            socket::finish_socket_recv_wait(sock, task);
        }
    }
}

#[cfg(test)]
static TEST_SOCKET_RECV_WAIT_CALLS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn wait_for_socket_recv(_sock: &SocketRef, _timeout_ns: u64) -> Result<(), i32> {
    TEST_SOCKET_RECV_WAIT_CALLS.fetch_add(1, Ordering::AcqRel);
    Err(EAGAIN)
}

fn read_sockaddr(ptr: *const u8, len: u32) -> Result<SockAddr, i32> {
    if ptr.is_null() {
        return Err(EFAULT);
    }
    let family = unsafe { core::ptr::read_unaligned(ptr as *const u16) };
    match family {
        AF_INET if len as usize >= core::mem::size_of::<LinuxSockAddrIn>() => {
            let raw = unsafe { core::ptr::read_unaligned(ptr as *const LinuxSockAddrIn) };
            Ok(SockAddr::Inet {
                addr: u32::from_be(raw.addr),
                port: u16::from_be(raw.port),
            })
        }
        AF_INET6 if len as usize >= core::mem::size_of::<LinuxSockAddrIn6>() => {
            let raw = unsafe { core::ptr::read_unaligned(ptr as *const LinuxSockAddrIn6) };
            Ok(SockAddr::Inet6 {
                addr: raw.addr,
                port: u16::from_be(raw.port),
            })
        }
        AF_UNIX => {
            if len <= 2 {
                return Err(EINVAL);
            }
            let bytes = unsafe { core::slice::from_raw_parts(ptr.add(2), len as usize - 2) };
            let path = if bytes.first() == Some(&0) {
                core::str::from_utf8(bytes).map_err(|_| EINVAL)?
            } else {
                let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
                core::str::from_utf8(&bytes[..end]).map_err(|_| EINVAL)?
            };
            let path = match if path.as_bytes().first() == Some(&0) {
                None
            } else {
                crate::fs::proc::fd::current_fd_path_from_proc_path(path)
            } {
                Some(Ok(path)) => path,
                Some(Err(errno)) => return Err(errno),
                None => String::from(path),
            };
            Ok(SockAddr::Unix(path))
        }
        AF_NETLINK if len as usize >= core::mem::size_of::<LinuxSockAddrNetlink>() => {
            let raw = unsafe { core::ptr::read_unaligned(ptr as *const LinuxSockAddrNetlink) };
            Ok(SockAddr::Netlink {
                pid: raw.pid,
                groups: raw.groups,
            })
        }
        AF_PACKET if len as usize >= core::mem::size_of::<LinuxSockAddr>() => {
            let raw = unsafe { core::ptr::read_unaligned(ptr as *const LinuxSockAddr) };
            let protocol = u16::from_be_bytes([raw.data[0], raw.data[1]]);
            let ifindex = u32::from_ne_bytes([raw.data[2], raw.data[3], raw.data[4], raw.data[5]]);
            Ok(SockAddr::Packet { ifindex, protocol })
        }
        _ => Err(EINVAL),
    }
}

fn split_last_path(path: &str) -> (&str, &str) {
    let trimmed = path.trim_end_matches('/');
    if let Some(idx) = trimmed.rfind('/') {
        if idx == 0 {
            ("/", &trimmed[1..])
        } else {
            (&trimmed[..idx], &trimmed[idx + 1..])
        }
    } else {
        ("", trimmed)
    }
}

fn ensure_unix_socket_node(path: &str) -> Result<(), i32> {
    if path.is_empty() || !path.starts_with('/') {
        return Ok(());
    }

    let (parent_path, name) = split_last_path(path);
    if name.is_empty() {
        return Err(EINVAL);
    }
    let (_, parent) = crate::fs::mount::resolve_path_follow(parent_path)?;
    let parent_inode = parent.inode().ok_or(ENOENT)?;
    if parent_inode.kind != InodeKind::Directory {
        return Err(ENOTDIR);
    }
    let negative_dentry = if let Some(existing) = crate::fs::dcache::d_lookup(&parent, name) {
        match existing.inode() {
            Some(inode) if inode.kind == InodeKind::Socket => return Ok(()),
            Some(_) => return Err(EADDRINUSE),
            None => Some(existing),
        }
    } else {
        None
    };
    if let Some(lookup) = parent_inode.ops.lookup
        && let Ok(inode) = lookup(&parent_inode, name)
    {
        return if inode.kind == InodeKind::Socket {
            let dentry = crate::fs::dcache::d_alloc_child(&parent, name);
            dentry.instantiate(inode);
            Ok(())
        } else {
            Err(EADDRINUSE)
        };
    }

    let sb = parent_inode.sb.lock().clone();
    let ino = sb.as_ref().map(|sb| sb.alloc_ino()).unwrap_or(0);
    let inode = Inode::new(
        ino,
        InodeKind::Socket,
        0o777,
        &NOOP_INODE_OPS,
        &NOOP_FILE_OPS,
        InodePrivate::None,
    );
    *inode.sb.lock() = sb;
    match &parent_inode.private {
        InodePrivate::RamDir(children) => {
            children.lock().insert(String::from(name), inode.clone());
        }
        _ => return Err(ENOSYS),
    }
    let dentry = negative_dentry.unwrap_or_else(|| crate::fs::dcache::d_alloc_child(&parent, name));
    dentry.instantiate(inode);
    crate::fs::inotify::notify_create(&parent, name, false);
    Ok(())
}

fn copy_sockaddr_data_to_user(raw: &[u8], out: *mut u8, have: u32) -> Result<(), i32> {
    if (have as i32) < 0 {
        return Err(EINVAL);
    }
    let copy_len = (have as usize).min(raw.len());
    if copy_len == 0 {
        return Ok(());
    }
    if out.is_null() {
        return Err(EFAULT);
    }
    let not_copied = unsafe { uaccess::copy_to_user(out, raw.as_ptr(), copy_len) };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(())
}

fn copy_sockaddr_to_user(raw: &[u8], out: *mut u8, out_len: *mut u32) -> Result<(), i32> {
    if out_len.is_null() {
        return Err(EFAULT);
    }
    let have = unsafe { uaccess::get_user_u32(out_len) }.map_err(|_| EFAULT)?;
    if (have as i32) < 0 {
        return Err(EINVAL);
    }
    unsafe { uaccess::put_user_u32(out_len, raw.len() as u32) }.map_err(|_| EFAULT)?;
    copy_sockaddr_data_to_user(raw, out, have)
}

fn copy_sockaddr_to_user_with_kernel_len(
    raw: &[u8],
    out: *mut u8,
    out_len: &mut u32,
) -> Result<(), i32> {
    let have = *out_len;
    if (have as i32) < 0 {
        return Err(EINVAL);
    }
    *out_len = raw.len() as u32;
    copy_sockaddr_data_to_user(raw, out, have)
}

fn write_sockaddr_with<F>(addr: &SockAddr, mut copy: F) -> Result<(), i32>
where
    F: FnMut(&[u8]) -> Result<(), i32>,
{
    match addr {
        SockAddr::Inet { addr, port } => {
            let raw = LinuxSockAddrIn {
                family: AF_INET,
                port: port.to_be(),
                addr: u32::from_ne_bytes(addr.to_be_bytes()),
                zero: [0; 8],
            };
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &raw as *const LinuxSockAddrIn as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>(),
                )
            };
            copy(bytes)
        }
        SockAddr::Inet6 { addr, port } => {
            let raw = LinuxSockAddrIn6 {
                family: AF_INET6,
                port: port.to_be(),
                flowinfo: 0,
                addr: *addr,
                scope_id: 0,
            };
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &raw as *const LinuxSockAddrIn6 as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn6>(),
                )
            };
            copy(bytes)
        }
        // `vendor/linux/net/unix/af_unix.c::unix_getname` copies the
        // stored `sockaddr_un` and returns the exact address length; then
        // `vendor/linux/net/socket.c::move_addr_to_user` truncates the copy to
        // the caller's buffer while still reporting that exact length.
        SockAddr::Unix(path) => {
            let path_bytes = path.as_bytes();
            let nul = if path.is_empty() || path_bytes.first() == Some(&0) {
                0
            } else {
                1
            };
            let mut raw = alloc::vec::Vec::with_capacity(2 + path_bytes.len() + nul);
            raw.extend_from_slice(&AF_UNIX.to_ne_bytes());
            raw.extend_from_slice(path_bytes);
            if nul != 0 {
                raw.push(0);
            }
            copy(&raw)
        }
        // `vendor/linux/net/netlink/af_netlink.c::netlink_getname`:
        //   nl_family = AF_NETLINK; nl_pad = 0;
        //   nl_pid = nlk->portid; nl_groups = nlk->groups[0];
        //   return sizeof(*nladdr);    /* 12 bytes */
        // systemd's `sd_netlink_open_fd()` calls getsockname() after socket()
        // to learn the assigned portid; without this arm the call returns
        // EINVAL and systemd logs "Failed to open netlink".
        SockAddr::Netlink { pid, groups } => {
            let raw = LinuxSockAddrNetlink {
                family: AF_NETLINK,
                pad: 0,
                pid: *pid,
                groups: *groups,
            };
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &raw as *const LinuxSockAddrNetlink as *const u8,
                    core::mem::size_of::<LinuxSockAddrNetlink>(),
                )
            };
            copy(bytes)
        }
        _ => Err(EINVAL),
    }
}

fn write_sockaddr(addr: &SockAddr, out: *mut u8, out_len: *mut u32) -> Result<(), i32> {
    write_sockaddr_with(addr, |raw| copy_sockaddr_to_user(raw, out, out_len))
}

fn write_sockaddr_with_kernel_len(
    addr: &SockAddr,
    out: *mut u8,
    out_len: &mut u32,
) -> Result<(), i32> {
    write_sockaddr_with(addr, |raw| {
        copy_sockaddr_to_user_with_kernel_len(raw, out, out_len)
    })
}

fn copy_iov_bytes(iov: *const LinuxIovec, iovlen: usize) -> Result<alloc::vec::Vec<u8>, i32> {
    if iov.is_null() && iovlen != 0 {
        return Err(EFAULT);
    }
    let mut bytes = alloc::vec::Vec::new();
    for n in 0..iovlen {
        let ent = unsafe { *iov.add(n) };
        if ent.base.is_null() && ent.len != 0 {
            return Err(EFAULT);
        }
        if bytes.len().saturating_add(ent.len) > MAX_RW {
            return Err(EINVAL);
        }
        if ent.len == 0 {
            continue;
        }
        let slice = unsafe { core::slice::from_raw_parts(ent.base as *const u8, ent.len) };
        bytes.extend_from_slice(slice);
    }
    Ok(bytes)
}

fn scatter_iov_bytes(iov: *mut LinuxIovec, iovlen: usize, bytes: &[u8]) -> Result<usize, i32> {
    if iov.is_null() && iovlen != 0 {
        return Err(EFAULT);
    }
    let mut copied = 0usize;
    for n in 0..iovlen {
        if copied == bytes.len() {
            break;
        }
        let ent = unsafe { *iov.add(n) };
        if ent.base.is_null() && ent.len != 0 {
            return Err(EFAULT);
        }
        if ent.len == 0 {
            continue;
        }
        let take = ent.len.min(bytes.len() - copied);
        if take == 0 {
            continue;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr().add(copied), ent.base, take);
        }
        copied += take;
    }
    Ok(copied)
}

pub unsafe fn sys_socket(family: i32, sock_type: i32, protocol: i32) -> i64 {
    let spec = match parse_socket_type(sock_type) {
        Ok(spec) => spec,
        Err(errno) => return -(errno as i64),
    };
    if let Err(errno) = check_socket_create(family, spec.kind, protocol) {
        return -(errno as i64);
    }
    match socket::socket(family as u16, spec.kind, protocol as u16)
        .and_then(|sock| install_socket_with(sock, spec.file_flags, spec.cloexec))
    {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_bind(fd: i32, addr: *const u8, addrlen: u32) -> i64 {
    let sock = match socket_from_fd(fd) {
        Ok(sock) => sock,
        Err(errno) => return -(errno as i64),
    };
    let parsed = read_sockaddr(addr, addrlen);
    match parsed.as_ref() {
        Ok(sa) => {
            let result = (|| {
                socket::bind(&sock, sa.clone())?;
                if let SockAddr::Unix(path) = sa {
                    if let Err(errno) = ensure_unix_socket_node(path) {
                        socket::rollback_bound_socket_addr(&sock, sa);
                        return Err(errno);
                    }
                }
                Ok::<(), i32>(())
            })();
            result.map(|_| 0).unwrap_or_else(|errno| -(errno as i64))
        }
        Err(errno) => -(*errno as i64),
    }
}

pub unsafe fn sys_listen(fd: i32, _backlog: i32) -> i64 {
    match socket_from_fd(fd).and_then(|sock| socket::listen(&sock)) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_connect(fd: i32, addr: *const u8, addrlen: u32) -> i64 {
    let (file, sock) = match socket_file_from_fd(fd) {
        Ok(pair) => pair,
        Err(errno) => return -(errno as i64),
    };
    let parsed = read_sockaddr(addr, addrlen);
    match parsed.and_then(|sa| socket::connect(&sock, sa)) {
        Ok(()) => 0,
        Err(EINPROGRESS) if socket_file_is_nonblocking(&file) => -(EINPROGRESS as i64),
        Err(EINPROGRESS) => loop {
            let _ = crate::linux_driver_abi::poll_driver_abi_events_for_wait();
            let outcome = {
                let mut socket = sock.lock();
                if socket.state == SocketState::Connected {
                    Some(Ok(()))
                } else if socket.pending_error != 0 {
                    let error = socket.pending_error;
                    socket.pending_error = 0;
                    Some(Err(error))
                } else {
                    None
                }
            };
            if let Some(outcome) = outcome {
                break outcome.map(|_| 0).unwrap_or_else(|errno| -(errno as i64));
            }
            if crate::kernel::signal::current_has_unblocked_pending_signals() {
                break -(EINTR as i64);
            }
            unsafe { crate::kernel::sched::schedule_with_irqs_enabled() };
        },
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_accept4(fd: i32, addr: *mut u8, addrlen: *mut u32, flags: i32) -> i64 {
    let (file_flags, cloexec) = match parse_accept_flags(flags) {
        Ok(parsed) => parsed,
        Err(errno) => return -(errno as i64),
    };
    let (listener_file, listener) = match socket_file_from_fd(fd) {
        Ok(pair) => pair,
        Err(errno) => return -(errno as i64),
    };
    let listener_nonblocking = socket_file_is_nonblocking(&listener_file);
    let listener_timeout_ns = listener.lock().recv_timeout_ns;
    let accepted = loop {
        match socket::accept4(&listener) {
            Ok(sock) => break sock,
            Err(EAGAIN) if !listener_nonblocking => {
                if let Err(errno) = wait_for_socket_recv(&listener, listener_timeout_ns) {
                    return -(errno as i64);
                }
            }
            Err(errno) => {
                return -(errno as i64);
            }
        }
    };
    if !addr.is_null() || !addrlen.is_null() {
        let peer = accepted.lock().peer.clone();
        if let Some(peer) = peer {
            if let Err(errno) = write_sockaddr(&peer, addr, addrlen) {
                socket::release_socket(&accepted);
                return -(errno as i64);
            }
        }
    }
    match install_socket_with(accepted, file_flags, cloexec) {
        Ok(newfd) => newfd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_accept(fd: i32, addr: *mut u8, addrlen: *mut u32) -> i64 {
    unsafe { sys_accept4(fd, addr, addrlen, 0) }
}

pub unsafe fn sys_socketpair(family: i32, sock_type: i32, protocol: i32, sv: *mut i32) -> i64 {
    if sv.is_null() {
        return -(EFAULT as i64);
    }
    let spec = match parse_socket_type(sock_type) {
        Ok(spec) => spec,
        Err(errno) => return -(errno as i64),
    };
    let (left, right) = match socket::socketpair(family as u16, spec.kind, protocol as u16) {
        Ok(pair) => pair,
        Err(errno) => return -(errno as i64),
    };
    let left_fd = match install_socket_with(left, spec.file_flags, spec.cloexec) {
        Ok(fd) => fd,
        Err(errno) => return -(errno as i64),
    };
    let right_fd = match install_socket_with(right, spec.file_flags, spec.cloexec) {
        Ok(fd) => fd,
        Err(errno) => {
            let _ = current_files().and_then(|ft| ft.close(left_fd));
            return -(errno as i64);
        }
    };
    unsafe {
        *sv = left_fd;
        *sv.add(1) = right_fd;
    }
    0
}

pub unsafe fn sys_shutdown(fd: i32, how: i32) -> i64 {
    let local_bits = match how {
        0 => socket::RCV_SHUTDOWN,
        1 => socket::SEND_SHUTDOWN,
        2 => socket::SHUTDOWN_MASK,
        _ => return -(EINVAL as i64),
    };
    match socket_from_fd(fd) {
        Ok(sock) => {
            let (peer, sock_type) = {
                let mut socket = sock.lock();
                socket.shutdown |= local_bits;
                (socket.peer_socket.clone(), socket.sock_type)
            };
            socket::wake_socket_recv(&sock);
            if matches!(sock_type, socket::SOCK_STREAM | socket::SOCK_SEQPACKET)
                && let Some(peer) = peer
            {
                let mut peer_bits = 0;
                if local_bits & socket::SEND_SHUTDOWN != 0 {
                    peer_bits |= socket::RCV_SHUTDOWN;
                }
                if local_bits & socket::RCV_SHUTDOWN != 0 {
                    peer_bits |= socket::SEND_SHUTDOWN;
                }
                peer.lock().shutdown |= peer_bits;
                socket::wake_socket_recv(&peer);
            }
            0
        }
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_getsockname(fd: i32, addr: *mut u8, addrlen: *mut u32) -> i64 {
    let sock = match socket_from_fd(fd) {
        Ok(sock) => sock,
        Err(errno) => return -(errno as i64),
    };
    let (family, local) = {
        let socket = sock.lock();
        (socket.family, socket.local.clone())
    };
    let local = local.or_else(|| unbound_sockaddr(family));
    match local {
        Some(local) => match write_sockaddr(&local, addr, addrlen) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
        None => -(EINVAL as i64),
    }
}

fn unbound_sockaddr(family: u16) -> Option<SockAddr> {
    match family {
        // vendor/linux/net/unix/af_unix.c::unix_getname(): unnamed AF_UNIX
        // sockets report only sa_family, not EINVAL.
        AF_UNIX => Some(SockAddr::Unix(String::new())),
        // inet_getname()/inet6_getname() report the wildcard address and port
        // zero before bind/autobind. curl calls this after an IPv6 route
        // failure while falling back to IPv4.
        AF_INET => Some(SockAddr::Inet { addr: 0, port: 0 }),
        AF_INET6 => Some(SockAddr::Inet6 {
            addr: [0; 16],
            port: 0,
        }),
        _ => None,
    }
}

pub unsafe fn sys_getpeername(fd: i32, addr: *mut u8, addrlen: *mut u32) -> i64 {
    let sock = match socket_from_fd(fd) {
        Ok(sock) => sock,
        Err(errno) => return -(errno as i64),
    };
    match sock.lock().peer.clone() {
        Some(peer) => match write_sockaddr(&peer, addr, addrlen) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
        None => -(ENOTCONN as i64),
    }
}

unsafe fn copy_setsockopt_struct_from_user<T: Copy>(val: *const u8, len: u32) -> Result<T, i32> {
    if val.is_null() {
        return Err(EFAULT);
    }
    if len < core::mem::size_of::<T>() as u32 {
        return Err(EINVAL);
    }

    let mut out = MaybeUninit::<T>::uninit();
    let not_copied = unsafe {
        uaccess::copy_from_user(out.as_mut_ptr() as *mut u8, val, core::mem::size_of::<T>())
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }

    Ok(unsafe { out.assume_init() })
}

unsafe fn read_timeval_timeout_ns(val: *const u8, len: u32) -> Result<u64, i32> {
    let tv =
        unsafe { copy_setsockopt_struct_from_user::<crate::kernel::syscalls::TimeVal>(val, len)? };
    if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= 1_000_000 {
        return Err(EINVAL);
    }
    Ok((tv.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add((tv.tv_usec as u64).saturating_mul(1_000)))
}

unsafe fn read_timespec_timeout_ns(val: *const u8, len: u32) -> Result<u64, i32> {
    let ts =
        unsafe { copy_setsockopt_struct_from_user::<crate::kernel::time::Timespec64>(val, len)? };
    if !ts.is_valid() {
        return Err(EINVAL);
    }
    Ok(ts.to_ns())
}

fn set_socket_timeout(sock: &SocketRef, opt: u32, timeout_ns: u64) -> Result<(), i32> {
    let mut socket = sock.lock();
    match opt {
        socket::SO_RCVTIMEO_OLD | socket::SO_RCVTIMEO_NEW => {
            socket.recv_timeout_ns = timeout_ns;
            Ok(())
        }
        socket::SO_SNDTIMEO_OLD | socket::SO_SNDTIMEO_NEW => {
            socket.send_timeout_ns = timeout_ns;
            Ok(())
        }
        _ => Err(EINVAL),
    }
}

pub unsafe fn sys_setsockopt(fd: i32, level: i32, opt: i32, val: *const u8, len: u32) -> i64 {
    if val.is_null() && len != 0 {
        return -(EFAULT as i64);
    }
    if level == SOL_PACKET {
        return match socket_from_fd(fd) {
            Ok(sock) if sock.lock().family == AF_PACKET => 0,
            Ok(_) => -(EINVAL as i64),
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && opt as u32 == socket::IP_RECVTTL {
        if val.is_null() || len < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        let value = match unsafe { uaccess::get_user_u32(val as *const u32) } {
            Ok(value) => value,
            Err(_) => return -(EFAULT as i64),
        };
        return match socket_from_fd(fd).and_then(|sock| socket::set_recv_ttl(&sock, value)) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && opt == IP_PKTINFO {
        if val.is_null() || len < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        let value = match unsafe { uaccess::get_user_u32(val as *const u32) } {
            Ok(value) => value,
            Err(_) => return -(EFAULT as i64),
        };
        return match socket_from_fd(fd).and_then(|sock| socket::set_recv_pktinfo(&sock, value)) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && opt == IP_UNICAST_IF {
        if val.is_null() || len < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        let value = match unsafe { uaccess::get_user_u32(val as *const u32) } {
            Ok(value) => value,
            Err(_) => return -(EFAULT as i64),
        };
        let ifindex = decode_unicast_if_sockopt(value);
        return match socket_from_fd(fd).and_then(|sock| socket::set_unicast_if(&sock, ifindex)) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && matches!(opt, IP_RECVERR | IP_MTU_DISCOVER | IP_RECVFRAGSIZE | IP_TTL) {
        return match socket_from_fd(fd) {
            Ok(sock) if sock.lock().family == AF_INET => 0,
            Ok(_) => -(EINVAL as i64),
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IPV6 && matches!(opt, IPV6_RECVERR | IPV6_RECVPKTINFO | IPV6_UNICAST_IF) {
        if opt == IPV6_RECVPKTINFO {
            if val.is_null() || len < core::mem::size_of::<u32>() as u32 {
                return -(EINVAL as i64);
            }
            let value = match unsafe { uaccess::get_user_u32(val as *const u32) } {
                Ok(value) => value,
                Err(_) => return -(EFAULT as i64),
            };
            return match socket_from_fd(fd).and_then(|sock| socket::set_recv_pktinfo(&sock, value))
            {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            };
        }
        if opt == IPV6_UNICAST_IF {
            if val.is_null() || len < core::mem::size_of::<u32>() as u32 {
                return -(EINVAL as i64);
            }
            let value = match unsafe { uaccess::get_user_u32(val as *const u32) } {
                Ok(value) => value,
                Err(_) => return -(EFAULT as i64),
            };
            let ifindex = decode_unicast_if_sockopt(value);
            return match socket_from_fd(fd).and_then(|sock| socket::set_unicast_if(&sock, ifindex))
            {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            };
        }
        return match socket_from_fd(fd) {
            Ok(sock) if sock.lock().family == AF_INET6 => 0,
            Ok(_) => -(EINVAL as i64),
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_NETLINK && opt == NETLINK_PKTINFO {
        if val.is_null() || len < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        let value = match unsafe { uaccess::get_user_u32(val as *const u32) } {
            Ok(value) => value,
            Err(_) => return -(EFAULT as i64),
        };
        #[cfg(not(test))]
        if crate::kernel::debug_trace::netlink_enabled() {
            let current = unsafe { sched::get_current() };
            let pid = if current.is_null() {
                0
            } else {
                unsafe { (*current).pid }
            };
            crate::linux_driver_abi::tty::serial_println!(
                "trace-netlink-pktinfo-setsockopt pid={} fd={} value={}",
                pid,
                fd,
                value
            );
        }
        return match socket_from_fd(fd).and_then(|sock| socket::set_recv_pktinfo(&sock, value)) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_NETLINK && matches!(opt, NETLINK_ADD_MEMBERSHIP | NETLINK_DROP_MEMBERSHIP) {
        if val.is_null() {
            return -(EFAULT as i64);
        }
        if len < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        let group = unsafe { core::ptr::read_unaligned(val as *const u32) };
        return match socket_from_fd(fd).and_then(|sock| {
            socket::set_netlink_membership(&sock, group, opt == NETLINK_ADD_MEMBERSHIP)
        }) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_SOCKET {
        if opt == SO_BINDTODEVICE {
            let ifindex = match parse_ifname_optval(val, len) {
                Ok(Some(name)) => match crate::net::device::lookup_netdevice(&name) {
                    Some(dev) => dev.ifindex,
                    None => return -(ENODEV as i64),
                },
                Ok(None) => 0,
                Err(errno) => return -(errno as i64),
            };
            return match socket_from_fd(fd)
                .and_then(|sock| socket::set_bound_ifindex(&sock, ifindex))
            {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            };
        }
        let opt_u32 = opt as u32;
        if matches!(
            opt_u32,
            socket::SO_RCVTIMEO_OLD
                | socket::SO_SNDTIMEO_OLD
                | socket::SO_RCVTIMEO_NEW
                | socket::SO_SNDTIMEO_NEW
        ) {
            let timeout_ns = if matches!(opt_u32, socket::SO_RCVTIMEO_NEW | socket::SO_SNDTIMEO_NEW)
            {
                match unsafe { read_timespec_timeout_ns(val, len) } {
                    Ok(ns) => ns,
                    Err(errno) => return -(errno as i64),
                }
            } else {
                match unsafe { read_timeval_timeout_ns(val, len) } {
                    Ok(ns) => ns,
                    Err(errno) => return -(errno as i64),
                }
            };
            return match socket_from_fd(fd)
                .and_then(|sock| set_socket_timeout(&sock, opt_u32, timeout_ns))
            {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            };
        }
    }
    let value = if !val.is_null() && len >= core::mem::size_of::<u32>() as u32 {
        unsafe { core::ptr::read_unaligned(val as *const u32) }
    } else {
        0
    };
    let ret = match socket_from_fd(fd).and_then(|sock| socket::setsockopt(&sock, opt as u32, value))
    {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    };
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled()
        && level == SOL_SOCKET
        && opt as u32 == socket::SO_PASSCRED
    {
        let cred = trace_current_ucred();
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-setsockopt-passcred fd={} value={} ret={} pid={} uid={} gid={}",
            fd,
            value,
            ret,
            cred.pid,
            cred.uid,
            cred.gid
        );
    }
    ret
}

pub unsafe fn sys_getsockopt(fd: i32, level: i32, opt: i32, val: *mut u8, len: *mut u32) -> i64 {
    if len.is_null() {
        return -(EFAULT as i64);
    }
    if level == SOL_PACKET {
        return match socket_from_fd(fd) {
            Ok(sock) if sock.lock().family == AF_PACKET => {
                if val.is_null() {
                    return -(EFAULT as i64);
                }
                let have = match unsafe { uaccess::get_user_u32(len) } {
                    Ok(have) => have,
                    Err(_) => return -(EFAULT as i64),
                };
                if have < core::mem::size_of::<u32>() as u32 {
                    return -(EINVAL as i64);
                }
                if unsafe { uaccess::put_user_u32(val as *mut u32, 0) }.is_err() {
                    return -(EFAULT as i64);
                }
                if unsafe { uaccess::put_user_u32(len, core::mem::size_of::<u32>() as u32) }
                    .is_err()
                {
                    return -(EFAULT as i64);
                }
                0
            }
            Ok(_) => -(EINVAL as i64),
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && opt as u32 == socket::IP_RECVTTL {
        if val.is_null() {
            return -(EFAULT as i64);
        }
        let have = match unsafe { uaccess::get_user_u32(len) } {
            Ok(have) => have,
            Err(_) => return -(EFAULT as i64),
        };
        if have < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        return match socket_from_fd(fd).and_then(|sock| socket::get_recv_ttl(&sock)) {
            Ok(value) => {
                if unsafe { uaccess::put_user_u32(val as *mut u32, value) }.is_err() {
                    return -(EFAULT as i64);
                }
                if unsafe { uaccess::put_user_u32(len, core::mem::size_of::<u32>() as u32) }
                    .is_err()
                {
                    return -(EFAULT as i64);
                }
                0
            }
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && opt == IP_PKTINFO {
        if val.is_null() {
            return -(EFAULT as i64);
        }
        let have = match unsafe { uaccess::get_user_u32(len) } {
            Ok(have) => have,
            Err(_) => return -(EFAULT as i64),
        };
        if have < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        return match socket_from_fd(fd).and_then(|sock| socket::get_recv_pktinfo(&sock)) {
            Ok(value) => {
                if unsafe { uaccess::put_user_u32(val as *mut u32, value) }.is_err() {
                    return -(EFAULT as i64);
                }
                if unsafe { uaccess::put_user_u32(len, core::mem::size_of::<u32>() as u32) }
                    .is_err()
                {
                    return -(EFAULT as i64);
                }
                0
            }
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && opt == IP_MTU {
        if val.is_null() {
            return -(EFAULT as i64);
        }
        let have = match unsafe { uaccess::get_user_u32(len) } {
            Ok(have) => have,
            Err(_) => return -(EFAULT as i64),
        };
        if have < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        return match socket_from_fd(fd).and_then(|sock| socket::get_inet_mtu(&sock)) {
            Ok(value) => {
                if unsafe { uaccess::put_user_u32(val as *mut u32, value) }.is_err() {
                    return -(EFAULT as i64);
                }
                if unsafe { uaccess::put_user_u32(len, core::mem::size_of::<u32>() as u32) }
                    .is_err()
                {
                    return -(EFAULT as i64);
                }
                0
            }
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IP && opt == IP_UNICAST_IF {
        if val.is_null() {
            return -(EFAULT as i64);
        }
        let have = match unsafe { uaccess::get_user_u32(len) } {
            Ok(have) => have,
            Err(_) => return -(EFAULT as i64),
        };
        if have < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        return match socket_from_fd(fd).and_then(|sock| socket::get_unicast_if(&sock)) {
            Ok(ifindex) => {
                let value = encode_unicast_if_sockopt(ifindex);
                if unsafe { uaccess::put_user_u32(val as *mut u32, value) }.is_err() {
                    return -(EFAULT as i64);
                }
                if unsafe { uaccess::put_user_u32(len, core::mem::size_of::<u32>() as u32) }
                    .is_err()
                {
                    return -(EFAULT as i64);
                }
                0
            }
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IPV6 && opt == IPV6_MTU {
        if val.is_null() {
            return -(EFAULT as i64);
        }
        let have = match unsafe { uaccess::get_user_u32(len) } {
            Ok(have) => have,
            Err(_) => return -(EFAULT as i64),
        };
        if have < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        return match socket_from_fd(fd).and_then(|sock| socket::get_inet_mtu(&sock)) {
            Ok(value) => {
                if unsafe { uaccess::put_user_u32(val as *mut u32, value) }.is_err() {
                    return -(EFAULT as i64);
                }
                if unsafe { uaccess::put_user_u32(len, core::mem::size_of::<u32>() as u32) }
                    .is_err()
                {
                    return -(EFAULT as i64);
                }
                0
            }
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_IPV6 && matches!(opt, IPV6_RECVPKTINFO | IPV6_UNICAST_IF) {
        if val.is_null() {
            return -(EFAULT as i64);
        }
        let have = match unsafe { uaccess::get_user_u32(len) } {
            Ok(have) => have,
            Err(_) => return -(EFAULT as i64),
        };
        if have < core::mem::size_of::<u32>() as u32 {
            return -(EINVAL as i64);
        }
        let result = if opt == IPV6_RECVPKTINFO {
            socket_from_fd(fd).and_then(|sock| socket::get_recv_pktinfo(&sock))
        } else {
            socket_from_fd(fd).and_then(|sock| socket::get_unicast_if(&sock))
        };
        return match result {
            Ok(ifindex) => {
                let value = if opt == IPV6_UNICAST_IF {
                    encode_unicast_if_sockopt(ifindex)
                } else {
                    ifindex
                };
                if unsafe { uaccess::put_user_u32(val as *mut u32, value) }.is_err() {
                    return -(EFAULT as i64);
                }
                if unsafe { uaccess::put_user_u32(len, core::mem::size_of::<u32>() as u32) }
                    .is_err()
                {
                    return -(EFAULT as i64);
                }
                0
            }
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_NETLINK {
        return match socket_from_fd(fd)
            .and_then(|sock| unsafe { copy_netlink_getsockopt(&sock, opt, val, len) })
        {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_SOCKET && opt as u32 == socket::SO_PEERSEC {
        return match socket_from_fd(fd) {
            // Lupos has no peer security label model yet. Linux reaches
            // security_socket_getpeersec_stream() here and returns
            // -ENOPROTOOPT when the active LSM has no stream peer label.
            Ok(_) => -(ENOPROTOOPT as i64),
            Err(errno) => -(errno as i64),
        };
    }
    if level == SOL_SOCKET && opt as u32 == socket::SO_PEERGROUPS {
        return match socket_from_fd(fd).and_then(|sock| copy_unix_peergroups(&sock, val, len)) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    if val.is_null() {
        return -(EFAULT as i64);
    }
    if opt as u32 == socket::SO_PEERCRED {
        let ret = match socket_from_fd(fd).and_then(|sock| copy_unix_peercred(&sock, val, len)) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
        #[cfg(not(test))]
        if crate::kernel::debug_trace::proc_enabled() && ret == 0 {
            let cred = unsafe { core::ptr::read_unaligned(val as *const LinuxUcred) };
            crate::linux_driver_abi::tty::serial_println!(
                "trace-proc-so-peercred fd={} pid={} uid={} gid={}",
                fd,
                cred.pid,
                cred.uid,
                cred.gid
            );
        }
        return ret;
    }
    if opt as u32 == socket::SO_PEERPIDFD {
        return match socket_from_fd(fd).and_then(|sock| copy_unix_peerpidfd(&sock, val, len)) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        };
    }
    let have = unsafe { core::ptr::read_unaligned(len) };
    if have < core::mem::size_of::<u32>() as u32 {
        return -(EINVAL as i64);
    }
    match socket_from_fd(fd).and_then(|sock| socket::getsockopt(&sock, opt as u32)) {
        Ok(value) => {
            unsafe {
                core::ptr::write_unaligned(val as *mut u32, value);
                core::ptr::write_unaligned(len, core::mem::size_of::<u32>() as u32);
            }
            0
        }
        Err(errno) => -(errno as i64),
    }
}

unsafe fn copy_netlink_getsockopt(
    sock: &SocketRef,
    opt: i32,
    val: *mut u8,
    len: *mut u32,
) -> Result<(), i32> {
    if sock.lock().family != AF_NETLINK {
        return Err(ENOPROTOOPT);
    }
    let have = unsafe { core::ptr::read_unaligned(len) };
    if have > i32::MAX as u32 {
        return Err(EINVAL);
    }
    match opt {
        NETLINK_PKTINFO => {
            if have < core::mem::size_of::<u32>() as u32 {
                return Err(EINVAL);
            }
            let value = socket::get_recv_pktinfo(sock)?;
            unsafe {
                core::ptr::write_unaligned(val as *mut u32, value);
                core::ptr::write_unaligned(len, core::mem::size_of::<u32>() as u32);
            }
            Ok(())
        }
        NETLINK_LIST_MEMBERSHIPS => {
            let groups = netlink_membership_groups(sock);
            let need = if groups == 0 {
                0
            } else {
                netlink_membership_len(sock)
            };
            if have != 0 && val.is_null() {
                return Err(EFAULT);
            }
            let copy_len = (have as usize).min(need);
            let mut written = 0usize;
            if copy_len >= core::mem::size_of::<u32>() {
                unsafe {
                    core::ptr::write_unaligned(val as *mut u32, groups);
                }
                written = core::mem::size_of::<u32>();
            }
            while written < copy_len {
                unsafe {
                    core::ptr::write(val.add(written), 0);
                }
                written += 1;
            }
            unsafe {
                core::ptr::write_unaligned(len, need as u32);
            }
            Ok(())
        }
        3 | 4 | 5 | 8 | 10 | 11 | 12 => {
            if have < core::mem::size_of::<u32>() as u32 {
                unsafe {
                    core::ptr::write_unaligned(len, core::mem::size_of::<u32>() as u32);
                }
                return Err(EINVAL);
            }
            if val.is_null() {
                return Err(EFAULT);
            }
            unsafe {
                core::ptr::write_unaligned(val as *mut u32, 0);
                core::ptr::write_unaligned(len, core::mem::size_of::<u32>() as u32);
            }
            Ok(())
        }
        _ => Err(ENOPROTOOPT),
    }
}

fn netlink_membership_groups(sock: &SocketRef) -> u32 {
    match sock.lock().local.as_ref() {
        Some(SockAddr::Netlink { groups, .. }) => *groups,
        _ => 0,
    }
}

fn netlink_membership_len(sock: &SocketRef) -> usize {
    let protocol = sock.lock().protocol;
    let groups = if protocol == crate::net::rtnetlink::NETLINK_ROUTE {
        RTNLGRP_MAX
    } else {
        32
    };
    ((groups + 7) / 8 + 3) & !3
}

fn current_socket_cred_fallback() -> socket::SocketCred {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return socket::SocketCred {
            pid: 0,
            uid: 0,
            gid: 0,
            groups: Default::default(),
            pid_ref: None,
        };
    }
    let cred = unsafe { (*task).cred };
    if cred.is_null() {
        socket::SocketCred {
            pid: unsafe { (*task).pid },
            uid: 0,
            gid: 0,
            groups: Default::default(),
            pid_ref: None,
        }
    } else {
        socket::SocketCred {
            pid: unsafe { (*task).pid },
            uid: unsafe { (*cred).euid.0 },
            gid: unsafe { (*cred).egid.0 },
            groups: unsafe { (*cred).group_info },
            pid_ref: None,
        }
    }
}

fn unix_peer_cred(sock: &SocketRef) -> socket::SocketCred {
    sock.lock()
        .peer_cred
        .clone()
        .unwrap_or_else(current_socket_cred_fallback)
}

fn copy_unix_peercred(sock: &SocketRef, val: *mut u8, len: *mut u32) -> Result<(), i32> {
    if sock.lock().family != AF_UNIX {
        return Err(EINVAL);
    }
    let need = core::mem::size_of::<LinuxUcred>() as u32;
    let have = unsafe { core::ptr::read_unaligned(len) };
    unsafe {
        core::ptr::write_unaligned(len, need);
    }
    if have < need {
        return Err(EINVAL);
    }

    let peer = unix_peer_cred(sock);
    let cred = LinuxUcred {
        pid: peer.pid,
        uid: peer.uid,
        gid: peer.gid,
    };
    unsafe {
        core::ptr::write_unaligned(val as *mut LinuxUcred, cred);
    }
    Ok(())
}

fn copy_unix_peergroups(sock: &SocketRef, val: *mut u8, len: *mut u32) -> Result<(), i32> {
    if sock.lock().family != AF_UNIX {
        return Err(EINVAL);
    }
    let peer = unix_peer_cred(sock);
    let group_count = (peer.groups.ngroups as usize).min(crate::kernel::cred::NGROUPS_MAX_INLINE);
    let need = (group_count * core::mem::size_of::<u32>()) as u32;
    let have = unsafe { core::ptr::read_unaligned(len) };
    unsafe {
        core::ptr::write_unaligned(len, need);
    }
    if have < need {
        return Err(ERANGE);
    }
    if need != 0 && val.is_null() {
        return Err(EFAULT);
    }
    for idx in 0..group_count {
        unsafe {
            core::ptr::write_unaligned((val as *mut u32).add(idx), peer.groups.gid[idx].0);
        }
    }
    Ok(())
}

fn copy_unix_peerpidfd(sock: &SocketRef, val: *mut u8, len: *mut u32) -> Result<(), i32> {
    if sock.lock().family != AF_UNIX {
        return Err(EINVAL);
    }
    let need = core::mem::size_of::<i32>() as u32;
    let have = unsafe { core::ptr::read_unaligned(len) };
    unsafe {
        core::ptr::write_unaligned(len, need);
    }
    if have < need {
        return Err(EINVAL);
    }

    let peer = sock.lock().peer_cred.clone().ok_or(ENOTCONN)?;
    let pidfd = install_scm_pidfd(&peer)?.ok_or(ENOTCONN)?;
    unsafe {
        core::ptr::write_unaligned(val as *mut i32, pidfd);
    }
    Ok(())
}

pub unsafe fn sys_sendto(
    fd: i32,
    buf: *const u8,
    len: usize,
    _flags: i32,
    dest: *const u8,
    dest_len: u32,
) -> i64 {
    if buf.is_null() && len != 0 {
        return -(EFAULT as i64);
    }
    let sock = match socket_from_fd(fd) {
        Ok(sock) => sock,
        Err(errno) => return -(errno as i64),
    };
    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(buf, len) }
    };
    let result = if !dest.is_null() {
        let parsed = read_sockaddr(dest, dest_len);
        parsed.and_then(|peer| socket::sendto(&sock, bytes, peer))
    } else {
        socket::sendmsg(&sock, bytes)
    };
    match result {
        Ok(n) => n as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_recvfrom(
    fd: i32,
    buf: *mut u8,
    len: usize,
    flags: i32,
    src: *mut u8,
    src_len: *mut u32,
) -> i64 {
    if buf.is_null() && len != 0 {
        return -(EFAULT as i64);
    }
    let (file, sock) = match socket_file_from_fd(fd) {
        Ok(pair) => pair,
        Err(errno) => return -(errno as i64),
    };
    let nonblocking = recvmsg_is_nonblocking(&file, flags);
    let recv_timeout_ns = sock.lock().recv_timeout_ns;
    let mut empty = [];
    let out = if len == 0 {
        &mut empty[..]
    } else {
        unsafe { core::slice::from_raw_parts_mut(buf, len) }
    };
    // Honour MSG_PEEK / MSG_TRUNC for parity with sock_recvmsg.
    // Ref: vendor/linux/net/socket.c::__sys_recvfrom.
    loop {
        match socket::recvmsg_full(&sock, out, flags) {
            Ok((n, peer, files, _, real_len, _)) => {
                drop_file_refs(files);
                if (!src.is_null() || !src_len.is_null())
                    && let Some(peer) = peer
                {
                    if let Err(errno) = write_sockaddr(&peer, src, src_len) {
                        return -(errno as i64);
                    }
                }
                let ret = if flags & MSG_TRUNC != 0 {
                    real_len as i64
                } else {
                    n as i64
                };
                return ret;
            }
            Err(EAGAIN) if !nonblocking => {
                if let Err(errno) = wait_for_socket_recv(&sock, recv_timeout_ns) {
                    return -(errno as i64);
                }
            }
            Err(errno) => return -(errno as i64),
        }
    }
}

pub unsafe fn sys_sendmsg(fd: i32, msg: *const LinuxMsghdr, flags: i32) -> i64 {
    if msg.is_null() {
        return -(EFAULT as i64);
    }
    let msg = unsafe { *msg };
    let bytes = match copy_iov_bytes(msg.iov, msg.iovlen) {
        Ok(bytes) => bytes,
        Err(errno) => return -(errno as i64),
    };

    // Harvest SCM_RIGHTS fds from the sender's control buffer.
    // Linux semantics: each int in the SCM_RIGHTS payload is a fd in
    // the sender's table; we clone the underlying `FileRef`s now so
    // they survive the sender closing the original fds before the
    // receiver dequeues the packet.
    let send_control = match unsafe { parse_sendmsg_control(msg.control, msg.controllen) } {
        Ok(v) => v,
        Err(errno) => return -(errno as i64),
    };
    let scm_fd_count = send_control.scm_fds.len();
    let mut files = alloc::vec::Vec::with_capacity(scm_fd_count);
    if !send_control.scm_fds.is_empty() {
        let ft = match current_files() {
            Ok(ft) => ft,
            Err(errno) => {
                drop_file_refs(files);
                return -(errno as i64);
            }
        };
        for fd in &send_control.scm_fds {
            match ft.get(*fd) {
                Ok(file) => files.push(crate::fs::file::fget(&file)),
                Err(errno) => {
                    drop_file_refs(files);
                    return -(errno as i64);
                }
            }
        }
    }
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() && !send_control.scm_fds.is_empty() {
        let cred = trace_current_ucred();
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-sendmsg-scm-rights fd={} nfds={} flags={:#x} pid={} uid={} gid={}",
            fd,
            scm_fd_count,
            flags,
            cred.pid,
            cred.uid,
            cred.gid
        );
    }
    let sock = match socket_from_fd(fd) {
        Ok(sock) => sock,
        Err(errno) => {
            drop_file_refs(files);
            return -(errno as i64);
        }
    };

    // If the caller specified an explicit destination address, use the
    // sendto path (which does external INET synthesis); otherwise fall
    // through to the connected/peer-socket sendmsg path.  fd-passing
    // only makes sense on AF_UNIX, but Linux silently ignores
    // SCM_RIGHTS on other families — match that.
    #[cfg(not(test))]
    let mut trace_dest_path: Option<String> = None;
    #[cfg(not(test))]
    let mut trace_explicit_peer: Option<SockAddr> = None;
    let result = if !msg.name.is_null() && msg.namelen != 0 {
        match read_sockaddr(msg.name as *const u8, msg.namelen) {
            Ok(peer) => {
                #[cfg(not(test))]
                if let SockAddr::Unix(path) = &peer
                    && trace_unix_interesting_path(path)
                {
                    trace_dest_path = Some(path.clone());
                }
                #[cfg(not(test))]
                {
                    trace_explicit_peer = Some(peer.clone());
                }
                socket::sendto_with_fds_meta_cred(
                    &sock,
                    &bytes,
                    peer,
                    files,
                    send_control.inet_meta,
                    send_control.scm_cred,
                )
            }
            Err(errno) => {
                drop_file_refs(files);
                Err(errno)
            }
        }
    } else {
        socket::sendmsg_with_fds_meta_cred(
            &sock,
            &bytes,
            files,
            send_control.inet_meta,
            send_control.scm_cred,
        )
    };
    #[cfg(not(test))]
    trace_unix_sendmsg(fd, &sock, trace_dest_path.as_deref(), &bytes, &result);
    #[cfg(not(test))]
    trace_notify_sendmsg(
        fd,
        trace_dest_path.as_deref(),
        &bytes,
        scm_fd_count,
        &result,
    );
    match result {
        Ok(n) => n as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_recvmsg(fd: i32, msg: *mut LinuxMsghdr, flags: i32) -> i64 {
    if msg.is_null() {
        return -(EFAULT as i64);
    }
    let mut msgval = unsafe { *msg };
    let (file, sock) = match socket_file_from_fd(fd) {
        Ok(pair) => pair,
        Err(errno) => return -(errno as i64),
    };
    let nonblocking = recvmsg_is_nonblocking(&file, flags);
    let (passcred, passpidfd, recv_ttl, recv_pktinfo, recv_timeout_ns) = {
        let socket = sock.lock();
        (
            socket.passcred,
            socket.passpidfd,
            socket.recv_ttl,
            socket.recv_pktinfo,
            socket.recv_timeout_ns,
        )
    };
    let total_iov_bytes = match iov_total_bytes(msgval.iov, msgval.iovlen) {
        Ok(len) => len,
        Err(errno) => return -(errno as i64),
    };
    let recv_tmp_len = {
        let locked = sock.lock();
        if locked.sock_type == socket::SOCK_STREAM {
            total_iov_bytes.min(MAX_RW)
        } else {
            MAX_RW.min(4096)
        }
    };

    // MSG_PEEK / MSG_TRUNC are the contract systemd's sd-netlink uses to
    // size + drain netlink datagrams.  See vendor/linux/net/socket.c::
    // sock_recvmsg.  Pass them through to the socket layer.
    let mut tmp = alloc::vec![0u8; recv_tmp_len];
    let (n, peer, files, packet_cred, real_len, had_packet) = loop {
        match socket::recvmsg_full(&sock, &mut tmp, flags) {
            Ok(t) => break t,
            Err(EAGAIN) if !nonblocking => {
                if let Err(errno) = wait_for_socket_recv(&sock, recv_timeout_ns) {
                    return -(errno as i64);
                }
            }
            Err(errno) => return -(errno as i64),
        }
    };
    let packet_meta = socket::last_rx_meta(&sock);
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() && had_packet {
        let family = sock.lock().family;
        if family == AF_NETLINK && n >= 16 {
            let current = unsafe { sched::get_current() };
            let pid = if current.is_null() {
                0
            } else {
                unsafe { (*current).pid }
            };
            let msg_type = u16::from_ne_bytes(tmp[4..6].try_into().unwrap());
            let msg_seq = u32::from_ne_bytes(tmp[8..12].try_into().unwrap());
            let msg_pid = u32::from_ne_bytes(tmp[12..16].try_into().unwrap());
            let peer_pid = match peer.as_ref() {
                Some(SockAddr::Netlink { pid, .. }) => *pid,
                _ => 0,
            };
            let peer_groups = match peer.as_ref() {
                Some(SockAddr::Netlink { groups, .. }) => *groups,
                _ => 0,
            };
            crate::linux_driver_abi::tty::serial_println!(
                "trace-netlink-recv pid={} fd={} nlmsg_type={} seq={} hdr_pid={} peer_pid={} peer_groups=0x{:x} pktinfo_group={} recv_pktinfo={} flags=0x{:x} real_len={} copied={}",
                pid,
                fd,
                msg_type,
                msg_seq,
                msg_pid,
                peer_pid,
                peer_groups,
                packet_meta.netlink_group,
                recv_pktinfo,
                flags,
                real_len,
                n,
            );
        }
    }
    let mut files = FileRefGuard::new(files);
    #[cfg(not(test))]
    trace_unix_recvmsg(
        fd,
        &sock,
        &tmp[..n],
        &packet_cred,
        passcred,
        passpidfd,
        files.len(),
        msgval.controllen,
        flags,
    );
    #[cfg(not(test))]
    trace_notify_recvmsg(
        fd,
        &sock,
        &tmp[..n],
        &packet_cred,
        passcred,
        passpidfd,
        files.len(),
        msgval.controllen,
        flags,
    );

    if !msgval.name.is_null()
        && let Some(peer) = peer.as_ref()
    {
        if let Err(errno) = write_sockaddr_with_kernel_len(&peer, msgval.name, &mut msgval.namelen)
        {
            return -(errno as i64);
        }
    }

    // Install received `FileRef`s into the caller's fdtable and
    // serialize the resulting fd numbers as an SCM_RIGHTS cmsg.  On
    // truncation, MSG_CTRUNC is OR'd into msg.flags and the cmsg is
    // dropped entirely.  If installing partway through fails (EMFILE),
    // we close the already-installed fds to avoid leaking.
    // Linux treats `msghdr.msg_flags` as output-only for recvmsg(2); userspace
    // input bits are ignored rather than reflected back into the result.
    let mut out_flags = 0;
    let mut control_written = 0usize;
    if passcred && had_packet {
        #[cfg(not(test))]
        if crate::kernel::debug_trace::proc_enabled() {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-proc-recvmsg-scm-credentials fd={} pid={} uid={} gid={} controllen={} flags={:#x}",
                fd,
                packet_cred.pid,
                packet_cred.uid,
                packet_cred.gid,
                msgval.controllen,
                flags
            );
        }
        let (written, truncated) = match unsafe {
            write_scm_credentials_at(
                msgval.control,
                msgval.controllen,
                control_written,
                &packet_cred,
            )
        } {
            Ok(result) => result,
            Err(errno) => return -(errno as i64),
        };
        control_written = written;
        if truncated {
            out_flags |= MSG_CTRUNC;
        }
    }
    if passpidfd && had_packet {
        match install_scm_pidfd(&packet_cred) {
            Ok(Some(pidfd)) => {
                let (written, truncated) = match unsafe {
                    write_scm_pidfd_at(msgval.control, msgval.controllen, control_written, pidfd)
                } {
                    Ok(result) => result,
                    Err(errno) => return -(errno as i64),
                };
                if truncated {
                    if let Ok(files) = current_files() {
                        let _ = files.close(pidfd);
                    }
                    out_flags |= MSG_CTRUNC;
                } else {
                    control_written = written;
                }
            }
            Ok(None) => {}
            Err(errno) => return -(errno as i64),
        }
    }
    if had_packet
        && recv_pktinfo
        && matches!(peer, Some(SockAddr::Netlink { .. }))
        && sock.lock().family == AF_NETLINK
    {
        let (written, truncated) = match unsafe {
            write_netlink_pktinfo_at(
                msgval.control,
                msgval.controllen,
                control_written,
                packet_meta.netlink_group,
            )
        } {
            Ok(result) => result,
            Err(errno) => return -(errno as i64),
        };
        control_written = written;
        if truncated {
            out_flags |= MSG_CTRUNC;
        }
    }
    if had_packet
        && recv_pktinfo
        && matches!(peer, Some(SockAddr::Inet { .. }))
        && sock.lock().family == AF_INET
        && packet_meta.local_inet_addr.is_some()
    {
        let (written, truncated) = match unsafe {
            write_ipv4_pktinfo_at(
                msgval.control,
                msgval.controllen,
                control_written,
                &packet_meta,
            )
        } {
            Ok(result) => result,
            Err(errno) => return -(errno as i64),
        };
        control_written = written;
        if truncated {
            out_flags |= MSG_CTRUNC;
        }
    }
    if had_packet
        && recv_ttl
        && matches!(peer, Some(SockAddr::Inet { .. }))
        && sock.lock().family == AF_INET
    {
        let (written, truncated) = match unsafe {
            write_ipv4_ttl_at(
                msgval.control,
                msgval.controllen,
                control_written,
                packet_meta.ttl.unwrap_or(64),
            )
        } {
            Ok(result) => result,
            Err(errno) => return -(errno as i64),
        };
        control_written = written;
        if truncated {
            out_flags |= MSG_CTRUNC;
        }
    }
    if !files.is_empty() {
        #[cfg(not(test))]
        if crate::kernel::debug_trace::proc_enabled() {
            let receiver = trace_current_ucred();
            crate::linux_driver_abi::tty::serial_println!(
                "trace-proc-recvmsg-scm-rights fd={} nfds={} pid={} uid={} gid={} controllen={} flags={:#x}",
                fd,
                files.len(),
                receiver.pid,
                receiver.uid,
                receiver.gid,
                msgval.controllen,
                flags
            );
        }
        let ft = match current_files() {
            Ok(ft) => ft,
            Err(errno) => return -(errno as i64),
        };
        let mut installed: alloc::vec::Vec<i32> = alloc::vec::Vec::with_capacity(files.len());
        let cloexec = flags & MSG_CMSG_CLOEXEC != 0;
        let mut incoming = files.take().into_iter();
        while let Some(file) = incoming.next() {
            // Preserve one explicit reference across install(): on an error
            // FileRef's plain Arc drop cannot run the VFS release hook, while
            // fput(cleanup) can. On success the installed fd keeps it alive.
            let cleanup = crate::fs::file::fget(&file);
            match ft.install(file, cloexec) {
                Ok(fd) => {
                    crate::fs::file::fput(cleanup);
                    installed.push(fd)
                }
                Err(_) => {
                    crate::fs::file::fput(cleanup);
                    for remaining in incoming {
                        crate::fs::file::fput(remaining);
                    }
                    for installed_fd in &installed {
                        let _ = ft.close(*installed_fd);
                    }
                    installed.clear();
                    out_flags |= MSG_CTRUNC;
                    #[cfg(not(test))]
                    if crate::kernel::debug_trace::proc_enabled() {
                        crate::linux_driver_abi::tty::serial_println!(
                            "trace-proc-recvmsg-scm-rights-ret fd={} installed=0 control_written={} out_flags={:#x} error=EMFILE",
                            fd,
                            control_written,
                            out_flags
                        );
                    }
                    break;
                }
            }
        }
        let (written, truncated) = match unsafe {
            write_scm_rights_at(
                msgval.control,
                msgval.controllen,
                control_written,
                &installed,
            )
        } {
            Ok(result) => result,
            Err(errno) => {
                for fd in &installed {
                    let _ = ft.close(*fd);
                }
                return -(errno as i64);
            }
        };
        control_written = written;
        if truncated {
            out_flags |= MSG_CTRUNC;
            for fd in &installed {
                let _ = ft.close(*fd);
            }
        }
        #[cfg(not(test))]
        if crate::kernel::debug_trace::proc_enabled() {
            let receiver = trace_current_ucred();
            crate::linux_driver_abi::tty::serial_println!(
                "trace-proc-recvmsg-scm-rights-ret fd={} installed={} pid={} uid={} gid={} control_written={} out_flags={:#x}",
                fd,
                if truncated { 0 } else { installed.len() },
                receiver.pid,
                receiver.uid,
                receiver.gid,
                control_written,
                out_flags
            );
        }
    }
    unsafe {
        (*msg).controllen = control_written;
    }

    // If the user's iov is shorter than the real packet, set MSG_TRUNC in
    // msg.flags (Linux sock_recvmsg sets this even when the user didn't
    // request it).  When MSG_TRUNC is in `flags`, the syscall reports the
    // *real* packet length so the caller can sniff the size with a tiny
    // buffer before allocating one large enough.  systemd-260.1 relies on
    // this in `src/libsystemd/sd-netlink/netlink-message.c`.
    if real_len > total_iov_bytes {
        out_flags |= MSG_TRUNC;
    }
    match scatter_iov_bytes(msgval.iov, msgval.iovlen, &tmp[..n]) {
        Ok(copied) => {
            unsafe {
                (*msg).namelen = msgval.namelen;
                (*msg).flags = out_flags;
            }
            if flags & MSG_TRUNC != 0 {
                real_len as i64
            } else {
                copied as i64
            }
        }
        Err(errno) => -(errno as i64),
    }
}

/// Sum the byte capacity advertised by a bounded user-space `struct iovec`
/// array after fault-tolerantly copying its metadata into kernel memory.
/// Needed to detect packet truncation against the receiver's buffer.
fn iov_total_bytes(iov: *mut LinuxIovec, iovlen: usize) -> Result<usize, i32> {
    if iovlen > UIO_MAXIOV {
        return Err(EINVAL);
    }
    if iovlen == 0 {
        return Ok(0);
    }
    if iov.is_null() {
        return Err(EFAULT);
    }

    let bytes = iovlen
        .checked_mul(core::mem::size_of::<LinuxIovec>())
        .ok_or(EINVAL)?;
    let mut entries = alloc::vec![LinuxIovec {
        base: core::ptr::null_mut(),
        len: 0,
    }; iovlen];
    let not_copied = unsafe {
        uaccess::copy_from_user(entries.as_mut_ptr().cast::<u8>(), iov.cast::<u8>(), bytes)
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }

    Ok(entries
        .iter()
        .fold(0usize, |total, entry| total.saturating_add(entry.len)))
}

pub unsafe fn sys_sendmmsg(fd: i32, msgvec: *mut LinuxMmsghdr, vlen: u32, flags: i32) -> i64 {
    if msgvec.is_null() && vlen != 0 {
        return -(EFAULT as i64);
    }
    let mut sent = 0i64;
    for idx in 0..vlen as usize {
        let entry = unsafe { &mut *msgvec.add(idx) };
        let ret = unsafe { sys_sendmsg(fd, &entry.msg_hdr, flags) };
        if ret < 0 {
            return if sent > 0 { sent } else { ret };
        }
        entry.msg_len = ret as u32;
        sent += 1;
    }
    sent
}

pub unsafe fn sys_recvmmsg(
    fd: i32,
    msgvec: *mut LinuxMmsghdr,
    vlen: u32,
    flags: i32,
    _timeout: *mut crate::kernel::time::Timespec64,
) -> i64 {
    if msgvec.is_null() && vlen != 0 {
        return -(EFAULT as i64);
    }
    let mut received = 0i64;
    for idx in 0..vlen as usize {
        let entry = unsafe { &mut *msgvec.add(idx) };
        let ret = unsafe { sys_recvmsg(fd, &mut entry.msg_hdr, flags) };
        if ret < 0 {
            return if received > 0 { received } else { ret };
        }
        entry.msg_len = ret as u32;
        received += 1;
    }
    received
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, format, string::String};

    use super::*;
    use crate::include::uapi::fcntl::{
        AT_FDCWD, FD_CLOEXEC, O_CLOEXEC, O_DIRECTORY, O_NONBLOCK, O_PATH,
    };
    use crate::kernel::capability::KernelCapT;
    use crate::kernel::cred::{Cred, GroupInfo, INIT_CRED, KGid, KUid, NGROUPS_MAX_INLINE};
    use crate::kernel::pid::{INIT_PID_NS, alloc_pid, put_pid};
    use crate::kernel::{files, sched, task::TaskStruct};
    use crate::net::fib::ipv4;
    use crate::net::ip::{IPPROTO_ICMP, checksum};
    use crate::security::hooks::{LSM_ID_UNDEF, LsmHooks, NOOP_HOOKS};
    use crate::security::lsm_list::{TEST_LSM_LOCK, register_lsm, reset_for_test};

    fn unix_sockaddr(path: &str) -> ([u8; 128], u32) {
        let mut raw = [0u8; 128];
        raw[..2].copy_from_slice(&AF_UNIX.to_ne_bytes());
        raw[2..2 + path.len()].copy_from_slice(path.as_bytes());
        raw[2 + path.len()] = 0;
        (raw, (3 + path.len()) as u32)
    }

    #[test]
    fn recvmsg_sockaddr_write_uses_kernel_local_namelen() {
        let peer = SockAddr::Inet {
            addr: ipv4(127, 0, 0, 1),
            port: 8080,
        };
        let mut out = [0u8; core::mem::size_of::<LinuxSockAddrIn>()];
        let mut kernel_namelen = out.len() as u32;

        assert_eq!(
            write_sockaddr_with_kernel_len(&peer, out.as_mut_ptr(), &mut kernel_namelen),
            Ok(())
        );
        assert_eq!(
            kernel_namelen,
            core::mem::size_of::<LinuxSockAddrIn>() as u32
        );
        assert_eq!(u16::from_ne_bytes([out[0], out[1]]), AF_INET);

        let bad_user_len = uaccess::TASK_SIZE_MAX as *mut u32;
        assert_eq!(
            write_sockaddr(&peer, out.as_mut_ptr(), bad_user_len),
            Err(EFAULT)
        );
    }

    #[test]
    fn getsockname_unbound_inet_reports_wildcard_address() {
        assert_eq!(
            unbound_sockaddr(AF_INET),
            Some(SockAddr::Inet { addr: 0, port: 0 })
        );
        assert_eq!(
            unbound_sockaddr(AF_INET6),
            Some(SockAddr::Inet6 {
                addr: [0; 16],
                port: 0,
            })
        );
    }

    #[test]
    fn socket_ioctl_reports_linux_netdevice_metadata() {
        crate::net::device::init();
        let name = "ioctl-net0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 9],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("register netdev");

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 276;
        current.tgid = 276;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(AF_INET as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(fd >= 0);

            let mut ifr = LinuxIfreq::default();
            ifr.set_ifname(name);
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCGIFINDEX,
                    (&mut ifr as *mut LinuxIfreq) as u64
                ),
                0
            );
            assert_eq!(ifr.ifindex(), dev.ifindex as i32);

            let mut flags_req = LinuxIfreq::default();
            flags_req.set_ifname(name);
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCGIFFLAGS,
                    (&mut flags_req as *mut LinuxIfreq) as u64
                ),
                0
            );
            assert_ne!(
                flags_req.flags() & crate::net::device::IFF_BROADCAST as u16,
                0
            );

            let mut mtu_req = LinuxIfreq::default();
            mtu_req.set_ifname(name);
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCGIFMTU,
                    (&mut mtu_req as *mut LinuxIfreq) as u64
                ),
                0
            );
            assert_eq!(
                i32::from_ne_bytes(mtu_req.ifru[..4].try_into().unwrap()),
                1500
            );

            let mut hwaddr_req = LinuxIfreq::default();
            hwaddr_req.set_ifname(name);
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCGIFHWADDR,
                    (&mut hwaddr_req as *mut LinuxIfreq) as u64
                ),
                0
            );
            let hwaddr = unsafe {
                core::ptr::read_unaligned(hwaddr_req.ifru.as_ptr().cast::<LinuxIfSockaddr>())
            };
            assert_eq!(hwaddr.sa_family, ARPHRD_ETHER);
            assert_eq!(&hwaddr.sa_data[..6], &[2, 0, 0, 0, 0, 9]);

            let mut name_req = LinuxIfreq::default();
            name_req.set_ifindex(dev.ifindex as i32);
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCGIFNAME,
                    (&mut name_req as *mut LinuxIfreq) as u64
                ),
                0
            );
            assert_eq!(name_req.ifname(), name);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(core::ptr::null_mut());
        }
        crate::net::device::unregister_netdevice(name).expect("unregister netdev");
    }

    #[test]
    fn socket_ioctl_ethtool_glink_and_drvinfo_follow_linux_shape() {
        crate::net::device::init();
        let name = "ioctl-ethtool0";
        let _ = crate::net::device::unregister_netdevice(name);
        let dev = crate::net::device::register_netdevice(
            name,
            1500,
            [2, 0, 0, 0, 0, 10],
            &crate::net::device::DUMMY_NETDEV_OPS,
        )
        .expect("register netdev");
        crate::net::device::set_carrier(&dev, true);

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 277;
        current.tgid = 277;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(AF_INET as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(fd >= 0);

            let mut drv = LinuxEthtoolDrvinfo {
                cmd: ETHTOOL_GDRVINFO,
                ..Default::default()
            };
            let mut ifr = LinuxIfreq::default();
            ifr.set_ifname(name);
            ifr.ifru[..8].copy_from_slice(
                &(((&mut drv as *mut LinuxEthtoolDrvinfo) as usize).to_ne_bytes()),
            );
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCETHTOOL,
                    (&mut ifr as *mut LinuxIfreq) as u64
                ),
                0
            );
            assert_eq!(drv.cmd, ETHTOOL_GDRVINFO);
            assert!(
                core::str::from_utf8(&drv.driver)
                    .unwrap()
                    .starts_with("dummy")
            );

            let mut glink = LinuxEthtoolValue {
                cmd: ETHTOOL_GLINK,
                ..Default::default()
            };
            ifr.ifru[..8].copy_from_slice(
                &(((&mut glink as *mut LinuxEthtoolValue) as usize).to_ne_bytes()),
            );
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCETHTOOL,
                    (&mut ifr as *mut LinuxIfreq) as u64
                ),
                0
            );
            assert_eq!(glink.data, 1);

            let mut link = LinuxEthtoolLinkSettings {
                cmd: ETHTOOL_GLINKSETTINGS,
                ..Default::default()
            };
            ifr.ifru[..8].copy_from_slice(
                &(((&mut link as *mut LinuxEthtoolLinkSettings) as usize).to_ne_bytes()),
            );
            assert_eq!(
                crate::fs::ioctl::sys_ioctl(
                    fd as i32,
                    SIOCETHTOOL,
                    (&mut ifr as *mut LinuxIfreq) as u64
                ),
                0
            );
            assert_eq!(link.cmd, ETHTOOL_GLINKSETTINGS);
            assert_eq!(link.speed, 10_000);
            assert_eq!(link.duplex, DUPLEX_FULL);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(core::ptr::null_mut());
        }
        crate::net::device::unregister_netdevice(name).expect("unregister netdev");
    }

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
            cap_bset: KernelCapT::full(),
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

    fn cred_with_groups(gids: &[u32]) -> Box<Cred> {
        let mut cred = unprivileged_cred();
        cred.group_info.ngroups = gids.len() as u32;
        for (idx, gid) in gids.iter().copied().enumerate() {
            cred.group_info.gid[idx] = KGid(gid);
        }
        cred
    }

    fn deny_socket_create(_family: i32, _kind: i32, _proto: i32) -> i32 {
        -crate::include::uapi::errno::EACCES
    }

    #[test]
    fn socket_create_invokes_lsm_hook_before_creation() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        register_lsm(LsmHooks {
            name: "deny-socket-create",
            id: LSM_ID_UNDEF,
            socket_create: Some(deny_socket_create),
            ..NOOP_HOOKS
        })
        .expect("register test lsm");

        assert_eq!(
            unsafe { sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0) },
            -(crate::include::uapi::errno::EACCES as i64)
        );

        reset_for_test();
    }

    #[test]
    fn unix_getsockopt_peergroups_returns_peer_supplementary_groups() {
        let previous = unsafe { sched::get_current() };
        let cred = cred_with_groups(&[10, 42]);
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 5521;
        current.tgid = 5521;
        current.cred = &*cred as *const Cred;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut sv = [-1i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_STREAM as i32,
                    0,
                    sv.as_mut_ptr()
                ),
                0
            );

            let mut one_gid = [0u32; 1];
            let mut one_gid_len = core::mem::size_of_val(&one_gid) as u32;
            assert_eq!(
                sys_getsockopt(
                    sv[0],
                    SOL_SOCKET,
                    socket::SO_PEERGROUPS as i32,
                    one_gid.as_mut_ptr() as *mut u8,
                    &mut one_gid_len,
                ),
                -(ERANGE as i64)
            );
            assert_eq!(one_gid_len, (2 * core::mem::size_of::<u32>()) as u32);

            let mut groups = [0u32; 2];
            let mut groups_len = core::mem::size_of_val(&groups) as u32;
            assert_eq!(
                sys_getsockopt(
                    sv[0],
                    SOL_SOCKET,
                    socket::SO_PEERGROUPS as i32,
                    groups.as_mut_ptr() as *mut u8,
                    &mut groups_len,
                ),
                0
            );
            assert_eq!(groups_len, (2 * core::mem::size_of::<u32>()) as u32);
            assert_eq!(groups, [10, 42]);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unprivileged_inet_raw_socket_requires_cap_net_raw() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        let previous = unsafe { sched::get_current() };
        let cred = unprivileged_cred();
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 5513;
        current.tgid = 5513;
        current.cred = &*cred as *const Cred;
        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(
                sys_socket(AF_INET as i32, socket::SOCK_RAW as i32, IPPROTO_ICMP as i32,),
                -(EPERM as i64)
            );
            sched::set_current(previous);
        }
        reset_for_test();
    }

    #[test]
    fn sockaddr_in_round_trip_uses_network_byte_order() {
        let raw = LinuxSockAddrIn {
            family: AF_INET,
            port: 8080u16.to_be(),
            addr: u32::from_ne_bytes(ipv4(127, 0, 0, 1).to_be_bytes()),
            zero: [0; 8],
        };
        let parsed = read_sockaddr(
            &raw as *const _ as *const u8,
            core::mem::size_of::<LinuxSockAddrIn>() as u32,
        )
        .unwrap();
        assert_eq!(
            parsed,
            SockAddr::Inet {
                addr: ipv4(127, 0, 0, 1),
                port: 8080
            }
        );
    }

    #[test]
    fn sockaddr_in_read_accepts_unaligned_user_buffer() {
        let raw = LinuxSockAddrIn {
            family: AF_INET,
            port: 53u16.to_be(),
            addr: u32::from_ne_bytes(ipv4(10, 0, 2, 3).to_be_bytes()),
            zero: [0; 8],
        };
        let mut bytes = [0u8; core::mem::size_of::<LinuxSockAddrIn>() + 1];
        unsafe {
            core::ptr::write_unaligned(bytes.as_mut_ptr().add(1) as *mut LinuxSockAddrIn, raw);
        }

        let parsed = read_sockaddr(
            unsafe { bytes.as_ptr().add(1) },
            core::mem::size_of::<LinuxSockAddrIn>() as u32,
        )
        .unwrap();
        assert_eq!(
            parsed,
            SockAddr::Inet {
                addr: ipv4(10, 0, 2, 3),
                port: 53
            }
        );
    }

    #[test]
    fn iovec_gather_scatter_round_trip() {
        let mut a = *b"ab";
        let mut b = *b"cd";
        let iov = [
            LinuxIovec {
                base: a.as_mut_ptr(),
                len: a.len(),
            },
            LinuxIovec {
                base: b.as_mut_ptr(),
                len: b.len(),
            },
        ];
        let bytes = copy_iov_bytes(iov.as_ptr(), iov.len()).unwrap();
        assert_eq!(&bytes, b"abcd");

        let mut out = [0u8; 4];
        let mut oiov = [LinuxIovec {
            base: out.as_mut_ptr(),
            len: out.len(),
        }];
        assert_eq!(scatter_iov_bytes(oiov.as_mut_ptr(), 1, b"wxyz").unwrap(), 4);
        assert_eq!(&out, b"wxyz");
    }

    #[test]
    fn iov_total_bytes_rejects_oversized_iovlen() {
        let mut iov = LinuxIovec {
            base: core::ptr::null_mut(),
            len: 0,
        };

        assert_eq!(
            iov_total_bytes(&mut iov, UIO_MAXIOV.saturating_add(1)),
            Err(EINVAL)
        );
    }

    #[test]
    fn iov_total_bytes_copies_bounded_iovec_metadata() {
        let mut a = *b"ab";
        let mut b = *b"cde";
        let mut iov = [
            LinuxIovec {
                base: a.as_mut_ptr(),
                len: a.len(),
            },
            LinuxIovec {
                base: b.as_mut_ptr(),
                len: b.len(),
            },
        ];

        assert_eq!(iov_total_bytes(iov.as_mut_ptr(), iov.len()), Ok(5));
    }

    #[test]
    fn iovec_zero_length_segments_may_have_null_base() {
        let mut a = *b"ab";
        let iov = [
            LinuxIovec {
                base: core::ptr::null_mut(),
                len: 0,
            },
            LinuxIovec {
                base: a.as_mut_ptr(),
                len: a.len(),
            },
        ];
        let bytes = copy_iov_bytes(iov.as_ptr(), iov.len()).unwrap();
        assert_eq!(&bytes, b"ab");

        let mut out = [0u8; 2];
        let mut oiov = [
            LinuxIovec {
                base: core::ptr::null_mut(),
                len: 0,
            },
            LinuxIovec {
                base: out.as_mut_ptr(),
                len: out.len(),
            },
        ];
        assert_eq!(
            scatter_iov_bytes(oiov.as_mut_ptr(), oiov.len(), b"xy").unwrap(),
            2
        );
        assert_eq!(&out, b"xy");
    }

    fn build_scm_rights_control(fd_count: usize) -> alloc::vec::Vec<u8> {
        let cmsg_len = CMSG_HDR_LEN + fd_count * core::mem::size_of::<i32>();
        let mut control = alloc::vec![0u8; cmsg_align(cmsg_len)];
        control[..core::mem::size_of::<usize>()].copy_from_slice(&cmsg_len.to_ne_bytes());
        control[8..12].copy_from_slice(&SOL_SOCKET.to_ne_bytes());
        control[12..16].copy_from_slice(&SCM_RIGHTS.to_ne_bytes());
        for i in 0..fd_count {
            let start = CMSG_HDR_LEN + i * core::mem::size_of::<i32>();
            control[start..start + core::mem::size_of::<i32>()]
                .copy_from_slice(&(i as i32).to_ne_bytes());
        }
        control
    }

    #[test]
    fn parse_scm_rights_accepts_scm_max_fd() {
        let control = build_scm_rights_control(SCM_MAX_FD);
        let parsed = unsafe { parse_sendmsg_control(control.as_ptr(), control.len()).unwrap() };
        let fds = parsed.scm_fds;
        assert_eq!(fds.len(), SCM_MAX_FD);
        assert_eq!(fds[0], 0);
        assert_eq!(fds[SCM_MAX_FD - 1], (SCM_MAX_FD - 1) as i32);
    }

    #[test]
    fn parse_scm_rights_rejects_more_than_scm_max_fd() {
        let control = build_scm_rights_control(SCM_MAX_FD + 1);
        assert!(matches!(
            unsafe { parse_sendmsg_control(control.as_ptr(), control.len()) },
            Err(EINVAL)
        ));
    }

    #[test]
    fn parse_scm_rights_rejects_aggregate_fd_count_above_cap() {
        let first = build_scm_rights_control(SCM_MAX_FD - 1);
        let second = build_scm_rights_control(2);
        let mut control = alloc::vec![0u8; first.len() + second.len()];
        control[..first.len()].copy_from_slice(&first);
        control[first.len()..].copy_from_slice(&second);

        assert!(matches!(
            unsafe { parse_sendmsg_control(control.as_ptr(), control.len()) },
            Err(EINVAL)
        ));
    }

    #[test]
    fn parse_scm_rights_rejects_excessive_control_len_before_allocating() {
        let control = build_scm_rights_control(1);
        assert!(matches!(
            unsafe { parse_sendmsg_control(control.as_ptr(), SCM_MAX_CONTROL_LEN + 1) },
            Err(EINVAL)
        ));
    }

    #[test]
    fn parse_scm_rights_rejects_wrapping_cmsg_arithmetic() {
        let mut control = alloc::vec![0u8; CMSG_HDR_LEN];
        control[..core::mem::size_of::<usize>()].copy_from_slice(&usize::MAX.to_ne_bytes());
        control[8..12].copy_from_slice(&SOL_SOCKET.to_ne_bytes());
        control[12..16].copy_from_slice(&SCM_RIGHTS.to_ne_bytes());

        assert!(matches!(
            unsafe { parse_sendmsg_control(control.as_ptr(), control.len()) },
            Err(EINVAL)
        ));
        assert_eq!(cmsg_align(usize::MAX), usize::MAX);
    }

    #[test]
    fn parse_sendmsg_control_accepts_ipv4_pktinfo_and_ttl() {
        let mut control = alloc::vec![0u8; cmsg_align(CMSG_HDR_LEN + core::mem::size_of::<LinuxInPktinfo>())
            + cmsg_align(CMSG_HDR_LEN + core::mem::size_of::<i32>())];
        let pktinfo_len = CMSG_HDR_LEN + core::mem::size_of::<LinuxInPktinfo>();
        control[..8].copy_from_slice(&pktinfo_len.to_ne_bytes());
        control[8..12].copy_from_slice(&SOL_IP.to_ne_bytes());
        control[12..16].copy_from_slice(&IP_PKTINFO.to_ne_bytes());
        let pktinfo = LinuxInPktinfo {
            ipi_ifindex: 7,
            ipi_spec_dst: u32::from_ne_bytes(ipv4(10, 0, 2, 15).to_be_bytes()),
            ipi_addr: 0,
        };
        control[CMSG_HDR_LEN..CMSG_HDR_LEN + core::mem::size_of::<LinuxInPktinfo>()]
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(
                    &pktinfo as *const LinuxInPktinfo as *const u8,
                    core::mem::size_of::<LinuxInPktinfo>(),
                )
            });

        let ttl_off = cmsg_align(pktinfo_len);
        let ttl_len = CMSG_HDR_LEN + core::mem::size_of::<i32>();
        control[ttl_off..ttl_off + 8].copy_from_slice(&ttl_len.to_ne_bytes());
        control[ttl_off + 8..ttl_off + 12].copy_from_slice(&SOL_IP.to_ne_bytes());
        control[ttl_off + 12..ttl_off + 16].copy_from_slice(&IP_TTL.to_ne_bytes());
        control[ttl_off + CMSG_HDR_LEN..ttl_off + CMSG_HDR_LEN + core::mem::size_of::<i32>()]
            .copy_from_slice(&(61i32).to_ne_bytes());

        let parsed = unsafe { parse_sendmsg_control(control.as_ptr(), control.len()) }.unwrap();
        assert!(parsed.scm_fds.is_empty());
        assert_eq!(
            parsed.inet_meta,
            Some(socket::PacketMeta {
                ifindex: 7,
                local_inet_addr: Some(ipv4(10, 0, 2, 15)),
                ttl: Some(61),
                netlink_group: 0,
            })
        );
    }

    #[test]
    fn parse_sendmsg_control_combines_scm_rights_and_pktinfo() {
        let rights = build_scm_rights_control(2);
        let pktinfo_len = CMSG_HDR_LEN + core::mem::size_of::<LinuxInPktinfo>();
        let mut control = alloc::vec![0u8; rights.len() + cmsg_align(pktinfo_len)];
        control[..rights.len()].copy_from_slice(&rights);
        let pktinfo_off = rights.len();
        control[pktinfo_off..pktinfo_off + 8].copy_from_slice(&pktinfo_len.to_ne_bytes());
        control[pktinfo_off + 8..pktinfo_off + 12].copy_from_slice(&SOL_IP.to_ne_bytes());
        control[pktinfo_off + 12..pktinfo_off + 16].copy_from_slice(&IP_PKTINFO.to_ne_bytes());
        let pktinfo = LinuxInPktinfo {
            ipi_ifindex: 3,
            ipi_spec_dst: 0,
            ipi_addr: 0,
        };
        control[pktinfo_off + CMSG_HDR_LEN
            ..pktinfo_off + CMSG_HDR_LEN + core::mem::size_of::<LinuxInPktinfo>()]
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(
                    &pktinfo as *const LinuxInPktinfo as *const u8,
                    core::mem::size_of::<LinuxInPktinfo>(),
                )
            });

        let parsed = unsafe { parse_sendmsg_control(control.as_ptr(), control.len()) }.unwrap();
        assert_eq!(parsed.scm_fds, alloc::vec![0, 1]);
        assert_eq!(
            parsed.inet_meta,
            Some(socket::PacketMeta {
                ifindex: 3,
                ..socket::PacketMeta::default()
            })
        );
    }

    #[test]
    fn recvmsg_blocking_mode_honors_msg_dontwait_and_file_flags() {
        let file = alloc_anon_file("socket-nonblock-test", &NOOP_FILE_OPS, 0);
        file.flags.store(O_RDWR, Ordering::Release);

        assert!(!recvmsg_is_nonblocking(&file, 0));
        assert!(recvmsg_is_nonblocking(&file, MSG_DONTWAIT));

        file.flags.store(O_RDWR | O_NONBLOCK, Ordering::Release);
        assert!(recvmsg_is_nonblocking(&file, 0));
    }

    #[test]
    fn recvfrom_empty_blocking_socket_waits_before_eagain() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 283;
        current.tgid = 283;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut sv = [0i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_STREAM as i32,
                    0,
                    sv.as_mut_ptr()
                ),
                0
            );

            let mut out = [0u8; 1];
            TEST_SOCKET_RECV_WAIT_CALLS.store(0, Ordering::Release);
            assert_eq!(
                sys_recvfrom(
                    sv[0],
                    out.as_mut_ptr(),
                    out.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                -(EAGAIN as i64)
            );
            assert_eq!(TEST_SOCKET_RECV_WAIT_CALLS.load(Ordering::Acquire), 1);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn accept4_empty_backlog_blocks_from_listener_file_flags() {
        let listener_file = alloc_anon_file("socket-accept-nonblock-test", &NOOP_FILE_OPS, 0);
        listener_file.flags.store(O_RDWR, Ordering::Release);
        let (accepted_file_flags, cloexec) =
            parse_accept_flags((socket::SOCK_CLOEXEC | socket::SOCK_NONBLOCK) as i32).unwrap();

        assert!(!socket_file_is_nonblocking(&listener_file));
        assert_eq!(accepted_file_flags & O_NONBLOCK, O_NONBLOCK);
        assert!(cloexec);

        listener_file
            .flags
            .store(O_RDWR | O_NONBLOCK, Ordering::Release);
        assert!(socket_file_is_nonblocking(&listener_file));
    }

    #[test]
    fn setsockopt_so_rcvtimeo_updates_socket_recv_timeout() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 282;
        current.tgid = 282;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(fd >= 0);
            let timeout = crate::kernel::syscalls::TimeVal {
                tv_sec: 0,
                tv_usec: 250_000,
            };
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_SOCKET,
                    socket::SO_RCVTIMEO_OLD as i32,
                    &timeout as *const _ as *const u8,
                    core::mem::size_of::<crate::kernel::syscalls::TimeVal>() as u32,
                ),
                0
            );
            let sock = socket_from_fd(fd as i32).unwrap();
            assert_eq!(sock.lock().recv_timeout_ns, 250_000_000);

            sched::set_current(previous);
        }
    }

    #[test]
    fn setsockopt_timeout_rejects_kernel_range_user_pointer() {
        let bad_user = crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX as *const u8;

        unsafe {
            assert_eq!(
                sys_setsockopt(
                    -1,
                    SOL_SOCKET,
                    socket::SO_RCVTIMEO_OLD as i32,
                    bad_user,
                    core::mem::size_of::<crate::kernel::syscalls::TimeVal>() as u32,
                ),
                -(EFAULT as i64)
            );
            assert_eq!(
                sys_setsockopt(
                    -1,
                    SOL_SOCKET,
                    socket::SO_SNDTIMEO_NEW as i32,
                    bad_user,
                    core::mem::size_of::<crate::kernel::time::Timespec64>() as u32,
                ),
                -(EFAULT as i64)
            );
        }
    }

    #[test]
    fn inet_icmp_recvmsg_attaches_ttl_control_message() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 279;
        current.tgid = 279;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(
                AF_INET as i32,
                socket::SOCK_DGRAM as i32,
                IPPROTO_ICMP as i32,
            );
            assert!(fd >= 0);
            let on = 1u32;
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_IP,
                    socket::IP_RECVTTL as i32,
                    &on as *const u32 as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                0
            );
            let dest = LinuxSockAddrIn {
                family: AF_INET,
                port: 0,
                addr: u32::from_ne_bytes(ipv4(93, 184, 216, 34).to_be_bytes()),
                zero: [0; 8],
            };
            let mut echo = alloc::vec![8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01, b'p', b'i', b'n', b'g'];
            let csum = checksum(&echo);
            echo[2..4].copy_from_slice(&csum.to_be_bytes());
            assert_eq!(
                sys_sendto(
                    fd as i32,
                    echo.as_ptr(),
                    echo.len(),
                    0,
                    &dest as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                echo.len() as i64
            );

            let mut out = [0u8; 64];
            let mut iov = LinuxIovec {
                base: out.as_mut_ptr(),
                len: out.len(),
            };
            let mut control = [0u8; 32];
            let mut hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut iov,
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control.len(),
                flags: 0,
            };
            assert_eq!(sys_recvmsg(fd as i32, &mut hdr, 0), echo.len() as i64);
            assert_eq!(out[0], 0);
            assert_eq!(hdr.controllen, cmsg_align(CMSG_HDR_LEN + 4));
            assert_eq!(
                core::ptr::read_unaligned(control.as_ptr() as *const usize),
                CMSG_HDR_LEN + 4
            );
            assert_eq!(
                core::ptr::read_unaligned(control.as_ptr().add(8) as *const i32),
                SOL_IP
            );
            assert_eq!(
                core::ptr::read_unaligned(control.as_ptr().add(12) as *const i32),
                IP_TTL
            );
            assert_eq!(
                core::ptr::read_unaligned(control.as_ptr().add(CMSG_HDR_LEN) as *const i32),
                64
            );

            sched::set_current(previous);
        }
    }

    #[test]
    fn inet_ip_pktinfo_and_unicast_if_sockopts_follow_linux_levels() {
        crate::net::device::init();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 5562;
        current.tgid = 5562;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(AF_INET as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(fd >= 0);

            let one = 1u32;
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_IP,
                    IP_PKTINFO,
                    &one as *const u32 as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                0
            );
            let mut pktinfo = 0u32;
            let mut pktinfo_len = core::mem::size_of::<u32>() as u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_IP,
                    IP_PKTINFO,
                    &mut pktinfo as *mut u32 as *mut u8,
                    &mut pktinfo_len,
                ),
                0
            );
            assert_eq!(pktinfo, 1);

            let lo = crate::net::device::lookup_netdevice("lo").expect("loopback");
            let ifindex = lo.ifindex;
            let ifindex_sockopt = ifindex.to_be();
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_IP,
                    IP_UNICAST_IF,
                    &ifindex_sockopt as *const u32 as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                0
            );
            let sock = socket_from_fd(fd as i32).unwrap();
            assert_eq!(socket::get_unicast_if(&sock), Ok(ifindex));
            let mut unicast_if = 0u32;
            let mut unicast_if_len = core::mem::size_of::<u32>() as u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_IP,
                    IP_UNICAST_IF,
                    &mut unicast_if as *mut u32 as *mut u8,
                    &mut unicast_if_len,
                ),
                0
            );
            assert_eq!(unicast_if, ifindex_sockopt);

            let bind_name = b"lo\0";
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_SOCKET,
                    SO_BINDTODEVICE,
                    bind_name.as_ptr(),
                    bind_name.len() as u32,
                ),
                0
            );
            assert_eq!(socket::get_bound_ifindex(&sock), Ok(ifindex));

            let fd6 = sys_socket(AF_INET6 as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(fd6 >= 0);
            assert_eq!(
                sys_setsockopt(
                    fd6 as i32,
                    SOL_IPV6,
                    IPV6_UNICAST_IF,
                    &ifindex_sockopt as *const u32 as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                0
            );
            let sock6 = socket_from_fd(fd6 as i32).unwrap();
            assert_eq!(socket::get_unicast_if(&sock6), Ok(ifindex));
            let mut unicast_if6 = 0u32;
            let mut unicast_if6_len = core::mem::size_of::<u32>() as u32;
            assert_eq!(
                sys_getsockopt(
                    fd6 as i32,
                    SOL_IPV6,
                    IPV6_UNICAST_IF,
                    &mut unicast_if6 as *mut u32 as *mut u8,
                    &mut unicast_if6_len,
                ),
                0
            );
            assert_eq!(unicast_if6, ifindex_sockopt);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn inet_ip_mtu_getsockopt_follows_connected_socket_state() {
        crate::net::device::init();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 5564;
        current.tgid = 5564;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(AF_INET as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(fd >= 0);

            let mut mtu = 0u32;
            let mut mtu_len = core::mem::size_of::<u32>() as u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_IP,
                    IP_MTU,
                    &mut mtu as *mut u32 as *mut u8,
                    &mut mtu_len,
                ),
                -(ENOTCONN as i64)
            );

            let loopback = LinuxSockAddrIn {
                family: AF_INET,
                port: 53u16.to_be(),
                addr: u32::from_ne_bytes(ipv4(127, 0, 0, 1).to_be_bytes()),
                zero: [0; 8],
            };
            assert_eq!(
                sys_connect(
                    fd as i32,
                    &loopback as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                0
            );

            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_IP,
                    IP_MTU,
                    &mut mtu as *mut u32 as *mut u8,
                    &mut mtu_len,
                ),
                0
            );
            assert_eq!(mtu, crate::net::device::LOOPBACK_MTU);
            assert_eq!(mtu_len, core::mem::size_of::<u32>() as u32);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn inet_udp_recvmsg_attaches_pktinfo_control_message() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 5563;
        current.tgid = 5563;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let server = sys_socket(AF_INET as i32, socket::SOCK_DGRAM as i32, 0);
            let client = sys_socket(AF_INET as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(server >= 0);
            assert!(client >= 0);

            let server_addr = LinuxSockAddrIn {
                family: AF_INET,
                port: 5301u16.to_be(),
                addr: u32::from_ne_bytes(ipv4(127, 0, 0, 53).to_be_bytes()),
                zero: [0; 8],
            };
            let client_addr = LinuxSockAddrIn {
                family: AF_INET,
                port: 5302u16.to_be(),
                addr: u32::from_ne_bytes(ipv4(127, 0, 0, 1).to_be_bytes()),
                zero: [0; 8],
            };
            assert_eq!(
                sys_bind(
                    server as i32,
                    &server_addr as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                0
            );
            assert_eq!(
                sys_bind(
                    client as i32,
                    &client_addr as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                0
            );

            let one = 1u32;
            assert_eq!(
                sys_setsockopt(
                    server as i32,
                    SOL_IP,
                    IP_PKTINFO,
                    &one as *const u32 as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                0
            );

            let payload = b"dns";
            assert_eq!(
                sys_sendto(
                    client as i32,
                    payload.as_ptr(),
                    payload.len(),
                    0,
                    &server_addr as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                payload.len() as i64
            );

            let mut out = [0u8; 16];
            let mut iov = LinuxIovec {
                base: out.as_mut_ptr(),
                len: out.len(),
            };
            let mut control = [0u8; 64];
            let mut hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut iov,
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control.len(),
                flags: 0,
            };
            assert_eq!(
                sys_recvmsg(server as i32, &mut hdr, 0),
                payload.len() as i64
            );
            assert_eq!(&out[..payload.len()], payload);
            assert_eq!(
                core::ptr::read_unaligned(control.as_ptr().add(8) as *const i32),
                SOL_IP
            );
            assert_eq!(
                core::ptr::read_unaligned(control.as_ptr().add(12) as *const i32),
                IP_PKTINFO
            );
            let pktinfo = core::ptr::read_unaligned(
                control.as_ptr().add(CMSG_HDR_LEN) as *const LinuxInPktinfo
            );
            assert_eq!(pktinfo.ipi_ifindex, 0);
            assert_eq!(
                pktinfo.ipi_spec_dst,
                u32::from_ne_bytes(ipv4(127, 0, 0, 53).to_be_bytes())
            );
            assert_eq!(pktinfo.ipi_addr, pktinfo.ipi_spec_dst);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn ip_recvttl_sockopt_rejects_non_user_pointers() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 5561;
        current.tgid = 5561;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(
                AF_INET as i32,
                socket::SOCK_DGRAM as i32,
                IPPROTO_ICMP as i32,
            );
            assert!(fd >= 0);

            let kernel_addr = (1u64 << 47) as *mut u8;
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_IP,
                    socket::IP_RECVTTL as i32,
                    kernel_addr as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                -(EFAULT as i64)
            );

            let mut value = 0u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_IP,
                    socket::IP_RECVTTL as i32,
                    &mut value as *mut u32 as *mut u8,
                    kernel_addr as *mut u32,
                ),
                -(EFAULT as i64)
            );

            let mut len = core::mem::size_of::<u32>() as u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_IP,
                    socket::IP_RECVTTL as i32,
                    kernel_addr,
                    &mut len,
                ),
                -(EFAULT as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn inet_icmp_recvmsg_skips_ttl_control_message_by_default() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 281;
        current.tgid = 281;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(
                AF_INET as i32,
                socket::SOCK_DGRAM as i32,
                IPPROTO_ICMP as i32,
            );
            assert!(fd >= 0);
            let dest = LinuxSockAddrIn {
                family: AF_INET,
                port: 0,
                addr: u32::from_ne_bytes(ipv4(93, 184, 216, 34).to_be_bytes()),
                zero: [0; 8],
            };
            let mut echo = alloc::vec![8, 0, 0, 0, 0x12, 0x34, 0x00, 0x02, b'p', b'i', b'n', b'g'];
            let csum = checksum(&echo);
            echo[2..4].copy_from_slice(&csum.to_be_bytes());
            assert_eq!(
                sys_sendto(
                    fd as i32,
                    echo.as_ptr(),
                    echo.len(),
                    0,
                    &dest as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                echo.len() as i64
            );

            let mut out = [0u8; 64];
            let mut iov = LinuxIovec {
                base: out.as_mut_ptr(),
                len: out.len(),
            };
            let mut control = [0xccu8; 32];
            let mut hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut iov,
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control.len(),
                flags: 0,
            };
            assert_eq!(sys_recvmsg(fd as i32, &mut hdr, 0), echo.len() as i64);
            assert_eq!(hdr.controllen, 0);
            assert_eq!(hdr.flags & MSG_CTRUNC, 0);
            assert!(control.iter().all(|byte| *byte == 0xcc));

            sched::set_current(previous);
        }
    }

    #[test]
    fn inet_recvmsg_ttl_control_rejects_bad_user_pointer() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 283;
        current.tgid = 283;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(
                AF_INET as i32,
                socket::SOCK_DGRAM as i32,
                IPPROTO_ICMP as i32,
            );
            assert!(fd >= 0);
            let on = 1u32;
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_IP,
                    socket::IP_RECVTTL as i32,
                    &on as *const u32 as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                0
            );
            let dest = LinuxSockAddrIn {
                family: AF_INET,
                port: 0,
                addr: u32::from_ne_bytes(ipv4(93, 184, 216, 34).to_be_bytes()),
                zero: [0; 8],
            };
            let mut echo = alloc::vec![8, 0, 0, 0, 0x12, 0x34, 0x00, 0x03, b'p', b'i', b'n', b'g'];
            let csum = checksum(&echo);
            echo[2..4].copy_from_slice(&csum.to_be_bytes());
            assert_eq!(
                sys_sendto(
                    fd as i32,
                    echo.as_ptr(),
                    echo.len(),
                    0,
                    &dest as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                echo.len() as i64
            );

            let mut out = [0u8; 64];
            let mut iov = LinuxIovec {
                base: out.as_mut_ptr(),
                len: out.len(),
            };
            let mut hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut iov,
                iovlen: 1,
                control: crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX as *mut u8,
                controllen: cmsg_align(CMSG_HDR_LEN + core::mem::size_of::<i32>()),
                flags: 0,
            };
            assert_eq!(sys_recvmsg(fd as i32, &mut hdr, 0), -(EFAULT as i64));

            sched::set_current(previous);
        }
    }

    #[test]
    fn inet_datagram_socket_poll_reports_writable_before_connect() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 280;
        current.tgid = 280;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(
                AF_INET as i32,
                socket::SOCK_DGRAM as i32,
                IPPROTO_ICMP as i32,
            );
            assert!(fd >= 0);
            let ft = files::get_task_files(&mut *current as *mut TaskStruct).unwrap();
            let file = ft.get(fd as i32).unwrap();

            assert_ne!(
                crate::fs::select::poll_mask(&file) & crate::fs::select::POLLOUT as u32,
                0,
                "Linux reports unconnected datagram/raw sockets writable when send buffer space exists"
            );

            sched::set_current(previous);
        }
    }

    fn icmp_echo(seq: u16) -> alloc::vec::Vec<u8> {
        let mut echo = alloc::vec![8, 0, 0, 0, 0x12, 0x34];
        echo.extend_from_slice(&seq.to_be_bytes());
        echo.extend_from_slice(b"ping");
        let csum = checksum(&echo);
        echo[2..4].copy_from_slice(&csum.to_be_bytes());
        echo
    }

    #[test]
    fn inet_icmp_ping_can_poll_writable_between_replies() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 281;
        current.tgid = 281;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(
                AF_INET as i32,
                socket::SOCK_DGRAM as i32,
                IPPROTO_ICMP as i32,
            );
            assert!(fd >= 0);
            let dest = LinuxSockAddrIn {
                family: AF_INET,
                port: 0,
                addr: u32::from_ne_bytes(ipv4(93, 184, 216, 34).to_be_bytes()),
                zero: [0; 8],
            };

            let first = icmp_echo(1);
            assert_eq!(
                sys_sendto(
                    fd as i32,
                    first.as_ptr(),
                    first.len(),
                    0,
                    &dest as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                first.len() as i64
            );
            let mut out = [0u8; 64];
            assert_eq!(
                sys_recvfrom(
                    fd as i32,
                    out.as_mut_ptr(),
                    out.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                first.len() as i64
            );

            let mut pfd = crate::fs::select::PollFd {
                fd: fd as i32,
                events: crate::fs::select::POLLIN | crate::fs::select::POLLOUT,
                revents: 0,
            };
            assert_eq!(crate::fs::syscalls::sys_poll(&mut pfd, 1, 0), 1);
            assert_eq!(
                pfd.revents & crate::fs::select::POLLOUT,
                crate::fs::select::POLLOUT
            );

            let second = icmp_echo(2);
            assert_eq!(
                sys_sendto(
                    fd as i32,
                    second.as_ptr(),
                    second.len(),
                    0,
                    &dest as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrIn>() as u32,
                ),
                second.len() as i64
            );
            assert_eq!(
                sys_recvfrom(
                    fd as i32,
                    out.as_mut_ptr(),
                    out.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                second.len() as i64
            );
            assert_eq!(out[0], 0);
            assert_eq!(&out[6..8], &2u16.to_be_bytes());

            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m78_socket_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 278;
        current.tgid = 278;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_socket(AF_UNIX as i32, 1, 0);
            assert!(fd >= 0);
            let flagged_fd = sys_socket(
                AF_UNIX as i32,
                (socket::SOCK_STREAM as u32 | socket::SOCK_CLOEXEC | socket::SOCK_NONBLOCK) as i32,
                0,
            );
            assert!(flagged_fd >= 0);
            let flagged_file = current_files().unwrap().get(flagged_fd as i32).unwrap();
            assert_eq!(
                current_files()
                    .unwrap()
                    .get_fd_flags(flagged_fd as i32)
                    .unwrap()
                    & FD_CLOEXEC,
                FD_CLOEXEC
            );
            assert_eq!(
                flagged_file.flags.load(Ordering::Acquire) & O_NONBLOCK,
                O_NONBLOCK
            );
            assert_eq!(
                sys_socket(
                    AF_UNIX as i32,
                    (socket::SOCK_STREAM as u32 | 0x4000_0000) as i32,
                    0
                ),
                -(EINVAL as i64)
            );
            let mut sock_type = 0u32;
            let mut sock_type_len = 4u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_SOCKET,
                    socket::SO_TYPE as i32,
                    &mut sock_type as *mut u32 as *mut u8,
                    &mut sock_type_len,
                ),
                0
            );
            assert_eq!(sock_type, socket::SOCK_STREAM as u32);
            assert_eq!(sock_type_len, 4);
            let sndbuf = 262_144u32;
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_SOCKET,
                    socket::SO_SNDBUF as i32,
                    &sndbuf as *const u32 as *const u8,
                    4,
                ),
                0
            );
            let mut sndbuf_out = 0u32;
            let mut sndbuf_len = 4u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    SOL_SOCKET,
                    socket::SO_SNDBUF as i32,
                    &mut sndbuf_out as *mut u32 as *mut u8,
                    &mut sndbuf_len,
                ),
                0
            );
            assert_ne!(sndbuf_out, 0);
            assert_eq!(sys_bind(fd as i32, core::ptr::null(), 1), -(EFAULT as i64));
            assert_eq!(sys_listen(fd as i32, 4), -(EINVAL as i64));
            assert_eq!(sys_connect(-1, core::ptr::null(), 0), -(EBADF as i64));
            assert_eq!(
                sys_accept(-1, core::ptr::null_mut(), core::ptr::null_mut()),
                -(EBADF as i64)
            );
            assert_eq!(
                sys_accept4(-1, core::ptr::null_mut(), core::ptr::null_mut(), 0),
                -(EBADF as i64)
            );

            let mut sv = [0i32; 2];
            assert_eq!(sys_socketpair(AF_UNIX as i32, 1, 0, sv.as_mut_ptr()), 0);
            let msg = b"hello";
            assert_eq!(
                sys_sendto(sv[0], msg.as_ptr(), msg.len(), 0, core::ptr::null(), 0),
                msg.len() as i64
            );
            let mut out = [0u8; 5];
            assert_eq!(
                sys_recvfrom(
                    sv[1],
                    out.as_mut_ptr(),
                    out.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                msg.len() as i64
            );
            assert_eq!(&out, msg);

            let socket_file = current_files().unwrap().get(sv[1]).unwrap();
            assert_eq!(
                crate::fs::select::poll_mask(&socket_file) & crate::fs::select::POLLIN as u32,
                0
            );
            let write_msg = b"write";
            assert_eq!(
                crate::fs::read_write::sys_write(sv[0], write_msg.as_ptr(), write_msg.len()),
                write_msg.len() as i64
            );
            assert_ne!(
                crate::fs::select::poll_mask(&socket_file) & crate::fs::select::POLLIN as u32,
                0
            );
            let mut read_out = [0u8; 5];
            assert_eq!(
                crate::fs::read_write::sys_read(sv[1], read_out.as_mut_ptr(), read_out.len()),
                write_msg.len() as i64
            );
            assert_eq!(&read_out, write_msg);

            let mut seq_sv = [0i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_SEQPACKET as i32,
                    0,
                    seq_sv.as_mut_ptr()
                ),
                0
            );
            let part_a = b"user:";
            let part_b = b"lookup";
            let writev_iov = [
                crate::fs::syscalls::IoVec {
                    iov_base: part_a.as_ptr() as *mut u8,
                    iov_len: part_a.len(),
                },
                crate::fs::syscalls::IoVec {
                    iov_base: part_b.as_ptr() as *mut u8,
                    iov_len: part_b.len(),
                },
            ];
            assert_eq!(
                crate::fs::syscalls::sys_writev(seq_sv[0], writev_iov.as_ptr(), writev_iov.len()),
                (part_a.len() + part_b.len()) as i64
            );
            let mut packet_out = [0u8; 11];
            assert_eq!(
                sys_recvfrom(
                    seq_sv[1],
                    packet_out.as_mut_ptr(),
                    packet_out.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                packet_out.len() as i64
            );
            assert_eq!(&packet_out, b"user:lookup");

            let mut iov = LinuxIovec {
                base: msg.as_ptr() as *mut u8,
                len: msg.len(),
            };
            let hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut iov,
                iovlen: 1,
                control: core::ptr::null_mut(),
                controllen: 0,
                flags: 0,
            };
            assert_eq!(sys_sendmsg(sv[0], &hdr, 0), msg.len() as i64);
            let mut recv = [0u8; 5];
            let mut riov = LinuxIovec {
                base: recv.as_mut_ptr(),
                len: recv.len(),
            };
            let mut rhdr = LinuxMsghdr {
                iov: &mut riov,
                iovlen: 1,
                ..hdr
            };
            assert_eq!(sys_recvmsg(sv[1], &mut rhdr, 0), msg.len() as i64);
            assert_eq!(&recv, msg);

            let mut mmsg = LinuxMmsghdr {
                msg_hdr: hdr,
                msg_len: 0,
            };
            assert_eq!(sys_sendmmsg(sv[0], &mut mmsg, 1, 0), 1);
            let mut rmmsg = LinuxMmsghdr {
                msg_hdr: rhdr,
                msg_len: 0,
            };
            assert_eq!(
                sys_recvmmsg(sv[1], &mut rmmsg, 1, 0, core::ptr::null_mut()),
                1
            );

            let opt = 1u32;
            assert_eq!(
                sys_setsockopt(fd as i32, 1, 1, &opt as *const u32 as *const u8, 4),
                -(EINVAL as i64)
            );
            let mut opt_out = 0u32;
            let mut opt_len = 4u32;
            assert_eq!(
                sys_getsockopt(
                    fd as i32,
                    1,
                    1,
                    &mut opt_out as *mut u32 as *mut u8,
                    &mut opt_len,
                ),
                -(EINVAL as i64)
            );
            let mut unnamed_unix = [0u8; 2];
            let mut unnamed_unix_len = unnamed_unix.len() as u32;
            assert_eq!(
                sys_getsockname(fd as i32, unnamed_unix.as_mut_ptr(), &mut unnamed_unix_len,),
                0
            );
            assert_eq!(unnamed_unix_len, 2);
            assert_eq!(unnamed_unix, AF_UNIX.to_ne_bytes());
            assert_eq!(
                sys_getsockname(fd as i32, core::ptr::null_mut(), core::ptr::null_mut()),
                -(EFAULT as i64)
            );
            assert_eq!(
                sys_getpeername(fd as i32, core::ptr::null_mut(), core::ptr::null_mut()),
                -(ENOTCONN as i64)
            );
            assert_eq!(sys_shutdown(fd as i32, 0), 0);

            let nl_fd = sys_socket(AF_NETLINK as i32, socket::SOCK_RAW as i32, 0);
            assert!(nl_fd >= 0);
            let mut unbound_nl = LinuxSockAddrNetlink {
                family: 0,
                pad: 0,
                pid: 0xdead,
                groups: 0xbeef,
            };
            let mut unbound_nl_len = core::mem::size_of::<LinuxSockAddrNetlink>() as u32;
            assert_eq!(
                sys_getsockname(
                    nl_fd as i32,
                    &mut unbound_nl as *mut _ as *mut u8,
                    &mut unbound_nl_len,
                ),
                0
            );
            assert_eq!(unbound_nl_len, 12);
            assert_eq!(unbound_nl.family, AF_NETLINK);
            assert_eq!(unbound_nl.pid, 0);
            assert_eq!(unbound_nl.groups, 0);

            let nl_addr = LinuxSockAddrNetlink {
                family: AF_NETLINK,
                pad: 0,
                pid: 0,
                groups: 1361,
            };
            assert_eq!(
                sys_bind(
                    nl_fd as i32,
                    &nl_addr as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrNetlink>() as u32,
                ),
                0
            );
            assert_eq!(sys_listen(nl_fd as i32, 4096), 0);
            assert_eq!(
                sys_setsockopt(nl_fd as i32, 270, 3, &opt as *const u32 as *const u8, 4),
                0
            );

            // getsockname() on a bound AF_NETLINK socket must return the
            // sockaddr_nl that was bound, all 12 bytes.  Ref:
            // vendor/linux/net/netlink/af_netlink.c::netlink_getname.
            let mut nl_out = LinuxSockAddrNetlink {
                family: 0,
                pad: 0,
                pid: 0xdead,
                groups: 0xbeef,
            };
            let mut nl_len = core::mem::size_of::<LinuxSockAddrNetlink>() as u32;
            assert_eq!(
                sys_getsockname(nl_fd as i32, &mut nl_out as *mut _ as *mut u8, &mut nl_len,),
                0
            );
            assert_eq!(nl_len, 12);
            assert_eq!(nl_out.family, AF_NETLINK);
            assert_eq!(nl_out.pad, 0);
            assert_ne!(nl_out.pid, 0);
            assert_eq!(nl_out.groups, 1361);

            // Linux netlink_bind() treats nl_pid=0 as an autobind request,
            // then netlink_getsockopt(NETLINK_LIST_MEMBERSHIPS) allows a
            // NULL optval probe that reports the required bitmap length.
            let mut memberships_len = 0u32;
            assert_eq!(
                sys_getsockopt(
                    nl_fd as i32,
                    SOL_NETLINK,
                    NETLINK_LIST_MEMBERSHIPS,
                    core::ptr::null_mut(),
                    &mut memberships_len,
                ),
                0
            );
            assert_eq!(memberships_len, 8);
            let mut memberships = [0u32; 2];
            assert_eq!(
                sys_getsockopt(
                    nl_fd as i32,
                    SOL_NETLINK,
                    NETLINK_LIST_MEMBERSHIPS,
                    memberships.as_mut_ptr() as *mut u8,
                    &mut memberships_len,
                ),
                0
            );
            assert_eq!(memberships_len, 8);
            assert_eq!(memberships[0], 1361);
            assert_eq!(memberships[1], 0);

            let nl_fd2 = sys_socket(AF_NETLINK as i32, socket::SOCK_RAW as i32, 0);
            assert!(nl_fd2 >= 0);
            assert_eq!(
                sys_bind(
                    nl_fd2 as i32,
                    &nl_addr as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrNetlink>() as u32,
                ),
                0
            );
            let mut nl_out2 = LinuxSockAddrNetlink {
                family: 0,
                pad: 0,
                pid: 0,
                groups: 0,
            };
            let mut nl_len2 = core::mem::size_of::<LinuxSockAddrNetlink>() as u32;
            assert_eq!(
                sys_getsockname(
                    nl_fd2 as i32,
                    &mut nl_out2 as *mut _ as *mut u8,
                    &mut nl_len2,
                ),
                0
            );
            assert_ne!(nl_out2.pid, nl_out.pid);

            // Linux's move_addr_to_user() truncates the copy to the caller's
            // buffer and still reports the required 12 bytes so libc can
            // retry with a larger sockaddr.
            let mut small_buf = [0u8; 4];
            let mut small_len = 4u32;
            assert_eq!(
                sys_getsockname(nl_fd as i32, small_buf.as_mut_ptr(), &mut small_len,),
                0
            );
            assert_eq!(small_len, 12);
            assert_eq!(&small_buf[..2], &AF_NETLINK.to_ne_bytes());

            // getsockname() receives userspace pointers from the syscall ABI;
            // reject kernel-space addresses instead of dereferencing them.
            let bad_user_addr = uaccess::TASK_SIZE_MAX as *mut u8;
            let mut bad_addr_len = core::mem::size_of::<LinuxSockAddrNetlink>() as u32;
            assert_eq!(
                sys_getsockname(nl_fd as i32, bad_user_addr, &mut bad_addr_len,),
                -(EFAULT as i64)
            );
            assert_eq!(bad_addr_len, 12);

            let mut bad_len_out = LinuxSockAddrNetlink {
                family: 0,
                pad: 0,
                pid: 0,
                groups: 0,
            };
            let bad_user_len = uaccess::TASK_SIZE_MAX as *mut u32;
            assert_eq!(
                sys_getsockname(
                    nl_fd as i32,
                    &mut bad_len_out as *mut _ as *mut u8,
                    bad_user_len,
                ),
                -(EFAULT as i64)
            );

            let audit_fd = sys_socket(
                AF_NETLINK as i32,
                socket::SOCK_RAW as i32,
                crate::net::rtnetlink::NETLINK_AUDIT as i32,
            );
            assert!(audit_fd >= 0);
            let audit_kernel = LinuxSockAddrNetlink {
                family: AF_NETLINK,
                pad: 0,
                pid: 0,
                groups: 0,
            };
            let audit_msg = b"audit-user";
            assert_eq!(
                sys_sendto(
                    audit_fd as i32,
                    audit_msg.as_ptr(),
                    audit_msg.len(),
                    0,
                    &audit_kernel as *const _ as *const u8,
                    core::mem::size_of::<LinuxSockAddrNetlink>() as u32,
                ),
                audit_msg.len() as i64
            );
            let mut audit_ack = [0u8; 64];
            assert!(
                sys_recvfrom(
                    audit_fd as i32,
                    audit_ack.as_mut_ptr(),
                    audit_ack.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ) > 0
            );
            let mut audit_iov = LinuxIovec {
                base: audit_msg.as_ptr() as *mut u8,
                len: audit_msg.len(),
            };
            let audit_hdr = LinuxMsghdr {
                name: &audit_kernel as *const _ as *mut u8,
                namelen: core::mem::size_of::<LinuxSockAddrNetlink>() as u32,
                iov: &mut audit_iov,
                iovlen: 1,
                control: core::ptr::null_mut(),
                controllen: 0,
                flags: 0,
            };
            assert_eq!(
                sys_sendmsg(audit_fd as i32, &audit_hdr, 0),
                audit_msg.len() as i64
            );
            assert!(
                sys_recvfrom(
                    audit_fd as i32,
                    audit_ack.as_mut_ptr(),
                    audit_ack.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ) > 0
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn sys_setsockopt_netlink_membership_replays_kobject_uevents() {
        let _guard = crate::net::uevent::test_lock();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 281;
        current.tgid = 281;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let _ = crate::net::uevent::drain_pending();
            crate::net::uevent::announce_class_device(
                "input",
                "event-syscall0",
                "input",
                "input/event-syscall0",
            );

            let fd = sys_socket(
                AF_NETLINK as i32,
                socket::SOCK_DGRAM as i32,
                crate::net::rtnetlink::NETLINK_KOBJECT_UEVENT as i32,
            );
            assert!(fd >= 0);
            let group = 1u32;
            assert_eq!(
                sys_setsockopt(
                    fd as i32,
                    SOL_NETLINK,
                    NETLINK_ADD_MEMBERSHIP,
                    &group as *const u32 as *const u8,
                    core::mem::size_of::<u32>() as u32,
                ),
                0
            );
            let mut nl_addr = LinuxSockAddrNetlink {
                family: 0,
                pad: 0,
                pid: 0,
                groups: 0,
            };
            let mut nl_addr_len = core::mem::size_of::<LinuxSockAddrNetlink>() as u32;
            assert_eq!(
                sys_getsockname(
                    fd as i32,
                    &mut nl_addr as *mut _ as *mut u8,
                    &mut nl_addr_len,
                ),
                0
            );
            assert_eq!(
                nl_addr_len,
                core::mem::size_of::<LinuxSockAddrNetlink>() as u32
            );
            assert_eq!(nl_addr.family, AF_NETLINK);
            assert_ne!(
                nl_addr.pid, 0,
                "membership join should autobind netlink portid"
            );
            assert_eq!(nl_addr.groups, 1);

            let mut out = [0u8; 256];
            let n = sys_recvfrom(
                fd as i32,
                out.as_mut_ptr(),
                out.len(),
                0,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
            assert!(n > 0);
            let payload = &out[..n as usize];
            assert!(payload.starts_with(b"add@/class/input/event-syscall0\0"));

            let _ = crate::net::uevent::drain_pending();
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_stream_sys_recvmsg_short_iov_preserves_remaining_auth_bytes() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 279;
        current.tgid = 279;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut sv = [0i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_STREAM as i32,
                    0,
                    sv.as_mut_ptr()
                ),
                0
            );

            let auth = b"\0AUTH EXTERNAL 31303030\r\nNEGOTIATE_UNIX_FD\r\nBEGIN\r\n";
            let mut send_iov = LinuxIovec {
                base: auth.as_ptr() as *mut u8,
                len: auth.len(),
            };
            let send_hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut send_iov,
                iovlen: 1,
                control: core::ptr::null_mut(),
                controllen: 0,
                flags: 0,
            };
            assert_eq!(sys_sendmsg(sv[0], &send_hdr, 0), auth.len() as i64);

            let mut first = [0u8; 1];
            let mut first_iov = LinuxIovec {
                base: first.as_mut_ptr(),
                len: first.len(),
            };
            let mut first_hdr = LinuxMsghdr {
                iov: &mut first_iov,
                iovlen: 1,
                ..send_hdr
            };
            assert_eq!(sys_recvmsg(sv[1], &mut first_hdr, 0), 1);
            assert_eq!(first[0], 0);

            let mut rest = [0u8; 64];
            let mut rest_iov = LinuxIovec {
                base: rest.as_mut_ptr(),
                len: rest.len(),
            };
            let mut rest_hdr = LinuxMsghdr {
                iov: &mut rest_iov,
                iovlen: 1,
                ..send_hdr
            };
            let n = sys_recvmsg(sv[1], &mut rest_hdr, 0);
            assert_eq!(n, (auth.len() - 1) as i64);
            assert_eq!(&rest[..n as usize], &auth[1..]);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_stream_sys_recvmsg_installs_scm_rights_with_cmsg_cloexec() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 281;
        current.tgid = 281;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut sv = [0i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_STREAM as i32,
                    0,
                    sv.as_mut_ptr()
                ),
                0
            );

            let dentry = crate::fs::dcache::d_alloc("systemd-pidfd");
            let passed = crate::fs::file::alloc_file(dentry, 0, 0, &crate::fs::ops::NOOP_FILE_OPS);
            let ft = current_files().unwrap();
            let source_fd = ft.install(passed.clone(), false).unwrap();
            let mut send_control = [0u8; 32];
            let (send_control_len, truncated) =
                write_scm_rights(send_control.as_mut_ptr(), send_control.len(), &[source_fd])
                    .unwrap();
            assert!(!truncated);

            let body = b"F";
            let mut iov = LinuxIovec {
                base: body.as_ptr() as *mut u8,
                len: body.len(),
            };
            let send_hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut iov,
                iovlen: 1,
                control: send_control.as_mut_ptr(),
                controllen: send_control_len,
                flags: 0,
            };
            assert_eq!(sys_sendmsg(sv[0], &send_hdr, 0), body.len() as i64);

            let mut recv_body = [0u8; 1];
            let mut recv_iov = LinuxIovec {
                base: recv_body.as_mut_ptr(),
                len: recv_body.len(),
            };
            let mut recv_control = [0u8; 64];
            let mut recv_hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: &mut recv_iov,
                iovlen: 1,
                control: recv_control.as_mut_ptr(),
                controllen: recv_control.len(),
                flags: 0,
            };
            assert_eq!(
                sys_recvmsg(sv[1], &mut recv_hdr, MSG_CMSG_CLOEXEC),
                body.len() as i64
            );
            assert_eq!(&recv_body, body);
            assert_eq!(recv_hdr.flags & MSG_CTRUNC, 0);
            assert_eq!(recv_hdr.controllen, cmsg_align(CMSG_HDR_LEN + 4));

            let cmsg_len = core::ptr::read_unaligned(recv_control.as_ptr() as *const usize);
            let cmsg_level = core::ptr::read_unaligned(recv_control.as_ptr().add(8) as *const i32);
            let cmsg_type = core::ptr::read_unaligned(recv_control.as_ptr().add(12) as *const i32);
            let received_fd =
                core::ptr::read_unaligned(recv_control.as_ptr().add(CMSG_HDR_LEN) as *const i32);
            assert_eq!(cmsg_len, CMSG_HDR_LEN + 4);
            assert_eq!(cmsg_level, SOL_SOCKET);
            assert_eq!(cmsg_type, SCM_RIGHTS);
            assert_ne!(received_fd, source_fd);
            assert!(alloc::sync::Arc::ptr_eq(
                &ft.get(received_fd).unwrap(),
                &passed
            ));
            assert_eq!(
                ft.get_fd_flags(received_fd).unwrap() & FD_CLOEXEC,
                FD_CLOEXEC
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn journald_runtime_socket_sequence_tolerates_missing_streams_and_options() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 280;
        current.tgid = 280;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run/systemd\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(
                    AT_FDCWD,
                    b"/run/systemd/journal\0".as_ptr(),
                    0o755
                ),
                0
            );
            assert_eq!(
                crate::fs::openat::sys_openat(
                    AT_FDCWD,
                    b"/run/systemd/journal/streams\0".as_ptr(),
                    (O_DIRECTORY | O_CLOEXEC | O_NONBLOCK) as i32,
                    0
                ),
                -(ENOENT as i64)
            );

            let one = 1u32;
            let zero = 0u32;
            let sock_flags = socket::SOCK_CLOEXEC | socket::SOCK_NONBLOCK;

            let stdout_fd = sys_socket(
                AF_UNIX as i32,
                (socket::SOCK_STREAM as u32 | sock_flags) as i32,
                0,
            );
            assert!(stdout_fd >= 0);
            let (stdout_addr, stdout_len) = unix_sockaddr("/run/systemd/journal/stdout");
            assert_eq!(
                sys_bind(stdout_fd as i32, stdout_addr.as_ptr(), stdout_len),
                0
            );
            assert_eq!(sys_listen(stdout_fd as i32, 4096), 0);
            let mut stdout_name = [0u8; 128];
            let mut stdout_name_len = stdout_name.len() as u32;
            assert_eq!(
                sys_getsockname(
                    stdout_fd as i32,
                    stdout_name.as_mut_ptr(),
                    &mut stdout_name_len,
                ),
                0
            );
            assert_eq!(stdout_name_len, stdout_len);
            assert_eq!(
                &stdout_name[..stdout_len as usize],
                &stdout_addr[..stdout_len as usize]
            );
            assert_eq!(
                sys_setsockopt(
                    stdout_fd as i32,
                    SOL_SOCKET,
                    socket::SO_PASSCRED as i32,
                    &one as *const u32 as *const u8,
                    4
                ),
                0
            );

            for (path, passrights) in [
                ("/run/systemd/journal/dev-log", true),
                ("/run/systemd/journal/socket", false),
            ] {
                let fd = sys_socket(
                    AF_UNIX as i32,
                    (socket::SOCK_DGRAM as u32 | sock_flags) as i32,
                    0,
                );
                assert!(fd >= 0);
                let (addr, len) = unix_sockaddr(path);
                assert_eq!(sys_bind(fd as i32, addr.as_ptr(), len), 0);
                assert_eq!(
                    sys_setsockopt(
                        fd as i32,
                        SOL_SOCKET,
                        socket::SO_PASSCRED as i32,
                        &one as *const u32 as *const u8,
                        4
                    ),
                    0
                );
                if passrights {
                    assert_eq!(
                        sys_setsockopt(
                            fd as i32,
                            SOL_SOCKET,
                            socket::SO_PASSRIGHTS as i32,
                            &zero as *const u32 as *const u8,
                            4
                        ),
                        0
                    );
                }
                assert_eq!(
                    sys_setsockopt(
                        fd as i32,
                        SOL_SOCKET,
                        socket::SO_TIMESTAMP_OLD as i32,
                        &one as *const u32 as *const u8,
                        4
                    ),
                    0
                );
            }

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_accept4_honors_flags_and_reports_peercred() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 315;
        current.tgid = 315;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run/systemd\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(
                    AT_FDCWD,
                    b"/run/systemd/journal\0".as_ptr(),
                    0o755
                ),
                0
            );

            let listener = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(listener >= 0);
            let (addr, addr_len) = unix_sockaddr("/run/systemd/journal/stdout");
            assert_eq!(sys_bind(listener as i32, addr.as_ptr(), addr_len), 0);
            assert_eq!(sys_listen(listener as i32, 4096), 0);
            let ft = current_files().unwrap();

            let client = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(client >= 0);
            assert_eq!(sys_connect(client as i32, addr.as_ptr(), addr_len), 0);
            let listener_file = ft.get(listener as i32).unwrap();
            let listener_mask = crate::fs::select::poll_mask(&listener_file);
            assert_ne!(
                listener_mask & crate::fs::select::POLLIN as u32,
                0,
                "listener should report accept readiness with POLLIN"
            );
            assert_ne!(
                listener_mask & crate::fs::select::POLLRDNORM as u32,
                0,
                "Linux AF_UNIX poll also reports listener readiness as POLLRDNORM"
            );

            let accepted = sys_accept4(
                listener as i32,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                (socket::SOCK_CLOEXEC | socket::SOCK_NONBLOCK) as i32,
            );
            assert!(accepted >= 0);
            let file = ft.get(accepted as i32).unwrap();
            assert_eq!(file.flags.load(Ordering::Acquire) & O_NONBLOCK, O_NONBLOCK);
            assert_eq!(
                ft.get_fd_flags(accepted as i32).unwrap() & FD_CLOEXEC,
                FD_CLOEXEC
            );
            assert_eq!(
                crate::fs::select::poll_mask(&listener_file) & crate::fs::select::POLLIN as u32,
                0,
                "listener must stop reporting accept readiness after accept drains backlog"
            );
            let message = b"hello";
            assert_eq!(
                sys_sendto(
                    client as i32,
                    message.as_ptr(),
                    message.len(),
                    0,
                    core::ptr::null(),
                    0
                ),
                message.len() as i64
            );
            assert_eq!(
                crate::fs::select::poll_mask(&listener_file) & crate::fs::select::POLLIN as u32,
                0,
                "stream payload data belongs to the accepted socket, not the listener"
            );
            let accepted_mask = crate::fs::select::poll_mask(&file);
            assert_ne!(
                accepted_mask & crate::fs::select::POLLIN as u32,
                0,
                "accepted stream socket should become readable after peer writes"
            );
            assert_ne!(
                accepted_mask & crate::fs::select::POLLRDNORM as u32,
                0,
                "Linux AF_UNIX poll also reports stream payload readiness as POLLRDNORM"
            );
            listener_file
                .flags
                .store(O_RDWR | O_NONBLOCK, Ordering::Release);
            assert_eq!(
                sys_accept4(
                    listener as i32,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    (socket::SOCK_CLOEXEC | socket::SOCK_NONBLOCK) as i32,
                ),
                -(EAGAIN as i64)
            );

            let mut cred = LinuxUcred::default();
            let mut cred_len = core::mem::size_of::<LinuxUcred>() as u32;
            assert_eq!(
                sys_getsockopt(
                    accepted as i32,
                    SOL_SOCKET,
                    socket::SO_PEERCRED as i32,
                    &mut cred as *mut LinuxUcred as *mut u8,
                    &mut cred_len,
                ),
                0
            );
            assert_eq!(cred_len, core::mem::size_of::<LinuxUcred>() as u32);
            assert_eq!(
                cred,
                LinuxUcred {
                    pid: 315,
                    uid: 0,
                    gid: 0
                }
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_accept4_truncates_abstract_peer_addr_without_dropping_connection() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 331;
        current.tgid = 331;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let abstract_addr = |name: &[u8]| {
                let mut raw = [0u8; 128];
                raw[..2].copy_from_slice(&AF_UNIX.to_ne_bytes());
                raw[2..2 + name.len()].copy_from_slice(name);
                (raw, (2 + name.len()) as u32)
            };

            let listener = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(listener >= 0);
            let (server_addr, server_len) = abstract_addr(b"\0dbus-system-bus");
            assert_eq!(
                sys_bind(listener as i32, server_addr.as_ptr(), server_len),
                0
            );
            assert_eq!(sys_listen(listener as i32, 4096), 0);

            let client = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(client >= 0);
            let client_name = b"\0systemctl-long-abstract-peer";
            let (client_addr, client_len) = abstract_addr(client_name);
            assert_eq!(sys_bind(client as i32, client_addr.as_ptr(), client_len), 0);
            assert_eq!(
                sys_connect(client as i32, server_addr.as_ptr(), server_len),
                0
            );

            let mut peer_out = [0u8; 4];
            let mut peer_len = peer_out.len() as u32;
            let accepted = sys_accept4(
                listener as i32,
                peer_out.as_mut_ptr(),
                &mut peer_len,
                socket::SOCK_CLOEXEC as i32,
            );
            assert!(accepted >= 0);
            assert_eq!(peer_len, (2 + client_name.len()) as u32);
            assert_eq!(&peer_out[..2], &AF_UNIX.to_ne_bytes());
            assert_eq!(&peer_out[2..], &client_name[..2]);

            let payload = b"hello";
            assert_eq!(
                sys_sendto(
                    client as i32,
                    payload.as_ptr(),
                    payload.len(),
                    0,
                    core::ptr::null(),
                    0,
                ),
                payload.len() as i64
            );
            let mut recv = [0u8; 8];
            assert_eq!(
                sys_recvfrom(
                    accepted as i32,
                    recv.as_mut_ptr(),
                    recv.len(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                payload.len() as i64
            );
            assert_eq!(&recv[..payload.len()], payload);
            let listener_file = current_files().unwrap().get(listener as i32).unwrap();
            listener_file
                .flags
                .store(O_RDWR | O_NONBLOCK, Ordering::Release);
            assert_eq!(
                sys_accept4(
                    listener as i32,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    socket::SOCK_CLOEXEC as i32,
                ),
                -(EAGAIN as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_passcred_recvmsg_delivers_scm_credentials() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 424;
        current.tgid = 424;
        current.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(current.pid)).expect("pid alloc");
        current.m26.thread_pid = Box::into_raw(kpid);
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );

            let server = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(server >= 0);
            let server = server as i32;
            let one = 1u32;
            assert_eq!(
                sys_setsockopt(
                    server,
                    SOL_SOCKET,
                    socket::SO_PASSCRED as i32,
                    &one as *const u32 as *const u8,
                    4,
                ),
                0
            );
            assert_eq!(
                sys_setsockopt(
                    server,
                    SOL_SOCKET,
                    socket::SO_PASSPIDFD as i32,
                    &one as *const u32 as *const u8,
                    4,
                ),
                0
            );
            let (addr, addr_len) = unix_sockaddr("/run/notify-passcred");
            assert_eq!(sys_bind(server, addr.as_ptr(), addr_len), 0);

            let client = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(client >= 0);
            let client = client as i32;
            assert_eq!(sys_connect(client, addr.as_ptr(), addr_len), 0);
            let payload = b"READY=1\n";
            assert_eq!(
                sys_sendto(
                    client as i32,
                    payload.as_ptr(),
                    payload.len(),
                    0,
                    core::ptr::null(),
                    0,
                ),
                payload.len() as i64
            );

            let mut body = [0u8; 32];
            let mut iov = [LinuxIovec {
                base: body.as_mut_ptr(),
                len: body.len(),
            }];
            let mut control = [0u8; 64];
            let mut hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: iov.as_mut_ptr(),
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control.len(),
                flags: 0,
            };
            assert_eq!(
                sys_recvmsg(server as i32, &mut hdr, 0),
                payload.len() as i64
            );
            assert_eq!(&body[..payload.len()], payload);
            assert_eq!(hdr.flags & MSG_CTRUNC, 0);
            let cred_cmsg_len = CMSG_HDR_LEN + core::mem::size_of::<LinuxUcred>();
            let pidfd_cmsg_off = cmsg_align(cred_cmsg_len);
            assert_eq!(
                hdr.controllen,
                pidfd_cmsg_off + cmsg_align(CMSG_HDR_LEN + core::mem::size_of::<i32>())
            );

            let cmsg_len = core::ptr::read_unaligned(control.as_ptr() as *const usize);
            let cmsg_level = core::ptr::read_unaligned(control.as_ptr().add(8) as *const i32);
            let cmsg_type = core::ptr::read_unaligned(control.as_ptr().add(12) as *const i32);
            let cred =
                core::ptr::read_unaligned(control.as_ptr().add(CMSG_HDR_LEN) as *const LinuxUcred);
            assert_eq!(cmsg_len, cred_cmsg_len);
            assert_eq!(cmsg_level, SOL_SOCKET);
            assert_eq!(cmsg_type, SCM_CREDENTIALS);
            assert_eq!(
                cred,
                LinuxUcred {
                    pid: 424,
                    uid: 0,
                    gid: 0
                }
            );
            let pidfd_cmsg = control.as_ptr().add(pidfd_cmsg_off);
            let pidfd_cmsg_len = core::ptr::read_unaligned(pidfd_cmsg as *const usize);
            let pidfd_cmsg_level = core::ptr::read_unaligned(pidfd_cmsg.add(8) as *const i32);
            let pidfd_cmsg_type = core::ptr::read_unaligned(pidfd_cmsg.add(12) as *const i32);
            let pidfd = core::ptr::read_unaligned(pidfd_cmsg.add(CMSG_HDR_LEN) as *const i32);
            assert_eq!(pidfd_cmsg_len, CMSG_HDR_LEN + core::mem::size_of::<i32>());
            assert_eq!(pidfd_cmsg_level, SOL_SOCKET);
            assert_eq!(pidfd_cmsg_type, SCM_PIDFD);
            assert_eq!(crate::fs::pidfd::pid_for_fd(pidfd), Ok(424));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(current.m26.thread_pid);
            current.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn unix_passcred_recvmsg_delivers_sendmsg_name_credentials() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut manager = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        manager.pid = 1;
        manager.tgid = 1;
        manager.cred = &raw const INIT_CRED;
        let manager_kpid = alloc_pid(&INIT_PID_NS, Some(manager.pid)).expect("manager pid alloc");
        manager.m26.thread_pid = Box::into_raw(manager_kpid);

        let mut service = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        service.pid = 465;
        service.tgid = 465;
        service.cred = &raw const INIT_CRED;
        let service_kpid = alloc_pid(&INIT_PID_NS, Some(service.pid)).expect("service pid alloc");
        service.m26.thread_pid = Box::into_raw(service_kpid);

        unsafe {
            files::set_task_files(&mut *manager as *mut TaskStruct, FilesStruct::new());
            files::set_task_files(&mut *service as *mut TaskStruct, FilesStruct::new());

            sched::set_current(&mut *manager as *mut TaskStruct);
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run/systemd\0".as_ptr(), 0o755),
                0
            );

            let server = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(server >= 0);
            let server = server as i32;
            let one = 1u32;
            assert_eq!(
                sys_setsockopt(
                    server as i32,
                    SOL_SOCKET,
                    socket::SO_PASSCRED as i32,
                    &one as *const u32 as *const u8,
                    4,
                ),
                0
            );
            let (addr, addr_len) = unix_sockaddr("/run/systemd/notify");
            assert_eq!(sys_bind(server as i32, addr.as_ptr(), addr_len), 0);

            sched::set_current(&mut *service as *mut TaskStruct);
            let client = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(client >= 0);
            let passed = crate::fs::file::alloc_file(
                crate::fs::dcache::d_alloc("udevd-inotify"),
                0,
                0,
                &crate::fs::ops::NOOP_FILE_OPS,
            );
            let source_fd = current_files()
                .unwrap()
                .install(passed.clone(), false)
                .unwrap();
            let mut send_control = [0u8; 32];
            let (send_control_len, truncated) =
                write_scm_rights(send_control.as_mut_ptr(), send_control.len(), &[source_fd])
                    .unwrap();
            assert!(!truncated);

            let payload = b"FDSTORE=1\nFDNAME=inotify\n";
            let mut send_iov = [LinuxIovec {
                base: payload.as_ptr() as *mut u8,
                len: payload.len(),
            }];
            let send_hdr = LinuxMsghdr {
                name: addr.as_ptr() as *mut u8,
                namelen: addr_len,
                iov: send_iov.as_mut_ptr(),
                iovlen: 1,
                control: send_control.as_mut_ptr(),
                controllen: send_control_len,
                flags: 0,
            };
            assert_eq!(
                sys_sendmsg(client as i32, &send_hdr, 0),
                payload.len() as i64
            );

            sched::set_current(&mut *manager as *mut TaskStruct);
            let mut body = [0u8; 64];
            let mut recv_iov = [LinuxIovec {
                base: body.as_mut_ptr(),
                len: body.len(),
            }];
            let mut control = [0u8; 64];
            let mut recv_hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: recv_iov.as_mut_ptr(),
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control.len(),
                flags: 0,
            };
            assert_eq!(
                sys_recvmsg(server as i32, &mut recv_hdr, MSG_CMSG_CLOEXEC),
                payload.len() as i64
            );
            assert_eq!(&body[..payload.len()], payload);
            assert_eq!(recv_hdr.flags & MSG_CTRUNC, 0);
            assert_eq!(
                recv_hdr.controllen,
                cmsg_align(CMSG_HDR_LEN + core::mem::size_of::<LinuxUcred>())
                    + cmsg_align(CMSG_HDR_LEN + core::mem::size_of::<i32>())
            );

            let cmsg_len = core::ptr::read_unaligned(control.as_ptr() as *const usize);
            let cmsg_level = core::ptr::read_unaligned(control.as_ptr().add(8) as *const i32);
            let cmsg_type = core::ptr::read_unaligned(control.as_ptr().add(12) as *const i32);
            let cred =
                core::ptr::read_unaligned(control.as_ptr().add(CMSG_HDR_LEN) as *const LinuxUcred);
            assert_eq!(cmsg_len, CMSG_HDR_LEN + core::mem::size_of::<LinuxUcred>());
            assert_eq!(cmsg_level, SOL_SOCKET);
            assert_eq!(cmsg_type, SCM_CREDENTIALS);
            assert_eq!(
                cred,
                LinuxUcred {
                    pid: 465,
                    uid: 0,
                    gid: 0
                }
            );
            let rights_off = cmsg_align(CMSG_HDR_LEN + core::mem::size_of::<LinuxUcred>());
            let rights_cmsg = control.as_ptr().add(rights_off);
            let rights_len = core::ptr::read_unaligned(rights_cmsg as *const usize);
            let rights_level = core::ptr::read_unaligned(rights_cmsg.add(8) as *const i32);
            let rights_type = core::ptr::read_unaligned(rights_cmsg.add(12) as *const i32);
            let received_fd =
                core::ptr::read_unaligned(rights_cmsg.add(CMSG_HDR_LEN) as *const i32);
            assert_eq!(rights_len, CMSG_HDR_LEN + core::mem::size_of::<i32>());
            assert_eq!(rights_level, SOL_SOCKET);
            assert_eq!(rights_type, SCM_RIGHTS);
            let manager_files = current_files().unwrap();
            assert!(alloc::sync::Arc::ptr_eq(
                &manager_files.get(received_fd).unwrap(),
                &passed
            ));

            files::drop_task_files(&mut *service as *mut TaskStruct);
            files::drop_task_files(&mut *manager as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(service.m26.thread_pid);
            service.m26.thread_pid = core::ptr::null_mut();
            put_pid(manager.m26.thread_pid);
            manager.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn unix_sendmsg_honors_explicit_scm_credentials_like_linux() {
        use crate::kernel::cred::{INIT_CRED, KGid, KUid, commit_creds, prepare_creds};

        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut manager = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        manager.pid = 1;
        manager.tgid = 1;
        manager.cred = &raw const INIT_CRED;
        let manager_kpid = alloc_pid(&INIT_PID_NS, Some(manager.pid)).expect("manager pid alloc");
        manager.m26.thread_pid = Box::into_raw(manager_kpid);

        let mut service = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        service.pid = 765;
        service.tgid = 765;
        service.cred = &raw const INIT_CRED;
        let service_kpid = alloc_pid(&INIT_PID_NS, Some(service.pid)).expect("service pid alloc");
        service.m26.thread_pid = Box::into_raw(service_kpid);

        unsafe {
            files::set_task_files(&mut *manager as *mut TaskStruct, FilesStruct::new());
            files::set_task_files(&mut *service as *mut TaskStruct, FilesStruct::new());

            sched::set_current(&mut *manager as *mut TaskStruct);
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            let server = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(server >= 0);
            let server = server as i32;
            let one = 1u32;
            assert_eq!(
                sys_setsockopt(
                    server,
                    SOL_SOCKET,
                    socket::SO_PASSCRED as i32,
                    &one as *const u32 as *const u8,
                    4,
                ),
                0
            );
            let (addr, addr_len) = unix_sockaddr("/run/scm-cred-override");
            assert_eq!(sys_bind(server, addr.as_ptr(), addr_len), 0);

            sched::set_current(&mut *service as *mut TaskStruct);
            let new = prepare_creds().expect("prepare creds");
            (*new).uid = KUid(1000);
            (*new).gid = KGid(1001);
            (*new).euid = KUid(0);
            (*new).egid = KGid(0);
            commit_creds(new);

            let client = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(client >= 0);
            let client = client as i32;
            let payload = b"READY=1\n";
            let mut send_iov = [LinuxIovec {
                base: payload.as_ptr() as *mut u8,
                len: payload.len(),
            }];
            let explicit = socket::SocketCred {
                pid: 765,
                uid: 0,
                gid: 0,
                groups: crate::kernel::cred::GroupInfo::default(),
                pid_ref: None,
            };
            let mut control = [0u8; 64];
            let (control_len, truncated) =
                write_scm_credentials_at(control.as_mut_ptr(), control.len(), 0, &explicit)
                    .expect("credentials cmsg");
            assert!(!truncated);
            let send_hdr = LinuxMsghdr {
                name: addr.as_ptr() as *mut u8,
                namelen: addr_len,
                iov: send_iov.as_mut_ptr(),
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control_len,
                flags: 0,
            };
            assert_eq!(sys_sendmsg(client, &send_hdr, 0), payload.len() as i64);

            sched::set_current(&mut *manager as *mut TaskStruct);
            let mut body = [0u8; 32];
            let mut recv_iov = [LinuxIovec {
                base: body.as_mut_ptr(),
                len: body.len(),
            }];
            let mut recv_control = [0u8; 64];
            let mut recv_hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: recv_iov.as_mut_ptr(),
                iovlen: 1,
                control: recv_control.as_mut_ptr(),
                controllen: recv_control.len(),
                flags: 0,
            };
            assert_eq!(sys_recvmsg(server, &mut recv_hdr, 0), payload.len() as i64);
            assert_eq!(&body[..payload.len()], payload);
            assert_eq!(recv_hdr.flags & MSG_CTRUNC, 0);

            let cmsg_len = core::ptr::read_unaligned(recv_control.as_ptr() as *const usize);
            let cmsg_level = core::ptr::read_unaligned(recv_control.as_ptr().add(8) as *const i32);
            let cmsg_type = core::ptr::read_unaligned(recv_control.as_ptr().add(12) as *const i32);
            let cred = core::ptr::read_unaligned(
                recv_control.as_ptr().add(CMSG_HDR_LEN) as *const LinuxUcred
            );
            assert_eq!(cmsg_len, CMSG_HDR_LEN + core::mem::size_of::<LinuxUcred>());
            assert_eq!(cmsg_level, SOL_SOCKET);
            assert_eq!(cmsg_type, SCM_CREDENTIALS);
            assert_eq!(
                cred,
                LinuxUcred {
                    pid: 765,
                    uid: 0,
                    gid: 0,
                }
            );

            files::drop_task_files(&mut *service as *mut TaskStruct);
            files::drop_task_files(&mut *manager as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(service.m26.thread_pid);
            service.m26.thread_pid = core::ptr::null_mut();
            put_pid(manager.m26.thread_pid);
            manager.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn unix_sendmsg_rejects_nonexistent_explicit_scm_pid_like_linux() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 901;
        current.tgid = 901;
        current.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(current.pid)).expect("pid alloc");
        current.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            let server = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(server >= 0);
            let (addr, addr_len) = unix_sockaddr("/run/scm-cred-esrch");
            assert_eq!(sys_bind(server as i32, addr.as_ptr(), addr_len), 0);

            let client = sys_socket(AF_UNIX as i32, socket::SOCK_DGRAM as i32, 0);
            assert!(client >= 0);
            let payload = b"x";
            let mut send_iov = [LinuxIovec {
                base: payload.as_ptr() as *mut u8,
                len: payload.len(),
            }];
            let explicit = socket::SocketCred {
                pid: 32_767,
                uid: 0,
                gid: 0,
                groups: crate::kernel::cred::GroupInfo::default(),
                pid_ref: None,
            };
            let mut control = [0u8; 64];
            let (control_len, truncated) =
                write_scm_credentials_at(control.as_mut_ptr(), control.len(), 0, &explicit)
                    .expect("credentials cmsg");
            assert!(!truncated);
            let send_hdr = LinuxMsghdr {
                name: addr.as_ptr() as *mut u8,
                namelen: addr_len,
                iov: send_iov.as_mut_ptr(),
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control_len,
                flags: 0,
            };
            assert_eq!(
                sys_sendmsg(client as i32, &send_hdr, 0),
                -(crate::include::uapi::errno::ESRCH as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(current.m26.thread_pid);
            current.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn unix_stream_eof_does_not_fabricate_scm_credentials() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 812;
        current.tgid = 812;
        current.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(current.pid)).expect("pid alloc");
        current.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut sv = [0i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_STREAM as i32,
                    0,
                    sv.as_mut_ptr()
                ),
                0
            );
            let one = 1u32;
            assert_eq!(
                sys_setsockopt(
                    sv[1],
                    SOL_SOCKET,
                    socket::SO_PASSCRED as i32,
                    &one as *const u32 as *const u8,
                    4,
                ),
                0
            );
            current_files().unwrap().close(sv[0]).expect("close writer");

            let mut body = [0u8; 1];
            let mut iov = [LinuxIovec {
                base: body.as_mut_ptr(),
                len: body.len(),
            }];
            let mut control = [0u8; 64];
            let mut hdr = LinuxMsghdr {
                name: core::ptr::null_mut(),
                namelen: 0,
                iov: iov.as_mut_ptr(),
                iovlen: 1,
                control: control.as_mut_ptr(),
                controllen: control.len(),
                flags: 0,
            };
            assert_eq!(sys_recvmsg(sv[1], &mut hdr, 0), 0);
            assert_eq!(hdr.controllen, 0);
            assert_eq!(hdr.flags & MSG_CTRUNC, 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(current.m26.thread_pid);
            current.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn unix_getsockopt_peerpidfd_installs_pidfd() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 425;
        current.tgid = 425;
        current.cred = &raw const INIT_CRED;
        // Test isolation: an earlier suite test may have leaked this pid's bitmap
        // bit (e.g. a pidfd reference outliving its task keeps the KPid refcount
        // above zero, so put_pid never reaches free_pid). Clear it first so the
        // fixed pid this test asserts on can be claimed. Guard with bit_is_set to
        // avoid an unbalanced pid_allocated decrement on an already-clear bit.
        if INIT_PID_NS.bit_is_set(current.pid) {
            crate::kernel::pid::free_pid(&INIT_PID_NS, current.pid);
        }
        let kpid = alloc_pid(&INIT_PID_NS, Some(current.pid)).expect("pid alloc");
        current.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut sv = [-1i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_STREAM as i32,
                    0,
                    sv.as_mut_ptr(),
                ),
                0
            );

            let mut pidfd = -1i32;
            let mut len = core::mem::size_of::<i32>() as u32;
            assert_eq!(
                sys_getsockopt(
                    sv[0],
                    SOL_SOCKET,
                    socket::SO_PEERPIDFD as i32,
                    &mut pidfd as *mut i32 as *mut u8,
                    &mut len,
                ),
                0
            );
            assert_eq!(len, core::mem::size_of::<i32>() as u32);
            assert_eq!(crate::fs::pidfd::pid_for_fd(pidfd), Ok(425));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(current.m26.thread_pid);
            current.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn unix_getsockopt_peersec_reports_unsupported_security_label() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 426;
        current.tgid = 426;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut sv = [-1i32; 2];
            assert_eq!(
                sys_socketpair(
                    AF_UNIX as i32,
                    socket::SOCK_STREAM as i32,
                    0,
                    sv.as_mut_ptr(),
                ),
                0
            );

            let mut label = [0u8; 1];
            let mut len = label.len() as u32;
            assert_eq!(
                sys_getsockopt(
                    sv[0],
                    SOL_SOCKET,
                    socket::SO_PEERSEC as i32,
                    label.as_mut_ptr(),
                    &mut len,
                ),
                -(ENOPROTOOPT as i64)
            );
            assert_eq!(len, 1);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_path_socket_unlink_allows_journald_rebind() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 281;
        current.tgid = 281;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run/systemd\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(
                    AT_FDCWD,
                    b"/run/systemd/journal\0".as_ptr(),
                    0o755
                ),
                0
            );

            let first = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(first >= 0);
            let path = "/run/systemd/journal/stdout-rebind";
            let (addr, addrlen) = unix_sockaddr(path);
            assert_eq!(sys_bind(first as i32, addr.as_ptr(), addrlen), 0);
            assert_eq!(sys_listen(first as i32, 4096), 0);

            let second = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(second >= 0);
            assert_eq!(
                sys_bind(second as i32, addr.as_ptr(), addrlen),
                -(EADDRINUSE as i64)
            );

            assert_eq!(
                crate::fs::syscalls::sys_unlink(b"/run/systemd/journal/stdout-rebind\0".as_ptr()),
                0
            );
            assert_eq!(sys_bind(second as i32, addr.as_ptr(), addrlen), 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_listener_bound_path_released_when_epoll_holds_file_reference() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 285;
        current.tgid = 285;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run/systemd\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(
                    AT_FDCWD,
                    b"/run/systemd/journal\0".as_ptr(),
                    0o755
                ),
                0
            );

            let first = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(first >= 0);
            let path = "/run/systemd/journal/epoll-release-rebind";
            let (addr, addrlen) = unix_sockaddr(path);
            assert_eq!(sys_bind(first as i32, addr.as_ptr(), addrlen), 0);
            assert_eq!(sys_listen(first as i32, 4096), 0);

            let epfd = crate::fs::eventpoll::sys_epoll_create1(0);
            assert!(epfd >= 0);
            let ev = crate::fs::eventpoll::EpollEvent {
                events: crate::fs::eventpoll::EPOLLIN,
                data: 0x285,
            };
            assert_eq!(
                crate::fs::eventpoll::sys_epoll_ctl(
                    epfd as i32,
                    crate::fs::eventpoll::EPOLL_CTL_ADD,
                    first as i32,
                    &ev,
                ),
                0
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());

            let second = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(second >= 0);
            assert_eq!(
                sys_bind(second as i32, addr.as_ptr(), addrlen),
                0,
                "epoll's watched file reference must fput the listener and clear BOUND"
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_path_bind_instantiates_negative_socket_dentry() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 284;
        current.tgid = 284;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(AT_FDCWD, b"/run/systemd\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                crate::fs::syscalls::sys_mkdirat(
                    AT_FDCWD,
                    b"/run/systemd/journal\0".as_ptr(),
                    0o755
                ),
                0
            );

            let (_, parent) = crate::fs::mount::resolve_path_follow("/run/systemd/journal")
                .expect("socket parent");
            let negative = crate::fs::dcache::d_cache_negative(&parent, "X0");
            assert!(negative.inode().is_none());

            let fd = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(fd >= 0);
            let (addr, addrlen) = unix_sockaddr("/run/systemd/journal/X0");
            assert_eq!(sys_bind(fd as i32, addr.as_ptr(), addrlen), 0);
            assert_eq!(
                negative.inode().expect("socket inode").kind,
                InodeKind::Socket
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_path_bind_rolls_back_when_socket_node_create_fails() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb.clone(), root.clone(), 0));

        let root_inode = root.inode().expect("root inode");
        let bad_dir = crate::fs::types::Inode::new(
            sb.alloc_ino(),
            InodeKind::Directory,
            0o755,
            &crate::fs::ops::NOOP_INODE_OPS,
            &crate::fs::ops::NOOP_FILE_OPS,
            InodePrivate::None,
        );
        *bad_dir.sb.lock() = Some(sb);
        let InodePrivate::RamDir(children) = &root_inode.private else {
            panic!("ramfs root must have RamDir children");
        };
        children.lock().insert(String::from("bad"), bad_dir);

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 283;
        current.tgid = 283;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let path = "/bad/X0";
            let (addr, addrlen) = unix_sockaddr(path);
            let first = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(first >= 0);
            assert_eq!(
                sys_bind(first as i32, addr.as_ptr(), addrlen),
                -(ENOSYS as i64)
            );

            let second = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(second >= 0);
            assert_eq!(
                sys_bind(second as i32, addr.as_ptr(), addrlen),
                -(ENOSYS as i64),
                "failed filesystem node creation must not leave the address internally busy"
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_abstract_sockaddr_preserves_name_after_leading_nul() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 282;
        current.tgid = 282;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let abstract_addr = |name: &[u8]| {
                let mut raw = [0u8; 128];
                raw[..2].copy_from_slice(&AF_UNIX.to_ne_bytes());
                raw[2..2 + name.len()].copy_from_slice(name);
                (raw, (2 + name.len()) as u32)
            };

            let first = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            let second = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            let duplicate = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(first >= 0 && second >= 0 && duplicate >= 0);

            let (one, one_len) = abstract_addr(b"\0systemctl-a");
            let (two, two_len) = abstract_addr(b"\0systemctl-b");
            assert_eq!(sys_bind(first as i32, one.as_ptr(), one_len), 0);
            assert_eq!(sys_bind(second as i32, two.as_ptr(), two_len), 0);
            assert_eq!(
                sys_bind(duplicate as i32, one.as_ptr(), one_len),
                -(EADDRINUSE as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unix_connect_via_proc_fd_opath_uses_bound_path() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        *crate::fs::mount::MOUNTS.root.lock() = None;
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = crate::fs::super_block::mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        crate::fs::mount::set_rootfs(crate::fs::mount::Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 279;
        current.tgid = 279;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let server = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(server >= 0);
            let (addr, addrlen) = unix_sockaddr("/sock-fd-test");
            assert_eq!(sys_bind(server as i32, addr.as_ptr(), addrlen), 0);
            assert_eq!(sys_listen(server as i32, 4), 0);

            let socket_path = b"/sock-fd-test\0";
            let opath =
                crate::fs::openat::sys_openat(AT_FDCWD, socket_path.as_ptr(), O_PATH as i32, 0);
            assert!(opath >= 0);

            let proc_path = format!("/proc/self/fd/{}", opath);
            let (proc_addr, proc_addrlen) = unix_sockaddr(&proc_path);
            let client = sys_socket(AF_UNIX as i32, socket::SOCK_STREAM as i32, 0);
            assert!(client >= 0);
            assert_eq!(
                sys_connect(client as i32, proc_addr.as_ptr(), proc_addrlen),
                0
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
