//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_preemptirq.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_preemptirq.c
//! Static tracepoints fired on irq-disable/enable, preempt-disable/enable.
//!
//! Ref: vendor/linux/kernel/trace/trace_preemptirq.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static IRQ_DISABLE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static IRQ_ENABLE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static PREEMPT_DISABLE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static PREEMPT_ENABLE_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn trace_irq_disable() {
    IRQ_DISABLE_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn trace_irq_enable() {
    IRQ_ENABLE_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn trace_preempt_disable() {
    PREEMPT_DISABLE_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn trace_preempt_enable() {
    PREEMPT_ENABLE_COUNT.fetch_add(1, Ordering::AcqRel);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_event_bumps_its_counter() {
        let d0 = IRQ_DISABLE_COUNT.load(Ordering::Acquire);
        trace_irq_disable();
        assert_eq!(IRQ_DISABLE_COUNT.load(Ordering::Acquire), d0 + 1);
        let p0 = PREEMPT_DISABLE_COUNT.load(Ordering::Acquire);
        trace_preempt_disable();
        assert_eq!(PREEMPT_DISABLE_COUNT.load(Ordering::Acquire), p0 + 1);
    }
}
