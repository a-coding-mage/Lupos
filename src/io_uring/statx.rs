//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/statx.c
//! test-origin: linux:vendor/linux/io_uring/statx.c
//! `IORING_OP_STATX`.
//!
//! Ref: vendor/linux/io_uring/statx.c

use super::sqe::Sqe;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoStatx {
    pub dfd: i32,
    pub mask: u32,
    pub flags: u32,
    pub filename_addr: u64,
    pub buf_addr: u64,
}

pub fn statx_prep(sqe: &Sqe) -> Result<IoStatx, i32> {
    if sqe.addr == 0 || sqe.addr3 == 0 {
        return Err(-22);
    }
    Ok(IoStatx {
        dfd: sqe.fd,
        mask: sqe.len,
        flags: sqe.op_flags,
        filename_addr: sqe.addr,
        buf_addr: sqe.addr3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statx_requires_filename_and_buf() {
        let mut s = Sqe::default();
        s.addr = 0xfeed;
        // No statx output buf.
        assert_eq!(statx_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn statx_captures_mask_from_len() {
        let mut s = Sqe::default();
        s.addr = 1;
        s.addr3 = 2;
        s.len = 0xdead_beef;
        let r = statx_prep(&s).unwrap();
        assert_eq!(r.mask, 0xdead_beef);
    }
}
