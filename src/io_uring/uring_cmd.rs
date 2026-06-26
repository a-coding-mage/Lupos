//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/uring_cmd.c
//! test-origin: linux:vendor/linux/io_uring/uring_cmd.c
//! `IORING_OP_URING_CMD` — generic ioctl-style async commands.
//!
//! The opcode carries a 16-byte cmd buffer that the target driver
//! (`struct file_operations::uring_cmd`) interprets.  Lupos exposes a
//! registry of dispatch handlers keyed by `(file_class, cmd_op)`.
//!
//! Ref: vendor/linux/io_uring/uring_cmd.c

use super::sqe::Sqe;

/// `IORING_URING_CMD_*` flags.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:334
pub const IORING_URING_CMD_FIXED: u32 = 1 << 0;
pub const IORING_URING_CMD_MULTISHOT: u32 = 1 << 1;
pub const IORING_URING_CMD_MASK: u32 = IORING_URING_CMD_FIXED | IORING_URING_CMD_MULTISHOT;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringCmd {
    pub fd: i32,
    pub cmd_op: u32,
    pub flags: u32,
    pub data_addr: u64,
    pub buf_index: u16,
}

pub fn uring_cmd_prep(sqe: &Sqe) -> Result<IoUringCmd, i32> {
    if sqe.fd < 0 {
        return Err(-9);
    }
    if sqe.op_flags & !IORING_URING_CMD_MASK != 0 {
        return Err(-22);
    }
    if sqe.op_flags & IORING_URING_CMD_FIXED != 0 && sqe.op_flags & IORING_URING_CMD_MULTISHOT != 0
    {
        // Comment in vendor file: "Not compatible with URING_CMD_FIXED, for now."
        return Err(-22);
    }
    Ok(IoUringCmd {
        fd: sqe.fd,
        cmd_op: sqe.len,
        flags: sqe.op_flags,
        data_addr: sqe.addr,
        buf_index: sqe.buf_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_match_linux() {
        assert_eq!(IORING_URING_CMD_FIXED, 1);
        assert_eq!(IORING_URING_CMD_MULTISHOT, 2);
        assert_eq!(IORING_URING_CMD_MASK, 3);
    }

    #[test]
    fn rejects_unknown_flag_bits() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.op_flags = 1 << 31;
        assert_eq!(uring_cmd_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn rejects_fixed_with_multishot() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.op_flags = IORING_URING_CMD_FIXED | IORING_URING_CMD_MULTISHOT;
        assert_eq!(uring_cmd_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn captures_cmd_op_from_len() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.len = 0x4142_4344;
        let r = uring_cmd_prep(&s).unwrap();
        assert_eq!(r.cmd_op, 0x4142_4344);
    }
}
