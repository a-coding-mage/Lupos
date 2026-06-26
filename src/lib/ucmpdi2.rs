//! linux-parity: complete
//! linux-source: vendor/linux/lib/ucmpdi2.c
//! test-origin: linux:vendor/linux/lib/ucmpdi2.c
//! Unsigned 64-bit libgcc comparison helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub type WordType = isize;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__ucmpdi2", __ucmpdi2 as usize, false);
}

pub extern "C" fn __ucmpdi2(a: u64, b: u64) -> WordType {
    let a_high = (a >> 32) as u32;
    let b_high = (b >> 32) as u32;
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
    fn libgcc_unsigned_di_comparison_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/ucmpdi2.c"
        ));
        assert!(source.contains("(unsigned int) au.s.high < (unsigned int) bu.s.high"));
        assert!(source.contains("(unsigned int) au.s.low > (unsigned int) bu.s.low"));
        assert!(source.contains("EXPORT_SYMBOL(__ucmpdi2);"));
        assert_eq!(__ucmpdi2(1, 2), 0);
        assert_eq!(__ucmpdi2(2, 1), 2);
        assert_eq!(__ucmpdi2(7, 7), 1);
        assert_eq!(__ucmpdi2(0x0000_0001_0000_0000, 0x0000_0000_ffff_ffff), 2);
        assert_eq!(__ucmpdi2(0x0000_0000_ffff_ffff, 0x0000_0001_0000_0000), 0);
    }

    #[test]
    fn libgcc_unsigned_di_export_registers_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__ucmpdi2"),
            Some(__ucmpdi2 as usize)
        );
    }
}
