//! linux-parity: complete
//! linux-source: vendor/linux/lib/debug_locks.c
//! test-origin: linux:vendor/linux/lib/debug_locks.c
//! Shared lock-debugging kill switch.

use core::sync::atomic::{AtomicI32, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

pub static DEBUG_LOCKS: AtomicI32 = AtomicI32::new(1);
pub static DEBUG_LOCKS_SILENT: AtomicI32 = AtomicI32::new(0);
static CONSOLE_VERBOSE_CALLS: AtomicI32 = AtomicI32::new(0);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "debug_locks",
        &DEBUG_LOCKS as *const AtomicI32 as usize,
        true,
    );
    export_symbol_once(
        "debug_locks_silent",
        &DEBUG_LOCKS_SILENT as *const AtomicI32 as usize,
        true,
    );
    export_symbol_once("debug_locks_off", debug_locks_off as usize, true);
}

pub fn set_debug_locks(value: i32) {
    DEBUG_LOCKS.store(value, Ordering::Release);
}

pub fn set_debug_locks_silent(value: i32) {
    DEBUG_LOCKS_SILENT.store(value, Ordering::Release);
}

pub fn debug_locks_value() -> i32 {
    DEBUG_LOCKS.load(Ordering::Acquire)
}

pub fn console_verbose_calls() -> i32 {
    CONSOLE_VERBOSE_CALLS.load(Ordering::Acquire)
}

pub fn __debug_locks_off() -> i32 {
    DEBUG_LOCKS.swap(0, Ordering::AcqRel)
}

pub extern "C" fn debug_locks_off() -> i32 {
    if DEBUG_LOCKS.load(Ordering::Acquire) != 0 && __debug_locks_off() != 0 {
        if DEBUG_LOCKS_SILENT.load(Ordering::Acquire) == 0 {
            CONSOLE_VERBOSE_CALLS.fetch_add(1, Ordering::AcqRel);
            return 1;
        }
    }
    0
}

#[cfg(test)]
pub fn reset_for_test() {
    DEBUG_LOCKS.store(1, Ordering::Release);
    DEBUG_LOCKS_SILENT.store(0, Ordering::Release);
    CONSOLE_VERBOSE_CALLS.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_locks_off_matches_linux_first_report_semantics() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/debug_locks.c"
        ));
        assert!(source.contains("int debug_locks __read_mostly = 1;"));
        assert!(source.contains("int debug_locks_silent __read_mostly;"));
        assert!(source.contains("if (debug_locks && __debug_locks_off())"));
        assert!(source.contains("console_verbose();"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(debug_locks_off);"));

        reset_for_test();
        assert_eq!(debug_locks_off(), 1);
        assert_eq!(debug_locks_value(), 0);
        assert_eq!(console_verbose_calls(), 1);
        assert_eq!(debug_locks_off(), 0);
        assert_eq!(console_verbose_calls(), 1);
    }

    #[test]
    fn silent_debug_locks_off_still_disables_lock_debugging() {
        reset_for_test();
        set_debug_locks_silent(1);
        assert_eq!(debug_locks_off(), 0);
        assert_eq!(debug_locks_value(), 0);
        assert_eq!(console_verbose_calls(), 0);
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("debug_locks_off"),
            Some(debug_locks_off as usize)
        );
    }
}
