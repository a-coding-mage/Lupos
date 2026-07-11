//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/msi
//! PCI MSI/MSI-X support surface.
//!
//! Mirrors `vendor/linux/drivers/pci/msi/`.

pub mod api;
pub mod irqdomain;
pub mod legacy;
pub mod msi;
pub mod pcidev_msi;

pub use api::{pci_alloc_irq_vectors, pci_free_irq_vectors};
pub use msi::MsiMsg;

pub fn register_module_exports() {
    api::register_module_exports();
}
