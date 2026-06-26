//! linux-parity: complete
//! linux-source: vendor/linux/lib/ashldi3.c
//! test-origin: linux:vendor/linux/lib/ashldi3.c
//! libgcc arithmetic left shift helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub type WordType = isize;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__ashldi3", __ashldi3 as usize, false);
}

pub extern "C" fn __ashldi3(value: i64, shift: WordType) -> i64 {
    if shift == 0 {
        value
    } else {
        ((value as u64) << (shift as u32)) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ashldi3_matches_linux_source_and_shift_results() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/ashldi3.c"
        ));
        assert!(source.contains("if (b == 0)"));
        assert!(source.contains("w.s.low = (unsigned int) uu.s.low << b;"));
        assert!(source.contains("EXPORT_SYMBOL(__ashldi3);"));
        assert_eq!(__ashldi3(0x11, 0), 0x11);
        assert_eq!(__ashldi3(0x11, 4), 0x110);
        assert_eq!(__ashldi3(1, 40), 1i64 << 40);
    }

    #[test]
    fn ashldi3_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__ashldi3"),
            Some(__ashldi3 as usize)
        );
    }
}
