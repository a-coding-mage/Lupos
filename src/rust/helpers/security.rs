//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/security.c
//! test-origin: linux:vendor/linux/rust/helpers/security.c
//! Rust helper shims for security hooks.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/security.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/security.h>",
        helper_symbol: "rust_helper_security_cred_getsecid",
        forwards_to: "security_cred_getsecid(c, secid)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/security.h>",
        helper_symbol: "rust_helper_security_secid_to_secctx",
        forwards_to: "security_secid_to_secctx(secid, cp)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/security.h>",
        helper_symbol: "rust_helper_security_release_secctx",
        forwards_to: "security_release_secctx(cp)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/security.h>",
        helper_symbol: "rust_helper_security_binder_set_context_mgr",
        forwards_to: "security_binder_set_context_mgr(mgr)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/security.h>",
        helper_symbol: "rust_helper_security_binder_transaction",
        forwards_to: "security_binder_transaction(from, to)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/security.h>",
        helper_symbol: "rust_helper_security_binder_transfer_binder",
        forwards_to: "security_binder_transfer_binder(from, to)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/security.h>",
        helper_symbol: "rust_helper_security_binder_transfer_file",
        forwards_to: "security_binder_transfer_file(from, to, file)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_security_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/security.c"
        ));
        assert!(source.contains("#ifndef CONFIG_SECURITY"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
