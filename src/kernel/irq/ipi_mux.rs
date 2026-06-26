//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/ipi-mux.c
//! test-origin: linux:vendor/linux/kernel/irq/ipi-mux.c
//! IPI mux coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/ipi-mux.c`.

use core::sync::atomic::{AtomicU64, Ordering};

#[repr(C)]
pub struct IpiMux {
    registered: AtomicU64,
}

impl IpiMux {
    pub const fn new() -> Self {
        Self {
            registered: AtomicU64::new(0),
        }
    }

    pub fn register(&self, slot: u8) -> bool {
        if slot >= 64 {
            return false;
        }
        let bit = 1u64 << slot;
        self.registered.fetch_or(bit, Ordering::AcqRel) & bit == 0
    }

    pub fn unregister(&self, slot: u8) {
        if slot < 64 {
            self.registered.fetch_and(!(1u64 << slot), Ordering::AcqRel);
        }
    }

    pub fn registered_mask(&self) -> u64 {
        self.registered.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_reports_first_claim() {
        let mux = IpiMux::new();
        assert!(mux.register(1));
        assert!(!mux.register(1));
        mux.unregister(1);
        assert_eq!(mux.registered_mask(), 0);
    }
}
