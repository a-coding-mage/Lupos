//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/svm/svm_onhyperv.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/svm/svm_onhyperv.c
//! SVM-on-Hyper-V policy.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/svm/svm_onhyperv.c

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SvmOnHypervState {
    pub running_on_hyperv: bool,
    pub enlightened_vmcb: bool,
    pub direct_tlb_flush: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvmOnHypervPolicy {
    pub use_enlightened_vmcb: bool,
    pub use_direct_flush: bool,
}

pub const fn svm_on_hyperv_policy(state: SvmOnHypervState) -> SvmOnHypervPolicy {
    SvmOnHypervPolicy {
        use_enlightened_vmcb: state.running_on_hyperv && state.enlightened_vmcb,
        use_direct_flush: state.running_on_hyperv
            && state.enlightened_vmcb
            && state.direct_tlb_flush,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_flush_requires_hyperv_and_enlightened_vmcb() {
        let state = SvmOnHypervState {
            running_on_hyperv: true,
            enlightened_vmcb: false,
            direct_tlb_flush: true,
        };
        assert_eq!(
            svm_on_hyperv_policy(state),
            SvmOnHypervPolicy {
                use_enlightened_vmcb: false,
                use_direct_flush: false
            }
        );
    }
}
