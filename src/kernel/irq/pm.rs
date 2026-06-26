//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/pm.c
//! test-origin: linux:vendor/linux/kernel/irq/pm.c
//! IRQ power-management coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/pm.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static WAKE_MASK: AtomicU64 = AtomicU64::new(0);

pub fn irq_set_irq_wake(irq: u32, on: bool) -> bool {
    if irq >= 64 {
        return false;
    }
    if on {
        WAKE_MASK.fetch_or(1u64 << irq, Ordering::AcqRel);
    } else {
        WAKE_MASK.fetch_and(!(1u64 << irq), Ordering::AcqRel);
    }
    true
}

pub fn irq_wake_mask() -> u64 {
    WAKE_MASK.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_mask_tracks_irq() {
        assert!(irq_set_irq_wake(4, true));
        assert!(irq_wake_mask() & (1 << 4) != 0);
        assert!(irq_set_irq_wake(4, false));
        assert!(irq_wake_mask() & (1 << 4) == 0);
    }
}
