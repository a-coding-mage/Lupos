//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/sgx.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/sgx.c
//! SGX virtualization policy for VMX.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/sgx.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmxSgxCaps {
    pub host_sgx: bool,
    pub guest_cpuid_sgx: bool,
    pub launch_control: bool,
}

pub const fn vmx_sgx_exposed(caps: VmxSgxCaps) -> bool {
    caps.host_sgx && caps.guest_cpuid_sgx && caps.launch_control
}

pub const fn vmx_sgx_vepc_required(caps: VmxSgxCaps) -> bool {
    vmx_sgx_exposed(caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgx_is_hidden_without_launch_control() {
        assert!(!vmx_sgx_exposed(VmxSgxCaps {
            host_sgx: true,
            guest_cpuid_sgx: true,
            launch_control: false,
        }));
    }
}
