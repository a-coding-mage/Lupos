//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_irqsoff.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_irqsoff.c
//! `irqsoff` / `preemptoff` / `preemptirqsoff` latency tracers.
//!
//! Ref: vendor/linux/kernel/trace/trace_irqsoff.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static MAX_IRQSOFF_NS: AtomicU64 = AtomicU64::new(0);

pub fn start(_now_ns: u64) {}

pub fn end(elapsed_ns: u64) {
    let cur = MAX_IRQSOFF_NS.load(Ordering::Acquire);
    if elapsed_ns > cur {
        MAX_IRQSOFF_NS.store(elapsed_ns, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_records_max() {
        MAX_IRQSOFF_NS.store(0, Ordering::Release);
        end(500);
        end(2000);
        end(1000);
        assert_eq!(MAX_IRQSOFF_NS.load(Ordering::Acquire), 2000);
    }
}
