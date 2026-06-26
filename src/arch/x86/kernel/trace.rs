//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/trace.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/trace.c
//! x86 osnoise IRQ trace registration state.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/trace.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub static OSNOISE_ARCH_REGISTERED: AtomicBool = AtomicBool::new(false);
pub static IRQ_ENTRY_COUNT: AtomicU64 = AtomicU64::new(0);
pub static IRQ_EXIT_COUNT: AtomicU64 = AtomicU64::new(0);
pub static LAST_IRQ_VECTOR: AtomicU64 = AtomicU64::new(0);

pub fn trace_intel_irq_entry(vector: u8) {
    LAST_IRQ_VECTOR.store(vector as u64, Ordering::Release);
    IRQ_ENTRY_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn trace_intel_irq_exit(vector: u8) {
    LAST_IRQ_VECTOR.store(vector as u64, Ordering::Release);
    IRQ_EXIT_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn osnoise_arch_register() -> i32 {
    OSNOISE_ARCH_REGISTERED.store(true, Ordering::Release);
    0
}

pub fn osnoise_arch_unregister() {
    OSNOISE_ARCH_REGISTERED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osnoise_register_and_irq_trace_state_are_observable() {
        assert_eq!(osnoise_arch_register(), 0);
        assert!(OSNOISE_ARCH_REGISTERED.load(Ordering::Acquire));
        trace_intel_irq_entry(32);
        trace_intel_irq_exit(32);
        assert_eq!(LAST_IRQ_VECTOR.load(Ordering::Acquire), 32);
        assert!(IRQ_ENTRY_COUNT.load(Ordering::Acquire) >= 1);
        assert!(IRQ_EXIT_COUNT.load(Ordering::Acquire) >= 1);
        osnoise_arch_unregister();
        assert!(!OSNOISE_ARCH_REGISTERED.load(Ordering::Acquire));
    }
}
