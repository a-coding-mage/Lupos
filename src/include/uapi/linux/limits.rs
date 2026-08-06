// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/limits.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016224

/// Maximum number of open files per process.
///
/// C source literal: `1024` (`int`).
pub const NR_OPEN: i32 = 1024;

/// Maximum number of supplemental group IDs.
///
/// C source literal: `65536` (`int`).
pub const NGROUPS_MAX: i32 = 65_536;

/// Maximum total byte count of arguments and environment for `exec()`.
///
/// C source literal: `131072` (`int`).
pub const ARG_MAX: i32 = 131_072;

/// Maximum number of links to a file.
///
/// C source literal: `127` (`int`).
pub const LINK_MAX: i32 = 127;

/// Size of the canonical input queue.
///
/// C source literal: `255` (`int`).
pub const MAX_CANON: i32 = 255;

/// Size of the type-ahead buffer.
///
/// C source literal: `255` (`int`).
pub const MAX_INPUT: i32 = 255;

/// Maximum number of characters in a file name.
///
/// C source literal: `255` (`int`).
pub const NAME_MAX: i32 = 255;

/// Maximum number of characters in a path name, including the NUL byte.
///
/// C source literal: `4096` (`int`).
pub const PATH_MAX: i32 = 4096;

/// Maximum bytes in an atomic write to a pipe.
///
/// C source literal: `4096` (`int`).
pub const PIPE_BUF: i32 = 4096;

/// Maximum number of characters in an extended-attribute name.
///
/// C source literal: `255` (`int`).
pub const XATTR_NAME_MAX: i32 = 255;

/// Maximum size of an extended-attribute value.
///
/// C source literal: `65536` (`int`).
pub const XATTR_SIZE_MAX: i32 = 65_536;

/// Maximum size of an extended-attribute name list.
///
/// C source literal: `65536` (`int`).
pub const XATTR_LIST_MAX: i32 = 65_536;

/// Maximum number of real-time signals.
///
/// C source literal: `32` (`int`).
pub const RTSIG_MAX: i32 = 32;
