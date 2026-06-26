//! linux-parity: complete
//! linux-source: vendor/linux/lib/clz_ctz.c
//! test-origin: linux:vendor/linux/lib/clz_ctz.c
//! Generic libgcc count-leading/trailing-zero helpers.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__ctzsi2", __ctzsi2 as usize, false);
    export_symbol_once("__clzsi2", __clzsi2 as usize, false);
    export_symbol_once("__clzdi2", __clzdi2 as usize, false);
    export_symbol_once("__ctzdi2", __ctzdi2 as usize, false);
}

pub extern "C" fn __ctzsi2(val: i32) -> i32 {
    (val as u32).trailing_zeros() as i32
}

pub extern "C" fn __clzsi2(val: i32) -> i32 {
    (val as u32).leading_zeros() as i32
}

pub extern "C" fn __clzdi2(val: u64) -> i32 {
    val.leading_zeros() as i32
}

pub extern "C" fn __ctzdi2(val: u64) -> i32 {
    val.trailing_zeros() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clz_ctz_helpers_match_linux_source_and_bit_results() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/clz_ctz.c"
        ));
        assert!(source.contains("return __ffs(val);"));
        assert!(source.contains("return 32 - fls(val);"));
        assert!(source.contains("return 64 - fls64(val);"));
        assert!(source.contains("return __ffs64(val);"));
        assert!(source.contains("EXPORT_SYMBOL(__ctzsi2);"));
        assert!(source.contains("EXPORT_SYMBOL(__clzsi2);"));
        assert!(source.contains("EXPORT_SYMBOL(__clzdi2);"));
        assert!(source.contains("EXPORT_SYMBOL(__ctzdi2);"));

        assert_eq!(__ctzsi2(0b1000), 3);
        assert_eq!(__ctzsi2(i32::MIN), 31);
        assert_eq!(__clzsi2(1), 31);
        assert_eq!(__clzsi2(i32::MIN), 0);
        assert_eq!(__clzdi2(1), 63);
        assert_eq!(__clzdi2(1u64 << 63), 0);
        assert_eq!(__ctzdi2(1u64 << 40), 40);
    }

    #[test]
    fn clz_ctz_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__ctzsi2"),
            Some(__ctzsi2 as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__clzsi2"),
            Some(__clzsi2 as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__clzdi2"),
            Some(__clzdi2 as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__ctzdi2"),
            Some(__ctzdi2 as usize)
        );
    }
}
