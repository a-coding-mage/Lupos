//! linux-parity: partial
//! linux-source: vendor/linux/mm/mm_init.c
//! test-origin: linux:vendor/linux/mm/mm_init.c
//! Module-visible memory-initialization state.
//!
//! Vendor modules built with `CONFIG_JUMP_LABEL=y` refer to
//! `init_on_free` from their `__jump_table`, even when
//! `CONFIG_INIT_ON_FREE_DEFAULT_ON` is disabled.  Lupos does not enable that
//! policy, so publish Linux's exact initially-false static-key object instead
//! of substituting a function or an untyped byte.

#[cfg(test)]
use core::sync::atomic::Ordering;
use core::sync::atomic::{AtomicI32, AtomicUsize};

use crate::kernel::module::{export_symbol, find_symbol};

/// `struct static_key_false` with `CONFIG_JUMP_LABEL=y` on x86_64.
///
/// `struct static_key` contains an `atomic_t`, the ABI padding required before
/// its pointer-sized union, and that union.  `struct static_key_false` is a
/// one-field wrapper and therefore has the same layout.
#[repr(C)]
struct LinuxStaticKeyFalse {
    enabled: AtomicI32,
    _padding: u32,
    entries_or_type: AtomicUsize,
}

impl LinuxStaticKeyFalse {
    const fn new_disabled() -> Self {
        Self {
            enabled: AtomicI32::new(0),
            _padding: 0,
            entries_or_type: AtomicUsize::new(0),
        }
    }
}

static INIT_ON_FREE: LinuxStaticKeyFalse = LinuxStaticKeyFalse::new_disabled();

pub fn register_module_exports() {
    if find_symbol("init_on_free").is_none() {
        export_symbol(
            "init_on_free",
            core::ptr::addr_of!(INIT_ON_FREE) as usize,
            false,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_on_free_matches_linux_disabled_static_key_abi() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/mm_init.c"
        ));
        let jump_label = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/jump_label.h"
        ));

        assert!(
            source
                .contains("DEFINE_STATIC_KEY_MAYBE(CONFIG_INIT_ON_FREE_DEFAULT_ON, init_on_free);")
        );
        assert!(source.contains("EXPORT_SYMBOL(init_on_free);"));
        assert!(jump_label.contains("struct static_key_false"));
        assert!(jump_label.contains("STATIC_KEY_INIT_FALSE"));

        assert_eq!(core::mem::size_of::<LinuxStaticKeyFalse>(), 16);
        assert_eq!(core::mem::align_of::<LinuxStaticKeyFalse>(), 8);
        assert_eq!(INIT_ON_FREE.enabled.load(Ordering::Relaxed), 0);
        assert_eq!(INIT_ON_FREE.entries_or_type.load(Ordering::Relaxed), 0);
    }
}
