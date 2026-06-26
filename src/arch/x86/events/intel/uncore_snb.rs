//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/uncore_snb.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/uncore_snb.c
//! Intel Sandy Bridge client uncore model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/uncore_snb.c

use super::uncore::{IntelUncoreBox, IntelUncoreBoxType, uncore_box};

pub const fn snb_client_supported(model: u8) -> bool {
    matches!(model, 0x2a | 0x3a | 0x3c | 0x3f | 0x45 | 0x46)
}

pub const fn snb_client_cbox(model: u8, box_id: u8) -> Option<IntelUncoreBox> {
    if snb_client_supported(model) {
        Some(uncore_box(IntelUncoreBoxType::Cbox, box_id))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snb_client_accepts_sandy_and_haswell_models() {
        assert!(snb_client_supported(0x2a));
        assert!(snb_client_supported(0x3c));
        assert!(!snb_client_supported(0x55));
    }
}
