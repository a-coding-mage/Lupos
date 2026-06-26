//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_ghc_udp.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_ghc_udp.c
//! 6LoWPAN RFC7400 UDP NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_ghc_udp.c",
    symbol: "ghc_udp",
    description: "RFC7400 UDP",
    module_description: "6LoWPAN generic header UDP compression",
    id: 0xd0,
    mask: 0xf8,
    id_literal: "0xd0",
    mask_literal: "0xf8",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpan_nhc_source_matches_linux() {
        super::super::assert_lowpan_nhc_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/net/6lowpan/nhc_ghc_udp.c"
            )),
            SOURCE,
        );
    }
}
