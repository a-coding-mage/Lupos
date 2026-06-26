//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_dest.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_dest.c
//! 6LoWPAN RFC6282 destination-options NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_dest.c",
    symbol: "nhc_dest",
    description: "RFC6282 Destination Options",
    module_description: "6LoWPAN next header RFC6282 Destination Options compression",
    id: 0xe6,
    mask: 0xfe,
    id_literal: "0xe6",
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
                "/vendor/linux/net/6lowpan/nhc_dest.c"
            )),
            SOURCE,
        );
    }
}
