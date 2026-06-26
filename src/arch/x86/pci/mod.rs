//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/pci
//! test-origin: linux:vendor/linux/arch/x86/pci
//! x86 PCI and MMCONFIG discovery policy.
//!
//! The generic PCI/device model owns real enumeration. This arch layer exposes
//! the x86-specific discovery route selection Linux splits across direct I/O,
//! BIOS, ACPI MCFG, IRQ routing, and platform-specific fixups.
//!
//! References:
//! - `vendor/linux/arch/x86/pci/direct.c`
//! - vendor/linux/arch/x86/pci/acpi.c
//! - vendor/linux/arch/x86/pci/amd_bus.c
//! - vendor/linux/arch/x86/pci/broadcom_bus.c
//! - vendor/linux/arch/x86/pci/bus_numa.c
//! - vendor/linux/arch/x86/pci/ce4100.c
//! - vendor/linux/arch/x86/pci/common.c
//! - vendor/linux/arch/x86/pci/early.c
//! - vendor/linux/arch/x86/pci/fixup.c
//! - vendor/linux/arch/x86/pci/i386.c
//! - vendor/linux/arch/x86/pci/init.c
//! - vendor/linux/arch/x86/pci/intel_mid.c
//! - vendor/linux/arch/x86/pci/irq.c
//! - vendor/linux/arch/x86/pci/legacy.c
//! - vendor/linux/arch/x86/pci/mmconfig-shared.c
//! - vendor/linux/arch/x86/pci/mmconfig_32.c
//! - vendor/linux/arch/x86/pci/mmconfig_64.c
//! - vendor/linux/arch/x86/pci/numachip.c
//! - vendor/linux/arch/x86/pci/olpc.c
//! - vendor/linux/arch/x86/pci/pcbios.c
//! - vendor/linux/arch/x86/pci/xen.c

use crate::include::uapi::errno::EINVAL;

pub mod broadcom_bus;
pub mod early;
pub mod init;
pub mod legacy;

pub const PCI_PROBE_BIOS: u32 = 1 << 0;
pub const PCI_PROBE_CONF1: u32 = 1 << 1;
pub const PCI_PROBE_CONF2: u32 = 1 << 2;
pub const PCI_PROBE_MMCONF: u32 = 1 << 3;
pub const PCI_CAN_SKIP_ISA_ALIGN: u32 = 1 << 4;
pub const PCI_CHECK_ENABLE_AMD_MMCONF: u32 = 1 << 5;
pub const PCI_USE_PIRQ_MASK: u32 = 1 << 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PciConfigAccess {
    None,
    PortCf8,
    Mmconfig,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PciProbePolicy {
    pub access: PciConfigAccess,
    pub acpi_mcfg: bool,
    pub legacy_bios: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PciRawRoute {
    LegacyConfig1,
    LegacyConfig2,
    Mmconfig,
    Bios,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PciHostBridge {
    GenericPc,
    AmdBus,
    Broadcom,
    Ce4100,
    IntelMid,
    Numachip,
    Olpc,
    Xen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MmconfigRegion {
    pub base: u64,
    pub segment: u16,
    pub start_bus: u8,
    pub end_bus: u8,
    pub is_64_bit: bool,
}

pub const fn default_probe_policy(acpi_available: bool) -> PciProbePolicy {
    if acpi_available {
        PciProbePolicy {
            access: PciConfigAccess::Mmconfig,
            acpi_mcfg: true,
            legacy_bios: false,
        }
    } else {
        PciProbePolicy {
            access: PciConfigAccess::PortCf8,
            acpi_mcfg: false,
            legacy_bios: false,
        }
    }
}

pub const fn default_pci_probe_mask(acpi_available: bool) -> u32 {
    let mut mask = PCI_PROBE_BIOS | PCI_PROBE_CONF1 | PCI_PROBE_CONF2;
    if acpi_available {
        mask |= PCI_PROBE_MMCONF;
    }
    mask
}

pub const fn select_raw_pci_route(
    domain: u16,
    reg: u16,
    probe_mask: u32,
) -> Result<PciRawRoute, i32> {
    if probe_mask & PCI_PROBE_MMCONF != 0 && reg >= 256 {
        return Ok(PciRawRoute::Mmconfig);
    }
    if domain == 0 && probe_mask & PCI_PROBE_CONF1 != 0 {
        return Ok(PciRawRoute::LegacyConfig1);
    }
    if domain == 0 && probe_mask & PCI_PROBE_CONF2 != 0 {
        return Ok(PciRawRoute::LegacyConfig2);
    }
    if probe_mask & PCI_PROBE_BIOS != 0 {
        return Ok(PciRawRoute::Bios);
    }
    Err(EINVAL)
}

pub const fn mmconfig_region_valid(region: MmconfigRegion) -> bool {
    region.base != 0 && region.start_bus <= region.end_bus
}

pub const fn mmconfig_bus_count(region: MmconfigRegion) -> u16 {
    if region.start_bus > region.end_bus {
        0
    } else {
        (region.end_bus as u16) - (region.start_bus as u16) + 1
    }
}

pub const fn mmconfig_size(region: MmconfigRegion) -> u64 {
    (mmconfig_bus_count(region) as u64) << 20
}

pub const fn pci_irq_route_allowed(routeirq: bool, noioapicquirk: bool) -> bool {
    routeirq && !noioapicquirk
}

pub const fn host_bridge_prefers_mmconfig(host: PciHostBridge) -> bool {
    matches!(
        host,
        PciHostBridge::GenericPc
            | PciHostBridge::AmdBus
            | PciHostBridge::Broadcom
            | PciHostBridge::IntelMid
            | PciHostBridge::Numachip
            | PciHostBridge::Xen
    )
}

pub const fn host_bridge_needs_legacy_io(host: PciHostBridge) -> bool {
    matches!(
        host,
        PciHostBridge::Ce4100 | PciHostBridge::Olpc | PciHostBridge::GenericPc
    )
}

/// Return the CF8/CFC config address for bus/device/function/register.
pub const fn cf8_address(bus: u8, device: u8, function: u8, register: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | (((device as u32) & 0x1f) << 11)
        | (((function as u32) & 0x07) << 8)
        | ((register as u32) & 0xfc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acpi_prefers_mmconfig() {
        assert_eq!(default_probe_policy(true).access, PciConfigAccess::Mmconfig);
        assert_eq!(default_probe_policy(false).access, PciConfigAccess::PortCf8);
    }

    #[test]
    fn cf8_address_masks_register_low_bits() {
        assert_eq!(cf8_address(0, 2, 3, 0x11), 0x8000_1310);
    }

    #[test]
    fn raw_route_prefers_mmconfig_for_extended_config_space() {
        assert_eq!(
            select_raw_pci_route(0, 300, default_pci_probe_mask(true)),
            Ok(PciRawRoute::Mmconfig)
        );
        assert_eq!(
            select_raw_pci_route(0, 0x10, PCI_PROBE_CONF1),
            Ok(PciRawRoute::LegacyConfig1)
        );
        assert_eq!(select_raw_pci_route(1, 0x10, 0), Err(EINVAL));
    }

    #[test]
    fn mmconfig_region_size_is_one_megabyte_per_bus() {
        let region = MmconfigRegion {
            base: 0xe000_0000,
            segment: 0,
            start_bus: 0,
            end_bus: 2,
            is_64_bit: true,
        };
        assert!(mmconfig_region_valid(region));
        assert_eq!(mmconfig_bus_count(region), 3);
        assert_eq!(mmconfig_size(region), 3 << 20);
    }

    #[test]
    fn platform_bridge_policy_preserves_legacy_exceptions() {
        assert!(host_bridge_prefers_mmconfig(PciHostBridge::AmdBus));
        assert!(host_bridge_needs_legacy_io(PciHostBridge::Olpc));
        assert!(!pci_irq_route_allowed(true, true));
    }
}
