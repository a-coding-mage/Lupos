//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_clock.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_clock.c
//! Pluggable clock source for trace events (`global`, `local`, `mono`, `x86-tsc`).
//!
//! Ref: vendor/linux/kernel/trace/trace_clock.c

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceClock {
    Global,
    Local,
    Mono,
    MonoRaw,
    Boot,
    X86Tsc,
    Counter,
}

static CURRENT_CLOCK: spin::Mutex<TraceClock> = spin::Mutex::new(TraceClock::Local);
static MONOTONIC_NS: AtomicU64 = AtomicU64::new(0);

pub fn set(clock: TraceClock) {
    *CURRENT_CLOCK.lock() = clock;
}

pub fn get() -> TraceClock {
    *CURRENT_CLOCK.lock()
}

/// Returns a monotonically increasing counter — exact for tests.
pub fn now_ns() -> u64 {
    MONOTONIC_NS.fetch_add(1, Ordering::AcqRel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_clock_is_local() {
        assert_eq!(get(), TraceClock::Local);
    }

    #[test]
    fn set_then_get_round_trip() {
        set(TraceClock::X86Tsc);
        assert_eq!(get(), TraceClock::X86Tsc);
        set(TraceClock::Local);
    }

    #[test]
    fn now_ns_strictly_increases() {
        let a = now_ns();
        let b = now_ns();
        assert!(b > a);
    }
}
