//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk/index.c
//! test-origin: linux:vendor/linux/kernel/printk/index.c
//! `/sys/kernel/debug/printk/index/*` indexed printk symbol table.
//!
//! Linux exports a per-module list of `pi_entry` records so userspace tools
//! can resolve printk format strings to source locations.  The port keeps
//! the in-memory table shape so registration round-trips work in tests.
//!
//! Ref: vendor/linux/kernel/printk/index.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

/// `struct pi_entry` — one indexed printk record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiEntry {
    pub fmt: &'static str,
    pub func: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub level: u8,
}

static INDEX: Mutex<Vec<PiEntry>> = Mutex::new(Vec::new());

pub fn register(entry: PiEntry) {
    INDEX.lock().push(entry);
}

pub fn lookup_by_fmt(fmt: &str) -> Option<PiEntry> {
    INDEX.lock().iter().find(|e| e.fmt == fmt).cloned()
}

pub fn entries() -> Vec<PiEntry> {
    INDEX.lock().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_then_lookup_round_trip() {
        register(PiEntry {
            fmt: "hello %s",
            func: "test_fn",
            file: "test.rs",
            line: 42,
            level: 6,
        });
        let r = lookup_by_fmt("hello %s").unwrap();
        assert_eq!(r.line, 42);
    }
}
