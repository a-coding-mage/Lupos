//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/blk.c
//! test-origin: linux:vendor/linux/rust/helpers/blk.c
//! Rust helper shims for block multiqueue request private data.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/blk.c",
        include_line: "#include <linux/blk-mq.h>",
        helper_symbol: "rust_helper_blk_mq_rq_to_pdu",
        forwards_to: "blk_mq_rq_to_pdu(rq)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/blk.c",
        include_line: "#include <linux/blkdev.h>",
        helper_symbol: "rust_helper_blk_mq_rq_from_pdu",
        forwards_to: "blk_mq_rq_from_pdu(pdu)",
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
            "/vendor/linux/rust/helpers/blk.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
