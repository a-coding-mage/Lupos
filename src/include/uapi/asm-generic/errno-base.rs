// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/errno-base.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016002

//! Base Linux UAPI errno values.

// The source definitions are unsuffixed decimal integer literals.  On both
// frozen LP64 targets their C type is `int`, represented here by `c_int`.
pub const EPERM: core::ffi::c_int = 1;
pub const ENOENT: core::ffi::c_int = 2;
pub const ESRCH: core::ffi::c_int = 3;
pub const EINTR: core::ffi::c_int = 4;
pub const EIO: core::ffi::c_int = 5;
pub const ENXIO: core::ffi::c_int = 6;
pub const E2BIG: core::ffi::c_int = 7;
pub const ENOEXEC: core::ffi::c_int = 8;
pub const EBADF: core::ffi::c_int = 9;
pub const ECHILD: core::ffi::c_int = 10;
pub const EAGAIN: core::ffi::c_int = 11;
pub const ENOMEM: core::ffi::c_int = 12;
pub const EACCES: core::ffi::c_int = 13;
pub const EFAULT: core::ffi::c_int = 14;
pub const ENOTBLK: core::ffi::c_int = 15;
pub const EBUSY: core::ffi::c_int = 16;
pub const EEXIST: core::ffi::c_int = 17;
pub const EXDEV: core::ffi::c_int = 18;
pub const ENODEV: core::ffi::c_int = 19;
pub const ENOTDIR: core::ffi::c_int = 20;
pub const EISDIR: core::ffi::c_int = 21;
pub const EINVAL: core::ffi::c_int = 22;
pub const ENFILE: core::ffi::c_int = 23;
pub const EMFILE: core::ffi::c_int = 24;
pub const ENOTTY: core::ffi::c_int = 25;
pub const ETXTBSY: core::ffi::c_int = 26;
pub const EFBIG: core::ffi::c_int = 27;
pub const ENOSPC: core::ffi::c_int = 28;
pub const ESPIPE: core::ffi::c_int = 29;
pub const EROFS: core::ffi::c_int = 30;
pub const EMLINK: core::ffi::c_int = 31;
pub const EPIPE: core::ffi::c_int = 32;
pub const EDOM: core::ffi::c_int = 33;
pub const ERANGE: core::ffi::c_int = 34;
