// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/falloc.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016124

//! File-space allocation operation flags.

/// Allocate a range, extending the file size by default.
pub const FALLOC_FL_ALLOCATE_RANGE: i32 = 0x00;
/// Keep the file size unchanged.
pub const FALLOC_FL_KEEP_SIZE: i32 = 0x01;
/// Deallocate a range.
pub const FALLOC_FL_PUNCH_HOLE: i32 = 0x02;
/// Reserved codepoint.
pub const FALLOC_FL_NO_HIDE_STALE: i32 = 0x04;
/// Remove a range and collapse the following file contents into it.
pub const FALLOC_FL_COLLAPSE_RANGE: i32 = 0x08;
/// Convert a range to zeroes while retaining allocation.
pub const FALLOC_FL_ZERO_RANGE: i32 = 0x10;
/// Insert a hole and shift following file contents right.
pub const FALLOC_FL_INSERT_RANGE: i32 = 0x20;
/// Unshare copy-on-write blocks in a range.
pub const FALLOC_FL_UNSHARE_RANGE: i32 = 0x40;
/// Zero a range for subsequent overwrite without mapping metadata changes.
pub const FALLOC_FL_WRITE_ZEROES: i32 = 0x80;
