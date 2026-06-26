//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/uncore_snbep.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/uncore_snbep.c
//! Intel Sandy Bridge-EP uncore model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/uncore_snbep.c

use super::uncore::{IntelUncoreBox, IntelUncoreBoxType, uncore_box};

pub const fn snbep_server_supported(model: u8) -> bool {
    matches!(model, 0x2d | 0x3e | 0x3f | 0x4f | 0x55 | 0x6a | 0x6c)
}

pub const fn snbep_imc_box(model: u8, socket: u8) -> Option<IntelUncoreBox> {
    if snbep_server_supported(model) {
        Some(uncore_box(IntelUncoreBoxType::Imc, socket))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snbep_server_model_table_covers_common_xeons() {
        assert!(snbep_server_supported(0x2d));
        assert!(snbep_server_supported(0x55));
        assert!(!snbep_server_supported(0x2a));
    }
}
