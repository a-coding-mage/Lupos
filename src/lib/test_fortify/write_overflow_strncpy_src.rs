//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_fortify/write_overflow-strncpy-src.c
//! test-origin: linux:vendor/linux/lib/test_fortify/write_overflow-strncpy-src.c
//! Fortify source read-overflow probe for `strncpy`.

use super::FortifyProbe;

pub const PROBE: FortifyProbe = FortifyProbe {
    linux_source: "vendor/linux/lib/test_fortify/write_overflow-strncpy-src.c",
    test_expression: "strncpy(small, large_src, sizeof(small) + 1)",
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
                "/vendor/linux/lib/test_fortify/write_overflow-strncpy-src.c"
            )),
            PROBE,
        );
    }
}
