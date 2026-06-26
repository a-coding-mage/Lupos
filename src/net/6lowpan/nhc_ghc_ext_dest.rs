//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_ghc_ext_dest.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_ghc_ext_dest.c
//! 6LoWPAN RFC7400 destination extension NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_ghc_ext_dest.c",
    symbol: "ghc_ext_dest",
    description: "RFC7400 Destination Extension Header",
    module_description: "6LoWPAN generic header destination extension compression",
    id: 0xb6,
    mask: 0xfe,
    id_literal: "0xb6",
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
                "/vendor/linux/net/6lowpan/nhc_ghc_ext_dest.c"
            )),
            SOURCE,
        );
    }
}
