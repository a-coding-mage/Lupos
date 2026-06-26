//! linux-parity: complete
//! linux-source: vendor/linux/lib/kunit/hooks.c
//! test-origin: linux:vendor/linux/lib/kunit/hooks.c
//! KUnit static key and hook-table exports.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

pub static KUNIT_RUNNING: AtomicBool = AtomicBool::new(false);
pub static KUNIT_HOOKS: KunitHooksTable = KunitHooksTable;

#[derive(Debug)]
pub struct KunitHooksTable;

pub const EXPORTED_SYMBOLS: &[&str] = &["kunit_running", "kunit_hooks"];

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "kunit_running",
        (&KUNIT_RUNNING as *const AtomicBool) as usize,
        false,
    );
    export_symbol_once(
        "kunit_hooks",
        (&KUNIT_HOOKS as *const KunitHooksTable) as usize,
        false,
    );
}

pub fn kunit_running_is_enabled() -> bool {
    KUNIT_RUNNING.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kunit_hooks_source_exports_symbols() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/kunit/hooks.c"
        ));
        assert!(source.contains("#include <kunit/test-bug.h>"));
        assert!(source.contains("DEFINE_STATIC_KEY_FALSE(kunit_running);"));
        assert!(source.contains("EXPORT_SYMBOL(kunit_running);"));
        assert!(source.contains("struct kunit_hooks_table kunit_hooks;"));
        assert!(source.contains("EXPORT_SYMBOL(kunit_hooks);"));
        assert_eq!(EXPORTED_SYMBOLS, ["kunit_running", "kunit_hooks"]);
        assert!(!kunit_running_is_enabled());
    }

    #[test]
    fn kunit_hooks_register_module_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("kunit_running"),
            Some((&KUNIT_RUNNING as *const AtomicBool) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kunit_hooks"),
            Some((&KUNIT_HOOKS as *const KunitHooksTable) as usize)
        );
    }
}
