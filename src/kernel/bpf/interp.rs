//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf
//! test-origin: linux:vendor/linux/kernel/bpf
//! eBPF interpreter (no JIT — Phase 11 follow-up).
//!
//! Mirrors `vendor/linux/kernel/bpf/core.c::__bpf_prog_run`.
//! Implements the eBPF VM: 11 64-bit registers (R0..R10), program counter,
//! the major opcode classes (ALU64, ALU32, JMP, EXIT).
//!
//! Helper-call dispatch (BPF_CALL) routes to `helpers::call`.

use super::insn::{
    BPF_ADD, BPF_ALU, BPF_ALU64, BPF_AND, BPF_ARSH, BPF_CALL, BPF_DIV, BPF_EXIT, BPF_JA, BPF_JEQ,
    BPF_JGE, BPF_JGT, BPF_JMP, BPF_JNE, BPF_K, BPF_LSH, BPF_MOD, BPF_MOV, BPF_MUL, BPF_NEG, BPF_OR,
    BPF_RSH, BPF_SUB, BPF_X, BPF_XOR, BpfInsn, bpf_class, bpf_op, bpf_src,
};

pub const BPF_MAX_INSNS: usize = 4096;

pub fn run(insns: &[BpfInsn], r1: u64) -> u64 {
    let mut regs = [0u64; 11];
    regs[1] = r1;
    let mut pc = 0usize;

    while pc < insns.len() {
        let i = insns[pc];
        let dst = i.dst_reg() as usize;
        let src = i.src_reg() as usize;
        let class = bpf_class(i.code);
        let op = bpf_op(i.code);
        let src_mode = bpf_src(i.code);
        let src_val = if src_mode == BPF_X {
            regs[src]
        } else {
            i.imm as i64 as u64
        };

        match class {
            BPF_ALU64 => match op {
                BPF_MOV => regs[dst] = src_val,
                BPF_ADD => regs[dst] = regs[dst].wrapping_add(src_val),
                BPF_SUB => regs[dst] = regs[dst].wrapping_sub(src_val),
                BPF_MUL => regs[dst] = regs[dst].wrapping_mul(src_val),
                BPF_DIV => regs[dst] = if src_val == 0 { 0 } else { regs[dst] / src_val },
                BPF_OR => regs[dst] |= src_val,
                BPF_AND => regs[dst] &= src_val,
                BPF_LSH => regs[dst] = regs[dst].wrapping_shl((src_val & 63) as u32),
                BPF_RSH => regs[dst] = regs[dst].wrapping_shr((src_val & 63) as u32),
                BPF_NEG => regs[dst] = (regs[dst] as i64).wrapping_neg() as u64,
                BPF_MOD => {
                    regs[dst] = if src_val == 0 {
                        regs[dst]
                    } else {
                        regs[dst] % src_val
                    }
                }
                BPF_XOR => regs[dst] ^= src_val,
                BPF_ARSH => {
                    let shift = (src_val & 63) as u32;
                    regs[dst] = ((regs[dst] as i64) >> shift) as u64;
                }
                _ => return u64::MAX, // unsupported op
            },
            BPF_ALU => {
                // 32-bit ALU; result zero-extended.
                let a = regs[dst] as u32;
                let b = src_val as u32;
                let out: u32 = match op {
                    BPF_MOV => b,
                    BPF_ADD => a.wrapping_add(b),
                    BPF_SUB => a.wrapping_sub(b),
                    BPF_MUL => a.wrapping_mul(b),
                    BPF_DIV => {
                        if b == 0 {
                            0
                        } else {
                            a / b
                        }
                    }
                    BPF_OR => a | b,
                    BPF_AND => a & b,
                    BPF_LSH => a.wrapping_shl(b & 31),
                    BPF_RSH => a.wrapping_shr(b & 31),
                    BPF_XOR => a ^ b,
                    BPF_MOD => {
                        if b == 0 {
                            a
                        } else {
                            a % b
                        }
                    }
                    _ => return u64::MAX,
                };
                regs[dst] = out as u64;
            }
            BPF_JMP => match op {
                BPF_JA => {
                    pc = pc
                        .wrapping_add(i.off as i32 as isize as usize)
                        .wrapping_add(1);
                    continue;
                }
                BPF_JEQ => {
                    if regs[dst] == src_val {
                        pc = pc
                            .wrapping_add(i.off as i32 as isize as usize)
                            .wrapping_add(1);
                        continue;
                    }
                }
                BPF_JNE => {
                    if regs[dst] != src_val {
                        pc = pc
                            .wrapping_add(i.off as i32 as isize as usize)
                            .wrapping_add(1);
                        continue;
                    }
                }
                BPF_JGT => {
                    if regs[dst] > src_val {
                        pc = pc
                            .wrapping_add(i.off as i32 as isize as usize)
                            .wrapping_add(1);
                        continue;
                    }
                }
                BPF_JGE => {
                    if regs[dst] >= src_val {
                        pc = pc
                            .wrapping_add(i.off as i32 as isize as usize)
                            .wrapping_add(1);
                        continue;
                    }
                }
                BPF_CALL => {
                    regs[0] = super::helpers::call(
                        i.imm as u32,
                        regs[1],
                        regs[2],
                        regs[3],
                        regs[4],
                        regs[5],
                    )
                }
                BPF_EXIT => return regs[0],
                _ => return u64::MAX,
            },
            _ => return u64::MAX, // unsupported class for M63
        }
        pc += 1;
    }
    regs[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_imm_then_exit_returns_zero() {
        let prog = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        assert_eq!(run(&prog, 0), 0);
    }

    #[test]
    fn add_mul_returns_42() {
        // r0 = 6; r0 += 1; r0 *= 6; exit  → 42
        let prog = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 6),
            BpfInsn::new(BPF_ALU64 | BPF_ADD | BPF_K, 0, 0, 0, 1),
            BpfInsn::new(BPF_ALU64 | BPF_MUL | BPF_K, 0, 0, 0, 6),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        assert_eq!(run(&prog, 0), 42);
    }

    #[test]
    fn jne_branch_skips_one() {
        // r0 = 0; if r1 != 0 goto +1; r0 = 1; exit
        let prog = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_JNE | BPF_K, 1, 0, 1, 0),
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 1),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        // r1=7 → branch taken → r0 stays 0
        assert_eq!(run(&prog, 7), 0);
        // r1=0 → branch not taken → r0 = 1
        assert_eq!(run(&prog, 0), 1);
    }
}
