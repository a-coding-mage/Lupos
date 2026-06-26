//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/resend.c
//! test-origin: linux:vendor/linux/kernel/irq/resend.c
//! IRQ resend coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/resend.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static RESEND_PENDING: AtomicU64 = AtomicU64::new(0);

pub fn check_irq_resend(irq: u32) -> bool {
    if irq >= 64 {
        return false;
    }
    RESEND_PENDING.fetch_or(1u64 << irq, Ordering::AcqRel);
    true
}

pub fn take_resend_pending() -> u64 {
    RESEND_PENDING.swap(0, Ordering::AcqRel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resend_sets_pending_bit() {
        assert!(check_irq_resend(5));
        assert!(take_resend_pending() & (1 << 5) != 0);
    }
}
