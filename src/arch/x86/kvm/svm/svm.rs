//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/svm/svm.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/svm/svm.c
//! AMD SVM feature and intercept model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/svm/svm.c

use crate::include::uapi::errno::{EINVAL, ENODEV};

pub const EFER_SVME: u64 = 1 << 12;
pub const SVM_FEATURE_NPT: u32 = 1 << 0;
pub const SVM_FEATURE_VGIF: u32 = 1 << 16;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SvmHostFeatures {
    pub efer: u64,
    pub cpuid_edx: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SvmIntercept {
    CrRead(u8),
    CrWrite(u8),
    Msr,
    Io,
    Hlt,
}

pub const fn svm_available(host: SvmHostFeatures) -> bool {
    host.efer & EFER_SVME != 0
}

pub const fn svm_has_npt(host: SvmHostFeatures) -> bool {
    host.cpuid_edx & SVM_FEATURE_NPT != 0
}

pub const fn svm_has_vgif(host: SvmHostFeatures) -> bool {
    host.cpuid_edx & SVM_FEATURE_VGIF != 0
}

pub const fn validate_svm_host(host: SvmHostFeatures) -> Result<(), i32> {
    if !svm_available(host) {
        Err(ENODEV)
    } else if svm_has_vgif(host) && !svm_has_npt(host) {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub const fn intercept_bit(intercept: SvmIntercept) -> u64 {
    match intercept {
        SvmIntercept::CrRead(reg) => 1u64 << reg,
        SvmIntercept::CrWrite(reg) => 1u64 << (16 + reg),
        SvmIntercept::Msr => 1u64 << 28,
        SvmIntercept::Io => 1u64 << 30,
        SvmIntercept::Hlt => 1u64 << 32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svm_requires_svme_in_efer() {
        assert_eq!(validate_svm_host(SvmHostFeatures::default()), Err(ENODEV));
        assert!(
            validate_svm_host(SvmHostFeatures {
                efer: EFER_SVME,
                cpuid_edx: SVM_FEATURE_NPT
            })
            .is_ok()
        );
    }

    #[test]
    fn intercept_bits_keep_cr_read_write_ranges_distinct() {
        assert_eq!(intercept_bit(SvmIntercept::CrRead(3)), 1 << 3);
        assert_eq!(intercept_bit(SvmIntercept::CrWrite(3)), 1 << 19);
    }
}
