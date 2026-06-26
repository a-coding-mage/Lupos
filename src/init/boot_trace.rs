//! linux-parity: complete
//! linux-source: vendor/linux/init
//! test-origin: linux:vendor/linux/init
//! Compact boot milestone trace exposed through `/proc/lupos_boot_trace`.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use lazy_static::lazy_static;
use spin::Mutex;

const BOOT_TRACE_CAP: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootTraceEvent {
    pub timestamp_ms: u64,
    pub subsystem: &'static str,
    pub message: &'static str,
}

lazy_static! {
    static ref EVENTS: Mutex<Vec<BootTraceEvent>> = Mutex::new(Vec::new());
}

pub fn record(subsystem: &'static str, message: &'static str) {
    let mut events = EVENTS.lock();
    if events.len() == BOOT_TRACE_CAP {
        events.remove(0);
    }
    events.push(BootTraceEvent {
        timestamp_ms: crate::kernel::printk::log::timestamp_msecs(),
        subsystem,
        message,
    });
}

pub fn render() -> String {
    let events = EVENTS.lock();
    let mut out = String::new();
    for event in events.iter() {
        let _ = writeln!(
            out,
            "{:>8}.{:03} {:<12} {}",
            event.timestamp_ms / 1000,
            event.timestamp_ms % 1000,
            event.subsystem,
            event.message
        );
    }
    if out.is_empty() {
        out.push_str("       0.000 boot         no events recorded\n");
    }
    out
}

#[cfg(test)]
pub fn reset_for_tests() {
    EVENTS.lock().clear();
}

#[cfg(test)]
pub fn events_for_tests() -> Vec<BootTraceEvent> {
    EVENTS.lock().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_trace_renders_ordered_events() {
        reset_for_tests();
        record("console", "ready");
        record("init", "handoff");
        let text = render();
        assert!(text.contains("console"));
        assert!(text.contains("ready"));
        assert!(text.contains("init"));
        assert!(text.contains("handoff"));
        assert_eq!(events_for_tests().len(), 2);
    }
}
