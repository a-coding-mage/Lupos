//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_hop.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_hop.c
//! 6LoWPAN RFC6282 hop-by-hop NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_hop.c",
    symbol: "nhc_hop",
    description: "RFC6282 Hop-by-Hop Options",
    module_description: "6LoWPAN next header RFC6282 Hop-by-Hop Options compression",
    id: 0xe0,
    mask: 0xfe,
    id_literal: "0xe0",
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
                "/vendor/linux/net/6lowpan/nhc_hop.c"
            )),
            SOURCE,
        );
    }
}
