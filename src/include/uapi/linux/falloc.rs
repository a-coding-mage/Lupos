// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/falloc.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016124

/// Allocate a range; the default operation extends the file size.
pub const FALLOC_FL_ALLOCATE_RANGE: i32 = 0x00;

/// Preserve the file size while performing the selected operation.
pub const FALLOC_FL_KEEP_SIZE: i32 = 0x01;

/// Deallocate the selected range.
pub const FALLOC_FL_PUNCH_HOLE: i32 = 0x02;

/// Reserved fallocate mode codepoint.
pub const FALLOC_FL_NO_HIDE_STALE: i32 = 0x04;

/// Remove a range and collapse subsequent file contents into it.
pub const FALLOC_FL_COLLAPSE_RANGE: i32 = 0x08;

/// Convert a range to zeroes, preferably without issuing data I/O.
pub const FALLOC_FL_ZERO_RANGE: i32 = 0x10;

/// Insert a hole by shifting subsequent file contents to the right.
pub const FALLOC_FL_INSERT_RANGE: i32 = 0x20;

/// Preemptively unshare copy-on-write blocks in the selected range.
pub const FALLOC_FL_UNSHARE_RANGE: i32 = 0x40;

/// Zero a range while preserving its mapping metadata for future overwrites.
pub const FALLOC_FL_WRITE_ZEROES: i32 = 0x80;
