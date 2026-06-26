//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_fortify/write_overflow-strcpy-lit.c
//! test-origin: linux:vendor/linux/lib/test_fortify/write_overflow-strcpy-lit.c
//! Fortify write-overflow probe for literal `strcpy`.

use super::FortifyProbe;

pub const PROBE: FortifyProbe = FortifyProbe {
    linux_source: "vendor/linux/lib/test_fortify/write_overflow-strcpy-lit.c",
    test_expression: "strcpy(small, LITERAL_LARGE)",
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
                "/vendor/linux/lib/test_fortify/write_overflow-strcpy-lit.c"
            )),
            PROBE,
        );
    }
}
