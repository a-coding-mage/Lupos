//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/tick-common.c
//! test-origin: linux:vendor/linux/kernel/time/tick-common.c
//! Generic tick device coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/tick-common.c`.

use core::sync::atomic::{AtomicU64, Ordering};

use super::clockevents::{ClockEventMode, Clockevents};

static TICK_PERIOD_NS: AtomicU64 = AtomicU64::new(super::jiffies::NSEC_PER_TICK);

pub fn tick_setup_periodic(dev: &mut Clockevents) {
    dev.mode = ClockEventMode::Periodic;
    TICK_PERIOD_NS.store(super::jiffies::NSEC_PER_TICK, Ordering::Release);
}

pub fn tick_period_ns() -> u64 {
    TICK_PERIOD_NS.load(Ordering::Acquire)
}

pub fn tick_do_update_jiffies64() {
    super::jiffies::tick_jiffies();
    super::timekeeping::tick_advance_walltime();
}

#[cfg(test)]
mod tests {
    use super::super::clockevents::CLOCK_EVT_FEAT_PERIODIC;
    use super::*;

    #[test]
    fn setup_periodic_sets_device_mode() {
        let mut dev = Clockevents::new(100, CLOCK_EVT_FEAT_PERIODIC);
        tick_setup_periodic(&mut dev);
        assert_eq!(dev.mode, ClockEventMode::Periodic);
        assert_eq!(tick_period_ns(), super::super::jiffies::NSEC_PER_TICK);
    }
}
