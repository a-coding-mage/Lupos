//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/riscv/crc64_msb.c
//! test-origin: linux:vendor/linux/lib/crc/riscv/crc64_msb.c
//! RISC-V CLMUL MSB-first CRC64 wrapper.

use super::CrcClmulSource;

pub const SOURCE: CrcClmulSource = CrcClmulSource {
    linux_source: "vendor/linux/lib/crc/riscv/crc64_msb.c",
    crc_type: "typedef u64 crc_t;",
    lsb_crc: false,
    symbol: "crc64_msb_clmul",
    return_type: "u64",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_clmul_source_matches_linux() {
        super::super::assert_crc_clmul_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/lib/crc/riscv/crc64_msb.c"
            )),
            SOURCE,
        );
    }
}
