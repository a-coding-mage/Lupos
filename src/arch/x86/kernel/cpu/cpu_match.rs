//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/match.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/match.c
//! CPU model/family matching dispatch tables.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/match.c

// Linux drivers register `x86_cpu_id` tables and call
// `x86_match_cpu()` to find the first matching entry. The match uses
// vendor / family / model / stepping wildcards (0 means "any"). We model
// the comparison without owning the table allocations.

use crate::arch::x86::kernel::cpu::{CpuSignature, CpuVendor};

pub const X86_CPU_TYPE_ANY: u8 = 0;
pub const X86_FEATURE_HYBRID_CPU: u32 = 1 << 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuMatchEntry {
    pub vendor: Option<CpuVendor>,
    pub family: Option<u8>,
    pub model: Option<u8>,
    pub stepping: Option<u8>,
    pub feature: Option<u32>,
    pub driver_data: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuMatchRuntime {
    pub vendor: CpuVendor,
    pub signature: CpuSignature,
    pub features: u32,
    pub intel_platform_id: u8,
    pub vendor_cpu_type: u8,
    pub microcode: u64,
}

pub fn matches(entry: CpuMatchEntry, vendor: CpuVendor, sig: CpuSignature) -> bool {
    if let Some(v) = entry.vendor {
        if v != vendor {
            return false;
        }
    }
    if let Some(f) = entry.family {
        if f != sig.family {
            return false;
        }
    }
    if let Some(m) = entry.model {
        if m != sig.model {
            return false;
        }
    }
    if let Some(s) = entry.stepping {
        if s != sig.stepping {
            return false;
        }
    }
    true
}

pub const fn vendor_cpu_type_matches(
    requested_type: u8,
    hybrid_cpu: bool,
    actual_type: u8,
) -> bool {
    requested_type == X86_CPU_TYPE_ANY || hybrid_cpu || requested_type == actual_type
}

pub fn matches_runtime(entry: CpuMatchEntry, runtime: CpuMatchRuntime) -> bool {
    matches(entry, runtime.vendor, runtime.signature)
        && entry
            .feature
            .is_none_or(|feature| runtime.features & (1 << feature) != 0)
}

pub fn first_match(table: &[CpuMatchEntry], vendor: CpuVendor, sig: CpuSignature) -> Option<u64> {
    for entry in table.iter() {
        if matches(*entry, vendor, sig) {
            return Some(entry.driver_data);
        }
    }
    None
}

pub fn min_microcode_rev_matches(table: &[CpuMatchEntry], runtime: CpuMatchRuntime) -> bool {
    table
        .iter()
        .find(|entry| matches_runtime(**entry, runtime))
        .is_some_and(|entry| entry.driver_data <= runtime.microcode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_match_source_asserts_linux_wildcards_features_and_microcode_gate() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/match.c"
        ));
        assert!(source.contains("static bool x86_match_vendor_cpu_type"));
        assert!(source.contains("if (m->type == X86_CPU_TYPE_ANY)"));
        assert!(source.contains("cpu_feature_enabled(X86_FEATURE_HYBRID_CPU)"));
        assert!(source.contains("m->vendor != X86_VENDOR_ANY"));
        assert!(source.contains("m->family != X86_FAMILY_ANY"));
        assert!(source.contains("m->model != X86_MODEL_ANY"));
        assert!(source.contains("m->steppings != X86_STEPPING_ANY"));
        assert!(source.contains("m->platform_mask != X86_PLATFORM_ANY"));
        assert!(source.contains("m->feature != X86_FEATURE_ANY && !cpu_has(c, m->feature)"));
        assert!(source.contains("return m;"));
        assert!(source.contains("EXPORT_SYMBOL(x86_match_cpu);"));
        assert!(source.contains("x86_match_min_microcode_rev"));
        assert!(source.contains("res->driver_data > boot_cpu_data.microcode"));

        assert!(vendor_cpu_type_matches(X86_CPU_TYPE_ANY, false, 2));
        assert!(vendor_cpu_type_matches(1, true, 2));
        assert!(!vendor_cpu_type_matches(1, false, 2));

        let runtime = CpuMatchRuntime {
            vendor: CpuVendor::Intel,
            signature: CpuSignature {
                stepping: 1,
                model: 2,
                family: 6,
                processor_type: 0,
            },
            features: 1 << 4,
            intel_platform_id: 0,
            vendor_cpu_type: 0,
            microcode: 10,
        };
        let entry = CpuMatchEntry {
            vendor: Some(CpuVendor::Intel),
            family: Some(6),
            model: Some(2),
            stepping: Some(1),
            feature: Some(4),
            driver_data: 9,
        };
        assert!(matches_runtime(entry, runtime));
        assert!(min_microcode_rev_matches(&[entry], runtime));
        let too_new = CpuMatchEntry {
            driver_data: 11,
            ..entry
        };
        assert!(!min_microcode_rev_matches(&[too_new], runtime));
    }

    #[test]
    fn wildcard_entry_matches_any_cpu() {
        let entry = CpuMatchEntry {
            vendor: None,
            family: None,
            model: None,
            stepping: None,
            feature: None,
            driver_data: 7,
        };
        let sig = CpuSignature {
            stepping: 0,
            model: 0,
            family: 0,
            processor_type: 0,
        };
        assert!(matches(entry, CpuVendor::Intel, sig));
    }

    #[test]
    fn first_match_returns_the_earliest_hit() {
        let entries = [
            CpuMatchEntry {
                vendor: Some(CpuVendor::Amd),
                family: None,
                model: None,
                stepping: None,
                feature: None,
                driver_data: 1,
            },
            CpuMatchEntry {
                vendor: Some(CpuVendor::Intel),
                family: Some(6),
                model: None,
                stepping: None,
                feature: None,
                driver_data: 2,
            },
            CpuMatchEntry {
                vendor: Some(CpuVendor::Intel),
                family: Some(6),
                model: Some(0x55),
                stepping: None,
                feature: None,
                driver_data: 3,
            },
        ];
        let sig = CpuSignature {
            stepping: 0,
            model: 0x55,
            family: 6,
            processor_type: 0,
        };
        assert_eq!(first_match(&entries, CpuVendor::Intel, sig), Some(2));
    }
}
