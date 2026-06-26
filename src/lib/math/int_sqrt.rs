//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/int_sqrt.c
//! test-origin: linux:vendor/linux/lib/math/int_sqrt.c
//! Integer square-root helpers.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("int_sqrt", int_sqrt as usize, false);
}

pub const fn int_sqrt(mut x: usize) -> usize {
    if x <= 1 {
        return x;
    }

    let msb = (usize::BITS - 1) - x.leading_zeros();
    let mut m = 1usize << (msb & !1);
    let mut y = 0usize;

    while m != 0 {
        let b = y + m;
        y >>= 1;
        if x >= b {
            x -= b;
            y += m;
        }
        m >>= 2;
    }
    y
}

pub const fn int_sqrt64(x: u64) -> u32 {
    if usize::BITS >= 64 || x <= usize::MAX as u64 {
        return int_sqrt(x as usize) as u32;
    }

    let mut x = x;
    let msb = 63 - x.leading_zeros();
    let mut m = 1u64 << (msb & !1);
    let mut y = 0u64;

    while m != 0 {
        let b = y + m;
        y >>= 1;
        if x >= b {
            x -= b;
            y += m;
        }
        m >>= 2;
    }
    y as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_sqrt_matches_linux_shift_and_subtract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/int_sqrt.c"
        ));
        assert!(source.contains("m = 1UL << (__fls(x) & ~1UL);"));
        assert!(source.contains("b = y + m;"));
        assert!(source.contains("y >>= 1;"));
        assert!(source.contains("x -= b;"));
        assert!(source.contains("EXPORT_SYMBOL(int_sqrt);"));

        for (x, expected) in [
            (0usize, 0usize),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 2),
            (15, 3),
            (16, 4),
            (17, 4),
            (usize::MAX, (1usize << (usize::BITS / 2)) - 1),
        ] {
            assert_eq!(int_sqrt(x), expected);
        }
        assert_eq!(int_sqrt64(u64::MAX), u32::MAX);
    }
}
