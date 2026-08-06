// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/vdso/time64.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016541

//! vDSO time-unit conversion parameters.

/// Milliseconds per second (`long` in the Linux source).
pub const MSEC_PER_SEC: core::ffi::c_long = 1_000;
/// Microseconds per millisecond (`long` in the Linux source).
pub const USEC_PER_MSEC: core::ffi::c_long = 1_000;
/// Nanoseconds per microsecond (`long` in the Linux source).
pub const NSEC_PER_USEC: core::ffi::c_long = 1_000;
/// Nanoseconds per millisecond (`long` in the Linux source).
pub const NSEC_PER_MSEC: core::ffi::c_long = 1_000_000;
/// Microseconds per second (`long` in the Linux source).
pub const USEC_PER_SEC: core::ffi::c_long = 1_000_000;
/// Nanoseconds per second (`long` in the Linux source).
pub const NSEC_PER_SEC: core::ffi::c_long = 1_000_000_000;
/// Picoseconds per second (`long long` in the Linux source).
pub const PSEC_PER_SEC: core::ffi::c_longlong = 1_000_000_000_000;
/// Femtoseconds per second (`long long` in the Linux source).
pub const FSEC_PER_SEC: core::ffi::c_longlong = 1_000_000_000_000_000;
