//! linux-parity: partial
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

/// Opaque address of vendor Linux's packed `struct pi_entry`, referenced by
/// one pointer in a module `.printk_index` section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModulePiEntry {
    pub owner: usize,
    pub address: usize,
}

static MODULE_INDEX: Mutex<Vec<ModulePiEntry>> = Mutex::new(Vec::new());

pub fn register(entry: PiEntry) {
    INDEX.lock().push(entry);
}

pub fn lookup_by_fmt(fmt: &str) -> Option<PiEntry> {
    INDEX.lock().iter().find(|e| e.fmt == fmt).cloned()
}

pub fn entries() -> Vec<PiEntry> {
    INDEX.lock().clone()
}

/// `pi_module_notify(MODULE_STATE_COMING)`.  Linux exposes these entries via
/// `/sys/kernel/debug/printk/index/<module>` without copying the packed
/// objects, so their lifetime is exactly the module lifetime.
pub fn module_coming(owner: usize, section: &[u8]) -> Result<(), i32> {
    if section.len() % core::mem::size_of::<usize>() != 0 {
        return Err(-8); // ENOEXEC
    }

    let mut entries = Vec::with_capacity(section.len() / core::mem::size_of::<usize>());
    for bytes in section.chunks_exact(core::mem::size_of::<usize>()) {
        let address = usize::from_le_bytes(bytes.try_into().map_err(|_| -8)?);
        if address == 0 {
            return Err(-8);
        }
        entries.push(ModulePiEntry { owner, address });
    }

    let mut index = MODULE_INDEX.lock();
    if index.iter().any(|entry| entry.owner == owner) {
        return Err(-17); // EEXIST
    }
    index.extend(entries);
    Ok(())
}

/// `pi_module_notify(MODULE_STATE_GOING)` removes the debugfs view before its
/// `pi_entry` pointers become invalid.
pub fn module_going(owner: usize) {
    MODULE_INDEX.lock().retain(|entry| entry.owner != owner);
}

pub fn module_entries(owner: usize) -> Vec<ModulePiEntry> {
    MODULE_INDEX
        .lock()
        .iter()
        .filter(|entry| entry.owner == owner)
        .copied()
        .collect()
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
