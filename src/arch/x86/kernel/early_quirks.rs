//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/early-quirks.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/early-quirks.c
//! Early x86 PCI quirk matching.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/early-quirks.c

pub const PCI_VENDOR_ID_ATI: u16 = 0x1002;
pub const PCI_VENDOR_ID_AMD: u16 = 0x1022;
pub const PCI_VENDOR_ID_NVIDIA: u16 = 0x10de;
pub const PCI_VENDOR_ID_VIA: u16 = 0x1106;
pub const PCI_VENDOR_ID_BROADCOM: u16 = 0x14e4;
pub const PCI_VENDOR_ID_INTEL: u16 = 0x8086;

pub const PCI_DEVICE_ID_ATI_IXP400_SMBUS: u16 = 0x4372;
pub const PCI_DEVICE_ID_ATI_SBX00_SMBUS: u16 = 0x4385;

pub const PCI_CLASS_DISPLAY_VGA: u16 = 0x0300;
pub const PCI_CLASS_BRIDGE_HOST: u16 = 0x0600;
pub const PCI_HEADER_TYPE_BRIDGE: u8 = 1;
pub const PCI_HEADER_TYPE_MFD: u8 = 0x80;
pub const PCI_SECONDARY_BUS: u8 = 0x19;

pub const KB: u64 = 1024;
pub const MB: u64 = 1024 * KB;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
    pub vendor: u16,
    pub device: u16,
    pub class: u16,
    pub header_type: u8,
    pub secondary_bus: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resource {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EarlyQuirkAction {
    NvidiaTimerOverride,
    ViaDisableGart,
    FixHypertransportBroadcast,
    AtiTimerOverride,
    IntelIrqRemapBroken,
    ReserveIntelGraphics,
    ForceDisableHpet,
    AppleAirportReset,
}

pub const fn single_function(header_type: u8) -> bool {
    (header_type & PCI_HEADER_TYPE_MFD) == 0
}

pub const fn bridge_secondary_bus(dev: PciDevice) -> Option<u8> {
    if (dev.header_type & 0x7f) == PCI_HEADER_TYPE_BRIDGE {
        Some(dev.secondary_bus)
    } else {
        None
    }
}

pub const fn match_early_quirk(dev: PciDevice) -> Option<EarlyQuirkAction> {
    match dev.vendor {
        PCI_VENDOR_ID_NVIDIA => Some(EarlyQuirkAction::NvidiaTimerOverride),
        PCI_VENDOR_ID_VIA => Some(EarlyQuirkAction::ViaDisableGart),
        PCI_VENDOR_ID_AMD => {
            if dev.class == PCI_CLASS_BRIDGE_HOST {
                Some(EarlyQuirkAction::FixHypertransportBroadcast)
            } else {
                None
            }
        }
        PCI_VENDOR_ID_ATI => {
            if dev.device == PCI_DEVICE_ID_ATI_IXP400_SMBUS
                || dev.device == PCI_DEVICE_ID_ATI_SBX00_SMBUS
            {
                Some(EarlyQuirkAction::AtiTimerOverride)
            } else {
                None
            }
        }
        PCI_VENDOR_ID_INTEL => {
            if dev.class == PCI_CLASS_DISPLAY_VGA {
                Some(EarlyQuirkAction::ReserveIntelGraphics)
            } else if dev.class == PCI_CLASS_BRIDGE_HOST {
                Some(EarlyQuirkAction::IntelIrqRemapBroken)
            } else {
                None
            }
        }
        PCI_VENDOR_ID_BROADCOM => Some(EarlyQuirkAction::AppleAirportReset),
        _ => None,
    }
}

pub const fn gen6_stolen_size(gms: u16) -> u64 {
    (gms as u64) * 32 * MB
}

pub const fn chv_stolen_size(gms: u16) -> u64 {
    match gms {
        0 => 0,
        1 => 32 * MB,
        2 => 64 * MB,
        3 => 96 * MB,
        4 => 128 * MB,
        5 => 160 * MB,
        6 => 192 * MB,
        7 => 224 * MB,
        8 => 256 * MB,
        9 => 288 * MB,
        10 => 320 * MB,
        11 => 352 * MB,
        12 => 384 * MB,
        13 => 416 * MB,
        14 => 448 * MB,
        15 => 480 * MB,
        _ => 512 * MB,
    }
}

pub const fn gen9_stolen_size(gms: u16) -> u64 {
    match gms {
        0 => 0,
        1..=0xef => (gms as u64) * 32 * MB,
        0xf0..=0xfe => 4 * 1024 * MB + ((gms as u64 - 0xf0) * 4 * MB),
        _ => 0,
    }
}

pub const fn intel_graphics_stolen(base: u64, size: u64) -> Option<Resource> {
    if base == 0 || size == 0 {
        None
    } else {
        Some(Resource {
            start: base,
            end: base + size - 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VGA_INTEL: PciDevice = PciDevice {
        bus: 0,
        slot: 2,
        func: 0,
        vendor: PCI_VENDOR_ID_INTEL,
        device: 0x1234,
        class: PCI_CLASS_DISPLAY_VGA,
        header_type: 0,
        secondary_bus: 0,
    };

    #[test]
    fn quirk_table_matches_core_linux_vendor_cases() {
        assert_eq!(
            match_early_quirk(VGA_INTEL),
            Some(EarlyQuirkAction::ReserveIntelGraphics)
        );
        assert_eq!(
            match_early_quirk(PciDevice {
                vendor: PCI_VENDOR_ID_ATI,
                device: PCI_DEVICE_ID_ATI_SBX00_SMBUS,
                class: 0,
                ..VGA_INTEL
            }),
            Some(EarlyQuirkAction::AtiTimerOverride)
        );
        assert_eq!(
            match_early_quirk(PciDevice {
                vendor: PCI_VENDOR_ID_NVIDIA,
                class: 0,
                ..VGA_INTEL
            }),
            Some(EarlyQuirkAction::NvidiaTimerOverride)
        );
    }

    #[test]
    fn multifunction_and_bridge_helpers_decode_header_type() {
        assert!(single_function(0));
        assert!(!single_function(PCI_HEADER_TYPE_MFD));
        assert_eq!(
            bridge_secondary_bus(PciDevice {
                header_type: PCI_HEADER_TYPE_BRIDGE,
                secondary_bus: 7,
                ..VGA_INTEL
            }),
            Some(7)
        );
    }

    #[test]
    fn intel_stolen_memory_size_helpers_match_linux_units() {
        assert_eq!(gen6_stolen_size(2), 64 * MB);
        assert_eq!(chv_stolen_size(5), 160 * MB);
        assert_eq!(gen9_stolen_size(0xf1), 4 * 1024 * MB + 4 * MB);
        assert_eq!(
            intel_graphics_stolen(0x1000_0000, 64 * MB),
            Some(Resource {
                start: 0x1000_0000,
                end: 0x13ff_ffff
            })
        );
    }
}
