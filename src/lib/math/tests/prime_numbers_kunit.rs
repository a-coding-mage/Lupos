//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/tests/prime_numbers_kunit.c
//! test-origin: linux:vendor/linux/lib/math/tests/prime_numbers_kunit.c
//! KUnit parity checks for the prime number helpers.

use crate::lib::math::prime_numbers::{is_prime_number, next_prime_number, slow_is_prime_number};

pub const PRIME_NUMBERS_TEST_MAX: usize = 65_536;
pub const MODULE_AUTHOR: &str = "Intel Corporation";
pub const MODULE_DESCRIPTION: &str = "Prime number library";
pub const MODULE_LICENSE: &str = "GPL";

pub fn prime_numbers_kunit_matches(max: usize) -> bool {
    let mut last = 0usize;
    let mut x = 2usize;

    while x < max {
        let slow = slow_is_prime_number(x);
        let fast = is_prime_number(x);
        if slow != fast {
            return false;
        }

        if slow {
            let next = next_prime_number(last);
            if next != x {
                return false;
            }
            last = next;
        }

        x += 1;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prime_numbers_kunit_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/tests/prime_numbers_kunit.c"
        ));
        assert!(source.contains("#include <linux/prime_numbers.h>"));
        assert!(source.contains("#include \"../prime_numbers_private.h\""));
        assert!(source.contains("const unsigned long max = 65536;"));
        assert!(source.contains("slow_is_prime_number(x);"));
        assert!(source.contains("is_prime_number(x);"));
        assert!(source.contains("next = next_prime_number(last);"));
        assert!(source.contains("with_primes(suite, dump_primes);"));
        assert!(source.contains(".name = \"math-prime_numbers\""));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(source.contains(MODULE_DESCRIPTION));

        assert!(!is_prime_number(0));
        assert!(!is_prime_number(1));
        assert!(is_prime_number(2));
        assert!(is_prime_number(65_521));
        assert!(!is_prime_number(65_535));
        assert_eq!(next_prime_number(0), 2);
        assert_eq!(next_prime_number(61), 67);
        assert!(prime_numbers_kunit_matches(PRIME_NUMBERS_TEST_MAX));
    }
}
