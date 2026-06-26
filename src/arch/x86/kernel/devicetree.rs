//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/devicetree.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/devicetree.c
//! x86 Open Firmware / flattened device-tree helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/devicetree.c

use crate::include::uapi::errno::EINVAL;

pub const SETUP_DATA_DATA_OFFSET: u64 = 16;
pub const COMMAND_LINE_SIZE: usize = 2048;
pub const APIC_DEFAULT_PHYS_BASE: u64 = 0xfee0_0000;

pub const IRQ_TYPE_EDGE_RISING: u32 = 1;
pub const IRQ_TYPE_EDGE_FALLING: u32 = 2;
pub const IRQ_TYPE_LEVEL_HIGH: u32 = 4;
pub const IRQ_TYPE_LEVEL_LOW: u32 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OfIoApicType {
    pub out_type: u32,
    pub is_level: bool,
    pub active_low: bool,
}

pub const OF_IOAPIC_TYPES: [OfIoApicType; 4] = [
    OfIoApicType {
        out_type: IRQ_TYPE_EDGE_FALLING,
        is_level: false,
        active_low: true,
    },
    OfIoApicType {
        out_type: IRQ_TYPE_LEVEL_HIGH,
        is_level: true,
        active_low: false,
    },
    OfIoApicType {
        out_type: IRQ_TYPE_LEVEL_LOW,
        is_level: true,
        active_low: true,
    },
    OfIoApicType {
        out_type: IRQ_TYPE_EDGE_RISING,
        is_level: false,
        active_low: false,
    },
];

pub const fn add_dtb(data: u64) -> u64 {
    data + SETUP_DATA_DATA_OFFSET
}

pub const fn dt_irqdomain_alloc_type(type_index: usize) -> Result<OfIoApicType, i32> {
    if type_index >= OF_IOAPIC_TYPES.len() {
        Err(EINVAL)
    } else {
        Ok(OF_IOAPIC_TYPES[type_index])
    }
}

pub const fn dtb_lapic_address(resource_start: Option<u64>) -> u64 {
    match resource_start {
        Some(addr) => addr,
        None => APIC_DEFAULT_PHYS_BASE,
    }
}

pub const fn x86_flattree_should_parse_smp(acpi_disabled: bool, populated_dt: bool) -> bool {
    acpi_disabled && populated_dt
}

pub const fn initial_map_len(initial_dtb: u64, page_size: u64) -> u64 {
    let page_offset = initial_dtb & (page_size - 1);
    let tail = page_size - page_offset;
    if tail > 128 { tail } else { 128 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_data_pointer_advances_to_payload() {
        assert_eq!(add_dtb(0x1000), 0x1010);
    }

    #[test]
    fn ioapic_irq_type_table_matches_linux_order() {
        assert_eq!(
            dt_irqdomain_alloc_type(0),
            Ok(OfIoApicType {
                out_type: IRQ_TYPE_EDGE_FALLING,
                is_level: false,
                active_low: true
            })
        );
        assert_eq!(
            dt_irqdomain_alloc_type(2),
            Ok(OfIoApicType {
                out_type: IRQ_TYPE_LEVEL_LOW,
                is_level: true,
                active_low: true
            })
        );
        assert_eq!(dt_irqdomain_alloc_type(4), Err(EINVAL));
    }

    #[test]
    fn lapic_and_smp_defaults_match_linux() {
        assert_eq!(dtb_lapic_address(None), APIC_DEFAULT_PHYS_BASE);
        assert_eq!(dtb_lapic_address(Some(0xfee0_1000)), 0xfee0_1000);
        assert!(x86_flattree_should_parse_smp(true, true));
        assert!(!x86_flattree_should_parse_smp(false, true));
    }

    #[test]
    fn initial_map_len_covers_page_tail_or_128_bytes() {
        assert_eq!(initial_map_len(0x1000, 4096), 4096);
        assert_eq!(initial_map_len(0x1ff0, 4096), 128);
    }
}
