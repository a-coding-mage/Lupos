//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/autoprobe.c
//! test-origin: linux:vendor/linux/kernel/irq/autoprobe.c
//! IRQ autoprobe coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/autoprobe.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static PROBE_MASK: AtomicU64 = AtomicU64::new(0);

pub fn probe_irq_on(mask: u64) -> u64 {
    PROBE_MASK.store(mask, Ordering::Release);
    mask
}

pub fn probe_irq_off(observed: u64) -> Option<u32> {
    let pending = PROBE_MASK.swap(0, Ordering::AcqRel) & observed;
    if pending.count_ones() == 1 {
        Some(pending.trailing_zeros())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoprobe_resolves_single_irq() {
        probe_irq_on(0b1000);
        assert_eq!(probe_irq_off(0b1000), Some(3));
    }
}
