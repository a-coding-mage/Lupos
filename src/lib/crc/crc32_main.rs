//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc32-main.c
//! test-origin: linux:vendor/linux/lib/crc/crc32-main.c
//! Generic CRC32 and CRC32C table walkers.

use crate::kernel::module::{export_symbol, find_symbol};

use super::gen_crc32table::{CRC32CTABLE_LE, CRC32TABLE_BE, CRC32TABLE_LE};

pub const MODULE_DESCRIPTION: &str = "CRC32 library functions";

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc32_le", crc32_le_raw as usize, false);
    export_symbol_once("crc32_be", crc32_be_raw as usize, false);
    export_symbol_once("crc32c", crc32c_raw as usize, false);
}

pub fn crc32_le(mut crc: u32, data: &[u8]) -> u32 {
    for byte in data {
        crc = (crc >> 8) ^ CRC32TABLE_LE[((crc & 255) ^ (*byte as u32)) as usize];
    }
    crc
}

pub fn crc32_be(mut crc: u32, data: &[u8]) -> u32 {
    for byte in data {
        crc = (crc << 8) ^ CRC32TABLE_BE[((crc >> 24) ^ (*byte as u32)) as usize];
    }
    crc
}

pub fn crc32c(mut crc: u32, data: &[u8]) -> u32 {
    for byte in data {
        crc = (crc >> 8) ^ CRC32CTABLE_LE[((crc & 255) ^ (*byte as u32)) as usize];
    }
    crc
}

pub unsafe extern "C" fn crc32_le_raw(crc: u32, p: *const u8, len: usize) -> u32 {
    if p.is_null() && len != 0 {
        return crc;
    }
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    };
    crc32_le(crc, data)
}

pub unsafe extern "C" fn crc32_be_raw(crc: u32, p: *const u8, len: usize) -> u32 {
    if p.is_null() && len != 0 {
        return crc;
    }
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    };
    crc32_be(crc, data)
}

pub unsafe extern "C" fn crc32c_raw(crc: u32, p: *const u8, len: usize) -> u32 {
    if p.is_null() && len != 0 {
        return crc;
    }
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    };
    crc32c(crc, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_main_matches_linux_generic_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc32-main.c"
        ));
        assert!(source.contains("crc = (crc >> 8) ^ crc32table_le[(crc & 255) ^ *p++];"));
        assert!(source.contains("crc = (crc << 8) ^ crc32table_be[(crc >> 24) ^ *p++];"));
        assert!(source.contains("crc = (crc >> 8) ^ crc32ctable_le[(crc & 255) ^ *p++];"));
        assert!(source.contains("return crc32_le_arch(crc, p, len);"));
        assert!(source.contains("return crc32_be_arch(crc, p, len);"));
        assert!(source.contains("return crc32c_arch(crc, p, len);"));
        assert!(source.contains("EXPORT_SYMBOL(crc32_le);"));
        assert!(source.contains("EXPORT_SYMBOL(crc32_be);"));
        assert!(source.contains("EXPORT_SYMBOL(crc32c);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"CRC32 library functions\")"));

        assert_eq!(crc32_le(!0, b"123456789") ^ !0, 0xcbf4_3926);
        assert_eq!(crc32c(!0, b"123456789") ^ !0, 0xe306_9283);
        assert_eq!(crc32_be(0, b"123456789"), 0x89a1_897f);
        assert_eq!(
            crc32_le(crc32_le(!0, b"1234"), b"56789"),
            crc32_le(!0, b"123456789")
        );

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc32_le"),
            Some(crc32_le_raw as usize)
        );
    }
}
