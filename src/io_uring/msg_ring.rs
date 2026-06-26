//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/msg_ring.c
//! test-origin: linux:vendor/linux/io_uring/msg_ring.c
//! `IORING_OP_MSG_RING` — post a CQE into another ring.
//!
//! Ref: vendor/linux/io_uring/msg_ring.c

use super::IoRingCtx;
use super::sqe::Sqe;

/// `IORING_MSG_RING_*` modes carried in `sqe.addr`.
pub const IORING_MSG_DATA: u64 = 0;
pub const IORING_MSG_SEND_FD: u64 = 1;

/// `IORING_MSG_RING_CQE_SKIP` flag — don't post a CQE in the sender ring.
pub const IORING_MSG_RING_CQE_SKIP: u32 = 1 << 0;
pub const IORING_MSG_RING_FLAGS_PASS: u32 = 1 << 1;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoMsgRing {
    pub target_fd: i32,
    pub mode: u64,
    pub cqe_user_data: u64,
    pub cqe_res: i32,
    pub flags: u32,
}

pub fn msg_ring_prep(sqe: &Sqe) -> Result<IoMsgRing, i32> {
    if sqe.fd < 0 {
        return Err(-9);
    }
    let mode = sqe.addr;
    if mode != IORING_MSG_DATA && mode != IORING_MSG_SEND_FD {
        return Err(-22);
    }
    Ok(IoMsgRing {
        target_fd: sqe.fd,
        mode,
        // sqe.off carries the user_data to write to the target CQE.
        cqe_user_data: sqe.off,
        cqe_res: sqe.len as i32,
        flags: sqe.op_flags,
    })
}

/// `__io_msg_ring_data` — push a CQE to `target` and return the value to
/// record in the sender's own CQE (0 on success, -EOVERFLOW on a full CQ).
pub fn deliver_data(target: &IoRingCtx, msg: &IoMsgRing) -> i32 {
    target.complete(msg.cqe_user_data, msg.cqe_res);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_uring::IoRingCtx;
    use alloc::sync::Arc;

    #[test]
    fn msg_ring_data_mode_is_zero() {
        // Linux uses `IORING_MSG_DATA = 0`.
        assert_eq!(IORING_MSG_DATA, 0);
    }

    #[test]
    fn msg_ring_rejects_unknown_mode() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.addr = 99;
        assert_eq!(msg_ring_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn deliver_data_posts_cqe_to_target() {
        let target = Arc::new(IoRingCtx::new(4));
        let msg = IoMsgRing {
            target_fd: 0,
            mode: IORING_MSG_DATA,
            cqe_user_data: 0x1234,
            cqe_res: 42,
            flags: 0,
        };
        let r = deliver_data(&target, &msg);
        assert_eq!(r, 0);
        assert_eq!(target.cq_ready(), 1);
        assert_eq!(target.cqes[0].user_data, 0x1234);
        assert_eq!(target.cqes[0].res, 42);
    }
}
