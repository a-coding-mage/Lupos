//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc-ccitt.c
//! test-origin: linux:vendor/linux/lib/crc/crc-ccitt.c
//! CRC-CCITT helper.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc_ccitt_table", CRC_CCITT_TABLE.as_ptr() as usize, false);
    export_symbol_once("crc_ccitt", crc_ccitt_raw as usize, false);
}

pub const CRC_CCITT_POLY_REFLECTED: u16 = 0x8408;

pub const fn build_crc_ccitt_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u16;
        let mut bit = 0;
        while bit < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC_CCITT_POLY_REFLECTED;
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

pub static CRC_CCITT_TABLE: [u16; 256] = build_crc_ccitt_table();

pub fn crc_ccitt_byte(crc: u16, c: u8) -> u16 {
    (crc >> 8) ^ CRC_CCITT_TABLE[((crc ^ c as u16) & 0xff) as usize]
}

pub fn crc_ccitt(mut crc: u16, buffer: &[u8]) -> u16 {
    for byte in buffer {
        crc = crc_ccitt_byte(crc, *byte);
    }
    crc
}

pub unsafe extern "C" fn crc_ccitt_raw(crc: u16, buffer: *const u8, len: usize) -> u16 {
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(buffer, len) }
    };
    crc_ccitt(crc, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_ccitt_table_and_update_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc-ccitt.c"
        ));
        assert!(source.contains("u16 const crc_ccitt_table[256]"));
        assert!(source.contains("0x0000, 0x1189, 0x2312, 0x329b"));
        assert!(source.contains("0x8408"));
        assert!(source.contains("crc = crc_ccitt_byte(crc, *buffer++);"));
        assert!(source.contains("EXPORT_SYMBOL(crc_ccitt_table);"));
        assert!(source.contains("EXPORT_SYMBOL(crc_ccitt);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"CRC-CCITT calculations\")"));

        assert_eq!(CRC_CCITT_TABLE[0], 0x0000);
        assert_eq!(CRC_CCITT_TABLE[1], 0x1189);
        assert_eq!(CRC_CCITT_TABLE[128], 0x8408);
        assert_eq!(crc_ccitt(0, b"123456789"), 0x2189);
        assert_eq!(crc_ccitt(0xffff, b"123456789"), 0x6F91);
        assert_eq!(
            unsafe { crc_ccitt_raw(0, b"123456789".as_ptr(), 9) },
            0x2189
        );
    }

    #[test]
    fn crc_ccitt_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc_ccitt"),
            Some(crc_ccitt_raw as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("crc_ccitt_table"),
            Some(CRC_CCITT_TABLE.as_ptr() as usize)
        );
    }
}
