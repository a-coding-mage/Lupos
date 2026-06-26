//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/riscv/crc32_msb.c
//! test-origin: linux:vendor/linux/lib/crc/riscv/crc32_msb.c
//! RISC-V CLMUL MSB-first CRC32 wrapper.

use super::CrcClmulSource;

pub const SOURCE: CrcClmulSource = CrcClmulSource {
    linux_source: "vendor/linux/lib/crc/riscv/crc32_msb.c",
    crc_type: "typedef u32 crc_t;",
    lsb_crc: false,
    symbol: "crc32_msb_clmul",
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
                "/vendor/linux/lib/crc/riscv/crc32_msb.c"
            )),
            SOURCE,
        );
    }
}
