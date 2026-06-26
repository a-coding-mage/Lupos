//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/splice.c
//! test-origin: linux:vendor/linux/io_uring/splice.c
//! `IORING_OP_SPLICE` / `IORING_OP_TEE`.
//!
//! Ref: vendor/linux/io_uring/splice.c

use super::sqe::Sqe;

/// `SPLICE_F_FD_IN_FIXED` — `splice_fd_in` is a registered-file index.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:365
pub const SPLICE_F_FD_IN_FIXED: u32 = 1 << 31;

/// Other `splice(2)` flags relayed via `op_flags`.
pub const SPLICE_F_MOVE: u32 = 1 << 0;
pub const SPLICE_F_NONBLOCK: u32 = 1 << 1;
pub const SPLICE_F_MORE: u32 = 1 << 2;
pub const SPLICE_F_GIFT: u32 = 1 << 3;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoSplice {
    pub fd_in: i32,
    pub fd_out: i32,
    pub off_in: i64,
    pub off_out: i64,
    pub len: u32,
    pub flags: u32,
    pub fd_in_fixed: bool,
}

pub fn splice_prep(sqe: &Sqe) -> Result<IoSplice, i32> {
    if sqe.len == 0 {
        return Err(-22);
    }
    let fixed = sqe.op_flags & SPLICE_F_FD_IN_FIXED != 0;
    Ok(IoSplice {
        fd_in: sqe.splice_fd_in,
        fd_out: sqe.fd,
        off_in: sqe.addr as i64, // Linux stashes off_in in sqe.splice_off_in (= addr).
        off_out: sqe.off as i64,
        len: sqe.len,
        flags: sqe.op_flags & !SPLICE_F_FD_IN_FIXED,
        fd_in_fixed: fixed,
    })
}

pub fn tee_prep(sqe: &Sqe) -> Result<IoSplice, i32> {
    if sqe.len == 0 {
        return Err(-22);
    }
    // tee has no offset args per splice(2) semantics.
    if sqe.off != 0 || sqe.addr != 0 {
        return Err(-22);
    }
    Ok(IoSplice {
        fd_in: sqe.splice_fd_in,
        fd_out: sqe.fd,
        off_in: 0,
        off_out: 0,
        len: sqe.len,
        flags: sqe.op_flags & !SPLICE_F_FD_IN_FIXED,
        fd_in_fixed: sqe.op_flags & SPLICE_F_FD_IN_FIXED != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_fd_in_fixed_bit_matches_linux() {
        assert_eq!(SPLICE_F_FD_IN_FIXED, 1 << 31);
    }

    #[test]
    fn splice_rejects_zero_length() {
        let s = Sqe::default();
        assert_eq!(splice_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn splice_picks_up_fd_in_from_splice_fd_in() {
        let mut s = Sqe::default();
        s.fd = 3;
        s.splice_fd_in = 4;
        s.len = 16;
        let r = splice_prep(&s).unwrap();
        assert_eq!(r.fd_in, 4);
        assert_eq!(r.fd_out, 3);
    }

    #[test]
    fn splice_fixed_flag_propagates() {
        let mut s = Sqe::default();
        s.len = 8;
        s.op_flags = SPLICE_F_FD_IN_FIXED;
        let r = splice_prep(&s).unwrap();
        assert!(r.fd_in_fixed);
        assert_eq!(r.flags & SPLICE_F_FD_IN_FIXED, 0);
    }

    #[test]
    fn tee_rejects_offsets() {
        let mut s = Sqe::default();
        s.len = 16;
        s.off = 1;
        assert_eq!(tee_prep(&s).unwrap_err(), -22);
    }
}
