//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/rational.c
//! test-origin: linux:vendor/linux/lib/math/rational.c
//! Rational approximation helper based on continued fractions.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "rational_best_approximation",
        rational_best_approximation_symbol as usize,
        false,
    );
}

pub const fn rational_best_approximation(
    given_numerator: usize,
    given_denominator: usize,
    max_numerator: usize,
    max_denominator: usize,
) -> (usize, usize) {
    let mut n = given_numerator;
    let mut d = given_denominator;
    let mut n0 = 0usize;
    let mut d0 = 1usize;
    let mut n1 = 1usize;
    let mut d1 = 0usize;

    loop {
        if d == 0 {
            break;
        }

        let dp = d;
        let a = n / d;
        d = n % d;
        n = dp;

        let n2 = n0.wrapping_add(a.wrapping_mul(n1));
        let d2 = d0.wrapping_add(a.wrapping_mul(d1));

        if n2 > max_numerator || d2 > max_denominator {
            let mut t = usize::MAX;
            if d1 != 0 {
                t = max_denominator.wrapping_sub(d0) / d1;
            }
            if n1 != 0 {
                let nt = max_numerator.wrapping_sub(n0) / n1;
                if nt < t {
                    t = nt;
                }
            }

            let twice_t = t.wrapping_mul(2);
            if d1 == 0 || twice_t > a || (twice_t == a && d0.wrapping_mul(dp) > d1.wrapping_mul(d))
            {
                n1 = n0.wrapping_add(t.wrapping_mul(n1));
                d1 = d0.wrapping_add(t.wrapping_mul(d1));
            }
            break;
        }

        n0 = n1;
        n1 = n2;
        d0 = d1;
        d1 = d2;
    }

    (n1, d1)
}

pub fn rational_best_approximation_into(
    given_numerator: usize,
    given_denominator: usize,
    max_numerator: usize,
    max_denominator: usize,
    best_numerator: &mut usize,
    best_denominator: &mut usize,
) {
    let (n, d) = rational_best_approximation(
        given_numerator,
        given_denominator,
        max_numerator,
        max_denominator,
    );
    *best_numerator = n;
    *best_denominator = d;
}

extern "C" fn rational_best_approximation_symbol(
    given_numerator: usize,
    given_denominator: usize,
    max_numerator: usize,
    max_denominator: usize,
    best_numerator: *mut usize,
    best_denominator: *mut usize,
) {
    let (n, d) = rational_best_approximation(
        given_numerator,
        given_denominator,
        max_numerator,
        max_denominator,
    );
    unsafe {
        if !best_numerator.is_null() {
            *best_numerator = n;
        }
        if !best_denominator.is_null() {
            *best_denominator = d;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_best_approximation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/rational.c"
        ));
        assert!(source.contains("void rational_best_approximation("));
        assert!(source.contains("a = n / d;"));
        assert!(source.contains("d = n % d;"));
        assert!(source.contains("n2 = n0 + a * n1;"));
        assert!(source.contains("d2 = d0 + a * d1;"));
        assert!(source.contains("2u * t == a && d0 * dp > d1 * d"));
        assert!(source.contains("EXPORT_SYMBOL(rational_best_approximation);"));

        assert_eq!(rational_best_approximation(1230, 10, 100, 20), (100, 1));
        assert_eq!(rational_best_approximation(27, 32, 16, 16), (11, 13));
        assert_eq!(rational_best_approximation(1155, 7735, 255, 255), (33, 221));

        let mut n = 0;
        let mut d = 0;
        rational_best_approximation_into(87, 32, 70, 32, &mut n, &mut d);
        assert_eq!((n, d), (68, 25));
    }

    #[test]
    fn rational_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("rational_best_approximation"),
            Some(rational_best_approximation_symbol as usize)
        );
    }
}
