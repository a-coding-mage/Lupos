//! linux-parity: partial
//! linux-source: vendor/linux/kernel/notifier.c
//! test-origin: linux:vendor/linux/kernel/notifier.c
//! Notifier-chain ABI exports used by Linux-built modules.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "blocking_notifier_chain_register",
        linux_blocking_notifier_chain_register as usize,
        true,
    );
    export_symbol_once(
        "blocking_notifier_chain_register_unique_prio",
        linux_blocking_notifier_chain_register_unique_prio as usize,
        true,
    );
    export_symbol_once(
        "blocking_notifier_chain_unregister",
        linux_blocking_notifier_chain_unregister as usize,
        true,
    );
    export_symbol_once(
        "blocking_notifier_call_chain",
        linux_blocking_notifier_call_chain as usize,
        true,
    );
}

/// `blocking_notifier_chain_register` - `vendor/linux/kernel/notifier.c`.
pub unsafe extern "C" fn linux_blocking_notifier_chain_register(
    _nh: *mut c_void,
    _nb: *mut c_void,
) -> i32 {
    0
}

/// `blocking_notifier_chain_register_unique_prio` - `vendor/linux/kernel/notifier.c`.
pub unsafe extern "C" fn linux_blocking_notifier_chain_register_unique_prio(
    nh: *mut c_void,
    nb: *mut c_void,
) -> i32 {
    unsafe { linux_blocking_notifier_chain_register(nh, nb) }
}

/// `blocking_notifier_chain_unregister` - `vendor/linux/kernel/notifier.c`.
pub unsafe extern "C" fn linux_blocking_notifier_chain_unregister(
    _nh: *mut c_void,
    _nb: *mut c_void,
) -> i32 {
    0
}

/// `blocking_notifier_call_chain` - `vendor/linux/kernel/notifier.c`.
pub unsafe extern "C" fn linux_blocking_notifier_call_chain(
    _nh: *mut c_void,
    _val: usize,
    _v: *mut c_void,
) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifier_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("blocking_notifier_chain_register"),
            Some(linux_blocking_notifier_chain_register as usize)
        );
    }
}
