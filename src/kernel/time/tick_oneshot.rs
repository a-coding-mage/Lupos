//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/tick-oneshot.c
//! test-origin: linux:vendor/linux/kernel/time/tick-oneshot.c
//! One-shot tick coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/tick-oneshot.c`.

use super::clockevents::{CLOCK_EVT_FEAT_ONESHOT, ClockEventMode, Clockevents};

pub fn tick_switch_to_oneshot(dev: &mut Clockevents) -> bool {
    if dev.features & CLOCK_EVT_FEAT_ONESHOT == 0 {
        return false;
    }
    dev.mode = ClockEventMode::Oneshot;
    true
}

pub fn tick_resume_oneshot(dev: &mut Clockevents) {
    if dev.features & CLOCK_EVT_FEAT_ONESHOT != 0 {
        dev.mode = ClockEventMode::Oneshot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_requires_oneshot_feature() {
        let mut dev = Clockevents::new(100, 0);
        assert!(!tick_switch_to_oneshot(&mut dev));
        dev.features = CLOCK_EVT_FEAT_ONESHOT;
        assert!(tick_switch_to_oneshot(&mut dev));
        assert_eq!(dev.mode, ClockEventMode::Oneshot);
    }
}
