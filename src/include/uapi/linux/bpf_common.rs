// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/bpf_common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016070

// These are expression macros, rather than functions, because the upstream
// definitions participate in static initializers and other integer-constant
// contexts.  The selected operands are BPF instruction-code fields (`__u8`)
// and `int` opcode constants.  Both frozen LP64 targets promote the former to
// 32-bit `int` before applying each positive `int` mask.
#[macro_export]
macro_rules! BPF_CLASS {
    ($code:expr) => {
        ((($code) as ::core::ffi::c_int) & 0x07)
    };
}

#[macro_export]
macro_rules! BPF_SIZE {
    ($code:expr) => {
        ((($code) as ::core::ffi::c_int) & 0x18)
    };
}

#[macro_export]
macro_rules! BPF_MODE {
    ($code:expr) => {
        ((($code) as ::core::ffi::c_int) & 0xe0)
    };
}

#[macro_export]
macro_rules! BPF_OP {
    ($code:expr) => {
        ((($code) as ::core::ffi::c_int) & 0xf0)
    };
}

#[macro_export]
macro_rules! BPF_SRC {
    ($code:expr) => {
        ((($code) as ::core::ffi::c_int) & 0x08)
    };
}

/* Instruction classes */
pub const BPF_LD: ::core::ffi::c_int = 0x00;
pub const BPF_LDX: ::core::ffi::c_int = 0x01;
pub const BPF_ST: ::core::ffi::c_int = 0x02;
pub const BPF_STX: ::core::ffi::c_int = 0x03;
pub const BPF_ALU: ::core::ffi::c_int = 0x04;
pub const BPF_JMP: ::core::ffi::c_int = 0x05;
pub const BPF_RET: ::core::ffi::c_int = 0x06;
pub const BPF_MISC: ::core::ffi::c_int = 0x07;

/* ld/ldx fields */
pub const BPF_W: ::core::ffi::c_int = 0x00; /* 32-bit */
pub const BPF_H: ::core::ffi::c_int = 0x08; /* 16-bit */
pub const BPF_B: ::core::ffi::c_int = 0x10; /*  8-bit */
// eBPF BPF_DW 0x18 64-bit

pub const BPF_IMM: ::core::ffi::c_int = 0x00;
pub const BPF_ABS: ::core::ffi::c_int = 0x20;
pub const BPF_IND: ::core::ffi::c_int = 0x40;
pub const BPF_MEM: ::core::ffi::c_int = 0x60;
pub const BPF_LEN: ::core::ffi::c_int = 0x80;
pub const BPF_MSH: ::core::ffi::c_int = 0xa0;

/* alu/jmp fields */
pub const BPF_ADD: ::core::ffi::c_int = 0x00;
pub const BPF_SUB: ::core::ffi::c_int = 0x10;
pub const BPF_MUL: ::core::ffi::c_int = 0x20;
pub const BPF_DIV: ::core::ffi::c_int = 0x30;
pub const BPF_OR: ::core::ffi::c_int = 0x40;
pub const BPF_AND: ::core::ffi::c_int = 0x50;
pub const BPF_LSH: ::core::ffi::c_int = 0x60;
pub const BPF_RSH: ::core::ffi::c_int = 0x70;
pub const BPF_NEG: ::core::ffi::c_int = 0x80;
pub const BPF_MOD: ::core::ffi::c_int = 0x90;
pub const BPF_XOR: ::core::ffi::c_int = 0xa0;

pub const BPF_JA: ::core::ffi::c_int = 0x00;
pub const BPF_JEQ: ::core::ffi::c_int = 0x10;
pub const BPF_JGT: ::core::ffi::c_int = 0x20;
pub const BPF_JGE: ::core::ffi::c_int = 0x30;
pub const BPF_JSET: ::core::ffi::c_int = 0x40;

pub const BPF_K: ::core::ffi::c_int = 0x00;
pub const BPF_X: ::core::ffi::c_int = 0x08;

// Mirrors `#ifndef BPF_MAXINSNS`: the frozen Kbuild commands provide no
// override, so the default is selected.  A Rust build that supplies the
// `bpf_maxinsns_override` feature must provide the inclusion-time replacement
// as `crate::BPF_MAXINSNS_OVERRIDE`; that binding is re-exported under the
// upstream public name instead of silently replacing it with the default.
#[cfg(feature = "bpf_maxinsns_override")]
pub use crate::BPF_MAXINSNS_OVERRIDE as BPF_MAXINSNS;

#[cfg(not(feature = "bpf_maxinsns_override"))]
pub const BPF_MAXINSNS: ::core::ffi::c_int = 4096;
