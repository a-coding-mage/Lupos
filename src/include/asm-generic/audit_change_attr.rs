// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/asm-generic/audit_change_attr.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S012499

/// Expands the x86_64 native instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_change_attr_x86_64_native {
    ($consumer:ident) => {
        $consumer!(
            90_u32, 91_u32, 92_u32, 94_u32, 93_u32, 188_u32, 463_u32,
            189_u32, 190_u32, 197_u32, 466_u32, 198_u32, 199_u32, 260_u32,
            268_u32, 452_u32, 86_u32, 265_u32,
        )
    };
}

/// Expands the x86_64 IA32 instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_change_attr_x86_64_ia32 {
    ($consumer:ident) => {
        $consumer!(
            15_u32, 94_u32, 182_u32, 16_u32, 95_u32, 226_u32, 463_u32,
            227_u32, 228_u32, 235_u32, 466_u32, 236_u32, 237_u32, 298_u32,
            306_u32, 452_u32, 212_u32, 207_u32, 198_u32, 9_u32, 303_u32,
        )
    };
}

/// Expands the AArch64 native instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_change_attr_aarch64_native {
    ($consumer:ident) => {
        $consumer!(
            52_u32, 55_u32, 5_u32, 463_u32, 6_u32, 7_u32, 14_u32, 466_u32,
            15_u32, 16_u32, 54_u32, 53_u32, 452_u32, 37_u32,
        )
    };
}

/// Expands the AArch64 AArch32-compat instance of the upstream initializer
/// fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_change_attr_aarch64_compat {
    ($consumer:ident) => {
        $consumer!(
            15_u32, 94_u32, 182_u32, 16_u32, 95_u32, 226_u32, 463_u32,
            227_u32, 228_u32, 235_u32, 466_u32, 236_u32, 237_u32, 325_u32,
            333_u32, 452_u32, 212_u32, 207_u32, 198_u32, 9_u32, 330_u32,
        )
    };
}
