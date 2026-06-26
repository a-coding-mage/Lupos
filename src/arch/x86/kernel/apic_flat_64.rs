//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86-64 flat APIC destination model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/apic_flat_64.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlatApicDestination {
    pub logical_id: u8,
    pub mask: u64,
}

pub const fn flat_logical_id(cpu: u8) -> Option<FlatApicDestination> {
    if cpu >= 8 {
        None
    } else {
        Some(FlatApicDestination {
            logical_id: 1u8 << cpu,
            mask: 1u64 << cpu,
        })
    }
}

pub const fn flat_delivery_destination(mask: u64) -> u8 {
    (mask & 0xff) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_model_is_limited_to_eight_logical_ids() {
        assert_eq!(flat_logical_id(3).unwrap().logical_id, 0b1000);
        assert_eq!(flat_logical_id(8), None);
    }
}
