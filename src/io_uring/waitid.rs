//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/waitid.c
//! test-origin: linux:vendor/linux/io_uring/waitid.c
//! `IORING_OP_WAITID`.
//!
//! Ref: vendor/linux/io_uring/waitid.c

use super::sqe::Sqe;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoWaitid {
    pub which: u32,
    pub upid: i32,
    pub options: u32,
    pub siginfo_addr: u64,
    pub rusage_addr: u64,
}

pub fn waitid_prep(sqe: &Sqe) -> Result<IoWaitid, i32> {
    // Linux validates `options` and `which` ranges in the actual waitid(2)
    // path; here we mirror the prep checks.
    if sqe.op_flags != 0 {
        return Err(-22);
    }
    Ok(IoWaitid {
        which: sqe.fd as u32,
        upid: sqe.len as i32,
        options: sqe.addr3 as u32,
        siginfo_addr: sqe.addr,
        rusage_addr: sqe.off,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waitid_rejects_flags() {
        let mut s = Sqe::default();
        s.op_flags = 1;
        assert_eq!(waitid_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn waitid_stashes_which_pid_options() {
        let mut s = Sqe::default();
        s.fd = 1; // P_PID
        s.len = 1234;
        s.addr3 = 4; // WEXITED
        let r = waitid_prep(&s).unwrap();
        assert_eq!(r.which, 1);
        assert_eq!(r.upid, 1234);
        assert_eq!(r.options, 4);
    }
}
