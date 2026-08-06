// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/asm-generic/audit_dir_write.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S012500

/// Expands the x86_64 native instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_dir_write_x86_64_native {
    ($consumer:ident) => {
        $consumer!(
            82_u32, 83_u32, 84_u32, 85_u32, 86_u32, 87_u32, 88_u32,
            133_u32, 258_u32, 259_u32, 263_u32, 264_u32, 265_u32, 266_u32,
            316_u32,
        )
    };
}

/// Expands the x86_64 IA32 instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_dir_write_x86_64_ia32 {
    ($consumer:ident) => {
        $consumer!(
            38_u32, 39_u32, 40_u32, 8_u32, 9_u32, 10_u32, 83_u32, 14_u32,
            296_u32, 297_u32, 301_u32, 302_u32, 303_u32, 304_u32, 353_u32,
        )
    };
}

/// Expands the AArch64 native instance of the upstream initializer fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_dir_write_aarch64_native {
    ($consumer:ident) => {
        $consumer!(34_u32, 33_u32, 35_u32, 38_u32, 37_u32, 36_u32, 276_u32,)
    };
}

/// Expands the AArch64 AArch32-compat instance of the upstream initializer
/// fragment.
///
/// The consumer owns the receiving `unsigned int`-compatible static array and
/// its following `~0U` sentinel, as each C inclusion site does.
#[macro_export]
macro_rules! audit_dir_write_aarch64_compat {
    ($consumer:ident) => {
        $consumer!(
            38_u32, 39_u32, 40_u32, 8_u32, 9_u32, 10_u32, 83_u32, 14_u32,
            323_u32, 324_u32, 328_u32, 329_u32, 330_u32, 331_u32, 382_u32,
        )
    };
}
