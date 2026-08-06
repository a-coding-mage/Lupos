// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: arch/x86/include/uapi/asm/unistd.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000803

/// x32 syscall flag bit's signed 32-bit bit pattern.
///
/// The upstream definition is an untyped C preprocessor replacement list.
/// This typed item preserves its standalone `int` value, but does not itself
/// reproduce C's per-expression usual arithmetic conversions.
pub const __X32_SYSCALL_BIT: i32 = 0x4000_0000;
