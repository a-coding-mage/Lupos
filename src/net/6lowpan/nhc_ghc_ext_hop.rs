//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_ghc_ext_hop.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_ghc_ext_hop.c
//! 6LoWPAN RFC7400 hop-by-hop extension NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_ghc_ext_hop.c",
    symbol: "ghc_ext_hop",
    description: "RFC7400 Hop-by-Hop Extension Header",
    module_description: "6LoWPAN generic header hop-by-hop extension compression",
    id: 0xb0,
    mask: 0xfe,
    id_literal: "0xb0",
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
                "/vendor/linux/net/6lowpan/nhc_ghc_ext_hop.c"
            )),
            SOURCE,
        );
    }
}
