//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/uncore.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/uncore.c
//! Intel uncore PMU model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/uncore.c

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntelUncoreBoxType {
    Cbox,
    Ubox,
    Mbox,
    Pcu,
    Imc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntelUncoreBox {
    pub box_type: IntelUncoreBoxType,
    pub box_id: u8,
    pub counters: u8,
}

pub const fn uncore_box(box_type: IntelUncoreBoxType, box_id: u8) -> IntelUncoreBox {
    let counters = match box_type {
        IntelUncoreBoxType::Cbox => 4,
        IntelUncoreBoxType::Ubox => 2,
        IntelUncoreBoxType::Mbox => 4,
        IntelUncoreBoxType::Pcu => 4,
        IntelUncoreBoxType::Imc => 4,
    };
    IntelUncoreBox {
        box_type,
        box_id,
        counters,
    }
}

pub const fn uncore_programming_errno() -> i32 {
    EOPNOTSUPP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncore_boxes_have_type_specific_counter_counts() {
        assert_eq!(uncore_box(IntelUncoreBoxType::Ubox, 0).counters, 2);
        assert_eq!(uncore_box(IntelUncoreBoxType::Cbox, 3).box_id, 3);
    }
}
