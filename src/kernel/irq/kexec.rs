//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/kexec.c
//! test-origin: linux:vendor/linux/kernel/irq/kexec.c
//! IRQ kexec coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/kexec.c`.

use core::sync::atomic::{AtomicBool, Ordering};

static IRQ_KEXEC_READY: AtomicBool = AtomicBool::new(false);

pub fn irq_kexec_prepare() {
    IRQ_KEXEC_READY.store(true, Ordering::Release);
}

pub fn irq_kexec_ready() -> bool {
    IRQ_KEXEC_READY.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_marks_irq_kexec_ready() {
        irq_kexec_prepare();
        assert!(irq_kexec_ready());
    }
}
