//! linux-parity: partial
//! linux-source: vendor/linux/kernel/panic.c
//! test-origin: linux:vendor/linux/kernel/panic.c
//! Kernel taint state.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

static TAINTED_MASK: AtomicU64 = AtomicU64::new(0);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("add_taint", linux_add_taint as usize, false);
}

pub fn test_taint(flag: u32) -> bool {
    flag < u64::BITS && (TAINTED_MASK.load(Ordering::Acquire) & (1u64 << flag)) != 0
}

pub fn get_taint() -> u64 {
    TAINTED_MASK.load(Ordering::Acquire)
}

/// `add_taint` - `vendor/linux/kernel/panic.c:954`.
#[unsafe(export_name = "add_taint")]
pub extern "C" fn linux_add_taint(flag: u32, _lockdep_ok: i32) {
    if flag < u64::BITS {
        TAINTED_MASK.fetch_or(1u64 << flag, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_taint_exports_and_sets_taint_bit() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/panic.c"
        ));
        assert!(source.contains("void add_taint(unsigned flag, enum lockdep_ok lockdep_ok)"));
        assert!(source.contains("EXPORT_SYMBOL(add_taint);"));

        register_module_exports();
        assert_eq!(find_symbol("add_taint"), Some(linux_add_taint as usize));

        linux_add_taint(18, 0);
        assert!(test_taint(18));
        assert_ne!(get_taint() & (1 << 18), 0);
    }
}
