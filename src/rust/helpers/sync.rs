//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/sync.c
//! test-origin: linux:vendor/linux/rust/helpers/sync.c
//! Rust helper shims for lockdep key registration.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/sync.c",
        include_line: "#include <linux/lockdep.h>",
        helper_symbol: "rust_helper_lockdep_register_key",
        forwards_to: "lockdep_register_key(k)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/sync.c",
        include_line: "#include <linux/lockdep.h>",
        helper_symbol: "rust_helper_lockdep_unregister_key",
        forwards_to: "lockdep_unregister_key(k)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/sync.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
