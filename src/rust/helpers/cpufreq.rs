//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/cpufreq.c
//! test-origin: linux:vendor/linux/rust/helpers/cpufreq.c
//! Rust helper shim for CPU frequency EM registration.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/cpufreq.c",
    include_line: "#include <linux/cpufreq.h>",
    helper_symbol: "rust_helper_cpufreq_register_em_with_opp",
    forwards_to: "cpufreq_register_em_with_opp(policy)",
};

pub fn source() -> RustHelperSource {
    SOURCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_metadata_matches_linux_source() {
        super::super::assert_helper_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/rust/helpers/cpufreq.c"
            )),
            SOURCE,
        );
    }
}
