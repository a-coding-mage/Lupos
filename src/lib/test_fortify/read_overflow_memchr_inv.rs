//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_fortify/read_overflow-memchr_inv.c
//! test-origin: linux:vendor/linux/lib/test_fortify/read_overflow-memchr_inv.c
//! Fortify read-overflow probe for `memchr_inv`.

use super::FortifyProbe;

pub const PROBE: FortifyProbe = FortifyProbe {
    linux_source: "vendor/linux/lib/test_fortify/read_overflow-memchr_inv.c",
    test_expression: "memchr_inv(small, 0x7A, sizeof(small) + 1)",
};

pub fn probe() -> FortifyProbe {
    PROBE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fortify_probe_matches_linux_source() {
        super::super::assert_fortify_probe(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/lib/test_fortify/read_overflow-memchr_inv.c"
            )),
            PROBE,
        );
    }
}
