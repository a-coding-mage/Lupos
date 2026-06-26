//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/nested.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/nested.c
//! Nested VMX control validation.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/nested.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NestedVmxControls {
    pub pinbased: u32,
    pub procbased: u32,
    pub secondary: u32,
    pub entry: u32,
    pub exit: u32,
}

pub const CPU_BASED_ACTIVATE_SECONDARY_CONTROLS: u32 = 1 << 31;
pub const SECONDARY_EXEC_ENABLE_EPT: u32 = 1 << 1;
pub const VM_ENTRY_IA32E_MODE: u32 = 1 << 9;
pub const VM_EXIT_HOST_ADDR_SPACE_SIZE: u32 = 1 << 9;

pub const fn validate_nested_vmx_controls(ctl: NestedVmxControls) -> Result<(), i32> {
    if ctl.secondary != 0 && ctl.procbased & CPU_BASED_ACTIVATE_SECONDARY_CONTROLS == 0 {
        return Err(EINVAL);
    }
    if ctl.entry & VM_ENTRY_IA32E_MODE != 0 && ctl.exit & VM_EXIT_HOST_ADDR_SPACE_SIZE == 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn nested_vmx_uses_ept(ctl: NestedVmxControls) -> bool {
    ctl.procbased & CPU_BASED_ACTIVATE_SECONDARY_CONTROLS != 0
        && ctl.secondary & SECONDARY_EXEC_ENABLE_EPT != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secondary_controls_require_activation_bit() {
        assert_eq!(
            validate_nested_vmx_controls(NestedVmxControls {
                secondary: SECONDARY_EXEC_ENABLE_EPT,
                ..NestedVmxControls::default()
            }),
            Err(EINVAL)
        );
    }
}
