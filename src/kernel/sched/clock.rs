//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched/clock.c
//! test-origin: linux:vendor/linux/kernel/sched/clock.c
//! Scheduler clock helpers.
//!
//! Provides the Linux scheduler-clock utility surface around the shared
//! high-resolution clock. Full unstable-clock and per-CPU synchronization from
//! `vendor/linux/kernel/sched/clock.c` is not yet implemented.

use core::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

pub use super::entity::SCHED_CLOCK_NS;

/// Periodic scheduler-tick duration for the configured `HZ=250`.
/// `sched_clock_ns()` itself is high-resolution and is not tick-derived.
pub const NSEC_PER_SCHED_TICK: u64 = crate::kernel::time::jiffies::NSEC_PER_TICK;
pub const SCHED_CLOCK_STABLE_LOG_PREFIX: &str = "sched_clock: Marking stable";

static SCHED_CLOCK_STABLE: AtomicBool = AtomicBool::new(false);
static SCHED_CLOCK_OFFSET: AtomicI64 = AtomicI64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedClockStableTransition {
    pub tick_gtod: u64,
    pub gtod_offset: i64,
    pub tick_raw: u64,
    pub sched_clock_offset: i64,
}

pub fn sched_clock_ns() -> u64 {
    super::entity::sched_clock_ns()
}

pub fn sched_clock_cpu(_cpu: u32) -> u64 {
    sched_clock_ns()
}

pub fn local_clock() -> u64 {
    sched_clock_cpu(super::current_cpu())
}

/// `local_clock` - `vendor/linux/kernel/sched/clock.c:317`.
#[unsafe(export_name = "local_clock")]
pub extern "C" fn linux_local_clock() -> u64 {
    local_clock()
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("local_clock", linux_local_clock as usize, true);
}

pub const fn sched_clock_from_ticks(ticks: u64, nsec_per_tick: u64) -> u64 {
    ticks.saturating_mul(nsec_per_tick)
}

const fn clamp_u64_to_i64(value: u64) -> i64 {
    if value > i64::MAX as u64 {
        i64::MAX
    } else {
        value as i64
    }
}

const fn signed_delta(lhs: u64, rhs: u64) -> i64 {
    if lhs >= rhs {
        clamp_u64_to_i64(lhs - rhs)
    } else {
        -clamp_u64_to_i64(rhs - lhs)
    }
}

pub const fn stable_transition(tick_gtod: u64, tick_raw: u64) -> SchedClockStableTransition {
    SchedClockStableTransition {
        tick_gtod,
        gtod_offset: 0,
        tick_raw,
        sched_clock_offset: signed_delta(tick_gtod, tick_raw),
    }
}

pub fn sched_clock_stable() -> bool {
    SCHED_CLOCK_STABLE.load(Ordering::Acquire)
}

pub fn sched_clock_offset() -> i64 {
    SCHED_CLOCK_OFFSET.load(Ordering::Acquire)
}

pub fn mark_sched_clock_stable() -> Option<SchedClockStableTransition> {
    if SCHED_CLOCK_STABLE.swap(true, Ordering::AcqRel) {
        return None;
    }

    let tick_gtod = crate::kernel::time::timekeeping::ktime_get();
    let tick_raw = sched_clock_ns();
    let transition = stable_transition(tick_gtod, tick_raw);
    SCHED_CLOCK_OFFSET.store(transition.sched_clock_offset, Ordering::Release);
    Some(transition)
}

/// Linux's `sched_clock_init_late()` marks the scheduler clock stable once
/// late built-in CPU/idle initialisation has had a chance to reject TSC. Lupos
/// currently accepts the TSC clocksource during boot, so emit the same stable
/// transition line when the scheduler clock is committed.
pub fn sched_clock_init_late() {
    if let Some(t) = mark_sched_clock_stable() {
        crate::log_info!(
            "",
            "{} ({}, {})->({}, {})",
            SCHED_CLOCK_STABLE_LOG_PREFIX,
            t.tick_gtod,
            t.gtod_offset,
            t.tick_raw,
            t.sched_clock_offset
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sched_clock_tick_conversion_saturates() {
        assert_eq!(sched_clock_from_ticks(2, NSEC_PER_SCHED_TICK), 8_000_000);
        assert_eq!(sched_clock_from_ticks(u64::MAX, 2), u64::MAX);
    }

    #[test]
    fn stable_transition_matches_linux_marking_shape() {
        let transition = stable_transition(120, 200);
        assert_eq!(SCHED_CLOCK_STABLE_LOG_PREFIX, "sched_clock: Marking stable");
        assert_eq!(
            transition,
            SchedClockStableTransition {
                tick_gtod: 120,
                gtod_offset: 0,
                tick_raw: 200,
                sched_clock_offset: -80,
            }
        );
    }

    #[test]
    fn local_clock_export_matches_vendor_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/sched/clock.c"
        ));

        assert!(source.contains("u64 local_clock(void)"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(local_clock);"));

        register_module_exports();
        assert_eq!(find_symbol("local_clock"), Some(linux_local_clock as usize));
        assert!(linux_local_clock() <= sched_clock_ns());
    }
}
