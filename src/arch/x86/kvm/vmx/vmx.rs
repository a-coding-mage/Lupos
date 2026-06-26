//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/vmx.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/vmx.c
//! VMX capability checks and fixed-bit application.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/vmx.c

use crate::include::uapi::errno::ENODEV;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmxBasicCaps {
    pub vmxon: bool,
    pub true_controls: bool,
    pub ept: bool,
    pub unrestricted_guest: bool,
}

pub const fn vmx_available(caps: VmxBasicCaps) -> bool {
    caps.vmxon
}

pub const fn validate_vmx_caps(caps: VmxBasicCaps) -> Result<(), i32> {
    if !vmx_available(caps) {
        Err(ENODEV)
    } else {
        Ok(())
    }
}

pub const fn apply_fixed_bits(value: u64, fixed0: u64, fixed1: u64) -> u64 {
    (value | fixed0) & fixed1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_bits_are_or_then_and_like_cr_masks() {
        assert_eq!(apply_fixed_bits(0b0101, 0b0010, 0b0111), 0b0111);
        assert_eq!(
            validate_vmx_caps(VmxBasicCaps {
                vmxon: false,
                true_controls: false,
                ept: false,
                unrestricted_guest: false,
            }),
            Err(ENODEV)
        );
    }
}
