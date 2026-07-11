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
    pub owner: Option<String>,
}

lazy_static! {
    static ref KSYMTAB: Mutex<Vec<ExportedSymbol>> = Mutex::new(Vec::new());
}

/// Register one symbol in the export table.
/// Call from `kernel_main` (or anywhere before modules are loaded).
pub fn export_symbol(name: &str, addr: usize, gpl_only: bool) {
    KSYMTAB.lock().push(ExportedSymbol {
        name: name.to_string(),
        addr,
        gpl_only,
        owner: None,
    });
}

/// Register one symbol exported by a loaded Linux module.
///
/// Mirrors the module-owned `mod->syms` search path in
/// `vendor/linux/kernel/module/main.c:find_symbol`.
pub fn export_module_symbol(owner: &str, name: &str, addr: usize, gpl_only: bool) {
    let mut table = KSYMTAB.lock();
    if let Some(symbol) = table
        .iter_mut()
        .find(|symbol| symbol.owner.as_deref() == Some(owner) && symbol.name == name)
    {
        symbol.addr = addr;
        symbol.gpl_only = gpl_only;
        return;
    }

    table.push(ExportedSymbol {
        name: name.to_string(),
        addr,
        gpl_only,
        owner: Some(owner.to_string()),
    });
}

/// Drop every symbol owned by a Linux module being unloaded or rejected.
pub fn unexport_module_symbols(owner: &str) {
    KSYMTAB
        .lock()
        .retain(|symbol| symbol.owner.as_deref() != Some(owner));
}

/// Look up a symbol by name.  Returns its address or `None`.
pub fn find_symbol(name: &str) -> Option<usize> {
    KSYMTAB
        .lock()
        .iter()
        .find(|s| s.name == name)
        .map(|s| s.addr)
}

/// Number of exported symbols (diagnostic).
pub fn symbol_count() -> usize {
    KSYMTAB.lock().len()
}

/// Names of all exported symbols (diagnostic / `/proc/kallsyms` stub).
pub fn symbol_names() -> Vec<String> {
    KSYMTAB.lock().iter().map(|s| s.name.clone()).collect()
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
}
