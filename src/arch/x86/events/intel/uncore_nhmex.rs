//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/uncore_nhmex.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/uncore_nhmex.c
//! Intel Nehalem-EX uncore model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/uncore_nhmex.c

use super::uncore::{IntelUncoreBox, IntelUncoreBoxType, uncore_box};

pub const NHMEX_MODEL: u8 = 0x2e;

pub const fn nhmex_supported(model: u8) -> bool {
    model == NHMEX_MODEL
}

pub const fn nhmex_box(model: u8, box_id: u8) -> Option<IntelUncoreBox> {
    if nhmex_supported(model) {
        Some(uncore_box(IntelUncoreBoxType::Cbox, box_id))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nhmex_model_is_exact() {
        assert!(nhmex_box(NHMEX_MODEL, 0).is_some());
        assert_eq!(nhmex_box(0x2f, 0), None);
    }
}
