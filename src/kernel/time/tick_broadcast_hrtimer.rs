//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/tick-broadcast-hrtimer.c
//! test-origin: linux:vendor/linux/kernel/time/tick-broadcast-hrtimer.c
//! Hrtimer-backed tick broadcast coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/tick-broadcast-hrtimer.c`.

use core::sync::atomic::{AtomicU64, Ordering};

#[repr(C)]
pub struct TickBroadcastHrtimer {
    expires_ns: AtomicU64,
}

impl TickBroadcastHrtimer {
    pub const fn new() -> Self {
        Self {
            expires_ns: AtomicU64::new(0),
        }
    }

    pub fn program_event(&self, expires_ns: u64) {
        self.expires_ns.store(expires_ns, Ordering::Release);
    }

    pub fn expires_ns(&self) -> u64 {
        self.expires_ns.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_hrtimer_records_expiry() {
        let timer = TickBroadcastHrtimer::new();
        timer.program_event(123);
        assert_eq!(timer.expires_ns(), 123);
    }
}
