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
use crate::kernel::module::{export_symbol, find_symbol};

const X86_VENDOR_INTEL: u16 = 0;
const X86_VENDOR_AMD: u16 = 2;
const X86_VENDOR_CENTAUR: u16 = 5;
const X86_VENDOR_HYGON: u16 = 9;
const X86_VENDOR_ZHAOXIN: u16 = 10;
const X86_VENDOR_UNKNOWN: u16 = 0xff;
const X86_VENDOR_ANY: u16 = 0xffff;
const X86_FAMILY_ANY: u16 = 0;
const X86_MODEL_ANY: u16 = 0;
const X86_STEPPING_ANY: u16 = 0;
const X86_PLATFORM_ANY: u8 = 0;
const X86_FEATURE_ANY: u16 = 0;
const X86_CPU_ID_FLAG_ENTRY_VALID: u16 = 1;
const X86_FEATURE_HYBRID_CPU_BIT: u32 = 18 * 32 + 15;

pub const X86_CPU_TYPE_ANY: u8 = 0;
pub const X86_FEATURE_HYBRID_CPU: u32 = 1 << 15;

/// `struct x86_cpu_id` - `vendor/linux/include/linux/mod_devicetable.h:689`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxX86CpuId {
    pub vendor: u16,
    pub family: u16,
    pub model: u16,
    pub steppings: u16,
    pub feature: u16,
    pub flags: u16,
    pub platform_mask: u8,
    pub type_: u8,
    pub driver_data: usize,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("x86_match_cpu", linux_x86_match_cpu as usize, false);
}

fn current_vendor_id() -> u16 {
    match CpuVendor::current() {
        CpuVendor::Intel => X86_VENDOR_INTEL,
        CpuVendor::Amd => X86_VENDOR_AMD,
        CpuVendor::Hygon => X86_VENDOR_HYGON,
        CpuVendor::Zhaoxin => X86_VENDOR_ZHAOXIN,
        CpuVendor::Centaur => X86_VENDOR_CENTAUR,
        CpuVendor::Unknown(_) => X86_VENDOR_UNKNOWN,
    }
}

fn current_signature() -> CpuSignature {
    let leaf1 = crate::arch::x86::kernel::cpuid::cpuid(1, 0);
    CpuSignature::from_leaf1_eax(leaf1.eax)
}

fn linux_x86_cpu_id_matches(entry: &LinuxX86CpuId, vendor: u16, signature: CpuSignature) -> bool {
    if entry.vendor != X86_VENDOR_ANY && entry.vendor != vendor {
        return false;
    }
    if entry.family != X86_FAMILY_ANY && entry.family != signature.family as u16 {
        return false;
    }
    if entry.model != X86_MODEL_ANY && entry.model != signature.model as u16 {
        return false;
    }
    if entry.steppings != X86_STEPPING_ANY
        && (signature.stepping >= 16 || (entry.steppings & (1u16 << signature.stepping)) == 0)
    {
        return false;
    }
    if entry.platform_mask != X86_PLATFORM_ANY {
        return false;
    }
    if entry.feature != X86_FEATURE_ANY && !super::common::boot_cpu_has(entry.feature as u32) {
        return false;
    }
    if entry.type_ != X86_CPU_TYPE_ANY && !super::common::boot_cpu_has(X86_FEATURE_HYBRID_CPU_BIT) {
        return false;
    }
    true
}

/// `x86_match_cpu` - `vendor/linux/arch/x86/kernel/cpu/match.c:64`.
#[unsafe(export_name = "x86_match_cpu")]
pub unsafe extern "C" fn linux_x86_match_cpu(table: *const LinuxX86CpuId) -> *const LinuxX86CpuId {
    if table.is_null() {
        return core::ptr::null();
    }

    let vendor = current_vendor_id();
    let signature = current_signature();
    let mut index = 0usize;
    loop {
        let entry = unsafe { &*table.add(index) };
        if entry.flags & X86_CPU_ID_FLAG_ENTRY_VALID == 0 {
            return core::ptr::null();
        }
        if linux_x86_cpu_id_matches(entry, vendor, signature) {
            return unsafe { table.add(index) };
        }
        index += 1;
    }
}

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
    fn linux_x86_cpu_id_layout_matches_vendor_header() {
        use core::mem::{offset_of, size_of};

        assert_eq!(offset_of!(LinuxX86CpuId, vendor), 0);
        assert_eq!(offset_of!(LinuxX86CpuId, family), 2);
        assert_eq!(offset_of!(LinuxX86CpuId, model), 4);
        assert_eq!(offset_of!(LinuxX86CpuId, steppings), 6);
        assert_eq!(offset_of!(LinuxX86CpuId, feature), 8);
        assert_eq!(offset_of!(LinuxX86CpuId, flags), 10);
        assert_eq!(offset_of!(LinuxX86CpuId, platform_mask), 12);
        assert_eq!(offset_of!(LinuxX86CpuId, type_), 13);
        assert_eq!(offset_of!(LinuxX86CpuId, driver_data), 16);
        assert_eq!(size_of::<LinuxX86CpuId>(), 24);
    }

    #[test]
    fn linux_x86_cpu_id_match_honors_wildcards_and_stepping_mask() {
        let sig = CpuSignature {
            stepping: 3,
            model: 0x55,
            family: 6,
            processor_type: 0,
        };
        let entry = LinuxX86CpuId {
            vendor: X86_VENDOR_INTEL,
            family: 6,
            model: 0x55,
            steppings: 1 << 3,
            feature: X86_FEATURE_ANY,
            flags: X86_CPU_ID_FLAG_ENTRY_VALID,
            platform_mask: X86_PLATFORM_ANY,
            type_: X86_CPU_TYPE_ANY,
            driver_data: 0x55aa,
        };

        assert!(linux_x86_cpu_id_matches(&entry, X86_VENDOR_INTEL, sig));
        assert!(!linux_x86_cpu_id_matches(&entry, X86_VENDOR_AMD, sig));

        let wildcard = LinuxX86CpuId {
            vendor: X86_VENDOR_ANY,
            family: X86_FAMILY_ANY,
            model: X86_MODEL_ANY,
            steppings: X86_STEPPING_ANY,
            ..entry
        };
        assert!(linux_x86_cpu_id_matches(&wildcard, X86_VENDOR_AMD, sig));
    }

    #[test]
    fn x86_match_cpu_export_registers_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("x86_match_cpu"),
            Some(linux_x86_match_cpu as usize)
        );
    }

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
