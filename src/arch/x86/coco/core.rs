//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/core.c
//! test-origin: linux:vendor/linux/arch/x86/coco/core.c
//! Confidential Computing Platform Capability checks.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/core.c

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

pub const MSR_AMD64_SEV_ENABLED: u64 = 1 << 0;
pub const MSR_AMD64_SEV_ES_ENABLED: u64 = 1 << 1;
pub const MSR_AMD64_SEV_SNP_ENABLED: u64 = 1 << 2;
pub const MSR_AMD64_SNP_VTOM: u64 = 1 << 3;
pub const MSR_AMD64_SNP_SECURE_TSC: u64 = 1 << 11;
pub const MSR_AMD64_SNP_SECURE_AVIC: u64 = 1 << 18;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CcVendor {
    None = 0,
    Amd = 1,
    Intel = 2,
}

impl CcVendor {
    const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Amd,
            2 => Self::Intel,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CcAttr {
    MemEncrypt,
    HostMemEncrypt,
    GuestMemEncrypt,
    GuestStateEncrypt,
    GuestUnrollStringIo,
    GuestSevSnp,
    GuestSnpSecureTsc,
    HostSevSnp,
    SnpSecureAvic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CcPlatformState {
    pub vendor: CcVendor,
    pub cc_mask: u64,
    pub sev_status: u64,
    pub host_sev_snp: bool,
}

impl Default for CcPlatformState {
    fn default() -> Self {
        Self {
            vendor: CcVendor::None,
            cc_mask: 0,
            sev_status: 0,
            host_sev_snp: false,
        }
    }
}

static CC_VENDOR: AtomicU8 = AtomicU8::new(CcVendor::None as u8);
static CC_MASK: AtomicU64 = AtomicU64::new(0);
static SEV_STATUS: AtomicU64 = AtomicU64::new(0);
static HOST_SEV_SNP: AtomicBool = AtomicBool::new(false);

pub fn publish_cc_state(state: CcPlatformState) {
    CC_MASK.store(state.cc_mask, Ordering::Release);
    SEV_STATUS.store(state.sev_status, Ordering::Release);
    HOST_SEV_SNP.store(state.host_sev_snp, Ordering::Release);
    CC_VENDOR.store(state.vendor as u8, Ordering::Release);
}

pub fn cc_state() -> CcPlatformState {
    CcPlatformState {
        vendor: CcVendor::from_u8(CC_VENDOR.load(Ordering::Acquire)),
        cc_mask: CC_MASK.load(Ordering::Acquire),
        sev_status: SEV_STATUS.load(Ordering::Acquire),
        host_sev_snp: HOST_SEV_SNP.load(Ordering::Acquire),
    }
}

pub fn cc_platform_has(attr: CcAttr) -> bool {
    cc_platform_has_state(cc_state(), attr)
}

pub const fn cc_platform_has_state(state: CcPlatformState, attr: CcAttr) -> bool {
    match state.vendor {
        CcVendor::Intel => matches!(
            attr,
            CcAttr::GuestUnrollStringIo | CcAttr::GuestMemEncrypt | CcAttr::MemEncrypt
        ),
        CcVendor::Amd => amd_cc_platform_has(state, attr),
        CcVendor::None => false,
    }
}

const fn amd_cc_platform_has(state: CcPlatformState, attr: CcAttr) -> bool {
    if state.sev_status & MSR_AMD64_SNP_VTOM != 0 {
        return matches!(attr, CcAttr::GuestMemEncrypt | CcAttr::MemEncrypt);
    }

    match attr {
        CcAttr::MemEncrypt => state.cc_mask != 0,
        CcAttr::HostMemEncrypt => {
            state.cc_mask != 0 && state.sev_status & MSR_AMD64_SEV_ENABLED == 0
        }
        CcAttr::GuestMemEncrypt => state.sev_status & MSR_AMD64_SEV_ENABLED != 0,
        CcAttr::GuestStateEncrypt => state.sev_status & MSR_AMD64_SEV_ES_ENABLED != 0,
        CcAttr::GuestUnrollStringIo => {
            state.sev_status & MSR_AMD64_SEV_ENABLED != 0
                && state.sev_status & MSR_AMD64_SEV_ES_ENABLED == 0
        }
        CcAttr::GuestSevSnp => state.sev_status & MSR_AMD64_SEV_SNP_ENABLED != 0,
        CcAttr::GuestSnpSecureTsc => state.sev_status & MSR_AMD64_SNP_SECURE_TSC != 0,
        CcAttr::HostSevSnp => state.host_sev_snp,
        CcAttr::SnpSecureAvic => state.sev_status & MSR_AMD64_SNP_SECURE_AVIC != 0,
    }
}

pub fn cc_mkenc(val: u64) -> u64 {
    cc_mkenc_state(cc_state(), val)
}

pub const fn cc_mkenc_state(state: CcPlatformState, val: u64) -> u64 {
    match state.vendor {
        CcVendor::Amd if state.sev_status & MSR_AMD64_SNP_VTOM != 0 => val & !state.cc_mask,
        CcVendor::Amd => val | state.cc_mask,
        CcVendor::Intel => val & !state.cc_mask,
        CcVendor::None => val,
    }
}

pub fn cc_mkdec(val: u64) -> u64 {
    cc_mkdec_state(cc_state(), val)
}

pub const fn cc_mkdec_state(state: CcPlatformState, val: u64) -> u64 {
    match state.vendor {
        CcVendor::Amd if state.sev_status & MSR_AMD64_SNP_VTOM != 0 => val | state.cc_mask,
        CcVendor::Amd => val & !state.cc_mask,
        CcVendor::Intel => val | state.cc_mask,
        CcVendor::None => val,
    }
}

pub fn cc_platform_set(attr: CcAttr) {
    if matches!(cc_state().vendor, CcVendor::Amd) && matches!(attr, CcAttr::HostSevSnp) {
        HOST_SEV_SNP.store(true, Ordering::Release);
    }
}

pub fn cc_platform_clear(attr: CcAttr) {
    if matches!(cc_state().vendor, CcVendor::Amd) && matches!(attr, CcAttr::HostSevSnp) {
        HOST_SEV_SNP.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amd_sev_snp_attributes_follow_status_bits() {
        let state = CcPlatformState {
            vendor: CcVendor::Amd,
            cc_mask: 1 << 47,
            sev_status: MSR_AMD64_SEV_ENABLED
                | MSR_AMD64_SEV_ES_ENABLED
                | MSR_AMD64_SEV_SNP_ENABLED
                | MSR_AMD64_SNP_SECURE_TSC,
            host_sev_snp: false,
        };
        assert!(cc_platform_has_state(state, CcAttr::GuestMemEncrypt));
        assert!(cc_platform_has_state(state, CcAttr::GuestStateEncrypt));
        assert!(cc_platform_has_state(state, CcAttr::GuestSevSnp));
        assert!(cc_platform_has_state(state, CcAttr::GuestSnpSecureTsc));
        assert!(!cc_platform_has_state(state, CcAttr::GuestUnrollStringIo));
    }

    #[test]
    fn amd_and_intel_encrypt_mask_polarity_matches_linux_comment() {
        let amd = CcPlatformState {
            vendor: CcVendor::Amd,
            cc_mask: 0x8000,
            sev_status: 0,
            host_sev_snp: false,
        };
        assert_eq!(cc_mkenc_state(amd, 0x1000), 0x9000);
        assert_eq!(cc_mkdec_state(amd, 0x9000), 0x1000);

        let intel = CcPlatformState {
            vendor: CcVendor::Intel,
            cc_mask: 0x8000,
            sev_status: 0,
            host_sev_snp: false,
        };
        assert_eq!(cc_mkenc_state(intel, 0x9000), 0x1000);
        assert_eq!(cc_mkdec_state(intel, 0x1000), 0x9000);
    }

    #[test]
    fn amd_vtom_reverses_amd_cbit_polarity() {
        let state = CcPlatformState {
            vendor: CcVendor::Amd,
            cc_mask: 0x4000,
            sev_status: MSR_AMD64_SNP_VTOM,
            host_sev_snp: false,
        };
        assert_eq!(cc_mkenc_state(state, 0x5000), 0x1000);
        assert_eq!(cc_mkdec_state(state, 0x1000), 0x5000);
        assert!(cc_platform_has_state(state, CcAttr::GuestMemEncrypt));
        assert!(!cc_platform_has_state(state, CcAttr::GuestSevSnp));
    }
}
