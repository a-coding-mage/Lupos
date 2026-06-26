//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/hyperv/hv_init.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/hv_init.c
//! Hyper-V CPUID/MSR initialization model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/hv_init.c

use crate::include::uapi::errno::EOPNOTSUPP;

pub const HYPERV_CPUID_VENDOR_AND_MAX_FUNCTIONS: u32 = 0x4000_0000;
pub const HYPERV_CPUID_INTERFACE: u32 = 0x4000_0001;
pub const HYPERV_CPUID_FEATURES: u32 = 0x4000_0003;
pub const HYPERV_CPUID_ENLIGHTMENT_INFO: u32 = 0x4000_0004;
pub const HYPERV_CPUID_IMPLEMENT_LIMITS: u32 = 0x4000_0005;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HypervFeatures {
    pub vp_runtime: bool,
    pub time_ref_count: bool,
    pub synic: bool,
    pub synthetic_timer: bool,
    pub apic_access: bool,
    pub hypercall: bool,
    pub vp_index: bool,
    pub reset: bool,
}

impl HypervFeatures {
    pub const fn from_feature_eax(eax: u32) -> Self {
        Self {
            vp_runtime: eax & (1 << 0) != 0,
            time_ref_count: eax & (1 << 1) != 0,
            synic: eax & (1 << 2) != 0,
            synthetic_timer: eax & (1 << 3) != 0,
            apic_access: eax & (1 << 4) != 0,
            hypercall: eax & (1 << 5) != 0,
            vp_index: eax & (1 << 6) != 0,
            reset: eax & (1 << 7) != 0,
        }
    }
}

pub const fn is_hyperv_vendor(vendor: [u8; 12]) -> bool {
    bytes_eq12(vendor, *b"Microsoft Hv")
}

pub const fn hyperv_initialized(vendor: [u8; 12], max_leaf: u32) -> bool {
    is_hyperv_vendor(vendor) && max_leaf >= HYPERV_CPUID_FEATURES
}

pub const fn hyperv_programming_errno() -> i32 {
    EOPNOTSUPP
}

const fn bytes_eq12(a: [u8; 12], b: [u8; 12]) -> bool {
    let mut i = 0;
    while i < 12 {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_leaf_decodes_documented_low_bits() {
        let f = HypervFeatures::from_feature_eax(0b1110_0101);
        assert!(f.vp_runtime);
        assert!(f.synic);
        assert!(f.hypercall);
        assert!(f.reset);
        assert!(!f.time_ref_count);
    }
}
