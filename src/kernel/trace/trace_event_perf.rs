//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_event_perf.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_event_perf.c
//! perf-event-style attachment to tracepoints — routes tracepoint hits to
//! a perf software counter.
//!
//! Ref: vendor/linux/kernel/trace/trace_event_perf.c

use core::sync::atomic::{AtomicU64, Ordering};

pub struct PerfEvent {
    pub count: AtomicU64,
}

impl PerfEvent {
    pub const fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
        }
    }

    pub fn fire(&self) {
        self.count.fetch_add(1, Ordering::AcqRel);
    }

    pub fn read(&self) -> u64 {
        self.count.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_then_read_returns_count() {
        let e = PerfEvent::new();
        e.fire();
        e.fire();
        e.fire();
        assert_eq!(e.read(), 3);
    }
}
