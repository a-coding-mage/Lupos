// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/device-id/mhi.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013721

/// C `unsigned long` used as MHI driver-private data.
///
/// The C typedef is within `__KERNEL__`; both frozen translation command
/// families select that branch and target 64-bit LP64, so this remains the
/// eight-byte C object type and preserves the field ABI of `mhi_device_id`.
#[allow(non_camel_case_types)]
pub type kernel_ulong_t = u64;

/// Translation of C's object-like `MHI_DEVICE_MODALIAS_FMT` macro.
///
/// Each invocation expands to the original NUL-terminated C string literal;
/// it does not declare an addressable header object or pointer alias.
#[macro_export]
macro_rules! MHI_DEVICE_MODALIAS_FMT {
    () => {
        b"mhi:%s\0"
    };
}

/// Translation of C's object-like `MHI_NAME_SIZE` macro.
///
/// The unsuffixed C literal has `int` type on both frozen targets.  Array
/// bounds explicitly convert that value where Rust requires `usize`.
#[macro_export]
macro_rules! MHI_NAME_SIZE {
    () => {
        32i32
    };
}

/// Translation of C's object-like `MHI_EP_DEVICE_MODALIAS_FMT` macro.
///
/// Each invocation expands to the original NUL-terminated C string literal;
/// it does not declare an addressable header object or pointer alias.
#[macro_export]
macro_rules! MHI_EP_DEVICE_MODALIAS_FMT {
    () => {
        b"mhi_ep:%s\0"
    };
}

/// C layout of `struct mhi_device_id`.
///
/// `chan` is an unsigned-octet array because the frozen kernel command lines
/// compile C `char` with `-funsigned-char`.  It is a fixed-width channel name,
/// not a Rust string.  The private field and read-only accessor preserve C's
/// `const` member contract.  `driver_data` is opaque machine-word driver data.
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct mhi_device_id {
    chan: [u8; MHI_NAME_SIZE!() as usize],
    pub driver_data: kernel_ulong_t,
}

impl mhi_device_id {
    /// Returns the inline, C-const channel-name bytes.
    #[inline]
    pub const fn chan(&self) -> &[u8; MHI_NAME_SIZE!() as usize] {
        &self.chan
    }
}
