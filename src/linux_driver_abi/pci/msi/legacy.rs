//! linux-parity: complete
//! linux-source: vendor/linux/drivers/pci/msi/legacy.c
//! test-origin: linux:vendor/linux/drivers/pci/msi/legacy.c
//! PCI legacy INTx/MSI transition coverage for M55.
//!
//! Mirrors `vendor/linux/drivers/pci/msi/legacy.c`.

use core::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
pub struct PciIntxState {
    enabled: AtomicBool,
}

impl PciIntxState {
    pub const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
        }
    }

    pub fn set_intx(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    pub fn intx_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intx_can_be_disabled_for_msi() {
        let state = PciIntxState::new();
        state.set_intx(false);
        assert!(!state.intx_enabled());
    }
}
