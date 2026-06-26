//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/pci/early.c
//! test-origin: linux:vendor/linux/arch/x86/pci/early.c
//! Early direct PCI config-space access helpers.

use super::init::PCI_PROBE_NOEARLY;
use super::{PCI_PROBE_CONF1, cf8_address};

pub const PCI_CONFIG_ADDRESS_PORT: u16 = 0x0cf8;
pub const PCI_CONFIG_DATA_PORT: u16 = 0x0cfc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EarlyPciAccess {
    pub address_port: u16,
    pub data_port: u16,
    pub address: u32,
}

pub const fn early_pci_access(bus: u8, slot: u8, func: u8, offset: u8) -> EarlyPciAccess {
    EarlyPciAccess {
        address_port: PCI_CONFIG_ADDRESS_PORT,
        data_port: PCI_CONFIG_DATA_PORT,
        address: cf8_address(bus, slot, func, offset),
    }
}

pub const fn read_pci_config_byte_port(offset: u8) -> u16 {
    PCI_CONFIG_DATA_PORT + ((offset & 3) as u16)
}

pub const fn read_pci_config_16_port(offset: u8) -> u16 {
    PCI_CONFIG_DATA_PORT + ((offset & 2) as u16)
}

pub const fn early_pci_allowed(pci_probe: u32) -> bool {
    (pci_probe & (PCI_PROBE_CONF1 | PCI_PROBE_NOEARLY)) == PCI_PROBE_CONF1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_pci_config_access_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/pci/early.c"
        ));
        assert!(
            source
                .contains("outl(0x80000000 | (bus<<16) | (slot<<11) | (func<<8) | offset, 0xcf8);")
        );
        assert!(source.contains("v = inl(0xcfc);"));
        assert!(source.contains("v = inb(0xcfc + (offset&3));"));
        assert!(source.contains("v = inw(0xcfc + (offset&2));"));
        assert!(source.contains("outb(val, 0xcfc + (offset&3));"));
        assert!(source.contains("early_pci_allowed"));
        assert!(source.contains("PCI_PROBE_CONF1|PCI_PROBE_NOEARLY"));

        let access = early_pci_access(0, 2, 3, 0x11);
        assert_eq!(access.address_port, 0x0cf8);
        assert_eq!(access.data_port, 0x0cfc);
        assert_eq!(access.address, 0x8000_1310);
        assert_eq!(read_pci_config_byte_port(5), 0x0cfd);
        assert_eq!(read_pci_config_16_port(6), 0x0cfe);
        assert!(early_pci_allowed(PCI_PROBE_CONF1));
        assert!(!early_pci_allowed(PCI_PROBE_CONF1 | PCI_PROBE_NOEARLY));
    }
}
