//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc16.c
//! test-origin: linux:vendor/linux/lib/crc/crc16.c
//! CRC-16 helper.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc16", crc16_symbol as usize, false);
}

pub const CRC16_POLY_REFLECTED: u16 = 0xA001;

pub const fn build_crc16_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u16;
        let mut bit = 0;
        while bit < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC16_POLY_REFLECTED;
            } else {
                crc >>= 1;
            }
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

pub const CRC16_TABLE: [u16; 256] = build_crc16_table();

pub fn crc16(mut crc: u16, data: &[u8]) -> u16 {
    for byte in data {
        crc = (crc >> 8) ^ CRC16_TABLE[((crc & 0xff) as u8 ^ *byte) as usize];
    }
    crc
}

unsafe extern "C" fn crc16_symbol(crc: u16, p: *const u8, len: usize) -> u16 {
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    };
    crc16(crc, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_table_and_update_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc16.c"
        ));
        assert!(source.contains("static const u16 crc16_table[256]"));
        assert!(source.contains("0x0000, 0xC0C1, 0xC181, 0x0140"));
        assert!(source.contains("crc = (crc >> 8) ^ crc16_table[(crc & 0xff) ^ *p++];"));
        assert!(source.contains("EXPORT_SYMBOL(crc16);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"CRC16 calculations\")"));

        assert_eq!(CRC16_TABLE[0], 0x0000);
        assert_eq!(CRC16_TABLE[1], 0xC0C1);
        assert_eq!(CRC16_TABLE[255], 0x4040);
        assert_eq!(crc16(0, b"123456789"), 0xBB3D);
        assert_eq!(unsafe { crc16_symbol(0, b"123456789".as_ptr(), 9) }, 0xBB3D);
    }

    #[test]
    fn crc16_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc16"),
            Some(crc16_symbol as usize)
        );
    }
}
