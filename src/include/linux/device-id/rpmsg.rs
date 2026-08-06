// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/device-id/rpmsg.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013730

/// C `unsigned long` on each frozen 64-bit target.
pub type kernel_ulong_t = u64;

/// `RPMSG_NAME_SIZE` from the C header; its unsuffixed literal has C `int` type.
pub const RPMSG_NAME_SIZE: i32 = 32;

/// Lowering of the object-like C string-literal macro.
///
/// The expansion is a reference to a fixed-size byte array, so it retains the
/// literal's trailing NUL and supplies a thin pointer through `as_ptr()` at an
/// FFI boundary.
#[macro_export]
macro_rules! RPMSG_DEVICE_MODALIAS_FMT {
    () => {
        b"rpmsg:%s\0"
    };
}

/// Rust representation of C `struct rpmsg_device_id`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct rpmsg_device_id {
    pub name: [u8; RPMSG_NAME_SIZE as usize],
    pub driver_data: kernel_ulong_t,
}
