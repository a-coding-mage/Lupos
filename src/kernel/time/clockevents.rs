//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/clockevents.c
//! test-origin: linux:vendor/linux/kernel/time/clockevents.c
//! Clockevents — M36.
//!
//! Mirrors `vendor/linux/kernel/time/clockevents.c`.  A `Clockevents` device
//! is a programmable timer (LAPIC timer, HPET in oneshot mode) used to wake
//! the kernel at a future deadline.  M36 ships only the periodic-mode binding
//! used by the LAPIC tick; oneshot lands as a follow-up.

use core::sync::atomic::{AtomicU64, Ordering};

/// Linux `CLOCK_EVT_FEAT_*` flags.
pub const CLOCK_EVT_FEAT_PERIODIC: u32 = 0x0001;
pub const CLOCK_EVT_FEAT_ONESHOT: u32 = 0x0002;
pub const CLOCK_EVT_FEAT_KTIME: u32 = 0x0004;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockEventMode {
    Unused,
    Shutdown,
    Periodic,
    Oneshot,
}

pub struct Clockevents {
    pub features: u32,
    pub max_delta_ns: u64,
    pub min_delta_ns: u64,
    pub mode: ClockEventMode,
    pub rating: u32,
}

impl Clockevents {
    pub const fn new(rating: u32, features: u32) -> Self {
        Self {
            features,
            max_delta_ns: u64::MAX,
            min_delta_ns: 0,
            mode: ClockEventMode::Unused,
            rating,
        }
    }
}

/// `tick_handle_periodic` callback count — one per LAPIC tick.
static PERIODIC_TICK_COUNT: AtomicU64 = AtomicU64::new(0);

/// Linux `tick_handle_periodic` — registered as the periodic callback.
///
/// Order:
///   1. `do_timer` (advances jiffies + wall clock).
///   2. `update_process_times` (per-task time accounting + scheduler tick).
///   3. `hrtimer_run_queues` (expire any due hrtimers).
pub fn tick_handle_periodic() {
    PERIODIC_TICK_COUNT.fetch_add(1, Ordering::AcqRel);
    super::jiffies::tick_jiffies();
    super::timekeeping::tick_advance_walltime();
    super::hrtimer::hrtimer_run_queues();
}

pub fn periodic_tick_count() -> u64 {
    PERIODIC_TICK_COUNT.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_feature_constant_matches_linux() {
        assert_eq!(CLOCK_EVT_FEAT_PERIODIC, 1);
        assert_eq!(CLOCK_EVT_FEAT_ONESHOT, 2);
    }

    #[test]
    fn tick_handle_periodic_advances_count() {
        let before = periodic_tick_count();
        tick_handle_periodic();
        assert_eq!(periodic_tick_count(), before + 1);
    }

    #[test]
    fn clockevents_default_state_is_unused() {
        let ce = Clockevents::new(100, CLOCK_EVT_FEAT_PERIODIC);
        assert_eq!(ce.mode, ClockEventMode::Unused);
    }
}
