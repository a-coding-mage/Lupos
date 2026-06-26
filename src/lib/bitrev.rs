//! linux-parity: complete
//! linux-source: vendor/linux/lib/bitrev.c
//! test-origin: linux:vendor/linux/lib/bitrev.c
//! Bit ordering reversal table and helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const LINUX_SOURCE: &str = "vendor/linux/lib/bitrev.c";

const fn reverse_byte(mut value: u8) -> u8 {
    let mut reversed = 0u8;
    let mut bit = 0;
    while bit < 8 {
        reversed = (reversed << 1) | (value & 1);
        value >>= 1;
        bit += 1;
    }
    reversed
}

const fn build_byte_rev_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut index = 0;
    while index < table.len() {
        table[index] = reverse_byte(index as u8);
        index += 1;
    }
    table
}

pub static BYTE_REV_TABLE: [u8; 256] = build_byte_rev_table();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("byte_rev_table", BYTE_REV_TABLE.as_ptr() as usize, true);
}

pub const fn bitrev8(value: u8) -> u8 {
    reverse_byte(value)
}

pub fn bitrev16(value: u16) -> u16 {
    ((bitrev8(value as u8) as u16) << 8) | bitrev8((value >> 8) as u8) as u16
}

pub fn bitrev32(value: u32) -> u32 {
    ((bitrev16(value as u16) as u32) << 16) | bitrev16((value >> 16) as u16) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_reverse_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/bitrev.c"
        ));
        assert!(source.contains("const u8 byte_rev_table[256]"));
        assert!(source.contains("0x00, 0x80, 0x40, 0xc0"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(byte_rev_table);"));
        assert_eq!(BYTE_REV_TABLE.len(), 256);
        assert_eq!(BYTE_REV_TABLE[0x00], 0x00);
        assert_eq!(BYTE_REV_TABLE[0x01], 0x80);
        assert_eq!(BYTE_REV_TABLE[0x12], 0x48);
        assert_eq!(BYTE_REV_TABLE[0xff], 0xff);
    }

    #[test]
    fn bit_reverse_helpers_use_linux_byte_ordering() {
        assert_eq!(bitrev8(0b0001_0110), 0b0110_1000);
        assert_eq!(bitrev16(0x1234), 0x2c48);
        assert_eq!(bitrev32(0x0123_4567), 0xe6a2_c480);
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("byte_rev_table"),
            Some(BYTE_REV_TABLE.as_ptr() as usize)
        );
    }
}
