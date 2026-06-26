//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc64-main.c
//! test-origin: linux:vendor/linux/lib/crc/crc64-main.c
//! Generic CRC64 ECMA-182 and NVMe helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const CRC64_ECMA182_POLY: u64 = 0x42f0_e1eb_a9ea_3693;
pub const CRC64_NVME_POLY: u64 = 0x9a6c_9329_ac4b_c9b5;
pub const MODULE_DESCRIPTION: &str = "CRC64 library functions";

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc64_be", crc64_be_raw as usize, true);
    export_symbol_once("crc64_nvme", crc64_nvme_raw as usize, true);
}

pub fn crc64_be(mut crc: u64, data: &[u8]) -> u64 {
    for byte in data {
        crc ^= (*byte as u64) << 56;
        for _ in 0..8 {
            crc = if crc & (1 << 63) != 0 {
                (crc << 1) ^ CRC64_ECMA182_POLY
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub fn crc64_nvme(crc: u64, data: &[u8]) -> u64 {
    let mut state = !crc;
    for byte in data {
        state ^= *byte as u64;
        for _ in 0..8 {
            state = if state & 1 != 0 {
                (state >> 1) ^ CRC64_NVME_POLY
            } else {
                state >> 1
            };
        }
    }
    !state
}

pub unsafe extern "C" fn crc64_be_raw(crc: u64, p: *const u8, len: usize) -> u64 {
    if p.is_null() && len != 0 {
        return crc;
    }
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    };
    crc64_be(crc, data)
}

pub unsafe extern "C" fn crc64_nvme_raw(crc: u64, p: *const u8, len: usize) -> u64 {
    if p.is_null() && len != 0 {
        return crc;
    }
    let data = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    };
    crc64_nvme(crc, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc64_main_matches_linux_generic_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc64-main.c"
        ));
        let generator = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/gen_crc64table.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/crc64.h"
        ));
        assert!(source.contains("crc = (crc << 8) ^ crc64table[(crc >> 56) ^ *p++];"));
        assert!(source.contains("crc = (crc >> 8) ^ crc64nvmetable[(crc & 0xff) ^ *p++];"));
        assert!(source.contains("return crc64_be_arch(crc, p, len);"));
        assert!(source.contains("return ~crc64_nvme_arch(~crc, p, len);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(crc64_be);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(crc64_nvme);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"CRC64 library functions\")"));
        assert!(generator.contains("#define CRC64_ECMA182_POLY 0x42F0E1EBA9EA3693ULL"));
        assert!(generator.contains("#define CRC64_NVME_POLY 0x9A6C9329AC4BC9B5ULL"));
        assert!(header.contains("including the bitwise inversion at the beginning and end"));

        assert_eq!(crc64_be(0, b"123456789"), 0x6c40_df5f_0b49_7347);
        assert_eq!(
            crc64_be(crc64_be(0, b"1234"), b"56789"),
            crc64_be(0, b"123456789")
        );
        assert_eq!(
            crc64_nvme(crc64_nvme(0, b"1234"), b"56789"),
            crc64_nvme(0, b"123456789")
        );
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc64_be"),
            Some(crc64_be_raw as usize)
        );
    }
}
