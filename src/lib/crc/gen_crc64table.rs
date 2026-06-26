//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/gen_crc64table.c
//! test-origin: linux:vendor/linux/lib/crc/gen_crc64table.c
//! Build-time CRC64 table generator logic.

pub const CRC64_ECMA182_POLY: u64 = 0x42F0_E1EB_A9EA_3693;
pub const CRC64_NVME_POLY: u64 = 0x9A6C_9329_AC4B_C9B5;
pub const CRC_TABLE_SIZE: usize = 256;

pub const fn generate_reflected_crc64_table(poly: u64) -> [u64; CRC_TABLE_SIZE] {
    let mut table = [0u64; CRC_TABLE_SIZE];
    let mut i = 0usize;
    while i < CRC_TABLE_SIZE {
        let mut crc = 0u64;
        let c = i as u64;
        let mut j = 0u64;
        while j < 8 {
            if ((crc ^ (c >> j)) & 1) != 0 {
                crc = (crc >> 1) ^ poly;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

pub const fn generate_crc64_table(poly: u64) -> [u64; CRC_TABLE_SIZE] {
    let mut table = [0u64; CRC_TABLE_SIZE];
    let mut i = 0usize;
    while i < CRC_TABLE_SIZE {
        let mut crc = 0u64;
        let mut c = (i as u64) << 56;
        let mut j = 0;
        while j < 8 {
            if ((crc ^ c) & 0x8000_0000_0000_0000) != 0 {
                crc = (crc << 1) ^ poly;
            } else {
                crc <<= 1;
            }
            c <<= 1;
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

pub const CRC64_TABLE: [u64; CRC_TABLE_SIZE] = generate_crc64_table(CRC64_ECMA182_POLY);
pub const CRC64_NVME_TABLE: [u64; CRC_TABLE_SIZE] = generate_reflected_crc64_table(CRC64_NVME_POLY);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_crc64table_matches_linux_generator() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/gen_crc64table.c"
        ));
        assert!(source.contains("#define CRC64_ECMA182_POLY 0x42F0E1EBA9EA3693ULL"));
        assert!(source.contains("#define CRC64_NVME_POLY 0x9A6C9329AC4BC9B5ULL"));
        assert!(source.contains("generate_crc64_table(crc64_table, CRC64_ECMA182_POLY);"));
        assert!(
            source.contains("generate_reflected_crc64_table(crc64_nvme_table, CRC64_NVME_POLY);")
        );
        assert!(source.contains(
            "printf(\"static const u64 ____cacheline_aligned crc64table[256] = {\\n\");"
        ));
        assert!(source.contains(
            "printf(\"\\nstatic const u64 ____cacheline_aligned crc64nvmetable[256] = {\\n\");"
        ));

        assert_eq!(CRC64_TABLE[0], 0);
        assert_eq!(CRC64_TABLE[1], 0x42F0_E1EB_A9EA_3693);
        assert_eq!(CRC64_NVME_TABLE[0], 0);
        assert_eq!(CRC64_NVME_TABLE[1], 0x7F6E_F0C8_3035_8979);
    }
}
