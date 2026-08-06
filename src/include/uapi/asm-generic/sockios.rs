// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/sockios.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016024

//! Socket-level I/O control call numbers.

// The source definitions are unsuffixed hexadecimal integer literals. On both
// frozen targets their C type is `int`, represented here by `c_int`.
pub const FIOSETOWN: core::ffi::c_int = 0x8901;
pub const SIOCSPGRP: core::ffi::c_int = 0x8902;
pub const FIOGETOWN: core::ffi::c_int = 0x8903;
pub const SIOCGPGRP: core::ffi::c_int = 0x8904;
pub const SIOCATMARK: core::ffi::c_int = 0x8905;

/// Get a socket timestamp as a `timeval`.
pub const SIOCGSTAMP_OLD: core::ffi::c_int = 0x8906;

/// Get a socket timestamp as a `timespec`.
pub const SIOCGSTAMPNS_OLD: core::ffi::c_int = 0x8907;
