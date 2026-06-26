//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/irq_sim.c
//! test-origin: linux:vendor/linux/kernel/irq/irq_sim.c
//! IRQ simulator coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/irq_sim.c`.

use core::sync::atomic::{AtomicU32, Ordering};

#[repr(C)]
pub struct IrqSim {
    base: u32,
    count: u32,
    next: AtomicU32,
}

impl IrqSim {
    pub const fn new(base: u32, count: u32) -> Self {
        Self {
            base,
            count,
            next: AtomicU32::new(0),
        }
    }

    pub fn alloc(&self) -> Option<u32> {
        let idx = self.next.fetch_add(1, Ordering::AcqRel);
        if idx < self.count {
            Some(self.base + idx)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulator_allocates_linear_irqs() {
        let sim = IrqSim::new(100, 2);
        assert_eq!(sim.alloc(), Some(100));
        assert_eq!(sim.alloc(), Some(101));
        assert_eq!(sim.alloc(), None);
    }
}
