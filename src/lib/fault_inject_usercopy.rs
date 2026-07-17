//! linux-parity: complete
//! linux-source: vendor/linux/lib/fault-inject-usercopy.c
//! test-origin: linux:vendor/linux/lib/fault-inject-usercopy.c
//! Usercopy fault-injection switch.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

static FAIL_USERCOPY: AtomicBool = AtomicBool::new(false);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("should_fail_usercopy", should_fail_usercopy as usize, true);
}

pub fn setup_fail_usercopy(config: &str) -> i32 {
    FAIL_USERCOPY.store(
        config == "1" || config.contains("probability=100"),
        Ordering::Release,
    );
    0
}

pub extern "C" fn should_fail_usercopy() -> bool {
    FAIL_USERCOPY.load(Ordering::Acquire)
}

#[cfg(test)]
pub fn reset_for_test() {
    FAIL_USERCOPY.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_usercopy_setup_and_export_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/fault-inject-usercopy.c"
        ));
        assert!(source.contains("FAULT_ATTR_INITIALIZER"));
        assert!(source.contains("setup_fault_attr(&fail_usercopy.attr, str);"));
        assert!(source.contains("__setup(\"fail_usercopy=\", setup_fail_usercopy);"));
        assert!(source.contains("fault_create_debugfs_attr(\"fail_usercopy\""));
        assert!(source.contains("return should_fail(&fail_usercopy.attr, 1);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(should_fail_usercopy);"));

        reset_for_test();
        assert!(!should_fail_usercopy());
        assert_eq!(setup_fail_usercopy("probability=100"), 0);
        assert!(should_fail_usercopy());
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("should_fail_usercopy"),
            Some(should_fail_usercopy as usize)
        );
        reset_for_test();
    }
}
