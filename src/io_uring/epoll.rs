//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/epoll.c
//! test-origin: linux:vendor/linux/io_uring/epoll.c
//! `IORING_OP_EPOLL_CTL` / `IORING_OP_EPOLL_WAIT`.
//!
//! Ref: vendor/linux/io_uring/epoll.c

use super::sqe::Sqe;

/// `EPOLL_CTL_*` opcodes (UAPI).
pub const EPOLL_CTL_ADD: u32 = 1;
pub const EPOLL_CTL_DEL: u32 = 2;
pub const EPOLL_CTL_MOD: u32 = 3;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoEpollCtl {
    pub epfd: i32,
    pub op: u32,
    pub fd: i32,
    pub event_addr: u64,
}

pub fn epoll_ctl_prep(sqe: &Sqe) -> Result<IoEpollCtl, i32> {
    if sqe.fd < 0 {
        return Err(-9);
    }
    let op = sqe.len;
    if op == 0 || op > EPOLL_CTL_MOD {
        return Err(-22);
    }
    if op != EPOLL_CTL_DEL && sqe.addr == 0 {
        return Err(-22);
    }
    Ok(IoEpollCtl {
        epfd: sqe.fd,
        op,
        fd: sqe.off as i32,
        event_addr: sqe.addr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoll_ctl_op_constants() {
        assert_eq!(EPOLL_CTL_ADD, 1);
        assert_eq!(EPOLL_CTL_DEL, 2);
        assert_eq!(EPOLL_CTL_MOD, 3);
    }

    #[test]
    fn epoll_ctl_rejects_bad_op() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.len = 99;
        assert_eq!(epoll_ctl_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn epoll_ctl_add_requires_event_ptr() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.len = EPOLL_CTL_ADD;
        assert_eq!(epoll_ctl_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn epoll_ctl_del_allows_null_event() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.len = EPOLL_CTL_DEL;
        assert!(epoll_ctl_prep(&s).is_ok());
    }
}
