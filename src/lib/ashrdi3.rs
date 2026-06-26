//! linux-parity: complete
//! linux-source: vendor/linux/lib/ashrdi3.c
//! test-origin: linux:vendor/linux/lib/ashrdi3.c
//! libgcc arithmetic right shift helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub type WordType = isize;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__ashrdi3", __ashrdi3 as usize, false);
}

pub extern "C" fn __ashrdi3(value: i64, shift: WordType) -> i64 {
    if shift == 0 {
        value
    } else {
        value >> (shift as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ashrdi3_matches_linux_source_and_shift_results() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/ashrdi3.c"
        ));
        assert!(source.contains("if (b == 0)"));
        assert!(source.contains("w.s.high ="));
        assert!(source.contains("uu.s.high >> 31"));
        assert!(source.contains("EXPORT_SYMBOL(__ashrdi3);"));
        assert_eq!(__ashrdi3(-8, 0), -8);
        assert_eq!(__ashrdi3(-8, 1), -4);
        assert_eq!(__ashrdi3(1i64 << 40, 40), 1);
    }

    #[test]
    fn ashrdi3_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__ashrdi3"),
            Some(__ashrdi3 as usize)
        );
    }
}
