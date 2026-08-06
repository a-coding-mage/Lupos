// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/asm-generic/bitops/builtin-ffs.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S012510

/// Finds the first set bit in an `int` value, using the Linux `ffs` numbering.
///
/// The result is zero for an all-zero input; otherwise it is one plus the
/// zero-based position of the least-significant set bit, including bit 31.
#[inline]
pub const fn ffs(x: i32) -> i32 {
    if x == 0 {
        0
    } else {
        x.trailing_zeros() as i32 + 1
    }
}
