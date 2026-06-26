//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/main.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/main.c
//! VMX module parameter policy.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/main.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmxModuleParams {
    pub enable_vpid: bool,
    pub enable_ept: bool,
    pub enable_unrestricted_guest: bool,
    pub flexpriority: bool,
}

pub const fn validate_vmx_module_params(params: VmxModuleParams) -> Result<(), i32> {
    if params.enable_unrestricted_guest && !params.enable_ept {
        return Err(EINVAL);
    }
    if params.enable_vpid && !params.enable_ept {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn default_vmx_module_params() -> VmxModuleParams {
    VmxModuleParams {
        enable_vpid: true,
        enable_ept: true,
        enable_unrestricted_guest: true,
        flexpriority: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrestricted_guest_is_ept_dependent() {
        assert!(validate_vmx_module_params(default_vmx_module_params()).is_ok());
        assert_eq!(
            validate_vmx_module_params(VmxModuleParams {
                enable_vpid: false,
                enable_ept: false,
                enable_unrestricted_guest: true,
                flexpriority: true
            }),
            Err(EINVAL)
        );
    }
}
