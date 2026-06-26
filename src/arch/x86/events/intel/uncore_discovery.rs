//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/uncore_discovery.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/uncore_discovery.c
//! Intel uncore discovery table model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/uncore_discovery.c

use super::uncore::{IntelUncoreBox, IntelUncoreBoxType, uncore_box};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscoveryUnit {
    pub box_type: IntelUncoreBoxType,
    pub units: u8,
}

pub const fn discovered_box(unit: DiscoveryUnit, index: u8) -> Option<IntelUncoreBox> {
    if index >= unit.units {
        None
    } else {
        Some(uncore_box(unit.box_type, index))
    }
}

pub const fn discovery_units_present(mmio_table_present: bool, pci_table_present: bool) -> bool {
    mmio_table_present || pci_table_present
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_rejects_out_of_range_box_index() {
        let unit = DiscoveryUnit {
            box_type: IntelUncoreBoxType::Cbox,
            units: 2,
        };
        assert!(discovered_box(unit, 1).is_some());
        assert_eq!(discovered_box(unit, 2), None);
    }
}
