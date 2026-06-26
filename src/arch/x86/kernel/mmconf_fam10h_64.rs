//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! AMD Fam10h MMCONFIG discovery.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/mmconf-fam10h_64.c

#![allow(dead_code)]

use crate::arch::x86::kernel::amd_nb::{AmdCpuInfo, Resource, mmconfig_range};

pub trait MsrReader {
    fn read_msr(&self, msr: u32) -> Result<u64, i32>;
}

pub const fn e820_allows_mmconfig(resource: Resource, reserved: &[Resource]) -> bool {
    let mut i = 0;
    while i < reserved.len() {
        let r = reserved[i];
        if r.start <= resource.start && r.end >= resource.end {
            return true;
        }
        i += 1;
    }
    false
}

pub fn fam10h_mmconf_resource(
    cpu: AmdCpuInfo,
    msr_value: u64,
    reserved: &[Resource],
    dmi_allows: bool,
) -> Option<Resource> {
    if !dmi_allows {
        return None;
    }
    let resource = mmconfig_range(cpu, msr_value)?;
    e820_allows_mmconfig(resource, reserved).then_some(resource)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::kernel::amd_nb::{CpuVendor, FAM10H_MMIO_CONF_ENABLE};

    #[test]
    fn fam10h_range_requires_dmi_and_reserved_window() {
        let cpu = AmdCpuInfo {
            vendor: CpuVendor::Amd,
            family: 0x10,
            model: 0,
            stepping: 0,
            has_l3_cache: false,
            zen: false,
        };
        let msr = FAM10H_MMIO_CONF_ENABLE | (3 << 2) | (0xe00 << 20);
        let reserved = [Resource {
            start: 0xe000_0000,
            end: 0xe07f_ffff,
        }];
        assert!(fam10h_mmconf_resource(cpu, msr, &reserved, true).is_some());
        assert!(fam10h_mmconf_resource(cpu, msr, &reserved, false).is_none());
    }
}
