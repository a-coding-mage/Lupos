//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/lcm.c
//! test-origin: linux:vendor/linux/lib/math/lcm.c
//! Lowest common multiple helpers.

use crate::kernel::module::{export_symbol, find_symbol};
pub use crate::lib::math::gcd::gcd;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("lcm", lcm as usize, true);
    export_symbol_once("lcm_not_zero", lcm_not_zero as usize, true);
}

pub const fn lcm(a: usize, b: usize) -> usize {
    if a != 0 && b != 0 {
        (a / gcd(a, b)) * b
    } else {
        0
    }
}

pub const fn lcm_not_zero(a: usize, b: usize) -> usize {
    let value = lcm(a, b);
    if value != 0 {
        value
    } else if b != 0 {
        b
    } else {
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcm_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/lcm.c"
        ));
        assert!(source.contains("#include <linux/gcd.h>"));
        assert!(source.contains("return (a / gcd(a, b)) * b;"));
        assert!(source.contains("return (b ? : a);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(lcm);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(lcm_not_zero);"));
        assert_eq!(lcm(6, 8), 24);
        assert_eq!(lcm(0, 8), 0);
        assert_eq!(lcm_not_zero(0, 8), 8);
        assert_eq!(lcm_not_zero(7, 0), 7);
        assert_eq!(lcm_not_zero(6, 8), 24);
    }

    #[test]
    fn lcm_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("lcm"),
            Some(lcm as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("lcm_not_zero"),
            Some(lcm_not_zero as usize)
        );
    }
}
