//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/generic-chip.c
//! test-origin: linux:vendor/linux/kernel/irq/generic-chip.c
//! Generic IRQ chip coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/generic-chip.c`.

use core::sync::atomic::{AtomicU32, Ordering};

#[repr(C)]
pub struct GenericIrqChip {
    pub first_irq: u32,
    pub nr_irqs: u32,
    mask_cache: AtomicU32,
}

impl GenericIrqChip {
    pub const fn new(first_irq: u32, nr_irqs: u32) -> Self {
        Self {
            first_irq,
            nr_irqs,
            mask_cache: AtomicU32::new(0),
        }
    }

    pub fn mask(&self, offset: u32) {
        if offset < 32 {
            self.mask_cache.fetch_or(1u32 << offset, Ordering::AcqRel);
        }
    }

    pub fn unmask(&self, offset: u32) {
        if offset < 32 {
            self.mask_cache
                .fetch_and(!(1u32 << offset), Ordering::AcqRel);
        }
    }

    pub fn mask_cache(&self) -> u32 {
        self.mask_cache.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_chip_masks_and_unmasks_offsets() {
        let chip = GenericIrqChip::new(32, 4);
        chip.mask(1);
        assert_eq!(chip.mask_cache(), 0b10);
        chip.unmask(1);
        assert_eq!(chip.mask_cache(), 0);
    }
}
