// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/device-id/spi.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013735

/// C `unsigned long` selected by the `__KERNEL__` branch on both frozen
/// 64-bit LP64 targets.
#[allow(non_camel_case_types)]
pub type kernel_ulong_t = u64;

/// Translation of C's object-like `SPI_NAME_SIZE` macro.
///
/// The unsuffixed C literal has `int` type on both frozen targets. Array
/// bounds explicitly convert that value where Rust requires `usize`.
#[macro_export]
macro_rules! SPI_NAME_SIZE {
    () => {
        32i32
    };
}

/// Translation of C's object-like `SPI_MODULE_PREFIX` macro.
///
/// Each invocation expands to the original NUL-terminated C string literal;
/// it does not declare an addressable header object or pointer alias.
#[macro_export]
macro_rules! SPI_MODULE_PREFIX {
    () => {
        b"spi:\0"
    };
}

/// C layout of `struct spi_device_id`.
///
/// `name` is a fixed-width C `char` array, not a Rust string. The frozen
/// command lines select unsigned C `char` on both architectures. `driver_data`
/// remains opaque machine-word driver-private data.
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct spi_device_id {
    pub name: [u8; SPI_NAME_SIZE!() as usize],
    pub driver_data: kernel_ulong_t,
}
