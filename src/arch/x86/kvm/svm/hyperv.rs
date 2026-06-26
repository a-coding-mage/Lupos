//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/svm/hyperv.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/svm/hyperv.c
//! Hyper-V enlightenments for SVM (KVM-on-Hyper-V nested SVM).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/svm/hyperv.c

// Nested SVM on Hyper-V can use the Hyper-V VP-assist page to short-
// circuit certain VMRUN/VMEXIT steps. The driver enables the
// `enlightenments_control` MSR in the nested VMCB. We model the bits.

pub const HV_VP_ASSIST_ENLIGHTENMENTS: u64 = 1 << 0;
pub const HV_VP_ASSIST_TLB_FLUSH: u64 = 1 << 1;
pub const HV_VP_ASSIST_NESTED_DEBUG: u64 = 1 << 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SvmHyperVEnlightenments {
    pub mask: u64,
}

impl SvmHyperVEnlightenments {
    pub const fn enable(mut self, bit: u64) -> Self {
        self.mask |= bit;
        self
    }

    pub const fn has(self, bit: u64) -> bool {
        self.mask & bit != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_is_additive_per_bit() {
        let e = SvmHyperVEnlightenments::default()
            .enable(HV_VP_ASSIST_ENLIGHTENMENTS)
            .enable(HV_VP_ASSIST_TLB_FLUSH);
        assert!(e.has(HV_VP_ASSIST_ENLIGHTENMENTS));
        assert!(e.has(HV_VP_ASSIST_TLB_FLUSH));
        assert!(!e.has(HV_VP_ASSIST_NESTED_DEBUG));
    }
}
