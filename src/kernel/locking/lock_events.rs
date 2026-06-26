//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/lock_events.c
//! test-origin: linux:vendor/linux/kernel/locking/lock_events.c
//! Lock event accounting coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/lock_events.c`.

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockEvent {
    Contended = 0,
    Wait = 1,
    Acquired = 2,
}

static EVENTS: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];

pub fn lockevent_inc(event: LockEvent) {
    EVENTS[event as usize].fetch_add(1, Ordering::AcqRel);
}

pub fn lockevent_count(event: LockEvent) -> u64 {
    EVENTS[event as usize].load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_counter_increments() {
        let before = lockevent_count(LockEvent::Contended);
        lockevent_inc(LockEvent::Contended);
        assert_eq!(lockevent_count(LockEvent::Contended), before + 1);
    }
}
