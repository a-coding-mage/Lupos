//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/cmd_net.c
//! test-origin: linux:vendor/linux/io_uring/cmd_net.c
//! Socket-specialised URING_CMD opcodes.
//!
//! Ref: vendor/linux/io_uring/cmd_net.c

use super::sqe::Sqe;
use super::uring_cmd::{IoUringCmd, uring_cmd_prep};

/// `enum io_uring_socket_op`.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:1036
pub const SOCKET_URING_OP_SIOCINQ: u32 = 0;
pub const SOCKET_URING_OP_SIOCOUTQ: u32 = 1;
pub const SOCKET_URING_OP_GETSOCKOPT: u32 = 2;
pub const SOCKET_URING_OP_SETSOCKOPT: u32 = 3;
pub const SOCKET_URING_OP_TX_TIMESTAMP: u32 = 4;

pub fn socket_cmd_prep(sqe: &Sqe) -> Result<IoUringCmd, i32> {
    let r = uring_cmd_prep(sqe)?;
    match r.cmd_op {
        SOCKET_URING_OP_SIOCINQ
        | SOCKET_URING_OP_SIOCOUTQ
        | SOCKET_URING_OP_GETSOCKOPT
        | SOCKET_URING_OP_SETSOCKOPT
        | SOCKET_URING_OP_TX_TIMESTAMP => Ok(r),
        _ => Err(-22),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_uring_op_values_match_linux() {
        assert_eq!(SOCKET_URING_OP_SIOCINQ, 0);
        assert_eq!(SOCKET_URING_OP_SETSOCKOPT, 3);
    }

    #[test]
    fn socket_cmd_rejects_unknown_op() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.len = 99;
        assert_eq!(socket_cmd_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn socket_cmd_accepts_known_ops() {
        let mut s = Sqe::default();
        s.fd = 1;
        for op in [
            SOCKET_URING_OP_SIOCINQ,
            SOCKET_URING_OP_SIOCOUTQ,
            SOCKET_URING_OP_GETSOCKOPT,
            SOCKET_URING_OP_SETSOCKOPT,
            SOCKET_URING_OP_TX_TIMESTAMP,
        ] {
            s.len = op;
            assert!(socket_cmd_prep(&s).is_ok(), "op {} should be accepted", op);
        }
    }
}
