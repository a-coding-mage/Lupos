//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/tdx.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/tdx.c
//! TDX VMX guest eligibility.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/tdx.c

use crate::include::uapi::errno::{EINVAL, ENODEV};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxHostCaps {
    pub tdx_module_loaded: bool,
    pub seamcall_available: bool,
    pub ept_enabled: bool,
}

pub const fn validate_tdx_guest(caps: TdxHostCaps) -> Result<(), i32> {
    if !caps.tdx_module_loaded || !caps.seamcall_available {
        Err(ENODEV)
    } else if !caps.ept_enabled {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tdx_requires_module_and_ept() {
        assert_eq!(
            validate_tdx_guest(TdxHostCaps {
                tdx_module_loaded: true,
                seamcall_available: true,
                ept_enabled: false,
            }),
            Err(EINVAL)
        );
    }
}
