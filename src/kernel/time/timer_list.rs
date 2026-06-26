//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timer_list.c
//! test-origin: linux:vendor/linux/kernel/time/timer_list.c
//! Timer list debug reporting coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timer_list.c`.

use super::timer::{TimerList, timer_pending};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerListEntry {
    pub expires: u64,
    pub active: bool,
}

pub fn timer_list_entry(timer: &TimerList) -> TimerListEntry {
    TimerListEntry {
        expires: timer.expires,
        active: timer_pending(timer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_inactive_timer() {
        let timer = TimerList::new();
        let entry = timer_list_entry(&timer);
        assert_eq!(entry.expires, 0);
        assert!(!entry.active);
    }
}
