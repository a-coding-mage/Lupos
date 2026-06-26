//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_fortify/write_overflow-memmove.c
//! test-origin: linux:vendor/linux/lib/test_fortify/write_overflow-memmove.c
//! Fortify write-overflow probe for `memmove`.

use super::FortifyProbe;

pub const PROBE: FortifyProbe = FortifyProbe {
    linux_source: "vendor/linux/lib/test_fortify/write_overflow-memmove.c",
    test_expression: "memmove(instance.buf, large_src, sizeof(large_src))",
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
                "/vendor/linux/lib/test_fortify/write_overflow-memmove.c"
            )),
            PROBE,
        );
    }
}
