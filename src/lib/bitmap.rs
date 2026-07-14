//! linux-parity: partial
//! linux-source: vendor/linux/lib/bitmap.c
//! test-origin: linux:vendor/linux/lib/bitmap.c
//! Generic Linux bitmap helpers exported to vendor modules.

use core::mem::size_of;

use crate::kernel::module::{export_symbol, find_symbol};

const BITS_PER_LONG: usize = usize::BITS as usize;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__bitmap_equal", linux___bitmap_equal as usize, false);
    export_symbol_once("__bitmap_and", linux___bitmap_and as usize, false);
    export_symbol_once("__bitmap_or", linux___bitmap_or as usize, false);
    export_symbol_once("__bitmap_andnot", linux___bitmap_andnot as usize, false);
    export_symbol_once(
        "__bitmap_intersects",
        linux___bitmap_intersects as usize,
        false,
    );
    export_symbol_once("__bitmap_subset", linux___bitmap_subset as usize, false);
    export_symbol_once("__bitmap_weight", linux___bitmap_weight as usize, false);
    export_symbol_once("__bitmap_set", linux___bitmap_set as usize, false);
    export_symbol_once("__bitmap_clear", linux___bitmap_clear as usize, false);
    export_symbol_once("bitmap_zalloc", linux_bitmap_zalloc as usize, false);
    export_symbol_once("bitmap_free", linux_bitmap_free as usize, false);
    export_symbol_once("bitmap_from_arr32", linux_bitmap_from_arr32 as usize, false);
}

fn bits_to_longs(bits: usize) -> usize {
    bits.div_ceil(BITS_PER_LONG)
}

fn bitmap_last_word_mask(bits: usize) -> usize {
    let rem = bits % BITS_PER_LONG;
    if rem == 0 {
        usize::MAX
    } else {
        (1usize << rem) - 1
    }
}

/// `__bitmap_equal` - `vendor/linux/lib/bitmap.c:37`.
pub unsafe extern "C" fn linux___bitmap_equal(
    bitmap1: *const usize,
    bitmap2: *const usize,
    bits: u32,
) -> bool {
    let bits = bits as usize;
    let full_words = bits / BITS_PER_LONG;
    if bits != 0 && (bitmap1.is_null() || bitmap2.is_null()) {
        return false;
    }

    for index in 0..full_words {
        if unsafe { *bitmap1.add(index) != *bitmap2.add(index) } {
            return false;
        }
    }

    if bits % BITS_PER_LONG != 0 {
        let mask = bitmap_last_word_mask(bits);
        if unsafe { (*bitmap1.add(full_words) ^ *bitmap2.add(full_words)) & mask } != 0 {
            return false;
        }
    }

    true
}

/// `__bitmap_and` - `vendor/linux/lib/bitmap.c:230`.
pub unsafe extern "C" fn linux___bitmap_and(
    dst: *mut usize,
    bitmap1: *const usize,
    bitmap2: *const usize,
    bits: u32,
) -> bool {
    let bits = bits as usize;
    let full_words = bits / BITS_PER_LONG;
    if bits != 0 && (dst.is_null() || bitmap1.is_null() || bitmap2.is_null()) {
        return false;
    }

    let mut result = 0usize;
    for index in 0..full_words {
        let word = unsafe { *bitmap1.add(index) & *bitmap2.add(index) };
        unsafe { *dst.add(index) = word };
        result |= word;
    }

    if bits % BITS_PER_LONG != 0 {
        let index = full_words;
        let word =
            unsafe { (*bitmap1.add(index) & *bitmap2.add(index)) & bitmap_last_word_mask(bits) };
        unsafe { *dst.add(index) = word };
        result |= word;
    }

    result != 0
}

/// `__bitmap_or` - `vendor/linux/lib/bitmap.c:246`.
pub unsafe extern "C" fn linux___bitmap_or(
    dst: *mut usize,
    bitmap1: *const usize,
    bitmap2: *const usize,
    bits: u32,
) {
    let words = bits_to_longs(bits as usize);
    if words != 0 && (dst.is_null() || bitmap1.is_null() || bitmap2.is_null()) {
        return;
    }
    for index in 0..words {
        unsafe {
            *dst.add(index) = *bitmap1.add(index) | *bitmap2.add(index);
        }
    }
}

/// `__bitmap_andnot` - `vendor/linux/lib/bitmap.c:268`.
pub unsafe extern "C" fn linux___bitmap_andnot(
    dst: *mut usize,
    bitmap1: *const usize,
    bitmap2: *const usize,
    bits: u32,
) -> bool {
    let bits = bits as usize;
    let full_words = bits / BITS_PER_LONG;
    if bits != 0 && (dst.is_null() || bitmap1.is_null() || bitmap2.is_null()) {
        return false;
    }

    let mut result = 0usize;
    for index in 0..full_words {
        let word = unsafe { *bitmap1.add(index) & !*bitmap2.add(index) };
        unsafe { *dst.add(index) = word };
        result |= word;
    }

    if bits % BITS_PER_LONG != 0 {
        let index = full_words;
        let word =
            unsafe { (*bitmap1.add(index) & !*bitmap2.add(index)) & bitmap_last_word_mask(bits) };
        unsafe { *dst.add(index) = word };
        result |= word;
    }

    result != 0
}

/// `__bitmap_intersects` - `vendor/linux/lib/bitmap.c:296`.
pub unsafe extern "C" fn linux___bitmap_intersects(
    bitmap1: *const usize,
    bitmap2: *const usize,
    bits: u32,
) -> bool {
    let bits = bits as usize;
    let full_words = bits / BITS_PER_LONG;
    if bits != 0 && (bitmap1.is_null() || bitmap2.is_null()) {
        return false;
    }

    for index in 0..full_words {
        if unsafe { *bitmap1.add(index) & *bitmap2.add(index) } != 0 {
            return true;
        }
    }

    if bits % BITS_PER_LONG != 0 {
        let mask = bitmap_last_word_mask(bits);
        if unsafe { (*bitmap1.add(full_words) & *bitmap2.add(full_words)) & mask } != 0 {
            return true;
        }
    }

    false
}

/// `__bitmap_subset` - `vendor/linux/lib/bitmap.c:311`.
pub unsafe extern "C" fn linux___bitmap_subset(
    bitmap1: *const usize,
    bitmap2: *const usize,
    bits: u32,
) -> bool {
    let bits = bits as usize;
    let full_words = bits / BITS_PER_LONG;

    if bits != 0 && (bitmap1.is_null() || bitmap2.is_null()) {
        return false;
    }

    for index in 0..full_words {
        let left = unsafe { *bitmap1.add(index) };
        let right = unsafe { *bitmap2.add(index) };
        if left & !right != 0 {
            return false;
        }
    }

    if bits % BITS_PER_LONG != 0 {
        let mask = bitmap_last_word_mask(bits);
        let left = unsafe { *bitmap1.add(full_words) };
        let right = unsafe { *bitmap2.add(full_words) };
        if (left & !right) & mask != 0 {
            return false;
        }
    }

    true
}

/// `__bitmap_weight` - `vendor/linux/lib/bitmap.c:339`.
pub unsafe extern "C" fn linux___bitmap_weight(bitmap: *const usize, bits: u32) -> u32 {
    let bits = bits as usize;
    let full_words = bits / BITS_PER_LONG;
    if bits != 0 && bitmap.is_null() {
        return 0;
    }

    let mut weight = 0u32;
    for index in 0..full_words {
        weight = weight.saturating_add(unsafe { (*bitmap.add(index)).count_ones() });
    }
    if bits % BITS_PER_LONG != 0 {
        let word = unsafe { *bitmap.add(full_words) & bitmap_last_word_mask(bits) };
        weight = weight.saturating_add(word.count_ones());
    }
    weight
}

/// `__bitmap_set` - `vendor/linux/lib/bitmap.c:373`.
pub unsafe extern "C" fn linux___bitmap_set(map: *mut usize, start: u32, len: i32) {
    if map.is_null() || len <= 0 {
        return;
    }
    let start = start as usize;
    for offset in 0..len as usize {
        let Some(bit) = start.checked_add(offset) else {
            break;
        };
        let word = bit / BITS_PER_LONG;
        let mask = 1usize << (bit % BITS_PER_LONG);
        unsafe {
            *map.add(word) |= mask;
        }
    }
}

/// `__bitmap_clear` - `vendor/linux/lib/bitmap.c:394`.
pub unsafe extern "C" fn linux___bitmap_clear(map: *mut usize, start: u32, len: i32) {
    if map.is_null() || len <= 0 {
        return;
    }
    let start = start as usize;
    for offset in 0..len as usize {
        let Some(bit) = start.checked_add(offset) else {
            break;
        };
        let word = bit / BITS_PER_LONG;
        let mask = 1usize << (bit % BITS_PER_LONG);
        unsafe {
            *map.add(word) &= !mask;
        }
    }
}

/// `bitmap_zalloc` - `vendor/linux/lib/bitmap.c:739`.
pub unsafe extern "C" fn linux_bitmap_zalloc(nbits: u32, flags: u32) -> *mut usize {
    let bytes = bits_to_longs(nbits as usize).saturating_mul(size_of::<usize>());
    if bytes == 0 {
        return core::ptr::null_mut();
    }
    let ptr = unsafe { crate::mm::slab::linux___kmalloc_noprof(bytes, flags) };
    if !ptr.is_null() {
        unsafe { core::ptr::write_bytes(ptr, 0, bytes) };
    }
    ptr.cast()
}

/// `bitmap_free` - `vendor/linux/lib/bitmap.c:758`.
pub unsafe extern "C" fn linux_bitmap_free(bitmap: *const usize) {
    unsafe { crate::mm::slab::linux_kfree(bitmap.cast_mut().cast::<u8>()) };
}

/// `bitmap_from_arr32` - `vendor/linux/lib/bitmap.c:803`.
pub unsafe extern "C" fn linux_bitmap_from_arr32(bitmap: *mut usize, buf: *const u32, nbits: u32) {
    let nbits = nbits as usize;
    if nbits == 0 || bitmap.is_null() || buf.is_null() {
        return;
    }

    let halfwords = nbits.div_ceil(32);
    for word_index in 0..bits_to_longs(nbits) {
        unsafe { *bitmap.add(word_index) = 0 };
    }

    for halfword in 0..halfwords {
        let value = unsafe { *buf.add(halfword) } as usize;
        let word = halfword / (BITS_PER_LONG / 32);
        let shift = (halfword % (BITS_PER_LONG / 32)) * 32;
        unsafe {
            *bitmap.add(word) |= value << shift;
        }
    }

    if nbits % BITS_PER_LONG != 0 {
        let last = bits_to_longs(nbits) - 1;
        unsafe {
            *bitmap.add(last) &= bitmap_last_word_mask(nbits);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_subset_exports_register() {
        register_module_exports();
        for name in [
            "__bitmap_equal",
            "__bitmap_and",
            "__bitmap_or",
            "__bitmap_andnot",
            "__bitmap_intersects",
            "__bitmap_subset",
            "__bitmap_weight",
            "__bitmap_set",
            "__bitmap_clear",
            "bitmap_zalloc",
            "bitmap_free",
            "bitmap_from_arr32",
        ] {
            assert!(
                crate::kernel::module::find_symbol(name).is_some(),
                "missing export {name}"
            );
        }
    }

    #[test]
    fn bitmap_subset_matches_partial_last_word() {
        let left = [0b0011usize, 1usize << 7];
        let right = [0b0111usize, 0];

        unsafe {
            assert!(linux___bitmap_subset(left.as_ptr(), right.as_ptr(), 64));
            assert!(linux___bitmap_subset(left.as_ptr(), right.as_ptr(), 65));
            assert!(!linux___bitmap_subset(left.as_ptr(), right.as_ptr(), 72));
        }
    }

    #[test]
    fn bitmap_subset_rejects_extra_full_word_bits() {
        let left = [0usize, 0b10usize];
        let right = [0usize, 0b01usize];

        unsafe {
            assert!(!linux___bitmap_subset(left.as_ptr(), right.as_ptr(), 128));
        }
    }

    #[test]
    fn bitmap_boolean_helpers_match_tail_masks() {
        let left = [0b0011usize, 1usize << 7];
        let right = [0b0101usize, 1usize << 8];
        let mut dst = [0usize; 2];

        unsafe {
            assert!(linux___bitmap_equal(left.as_ptr(), left.as_ptr(), 72));
            let tail_left = [0b0011usize, 1usize << 7];
            let tail_right = [0b0011usize, 1usize << 8];
            assert!(linux___bitmap_equal(
                tail_left.as_ptr(),
                tail_right.as_ptr(),
                64
            ));
            assert!(!linux___bitmap_equal(
                tail_left.as_ptr(),
                tail_right.as_ptr(),
                72
            ));

            assert!(linux___bitmap_and(
                dst.as_mut_ptr(),
                left.as_ptr(),
                right.as_ptr(),
                72
            ));
            assert_eq!(dst[0], 0b0001);
            assert_eq!(dst[1], 0);

            linux___bitmap_or(dst.as_mut_ptr(), left.as_ptr(), right.as_ptr(), 65);
            assert_eq!(dst, [0b0111usize, (1usize << 7) | (1usize << 8)]);

            assert!(linux___bitmap_intersects(left.as_ptr(), right.as_ptr(), 64));
            assert!(!linux___bitmap_intersects(
                left.as_ptr().add(1),
                right.as_ptr().add(1),
                8
            ));
            assert!(linux___bitmap_andnot(
                dst.as_mut_ptr(),
                left.as_ptr(),
                right.as_ptr(),
                72
            ));
            assert_eq!(dst[0], 0b0010);
            assert_eq!(dst[1], 1usize << 7);
        }
    }

    #[test]
    fn bitmap_exports_track_vendor_symbols() {
        let source = include_str!("../../vendor/linux/lib/bitmap.c");
        for symbol in [
            "EXPORT_SYMBOL(__bitmap_equal);",
            "EXPORT_SYMBOL(__bitmap_and);",
            "EXPORT_SYMBOL(__bitmap_or);",
            "EXPORT_SYMBOL(__bitmap_andnot);",
            "EXPORT_SYMBOL(__bitmap_intersects);",
        ] {
            assert!(source.contains(symbol), "missing vendor export {symbol}");
        }
    }

    #[test]
    fn bitmap_set_clear_weight_and_arr32_work() {
        let mut map = [0usize; 2];

        unsafe {
            linux___bitmap_set(map.as_mut_ptr(), 3, 66);
            assert_eq!(linux___bitmap_weight(map.as_ptr(), 128), 66);
            linux___bitmap_clear(map.as_mut_ptr(), 4, 64);
            assert_eq!(linux___bitmap_weight(map.as_ptr(), 128), 2);

            let buf = [0xffff_0000u32, 0x8000_0001u32, 0xffff_ffffu32];
            linux_bitmap_from_arr32(map.as_mut_ptr(), buf.as_ptr(), 65);
            assert_eq!(map[0], 0x8000_0001_ffff_0000usize);
            assert_eq!(map[1], 1);
        }
    }
}
