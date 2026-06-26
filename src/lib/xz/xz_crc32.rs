//! linux-parity: complete
//! linux-source: vendor/linux/lib/xz/xz_crc32.c
//! test-origin: linux:vendor/linux/lib/xz/xz_crc32.c
//! Compact IEEE CRC32 used by the XZ decoder.

pub const XZ_CRC32_POLY: u32 = 0xEDB8_8320;

pub const fn build_xz_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut r = i as u32;
        let mut j = 0;
        while j < 8 {
            r = (r >> 1) ^ (XZ_CRC32_POLY & !((r & 1).wrapping_sub(1)));
            j += 1;
        }
        table[i] = r;
        i += 1;
    }
    table
}

pub const XZ_CRC32_TABLE: [u32; 256] = build_xz_crc32_table();

pub const fn xz_crc32_init() -> [u32; 256] {
    build_xz_crc32_table()
}

pub fn xz_crc32(buf: &[u8], crc: u32) -> u32 {
    let mut crc = !crc;
    for byte in buf {
        let idx = (*byte ^ (crc as u8)) as usize;
        crc = XZ_CRC32_TABLE[idx] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xz_crc32_table_and_update_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/xz/xz_crc32.c"
        ));
        assert!(source.contains("const uint32_t poly = 0xEDB88320;"));
        assert!(source.contains("for (i = 0; i < 256; ++i)"));
        assert!(source.contains("for (j = 0; j < 8; ++j)"));
        assert!(source.contains("xz_crc32_table[i] = r;"));
        assert!(source.contains("crc = ~crc;"));
        assert!(source.contains("xz_crc32_table[*buf++ ^ (crc & 0xFF)] ^ (crc >> 8);"));
        assert!(source.contains("return ~crc;"));

        let table = xz_crc32_init();
        assert_eq!(table, XZ_CRC32_TABLE);
        assert_eq!(XZ_CRC32_TABLE[0], 0);
        assert_eq!(XZ_CRC32_TABLE[1], 0x7707_3096);
        assert_eq!(xz_crc32(b"123456789", 0), 0xcbf4_3926);

        let first = xz_crc32(b"1234", 0);
        assert_eq!(xz_crc32(b"56789", first), 0xcbf4_3926);
    }
}
