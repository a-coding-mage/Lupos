//! linux-parity: partial
//! linux-source: vendor/linux/lib/kstrtox.c
//! test-origin: linux:vendor/linux/lib/kstrtox.c
//! Integer string conversion helpers exported to Linux-built modules.

use core::ffi::c_char;

use crate::include::uapi::errno::{EFAULT, EINVAL, ERANGE};
use crate::kernel::module::{export_symbol, find_symbol};

const KSTRTOINT_FROM_USER_BUF: usize = 1 + core::mem::size_of::<i32>() * 8 + 1 + 1;
const KSTRTOUINT_FROM_USER_BUF: usize = 1 + core::mem::size_of::<u32>() * 8 + 1 + 1;
const KSTRTOULONG_FROM_USER_BUF: usize = 1 + core::mem::size_of::<usize>() * 8 + 1 + 1;
const KSTRTOBOOL_FROM_USER_BUF: usize = 4;
const KSTRTOX_CSTR_LIMIT: usize = 128;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("kstrtouint", linux_kstrtouint as usize, false);
    export_symbol_once(
        "kstrtouint_from_user",
        linux_kstrtouint_from_user as usize,
        false,
    );
    export_symbol_once(
        "kstrtoul_from_user",
        linux_kstrtoul_from_user as usize,
        false,
    );
    export_symbol_once("kstrtoint", linux_kstrtoint as usize, false);
    export_symbol_once(
        "kstrtoint_from_user",
        linux_kstrtoint_from_user as usize,
        false,
    );
    export_symbol_once(
        "kstrtobool_from_user",
        linux_kstrtobool_from_user as usize,
        false,
    );
}

const fn to_lower_ascii(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' {
        byte + 32
    } else {
        byte
    }
}

const fn digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some((byte - b'0') as u32),
        b'a'..=b'f' => Some((byte - b'a') as u32 + 10),
        b'A'..=b'F' => Some((byte - b'A') as u32 + 10),
        _ => None,
    }
}

fn has_hex_digit_after_prefix(bytes: &[u8]) -> bool {
    bytes.len() >= 3
        && bytes[0] == b'0'
        && to_lower_ascii(bytes[1]) == b'x'
        && digit_value(bytes[2]).is_some()
}

fn parse_i32_bytes(mut bytes: &[u8], mut base: u32) -> Result<i32, i32> {
    if bytes.is_empty() || (base != 0 && !(2..=16).contains(&base)) {
        return Err(-EINVAL);
    }

    let negative = match bytes[0] {
        b'-' => {
            bytes = &bytes[1..];
            true
        }
        b'+' => {
            bytes = &bytes[1..];
            false
        }
        _ => false,
    };
    if bytes.is_empty() {
        return Err(-EINVAL);
    }

    if base == 0 {
        base = if has_hex_digit_after_prefix(bytes) {
            16
        } else if bytes[0] == b'0' {
            8
        } else {
            10
        };
    }
    if base == 16 && bytes.len() >= 2 && bytes[0] == b'0' && to_lower_ascii(bytes[1]) == b'x' {
        bytes = &bytes[2..];
    }

    let limit = if negative {
        i32::MAX as u64 + 1
    } else {
        i32::MAX as u64
    };
    let mut value = 0u64;
    let mut digits = 0usize;
    let mut idx = 0usize;

    while idx < bytes.len() {
        let digit = match digit_value(bytes[idx]) {
            Some(digit) if digit < base => digit as u64,
            _ => break,
        };
        if value > (limit - digit) / base as u64 {
            return Err(-ERANGE);
        }
        value = value * base as u64 + digit;
        digits += 1;
        idx += 1;
    }

    if digits == 0 {
        return Err(-EINVAL);
    }
    if idx < bytes.len() && bytes[idx] == b'\n' {
        idx += 1;
    }
    if idx != bytes.len() {
        return Err(-EINVAL);
    }

    if negative {
        if value == i32::MAX as u64 + 1 {
            Ok(i32::MIN)
        } else {
            Ok(-(value as i32))
        }
    } else {
        Ok(value as i32)
    }
}

fn parse_u64_bytes_limited(mut bytes: &[u8], mut base: u32, limit: u64) -> Result<u64, i32> {
    if bytes.is_empty() || (base != 0 && !(2..=16).contains(&base)) {
        return Err(-EINVAL);
    }

    match bytes[0] {
        b'+' => bytes = &bytes[1..],
        b'-' => return Err(-EINVAL),
        _ => {}
    }
    if bytes.is_empty() {
        return Err(-EINVAL);
    }

    if base == 0 {
        base = if has_hex_digit_after_prefix(bytes) {
            16
        } else if bytes[0] == b'0' {
            8
        } else {
            10
        };
    }
    if base == 16 && bytes.len() >= 2 && bytes[0] == b'0' && to_lower_ascii(bytes[1]) == b'x' {
        bytes = &bytes[2..];
    }

    let mut value = 0u64;
    let mut digits = 0usize;
    let mut idx = 0usize;
    while idx < bytes.len() {
        let digit = match digit_value(bytes[idx]) {
            Some(digit) if digit < base => digit as u64,
            _ => break,
        };
        if value > (limit - digit) / base as u64 {
            return Err(-ERANGE);
        }
        value = value * base as u64 + digit;
        digits += 1;
        idx += 1;
    }

    if digits == 0 {
        return Err(-EINVAL);
    }
    if idx < bytes.len() && bytes[idx] == b'\n' {
        idx += 1;
    }
    if idx != bytes.len() {
        return Err(-EINVAL);
    }

    Ok(value)
}

fn parse_u32_bytes(bytes: &[u8], base: u32) -> Result<u32, i32> {
    parse_u64_bytes_limited(bytes, base, u32::MAX as u64).map(|value| value as u32)
}

fn parse_bool_bytes(bytes: &[u8]) -> Result<bool, i32> {
    let Some(first) = bytes.first().copied() else {
        return Err(-EINVAL);
    };

    match first {
        b'e' | b'E' | b'y' | b'Y' | b't' | b'T' | b'1' => Ok(true),
        b'd' | b'D' | b'n' | b'N' | b'f' | b'F' | b'0' => Ok(false),
        b'o' | b'O' => match bytes.get(1).copied() {
            Some(b'n' | b'N') => Ok(true),
            Some(b'f' | b'F') => Ok(false),
            _ => Err(-EINVAL),
        },
        _ => Err(-EINVAL),
    }
}

unsafe fn c_str_bytes<'a>(s: *const c_char, max_len: usize) -> Option<&'a [u8]> {
    if s.is_null() {
        return None;
    }
    let mut len = 0usize;
    while len < max_len {
        if unsafe { *s.add(len) } == 0 {
            return Some(unsafe { core::slice::from_raw_parts(s.cast::<u8>(), len) });
        }
        len += 1;
    }
    None
}

/// `kstrtouint` - `vendor/linux/lib/kstrtox.c:235`.
pub unsafe extern "C" fn linux_kstrtouint(s: *const c_char, base: u32, res: *mut u32) -> i32 {
    if res.is_null() {
        return -EINVAL;
    }
    let bytes = match unsafe { c_str_bytes(s, KSTRTOX_CSTR_LIMIT) } {
        Some(bytes) => bytes,
        None => return -EINVAL,
    };
    match parse_u32_bytes(bytes, base) {
        Ok(value) => {
            unsafe { res.write(value) };
            0
        }
        Err(err) => err,
    }
}

/// `kstrtoint` - `vendor/linux/lib/kstrtox.c:266`.
pub unsafe extern "C" fn linux_kstrtoint(s: *const c_char, base: u32, res: *mut i32) -> i32 {
    if res.is_null() {
        return -EINVAL;
    }
    let bytes = match unsafe { c_str_bytes(s, KSTRTOX_CSTR_LIMIT) } {
        Some(bytes) => bytes,
        None => return -EINVAL,
    };
    match parse_i32_bytes(bytes, base) {
        Ok(value) => {
            unsafe { res.write(value) };
            0
        }
        Err(err) => err,
    }
}

/// `kstrtouint_from_user` - `vendor/linux/lib/kstrtox.c:437`.
pub unsafe extern "C" fn linux_kstrtouint_from_user(
    s: *const c_char,
    count: usize,
    base: u32,
    res: *mut u32,
) -> i32 {
    if res.is_null() {
        return -EINVAL;
    }
    let copy_len = core::cmp::min(count, KSTRTOUINT_FROM_USER_BUF - 1);
    let mut buf = [0u8; KSTRTOUINT_FROM_USER_BUF];
    let not_copied =
        unsafe { crate::lib::usercopy::_copy_from_user(buf.as_mut_ptr(), s.cast(), copy_len) };
    if not_copied != 0 {
        return -EFAULT;
    }
    match parse_u32_bytes(&buf[..copy_len], base) {
        Ok(value) => {
            unsafe { res.write(value) };
            0
        }
        Err(err) => err,
    }
}

/// `kstrtoul_from_user` - `vendor/linux/lib/kstrtox.c:435`.
pub unsafe extern "C" fn linux_kstrtoul_from_user(
    s: *const c_char,
    count: usize,
    base: u32,
    res: *mut usize,
) -> i32 {
    if res.is_null() {
        return -EINVAL;
    }
    let copy_len = core::cmp::min(count, KSTRTOULONG_FROM_USER_BUF - 1);
    let mut buf = [0u8; KSTRTOULONG_FROM_USER_BUF];
    let not_copied =
        unsafe { crate::lib::usercopy::_copy_from_user(buf.as_mut_ptr(), s.cast(), copy_len) };
    if not_copied != 0 {
        return -EFAULT;
    }
    match parse_u64_bytes_limited(&buf[..copy_len], base, usize::MAX as u64) {
        Ok(value) => {
            unsafe { res.write(value as usize) };
            0
        }
        Err(err) => err,
    }
}

/// `kstrtoint_from_user` - `vendor/linux/lib/kstrtox.c:438`.
pub unsafe extern "C" fn linux_kstrtoint_from_user(
    s: *const c_char,
    count: usize,
    base: u32,
    res: *mut i32,
) -> i32 {
    if res.is_null() {
        return -EINVAL;
    }
    let copy_len = core::cmp::min(count, KSTRTOINT_FROM_USER_BUF - 1);
    let mut buf = [0u8; KSTRTOINT_FROM_USER_BUF];
    let not_copied =
        unsafe { crate::lib::usercopy::_copy_from_user(buf.as_mut_ptr(), s.cast(), copy_len) };
    if not_copied != 0 {
        return -EFAULT;
    }
    match parse_i32_bytes(&buf[..copy_len], base) {
        Ok(value) => {
            unsafe { res.write(value) };
            0
        }
        Err(err) => err,
    }
}

/// `kstrtobool_from_user` - `vendor/linux/lib/kstrtox.c:406`.
pub unsafe extern "C" fn linux_kstrtobool_from_user(
    s: *const c_char,
    count: usize,
    res: *mut bool,
) -> i32 {
    if res.is_null() {
        return -EINVAL;
    }
    let copy_len = core::cmp::min(count, KSTRTOBOOL_FROM_USER_BUF - 1);
    let mut buf = [0u8; KSTRTOBOOL_FROM_USER_BUF];
    let not_copied =
        unsafe { crate::lib::usercopy::_copy_from_user(buf.as_mut_ptr(), s.cast(), copy_len) };
    if not_copied != 0 {
        return -EFAULT;
    }
    match parse_bool_bytes(&buf[..copy_len]) {
        Ok(value) => {
            unsafe { res.write(value) };
            0
        }
        Err(err) => err,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kstrtoint_exports_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/kstrtox.c"
        ));
        assert!(
            source.contains("int kstrtouint(const char *s, unsigned int base, unsigned int *res)")
        );
        assert!(
            source.contains("kstrto_from_user(kstrtouint_from_user,\tkstrtouint,\tunsigned int);")
        );
        assert!(
            source.contains("kstrto_from_user(kstrtoul_from_user,\tkstrtoul,\tunsigned long);")
        );
        assert!(source.contains("int kstrtoint(const char *s, unsigned int base, int *res)"));
        assert!(source.contains("kstrto_from_user(kstrtoint_from_user,\tkstrtoint,\tint);"));
        assert!(
            source.contains(
                "int kstrtobool_from_user(const char __user *s, size_t count, bool *res)"
            )
        );
        assert!(source.contains("EXPORT_SYMBOL(kstrtoint);"));
        assert!(source.contains("EXPORT_SYMBOL(kstrtouint);"));

        register_module_exports();
        assert_eq!(find_symbol("kstrtouint"), Some(linux_kstrtouint as usize));
        assert_eq!(
            find_symbol("kstrtouint_from_user"),
            Some(linux_kstrtouint_from_user as usize)
        );
        assert_eq!(
            find_symbol("kstrtoul_from_user"),
            Some(linux_kstrtoul_from_user as usize)
        );
        assert_eq!(find_symbol("kstrtoint"), Some(linux_kstrtoint as usize));
        assert_eq!(
            find_symbol("kstrtoint_from_user"),
            Some(linux_kstrtoint_from_user as usize)
        );
        assert_eq!(
            find_symbol("kstrtobool_from_user"),
            Some(linux_kstrtobool_from_user as usize)
        );
    }

    #[test]
    fn kstrtoint_parses_linux_integer_forms() {
        assert_eq!(parse_i32_bytes(b"42\n", 10), Ok(42));
        assert_eq!(parse_i32_bytes(b"+17", 0), Ok(17));
        assert_eq!(parse_i32_bytes(b"-0x80000000", 0), Ok(i32::MIN));
        assert_eq!(parse_i32_bytes(b"0377", 0), Ok(255));
        assert_eq!(parse_i32_bytes(b"2147483648", 10), Err(-ERANGE));
        assert_eq!(parse_i32_bytes(b"12x", 10), Err(-EINVAL));
        assert_eq!(parse_i32_bytes(b"", 10), Err(-EINVAL));
    }

    #[test]
    fn kstrtouint_parses_linux_unsigned_forms() {
        assert_eq!(parse_u32_bytes(b"42\n", 10), Ok(42));
        assert_eq!(parse_u32_bytes(b"+0x2a", 0), Ok(42));
        assert_eq!(parse_u32_bytes(b"0377", 0), Ok(255));
        assert_eq!(parse_u32_bytes(b"-1", 10), Err(-EINVAL));
        assert_eq!(parse_u32_bytes(b"4294967296", 10), Err(-ERANGE));
        assert_eq!(parse_u32_bytes(b"12x", 10), Err(-EINVAL));
    }

    #[test]
    fn exported_kstrtoint_writes_only_on_success() {
        let mut out = 99i32;
        assert_eq!(unsafe { linux_kstrtoint(c"-15".as_ptr(), 10, &mut out) }, 0);
        assert_eq!(out, -15);

        assert_eq!(
            unsafe { linux_kstrtoint(c"9999999999".as_ptr(), 10, &mut out) },
            -ERANGE
        );
        assert_eq!(out, -15);
    }

    #[test]
    fn kstrtoint_from_user_copies_bounded_input() {
        let input = b"123 ignored";
        let mut out = 0i32;

        assert_eq!(
            unsafe { linux_kstrtoint_from_user(input.as_ptr().cast(), 3, 10, &mut out) },
            0
        );
        assert_eq!(out, 123);

        let invalid = (1u64 << 47) as *const c_char;
        assert_eq!(
            unsafe { linux_kstrtoint_from_user(invalid, 4, 10, &mut out) },
            -EFAULT
        );
    }

    #[test]
    fn kstrtouint_from_user_copies_bounded_input() {
        let input = b"0x2a ignored";
        let mut out = 0u32;

        assert_eq!(
            unsafe { linux_kstrtouint_from_user(input.as_ptr().cast(), 4, 0, &mut out) },
            0
        );
        assert_eq!(out, 42);

        assert_eq!(
            unsafe { linux_kstrtouint_from_user(c"-1".as_ptr(), 2, 10, &mut out) },
            -EINVAL
        );

        let invalid = (1u64 << 47) as *const c_char;
        assert_eq!(
            unsafe { linux_kstrtouint_from_user(invalid, 4, 10, &mut out) },
            -EFAULT
        );
    }

    #[test]
    fn kstrtobool_from_user_matches_linux_boolean_forms() {
        let mut out = false;
        assert_eq!(
            unsafe { linux_kstrtobool_from_user(c"on".as_ptr(), 2, &mut out) },
            0
        );
        assert!(out);

        assert_eq!(
            unsafe { linux_kstrtobool_from_user(c"off".as_ptr(), 3, &mut out) },
            0
        );
        assert!(!out);

        assert_eq!(
            unsafe { linux_kstrtobool_from_user(c"disable".as_ptr(), 7, &mut out) },
            0
        );
        assert!(!out);

        assert_eq!(
            unsafe { linux_kstrtobool_from_user(c"maybe".as_ptr(), 5, &mut out) },
            -EINVAL
        );

        let invalid = (1u64 << 47) as *const c_char;
        assert_eq!(
            unsafe { linux_kstrtobool_from_user(invalid, 2, &mut out) },
            -EFAULT
        );
    }
}
