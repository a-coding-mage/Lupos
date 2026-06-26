//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/hyperv/hv_apic.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/hv_apic.c
//! Hyper-V APIC and SynIC interrupt model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/hv_apic.c

use crate::include::uapi::errno::EINVAL;

pub const HV_SYNIC_SINT_COUNT: u8 = 16;
pub const HV_APIC_VECTOR_MIN: u8 = 0x10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SynicSint {
    pub sint: u8,
    pub vector: u8,
    pub masked: bool,
    pub auto_eoi: bool,
}

pub const fn synthetic_vector_valid(vector: u8) -> Result<(), i32> {
    if vector >= HV_APIC_VECTOR_MIN {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

pub const fn synic_sint(sint: u8, vector: u8) -> Result<SynicSint, i32> {
    if sint >= HV_SYNIC_SINT_COUNT {
        return Err(EINVAL);
    }
    match synthetic_vector_valid(vector) {
        Ok(()) => Ok(SynicSint {
            sint,
            vector,
            masked: false,
            auto_eoi: true,
        }),
        Err(e) => Err(e),
    }
}

pub const fn hv_apic_eoi_required(sint: SynicSint) -> bool {
    !sint.auto_eoi && !sint.masked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synic_vectors_reject_exception_range() {
        assert_eq!(synic_sint(0, 0x0f), Err(EINVAL));
        assert_eq!(synic_sint(16, 0x20), Err(EINVAL));
        assert_eq!(synic_sint(1, 0x20).unwrap().auto_eoi, true);
    }
}
