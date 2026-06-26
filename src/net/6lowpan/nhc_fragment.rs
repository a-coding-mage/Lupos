//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_fragment.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_fragment.c
//! 6LoWPAN RFC6282 fragment NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_fragment.c",
    symbol: "nhc_fragment",
    description: "RFC6282 Fragment",
    module_description: "6LoWPAN next header RFC6282 Fragment compression",
    id: 0xe4,
    mask: 0xfe,
    id_literal: "0xe4",
    mask_literal: "0xfe",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpan_nhc_source_matches_linux() {
        super::super::assert_lowpan_nhc_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/net/6lowpan/nhc_fragment.c"
            )),
            SOURCE,
        );
    }
}
