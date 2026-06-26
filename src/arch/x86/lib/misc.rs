//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/misc.c
//! test-origin: linux:vendor/linux/arch/x86/lib/misc.c
//! Miscellaneous x86 library helpers.

pub use super::arch_lib::num_digits;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_digits_matches_misc_c_edges() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/misc.c"
        ));
        assert!(source.contains("int num_digits(int val)"));
        assert_eq!(num_digits(0), 1);
        assert_eq!(num_digits(10), 2);
        assert_eq!(num_digits(-99), 3);
        assert_eq!(num_digits(i32::MIN), 11);
    }
}
