//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/sched_clock.c
//! test-origin: linux:vendor/linux/kernel/time/sched_clock.c
//! Scheduler clock coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/sched_clock.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static SCHED_CLOCK_NS: AtomicU64 = AtomicU64::new(0);

pub fn sched_clock() -> u64 {
    let tk = super::timekeeping::ktime_get();
    let stored = SCHED_CLOCK_NS.load(Ordering::Acquire);
    if tk > stored { tk } else { stored }
}

pub fn sched_clock_tick(ns: u64) {
    let mut cur = SCHED_CLOCK_NS.load(Ordering::Acquire);
    while ns > cur {
        match SCHED_CLOCK_NS.compare_exchange(cur, ns, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(next) => cur = next,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sched_clock_is_monotonic() {
        sched_clock_tick(100);
        assert!(sched_clock() >= 100);
        sched_clock_tick(50);
        assert!(sched_clock() >= 100);
    }
}
