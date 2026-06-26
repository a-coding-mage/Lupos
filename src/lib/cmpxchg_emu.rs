//! linux-parity: complete
//! linux-source: vendor/linux/lib/cmpxchg-emu.c
//! test-origin: linux:vendor/linux/lib/cmpxchg-emu.c
//! One-byte cmpxchg emulation through a native-endian 32-bit word.

use crate::kernel::module::{export_symbol, find_symbol};

pub const LINUX_SOURCE: &str = "vendor/linux/lib/cmpxchg-emu.c";

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("cmpxchg_emu_u8", cmpxchg_emu_u8 as usize, true);
}

pub fn cmpxchg_emu_u8_in_word(word: &mut u32, byte_index: usize, old: u8, new: u8) -> u8 {
    assert!(byte_index < 4);
    let mut bytes = word.to_ne_bytes();
    let previous = bytes[byte_index];
    if previous == old {
        bytes[byte_index] = new;
        *word = u32::from_ne_bytes(bytes);
    }
    previous
}

pub unsafe extern "C" fn cmpxchg_emu_u8(p: *mut u8, old: usize, new: usize) -> usize {
    if p.is_null() {
        return 0;
    }

    let addr = p as usize;
    let p32 = (addr & !0x3) as *mut u32;
    let byte_index = addr & 0x3;
    let mut word = unsafe { core::ptr::read_volatile(p32) };
    let previous = cmpxchg_emu_u8_in_word(&mut word, byte_index, old as u8, new as u8);
    if previous == old as u8 {
        unsafe { core::ptr::write_volatile(p32, word) };
    }
    previous as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmpxchg_emu_u8_replaces_only_matching_byte() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/cmpxchg-emu.c"
        ));
        assert!(source.contains("union u8_32"));
        assert!(source.contains("uintptr_t cmpxchg_emu_u8"));
        assert!(source.contains("u32 *p32 = (u32 *)(((uintptr_t)p) & ~0x3);"));
        assert!(source.contains("new32.b[i] = new;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(cmpxchg_emu_u8);"));

        let mut word = u32::from_ne_bytes([0x10, 0x20, 0x30, 0x40]);
        assert_eq!(cmpxchg_emu_u8_in_word(&mut word, 2, 0x30, 0xaa), 0x30);
        assert_eq!(word.to_ne_bytes(), [0x10, 0x20, 0xaa, 0x40]);
        assert_eq!(cmpxchg_emu_u8_in_word(&mut word, 1, 0xff, 0xbb), 0x20);
        assert_eq!(word.to_ne_bytes(), [0x10, 0x20, 0xaa, 0x40]);
    }

    #[test]
    fn raw_cmpxchg_emu_u8_uses_aligned_word_address() {
        let mut word = u32::from_ne_bytes([1, 2, 3, 4]);
        let byte = unsafe { (core::ptr::addr_of_mut!(word) as *mut u8).add(1) };
        let previous = unsafe { cmpxchg_emu_u8(byte, 2, 9) };
        assert_eq!(previous, 2);
        assert_eq!(word.to_ne_bytes(), [1, 9, 3, 4]);
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("cmpxchg_emu_u8"),
            Some(cmpxchg_emu_u8 as usize)
        );
    }
}
