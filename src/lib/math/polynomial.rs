//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/polynomial.c
//! test-origin: linux:vendor/linux/lib/math/polynomial.c
//! Integer polynomial calculation with Linux `mult_frac` redistribution.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("polynomial_calc", polynomial_calc_raw as usize, true);
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolynomialTerm {
    pub deg: u32,
    pub coef: isize,
    pub divider: isize,
    pub divider_leftover: isize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Polynomial<'a> {
    pub total_divider: isize,
    pub terms: &'a [PolynomialTerm],
}

#[repr(C)]
pub struct RawPolynomial {
    pub total_divider: isize,
    pub terms: [PolynomialTerm; 0],
}

pub const fn mult_frac(x: isize, n: isize, d: isize) -> isize {
    let q = x / d;
    let r = x % d;
    q * n + r * n / d
}

pub fn polynomial_calc(poly: &Polynomial<'_>, data: isize) -> isize {
    let total_divider = if poly.total_divider != 0 {
        poly.total_divider
    } else {
        1
    };
    let mut ret = 0isize;

    for term in poly.terms {
        let mut tmp = term.coef;
        let mut deg = 0;
        while deg < term.deg {
            tmp = mult_frac(tmp, data, term.divider);
            deg += 1;
        }
        ret += tmp / term.divider_leftover;
        if term.deg == 0 {
            break;
        }
    }

    ret / total_divider
}

pub unsafe extern "C" fn polynomial_calc_raw(poly: *const RawPolynomial, data: isize) -> isize {
    if poly.is_null() {
        return 0;
    }

    let raw = unsafe { &*poly };
    let total_divider = if raw.total_divider != 0 {
        raw.total_divider
    } else {
        1
    };
    let mut ret = 0isize;
    let mut term = raw.terms.as_ptr();

    loop {
        let t = unsafe { *term };
        let mut tmp = t.coef;
        let mut deg = 0;
        while deg < t.deg {
            tmp = mult_frac(tmp, data, t.divider);
            deg += 1;
        }
        ret += tmp / t.divider_leftover;
        if t.deg == 0 {
            break;
        }
        term = unsafe { term.add(1) };
    }

    ret / total_divider
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polynomial_calc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/polynomial.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/polynomial.h"
        ));
        let math = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/math.h"
        ));
        assert!(source.contains("long polynomial_calc(const struct polynomial *poly, long data)"));
        assert!(source.contains("long total_divider = poly->total_divider ?: 1;"));
        assert!(source.contains("tmp = term->coef;"));
        assert!(source.contains("tmp = mult_frac(tmp, data, term->divider);"));
        assert!(source.contains("ret += tmp / term->divider_leftover;"));
        assert!(source.contains("return ret / total_divider;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(polynomial_calc);"));
        assert!(header.contains("struct polynomial_term"));
        assert!(math.contains("q * n_ + r * n_ / d_;"));

        let temp_to_n = [
            PolynomialTerm {
                deg: 4,
                coef: 18_322,
                divider: 10_000,
                divider_leftover: 10_000,
            },
            PolynomialTerm {
                deg: 3,
                coef: 2_343,
                divider: 10_000,
                divider_leftover: 10,
            },
            PolynomialTerm {
                deg: 2,
                coef: 87_018,
                divider: 10_000,
                divider_leftover: 10,
            },
            PolynomialTerm {
                deg: 1,
                coef: 39_269,
                divider: 1_000,
                divider_leftover: 1,
            },
            PolynomialTerm {
                deg: 0,
                coef: 1_720_400,
                divider: 1,
                divider_leftover: 1,
            },
        ];
        let poly = Polynomial {
            total_divider: 10_000,
            terms: &temp_to_n,
        };
        assert_eq!(polynomial_calc(&poly, 0), 172);
        assert_eq!(polynomial_calc(&poly, 25_000), 276);
        assert_eq!(mult_frac(39_269, 25_000, 1_000), 981_725);

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("polynomial_calc"),
            Some(polynomial_calc_raw as usize)
        );
    }
}
