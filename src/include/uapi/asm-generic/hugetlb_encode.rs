// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/asm-generic/hugetlb_encode.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016005

/*
 * Several system calls take a flag to request "hugetlb" huge pages.
 * Without further specification, these system calls use the system's
 * default huge page size.  If a system supports multiple huge page sizes,
 * the desired huge page size is specified in bits [26:31] of the flag
 * arguments.  The value in these 6 bits encodes the log2 of the huge page
 * size.
 *
 * The definitions below are shared by system-call-specific UAPI headers.
 */

pub const HUGETLB_FLAG_ENCODE_SHIFT: i32 = 26;
pub const HUGETLB_FLAG_ENCODE_MASK: i32 = 0x3f;

pub const HUGETLB_FLAG_ENCODE_16KB: u32 = 14u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_64KB: u32 = 16u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_512KB: u32 = 19u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_1MB: u32 = 20u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_2MB: u32 = 21u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_8MB: u32 = 23u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_16MB: u32 = 24u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_32MB: u32 = 25u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_256MB: u32 = 28u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_512MB: u32 = 29u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_1GB: u32 = 30u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_2GB: u32 = 31u32 << HUGETLB_FLAG_ENCODE_SHIFT;
pub const HUGETLB_FLAG_ENCODE_16GB: u32 = 34u32 << HUGETLB_FLAG_ENCODE_SHIFT;
