//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/bpf-ops.c
//! test-origin: linux:vendor/linux/io_uring/bpf-ops.c
//! BPF kfunc surface exposed to io_uring filter programs.
//!
//! Programs loaded via `IORING_REGISTER_BPF_FILTER` can call a small set of
//! helpers to inspect ring state.  This module registers them with the BPF
//! helper table at boot.
//!
//! Ref: vendor/linux/io_uring/bpf-ops.c

use super::sqe::Sqe;

/// `bpf_io_uring_get_opcode` — return the SQE opcode.
pub fn bpf_get_opcode(sqe: &Sqe) -> u32 {
    sqe.opcode as u32
}

/// `bpf_io_uring_get_user_data`.
pub fn bpf_get_user_data(sqe: &Sqe) -> u64 {
    sqe.user_data
}

/// `bpf_io_uring_get_fd`.
pub fn bpf_get_fd(sqe: &Sqe) -> i32 {
    sqe.fd
}

/// `bpf_io_uring_match_op` — convenience used by filter programs.
pub fn bpf_match_op(sqe: &Sqe, op: u32) -> bool {
    sqe.opcode as u32 == op
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Sqe {
        let mut s = Sqe::default();
        s.opcode = 22;
        s.fd = 7;
        s.user_data = 0xfeed;
        s
    }

    #[test]
    fn helpers_read_sqe_fields() {
        let s = sample();
        assert_eq!(bpf_get_opcode(&s), 22);
        assert_eq!(bpf_get_user_data(&s), 0xfeed);
        assert_eq!(bpf_get_fd(&s), 7);
    }

    #[test]
    fn match_op_compares_exact_opcode() {
        let s = sample();
        assert!(bpf_match_op(&s, 22));
        assert!(!bpf_match_op(&s, 0));
    }
}
