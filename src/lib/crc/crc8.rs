//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc8.c
//! test-origin: linux:vendor/linux/lib/crc/crc8.c
//! CRC8 table population and update helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const CRC8_TABLE_SIZE: usize = 256;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc8_populate_msb", crc8_populate_msb_raw as usize, false);
    export_symbol_once("crc8_populate_lsb", crc8_populate_lsb_raw as usize, false);
    export_symbol_once("crc8", crc8_raw as usize, false);
}

pub fn crc8_populate_msb(table: &mut [u8; CRC8_TABLE_SIZE], polynomial: u8) {
    let msbit = 0x80u8;
    let mut t = msbit;
    table[0] = 0;

    let mut i = 1usize;
    while i < CRC8_TABLE_SIZE {
        t = (t << 1) ^ if t & msbit != 0 { polynomial } else { 0 };
        let mut j = 0usize;
        while j < i {
            table[i + j] = table[j] ^ t;
            j += 1;
        }
        i *= 2;
    }
}

pub fn crc8_populate_lsb(table: &mut [u8; CRC8_TABLE_SIZE], polynomial: u8) {
    let mut t = 1u8;
    table[0] = 0;

    let mut i = CRC8_TABLE_SIZE >> 1;
    while i != 0 {
        t = (t >> 1) ^ if t & 1 != 0 { polynomial } else { 0 };
        let mut j = 0usize;
        while j < CRC8_TABLE_SIZE {
            table[i + j] = table[j] ^ t;
            j += 2 * i;
        }
        i >>= 1;
    }
}

pub fn crc8(table: &[u8; CRC8_TABLE_SIZE], pdata: &[u8], mut crc: u8) -> u8 {
    for byte in pdata {
        crc = table[(crc ^ *byte) as usize];
    }
    crc
}

pub unsafe extern "C" fn crc8_populate_msb_raw(table: *mut u8, polynomial: u8) {
    if table.is_null() {
        return;
    }
    let table = unsafe { &mut *(table as *mut [u8; CRC8_TABLE_SIZE]) };
    crc8_populate_msb(table, polynomial);
}

pub unsafe extern "C" fn crc8_populate_lsb_raw(table: *mut u8, polynomial: u8) {
    if table.is_null() {
        return;
    }
    let table = unsafe { &mut *(table as *mut [u8; CRC8_TABLE_SIZE]) };
    crc8_populate_lsb(table, polynomial);
}

pub unsafe extern "C" fn crc8_raw(
    table: *const u8,
    pdata: *const u8,
    nbytes: usize,
    crc: u8,
) -> u8 {
    if table.is_null() || (pdata.is_null() && nbytes != 0) {
        return crc;
    }
    let table = unsafe { &*(table as *const [u8; CRC8_TABLE_SIZE]) };
    let data = if nbytes == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(pdata, nbytes) }
    };
    crc8(table, data, crc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc8_matches_linux_table_generation_and_exports() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc8.c"
        ));
        assert!(
            source.contains("void crc8_populate_msb(u8 table[CRC8_TABLE_SIZE], u8 polynomial)")
        );
        assert!(source.contains("const u8 msbit = 0x80;"));
        assert!(source.contains("for (i = 1; i < CRC8_TABLE_SIZE; i *= 2)"));
        assert!(
            source.contains("void crc8_populate_lsb(u8 table[CRC8_TABLE_SIZE], u8 polynomial)")
        );
        assert!(source.contains("for (i = (CRC8_TABLE_SIZE >> 1); i; i >>= 1)"));
        assert!(source.contains("crc = table[(crc ^ *pdata++) & 0xff];"));
        assert!(source.contains("EXPORT_SYMBOL(crc8);"));

        let mut msb = [0u8; CRC8_TABLE_SIZE];
        crc8_populate_msb(&mut msb, 0x07);
        assert_eq!(crc8(&msb, b"123456789", 0), 0xf4);

        let mut lsb = [0u8; CRC8_TABLE_SIZE];
        crc8_populate_lsb(&mut lsb, 0x8c);
        assert_eq!(crc8(&lsb, b"123456789", 0), 0xa1);

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc8"),
            Some(crc8_raw as usize)
        );
    }
}
