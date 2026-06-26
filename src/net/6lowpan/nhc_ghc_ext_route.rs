//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_ghc_ext_route.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_ghc_ext_route.c
//! 6LoWPAN RFC7400 routing extension NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_ghc_ext_route.c",
    symbol: "ghc_ext_route",
    description: "RFC7400 Routing Extension Header",
    module_description: "6LoWPAN generic header routing extension compression",
    id: 0xb2,
    mask: 0xfe,
    id_literal: "0xb2",
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
                "/vendor/linux/net/6lowpan/nhc_ghc_ext_route.c"
            )),
            SOURCE,
        );
    }
}
