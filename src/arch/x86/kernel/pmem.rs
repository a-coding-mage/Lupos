//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/pmem.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/pmem.c
//! Legacy e820 persistent-memory platform device discovery.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/pmem.c

#![allow(dead_code)]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::linux_driver_abi::base::device::Device;
use crate::linux_driver_abi::base::platform::platform_device_register;

pub const E820_PMEM_NAME: &str = "e820_pmem";
pub const E820_PMEM_COMPATIBLE: &str = "lupos,e820-pmem";
pub const IORES_DESC_PERSISTENT_MEMORY_LEGACY: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceDesc {
    pub start: u64,
    pub end: u64,
    pub desc: u32,
}

pub fn legacy_pmem_ranges(resources: &[ResourceDesc]) -> Vec<ResourceDesc> {
    resources
        .iter()
        .copied()
        .filter(|r| r.desc == IORES_DESC_PERSISTENT_MEMORY_LEGACY && r.end >= r.start)
        .collect()
}

pub fn register_e820_pmem(resources: &[ResourceDesc]) -> Result<Option<Arc<Device>>, i32> {
    if legacy_pmem_ranges(resources).is_empty() {
        return Ok(None);
    }
    platform_device_register(E820_PMEM_NAME, E820_PMEM_COMPATIBLE).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_legacy_persistent_memory_ranges() {
        let ranges = legacy_pmem_ranges(&[
            ResourceDesc {
                start: 0,
                end: 1,
                desc: 0,
            },
            ResourceDesc {
                start: 0x1000,
                end: 0x1fff,
                desc: IORES_DESC_PERSISTENT_MEMORY_LEGACY,
            },
        ]);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 0x1000);
    }
}
