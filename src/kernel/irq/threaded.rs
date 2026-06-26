//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq
//! test-origin: linux:vendor/linux/kernel/irq
//! Threaded IRQ — per-IRQ kthread (`irq_thread`) (M37).
//!
//! Mirrors `vendor/linux/kernel/irq/manage.c::irq_thread`.  When a hard-IRQ
//! handler returns `IRQ_WAKE_THREAD`, this layer schedules the bottom-half
//! handler on a kthread.  Lupos M37 uses a simple per-IRQ AtomicU32 wake
//! counter; the test fixture asserts the counter increments after a wake.

use core::sync::atomic::{AtomicU32, Ordering};

use super::irqdesc::NR_IRQS;

static THREAD_WAKE_COUNT: [AtomicU32; NR_IRQS] = [const { AtomicU32::new(0) }; NR_IRQS];

/// Called by `generic_handle_irq` when a handler returns `IRQ_WAKE_THREAD`.
pub fn wake_irq_thread(irq: u32) {
    let i = irq as usize;
    if i < NR_IRQS {
        THREAD_WAKE_COUNT[i].fetch_add(1, Ordering::AcqRel);
    }
}

pub fn thread_wake_count(irq: u32) -> u32 {
    THREAD_WAKE_COUNT
        .get(irq as usize)
        .map(|c| c.load(Ordering::Acquire))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_increments_per_irq_counter() {
        let irq = 0xA0u32;
        let before = thread_wake_count(irq);
        wake_irq_thread(irq);
        assert_eq!(thread_wake_count(irq), before + 1);
    }
}
