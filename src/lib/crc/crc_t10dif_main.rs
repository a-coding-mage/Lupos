//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc-t10dif-main.c
//! test-origin: linux:vendor/linux/lib/crc/crc-t10dif-main.c
//! T10-DIF CRC16 table and update helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub const T10_DIF_POLY: u16 = 0x8bb7;
pub const T10_DIF_TABLE_SIZE: usize = 256;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc_t10dif_update", crc_t10dif_update_raw as usize, false);
}

pub const fn generate_t10_dif_crc_table() -> [u16; T10_DIF_TABLE_SIZE] {
    let mut table = [0u16; T10_DIF_TABLE_SIZE];
    let mut i = 0usize;
    while i < T10_DIF_TABLE_SIZE {
        let mut crc = (i as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ T10_DIF_POLY;
            } else {
                crc <<= 1;
            }
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

pub const T10_DIF_CRC_TABLE: [u16; T10_DIF_TABLE_SIZE] = generate_t10_dif_crc_table();

pub fn crc_t10dif_update(mut crc: u16, data: &[u8]) -> u16 {
    for byte in data {
        crc = (crc << 8) ^ T10_DIF_CRC_TABLE[((crc >> 8) as u8 ^ *byte) as usize];
    }
    crc
}

pub unsafe extern "C" fn crc_t10dif_update_raw(crc: u16, p: *const u8, len: usize) -> u16 {
    if p.is_null() && len != 0 {
        return crc;
    }
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    };
    crc_t10dif_update(crc, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_t10dif_matches_linux_table_and_update() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc-t10dif-main.c"
        ));
        assert!(source.contains("gt: 0x8bb7"));
        assert!(source.contains("static const u16 t10_dif_crc_table[256]"));
        assert!(source.contains("0x0000, 0x8BB7, 0x9CD9, 0x176E"));
        assert!(source.contains("crc = (crc << 8) ^ t10_dif_crc_table[(crc >> 8) ^ *p++];"));
        assert!(source.contains("return crc_t10dif_arch(crc, p, len);"));
        assert!(source.contains("EXPORT_SYMBOL(crc_t10dif_update);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"CRC-T10DIF library functions\")"));

        assert_eq!(&T10_DIF_CRC_TABLE[..4], &[0x0000, 0x8bb7, 0x9cd9, 0x176e]);
        assert_eq!(crc_t10dif_update(0, b"123456789"), 0xd0db);
        assert_eq!(
            crc_t10dif_update(crc_t10dif_update(0, b"1234"), b"56789"),
            0xd0db
        );
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc_t10dif_update"),
            Some(crc_t10dif_update_raw as usize)
        );
    }
}
