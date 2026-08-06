// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/device-id/auxiliary.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013698

/// Linux `kernel_ulong_t`, which is `unsigned long` on both frozen targets.
pub type kernel_ulong_t = core::ffi::c_ulong;

/// C's unsuffixed `40` macro, whose type is `int` for the frozen targets.
pub const AUXILIARY_NAME_SIZE: core::ffi::c_int = 40;

/// C string-literal storage for `"auxiliary:"`, including its implicit NUL.
pub static AUXILIARY_MODULE_PREFIX: [u8; 11] = *b"auxiliary:\0";

/// Auxiliary-bus device identifier, with the C ABI layout used by ID tables.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct auxiliary_device_id {
    pub name: [u8; AUXILIARY_NAME_SIZE as usize],
    pub driver_data: kernel_ulong_t,
}
