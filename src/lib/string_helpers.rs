//! linux-parity: partial
//! linux-source: vendor/linux/lib/string_helpers.c
//! test-origin: linux:vendor/linux/lib/string_helpers.c
//! String helper exports used by Linux-built modules.

use core::ffi::c_char;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("string_get_size", linux_string_get_size as usize, true);
    export_symbol_once("skip_spaces", linux_skip_spaces as usize, false);
    export_symbol_once("strreplace", linux_strreplace as usize, false);
    export_symbol_once("match_string", linux_match_string as usize, false);
    export_symbol_once(
        "__sysfs_match_string",
        linux___sysfs_match_string as usize,
        true,
    );
    export_symbol_once("sysfs_streq", linux_sysfs_streq as usize, false);
}

unsafe fn cstr_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut idx = 0usize;
    loop {
        let av = unsafe { *a.add(idx) };
        let bv = unsafe { *b.add(idx) };
        if av != bv {
            return false;
        }
        if av == 0 {
            return true;
        }
        idx += 1;
    }
}

unsafe fn cstr_len_without_final_newline(s: *const c_char) -> Option<usize> {
    if s.is_null() {
        return None;
    }
    let mut len = 0usize;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    if len != 0 && unsafe { *s.add(len - 1) } == b'\n' as c_char {
        len -= 1;
    }
    Some(len)
}

fn linux_isspace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

unsafe fn write_byte(buf: *mut c_char, size: usize, pos: &mut usize, byte: u8) {
    if *pos + 1 < size {
        unsafe { *buf.add(*pos) = byte as c_char };
    }
    *pos += 1;
}

unsafe fn write_bytes(buf: *mut c_char, size: usize, pos: &mut usize, bytes: &[u8]) {
    for byte in bytes {
        unsafe { write_byte(buf, size, pos, *byte) };
    }
}

unsafe fn write_decimal(buf: *mut c_char, size: usize, pos: &mut usize, mut value: usize) {
    let mut tmp = [0u8; 32];
    let mut len = 0usize;
    if value == 0 {
        unsafe { write_byte(buf, size, pos, b'0') };
        return;
    }
    while value != 0 && len < tmp.len() {
        tmp[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len != 0 {
        len -= 1;
        unsafe { write_byte(buf, size, pos, tmp[len]) };
    }
}

unsafe fn terminate(buf: *mut c_char, size: usize, pos: usize) {
    if size == 0 || buf.is_null() {
        return;
    }
    let nul = core::cmp::min(pos, size - 1);
    unsafe { *buf.add(nul) = 0 };
}

/// `skip_spaces` - `vendor/linux/lib/string_helpers.c:846`.
#[unsafe(export_name = "skip_spaces")]
pub unsafe extern "C" fn linux_skip_spaces(str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        return core::ptr::null_mut();
    }
    let mut s = str_.cast::<u8>();
    while linux_isspace(unsafe { *s }) {
        s = unsafe { s.add(1) };
    }
    s.cast_mut().cast::<c_char>()
}

/// `string_get_size` - `vendor/linux/lib/string_helpers.c`.
pub unsafe extern "C" fn linux_string_get_size(
    size: u64,
    blk_size: u64,
    _units: i32,
    buf: *mut c_char,
    len: i32,
) {
    if buf.is_null() || len <= 0 {
        return;
    }
    let value = size.saturating_mul(blk_size);
    let mut pos = 0usize;
    unsafe { write_decimal(buf, len as usize, &mut pos, value as usize) };
    unsafe { write_bytes(buf, len as usize, &mut pos, b" B") };
    unsafe { terminate(buf, len as usize, pos) };
}

/// `strreplace` - `vendor/linux/lib/string_helpers.c`.
pub unsafe extern "C" fn linux_strreplace(
    str_: *mut c_char,
    old: c_char,
    new: c_char,
) -> *mut c_char {
    if str_.is_null() {
        return core::ptr::null_mut();
    }

    let mut s = str_;
    while unsafe { *s } != 0 {
        if unsafe { *s } == old {
            unsafe { *s = new };
        }
        s = unsafe { s.add(1) };
    }

    str_
}

/// `sysfs_streq` - `vendor/linux/lib/string_helpers.c`.
pub unsafe extern "C" fn linux_sysfs_streq(s1: *const c_char, s2: *const c_char) -> bool {
    let Some(len1) = (unsafe { cstr_len_without_final_newline(s1) }) else {
        return false;
    };
    let Some(len2) = (unsafe { cstr_len_without_final_newline(s2) }) else {
        return false;
    };
    if len1 != len2 {
        return false;
    }
    for idx in 0..len1 {
        if unsafe { *s1.add(idx) != *s2.add(idx) } {
            return false;
        }
    }
    true
}

/// `__sysfs_match_string` - `vendor/linux/lib/string_helpers.c`.
pub unsafe extern "C" fn linux___sysfs_match_string(
    array: *const *const c_char,
    n: usize,
    str_: *const c_char,
) -> i32 {
    if array.is_null() || str_.is_null() {
        return -22;
    }
    let mut idx = 0usize;
    while idx < n {
        let candidate = unsafe { *array.add(idx) };
        if !candidate.is_null() && unsafe { cstr_eq(candidate, str_) } {
            return idx as i32;
        }
        idx += 1;
    }
    -22
}

/// `match_string` - `vendor/linux/lib/string_helpers.c`.
pub unsafe extern "C" fn linux_match_string(
    array: *const *const c_char,
    n: usize,
    string: *const c_char,
) -> i32 {
    if array.is_null() || string.is_null() {
        return -22;
    }
    let mut idx = 0usize;
    while idx < n {
        let candidate = unsafe { *array.add(idx) };
        if candidate.is_null() {
            break;
        }
        if unsafe { cstr_eq(candidate, string) } {
            return idx as i32;
        }
        idx += 1;
    }
    -22
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_helper_exports_register_for_modules() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/string_helpers.c"
        ));
        assert!(source.contains("char *skip_spaces(const char *str)"));
        assert!(source.contains("while (isspace(*str))"));
        assert!(source.contains("EXPORT_SYMBOL(skip_spaces);"));
        assert!(source.contains("EXPORT_SYMBOL(strreplace);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("skip_spaces"),
            Some(linux_skip_spaces as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("string_get_size"),
            Some(linux_string_get_size as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("strreplace"),
            Some(linux_strreplace as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__sysfs_match_string"),
            Some(linux___sysfs_match_string as usize)
        );
    }

    #[test]
    fn sysfs_match_string_returns_linux_style_index_or_einval() {
        unsafe {
            let a = b"none\0";
            let b = b"writeback\0";
            let list = [a.as_ptr().cast::<c_char>(), b.as_ptr().cast::<c_char>()];
            assert_eq!(
                linux___sysfs_match_string(list.as_ptr(), list.len(), b.as_ptr().cast()),
                1
            );
            let miss = b"missing\0";
            assert_eq!(
                linux___sysfs_match_string(list.as_ptr(), list.len(), miss.as_ptr().cast()),
                -22
            );
        }
    }

    #[test]
    fn strreplace_replaces_in_place_and_returns_original_pointer() {
        let mut bytes = *b"a:b:c\0";
        let original = bytes.as_mut_ptr().cast::<c_char>();

        let returned = unsafe { linux_strreplace(original, b':' as c_char, b'_' as c_char) };

        assert_eq!(returned, original);
        assert_eq!(&bytes, b"a_b_c\0");
    }

    #[test]
    fn skip_spaces_returns_first_non_whitespace_byte() {
        let bytes = b" \t\nvalue\0";
        let result = unsafe { linux_skip_spaces(bytes.as_ptr().cast::<c_char>()) };

        assert_eq!(result, unsafe { bytes.as_ptr().add(3).cast_mut().cast() });
    }
}
