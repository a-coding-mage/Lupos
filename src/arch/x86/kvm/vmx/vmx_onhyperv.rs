//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/vmx/vmx_onhyperv.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/vmx_onhyperv.c
//! VMX-on-Hyper-V enlightenment policy.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/vmx_onhyperv.c

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VmxOnHypervState {
    pub running_on_hyperv: bool,
    pub enlightened_vmcs: bool,
    pub nested_flush_hypercall: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmxOnHypervPolicy {
    pub use_enlightened_vmcs: bool,
    pub use_nested_flush_hypercall: bool,
}

pub const fn vmx_on_hyperv_policy(state: VmxOnHypervState) -> VmxOnHypervPolicy {
    VmxOnHypervPolicy {
        use_enlightened_vmcs: state.running_on_hyperv && state.enlightened_vmcs,
        use_nested_flush_hypercall: state.running_on_hyperv
            && state.enlightened_vmcs
            && state.nested_flush_hypercall,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_flush_depends_on_enlightened_vmcs() {
        let policy = vmx_on_hyperv_policy(VmxOnHypervState {
            running_on_hyperv: true,
            enlightened_vmcs: false,
            nested_flush_hypercall: true,
        });
        assert_eq!(policy.use_nested_flush_hypercall, false);
    }
}
