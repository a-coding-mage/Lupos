//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/hyperv_evmcs.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/hyperv_evmcs.c
//! Hyper-V enlightened VMCS compatibility.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/hyperv_evmcs.c

pub const EVMCS_VERSION_MIN: u16 = 1;
pub const EVMCS_VERSION_MAX: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnlightenedVmcsVersion {
    pub min: u16,
    pub max: u16,
}

pub const fn evmcs_version_supported(version: EnlightenedVmcsVersion) -> bool {
    version.min <= EVMCS_VERSION_MAX && version.max >= EVMCS_VERSION_MIN
}

pub const fn evmcs_can_skip_vmread(clean_fields: u64, field_bit: u8) -> bool {
    field_bit < 64 && (clean_fields & (1u64 << field_bit)) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_one_overlaps_supported_range() {
        assert!(evmcs_version_supported(EnlightenedVmcsVersion {
            min: 1,
            max: 1
        }));
        assert!(!evmcs_version_supported(EnlightenedVmcsVersion {
            min: 2,
            max: 3
        }));
    }
}
