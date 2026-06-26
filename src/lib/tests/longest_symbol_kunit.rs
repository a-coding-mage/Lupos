//! linux-parity: complete
//! linux-source: vendor/linux/lib/tests/longest_symbol_kunit.c
//! test-origin: linux:vendor/linux/lib/tests/longest_symbol_kunit.c
//! KUnit coverage for the longest kallsyms symbol contract.

pub const KSYM_NAME_LEN: usize = 512;
pub const LONGEST_SYMBOL_NAME_BYTES: usize = KSYM_NAME_LEN - 1;
pub const RETURN_LONGEST_SYM: i32 = 0xAAAAA;
pub const SUITE_NAME: &str = "longest-symbol";
pub const MODULE_DESCRIPTION: &str = "Test the longest symbol length";

pub const fn longest_symbol_value() -> i32 {
    RETURN_LONGEST_SYM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_symbol_kunit_matches_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/longest_symbol_kunit.c"
        ));
        assert!(source.contains("#include <linux/kprobes.h>"));
        assert!(source.contains("#include <linux/kallsyms.h>"));
        assert!(source.contains("#define LONGEST_SYM_NAME  DDDDDI(g1h2i3j4k5l6m7n)"));
        assert!(source.contains("#define RETURN_LONGEST_SYM 0xAAAAA"));
        assert!(source.contains("sizeof(__stringify(LONGEST_SYM_NAME)) == KSYM_NAME_LEN"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, RETURN_LONGEST_SYM, LONGEST_SYM_NAME())"));
        assert!(source.contains(".symbol_name = \"kallsyms_lookup_name\""));
        assert!(source.contains("KUNIT_CASE(test_longest_symbol_kallsyms)"));
        assert!(source.contains(".name = \"longest-symbol\""));
        assert!(source.contains("MODULE_DESCRIPTION(\"Test the longest symbol length\")"));

        assert_eq!(LONGEST_SYMBOL_NAME_BYTES, 511);
        assert_eq!(longest_symbol_value(), RETURN_LONGEST_SYM);
        assert_eq!(SUITE_NAME, "longest-symbol");
        assert_eq!(MODULE_DESCRIPTION, "Test the longest symbol length");
    }
}
