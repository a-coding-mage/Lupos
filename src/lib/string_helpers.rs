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
    export_symbol_once(
        "__sysfs_match_string",
        linux___sysfs_match_string as usize,
        true,
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_helper_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("string_get_size"),
            Some(linux_string_get_size as usize)
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
}
