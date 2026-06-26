//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc7.c
//! test-origin: linux:vendor/linux/lib/crc/crc7.c
//! Big-endian CRC7 helper.

use crate::kernel::module::{export_symbol, find_symbol};

const fn build_crc7_be_syndrome_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut i = 0usize;
    while i < table.len() {
        let mut crc = i as u8;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x12
            } else {
                crc << 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

pub const CRC7_BE_SYNDROME_TABLE: [u8; 256] = build_crc7_be_syndrome_table();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc7_be", crc7_be_raw as usize, false);
}

pub fn crc7_be(mut crc: u8, buffer: &[u8]) -> u8 {
    for byte in buffer {
        crc = CRC7_BE_SYNDROME_TABLE[(crc ^ *byte) as usize];
    }
    crc
}

pub unsafe extern "C" fn crc7_be_raw(crc: u8, buffer: *const u8, len: usize) -> u8 {
    if len == 0 {
        return crc;
    }
    if buffer.is_null() {
        return crc;
    }
    let bytes = unsafe { core::slice::from_raw_parts(buffer, len) };
    crc7_be(crc, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc7_be_matches_linux_syndrome_table() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc7.c"
        ));
        assert!(source.contains("polynomial x^7 + x^3 + 1"));
        assert!(source.contains("static const u8 crc7_be_syndrome_table[256]"));
        assert!(source.contains("crc = crc7_be_syndrome_table[crc ^ *buffer++];"));
        assert!(source.contains("EXPORT_SYMBOL(crc7_be);"));

        assert_eq!(CRC7_BE_SYNDROME_TABLE[0], 0x00);
        assert_eq!(CRC7_BE_SYNDROME_TABLE[1], 0x12);
        assert_eq!(CRC7_BE_SYNDROME_TABLE[255], 0xf2);
        assert_eq!(crc7_be(0, b"123456789"), 0xea);
    }

    #[test]
    fn crc7_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc7_be"),
            Some(crc7_be_raw as usize)
        );
    }
}
