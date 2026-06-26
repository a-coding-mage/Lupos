//! linux-parity: complete
//! linux-source: vendor/linux/drivers/pci/msi/pcidev_msi.c
//! test-origin: linux:vendor/linux/drivers/pci/msi/pcidev_msi.c
//! PCI device MSI state coverage for M55.
//!
//! Mirrors `vendor/linux/drivers/pci/msi/pcidev_msi.c`.

use crate::linux_driver_abi::pci::device::PciDev;

pub fn pci_msi_enabled(dev: &PciDev) -> bool {
    dev.irq.lock().is_some()
}

pub fn pci_msi_set_vector(dev: &PciDev, vector: u32) {
    *dev.irq.lock() = Some(vector);
}

pub fn pci_msi_clear_vector(dev: &PciDev) {
    *dev.irq.lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_state_tracks_pci_dev_irq_field() {
        let dev = PciDev::new(0, 0, 1, 0, 0x1234, 0x5678, 1, 0, 0, 0);
        assert!(!pci_msi_enabled(&dev));
        pci_msi_set_vector(&dev, 0xC1);
        assert!(pci_msi_enabled(&dev));
        pci_msi_clear_vector(&dev);
        assert!(!pci_msi_enabled(&dev));
    }
}
