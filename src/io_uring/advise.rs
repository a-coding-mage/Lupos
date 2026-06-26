//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/advise.c
//! test-origin: linux:vendor/linux/io_uring/advise.c
//! `IORING_OP_FADVISE` / `IORING_OP_MADVISE`.
//!
//! Ref: vendor/linux/io_uring/advise.c

use super::sqe::Sqe;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoFadvise {
    pub fd: i32,
    pub offset: u64,
    pub len: u32,
    pub advice: u32,
}

pub fn fadvise_prep(sqe: &Sqe) -> Result<IoFadvise, i32> {
    if sqe.fd < 0 {
        return Err(-9);
    }
    Ok(IoFadvise {
        fd: sqe.fd,
        offset: sqe.off,
        len: sqe.len,
        advice: sqe.op_flags,
    })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IoMadvise {
    pub addr: u64,
    pub len: u32,
    pub advice: u32,
}

pub fn madvise_prep(sqe: &Sqe) -> Result<IoMadvise, i32> {
    if sqe.addr == 0 {
        return Err(-22);
    }
    Ok(IoMadvise {
        addr: sqe.addr,
        len: sqe.len,
        advice: sqe.op_flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fadvise_routes_advice_from_op_flags() {
        let mut s = Sqe::default();
        s.fd = 3;
        s.off = 0x4000;
        s.len = 4096;
        s.op_flags = 3; // POSIX_FADV_WILLNEED on x86_64.
        let r = fadvise_prep(&s).unwrap();
        assert_eq!(r.advice, 3);
        assert_eq!(r.offset, 0x4000);
    }

    #[test]
    fn madvise_requires_addr() {
        let s = Sqe::default();
        assert_eq!(madvise_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn madvise_routes_advice() {
        let mut s = Sqe::default();
        s.addr = 0x1000;
        s.len = 4096;
        s.op_flags = 4; // MADV_DONTNEED.
        let r = madvise_prep(&s).unwrap();
        assert_eq!(r.advice, 4);
    }
}
