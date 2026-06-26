//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/hyperv/hv_vtl.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/hv_vtl.c
//! Hyper-V VTL model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/hv_vtl.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HvVtl {
    Vtl0,
    Vtl1,
    Vtl2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvVtlState {
    pub current: HvVtl,
    pub secure_vtl: bool,
}

pub const fn vtl_from_register(value: u64) -> HvVtl {
    match value & 0x0f {
        1 => HvVtl::Vtl1,
        2 => HvVtl::Vtl2,
        _ => HvVtl::Vtl0,
    }
}

pub const fn vtl_requires_isolation(vtl: HvVtl) -> bool {
    !matches!(vtl, HvVtl::Vtl0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vtl_decode_uses_low_nibble() {
        assert_eq!(vtl_from_register(0x12), HvVtl::Vtl2);
        assert!(vtl_requires_isolation(HvVtl::Vtl1));
        assert!(!vtl_requires_isolation(HvVtl::Vtl0));
    }
}
