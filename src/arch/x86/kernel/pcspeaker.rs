//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/pcspeaker.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/pcspeaker.c
//! Legacy PC speaker platform device registration.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/pcspeaker.c

#![allow(dead_code)]

extern crate alloc;

use alloc::sync::Arc;

use crate::linux_driver_abi::base::device::Device;
use crate::linux_driver_abi::base::platform::platform_device_register;

pub const PCSPKR_NAME: &str = "pcspkr";
pub const PCSPKR_COMPATIBLE: &str = "pnpPNP0800";

pub fn register_pcspkr() -> Result<Arc<Device>, i32> {
    platform_device_register(PCSPKR_NAME, PCSPKR_COMPATIBLE)
}

pub const fn pcspkr_platform_name() -> &'static str {
    PCSPKR_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_linux_platform_device() {
        assert_eq!(pcspkr_platform_name(), "pcspkr");
        assert_eq!(PCSPKR_COMPATIBLE, "pnpPNP0800");
    }
}
