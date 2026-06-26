//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc-itu-t.c
//! test-origin: linux:vendor/linux/lib/crc/crc-itu-t.c
//! CRC ITU-T V.41 helpers.

use crate::kernel::module::{export_symbol, find_symbol};

const fn build_crc_itu_t_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < table.len() {
        let mut crc = (i as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
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

pub static CRC_ITU_T_TABLE: [u16; 256] = build_crc_itu_t_table();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc_itu_t_table", CRC_ITU_T_TABLE.as_ptr() as usize, false);
    export_symbol_once("crc_itu_t", crc_itu_t_raw as usize, false);
}

pub fn crc_itu_t_byte(crc: u16, data: u8) -> u16 {
    (crc << 8) ^ CRC_ITU_T_TABLE[(((crc >> 8) as u8 ^ data) & 0xff) as usize]
}

pub fn crc_itu_t(mut crc: u16, buffer: &[u8]) -> u16 {
    let mut i = 0usize;
    while i < buffer.len() {
        crc = crc_itu_t_byte(crc, buffer[i]);
        i += 1;
    }
    crc
}

pub unsafe extern "C" fn crc_itu_t_raw(crc: u16, buffer: *const u8, len: usize) -> u16 {
    if len == 0 {
        return crc;
    }
    if buffer.is_null() {
        return crc;
    }
    let bytes = unsafe { core::slice::from_raw_parts(buffer, len) };
    crc_itu_t(crc, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_itu_t_matches_linux_table_and_byte_helper() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc-itu-t.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/crc-itu-t.h"
        ));
        assert!(source.contains("CRC ITU-T V.41 0x1021"));
        assert!(source.contains("const u16 crc_itu_t_table[256]"));
        assert!(source.contains("crc = crc_itu_t_byte(crc, *buffer++);"));
        assert!(source.contains("EXPORT_SYMBOL(crc_itu_t_table);"));
        assert!(source.contains("EXPORT_SYMBOL(crc_itu_t);"));
        assert!(header.contains("(crc << 8) ^ crc_itu_t_table[((crc >> 8) ^ data) & 0xff]"));

        assert_eq!(CRC_ITU_T_TABLE[0], 0x0000);
        assert_eq!(CRC_ITU_T_TABLE[1], 0x1021);
        assert_eq!(CRC_ITU_T_TABLE[255], 0x1ef0);
        assert_eq!(crc_itu_t(0, b"123456789"), 0x31c3);
        assert_eq!(crc_itu_t(0xffff, b"123456789"), 0x29b1);
    }

    #[test]
    fn crc_itu_t_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc_itu_t"),
            Some(crc_itu_t_raw as usize)
        );
    }
}
