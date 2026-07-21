//! linux-parity: partial
//! linux-source: vendor/linux/kernel/time/clockevents.c
//! linux-source: vendor/linux/kernel/time/tick-common.c
//! test-origin: linux:vendor/linux/kernel/time/clockevents.c
//! test-origin: linux:vendor/linux/kernel/time/tick-common.c
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

/// CPU responsible for Linux `do_timer()`/`update_wall_time()` work.
///
/// Linux's `tick_do_timer_cpu` can hand this role off for NOHZ and CPU hotplug.
/// Lupos does not support either transition yet, so the always-online BSP owns
/// the role.  Keeping a single owner is essential: every CPU still receives a
/// local scheduler tick, but jiffies and wall time advance only once per period.
pub const TICK_DO_TIMER_CPU: usize = 0;

#[inline]
pub const fn tick_do_timer_cpu(cpu: usize) -> bool {
    cpu == TICK_DO_TIMER_CPU
}

/// Linux `tick_handle_periodic` — registered as the periodic callback.
///
/// Order:
///   1. `do_timer` (advances jiffies + wall clock).
///   2. `hrtimer_run_queues` (expire any due hrtimers).
///
/// `apic_timer::on_tick()` performs Linux's per-CPU
/// `update_process_times()`/`scheduler_tick()` work after this callback.
pub fn tick_handle_periodic() {
    tick_handle_periodic_for_cpu(crate::arch::x86::kernel::setup_percpu::current_cpu_number());
}

/// CPU-explicit periodic tick handler, mirroring `tick_periodic(cpu)`.
///
/// The hrtimer implementation currently has one global queue rather than
/// Linux's per-CPU bases.  Running that queue on the timekeeper CPU serializes
/// callback execution and avoids all LAPICs contending on the same locks.
pub fn tick_handle_periodic_for_cpu(cpu: usize) {
    PERIODIC_TICK_COUNT.fetch_add(1, Ordering::AcqRel);
    if tick_do_timer_cpu(cpu) {
        super::jiffies::tick_jiffies();
        super::timekeeping::tick_advance_walltime();
        super::hrtimer::hrtimer_run_queues();
    }
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
    fn only_timekeeper_cpu_owns_global_tick() {
        assert!(tick_do_timer_cpu(TICK_DO_TIMER_CPU));
        assert!(!tick_do_timer_cpu(TICK_DO_TIMER_CPU + 1));
    }

    #[test]
    fn clockevents_default_state_is_unused() {
        let ce = Clockevents::new(100, CLOCK_EVT_FEAT_PERIODIC);
        assert_eq!(ce.mode, ClockEventMode::Unused);
    }
}
