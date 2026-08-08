// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/bpf_common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016070

//! Instruction classes and fields from Linux's classic BPF UAPI.

/// Extract the instruction class from an opcode.
#[inline]
pub const fn BPF_CLASS(code: u32) -> u32 {
    code & 0x07
}

pub const BPF_LD: u32 = 0x00;
pub const BPF_LDX: u32 = 0x01;
pub const BPF_ST: u32 = 0x02;
pub const BPF_STX: u32 = 0x03;
pub const BPF_ALU: u32 = 0x04;
pub const BPF_JMP: u32 = 0x05;
pub const BPF_RET: u32 = 0x06;
pub const BPF_MISC: u32 = 0x07;

/// Extract the load/store size field from an opcode.
#[inline]
pub const fn BPF_SIZE(code: u32) -> u32 {
    code & 0x18
}

pub const BPF_W: u32 = 0x00;
pub const BPF_H: u32 = 0x08;
pub const BPF_B: u32 = 0x10;

/// Extract the load/store addressing mode from an opcode.
#[inline]
pub const fn BPF_MODE(code: u32) -> u32 {
    code & 0xe0
}

pub const BPF_IMM: u32 = 0x00;
pub const BPF_ABS: u32 = 0x20;
pub const BPF_IND: u32 = 0x40;
pub const BPF_MEM: u32 = 0x60;
pub const BPF_LEN: u32 = 0x80;
pub const BPF_MSH: u32 = 0xa0;

/// Extract the ALU/jump operation field from an opcode.
#[inline]
pub const fn BPF_OP(code: u32) -> u32 {
    code & 0xf0
}

pub const BPF_ADD: u32 = 0x00;
pub const BPF_SUB: u32 = 0x10;
pub const BPF_MUL: u32 = 0x20;
pub const BPF_DIV: u32 = 0x30;
pub const BPF_OR: u32 = 0x40;
pub const BPF_AND: u32 = 0x50;
pub const BPF_LSH: u32 = 0x60;
pub const BPF_RSH: u32 = 0x70;
pub const BPF_NEG: u32 = 0x80;
pub const BPF_MOD: u32 = 0x90;
pub const BPF_XOR: u32 = 0xa0;

pub const BPF_JA: u32 = 0x00;
pub const BPF_JEQ: u32 = 0x10;
pub const BPF_JGT: u32 = 0x20;
pub const BPF_JGE: u32 = 0x30;
pub const BPF_JSET: u32 = 0x40;

/// Extract the ALU/jump source field from an opcode.
#[inline]
pub const fn BPF_SRC(code: u32) -> u32 {
    code & 0x08
}

pub const BPF_K: u32 = 0x00;
pub const BPF_X: u32 = 0x08;

pub const BPF_MAXINSNS: u32 = 4096;
