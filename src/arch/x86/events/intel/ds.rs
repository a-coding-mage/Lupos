//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/ds.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/ds.c
//! Intel Debug Store / PEBS model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/ds.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DebugStoreArea {
    pub bts_base: u64,
    pub bts_index: u64,
    pub bts_absolute_maximum: u64,
    pub pebs_base: u64,
    pub pebs_index: u64,
    pub pebs_absolute_maximum: u64,
}

pub const fn debug_store_range_valid(base: u64, index: u64, maximum: u64) -> Result<(), i32> {
    if base <= index && index <= maximum {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

pub const fn pebs_record_size(format: u8) -> usize {
    match format {
        0 => 144,
        1 => 176,
        2 => 256,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_store_ranges_are_half_open_like_linux_checks() {
        assert_eq!(debug_store_range_valid(0x1000, 0x1008, 0x2000), Ok(()));
        assert_eq!(debug_store_range_valid(0x2000, 0x1000, 0x3000), Err(EINVAL));
    }
}
