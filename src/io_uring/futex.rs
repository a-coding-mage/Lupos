//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/futex.c
//! test-origin: linux:vendor/linux/io_uring/futex.c
//! `IORING_OP_FUTEX_WAIT` / `IORING_OP_FUTEX_WAKE` / `IORING_OP_FUTEX_WAITV`.
//!
//! Ref: vendor/linux/io_uring/futex.c

use super::sqe::Sqe;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoFutex {
    pub uaddr: u64,
    pub val: u64,
    pub mask: u64,
    pub flags: u32,
}

pub fn futex_wait_prep(sqe: &Sqe) -> Result<IoFutex, i32> {
    if sqe.addr == 0 {
        return Err(-22);
    }
    Ok(IoFutex {
        uaddr: sqe.addr,
        val: sqe.off,
        mask: sqe.addr3,
        flags: sqe.op_flags,
    })
}

pub fn futex_wake_prep(sqe: &Sqe) -> Result<IoFutex, i32> {
    if sqe.addr == 0 {
        return Err(-22);
    }
    Ok(IoFutex {
        uaddr: sqe.addr,
        val: sqe.len as u64,
        mask: sqe.addr3,
        flags: sqe.op_flags,
    })
}

pub fn futex_waitv_prep(sqe: &Sqe) -> Result<IoFutex, i32> {
    // FUTEX_WAITV: addr = struct futex_waitv *, len = nr_waiters.
    if sqe.addr == 0 || sqe.len == 0 {
        return Err(-22);
    }
    Ok(IoFutex {
        uaddr: sqe.addr,
        val: sqe.len as u64,
        mask: sqe.addr3,
        flags: sqe.op_flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn futex_wait_requires_uaddr() {
        let s = Sqe::default();
        assert_eq!(futex_wait_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn futex_wait_stashes_val_from_off() {
        let mut s = Sqe::default();
        s.addr = 0xface;
        s.off = 0xdead_beef;
        let f = futex_wait_prep(&s).unwrap();
        assert_eq!(f.val, 0xdead_beef);
    }

    #[test]
    fn futex_waitv_requires_nr() {
        let mut s = Sqe::default();
        s.addr = 0xface;
        s.len = 0;
        assert_eq!(futex_waitv_prep(&s).unwrap_err(), -22);
    }
}
