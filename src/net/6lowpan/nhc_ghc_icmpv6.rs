//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_ghc_icmpv6.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_ghc_icmpv6.c
//! 6LoWPAN RFC7400 ICMPv6 NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_ghc_icmpv6.c",
    symbol: "ghc_icmpv6",
    description: "RFC7400 ICMPv6",
    module_description: "6LoWPAN generic header ICMPv6 compression",
    id: 0xdf,
    mask: 0xff,
    id_literal: "0xdf",
    mask_literal: "0xff",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpan_nhc_source_matches_linux() {
        super::super::assert_lowpan_nhc_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/net/6lowpan/nhc_ghc_icmpv6.c"
            )),
            SOURCE,
        );
    }
}
