// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/memfd.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016242

//! `memfd_create(2)` UAPI flags and huge-page size encodings.

use crate::include::uapi::asm_generic::hugetlb_encode::{
    HUGETLB_FLAG_ENCODE_16GB, HUGETLB_FLAG_ENCODE_1GB, HUGETLB_FLAG_ENCODE_1MB,
    HUGETLB_FLAG_ENCODE_2GB, HUGETLB_FLAG_ENCODE_2MB, HUGETLB_FLAG_ENCODE_32MB,
    HUGETLB_FLAG_ENCODE_512KB, HUGETLB_FLAG_ENCODE_512MB, HUGETLB_FLAG_ENCODE_64KB,
    HUGETLB_FLAG_ENCODE_8MB, HUGETLB_FLAG_ENCODE_MASK, HUGETLB_FLAG_ENCODE_SHIFT,
    HUGETLB_FLAG_ENCODE_16MB, HUGETLB_FLAG_ENCODE_256MB,
};

/// Close the returned file descriptor on `exec`.
pub const MFD_CLOEXEC: u32 = 0x0001;
/// Permit sealing operations on the created memfd.
pub const MFD_ALLOW_SEALING: u32 = 0x0002;
/// Create the memfd using hugetlb pages.
pub const MFD_HUGETLB: u32 = 0x0004;
/// Create a non-executable memfd and seal it against becoming executable.
pub const MFD_NOEXEC_SEAL: u32 = 0x0008;
/// Create an executable memfd.
pub const MFD_EXEC: u32 = 0x0010;

/// Bit position of the huge-page size encoding when `MFD_HUGETLB` is set.
pub const MFD_HUGE_SHIFT: i32 = HUGETLB_FLAG_ENCODE_SHIFT;
/// Mask of the huge-page size encoding before it is shifted.
pub const MFD_HUGE_MASK: i32 = HUGETLB_FLAG_ENCODE_MASK;

pub const MFD_HUGE_64KB: u32 = HUGETLB_FLAG_ENCODE_64KB;
pub const MFD_HUGE_512KB: u32 = HUGETLB_FLAG_ENCODE_512KB;
pub const MFD_HUGE_1MB: u32 = HUGETLB_FLAG_ENCODE_1MB;
pub const MFD_HUGE_2MB: u32 = HUGETLB_FLAG_ENCODE_2MB;
pub const MFD_HUGE_8MB: u32 = HUGETLB_FLAG_ENCODE_8MB;
pub const MFD_HUGE_16MB: u32 = HUGETLB_FLAG_ENCODE_16MB;
pub const MFD_HUGE_32MB: u32 = HUGETLB_FLAG_ENCODE_32MB;
pub const MFD_HUGE_256MB: u32 = HUGETLB_FLAG_ENCODE_256MB;
pub const MFD_HUGE_512MB: u32 = HUGETLB_FLAG_ENCODE_512MB;
pub const MFD_HUGE_1GB: u32 = HUGETLB_FLAG_ENCODE_1GB;
pub const MFD_HUGE_2GB: u32 = HUGETLB_FLAG_ENCODE_2GB;
pub const MFD_HUGE_16GB: u32 = HUGETLB_FLAG_ENCODE_16GB;
