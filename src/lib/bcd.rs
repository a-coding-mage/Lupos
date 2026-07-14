//! linux-parity: complete
//! linux-source: vendor/linux/lib/bcd.c
//! test-origin: linux:vendor/linux/lib/bcd.c
//! Binary-coded decimal conversion helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const LINUX_SOURCE: &str = "vendor/linux/lib/bcd.c";
pub const LINUX_BCD_INCLUDE: &str = "#include <linux/bcd.h>";
pub const LINUX_EXPORT_INCLUDE: &str = "#include <linux/export.h>";
pub const BCD2BIN_SYMBOL: &str = "_bcd2bin";
pub const BIN2BCD_SYMBOL: &str = "_bin2bcd";
pub const BCD2BIN_RETURN: &str = "unsigned";
pub const BIN2BCD_RETURN: &str = "unsigned char";
pub const BCD2BIN_ARG: &str = "unsigned char val";
pub const BIN2BCD_ARG: &str = "unsigned val";
pub const BCD_LOW_NIBBLE_MASK: u8 = 0x0f;
pub const BCD_DECIMAL_BASE: u32 = 10;
pub const BCD_HIGH_NIBBLE_SHIFT: u32 = 4;
pub const BIN2BCD_TENS_MULTIPLIER: u32 = 103;
pub const BIN2BCD_TENS_SHIFT: u32 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxBcdExport {
    pub symbol: &'static str,
    pub return_type: &'static str,
    pub argument: &'static str,
    pub export_symbol: &'static str,
}

pub const BCD2BIN_EXPORT: LinuxBcdExport = LinuxBcdExport {
    symbol: BCD2BIN_SYMBOL,
    return_type: BCD2BIN_RETURN,
    argument: BCD2BIN_ARG,
    export_symbol: "EXPORT_SYMBOL(_bcd2bin);",
};

pub const BIN2BCD_EXPORT: LinuxBcdExport = LinuxBcdExport {
    symbol: BIN2BCD_SYMBOL,
    return_type: BIN2BCD_RETURN,
    argument: BIN2BCD_ARG,
    export_symbol: "EXPORT_SYMBOL(_bin2bcd);",
};

pub const fn bcd2bin(val: u8) -> u32 {
    ((val & BCD_LOW_NIBBLE_MASK) as u32)
        + ((val >> BCD_HIGH_NIBBLE_SHIFT) as u32) * BCD_DECIMAL_BASE
}

pub const fn bin2bcd(val: u32) -> u8 {
    let tens = (val * BIN2BCD_TENS_MULTIPLIER) >> BIN2BCD_TENS_SHIFT;
    ((tens << BCD_HIGH_NIBBLE_SHIFT) | (val - tens * BCD_DECIMAL_BASE)) as u8
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(BCD2BIN_SYMBOL, linux_bcd2bin as usize, false);
    export_symbol_once(BIN2BCD_SYMBOL, linux_bin2bcd as usize, false);
}

/// `_bcd2bin` - `vendor/linux/lib/bcd.c`.
pub unsafe extern "C" fn linux_bcd2bin(val: u8) -> u32 {
    bcd2bin(val)
}

/// `_bin2bcd` - `vendor/linux/lib/bcd.c`.
pub unsafe extern "C" fn linux_bin2bcd(val: u32) -> u8 {
    bin2bcd(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_conversions_match_linux_arithmetic() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/bcd.c"
        ));
        assert!(source.contains(LINUX_BCD_INCLUDE));
        assert!(source.contains(LINUX_EXPORT_INCLUDE));
        assert!(source.contains("unsigned _bcd2bin(unsigned char val)"));
        assert!(source.contains("return (val & 0x0f) + (val >> 4) * 10;"));
        assert!(source.contains(BCD2BIN_EXPORT.export_symbol));
        assert!(source.contains("unsigned char _bin2bcd(unsigned val)"));
        assert!(source.contains("const unsigned int t = (val * 103) >> 10;"));
        assert!(source.contains("return (t << 4) | (val - t * 10);"));
        assert!(source.contains(BIN2BCD_EXPORT.export_symbol));
        assert_eq!(
            BCD2BIN_EXPORT,
            LinuxBcdExport {
                symbol: "_bcd2bin",
                return_type: "unsigned",
                argument: "unsigned char val",
                export_symbol: "EXPORT_SYMBOL(_bcd2bin);",
            }
        );
        assert_eq!(
            BIN2BCD_EXPORT,
            LinuxBcdExport {
                symbol: "_bin2bcd",
                return_type: "unsigned char",
                argument: "unsigned val",
                export_symbol: "EXPORT_SYMBOL(_bin2bcd);",
            }
        );
        assert_eq!(bcd2bin(0x42), 42);
        assert_eq!(bin2bcd(42), 0x42);
        assert_eq!(bin2bcd(99), 0x99);
        assert_eq!(bcd2bin(0x00), 0);
        assert_eq!(bcd2bin(0x99), 99);
    }

    #[test]
    fn bcd_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("_bcd2bin"),
            Some(linux_bcd2bin as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("_bin2bcd"),
            Some(linux_bin2bcd as usize)
        );
    }
}
