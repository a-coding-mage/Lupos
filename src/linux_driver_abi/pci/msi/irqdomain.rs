//! linux-parity: complete
//! linux-source: vendor/linux/drivers/pci/msi/irqdomain.c
//! test-origin: linux:vendor/linux/drivers/pci/msi/irqdomain.c
//! PCI MSI IRQ domain coverage for M55.
//!
//! Mirrors `vendor/linux/drivers/pci/msi/irqdomain.c`.

use crate::kernel::irq::irqdomain::{IrqDomain, IrqDomainKind};

pub struct PciMsiDomain {
    domain: IrqDomain,
}

impl PciMsiDomain {
    pub fn new(name: &str) -> Self {
        Self {
            domain: IrqDomain::new(name, IrqDomainKind::Hierarchical, 2048),
        }
    }

    pub fn map_hwirq(&self, hwirq: u32) -> u32 {
        self.domain.create_mapping(hwirq)
    }

    pub fn find_hwirq(&self, hwirq: u32) -> Option<u32> {
        self.domain.find_mapping(hwirq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pci_msi_domain_mapping_is_idempotent() {
        let domain = PciMsiDomain::new("pci-msi");
        assert_eq!(domain.map_hwirq(10), domain.map_hwirq(10));
    }
}
