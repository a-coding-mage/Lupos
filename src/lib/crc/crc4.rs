//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/crc4.c
//! test-origin: linux:vendor/linux/lib/crc/crc4.c
//! CRC-4 helper using the Linux nibble table.

use crate::kernel::module::{export_symbol, find_symbol};

pub const CRC4_TAB: [u8; 16] = [
    0x0, 0x7, 0xe, 0x9, 0xb, 0xc, 0x5, 0x2, 0x1, 0x6, 0xf, 0x8, 0xa, 0xd, 0x4, 0x3,
];

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crc4", crc4 as usize, true);
}

pub extern "C" fn crc4(mut c: u8, mut x: u64, bits: i32) -> u8 {
    c &= 0x0f;
    if bits <= 0 {
        return c;
    }

    let bit_count = (bits as u32).min(64);
    if bit_count < 64 {
        x &= (1u64 << bit_count) - 1;
    }

    let aligned_bits = ((bit_count + 3) & !0x3).min(64);
    let mut shift = aligned_bits as i32 - 4;
    while shift >= 0 {
        let nibble = ((x >> shift) & 0x0f) as u8;
        c = CRC4_TAB[(c ^ nibble) as usize];
        shift -= 4;
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc4_matches_linux_table_and_bit_alignment() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/crc4.c"
        ));
        assert!(source.contains("static const uint8_t crc4_tab[]"));
        assert!(source.contains("x &= (1ull << bits) - 1;"));
        assert!(source.contains("bits = (bits + 3) & ~0x3;"));
        assert!(source.contains("c = crc4_tab[c ^ ((x >> i) & 0xf)]"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(crc4);"));

        assert_eq!(crc4(0, 0b1011, 4), 0x8);
        assert_eq!(crc4(0, 0b1_1011, 5), crc4(0, 0b1_1011, 8));
        assert_eq!(crc4(0xa, 0, 0), 0xa);
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("crc4"),
            Some(crc4 as usize)
        );
    }
}
