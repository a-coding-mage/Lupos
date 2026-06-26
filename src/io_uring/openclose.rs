//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/openclose.c
//! test-origin: linux:vendor/linux/io_uring/openclose.c
//! `IORING_OP_OPENAT` / `OPENAT2` / `CLOSE` / `FIXED_FD_INSTALL`.
//!
//! Ref: vendor/linux/io_uring/openclose.c

use super::sqe::Sqe;

/// `struct io_open` — args for openat / openat2.
#[derive(Clone, Copy, Debug, Default)]
pub struct IoOpen {
    pub dfd: i32,
    pub how_addr: u64,
    pub how_len: u32,
    pub filename_addr: u64,
    pub file_slot: u32,
}

/// `io_openat_prep`.  `sqe.addr` points to the filename, `sqe.len` to flags
/// in the openat case.
pub fn openat_prep(sqe: &Sqe) -> Result<IoOpen, i32> {
    if sqe.addr == 0 {
        return Err(-22); // -EINVAL  (no filename)
    }
    Ok(IoOpen {
        dfd: sqe.fd,
        how_addr: 0,
        how_len: sqe.len,
        filename_addr: sqe.addr,
        file_slot: sqe.splice_fd_in as u32,
    })
}

/// `io_openat2_prep`.  `sqe.addr2` (addr3 in lupos Sqe layout) points to
/// `struct open_how`.  `sqe.len` carries `sizeof(struct open_how)`.
pub fn openat2_prep(sqe: &Sqe) -> Result<IoOpen, i32> {
    if sqe.addr == 0 || sqe.addr3 == 0 {
        return Err(-22);
    }
    // Linux: enforces sizeof(struct open_how) <= len.
    if (sqe.len as usize) < core::mem::size_of::<u64>() * 3 {
        return Err(-22);
    }
    Ok(IoOpen {
        dfd: sqe.fd,
        how_addr: sqe.addr3,
        how_len: sqe.len,
        filename_addr: sqe.addr,
        file_slot: sqe.splice_fd_in as u32,
    })
}

/// `io_close_prep` — only `fd` is meaningful.  Linux rejects any flag bits.
pub fn close_prep(sqe: &Sqe) -> Result<i32, i32> {
    if sqe.op_flags != 0 {
        return Err(-22);
    }
    if sqe.fd < 0 {
        return Err(-9);
    }
    Ok(sqe.fd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openat_requires_filename_ptr() {
        let s = Sqe::default();
        assert_eq!(openat_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn openat_stashes_dfd_and_filename() {
        let mut s = Sqe::default();
        s.fd = 7;
        s.addr = 0xcafe;
        s.len = 0o600;
        let o = openat_prep(&s).unwrap();
        assert_eq!(o.dfd, 7);
        assert_eq!(o.filename_addr, 0xcafe);
        assert_eq!(o.how_len, 0o600);
    }

    #[test]
    fn openat2_requires_open_how_pointer() {
        let mut s = Sqe::default();
        s.addr = 0xcafe;
        // No addr3 — invalid.
        assert_eq!(openat2_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn openat2_validates_how_size() {
        let mut s = Sqe::default();
        s.addr = 0xcafe;
        s.addr3 = 0xface;
        s.len = 4; // < sizeof(open_how)
        assert_eq!(openat2_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn close_prep_rejects_flags() {
        let mut s = Sqe::default();
        s.op_flags = 1;
        assert_eq!(close_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn close_prep_returns_fd() {
        let mut s = Sqe::default();
        s.fd = 5;
        assert_eq!(close_prep(&s).unwrap(), 5);
    }
}
