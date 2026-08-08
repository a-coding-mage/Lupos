// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: arch/x86/include/uapi/asm/debugreg.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000772

//! x86 debug-register numbers and bit encodings exposed by the Linux UAPI.

pub const DR_FIRSTADDR: i32 = 0;
pub const DR_LASTADDR: i32 = 3;

pub const DR_STATUS: i32 = 6;
pub const DR_CONTROL: i32 = 7;

/* This hexadecimal C literal has unsigned-int type. */
pub const DR6_RESERVED: u32 = 0xFFFF_0FF0;

pub const DR_TRAP0: i32 = 0x1;
pub const DR_TRAP1: i32 = 0x2;
pub const DR_TRAP2: i32 = 0x4;
pub const DR_TRAP3: i32 = 0x8;
pub const DR_TRAP_BITS: i32 = DR_TRAP0 | DR_TRAP1 | DR_TRAP2 | DR_TRAP3;

pub const DR_BUS_LOCK: i32 = 0x800;
pub const DR_STEP: i32 = 0x4000;
pub const DR_SWITCH: i32 = 0x8000;

pub const DR_CONTROL_SHIFT: i32 = 16;
pub const DR_CONTROL_SIZE: i32 = 4;

pub const DR_RW_EXECUTE: i32 = 0x0;
pub const DR_RW_WRITE: i32 = 0x1;
pub const DR_RW_READ: i32 = 0x3;

pub const DR_LEN_1: i32 = 0x0;
pub const DR_LEN_2: i32 = 0x4;
pub const DR_LEN_4: i32 = 0xC;
pub const DR_LEN_8: i32 = 0x8;

pub const DR_LOCAL_ENABLE_SHIFT: i32 = 0;
pub const DR_GLOBAL_ENABLE_SHIFT: i32 = 1;
pub const DR_LOCAL_ENABLE: i32 = 0x1;
pub const DR_GLOBAL_ENABLE: i32 = 0x2;
pub const DR_ENABLE_SIZE: i32 = 2;

pub const DR_LOCAL_ENABLE_MASK: i32 = 0x55;
pub const DR_GLOBAL_ENABLE_MASK: i32 = 0xAA;

/* The selected x86_64 branch is the C unsigned-long value. */
pub const DR_CONTROL_RESERVED: u64 = 0xFFFF_FFFF_0000_FC00;

pub const DR_LOCAL_SLOWDOWN: i32 = 0x100;
pub const DR_GLOBAL_SLOWDOWN: i32 = 0x200;
