// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/crc32poly.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013661

/// The polynomial used by `crc32_le()`, in integer form.
///
/// This preserves the `unsigned int` type of Linux's unsuffixed hexadecimal
/// integer literal.
pub const CRC32_POLY_LE: u32 = 0xedb8_8320;

/// The polynomial used by `crc32_be()`, in integer form.
///
/// This preserves the `int` type of Linux's unsuffixed hexadecimal integer
/// literal. C's usual arithmetic conversions convert it to `unsigned int`
/// when it is combined with a `u32` CRC value.
pub const CRC32_POLY_BE: i32 = 0x04c1_1db7;

/// The polynomial used by `crc32c()`, in integer form.
///
/// This preserves the `unsigned int` type of Linux's unsuffixed hexadecimal
/// integer literal.
pub const CRC32C_POLY_LE: u32 = 0x82f6_3b78;
