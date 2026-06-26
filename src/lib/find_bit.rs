//! linux-parity: partial
//! linux-source: vendor/linux/lib/find_bit.c
//! test-origin: linux:vendor/linux/lib/find_bit.c
//! Bit search helpers exported to Linux-built modules.

use crate::kernel::module::{export_symbol, find_symbol};

const BITS_PER_LONG: usize = usize::BITS as usize;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("_find_first_bit", _find_first_bit as usize, false);
    export_symbol_once("_find_next_bit", _find_next_bit as usize, false);
    export_symbol_once("_find_first_zero_bit", _find_first_zero_bit as usize, false);
    export_symbol_once("_find_next_zero_bit", _find_next_zero_bit as usize, false);
}

#[inline]
fn first_word_mask(start: usize) -> usize {
    usize::MAX << (start % BITS_PER_LONG)
}

#[inline]
fn last_word_mask(valid_bits: usize) -> usize {
    if valid_bits >= BITS_PER_LONG {
        usize::MAX
    } else if valid_bits == 0 {
        0
    } else {
        (1usize << valid_bits) - 1
    }
}

unsafe fn find_next_bit_value(
    addr: *const usize,
    size: usize,
    start: usize,
    invert: bool,
) -> usize {
    if addr.is_null() || start >= size {
        return size;
    }

    let mut index = start / BITS_PER_LONG;
    loop {
        let base = index * BITS_PER_LONG;
        if base >= size {
            return size;
        }

        let mut word = unsafe { *addr.add(index) };
        if invert {
            word = !word;
        }
        if index == start / BITS_PER_LONG {
            word &= first_word_mask(start);
        }
        word &= last_word_mask(size - base);

        if word != 0 {
            return (base + word.trailing_zeros() as usize).min(size);
        }

        index += 1;
    }
}

pub unsafe extern "C" fn _find_first_bit(addr: *const usize, size: usize) -> usize {
    unsafe { find_next_bit_value(addr, size, 0, false) }
}

pub unsafe extern "C" fn _find_next_bit(addr: *const usize, size: usize, offset: usize) -> usize {
    unsafe { find_next_bit_value(addr, size, offset, false) }
}

pub unsafe extern "C" fn _find_first_zero_bit(addr: *const usize, size: usize) -> usize {
    unsafe { find_next_bit_value(addr, size, 0, true) }
}

pub unsafe extern "C" fn _find_next_zero_bit(
    addr: *const usize,
    size: usize,
    offset: usize,
) -> usize {
    unsafe { find_next_bit_value(addr, size, offset, true) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_helpers_match_linux_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/find_bit.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL(_find_first_bit);"));
        assert!(source.contains("EXPORT_SYMBOL(_find_next_bit);"));
        assert!(source.contains("EXPORT_SYMBOL(_find_first_zero_bit);"));
        assert!(source.contains("EXPORT_SYMBOL(_find_next_zero_bit);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("_find_next_bit"),
            Some(_find_next_bit as usize)
        );
    }

    #[test]
    fn finds_set_bits_across_word_boundaries() {
        let mut bitmap = [0usize; 3];
        bitmap[0] = (1usize << 3) | (1usize << 17);
        bitmap[1] = 1usize << 2;

        unsafe {
            assert_eq!(_find_first_bit(bitmap.as_ptr(), BITS_PER_LONG * 3), 3);
            assert_eq!(_find_next_bit(bitmap.as_ptr(), BITS_PER_LONG * 3, 4), 17);
            assert_eq!(
                _find_next_bit(bitmap.as_ptr(), BITS_PER_LONG * 3, 18),
                BITS_PER_LONG + 2
            );
            assert_eq!(
                _find_next_bit(bitmap.as_ptr(), BITS_PER_LONG + 2, BITS_PER_LONG + 2),
                BITS_PER_LONG + 2
            );
        }
    }

    #[test]
    fn finds_zero_bits_with_tail_masking() {
        let bitmap = [usize::MAX, !(1usize << 5)];
        unsafe {
            assert_eq!(
                _find_first_zero_bit(bitmap.as_ptr(), BITS_PER_LONG * 2),
                BITS_PER_LONG + 5
            );
            assert_eq!(
                _find_next_zero_bit(bitmap.as_ptr(), BITS_PER_LONG + 5, 0),
                BITS_PER_LONG + 5
            );
            assert_eq!(
                _find_next_zero_bit(bitmap.as_ptr(), BITS_PER_LONG + 6, BITS_PER_LONG + 6),
                BITS_PER_LONG + 6
            );
        }
    }
}
