//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci
//! PCI / PCIe subsystem — M55.
//!
//! Mirrors `vendor/linux/drivers/pci/` and `include/linux/pci.h`.
//! Enumeration uses ACPI MCFG ECAM (PCI Express base specification §7.2.2).
//!
//! References:
//!   - `include/linux/pci.h:351`   — `struct pci_dev`
//!   - `include/linux/pci.h:700`   — `struct pci_bus`
//!   - `include/linux/pci.h:1021`  — `struct pci_driver`
//!   - `drivers/pci/ecam.c`        — ECAM address calculation
//!   - `drivers/pci/probe.c`       — bus/device/function scan
//!   - `drivers/pci/pci-driver.c`  — driver registration + probe dispatch

pub mod access;
pub mod device;
pub mod driver;
pub mod ecam;
pub mod enumerate;
pub mod iomap;
pub mod linux_sources;
pub mod msi;
pub mod pci;

pub use device::{PCI_CLASS_BRIDGE_HOST, PciBus, PciDev};
pub use ecam::McfgEntry;
pub use enumerate::{pci_devices, pci_enumerate, pci_enumerate_legacy_cf8, pci_find_device};

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

extern crate alloc;

lazy_static! {
    pub(crate) static ref PCI_BUSES: Mutex<Vec<alloc::sync::Arc<PciBus>>> = Mutex::new(Vec::new());
}

pub fn register_module_exports() {
    device::register_module_exports();
    driver::register_module_exports();
    access::register_module_exports();
    pci::register_module_exports();
    iomap::register_module_exports();
    msi::register_module_exports();
}
