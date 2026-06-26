//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/riscv/crc16_msb.c
//! test-origin: linux:vendor/linux/lib/crc/riscv/crc16_msb.c
//! RISC-V CLMUL MSB-first CRC16 wrapper.

use super::CrcClmulSource;

pub const SOURCE: CrcClmulSource = CrcClmulSource {
    linux_source: "vendor/linux/lib/crc/riscv/crc16_msb.c",
    crc_type: "typedef u16 crc_t;",
    lsb_crc: false,
    symbol: "crc16_msb_clmul",
    return_type: "u16",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_clmul_source_matches_linux() {
        super::super::assert_crc_clmul_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/lib/crc/riscv/crc16_msb.c"
            )),
            SOURCE,
        );
    }
}
