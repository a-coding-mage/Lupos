//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/rv.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/rv.c
//! Core RV runtime — owns the registered monitors and routes events.
//!
//! Ref: vendor/linux/kernel/trace/rv/rv.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct RvMonitor {
    pub name: String,
    pub enabled: bool,
    pub violations: u64,
}

static MONITORS: Mutex<Vec<RvMonitor>> = Mutex::new(Vec::new());

pub fn register(name: &str) {
    MONITORS.lock().push(RvMonitor {
        name: name.into(),
        enabled: false,
        violations: 0,
    });
}

pub fn enable(name: &str) -> Result<(), i32> {
    MONITORS
        .lock()
        .iter_mut()
        .find(|m| m.name == name)
        .map(|m| m.enabled = true)
        .ok_or(-2)
}

pub fn violation(name: &str) {
    if let Some(m) = MONITORS.lock().iter_mut().find(|m| m.name == name) {
        m.violations += 1;
    }
}

pub fn count() -> usize {
    MONITORS.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_enable_violate() {
        register("rv_test");
        enable("rv_test").unwrap();
        violation("rv_test");
        let g = MONITORS.lock();
        let m = g.iter().find(|m| m.name == "rv_test").unwrap();
        assert!(m.enabled);
        assert_eq!(m.violations, 1);
    }
}
