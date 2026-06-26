//! linux-parity: complete
//! linux-source: vendor/linux/lib/ucs2_string.c
//! test-origin: linux:vendor/linux/lib/ucs2_string.c
//! UCS-2 string helpers.

use crate::include::uapi::errno::E2BIG;
use crate::kernel::module::{export_symbol, find_symbol};

pub type Ucs2Char = u16;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("ucs2_strnlen", ucs2_strnlen as usize, false);
    export_symbol_once("ucs2_strlen", ucs2_strlen as usize, false);
    export_symbol_once("ucs2_strsize", ucs2_strsize as usize, false);
    export_symbol_once("ucs2_strscpy", ucs2_strscpy as usize, false);
    export_symbol_once("ucs2_strncmp", ucs2_strncmp as usize, false);
    export_symbol_once("ucs2_utf8size", ucs2_utf8size as usize, false);
    export_symbol_once("ucs2_as_utf8", ucs2_as_utf8 as usize, false);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucs2_strnlen(mut s: *const Ucs2Char, maxlength: usize) -> usize {
    let mut length = 0usize;
    while unsafe { *s } != 0 && length < maxlength {
        s = unsafe { s.add(1) };
        length += 1;
    }
    length
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucs2_strlen(s: *const Ucs2Char) -> usize {
    unsafe { ucs2_strnlen(s, usize::MAX) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucs2_strsize(data: *const Ucs2Char, maxlength: usize) -> usize {
    (unsafe { ucs2_strnlen(data, maxlength / core::mem::size_of::<Ucs2Char>()) })
        * core::mem::size_of::<Ucs2Char>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucs2_strscpy(
    dst: *mut Ucs2Char,
    src: *const Ucs2Char,
    count: usize,
) -> isize {
    if count == 0 || count > i32::MAX as usize / core::mem::size_of::<Ucs2Char>() {
        return -(E2BIG as isize);
    }

    let mut res = 0usize;
    while res < count {
        let c = unsafe { *src.add(res) };
        unsafe { *dst.add(res) = c };
        if c == 0 {
            return res as isize;
        }
        res += 1;
    }

    unsafe { *dst.add(count - 1) = 0 };
    -(E2BIG as isize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucs2_strncmp(
    mut a: *const Ucs2Char,
    mut b: *const Ucs2Char,
    mut len: usize,
) -> i32 {
    loop {
        if len == 0 {
            return 0;
        }
        let av = unsafe { *a };
        let bv = unsafe { *b };
        if av < bv {
            return -1;
        }
        if av > bv {
            return 1;
        }
        if av == 0 {
            return 0;
        }
        a = unsafe { a.add(1) };
        b = unsafe { b.add(1) };
        len -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucs2_utf8size(src: *const Ucs2Char) -> usize {
    let mut i = 0usize;
    let mut j = 0usize;
    loop {
        let c = unsafe { *src.add(i) };
        if c == 0 {
            return j;
        }
        if c >= 0x800 {
            j += 3;
        } else if c >= 0x80 {
            j += 2;
        } else {
            j += 1;
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ucs2_as_utf8(
    dest: *mut u8,
    src: *const Ucs2Char,
    mut maxlength: usize,
) -> usize {
    let mut j = 0usize;
    let limit = unsafe { ucs2_strnlen(src, maxlength) };

    let mut i = 0usize;
    while maxlength != 0 && i < limit {
        let c = unsafe { *src.add(i) };
        if c >= 0x800 {
            if maxlength < 3 {
                break;
            }
            maxlength -= 3;
            unsafe {
                *dest.add(j) = 0xe0 | (((c & 0xf000) >> 12) as u8);
                *dest.add(j + 1) = 0x80 | (((c & 0x0fc0) >> 6) as u8);
                *dest.add(j + 2) = 0x80 | ((c & 0x003f) as u8);
            }
            j += 3;
        } else if c >= 0x80 {
            if maxlength < 2 {
                break;
            }
            maxlength -= 2;
            unsafe {
                *dest.add(j) = 0xc0 | (((c & 0x07c0) >> 6) as u8);
                *dest.add(j + 1) = 0x80 | ((c & 0x003f) as u8);
            }
            j += 2;
        } else {
            maxlength -= 1;
            unsafe { *dest.add(j) = (c & 0x007f) as u8 };
            j += 1;
        }
        i += 1;
    }

    if maxlength != 0 {
        unsafe { *dest.add(j) = 0 };
    }
    j
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ucs2_string_source_matches_linux_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/ucs2_string.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/ucs2_string.h"
        ));
        assert!(source.contains("ucs2_strnlen(const ucs2_char_t *s, size_t maxlength)"));
        assert!(source.contains("return ucs2_strnlen(s, ~0UL);"));
        assert!(source.contains("maxlength/sizeof(ucs2_char_t)"));
        assert!(source.contains("return -E2BIG;"));
        assert!(source.contains("dst[count - 1] = 0;"));
        assert!(source.contains("dest[j++] = 0xe0 | (c & 0xf000) >> 12;"));
        assert!(source.contains("EXPORT_SYMBOL(ucs2_as_utf8);"));
        assert!(header.contains("typedef u16 ucs2_char_t;"));
    }

    #[test]
    fn ucs2_lengths_and_sizes_match_linux() {
        let text = [b'A' as u16, 0x00e9, 0x20ac, 0];
        assert_eq!(unsafe { ucs2_strnlen(text.as_ptr(), 0) }, 0);
        assert_eq!(unsafe { ucs2_strnlen(text.as_ptr(), 2) }, 2);
        assert_eq!(unsafe { ucs2_strlen(text.as_ptr()) }, 3);
        assert_eq!(unsafe { ucs2_strsize(text.as_ptr(), 3) }, 2);
        assert_eq!(unsafe { ucs2_strsize(text.as_ptr(), 6) }, 6);
        assert_eq!(unsafe { ucs2_utf8size(text.as_ptr()) }, 6);
    }

    #[test]
    fn ucs2_strscpy_copies_or_truncates_like_strscpy() {
        let src = [b'A' as u16, b'B' as u16, b'C' as u16, 0];
        let mut dst = [0xffffu16; 5];
        assert_eq!(
            unsafe { ucs2_strscpy(dst.as_mut_ptr(), src.as_ptr(), 5) },
            3
        );
        assert_eq!(&dst[..4], &src);

        let mut truncated = [0xffffu16; 3];
        assert_eq!(
            unsafe { ucs2_strscpy(truncated.as_mut_ptr(), src.as_ptr(), 3) },
            -(E2BIG as isize)
        );
        assert_eq!(truncated, [b'A' as u16, b'B' as u16, 0]);
        assert_eq!(
            unsafe { ucs2_strscpy(truncated.as_mut_ptr(), src.as_ptr(), 0) },
            -(E2BIG as isize)
        );
    }

    #[test]
    fn ucs2_strncmp_matches_lexical_and_nul_rules() {
        let a = [b'a' as u16, b'b' as u16, 0];
        let b = [b'a' as u16, b'c' as u16, 0];
        assert_eq!(unsafe { ucs2_strncmp(a.as_ptr(), b.as_ptr(), 1) }, 0);
        assert_eq!(unsafe { ucs2_strncmp(a.as_ptr(), b.as_ptr(), 2) }, -1);
        assert_eq!(unsafe { ucs2_strncmp(b.as_ptr(), a.as_ptr(), 2) }, 1);
        assert_eq!(unsafe { ucs2_strncmp(a.as_ptr(), a.as_ptr(), 10) }, 0);
    }

    #[test]
    fn ucs2_as_utf8_copies_whole_characters_and_nul_terminates_when_space_remains() {
        let src = [b'A' as u16, 0x00e9, 0x20ac, 0];
        let mut out = [0xffu8; 8];
        assert_eq!(
            unsafe { ucs2_as_utf8(out.as_mut_ptr(), src.as_ptr(), out.len()) },
            6
        );
        assert_eq!(&out[..7], &[0x41, 0xc3, 0xa9, 0xe2, 0x82, 0xac, 0]);

        let mut short = [0xffu8; 4];
        assert_eq!(
            unsafe { ucs2_as_utf8(short.as_mut_ptr(), src.as_ptr(), short.len()) },
            3
        );
        assert_eq!(short, [0x41, 0xc3, 0xa9, 0]);

        let mut exact = [0xffu8; 3];
        assert_eq!(
            unsafe { ucs2_as_utf8(exact.as_mut_ptr(), src.as_ptr(), exact.len()) },
            3
        );
        assert_eq!(exact, [0x41, 0xc3, 0xa9]);
    }

    #[test]
    fn ucs2_exports_register_for_modules() {
        register_module_exports();
        for (name, addr) in [
            ("ucs2_strnlen", ucs2_strnlen as usize),
            ("ucs2_strlen", ucs2_strlen as usize),
            ("ucs2_strsize", ucs2_strsize as usize),
            ("ucs2_strscpy", ucs2_strscpy as usize),
            ("ucs2_strncmp", ucs2_strncmp as usize),
            ("ucs2_utf8size", ucs2_utf8size as usize),
            ("ucs2_as_utf8", ucs2_as_utf8 as usize),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
        }
    }
}
