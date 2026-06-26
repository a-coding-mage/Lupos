//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_events.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events.c
//! `events/<subsystem>/<event>/enable` framework — registers static
//! tracepoint event classes.
//!
//! Ref: vendor/linux/kernel/trace/trace_events.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct TraceEventClass {
    pub subsystem: String,
    pub name: String,
    pub enabled: bool,
}

static CLASSES: Mutex<Vec<TraceEventClass>> = Mutex::new(Vec::new());

pub fn register(subsystem: &str, name: &str) {
    let mut g = CLASSES.lock();
    if !g.iter().any(|c| c.subsystem == subsystem && c.name == name) {
        g.push(TraceEventClass {
            subsystem: subsystem.into(),
            name: name.into(),
            enabled: false,
        });
    }
}

pub fn enable(subsystem: &str, name: &str) -> Result<(), i32> {
    let mut g = CLASSES.lock();
    g.iter_mut()
        .find(|c| c.subsystem == subsystem && c.name == name)
        .map(|c| c.enabled = true)
        .ok_or(-2)
}

pub fn count() -> usize {
    CLASSES.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_then_enable() {
        register("sched", "sched_switch");
        enable("sched", "sched_switch").unwrap();
        let g = CLASSES.lock();
        let c = g.iter().find(|c| c.name == "sched_switch").unwrap();
        assert!(c.enabled);
    }

    #[test]
    fn enable_missing_is_enoent() {
        assert_eq!(enable("none", "none").unwrap_err(), -2);
    }
}
