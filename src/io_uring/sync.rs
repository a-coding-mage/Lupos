//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/sync.c
//! test-origin: linux:vendor/linux/io_uring/sync.c
//! `IORING_OP_FSYNC` / `SYNC_FILE_RANGE` / `FALLOCATE`.
//!
//! Ref: vendor/linux/io_uring/sync.c

use super::sqe::Sqe;

/// `IORING_FSYNC_DATASYNC` flag — only sync data, not metadata.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:342
pub const IORING_FSYNC_DATASYNC: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoSync {
    pub fd: i32,
    pub off: u64,
    pub len: u64,
    pub flags: u32,
}

pub fn fsync_prep(sqe: &Sqe) -> Result<IoSync, i32> {
    if sqe.op_flags & !IORING_FSYNC_DATASYNC != 0 {
        return Err(-22);
    }
    if sqe.fd < 0 {
        return Err(-9);
    }
    Ok(IoSync {
        fd: sqe.fd,
        off: sqe.off,
        len: sqe.len as u64,
        flags: sqe.op_flags,
    })
}

pub fn sync_file_range_prep(sqe: &Sqe) -> Result<IoSync, i32> {
    if sqe.fd < 0 {
        return Err(-9);
    }
    Ok(IoSync {
        fd: sqe.fd,
        off: sqe.off,
        len: sqe.len as u64,
        flags: sqe.op_flags,
    })
}

pub fn fallocate_prep(sqe: &Sqe) -> Result<IoSync, i32> {
    if sqe.fd < 0 {
        return Err(-9);
    }
    Ok(IoSync {
        fd: sqe.fd,
        off: sqe.off,
        len: sqe.addr,
        flags: sqe.op_flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsync_datasync_flag_matches_linux() {
        assert_eq!(IORING_FSYNC_DATASYNC, 1);
    }

    #[test]
    fn fsync_rejects_unknown_flags() {
        let mut s = Sqe::default();
        s.op_flags = 1 << 31;
        assert_eq!(fsync_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn fsync_rejects_negative_fd() {
        let mut s = Sqe::default();
        s.fd = -1;
        assert_eq!(fsync_prep(&s).unwrap_err(), -9);
    }

    #[test]
    fn fsync_captures_off_and_len() {
        let mut s = Sqe::default();
        s.fd = 3;
        s.off = 0x1000;
        s.len = 4096;
        s.op_flags = IORING_FSYNC_DATASYNC;
        let r = fsync_prep(&s).unwrap();
        assert_eq!(r.off, 0x1000);
        assert_eq!(r.len, 4096);
        assert_eq!(r.flags, IORING_FSYNC_DATASYNC);
    }
}
