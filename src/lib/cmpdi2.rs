//! linux-parity: complete
//! linux-source: vendor/linux/lib/cmpdi2.c
//! test-origin: linux:vendor/linux/lib/cmpdi2.c
//! Signed 64-bit libgcc comparison helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub type WordType = isize;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__cmpdi2", __cmpdi2 as usize, false);
}

pub extern "C" fn __cmpdi2(a: i64, b: i64) -> WordType {
    let a_high = (a >> 32) as i32;
    let b_high = (b >> 32) as i32;
    if a_high < b_high {
        return 0;
    }
    if a_high > b_high {
        return 2;
    }

    let a_low = a as u32;
    let b_low = b as u32;
    if a_low < b_low {
        0
    } else if a_low > b_low {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libgcc_signed_di_comparison_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/cmpdi2.c"
        ));
        assert!(source.contains("if (au.s.high < bu.s.high)"));
        assert!(source.contains("(unsigned int) au.s.low > (unsigned int) bu.s.low"));
        assert!(source.contains("EXPORT_SYMBOL(__cmpdi2);"));
        assert_eq!(__cmpdi2(-2, -1), 0);
        assert_eq!(__cmpdi2(-1, -2), 2);
        assert_eq!(__cmpdi2(7, 7), 1);
        assert_eq!(__cmpdi2(i64::MIN, 0), 0);
        assert_eq!(__cmpdi2(0, i64::MIN), 2);
        assert_eq!(__cmpdi2(0x0000_0000_ffff_ffff, 0x0000_0001_0000_0000), 0);
    }

    #[test]
    fn libgcc_signed_di_export_registers_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__cmpdi2"),
            Some(__cmpdi2 as usize)
        );
    }
}
