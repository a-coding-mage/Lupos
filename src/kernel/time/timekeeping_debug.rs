//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timekeeping_debug.c
//! test-origin: linux:vendor/linux/kernel/time/timekeeping_debug.c
//! Timekeeping debug coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timekeeping_debug.c`.

use core::sync::atomic::Ordering;

use super::timekeeping::TK;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimekeepingDebugSnapshot {
    pub realtime_ns: u64,
    pub monotonic_ns: u64,
    pub boottime_ns: u64,
    pub tai_offset: u64,
}

pub fn timekeeping_debug_snapshot() -> TimekeepingDebugSnapshot {
    let realtime_ns = TK
        .xtime_sec
        .load(Ordering::Acquire)
        .saturating_mul(1_000_000_000)
        .saturating_add(TK.xtime_nsec.load(Ordering::Acquire));
    TimekeepingDebugSnapshot {
        realtime_ns,
        monotonic_ns: TK.mono_ns.load(Ordering::Acquire),
        boottime_ns: TK.boot_ns.load(Ordering::Acquire),
        tai_offset: TK.tai_offset.load(Ordering::Acquire),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_contains_tai_offset() {
        assert!(timekeeping_debug_snapshot().tai_offset >= 37);
    }
}
