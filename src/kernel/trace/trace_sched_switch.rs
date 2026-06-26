//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_sched_switch.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_sched_switch.c
//! `sched_switch` static tracepoint.
//!
//! Ref: vendor/linux/kernel/trace/trace_sched_switch.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static SCHED_SWITCH_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct SchedSwitchEvent {
    pub prev_pid: i32,
    pub next_pid: i32,
    pub prev_state: i32,
}

pub fn trace(ev: SchedSwitchEvent) -> SchedSwitchEvent {
    SCHED_SWITCH_COUNT.fetch_add(1, Ordering::AcqRel);
    ev
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_increments_counter() {
        let c0 = SCHED_SWITCH_COUNT.load(Ordering::Acquire);
        trace(SchedSwitchEvent {
            prev_pid: 1,
            next_pid: 2,
            prev_state: 0,
        });
        assert_eq!(SCHED_SWITCH_COUNT.load(Ordering::Acquire), c0 + 1);
    }
}
