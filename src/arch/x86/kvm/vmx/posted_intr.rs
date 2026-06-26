//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/posted_intr.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/posted_intr.c
//! Posted interrupt descriptor helpers.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/posted_intr.c

pub const POSTED_INTR_ON: u32 = 1 << 0;
pub const POSTED_INTR_SN: u32 = 1 << 1;
pub const POSTED_INTR_NV_MASK: u16 = 0xff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostedIntrDescriptor {
    pub control: u32,
    pub notification_vector: u8,
}

pub const fn posted_intr_is_active(desc: PostedIntrDescriptor) -> bool {
    desc.control & POSTED_INTR_ON != 0 && desc.control & POSTED_INTR_SN == 0
}

pub const fn posted_intr_vector_valid(vector: u8) -> bool {
    vector >= 0x10 && vector != 0xff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppress_notification_masks_active_delivery() {
        let desc = PostedIntrDescriptor {
            control: POSTED_INTR_ON | POSTED_INTR_SN,
            notification_vector: 0xf2,
        };
        assert!(!posted_intr_is_active(desc));
        assert!(posted_intr_vector_valid(desc.notification_vector));
    }
}
