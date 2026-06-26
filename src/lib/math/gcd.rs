//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/gcd.c
//! test-origin: linux:vendor/linux/lib/math/gcd.c
//! Binary greatest-common-divisor helper.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("gcd", gcd as usize, true);
}

const fn low_bit(value: usize) -> usize {
    value & value.wrapping_neg()
}

pub const fn binary_gcd(mut a: usize, mut b: usize) -> usize {
    let r = a | b;

    b >>= b.trailing_zeros();
    if b == 1 {
        return low_bit(r);
    }

    loop {
        a >>= a.trailing_zeros();
        if a == 1 {
            return low_bit(r);
        }
        if a == b {
            return a << r.trailing_zeros();
        }

        if a < b {
            let tmp = a;
            a = b;
            b = tmp;
        }
        a -= b;
    }
}

pub const fn gcd_with_efficient_ffs(mut a: usize, mut b: usize, efficient_ffs: bool) -> usize {
    let mut r = a | b;

    if a == 0 || b == 0 {
        return r;
    }

    if efficient_ffs {
        return binary_gcd(a, b);
    }

    r = low_bit(r);

    while b & r == 0 {
        b >>= 1;
    }
    if b == r {
        return r;
    }

    loop {
        while a & r == 0 {
            a >>= 1;
        }
        if a == r {
            return r;
        }
        if a == b {
            return a;
        }

        if a < b {
            let tmp = a;
            a = b;
            b = tmp;
        }
        a -= b;
        a >>= 1;
        if a & r != 0 {
            a += b;
        }
        a >>= 1;
    }
}

pub const fn gcd(a: usize, b: usize) -> usize {
    gcd_with_efficient_ffs(a, b, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/gcd.c"
        ));
        assert!(source.contains("DEFINE_STATIC_KEY_TRUE(efficient_ffs_key);"));
        assert!(
            source.contains("static unsigned long binary_gcd(unsigned long a, unsigned long b)")
        );
        assert!(source.contains("if (static_branch_likely(&efficient_ffs_key))"));
        assert!(source.contains("return binary_gcd(a, b);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(gcd);"));

        assert_eq!(gcd(48, 18), 6);
        assert_eq!(gcd(18, 48), 6);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(7, 0), 7);
        assert_eq!(gcd(usize::MAX, 1), 1);
        assert_eq!(gcd(usize::MAX, usize::MAX), usize::MAX);
        assert_eq!(gcd_with_efficient_ffs(270, 192, false), 6);
    }

    #[test]
    fn gcd_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("gcd"),
            Some(gcd as usize)
        );
    }
}
