// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/device-id/i2c.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013711

pub type kernel_ulong_t = u64;

/// Translation of C's object-like `I2C_NAME_SIZE` macro.
///
/// The unsuffixed C literal has `int` type on both frozen targets. Array
/// bounds explicitly convert that value where Rust requires `usize`.
#[macro_export]
macro_rules! I2C_NAME_SIZE {
    () => {
        20i32
    };
}

/// Translation of C's object-like `I2C_MODULE_PREFIX` macro.
///
/// Each invocation expands to the original NUL-terminated C string literal;
/// it does not declare an addressable header object or pointer alias.
#[macro_export]
macro_rules! I2C_MODULE_PREFIX {
    () => {
        b"i2c:\0"
    };
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct i2c_device_id {
    pub name: [u8; I2C_NAME_SIZE!() as usize],
    pub driver_data: kernel_ulong_t,
}
