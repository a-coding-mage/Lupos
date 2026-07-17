//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module
//! test-origin: linux:vendor/linux/kernel/module
//! EXPORT_SYMBOL registry — `include/linux/export.h:89`.
//!
//! Linux builds a `__ksymtab` ELF section at link time.  We maintain a
//! runtime table instead: each `export_symbol!` call pushes one
//! `ExportedSymbol` entry.  The module loader uses `find_symbol` to resolve
//! undefined references in `.ko` files.
//!
//! For M56 the table is hand-populated; a linker-section approach can land
//! later once we have a custom `build.rs` step that emits `__ksymtab`.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

/// One exported kernel symbol — mirrors `struct kernel_symbol`.
pub struct ExportedSymbol {
    pub name: String,
    /// Absolute virtual address of the symbol in the kernel image.
    pub addr: usize,
    pub gpl_only: bool,
    /// Optional genksyms/gendwarfksyms CRC from `__kcrctab`.
    ///
    /// Linux deliberately accepts an unversioned exporting symbol even when
    /// the importing module carries a version record (and taints the kernel).
    /// `None` preserves that distinction; a synthetic zero CRC would not.
    pub crc: Option<u32>,
    pub owner: Option<String>,
}

struct SymbolRegistry {
    entries: Vec<ExportedSymbol>,
    by_name: Vec<(String, usize)>,
}

impl SymbolRegistry {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            by_name: Vec::new(),
        }
    }

    fn lookup_index(&self, name: &str) -> Result<usize, usize> {
        self.by_name
            .binary_search_by(|(symbol_name, _)| symbol_name.as_str().cmp(name))
    }

    fn insert_lookup_if_absent(&mut self, name: &str, entry_index: usize) {
        if let Err(index) = self.lookup_index(name) {
            self.by_name.insert(index, (name.to_string(), entry_index));
        }
    }

    fn rebuild_lookup(&mut self) {
        self.by_name.clear();
        for (index, symbol) in self.entries.iter().enumerate() {
            if let Err(lookup_index) = self.lookup_index(symbol.name.as_str()) {
                self.by_name
                    .insert(lookup_index, (symbol.name.clone(), index));
            }
        }
    }

    fn push(&mut self, symbol: ExportedSymbol) {
        let index = self.entries.len();
        self.insert_lookup_if_absent(symbol.name.as_str(), index);
        self.entries.push(symbol);
    }

    fn find_addr(&self, name: &str) -> Option<usize> {
        self.lookup_index(name)
            .ok()
            .and_then(|index| self.by_name.get(index))
            .map(|(_, entry_index)| *entry_index)
            .and_then(|index| self.entries.get(index))
            .map(|symbol| symbol.addr)
    }

    fn find_gpl_only(&self, name: &str) -> Option<bool> {
        self.lookup_index(name)
            .ok()
            .and_then(|index| self.by_name.get(index))
            .map(|(_, entry_index)| *entry_index)
            .and_then(|index| self.entries.get(index))
            .map(|symbol| symbol.gpl_only)
    }

    fn find_crc(&self, name: &str) -> Option<u32> {
        self.lookup_index(name)
            .ok()
            .and_then(|index| self.by_name.get(index))
            .map(|(_, entry_index)| *entry_index)
            .and_then(|index| self.entries.get(index))
            .and_then(|symbol| symbol.crc)
    }
}

lazy_static! {
    static ref KSYMTAB: Mutex<SymbolRegistry> = Mutex::new(SymbolRegistry::new());
}

/// Register one symbol in the export table.
/// Call from `kernel_main` (or anywhere before modules are loaded).
pub fn export_symbol(name: &str, addr: usize, gpl_only: bool) {
    export_symbol_with_crc(name, addr, gpl_only, None);
}

/// Register one built-in export and its optional module-version CRC.
///
/// Mirrors the pairing of `__ksymtab` with `__kcrctab` in
/// `vendor/linux/kernel/module/main.c::find_symbol`.
pub fn export_symbol_with_crc(name: &str, addr: usize, gpl_only: bool, crc: Option<u32>) {
    KSYMTAB.lock().push(ExportedSymbol {
        name: name.to_string(),
        addr,
        gpl_only,
        crc,
        owner: None,
    });
}

/// Register one symbol exported by a loaded Linux module.
///
/// Mirrors the module-owned `mod->syms` search path in
/// `vendor/linux/kernel/module/main.c:find_symbol`.
pub fn export_module_symbol(owner: &str, name: &str, addr: usize, gpl_only: bool) {
    export_module_symbol_with_crc(owner, name, addr, gpl_only, None);
}

/// Register one module-owned export and its optional version CRC.
pub fn export_module_symbol_with_crc(
    owner: &str,
    name: &str,
    addr: usize,
    gpl_only: bool,
    crc: Option<u32>,
) {
    let mut table = KSYMTAB.lock();
    if let Some(symbol) = table
        .entries
        .iter_mut()
        .find(|symbol| symbol.owner.as_deref() == Some(owner) && symbol.name == name)
    {
        symbol.addr = addr;
        symbol.gpl_only = gpl_only;
        symbol.crc = crc;
        return;
    }

    table.push(ExportedSymbol {
        name: name.to_string(),
        addr,
        gpl_only,
        crc,
        owner: Some(owner.to_string()),
    });
}

/// Drop every symbol owned by a Linux module being unloaded or rejected.
pub fn unexport_module_symbols(owner: &str) {
    let mut table = KSYMTAB.lock();
    table
        .entries
        .retain(|symbol| symbol.owner.as_deref() != Some(owner));
    table.rebuild_lookup();
}

/// Look up a symbol by name.  Returns its address or `None`.
pub fn find_symbol(name: &str) -> Option<usize> {
    KSYMTAB.lock().find_addr(name)
}

/// Look up whether a symbol is GPL-only.  Returns `None` for unknown symbols.
pub fn find_symbol_gpl_only(name: &str) -> Option<bool> {
    KSYMTAB.lock().find_gpl_only(name)
}

/// Return the exporting symbol's module-version CRC, if it has one.
pub fn find_symbol_crc(name: &str) -> Option<u32> {
    KSYMTAB.lock().find_crc(name)
}

/// Number of exported symbols (diagnostic).
pub fn symbol_count() -> usize {
    KSYMTAB.lock().entries.len()
}

/// Names of all exported symbols (diagnostic / `/proc/kallsyms` stub).
pub fn symbol_names() -> Vec<String> {
    KSYMTAB
        .lock()
        .entries
        .iter()
        .map(|s| s.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_and_find() {
        static DUMMY: u64 = 42;
        export_symbol("lupos_dummy_test", &DUMMY as *const u64 as usize, false);
        let addr = find_symbol("lupos_dummy_test");
        assert!(addr.is_some());
        assert_eq!(addr.unwrap(), &DUMMY as *const u64 as usize);
    }

    #[test]
    fn unknown_symbol_is_none() {
        assert!(find_symbol("__this_symbol_does_not_exist__").is_none());
    }

    #[test]
    fn gpl_only_metadata_uses_lookup_precedence() {
        export_symbol("lupos_gpl_metadata_test", 1, true);
        export_module_symbol(
            "sample_mod_gpl_metadata",
            "lupos_gpl_metadata_test",
            2,
            false,
        );

        assert_eq!(find_symbol_gpl_only("lupos_gpl_metadata_test"), Some(true));
        unexport_module_symbols("sample_mod_gpl_metadata");
        assert_eq!(find_symbol_gpl_only("lupos_gpl_metadata_test"), Some(true));
    }

    #[test]
    fn module_owned_symbols_are_removed_together() {
        static DUMMY: u64 = 7;
        export_module_symbol(
            "sample_mod",
            "sample_export",
            &DUMMY as *const u64 as usize,
            true,
        );
        assert_eq!(
            find_symbol("sample_export"),
            Some(&DUMMY as *const u64 as usize)
        );
        unexport_module_symbols("sample_mod");
        assert!(find_symbol("sample_export").is_none());
    }

    #[test]
    fn builtin_symbol_keeps_precedence_over_module_duplicate() {
        static BUILTIN: u64 = 11;
        static MODULE: u64 = 12;
        export_symbol(
            "lupos_precedence_test",
            &BUILTIN as *const u64 as usize,
            false,
        );
        export_module_symbol(
            "sample_mod_precedence",
            "lupos_precedence_test",
            &MODULE as *const u64 as usize,
            false,
        );

        assert_eq!(
            find_symbol("lupos_precedence_test"),
            Some(&BUILTIN as *const u64 as usize)
        );
        unexport_module_symbols("sample_mod_precedence");
        assert_eq!(
            find_symbol("lupos_precedence_test"),
            Some(&BUILTIN as *const u64 as usize)
        );
    }

    #[test]
    fn module_duplicate_lookup_falls_back_after_unexport() {
        static FIRST: u64 = 21;
        static SECOND: u64 = 22;
        export_module_symbol(
            "sample_mod_first",
            "sample_module_duplicate",
            &FIRST as *const u64 as usize,
            false,
        );
        export_module_symbol(
            "sample_mod_second",
            "sample_module_duplicate",
            &SECOND as *const u64 as usize,
            false,
        );

        assert_eq!(
            find_symbol("sample_module_duplicate"),
            Some(&FIRST as *const u64 as usize)
        );
        unexport_module_symbols("sample_mod_first");
        assert_eq!(
            find_symbol("sample_module_duplicate"),
            Some(&SECOND as *const u64 as usize)
        );
        unexport_module_symbols("sample_mod_second");
        assert!(find_symbol("sample_module_duplicate").is_none());
    }
}
