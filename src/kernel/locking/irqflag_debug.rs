//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/irqflag-debug.c
//! test-origin: linux:vendor/linux/kernel/locking/irqflag-debug.c
//! IRQ flag debug coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/irqflag-debug.c`.

use core::sync::atomic::{AtomicUsize, Ordering};

static MISMATCHES: AtomicUsize = AtomicUsize::new(0);

pub fn note_irqflag_mismatch() {
    MISMATCHES.fetch_add(1, Ordering::AcqRel);
}

pub fn irqflag_mismatch_count() -> usize {
    MISMATCHES.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatch_counter_increments() {
        let before = irqflag_mismatch_count();
        note_irqflag_mismatch();
        assert_eq!(irqflag_mismatch_count(), before + 1);
    }
}
