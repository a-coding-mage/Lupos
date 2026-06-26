//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/vmx/vmcs12.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/vmx/vmcs12.c
//! VMCS12 field encoding model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/vmx/vmcs12.c

pub const VMCS_FIELD_WIDTH_MASK: u16 = 0x6000;
pub const VMCS_FIELD_TYPE_MASK: u16 = 0x0c00;
pub const VMCS_FIELD_INDEX_MASK: u16 = 0x01ff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmcsField {
    pub width: u16,
    pub field_type: u16,
    pub index: u16,
}

pub const fn decode_vmcs_field(encoding: u16) -> VmcsField {
    VmcsField {
        width: (encoding & VMCS_FIELD_WIDTH_MASK) >> 13,
        field_type: (encoding & VMCS_FIELD_TYPE_MASK) >> 10,
        index: encoding & VMCS_FIELD_INDEX_MASK,
    }
}

pub const fn vmcs_field_is_read_only(encoding: u16) -> bool {
    decode_vmcs_field(encoding).field_type == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_decoder_keeps_width_type_and_index() {
        let field = decode_vmcs_field(0x681e);
        assert_eq!(field.width, 3);
        assert_eq!(field.field_type, 2);
        assert_eq!(field.index, 0x1e);
    }
}
