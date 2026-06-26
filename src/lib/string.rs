//! linux-parity: complete
//! linux-source: vendor/linux/lib/string.c
//! test-origin: linux:vendor/linux/lib/string.c
//! C string helpers exported to Linux-built modules.

extern crate alloc;

use core::ffi::{c_char, c_int, c_void};

use crate::include::uapi::errno::E2BIG;
use crate::kernel::module::{export_symbol, find_symbol};

const EXPORTED_STRING_SYMBOLS: &[&str] = &[
    "strncasecmp",
    "strcasecmp",
    "strcpy",
    "strncpy",
    "sized_strscpy",
    "stpcpy",
    "strcat",
    "strncat",
    "strlcat",
    "strcmp",
    "strncmp",
    "strchr",
    "strchrnul",
    "strrchr",
    "strnchr",
    "strlen",
    "strnlen",
    "strspn",
    "strcspn",
    "strpbrk",
    "strsep",
    "memset",
    "memset16",
    "memset32",
    "memset64",
    "memcpy",
    "memmove",
    "memcmp",
    "bcmp",
    "memscan",
    "strstr",
    "strnstr",
    "memchr",
    "memchr_inv",
];

fn symbol_addr(name: &str) -> Option<usize> {
    Some(match name {
        "strncasecmp" => linux_strncasecmp as usize,
        "strcasecmp" => linux_strcasecmp as usize,
        "strcpy" => linux_strcpy as usize,
        "strncpy" => linux_strncpy as usize,
        "sized_strscpy" => linux_sized_strscpy as usize,
        "stpcpy" => linux_stpcpy as usize,
        "strcat" => linux_strcat as usize,
        "strncat" => linux_strncat as usize,
        "strlcat" => linux_strlcat as usize,
        "strcmp" => linux_strcmp as usize,
        "strncmp" => linux_strncmp as usize,
        "strchr" => linux_strchr as usize,
        "strchrnul" => linux_strchrnul as usize,
        "strrchr" => linux_strrchr as usize,
        "strnchr" => linux_strnchr as usize,
        "strlen" => linux_strlen as usize,
        "strnlen" => linux_strnlen as usize,
        "strspn" => linux_strspn as usize,
        "strcspn" => linux_strcspn as usize,
        "strpbrk" => linux_strpbrk as usize,
        "strsep" => linux_strsep as usize,
        "memset" => linux_memset as usize,
        "memset16" => linux_memset16 as usize,
        "memset32" => linux_memset32 as usize,
        "memset64" => linux_memset64 as usize,
        "memcpy" => linux_memcpy as usize,
        "memmove" => linux_memmove as usize,
        "memcmp" => linux_memcmp as usize,
        "bcmp" => linux_bcmp as usize,
        "memscan" => linux_memscan as usize,
        "strstr" => linux_strstr as usize,
        "strnstr" => linux_strnstr as usize,
        "memchr" => linux_memchr as usize,
        "memchr_inv" => linux_memchr_inv as usize,
        _ => return None,
    })
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    for name in EXPORTED_STRING_SYMBOLS {
        export_symbol_once(name, symbol_addr(name).expect("known string symbol"), true);
    }
}

#[inline]
unsafe fn read_c_byte(ptr: *const c_char, idx: usize) -> u8 {
    unsafe { *ptr.cast::<u8>().add(idx) }
}

#[inline]
unsafe fn write_c_byte(ptr: *mut c_char, idx: usize, value: u8) {
    unsafe {
        *ptr.cast::<u8>().add(idx) = value;
    }
}

#[inline]
const fn ascii_lower(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' {
        byte + (b'a' - b'A')
    } else {
        byte
    }
}

pub unsafe fn c_strlen(ptr: *const c_char, max: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let mut len = 0usize;
    while len < max {
        if unsafe { read_c_byte(ptr, len) } == 0 {
            break;
        }
        len += 1;
    }
    len
}

/// `strncasecmp` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strncasecmp(
    s1: *const c_char,
    s2: *const c_char,
    len: usize,
) -> c_int {
    if len == 0 {
        return 0;
    }
    let mut idx = 0usize;
    loop {
        let c1 = unsafe { read_c_byte(s1, idx) };
        let c2 = unsafe { read_c_byte(s2, idx) };
        if c1 == 0 || c2 == 0 {
            return c1 as c_int - c2 as c_int;
        }
        let l1 = ascii_lower(c1);
        let l2 = ascii_lower(c2);
        if l1 != l2 {
            return l1 as c_int - l2 as c_int;
        }
        idx += 1;
        if idx == len {
            return 0;
        }
    }
}

/// `strcasecmp` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int {
    let mut idx = 0usize;
    loop {
        let c1 = ascii_lower(unsafe { read_c_byte(s1, idx) });
        let c2 = ascii_lower(unsafe { read_c_byte(s2, idx) });
        if c1 != c2 || c1 == 0 {
            return c1 as c_int - c2 as c_int;
        }
        idx += 1;
    }
}

/// `strcpy` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut idx = 0usize;
    loop {
        let byte = unsafe { read_c_byte(src, idx) };
        unsafe { write_c_byte(dest, idx, byte) };
        if byte == 0 {
            return dest;
        }
        idx += 1;
    }
}

/// `strncpy` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strncpy(
    dest: *mut c_char,
    src: *const c_char,
    count: usize,
) -> *mut c_char {
    let mut idx = 0usize;
    while idx < count {
        let byte = unsafe { read_c_byte(src, idx) };
        unsafe { write_c_byte(dest, idx, byte) };
        idx += 1;
        if byte == 0 {
            break;
        }
    }
    while idx < count {
        unsafe { write_c_byte(dest, idx, 0) };
        idx += 1;
    }
    dest
}

/// `sized_strscpy` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_sized_strscpy(
    dest: *mut c_char,
    src: *const c_char,
    count: usize,
) -> isize {
    if count == 0 || count > c_int::MAX as usize {
        return -(E2BIG as isize);
    }
    let mut idx = 0usize;
    while idx + 1 < count {
        let byte = unsafe { read_c_byte(src, idx) };
        unsafe { write_c_byte(dest, idx, byte) };
        if byte == 0 {
            return idx as isize;
        }
        idx += 1;
    }
    unsafe { write_c_byte(dest, idx, 0) };
    if unsafe { read_c_byte(src, idx) } == 0 {
        idx as isize
    } else {
        -(E2BIG as isize)
    }
}

/// `stpcpy` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_stpcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut idx = 0usize;
    loop {
        let byte = unsafe { read_c_byte(src, idx) };
        unsafe { write_c_byte(dest, idx, byte) };
        if byte == 0 {
            return unsafe { dest.add(idx) };
        }
        idx += 1;
    }
}

/// `strcat` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strcat(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    let dlen = unsafe { linux_strlen(dest.cast_const()) };
    unsafe { linux_strcpy(dest.add(dlen), src) };
    dest
}

/// `strncat` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strncat(
    dest: *mut c_char,
    src: *const c_char,
    count: usize,
) -> *mut c_char {
    let mut dlen = unsafe { linux_strlen(dest.cast_const()) };
    let mut idx = 0usize;
    while idx < count {
        let byte = unsafe { read_c_byte(src, idx) };
        if byte == 0 {
            break;
        }
        unsafe { write_c_byte(dest, dlen, byte) };
        dlen += 1;
        idx += 1;
    }
    unsafe { write_c_byte(dest, dlen, 0) };
    dest
}

/// `strlcat` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strlcat(
    dest: *mut c_char,
    src: *const c_char,
    count: usize,
) -> usize {
    let dsize = unsafe { linux_strlen(dest.cast_const()) };
    let slen = unsafe { linux_strlen(src) };
    assert!(dsize < count, "BUG_ON(dsize >= count)");
    let mut copied = 0usize;
    while dsize + copied + 1 < count {
        let byte = unsafe { read_c_byte(src, copied) };
        if byte == 0 {
            break;
        }
        unsafe { write_c_byte(dest, dsize + copied, byte) };
        copied += 1;
    }
    unsafe { write_c_byte(dest, dsize + copied, 0) };
    dsize + slen
}

/// `strcmp` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
    let mut idx = 0usize;
    loop {
        let c1 = unsafe { read_c_byte(s1, idx) };
        let c2 = unsafe { read_c_byte(s2, idx) };
        if c1 != c2 || c1 == 0 {
            return match c1.cmp(&c2) {
                core::cmp::Ordering::Less => -1,
                core::cmp::Ordering::Equal => 0,
                core::cmp::Ordering::Greater => 1,
            };
        }
        idx += 1;
    }
}

/// `strncmp` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strncmp(
    s1: *const c_char,
    s2: *const c_char,
    count: usize,
) -> c_int {
    let mut idx = 0usize;
    while idx < count {
        let c1 = unsafe { read_c_byte(s1, idx) };
        let c2 = unsafe { read_c_byte(s2, idx) };
        if c1 != c2 || c1 == 0 {
            return match c1.cmp(&c2) {
                core::cmp::Ordering::Less => -1,
                core::cmp::Ordering::Equal => 0,
                core::cmp::Ordering::Greater => 1,
            };
        }
        idx += 1;
    }
    0
}

/// `strchr` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strchr(s: *const c_char, c: c_int) -> *mut c_char {
    let target = c as u8;
    let mut idx = 0usize;
    loop {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == target {
            return unsafe { s.add(idx).cast_mut() };
        }
        if byte == 0 {
            return core::ptr::null_mut();
        }
        idx += 1;
    }
}

/// `strchrnul` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strchrnul(s: *const c_char, c: c_int) -> *mut c_char {
    let target = c as u8;
    let mut idx = 0usize;
    loop {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == target || byte == 0 {
            return unsafe { s.add(idx).cast_mut() };
        }
        idx += 1;
    }
}

/// `strnchrnul` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strnchrnul(s: *const c_char, count: usize, c: c_int) -> *mut c_char {
    let target = c as u8;
    let mut idx = 0usize;
    while idx < count {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == 0 || byte == target {
            return unsafe { s.add(idx).cast_mut() };
        }
        idx += 1;
    }
    unsafe { s.add(idx).cast_mut() }
}

/// `strrchr` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strrchr(s: *const c_char, c: c_int) -> *mut c_char {
    let target = c as u8;
    let mut idx = 0usize;
    let mut last = core::ptr::null_mut();
    loop {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == target {
            last = unsafe { s.add(idx).cast_mut() };
        }
        if byte == 0 {
            return last;
        }
        idx += 1;
    }
}

/// `strnchr` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strnchr(s: *const c_char, count: usize, c: c_int) -> *mut c_char {
    let target = c as u8;
    let mut idx = 0usize;
    while idx < count {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == target {
            return unsafe { s.add(idx).cast_mut() };
        }
        if byte == 0 {
            break;
        }
        idx += 1;
    }
    core::ptr::null_mut()
}

/// `strlen` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strlen(ptr: *const c_char) -> usize {
    unsafe { c_strlen(ptr, usize::MAX / 2) }
}

/// `strnlen` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strnlen(ptr: *const c_char, count: usize) -> usize {
    unsafe { c_strlen(ptr, count) }
}

/// `strspn` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strspn(s: *const c_char, accept: *const c_char) -> usize {
    let mut idx = 0usize;
    loop {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == 0 || unsafe { linux_strchr(accept, byte as c_int) }.is_null() {
            return idx;
        }
        idx += 1;
    }
}

/// `strcspn` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strcspn(s: *const c_char, reject: *const c_char) -> usize {
    let mut idx = 0usize;
    loop {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == 0 || !unsafe { linux_strchr(reject, byte as c_int) }.is_null() {
            return idx;
        }
        idx += 1;
    }
}

/// `strpbrk` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strpbrk(s: *const c_char, accept: *const c_char) -> *mut c_char {
    let mut idx = 0usize;
    loop {
        let byte = unsafe { read_c_byte(s, idx) };
        if byte == 0 {
            return core::ptr::null_mut();
        }
        if !unsafe { linux_strchr(accept, byte as c_int) }.is_null() {
            return unsafe { s.add(idx).cast_mut() };
        }
        idx += 1;
    }
}

/// `strsep` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strsep(s: *mut *mut c_char, ct: *const c_char) -> *mut c_char {
    if s.is_null() {
        return core::ptr::null_mut();
    }
    let sbegin = unsafe { *s };
    if sbegin.is_null() {
        return core::ptr::null_mut();
    }
    let mut scan = sbegin;
    loop {
        let byte = unsafe { *scan.cast::<u8>() };
        if byte == 0 {
            unsafe { *s = core::ptr::null_mut() };
            return sbegin;
        }
        if !unsafe { linux_strchr(ct, byte as c_int) }.is_null() {
            unsafe {
                *scan.cast::<u8>() = 0;
                *s = scan.add(1);
            }
            return sbegin;
        }
        scan = unsafe { scan.add(1) };
    }
}

/// `memset` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memset(s: *mut c_void, c: c_int, count: usize) -> *mut c_void {
    unsafe { core::ptr::write_bytes(s.cast::<u8>(), c as u8, count) };
    s
}

/// `memset16` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memset16(s: *mut u16, v: u16, count: usize) -> *mut u16 {
    for idx in 0..count {
        unsafe { *s.add(idx) = v };
    }
    s
}

/// `memset32` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memset32(s: *mut u32, v: u32, count: usize) -> *mut u32 {
    for idx in 0..count {
        unsafe { *s.add(idx) = v };
    }
    s
}

/// `memset64` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memset64(s: *mut u64, v: u64, count: usize) -> *mut u64 {
    for idx in 0..count {
        unsafe { *s.add(idx) = v };
    }
    s
}

/// `memcpy` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memcpy(
    dst: *mut c_void,
    src: *const c_void,
    count: usize,
) -> *mut c_void {
    unsafe { core::ptr::copy_nonoverlapping(src.cast::<u8>(), dst.cast::<u8>(), count) };
    dst
}

/// `memmove` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memmove(
    dst: *mut c_void,
    src: *const c_void,
    count: usize,
) -> *mut c_void {
    unsafe { core::ptr::copy(src.cast::<u8>(), dst.cast::<u8>(), count) };
    dst
}

/// `memcmp` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memcmp(a: *const c_void, b: *const c_void, count: usize) -> c_int {
    for idx in 0..count {
        let av = unsafe { *a.cast::<u8>().add(idx) };
        let bv = unsafe { *b.cast::<u8>().add(idx) };
        if av != bv {
            return av as c_int - bv as c_int;
        }
    }
    0
}

/// `bcmp` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_bcmp(a: *const c_void, b: *const c_void, count: usize) -> c_int {
    unsafe { linux_memcmp(a, b, count) }
}

/// `memscan` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memscan(addr: *mut c_void, c: c_int, size: usize) -> *mut c_void {
    let target = c as u8;
    let ptr = addr.cast::<u8>();
    for idx in 0..size {
        if unsafe { *ptr.add(idx) } == target {
            return unsafe { ptr.add(idx).cast::<c_void>() };
        }
    }
    unsafe { ptr.add(size).cast::<c_void>() }
}

/// `strstr` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strstr(s1: *const c_char, s2: *const c_char) -> *mut c_char {
    let needle_len = unsafe { linux_strlen(s2) };
    if needle_len == 0 {
        return s1.cast_mut();
    }
    let mut idx = 0usize;
    while unsafe { read_c_byte(s1, idx) } != 0 {
        if unsafe { linux_strncmp(s1.add(idx), s2, needle_len) } == 0 {
            return unsafe { s1.add(idx).cast_mut() };
        }
        idx += 1;
    }
    core::ptr::null_mut()
}

/// `strnstr` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_strnstr(
    s1: *const c_char,
    s2: *const c_char,
    len: usize,
) -> *mut c_char {
    let needle_len = unsafe { linux_strlen(s2) };
    if needle_len == 0 {
        return s1.cast_mut();
    }
    if needle_len > len {
        return core::ptr::null_mut();
    }
    let mut idx = 0usize;
    while idx + needle_len <= len {
        if unsafe { linux_memcmp(s1.add(idx).cast(), s2.cast(), needle_len) } == 0 {
            return unsafe { s1.add(idx).cast_mut() };
        }
        idx += 1;
    }
    core::ptr::null_mut()
}

/// `memchr` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memchr(s: *const c_void, c: c_int, count: usize) -> *mut c_void {
    let target = c as u8;
    let ptr = s.cast::<u8>();
    for idx in 0..count {
        if unsafe { *ptr.add(idx) } == target {
            return unsafe { ptr.add(idx).cast_mut().cast::<c_void>() };
        }
    }
    core::ptr::null_mut()
}

/// `memchr_inv` - `vendor/linux/lib/string.c`.
pub unsafe extern "C" fn linux_memchr_inv(s: *const c_void, c: c_int, count: usize) -> *mut c_void {
    let target = c as u8;
    let ptr = s.cast::<u8>();
    for idx in 0..count {
        if unsafe { *ptr.add(idx) } != target {
            return unsafe { ptr.add(idx).cast_mut().cast::<c_void>() };
        }
    }
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_string_exports_match_linux_string_c() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/string.c"
        ));
        for name in EXPORTED_STRING_SYMBOLS {
            let export = alloc::format!("EXPORT_SYMBOL({name});");
            assert!(source.contains(&export), "missing Linux export {name}");
            register_module_exports();
            assert_eq!(crate::kernel::module::find_symbol(name), symbol_addr(name));
        }
        assert_eq!(EXPORTED_STRING_SYMBOLS.len(), 34);
    }

    #[test]
    fn memory_helpers_follow_linux_contracts() {
        unsafe {
            let mut bytes = *b"abcde";
            let ptr = bytes.as_mut_ptr();
            linux_memmove(ptr.add(1).cast(), ptr.cast(), 4);
            assert_eq!(&bytes, b"aabcd");

            let mut dst = [0u8; 5];
            linux_memcpy(dst.as_mut_ptr().cast(), b"test".as_ptr().cast(), 4);
            assert_eq!(&dst, b"test\0");

            linux_memset(dst.as_mut_ptr().cast(), b'x' as c_int, 2);
            assert_eq!(&dst[..4], b"xxst");
        }
    }

    #[test]
    fn typed_memset_and_memory_search_follow_linux_contracts() {
        unsafe {
            let mut words16 = [0u16; 3];
            let mut words32 = [0u32; 2];
            let mut words64 = [0u64; 2];
            linux_memset16(words16.as_mut_ptr(), 0x1234, words16.len());
            linux_memset32(words32.as_mut_ptr(), 0x89ab_cdef, words32.len());
            linux_memset64(words64.as_mut_ptr(), 0x0123_4567_89ab_cdef, words64.len());
            assert_eq!(words16, [0x1234; 3]);
            assert_eq!(words32, [0x89ab_cdef; 2]);
            assert_eq!(words64, [0x0123_4567_89ab_cdef; 2]);

            let bytes = [0xaa_u8, 0xaa, 0xbb, 0xaa];
            assert_eq!(
                linux_memcmp(bytes.as_ptr().cast(), [0xaa_u8, 0xaa].as_ptr().cast(), 2),
                0
            );
            let different = [0xaa_u8, 0xab];
            assert_ne!(
                linux_bcmp(bytes.as_ptr().cast(), different.as_ptr().cast(), 2),
                0
            );
            assert_eq!(
                linux_memscan(bytes.as_ptr().cast_mut().cast(), 0xbb, bytes.len()),
                unsafe { bytes.as_ptr().add(2).cast_mut().cast() }
            );
            assert_eq!(
                linux_memscan(bytes.as_ptr().cast_mut().cast(), 0xcc, bytes.len()),
                unsafe { bytes.as_ptr().add(bytes.len()).cast_mut().cast() }
            );
            assert_eq!(
                linux_memchr(bytes.as_ptr().cast(), 0xbb, bytes.len()),
                unsafe { bytes.as_ptr().add(2).cast_mut().cast() }
            );
            assert!(linux_memchr(bytes.as_ptr().cast(), 0xcc, bytes.len()).is_null());
            assert_eq!(
                linux_memchr_inv(bytes.as_ptr().cast(), 0xaa, bytes.len()),
                unsafe { bytes.as_ptr().add(2).cast_mut().cast() }
            );
            assert!(linux_memchr_inv([0u8; 4].as_ptr().cast(), 0, 4).is_null());
        }
    }

    #[test]
    fn c_string_copy_and_concat_follow_linux_contracts() {
        unsafe {
            let mut buf = [0u8; 16];
            linux_strcpy(buf.as_mut_ptr().cast(), b"abc\0".as_ptr().cast());
            assert_eq!(&buf[..4], b"abc\0");

            let end = linux_stpcpy(buf.as_mut_ptr().cast(), b"xy\0".as_ptr().cast());
            assert_eq!(end, buf.as_mut_ptr().add(2).cast());
            assert_eq!(&buf[..3], b"xy\0");

            buf.fill(0xff);
            linux_strncpy(buf.as_mut_ptr().cast(), b"z\0".as_ptr().cast(), 4);
            assert_eq!(&buf[..4], b"z\0\0\0");

            let mut cat = *b"ab\0\0\0\0\0\0";
            linux_strcat(cat.as_mut_ptr().cast(), b"cd\0".as_ptr().cast());
            assert_eq!(&cat[..5], b"abcd\0");
            linux_strncat(cat.as_mut_ptr().cast(), b"efgh\0".as_ptr().cast(), 2);
            assert_eq!(&cat[..7], b"abcdef\0");

            let mut limited = *b"ab\0\0\0";
            assert_eq!(
                linux_strlcat(limited.as_mut_ptr().cast(), b"cdef\0".as_ptr().cast(), 5),
                6
            );
            assert_eq!(&limited, b"abcd\0");
        }
    }

    #[test]
    fn string_compare_and_length_helpers_follow_linux_contracts() {
        unsafe {
            assert_eq!(linux_strlen(b"virtio\0".as_ptr().cast()), 6);
            assert_eq!(linux_strnlen(b"abcdef\0".as_ptr().cast(), 3), 3);
            assert_eq!(
                linux_strcmp(b"abc\0".as_ptr().cast(), b"abd\0".as_ptr().cast()),
                -1
            );
            assert_eq!(
                linux_strcmp(b"z\0".as_ptr().cast(), b"a\0".as_ptr().cast()),
                1
            );
            assert_eq!(
                linux_strncmp(b"abc\0".as_ptr().cast(), b"abd\0".as_ptr().cast(), 2),
                0
            );
            assert_eq!(
                linux_strncmp(b"abz\0".as_ptr().cast(), b"aba\0".as_ptr().cast(), 3),
                1
            );
            assert_eq!(
                linux_strcasecmp(b"AbC\0".as_ptr().cast(), b"aBc\0".as_ptr().cast()),
                0
            );
            assert_eq!(
                linux_strncasecmp(b"AbD\0".as_ptr().cast(), b"aBc\0".as_ptr().cast(), 2),
                0
            );
        }
    }

    #[test]
    fn string_search_and_token_helpers_follow_linux_contracts() {
        unsafe {
            let s = b"alpha,beta\0";
            assert_eq!(
                linux_strchr(s.as_ptr().cast(), b',' as c_int),
                s.as_ptr().add(5).cast_mut().cast()
            );
            assert!(linux_strchr(s.as_ptr().cast(), b'!' as c_int).is_null());
            assert_eq!(
                linux_strchrnul(s.as_ptr().cast(), b'!' as c_int),
                s.as_ptr().add(10).cast_mut().cast()
            );
            assert_eq!(
                linux_strnchrnul(s.as_ptr().cast(), 4, b',' as c_int),
                s.as_ptr().add(4).cast_mut().cast()
            );
            assert_eq!(
                linux_strrchr(s.as_ptr().cast(), b'a' as c_int),
                s.as_ptr().add(9).cast_mut().cast()
            );
            assert_eq!(
                linux_strnchr(s.as_ptr().cast(), 5, b'a' as c_int),
                s.as_ptr().cast_mut().cast()
            );
            assert_eq!(
                linux_strspn(b"abc123\0".as_ptr().cast(), b"abc\0".as_ptr().cast()),
                3
            );
            assert_eq!(
                linux_strcspn(b"abc123\0".as_ptr().cast(), b"123\0".as_ptr().cast()),
                3
            );
            assert_eq!(
                linux_strpbrk(s.as_ptr().cast(), b",!\0".as_ptr().cast()),
                s.as_ptr().add(5).cast_mut().cast()
            );
            assert_eq!(
                linux_strstr(s.as_ptr().cast(), b"beta\0".as_ptr().cast()),
                s.as_ptr().add(6).cast_mut().cast()
            );
            assert!(linux_strnstr(s.as_ptr().cast(), b"beta\0".as_ptr().cast(), 6).is_null());
            assert_eq!(
                linux_strnstr(s.as_ptr().cast(), b"beta\0".as_ptr().cast(), 10),
                s.as_ptr().add(6).cast_mut().cast()
            );
            let bounded = b"a\0beta\0";
            assert_eq!(
                linux_strnstr(bounded.as_ptr().cast(), b"beta\0".as_ptr().cast(), 6),
                bounded.as_ptr().add(2).cast_mut().cast()
            );

            let mut mutable = *b"left,right\0";
            let mut cursor = mutable.as_mut_ptr().cast::<c_char>();
            let first = linux_strsep(&mut cursor, b",\0".as_ptr().cast());
            assert_eq!(first, mutable.as_mut_ptr().cast());
            assert_eq!(cursor, mutable.as_mut_ptr().add(5).cast());
            assert_eq!(&mutable[..5], b"left\0");
            let second = linux_strsep(&mut cursor, b",\0".as_ptr().cast());
            assert_eq!(second, mutable.as_mut_ptr().add(5).cast());
            assert!(cursor.is_null());
        }
    }

    #[test]
    fn sized_strscpy_returns_length_or_e2big() {
        unsafe {
            let mut dst = [0u8; 8];
            assert_eq!(
                linux_sized_strscpy(dst.as_mut_ptr().cast(), b"abc\0".as_ptr().cast(), dst.len()),
                3
            );
            assert_eq!(&dst[..4], b"abc\0");

            let mut small = [0u8; 3];
            assert_eq!(
                linux_sized_strscpy(
                    small.as_mut_ptr().cast(),
                    b"abcdef\0".as_ptr().cast(),
                    small.len()
                ),
                -(E2BIG as isize)
            );
            assert_eq!(&small, b"ab\0");
            assert_eq!(
                linux_sized_strscpy(small.as_mut_ptr().cast(), b"x\0".as_ptr().cast(), 0),
                -(E2BIG as isize)
            );
        }
    }
}
