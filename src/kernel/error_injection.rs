//! linux-parity: partial
//! linux-source: vendor/linux/lib/error-inject.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/bpf
//! Module lifecycle for `_error_injection_whitelist`.

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

pub const ERROR_INJECTION_ENTRY_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErrorInjectionEntry {
    pub owner: usize,
    pub start: usize,
    pub end: usize,
    pub error_type: i32,
}

static ENTRIES: Mutex<Vec<ErrorInjectionEntry>> = Mutex::new(Vec::new());

/// `module_load_ei_list()` at `MODULE_STATE_COMING`.
///
/// Linux ignores whitelist records that do not resolve to kernel text with a
/// known symbol size.  Module kallsyms therefore has to be published before
/// this hook is invoked.
pub fn module_coming(owner: usize, section: &[u8]) -> Result<(), i32> {
    if section.len() % ERROR_INJECTION_ENTRY_SIZE != 0 {
        return Err(-8); // ENOEXEC
    }

    let mut additions = Vec::new();
    for record in section.chunks_exact(ERROR_INJECTION_ENTRY_SIZE) {
        let address = usize::from_le_bytes(record[0..8].try_into().map_err(|_| -8)?);
        let error_type = i32::from_le_bytes(record[8..12].try_into().map_err(|_| -8)?);
        if !(0..=3).contains(&error_type) {
            continue;
        }
        let Some(symbol) = crate::kernel::module::kallsyms::lookup_address(address) else {
            continue;
        };
        if symbol.address != address || symbol.size == 0 {
            continue;
        }
        let Some(end) = address.checked_add(symbol.size) else {
            continue;
        };
        additions.push(ErrorInjectionEntry {
            owner,
            start: address,
            end,
            error_type,
        });
    }

    let mut entries = ENTRIES.lock();
    if entries.iter().any(|entry| entry.owner == owner) {
        return Err(-17); // EEXIST
    }
    entries.extend(additions);
    Ok(())
}

/// `module_unload_ei_list()` at `MODULE_STATE_GOING`.
pub fn module_going(owner: usize) {
    ENTRIES.lock().retain(|entry| entry.owner != owner);
}

pub fn within_error_injection_list(address: usize) -> bool {
    ENTRIES
        .lock()
        .iter()
        .any(|entry| address >= entry.start && address < entry.end)
}

pub fn get_injectable_error_type(address: usize) -> Result<i32, i32> {
    ENTRIES
        .lock()
        .iter()
        .find(|entry| address >= entry.start && address < entry.end)
        .map(|entry| entry.error_type)
        .ok_or(-22) // EINVAL
}

pub fn module_entries(owner: usize) -> Vec<ErrorInjectionEntry> {
    ENTRIES
        .lock()
        .iter()
        .filter(|entry| entry.owner == owner)
        .copied()
        .collect()
}
