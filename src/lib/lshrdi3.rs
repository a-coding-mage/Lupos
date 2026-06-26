//! linux-parity: complete
//! linux-source: vendor/linux/lib/lshrdi3.c
//! test-origin: linux:vendor/linux/lib/lshrdi3.c
//! libgcc logical right shift helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub type WordType = isize;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__lshrdi3", __lshrdi3 as usize, false);
}

pub extern "C" fn __lshrdi3(value: i64, shift: WordType) -> i64 {
    if shift == 0 {
        value
    } else {
        ((value as u64) >> (shift as u32)) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lshrdi3_matches_linux_source_and_shift_results() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/lshrdi3.c"
        ));
        assert!(source.contains("if (b == 0)"));
        assert!(source.contains("w.s.high = (unsigned int) uu.s.high >> b;"));
        assert!(source.contains("EXPORT_SYMBOL(__lshrdi3);"));
        assert_eq!(__lshrdi3(-1, 0), -1);
        assert_eq!(__lshrdi3(-1, 1) as u64, u64::MAX >> 1);
        assert_eq!(__lshrdi3(1i64 << 40, 40), 1);
    }

    #[test]
    fn lshrdi3_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__lshrdi3"),
            Some(__lshrdi3 as usize)
        );
    }
}
