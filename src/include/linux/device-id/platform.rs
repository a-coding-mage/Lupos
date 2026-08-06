// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/device-id/platform.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013727

/// Linux `kernel_ulong_t`, which is `unsigned long` for the frozen 64-bit
/// kernel configurations.
pub type kernel_ulong_t = u64;

/// Size of `platform_device_id::name` in bytes.
pub const PLATFORM_NAME_SIZE: usize = 24;

/// Platform-driver match entry.
///
/// This preserves the C field order and native alignment used by both frozen
/// 64-bit architectures.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct platform_device_id {
    pub name: [u8; PLATFORM_NAME_SIZE],
    pub driver_data: kernel_ulong_t,
}
