//! linux-parity: partial
//! linux-source: vendor/linux/lib/crc
//! CRC helper source coverage.

pub mod arm64;
pub mod crc16;
pub mod crc32_main;
pub mod crc4;
pub mod crc64_main;
pub mod crc7;
pub mod crc8;
pub mod crc_ccitt;
pub mod crc_itu_t;
pub mod crc_t10dif_main;
pub mod gen_crc32table;
pub mod gen_crc64table;
pub mod riscv;
pub mod tests;

pub fn register_module_exports() {
    crc16::register_module_exports();
    crc32_main::register_module_exports();
    crc7::register_module_exports();
    crc8::register_module_exports();
    crc_ccitt::register_module_exports();
    crc_itu_t::register_module_exports();
    crc4::register_module_exports();
    crc64_main::register_module_exports();
    crc_t10dif_main::register_module_exports();
}
