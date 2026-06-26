//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/irq.c
//! test-origin: linux:vendor/linux/rust/helpers/irq.c
//! Rust helper shim for IRQ registration.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/irq.c",
    include_line: "#include <linux/interrupt.h>",
    helper_symbol: "rust_helper_request_irq",
    forwards_to: "request_irq(irq, handler, flags, name, dev)",
};

pub fn source() -> RustHelperSource {
    SOURCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_metadata_matches_linux_source() {
        super::super::assert_helper_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/rust/helpers/irq.c"
            )),
            SOURCE,
        );
    }
}
