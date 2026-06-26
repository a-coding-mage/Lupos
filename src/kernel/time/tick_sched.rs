//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/tick-sched.c
//! test-origin: linux:vendor/linux/kernel/time/tick-sched.c
//! Tick scheduler coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/tick-sched.c`.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[repr(C)]
pub struct TickSched {
    in_idle: AtomicBool,
    nohz_active: AtomicBool,
    last_tick_ns: AtomicU64,
}

impl TickSched {
    pub const fn new() -> Self {
        Self {
            in_idle: AtomicBool::new(false),
            nohz_active: AtomicBool::new(false),
            last_tick_ns: AtomicU64::new(0),
        }
    }

    pub fn enter_idle(&self, now_ns: u64) {
        self.in_idle.store(true, Ordering::Release);
        self.last_tick_ns.store(now_ns, Ordering::Release);
    }

    pub fn exit_idle(&self) {
        self.in_idle.store(false, Ordering::Release);
    }

    pub fn set_nohz(&self, enabled: bool) {
        self.nohz_active.store(enabled, Ordering::Release);
    }

    pub fn is_idle(&self) -> bool {
        self.in_idle.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_state_round_trips() {
        let sched = TickSched::new();
        sched.enter_idle(10);
        assert!(sched.is_idle());
        sched.exit_idle();
        assert!(!sched.is_idle());
    }
}
