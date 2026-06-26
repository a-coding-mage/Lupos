//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/rcu.c
//! test-origin: linux:vendor/linux/rust/helpers/rcu.c
//! Rust helper shims for RCU read-side locking.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/rcu.c",
        include_line: "#include <linux/rcupdate.h>",
        helper_symbol: "rust_helper_rcu_read_lock",
        forwards_to: "rcu_read_lock()",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/rcu.c",
        include_line: "#include <linux/rcupdate.h>",
        helper_symbol: "rust_helper_rcu_read_unlock",
        forwards_to: "rcu_read_unlock()",
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
            "/vendor/linux/rust/helpers/rcu.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
