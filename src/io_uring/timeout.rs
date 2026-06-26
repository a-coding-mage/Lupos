//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/timeout.c
//! test-origin: linux:vendor/linux/io_uring/timeout.c
//! `IORING_OP_TIMEOUT` / `TIMEOUT_REMOVE` / `LINK_TIMEOUT`.
//!
//! Ref: vendor/linux/io_uring/timeout.c

use super::sqe::Sqe;

/// `IORING_TIMEOUT_*` flags.  Ref: vendor/linux/include/uapi/linux/io_uring.h:351-358
pub const IORING_TIMEOUT_ABS: u32 = 1 << 0;
pub const IORING_TIMEOUT_UPDATE: u32 = 1 << 1;
pub const IORING_TIMEOUT_BOOTTIME: u32 = 1 << 2;
pub const IORING_TIMEOUT_REALTIME: u32 = 1 << 3;
pub const IORING_LINK_TIMEOUT_UPDATE: u32 = 1 << 4;
pub const IORING_TIMEOUT_ETIME_SUCCESS: u32 = 1 << 5;
pub const IORING_TIMEOUT_MULTISHOT: u32 = 1 << 6;
pub const IORING_TIMEOUT_IMMEDIATE_ARG: u32 = 1 << 7;

const TIMEOUT_CLOCK_MASK: u32 = IORING_TIMEOUT_BOOTTIME | IORING_TIMEOUT_REALTIME;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoTimeout {
    pub ts_addr: u64,
    pub count: u32,
    pub flags: u32,
}

pub fn timeout_prep(sqe: &Sqe) -> Result<IoTimeout, i32> {
    // Linux: BOOTTIME and REALTIME are mutually exclusive.
    if (sqe.op_flags & TIMEOUT_CLOCK_MASK) == TIMEOUT_CLOCK_MASK {
        return Err(-22);
    }
    if sqe.addr == 0 && sqe.op_flags & IORING_TIMEOUT_IMMEDIATE_ARG == 0 {
        return Err(-22);
    }
    Ok(IoTimeout {
        ts_addr: sqe.addr,
        count: sqe.len,
        flags: sqe.op_flags,
    })
}

pub fn timeout_remove_prep(sqe: &Sqe) -> Result<u64, i32> {
    // user_data of the timeout to cancel.
    Ok(sqe.addr)
}

pub fn link_timeout_prep(sqe: &Sqe) -> Result<IoTimeout, i32> {
    if sqe.addr == 0 {
        return Err(-22);
    }
    Ok(IoTimeout {
        ts_addr: sqe.addr,
        count: 0,
        flags: sqe.op_flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_flags_match_linux() {
        assert_eq!(IORING_TIMEOUT_ABS, 1);
        assert_eq!(IORING_TIMEOUT_MULTISHOT, 0x40);
    }

    #[test]
    fn timeout_rejects_both_clock_flags() {
        let mut s = Sqe::default();
        s.op_flags = IORING_TIMEOUT_BOOTTIME | IORING_TIMEOUT_REALTIME;
        s.addr = 0xface;
        assert_eq!(timeout_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn timeout_requires_ts_or_immediate_flag() {
        let s = Sqe::default();
        assert_eq!(timeout_prep(&s).unwrap_err(), -22);

        let mut s2 = Sqe::default();
        s2.op_flags = IORING_TIMEOUT_IMMEDIATE_ARG;
        assert!(timeout_prep(&s2).is_ok());
    }

    #[test]
    fn timeout_remove_extracts_user_data() {
        let mut s = Sqe::default();
        s.addr = 0x1234;
        assert_eq!(timeout_remove_prep(&s).unwrap(), 0x1234);
    }
}
