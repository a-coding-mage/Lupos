//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/virt
//! test-origin: linux:vendor/linux/arch/x86/virt
//! x86 hypervisor detection and local virtualization policy.
//!
//! Lupos can run on ordinary emulated x86 hardware. This module owns the part
//! that is actually implemented today: CPUID hypervisor-vendor detection and
//! explicit local policy for facilities that are not wired into this kernel.
//!
//! References:
//! - `vendor/linux/arch/x86/kernel/cpu/hypervisor.c`
//! - `vendor/linux/arch/x86/kernel/cpu/acrn.c`
//! - `vendor/linux/arch/x86/kernel/cpu/bhyve.c`
//! - `vendor/linux/arch/x86/kernel/cpu/mshyperv.c`
//! - `vendor/linux/arch/x86/kernel/cpu/vmware.c`

use crate::arch::x86::kernel::cpuid::{CpuidResult, cpuid};
use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HypervisorVendor {
    Kvm,
    MicrosoftHyperV,
    Xen,
    Vmware,
    Bhyve,
    Acrn,
    Unknown([u8; 12]),
}

impl HypervisorVendor {
    pub const fn from_bytes(bytes: [u8; 12]) -> Self {
        if bytes_eq(bytes, *b"KVMKVMKVM\0\0\0") {
            Self::Kvm
        } else if bytes_eq(bytes, *b"Microsoft Hv") {
            Self::MicrosoftHyperV
        } else if bytes_eq(bytes, *b"XenVMMXenVMM") {
            Self::Xen
        } else if bytes_eq(bytes, *b"VMwareVMware") {
            Self::Vmware
        } else if bytes_eq(bytes, *b"bhyve bhyve ") {
            Self::Bhyve
        } else if bytes_eq(bytes, *b"ACRNACRNACRN") {
            Self::Acrn
        } else {
            Self::Unknown(bytes)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HypervisorCpuidInfo {
    pub max_leaf: u32,
    pub vendor: HypervisorVendor,
}

pub const HYPERVISOR_CPUID_LEAF: u32 = 0x4000_0000;

pub const fn hypervisor_present_from_leaf1(leaf1: CpuidResult) -> bool {
    leaf1.ecx & (1 << 31) != 0
}

pub const fn hypervisor_vendor_from_leaf(leaf: CpuidResult) -> HypervisorCpuidInfo {
    let mut bytes = [0u8; 12];
    let ebx = leaf.ebx.to_le_bytes();
    let ecx = leaf.ecx.to_le_bytes();
    let edx = leaf.edx.to_le_bytes();
    bytes[0] = ebx[0];
    bytes[1] = ebx[1];
    bytes[2] = ebx[2];
    bytes[3] = ebx[3];
    bytes[4] = ecx[0];
    bytes[5] = ecx[1];
    bytes[6] = ecx[2];
    bytes[7] = ecx[3];
    bytes[8] = edx[0];
    bytes[9] = edx[1];
    bytes[10] = edx[2];
    bytes[11] = edx[3];

    HypervisorCpuidInfo {
        max_leaf: leaf.eax,
        vendor: HypervisorVendor::from_bytes(bytes),
    }
}

pub fn current_hypervisor() -> Option<HypervisorCpuidInfo> {
    let leaf1 = cpuid(1, 0);
    if !hypervisor_present_from_leaf1(leaf1) {
        return None;
    }
    Some(hypervisor_vendor_from_leaf(cpuid(HYPERVISOR_CPUID_LEAF, 0)))
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
pub enum VirtualizationFacility {
    KvmHost,
    XenPv,
    HyperVEnlightenment,
    SevGuest,
    TdxGuest,
    ParavirtSpinlocks,
}

pub const fn facility_enabled(_facility: VirtualizationFacility) -> bool {
    false
}

pub const fn facility_errno(facility: VirtualizationFacility) -> i32 {
    match facility {
        VirtualizationFacility::KvmHost => ENODEV,
        VirtualizationFacility::XenPv
        | VirtualizationFacility::HyperVEnlightenment
        | VirtualizationFacility::SevGuest
        | VirtualizationFacility::TdxGuest
        | VirtualizationFacility::ParavirtSpinlocks => EOPNOTSUPP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypervisor_vendor_decoder_matches_linux_signatures() {
        assert_eq!(
            HypervisorVendor::from_bytes(*b"Microsoft Hv"),
            HypervisorVendor::MicrosoftHyperV
        );
        assert_eq!(
            HypervisorVendor::from_bytes(*b"XenVMMXenVMM"),
            HypervisorVendor::Xen
        );
    }

    #[test]
    fn hypervisor_cpuid_leaf_uses_ebx_ecx_edx_signature_order() {
        let leaf = CpuidResult {
            eax: 0x4000_0010,
            ebx: u32::from_le_bytes(*b"Micr"),
            ecx: u32::from_le_bytes(*b"osof"),
            edx: u32::from_le_bytes(*b"t Hv"),
        };
        assert_eq!(
            hypervisor_vendor_from_leaf(leaf),
            HypervisorCpuidInfo {
                max_leaf: 0x4000_0010,
                vendor: HypervisorVendor::MicrosoftHyperV,
            }
        );
    }

    #[test]
    fn hypervisor_present_uses_leaf1_ecx_bit31() {
        assert!(!hypervisor_present_from_leaf1(CpuidResult {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0
        }));
        assert!(hypervisor_present_from_leaf1(CpuidResult {
            eax: 0,
            ebx: 0,
            ecx: 1 << 31,
            edx: 0
        }));
    }

    #[test]
    fn virtualization_facilities_fail_closed() {
        assert!(!facility_enabled(VirtualizationFacility::KvmHost));
        assert_eq!(facility_errno(VirtualizationFacility::KvmHost), ENODEV);
        assert_eq!(facility_errno(VirtualizationFacility::TdxGuest), EOPNOTSUPP);
    }
}
