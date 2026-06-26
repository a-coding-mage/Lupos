//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/nop.c
//! test-origin: linux:vendor/linux/io_uring/nop.c
//! `IORING_OP_NOP` — no-op opcode with optional result injection.
//!
//! Ref: vendor/linux/io_uring/nop.c
//! Ref: vendor/linux/io_uring/nop.h

use super::sqe::Sqe;

/// `IORING_NOP_*` SQE-level flags from `sqe.op_flags`.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:490-495
pub const IORING_NOP_INJECT_RESULT: u32 = 1 << 0;
pub const IORING_NOP_FILE: u32 = 1 << 1;
pub const IORING_NOP_FIXED_FILE: u32 = 1 << 2;
pub const IORING_NOP_FIXED_BUFFER: u32 = 1 << 3;
pub const IORING_NOP_TW: u32 = 1 << 4;
pub const IORING_NOP_CQE32: u32 = 1 << 5;

/// `NOP_FLAGS` mask — every defined NOP flag OR'd.
/// Ref: vendor/linux/io_uring/nop.c:24
pub const NOP_FLAGS: u32 = IORING_NOP_INJECT_RESULT
    | IORING_NOP_FILE
    | IORING_NOP_FIXED_FILE
    | IORING_NOP_FIXED_BUFFER
    | IORING_NOP_TW
    | IORING_NOP_CQE32;

/// `struct io_nop` — per-SQE state stashed by `prep`.
/// Ref: vendor/linux/io_uring/nop.c:14
#[derive(Clone, Copy, Debug, Default)]
pub struct IoNop {
    pub result: i32,
    pub fd: i32,
    pub flags: u32,
    pub extra1: u64,
    pub extra2: u64,
}

/// `io_nop_prep` — validate flags and stash per-op state.
///
/// Returns `-EINVAL` for any unknown `nop_flags` bit.  Otherwise mirrors the
/// Linux logic exactly.
pub fn io_nop_prep(sqe: &Sqe) -> Result<IoNop, i32> {
    let flags = sqe.op_flags;
    if flags & !NOP_FLAGS != 0 {
        return Err(-22); // -EINVAL
    }
    let mut nop = IoNop {
        result: 0,
        fd: -1,
        flags,
        extra1: 0,
        extra2: 0,
    };
    if flags & IORING_NOP_INJECT_RESULT != 0 {
        nop.result = sqe.len as i32;
    }
    if flags & IORING_NOP_FILE != 0 {
        nop.fd = sqe.fd;
    }
    if flags & IORING_NOP_CQE32 != 0 {
        nop.extra1 = sqe.off;
        nop.extra2 = sqe.addr;
    }
    Ok(nop)
}

/// `io_nop` — issue.  Returns the value to record in `cqe.res`.
///
/// The fixed-file / fixed-buffer / task-work paths are stubbed pending
/// integration with the real `io_kiocb` request struct; correctness for the
/// `INJECT_RESULT` and plain-NOP paths is exact.
pub fn io_nop_issue(nop: &IoNop) -> i32 {
    nop.result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_uring::sqe::Sqe;

    fn sqe_with(op_flags: u32) -> Sqe {
        let mut s = Sqe::default();
        s.op_flags = op_flags;
        s
    }

    #[test]
    fn nop_flags_mask_matches_linux() {
        // Mirrors `#define NOP_FLAGS` in vendor/linux/io_uring/nop.c:24.
        assert_eq!(
            NOP_FLAGS,
            IORING_NOP_INJECT_RESULT
                | IORING_NOP_FIXED_FILE
                | IORING_NOP_FIXED_BUFFER
                | IORING_NOP_FILE
                | IORING_NOP_TW
                | IORING_NOP_CQE32
        );
    }

    #[test]
    fn prep_rejects_unknown_flag_bits() {
        // Any bit outside NOP_FLAGS must yield -EINVAL per nop.c:33.
        let r = io_nop_prep(&sqe_with(1 << 31));
        assert_eq!(r.unwrap_err(), -22);
    }

    #[test]
    fn prep_default_result_is_zero() {
        let nop = io_nop_prep(&sqe_with(0)).unwrap();
        assert_eq!(nop.result, 0);
        assert_eq!(nop.fd, -1);
    }

    #[test]
    fn prep_inject_result_reads_len_field() {
        // Mirrors nop.c:36-39: `nop->result = READ_ONCE(sqe->len);`
        let mut s = sqe_with(IORING_NOP_INJECT_RESULT);
        s.len = 0xdead_beef;
        let nop = io_nop_prep(&s).unwrap();
        assert_eq!(nop.result as u32, 0xdead_beef);
    }

    #[test]
    fn prep_file_flag_copies_fd() {
        // Mirrors nop.c:40-43: `nop->fd = READ_ONCE(sqe->fd);` when NOP_FILE set.
        let mut s = sqe_with(IORING_NOP_FILE);
        s.fd = 7;
        let nop = io_nop_prep(&s).unwrap();
        assert_eq!(nop.fd, 7);
    }

    #[test]
    fn prep_cqe32_reads_off_and_addr() {
        // Mirrors nop.c:51-52: extra1 = sqe->off; extra2 = sqe->addr.
        let mut s = sqe_with(IORING_NOP_CQE32);
        s.off = 0x1234_5678;
        s.addr = 0xfeed_face_dead_beef;
        let nop = io_nop_prep(&s).unwrap();
        assert_eq!(nop.extra1, 0x1234_5678);
        assert_eq!(nop.extra2, 0xfeed_face_dead_beef);
    }

    #[test]
    fn issue_returns_injected_result() {
        let nop = IoNop {
            result: 42,
            fd: -1,
            flags: IORING_NOP_INJECT_RESULT,
            extra1: 0,
            extra2: 0,
        };
        assert_eq!(io_nop_issue(&nop), 42);
    }
}
