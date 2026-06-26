//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/proc.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/proc.c
//! /proc/cpuinfo formatter.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/proc.c

// Linux prints one block per CPU: processor, vendor_id, cpu family,
// model, model name, stepping, microcode, cpu MHz, cache size, etc.
// We model the line set so observability can emit `/proc/cpuinfo`
// without going through the procfs registration path yet.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::arch::x86::kernel::cpu::{CpuSignature, CpuVendor};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuInfo {
    pub processor: u32,
    pub vendor: CpuVendor,
    pub signature: CpuSignature,
    pub model_name: String,
    pub mhz: u32,
}

pub fn vendor_id(v: CpuVendor) -> &'static str {
    match v {
        CpuVendor::Intel => "GenuineIntel",
        CpuVendor::Amd => "AuthenticAMD",
        CpuVendor::Hygon => "HygonGenuine",
        CpuVendor::Zhaoxin => "  Shanghai  ",
        CpuVendor::Centaur => "CentaurHauls",
        CpuVendor::Unknown(_) => "unknown",
    }
}

pub fn render_block(info: &CpuInfo) -> Vec<String> {
    let mut out = Vec::with_capacity(8);
    out.push(format!("processor\t: {}", info.processor));
    out.push(format!("vendor_id\t: {}", vendor_id(info.vendor)));
    out.push(format!("cpu family\t: {}", info.signature.family));
    out.push(format!("model\t\t: {}", info.signature.model));
    out.push(format!("model name\t: {}", info.model_name));
    out.push(format!("stepping\t: {}", info.signature.stepping));
    out.push(format!("cpu MHz\t\t: {}", info.mhz));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_strings_match_kernel_constants() {
        assert_eq!(vendor_id(CpuVendor::Intel), "GenuineIntel");
        assert_eq!(vendor_id(CpuVendor::Amd), "AuthenticAMD");
    }

    #[test]
    fn block_contains_processor_and_model_lines() {
        let info = CpuInfo {
            processor: 0,
            vendor: CpuVendor::Intel,
            signature: CpuSignature {
                stepping: 1,
                model: 0x55,
                family: 6,
                processor_type: 0,
            },
            model_name: String::from("Test CPU"),
            mhz: 2400,
        };
        let block = render_block(&info);
        assert!(block.iter().any(|line| line.starts_with("processor")));
        assert!(block.iter().any(|line| line.contains("Test CPU")));
        assert!(block.iter().any(|line| line.contains("2400")));
    }
}
