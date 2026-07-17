//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module/kallsyms.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/module
//! Runtime symbol tables for loaded modules.
//!
//! The ELF loader performs Linux's `simplify_symbols()` first, then provides
//! the retained symbol records here from its `post_relocation()` hook.  This
//! registry supports the same name/address consumers as module kallsyms while
//! leaving ELF parsing and section ownership in the loader.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSymbol {
    pub name: String,
    pub address: usize,
    pub size: usize,
    /// `nm`-compatible symbol type computed by Linux's `elf_type()` rules.
    pub symbol_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleAddressRange {
    pub start: usize,
    pub end: usize,
}

impl ModuleAddressRange {
    pub fn new(start: usize, bytes: usize) -> Option<Self> {
        Some(Self {
            start,
            end: start.checked_add(bytes)?,
        })
    }

    fn contains(&self, address: usize) -> bool {
        self.start <= address && address < self.end
    }
}

#[derive(Clone, Debug)]
pub struct ModuleKallsyms {
    pub module: String,
    pub symbols: Vec<ModuleSymbol>,
    pub ranges: Vec<ModuleAddressRange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSymbolMatch {
    pub module: String,
    pub name: String,
    pub address: usize,
    pub size: usize,
    pub offset: usize,
    pub symbol_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KallsymsError {
    DuplicateModule,
    InvalidRange,
}

lazy_static! {
    static ref MODULE_KALLSYMS: Mutex<Vec<ModuleKallsyms>> = Mutex::new(Vec::new());
}

/// Publish a module's relocated symbols during `post_relocation()`.
pub fn register(mut table: ModuleKallsyms) -> Result<(), KallsymsError> {
    if table.ranges.iter().any(|range| range.start > range.end) {
        return Err(KallsymsError::InvalidRange);
    }
    // Linux's lookup walks values; sorting here makes nearest-symbol lookup
    // deterministic while retaining duplicate aliases at the same address.
    table.symbols.sort_by(|left, right| {
        left.address
            .cmp(&right.address)
            .then(left.name.cmp(&right.name))
    });

    let mut modules = MODULE_KALLSYMS.lock();
    if modules.iter().any(|entry| entry.module == table.module) {
        return Err(KallsymsError::DuplicateModule);
    }
    modules.push(table);
    Ok(())
}

/// Remove kallsyms before the module's executable memory is released.
pub fn unregister(module: &str) {
    MODULE_KALLSYMS
        .lock()
        .retain(|entry| entry.module != module);
}

/// Implements `module_kallsyms_lookup_name()`, including `module:symbol`.
pub fn lookup_name(name: &str) -> Option<usize> {
    let (module_filter, symbol_name) = name
        .split_once(':')
        .map_or((None, name), |(module, symbol)| (Some(module), symbol));
    MODULE_KALLSYMS
        .lock()
        .iter()
        .filter(|table| module_filter.is_none_or(|module| table.module == module))
        .flat_map(|table| table.symbols.iter())
        .find(|symbol| symbol.name == symbol_name && symbol.address != 0)
        .map(|symbol| symbol.address)
}

/// Implements the module portion of `kallsyms_lookup()`.
pub fn lookup_address(address: usize) -> Option<ModuleSymbolMatch> {
    let modules = MODULE_KALLSYMS.lock();
    for table in modules.iter() {
        if !table.ranges.iter().any(|range| range.contains(address)) {
            continue;
        }

        let symbol = table
            .symbols
            .iter()
            .filter(|symbol| symbol.address != 0 && symbol.address <= address)
            .max_by_key(|symbol| symbol.address)?;
        let next_address = table
            .symbols
            .iter()
            .filter(|candidate| candidate.address > symbol.address)
            .map(|candidate| candidate.address)
            .min();
        let range_end = table
            .ranges
            .iter()
            .filter(|range| range.contains(address))
            .map(|range| range.end)
            .min()
            .unwrap_or(address);
        // Linux deliberately derives size from the next symbol rather than
        // trusting ELF `st_size`.
        let size = next_address
            .unwrap_or(range_end)
            .saturating_sub(symbol.address);
        return Some(ModuleSymbolMatch {
            module: table.module.to_string(),
            name: symbol.name.to_string(),
            address: symbol.address,
            size,
            offset: address.saturating_sub(symbol.address),
            symbol_type: symbol.symbol_type,
        });
    }
    None
}

pub fn for_each_symbol(mut callback: impl FnMut(&str, &str, usize) -> i32) -> i32 {
    let modules = MODULE_KALLSYMS.lock();
    for table in modules.iter() {
        for symbol in table.symbols.iter() {
            let result = callback(&table.module, &symbol.name, symbol.address);
            if result != 0 {
                return result;
            }
        }
    }
    0
}
