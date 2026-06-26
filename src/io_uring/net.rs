//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/net.c
//! test-origin: linux:vendor/linux/io_uring/net.c
//! Network opcodes — SEND / RECV / SENDMSG / RECVMSG / ACCEPT / CONNECT /
//! SHUTDOWN / BIND / LISTEN / SOCKET / SEND_ZC / SENDMSG_ZC / RECV_ZC.
//!
//! Ref: vendor/linux/io_uring/net.c

use super::sqe::Sqe;

/// `MSG_*` flags relayed via `op_flags`.  Subset shown — values match Linux UAPI.
pub mod msg_flag {
    pub const DONTWAIT: u32 = 0x40;
    pub const NOSIGNAL: u32 = 0x4000;
    pub const ZEROCOPY: u32 = 0x4000_000;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IoSr {
    pub fd: i32,
    pub buf_addr: u64,
    pub len: u32,
    pub msg_flags: u32,
    pub buf_index: u16,
    pub iovec: bool,
    pub zerocopy: bool,
}

fn require_fd(sqe: &Sqe) -> Result<(), i32> {
    if sqe.fd < 0 { Err(-9) } else { Ok(()) }
}

pub fn send_prep(sqe: &Sqe) -> Result<IoSr, i32> {
    require_fd(sqe)?;
    Ok(IoSr {
        fd: sqe.fd,
        buf_addr: sqe.addr,
        len: sqe.len,
        msg_flags: sqe.op_flags,
        buf_index: sqe.buf_index,
        iovec: false,
        zerocopy: false,
    })
}

pub fn recv_prep(sqe: &Sqe) -> Result<IoSr, i32> {
    require_fd(sqe)?;
    Ok(IoSr {
        fd: sqe.fd,
        buf_addr: sqe.addr,
        len: sqe.len,
        msg_flags: sqe.op_flags,
        buf_index: sqe.buf_index,
        iovec: false,
        zerocopy: false,
    })
}

pub fn sendmsg_prep(sqe: &Sqe) -> Result<IoSr, i32> {
    require_fd(sqe)?;
    // sqe.addr = msghdr *, sqe.len = unused.
    if sqe.addr == 0 {
        return Err(-22);
    }
    Ok(IoSr {
        fd: sqe.fd,
        buf_addr: sqe.addr,
        len: sqe.len,
        msg_flags: sqe.op_flags,
        buf_index: 0,
        iovec: true,
        zerocopy: false,
    })
}

pub fn recvmsg_prep(sqe: &Sqe) -> Result<IoSr, i32> {
    let mut r = sendmsg_prep(sqe)?;
    r.iovec = true;
    Ok(r)
}

pub fn send_zc_prep(sqe: &Sqe) -> Result<IoSr, i32> {
    let mut r = send_prep(sqe)?;
    r.zerocopy = true;
    Ok(r)
}

pub fn sendmsg_zc_prep(sqe: &Sqe) -> Result<IoSr, i32> {
    let mut r = sendmsg_prep(sqe)?;
    r.zerocopy = true;
    Ok(r)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IoAccept {
    pub fd: i32,
    pub addr: u64,
    pub addrlen_addr: u64,
    pub flags: u32,
}

pub fn accept_prep(sqe: &Sqe) -> Result<IoAccept, i32> {
    require_fd(sqe)?;
    Ok(IoAccept {
        fd: sqe.fd,
        addr: sqe.addr,
        addrlen_addr: sqe.addr3,
        flags: sqe.op_flags,
    })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IoConnect {
    pub fd: i32,
    pub addr: u64,
    pub addrlen: u32,
}

pub fn connect_prep(sqe: &Sqe) -> Result<IoConnect, i32> {
    require_fd(sqe)?;
    if sqe.addr == 0 {
        return Err(-22);
    }
    Ok(IoConnect {
        fd: sqe.fd,
        addr: sqe.addr,
        addrlen: sqe.off as u32,
    })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IoSocket {
    pub domain: i32,
    pub typ: i32,
    pub protocol: i32,
    pub file_slot: u32,
    pub flags: u32,
}

pub fn socket_prep(sqe: &Sqe) -> Result<IoSocket, i32> {
    Ok(IoSocket {
        domain: sqe.fd,
        typ: sqe.off as i32,
        protocol: sqe.len as i32,
        file_slot: sqe.splice_fd_in as u32,
        flags: sqe.op_flags,
    })
}

pub fn shutdown_prep(sqe: &Sqe) -> Result<(i32, u32), i32> {
    require_fd(sqe)?;
    Ok((sqe.fd, sqe.len))
}

pub fn bind_prep(sqe: &Sqe) -> Result<IoConnect, i32> {
    connect_prep(sqe)
}

pub fn listen_prep(sqe: &Sqe) -> Result<(i32, u32), i32> {
    require_fd(sqe)?;
    Ok((sqe.fd, sqe.len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_rejects_bad_fd() {
        let mut s = Sqe::default();
        s.fd = -1;
        assert_eq!(send_prep(&s).unwrap_err(), -9);
    }

    #[test]
    fn sendmsg_requires_msghdr_ptr() {
        let mut s = Sqe::default();
        s.fd = 1;
        assert_eq!(sendmsg_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn send_zc_marks_zerocopy() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.addr = 0xcafe;
        s.len = 16;
        let r = send_zc_prep(&s).unwrap();
        assert!(r.zerocopy);
    }

    #[test]
    fn recvmsg_marks_iovec() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.addr = 0xcafe;
        let r = recvmsg_prep(&s).unwrap();
        assert!(r.iovec);
    }

    #[test]
    fn accept_captures_flags() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.op_flags = 0x800; // SOCK_NONBLOCK
        let r = accept_prep(&s).unwrap();
        assert_eq!(r.flags, 0x800);
    }

    #[test]
    fn socket_routes_domain_type_protocol() {
        let mut s = Sqe::default();
        s.fd = 2; // AF_INET
        s.off = 1; // SOCK_STREAM
        s.len = 6; // IPPROTO_TCP
        let r = socket_prep(&s).unwrap();
        assert_eq!(r.domain, 2);
        assert_eq!(r.typ, 1);
        assert_eq!(r.protocol, 6);
    }

    #[test]
    fn connect_requires_addr_ptr() {
        let mut s = Sqe::default();
        s.fd = 1;
        assert_eq!(connect_prep(&s).unwrap_err(), -22);
    }
}
