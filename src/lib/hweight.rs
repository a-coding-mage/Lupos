//! linux-parity: complete
//! linux-source: vendor/linux/lib/hweight.c
//! test-origin: linux:vendor/linux/lib/hweight.c
//! Software Hamming weight helpers.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__sw_hweight8", __sw_hweight8 as usize, false);
    export_symbol_once("__sw_hweight16", __sw_hweight16 as usize, false);
    export_symbol_once("__sw_hweight32", __sw_hweight32 as usize, false);
    export_symbol_once("__sw_hweight64", __sw_hweight64 as usize, false);
}

pub const fn __sw_hweight8(w: u32) -> u32 {
    let mut res = w & 0xff;
    res -= (res >> 1) & 0x55;
    res = (res & 0x33) + ((res >> 2) & 0x33);
    (res + (res >> 4)) & 0x0f
}

pub const fn __sw_hweight16(w: u32) -> u32 {
    let mut res = w & 0xffff;
    res -= (res >> 1) & 0x5555;
    res = (res & 0x3333) + ((res >> 2) & 0x3333);
    res = (res + (res >> 4)) & 0x0f0f;
    (res + (res >> 8)) & 0x00ff
}

pub const fn __sw_hweight32(w: u32) -> u32 {
    let mut res = w - ((w >> 1) & 0x5555_5555);
    res = (res & 0x3333_3333) + ((res >> 2) & 0x3333_3333);
    res = (res + (res >> 4)) & 0x0f0f_0f0f;
    res += res >> 8;
    (res + (res >> 16)) & 0x0000_00ff
}

pub const fn __sw_hweight64(w: u64) -> usize {
    let mut res = w - ((w >> 1) & 0x5555_5555_5555_5555);
    res = (res & 0x3333_3333_3333_3333) + ((res >> 2) & 0x3333_3333_3333_3333);
    res = (res + (res >> 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    res += res >> 8;
    res += res >> 16;
    ((res + (res >> 32)) & 0x0000_0000_0000_00ff) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hweight_helpers_match_linux_masks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/hweight.c"
        ));
        assert!(source.contains("w - ((w >> 1) & 0x55555555)"));
        assert!(source.contains("(res & 0x33333333) + ((res >> 2) & 0x33333333)"));
        assert!(source.contains("EXPORT_SYMBOL(__sw_hweight32);"));
        assert!(source.contains("EXPORT_SYMBOL(__sw_hweight64);"));

        assert_eq!(__sw_hweight8(0b1010_0101), 4);
        assert_eq!(__sw_hweight16(0xffff), 16);
        assert_eq!(__sw_hweight32(0xf0f0_f00f), 16);
        assert_eq!(__sw_hweight64(u64::MAX), 64);
        assert_eq!(__sw_hweight64(0x8000_0000_0000_0001), 2);
    }
}
