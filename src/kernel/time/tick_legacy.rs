//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/tick-legacy.c
//! test-origin: linux:vendor/linux/kernel/time/tick-legacy.c
//! Legacy PIT tick coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/tick-legacy.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static LEGACY_TICKS: AtomicU64 = AtomicU64::new(0);

pub fn legacy_timer_tick() {
    LEGACY_TICKS.fetch_add(1, Ordering::AcqRel);
    super::clockevents::tick_handle_periodic();
}

pub fn legacy_tick_count() -> u64 {
    LEGACY_TICKS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_tick_counter_advances() {
        let before = legacy_tick_count();
        legacy_timer_tick();
        assert_eq!(legacy_tick_count(), before + 1);
    }
}
