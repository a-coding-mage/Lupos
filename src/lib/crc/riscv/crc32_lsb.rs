//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/riscv/crc32_lsb.c
//! test-origin: linux:vendor/linux/lib/crc/riscv/crc32_lsb.c
//! RISC-V CLMUL LSB-first CRC32 wrapper.

use super::CrcClmulSource;

pub const SOURCE: CrcClmulSource = CrcClmulSource {
    linux_source: "vendor/linux/lib/crc/riscv/crc32_lsb.c",
    crc_type: "typedef u32 crc_t;",
    lsb_crc: true,
    symbol: "crc32_lsb_clmul",
    return_type: "u32",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_clmul_source_matches_linux() {
        super::super::assert_crc_clmul_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/lib/crc/riscv/crc32_lsb.c"
            )),
            SOURCE,
        );
    }
}
