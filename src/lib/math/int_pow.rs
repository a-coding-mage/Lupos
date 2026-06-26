//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/int_pow.c
//! test-origin: linux:vendor/linux/lib/math/int_pow.c
//! Unsigned integer exponentiation helper.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("int_pow", int_pow as usize, true);
}

pub const fn int_pow(mut base: u64, mut exp: u32) -> u64 {
    let mut result = 1u64;
    while exp != 0 {
        if exp & 1 != 0 {
            result = result.wrapping_mul(base);
        }
        exp >>= 1;
        base = base.wrapping_mul(base);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_pow_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/int_pow.c"
        ));
        assert!(source.contains("u64 result = 1;"));
        assert!(source.contains("if (exp & 1)"));
        assert!(source.contains("exp >>= 1;"));
        assert!(source.contains("base *= base;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(int_pow);"));
        assert_eq!(int_pow(5, 0), 1);
        assert_eq!(int_pow(2, 10), 1024);
        assert_eq!(int_pow(u64::MAX, 2), 1);
    }

    #[test]
    fn int_pow_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("int_pow"),
            Some(int_pow as usize)
        );
    }
}
