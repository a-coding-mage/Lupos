// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/device-id/isapnp.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013716

/// Linux `kernel_ulong_t`, which is C `unsigned long` on both frozen targets.
pub type kernel_ulong_t = core::ffi::c_ulong;

/// C's unsuffixed `0xffff` macro, whose type is `int` on the frozen targets.
pub const ISAPNP_ANY_ID: core::ffi::c_int = 0xffff;

/// C ABI representation of `struct isapnp_device_id`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct isapnp_device_id {
    pub card_vendor: u16,
    pub card_device: u16,
    pub vendor: u16,
    pub function: u16,
    /// Data private to the driver.
    pub driver_data: kernel_ulong_t,
}
