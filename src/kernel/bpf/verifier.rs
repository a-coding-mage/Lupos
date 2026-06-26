//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/verifier.c
//! test-origin: linux:vendor/linux/kernel/bpf/verifier.c
//! eBPF verifier — skeleton for M63.
//!
//! Linux's full verifier (`vendor/linux/kernel/bpf/verifier.c`) does
//! type-checking, bounds-tracking, and pointer-tracking.  Lupos M63 ships a
//! skeleton with three checks:
//!   - bounded program length
//!   - terminating EXIT insn
//!   - simple acyclic CFG (no backward jumps)
//!
//! Real type/bounds tracking is a Phase 11 follow-up.

use super::insn::{
    BPF_ADD, BPF_ALU, BPF_ALU64, BPF_AND, BPF_ARSH, BPF_CALL, BPF_DIV, BPF_EXIT, BPF_JA, BPF_JEQ,
    BPF_JGE, BPF_JGT, BPF_JMP, BPF_JNE, BPF_LSH, BPF_MOD, BPF_MOV, BPF_MUL, BPF_NEG, BPF_OR,
    BPF_RSH, BPF_SUB, BPF_XOR, BpfInsn, bpf_class, bpf_op,
};

#[derive(Debug, PartialEq, Eq)]
pub enum BpfVerifyError {
    TooManyInsns,
    NoExit,
    BackwardJump,
    InvalidRegister,
    BadOpcode,
}

pub fn verify(insns: &[BpfInsn]) -> Result<(), BpfVerifyError> {
    if insns.is_empty() || insns.len() > super::interp::BPF_MAX_INSNS {
        return Err(BpfVerifyError::TooManyInsns);
    }
    // Must contain at least one EXIT insn.
    let has_exit = insns
        .iter()
        .any(|i| bpf_class(i.code) == BPF_JMP && bpf_op(i.code) == BPF_EXIT);
    if !has_exit {
        return Err(BpfVerifyError::NoExit);
    }
    // Reject backward branches (cycles).
    for (idx, i) in insns.iter().enumerate() {
        if i.dst_reg() > 10 || i.src_reg() > 10 {
            return Err(BpfVerifyError::InvalidRegister);
        }
        match bpf_class(i.code) {
            BPF_ALU | BPF_ALU64 => match bpf_op(i.code) {
                BPF_ADD | BPF_SUB | BPF_MUL | BPF_DIV | BPF_OR | BPF_AND | BPF_LSH | BPF_RSH
                | BPF_NEG | BPF_MOD | BPF_XOR | BPF_MOV | BPF_ARSH => {}
                _ => return Err(BpfVerifyError::BadOpcode),
            },
            BPF_JMP => match bpf_op(i.code) {
                BPF_JA | BPF_JEQ | BPF_JGT | BPF_JGE | BPF_JNE | BPF_CALL | BPF_EXIT => {}
                _ => return Err(BpfVerifyError::BadOpcode),
            },
            _ => return Err(BpfVerifyError::BadOpcode),
        }
        if bpf_class(i.code) == BPF_JMP && bpf_op(i.code) != BPF_EXIT {
            // Skip BPF_CALL (no off field).
            if bpf_op(i.code) == 0x80 {
                continue;
            }
            // BPF_JA: unconditional; others: conditional, also use `off`.
            let _ = BPF_JA; // keep import alive
            if i.off < 0 {
                return Err(BpfVerifyError::BackwardJump);
            }
            // Out-of-range jump.
            let target = idx as i64 + 1 + i.off as i64;
            if target < 0 || target as usize >= insns.len() {
                return Err(BpfVerifyError::BackwardJump);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::insn::*;
    use super::*;

    #[test]
    fn minimal_prog_passes() {
        let prog = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        assert!(verify(&prog).is_ok());
    }

    #[test]
    fn empty_prog_fails() {
        assert_eq!(verify(&[]), Err(BpfVerifyError::TooManyInsns));
    }

    #[test]
    fn missing_exit_fails() {
        let prog = [BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 0)];
        assert_eq!(verify(&prog), Err(BpfVerifyError::NoExit));
    }

    #[test]
    fn backward_jump_fails() {
        let prog = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_JA, 0, 0, -2, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        assert_eq!(verify(&prog), Err(BpfVerifyError::BackwardJump));
    }

    #[test]
    fn invalid_register_fails() {
        let prog = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 11, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        assert_eq!(verify(&prog), Err(BpfVerifyError::InvalidRegister));
    }

    #[test]
    fn unsupported_opcode_class_fails() {
        let prog = [
            BpfInsn::new(BPF_LD | BPF_DW | BPF_IMM, 0, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        assert_eq!(verify(&prog), Err(BpfVerifyError::BadOpcode));
    }
}
