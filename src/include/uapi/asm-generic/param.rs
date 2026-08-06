// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/param.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016014

/// Default user-visible clock-tick frequency.
pub const __USER_HZ: i32 = 100;

/// Default clock-tick frequency, expanding to `__USER_HZ` in the UAPI header.
pub const HZ: i32 = __USER_HZ;

/// Default executable page size. Architecture UAPI headers may provide it first.
pub const EXEC_PAGESIZE: i32 = 4096;

/// Sentinel for no group.
pub const NOGROUP: i32 = -1;

/// Maximum hostname length.
pub const MAXHOSTNAMELEN: i32 = 64;
