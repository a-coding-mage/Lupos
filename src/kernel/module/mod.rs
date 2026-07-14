//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module
//! Kernel module (.ko) loader — M56.
//!
//! Mirrors `kernel/module/main.c` (Linux 6.x).
//!
//! A `.ko` file is an ET_REL ELF object.  The loader:
//!   1. Reads section headers and locates `.text`, `.data`, `.bss`,
//!      `.modinfo`, `.gnu.linkonce.this_module`.
//!   2. Allocates executable + data pages.
//!   3. Applies `R_X86_64_*` relocations using the EXPORT_SYMBOL table.
//!   4. Calls `module->init()`.
//!   5. On unload, calls `module->exit()` and frees memory.
//!
//! References:
//!   - `kernel/module/main.c:load_module` (overall flow)
//!   - `kernel/module/main.c:find_module_sections` (line 2659)
//!   - `kernel/module/main.c:apply_relocations` (line 1608)
//!   - `arch/x86/kernel/module.c:apply_relocate_add` (line 219)
//!   - `include/linux/module.h:397` — `struct module`
//!   - `include/linux/export.h:89`  — `EXPORT_SYMBOL`

pub mod debug_kmemleak;
pub mod kdb;
pub mod linux_sources;
pub mod livepatch;
pub mod loader;
pub mod relocate;
pub mod signing;
pub mod symbols;
pub mod syscalls;
pub mod tracking;
pub mod tree_lookup;

pub use loader::{
    KernelModule, LoadModuleError, ModuleState, delete_module, find_module, inserted_modules,
    load_module, with_module_address,
};
pub use symbols::{ExportedSymbol, export_symbol, find_symbol, find_symbol_gpl_only};

pub fn register_module_exports() {
    loader::register_module_exports();
}
