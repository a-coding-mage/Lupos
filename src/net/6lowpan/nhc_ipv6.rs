//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_ipv6.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_ipv6.c
//! 6LoWPAN RFC6282 IPv6 NHC registration.

use super::LowpanNhcSource;

pub const SOURCE: LowpanNhcSource = LowpanNhcSource {
    linux_source: "vendor/linux/net/6lowpan/nhc_ipv6.c",
    symbol: "nhc_ipv6",
    description: "RFC6282 IPv6",
    module_description: "6LoWPAN next header RFC6282 IPv6 compression",
    id: 0xee,
    mask: 0xfe,
    id_literal: "0xee",
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
                "/vendor/linux/net/6lowpan/nhc_ipv6.c"
            )),
            SOURCE,
        );
    }
}
