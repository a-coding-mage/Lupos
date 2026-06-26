//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu
//! x86 CPU identity and feature decoding.
//!
//! Linux builds `boot_cpu_data` from CPUID leaves and then lets vendor files
//! apply quirks. Lupos keeps the same split at a smaller scale: raw CPUID lives
//! in `cpuid.rs`, while this module gives the rest of the kernel named feature
//! predicates and vendor/model decoding.
//!
//! References:
//! - `vendor/linux/arch/x86/kernel/cpu/common.c`
//! - `vendor/linux/arch/x86/kernel/cpuid.c`
//! - `vendor/linux/arch/x86/kernel/cpu/intel.c`
//! - `vendor/linux/arch/x86/kernel/cpu/amd.c`
//! - `vendor/linux/arch/x86/kernel/cpu/hygon.c`
//! - `vendor/linux/arch/x86/kernel/cpu/zhaoxin.c`
//! - `vendor/linux/arch/x86/kernel/cpu/centaur.c`

pub mod acrn;
pub mod amd_cache_disable;
pub mod aperfmperf;
pub mod bhyve;
pub mod bugs;
pub mod bus_lock;
pub mod cacheinfo;
pub mod common;
pub mod cpu_match;
pub mod cpuid_0x2_table;
pub mod cpuid_deps;
pub mod cyrix;
pub mod debugfs;
pub mod feat_ctl;
pub mod hypervisor;
pub mod intel_epb;
pub mod mce;
pub mod microcode;
pub mod mtrr;
pub mod perfctr_watchdog;
pub mod powerflags;
pub mod proc;
pub mod rdrand;
pub mod resctrl;
pub mod scattered;
pub mod sgx;
pub mod topology;
pub mod topology_amd;
pub mod topology_common;
pub mod topology_ext;
pub mod transmeta;
pub mod tsx;
pub mod umc;
pub mod umwait;
pub mod vortex;
pub mod zhaoxin;

use super::cpuid::{CpuidResult, cpuid, vendor_string};
use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuVendor {
    Intel,
    Amd,
    Hygon,
    Zhaoxin,
    Centaur,
    Unknown([u8; 12]),
}

impl CpuVendor {
    pub fn current() -> Self {
        Self::from_bytes(vendor_string())
    }

    pub const fn from_bytes(bytes: [u8; 12]) -> Self {
        if bytes_eq(bytes, *b"GenuineIntel") {
            Self::Intel
        } else if bytes_eq(bytes, *b"AuthenticAMD") {
            Self::Amd
        } else if bytes_eq(bytes, *b"HygonGenuine") {
            Self::Hygon
        } else if bytes_eq(bytes, *b"  Shanghai  ") {
            Self::Zhaoxin
        } else if bytes_eq(bytes, *b"CentaurHauls") {
            Self::Centaur
        } else {
            Self::Unknown(bytes)
        }
    }
}

const fn bytes_eq(a: [u8; 12], b: [u8; 12]) -> bool {
    let mut i = 0;
    while i < 12 {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuSignature {
    pub stepping: u8,
    pub model: u8,
    pub family: u8,
    pub processor_type: u8,
}

impl CpuSignature {
    pub const fn from_leaf1_eax(eax: u32) -> Self {
        let stepping = (eax & 0x0f) as u8;
        let base_model = ((eax >> 4) & 0x0f) as u8;
        let base_family = ((eax >> 8) & 0x0f) as u8;
        let processor_type = ((eax >> 12) & 0x03) as u8;
        let ext_model = ((eax >> 16) & 0x0f) as u8;
        let ext_family = ((eax >> 20) & 0xff) as u8;
        let family = if base_family == 0x0f {
            base_family.wrapping_add(ext_family)
        } else {
            base_family
        };
        let model = if base_family == 0x06 || base_family == 0x0f {
            base_model.wrapping_add(ext_model << 4)
        } else {
            base_model
        };
        Self {
            stepping,
            model,
            family,
            processor_type,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuFeatures {
    pub leaf1_ecx: u32,
    pub leaf1_edx: u32,
    pub leaf7_ebx: u32,
    pub leaf7_ecx: u32,
    pub ext_edx: u32,
}

impl CpuFeatures {
    pub fn current() -> Self {
        let leaf1 = cpuid(1, 0);
        let leaf7 = cpuid(7, 0);
        let ext = cpuid(0x8000_0001, 0);
        Self::from_cpuid(leaf1, leaf7, ext)
    }

    pub const fn from_cpuid(leaf1: CpuidResult, leaf7: CpuidResult, ext: CpuidResult) -> Self {
        Self {
            leaf1_ecx: leaf1.ecx,
            leaf1_edx: leaf1.edx,
            leaf7_ebx: leaf7.ebx,
            leaf7_ecx: leaf7.ecx,
            ext_edx: ext.edx,
        }
    }

    pub const fn has_apic(self) -> bool {
        self.leaf1_edx & (1 << 9) != 0
    }

    pub const fn has_tsc(self) -> bool {
        self.leaf1_edx & (1 << 4) != 0
    }

    pub const fn has_msr(self) -> bool {
        self.leaf1_edx & (1 << 5) != 0
    }

    pub const fn has_mce(self) -> bool {
        self.leaf1_edx & (1 << 7) != 0
    }

    pub const fn has_mca(self) -> bool {
        self.leaf1_edx & (1 << 14) != 0
    }

    pub const fn has_pat(self) -> bool {
        self.leaf1_edx & (1 << 16) != 0
    }

    pub const fn has_fxsr(self) -> bool {
        self.leaf1_edx & (1 << 24) != 0
    }

    pub const fn has_sse(self) -> bool {
        self.leaf1_edx & (1 << 25) != 0
    }

    pub const fn has_sse2(self) -> bool {
        self.leaf1_edx & (1 << 26) != 0
    }

    pub const fn has_x2apic(self) -> bool {
        self.leaf1_ecx & (1 << 21) != 0
    }

    pub const fn has_xsave(self) -> bool {
        self.leaf1_ecx & (1 << 26) != 0
    }

    pub const fn has_osxsave(self) -> bool {
        self.leaf1_ecx & (1 << 27) != 0
    }

    pub const fn has_avx(self) -> bool {
        self.leaf1_ecx & (1 << 28) != 0
    }

    pub const fn has_hypervisor(self) -> bool {
        self.leaf1_ecx & (1 << 31) != 0
    }

    pub const fn has_syscall(self) -> bool {
        self.ext_edx & (1 << 11) != 0
    }

    pub const fn has_nx(self) -> bool {
        self.ext_edx & (1 << 20) != 0
    }

    pub const fn has_long_mode(self) -> bool {
        self.ext_edx & (1 << 29) != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuOptionalFacility {
    MachineCheck,
    MicrocodeUpdate,
    ResourceControl,
    SoftwareGuardExtensions,
    BusLockDetect,
    Umwait,
    TscDeadlinePerfWatchdog,
}

pub const fn optional_facility_enabled(_facility: CpuOptionalFacility) -> bool {
    false
}

pub const fn optional_facility_errno(facility: CpuOptionalFacility) -> i32 {
    match facility {
        CpuOptionalFacility::MachineCheck | CpuOptionalFacility::MicrocodeUpdate => ENODEV,
        CpuOptionalFacility::ResourceControl
        | CpuOptionalFacility::SoftwareGuardExtensions
        | CpuOptionalFacility::BusLockDetect
        | CpuOptionalFacility::Umwait
        | CpuOptionalFacility::TscDeadlinePerfWatchdog => EOPNOTSUPP,
    }
}

fn vendor_label(vendor: CpuVendor) -> &'static str {
    match vendor {
        CpuVendor::Intel => "GenuineIntel",
        CpuVendor::Amd => "AuthenticAMD",
        CpuVendor::Hygon => "HygonGenuine",
        CpuVendor::Zhaoxin => "  Shanghai  ",
        CpuVendor::Centaur => "CentaurHauls",
        CpuVendor::Unknown(_) => "Unknown",
    }
}

fn trim_brand(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let raw = core::str::from_utf8(&bytes[..end]).unwrap_or("");
    raw.trim()
}

/// Print the canonical Linux `CPU: ...` boot line.
///
/// Linux: `vendor/linux/arch/x86/kernel/cpu/common.c` — `print_cpu_info()`:
///   pr_cont("%s %s", vendor, c->x86_model_id);
///   pr_cont(" (family: 0x%x, model: 0x%x, stepping: 0x%x)\n",
///           c->x86, c->x86_model, c->x86_stepping);
pub fn print_cpu_info() {
    let vendor = CpuVendor::current();
    let signature = CpuSignature::from_leaf1_eax(cpuid(1, 0).eax);
    let brand_bytes = super::cpuid::brand_string();
    let brand = trim_brand(&brand_bytes);
    crate::log_info!(
        "",
        "CPU: {} {} (family: {:#x}, model: {:#x}, stepping: {:#x})",
        vendor_label(vendor),
        brand,
        signature.family,
        signature.model,
        signature.stepping,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_decoder_recognizes_linux_vendor_strings() {
        assert_eq!(CpuVendor::from_bytes(*b"GenuineIntel"), CpuVendor::Intel);
        assert_eq!(CpuVendor::from_bytes(*b"AuthenticAMD"), CpuVendor::Amd);
        assert_eq!(CpuVendor::from_bytes(*b"HygonGenuine"), CpuVendor::Hygon);
        assert_eq!(CpuVendor::from_bytes(*b"CentaurHauls"), CpuVendor::Centaur);
    }

    #[test]
    fn cpu_signature_applies_extended_family_and_model_rules() {
        let sig = CpuSignature::from_leaf1_eax(0x0003_06a9);
        assert_eq!(sig.family, 0x06);
        assert_eq!(sig.model, 0x3a);
        assert_eq!(sig.stepping, 0x09);

        let sig = CpuSignature::from_leaf1_eax(0x0010_0f10);
        assert_eq!(sig.family, 0x10);
        assert_eq!(sig.model, 0x01);
    }

    #[test]
    fn feature_predicates_use_linux_cpuid_bit_positions() {
        let leaf1 = CpuidResult {
            eax: 0,
            ebx: 0,
            ecx: (1 << 21) | (1 << 26) | (1 << 27) | (1 << 28),
            edx: (1 << 4)
                | (1 << 5)
                | (1 << 7)
                | (1 << 9)
                | (1 << 14)
                | (1 << 16)
                | (1 << 24)
                | (1 << 25)
                | (1 << 26),
        };
        let ext = CpuidResult {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: (1 << 11) | (1 << 20) | (1 << 29),
        };
        let f = CpuFeatures::from_cpuid(
            leaf1,
            CpuidResult {
                eax: 0,
                ebx: 0,
                ecx: 0,
                edx: 0,
            },
            ext,
        );
        assert!(f.has_apic());
        assert!(f.has_tsc());
        assert!(f.has_msr());
        assert!(f.has_mce());
        assert!(f.has_mca());
        assert!(f.has_pat());
        assert!(f.has_fxsr());
        assert!(f.has_sse());
        assert!(f.has_sse2());
        assert!(f.has_x2apic());
        assert!(f.has_xsave());
        assert!(f.has_osxsave());
        assert!(f.has_avx());
        assert!(f.has_syscall());
        assert!(f.has_nx());
        assert!(f.has_long_mode());
    }

    #[test]
    fn optional_cpu_facilities_fail_closed_until_subsystems_exist() {
        assert!(!optional_facility_enabled(
            CpuOptionalFacility::MachineCheck
        ));
        assert_eq!(
            optional_facility_errno(CpuOptionalFacility::MicrocodeUpdate),
            ENODEV
        );
        assert_eq!(
            optional_facility_errno(CpuOptionalFacility::SoftwareGuardExtensions),
            EOPNOTSUPP
        );
    }
}
