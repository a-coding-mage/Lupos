// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/asm-generic/audit_read.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S012501

/// Expands the x86_64 native instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as the C inclusion site does.
#[macro_export]
macro_rules! audit_read_x86_64_native {
    ($consumer:ident) => {
        $consumer!(
            89_u32, 179_u32, 194_u32, 465_u32, 195_u32, 196_u32, 191_u32,
            464_u32, 192_u32, 193_u32, 267_u32,
        )
    };
}

/// Expands the x86_64 IA32 instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as the C inclusion site does.
#[macro_export]
macro_rules! audit_read_x86_64_ia32 {
    ($consumer:ident) => {
        $consumer!(
            85_u32, 131_u32, 232_u32, 465_u32, 233_u32, 234_u32, 229_u32,
            464_u32, 230_u32, 231_u32, 305_u32,
        )
    };
}

/// Expands the AArch64 native instance of the upstream initializer fragment.
///
/// `__NR_readlink` is absent from the AArch64 syscall header, so its guarded
/// entry is omitted exactly as it is by the C preprocessor. The consumer owns
/// the receiving `unsigned int`-compatible static array and its following
/// `~0U` sentinel.
#[macro_export]
macro_rules! audit_read_aarch64_native {
    ($consumer:ident) => {
        $consumer!(
            60_u32, 11_u32, 465_u32, 12_u32, 13_u32, 8_u32, 464_u32, 9_u32,
            10_u32, 78_u32,
        )
    };
}

/// Expands the AArch64 AArch32-compat instance of the upstream initializer
/// fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as the C inclusion site does.
#[macro_export]
macro_rules! audit_read_aarch64_compat {
    ($consumer:ident) => {
        $consumer!(
            85_u32, 131_u32, 232_u32, 465_u32, 233_u32, 234_u32, 229_u32,
            464_u32, 230_u32, 231_u32, 332_u32,
        )
    };
}
