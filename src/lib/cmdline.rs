//! linux-parity: complete
//! linux-source: vendor/linux/lib/cmdline.c
//! test-origin: linux:vendor/linux/lib/cmdline.c
//! Kernel command-line parsing helpers.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("get_option", get_option as usize, false);
    export_symbol_once("get_options", get_options as usize, false);
    export_symbol_once("memparse", memparse as usize, false);
    export_symbol_once("next_arg", next_arg as usize, false);
}

fn digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some((byte - b'0') as u32),
        b'a'..=b'f' => Some((byte - b'a' + 10) as u32),
        b'A'..=b'F' => Some((byte - b'A' + 10) as u32),
        _ => None,
    }
}

unsafe fn parse_unsigned(mut ptr: *mut u8, mut base: u32) -> (u64, *mut u8) {
    let start = ptr;
    if unsafe { *ptr } == b'+' {
        ptr = unsafe { ptr.add(1) };
    }
    if base == 0 {
        if unsafe { *ptr } == b'0' {
            if matches!(unsafe { *ptr.add(1) }, b'x' | b'X')
                && digit_value(unsafe { *ptr.add(2) }).is_some_and(|v| v < 16)
            {
                base = 16;
                ptr = unsafe { ptr.add(2) };
            } else {
                base = 8;
            }
        } else {
            base = 10;
        }
    }
    let digit_start = ptr;
    let mut value = 0u64;
    loop {
        let Some(digit) = digit_value(unsafe { *ptr }) else {
            break;
        };
        if digit >= base {
            break;
        }
        value = value.wrapping_mul(base as u64).wrapping_add(digit as u64);
        ptr = unsafe { ptr.add(1) };
    }
    if ptr == digit_start {
        (0, start)
    } else {
        (value, ptr)
    }
}

unsafe fn parse_signed(ptr: *mut u8, base: u32) -> (i64, *mut u8) {
    if unsafe { *ptr } == b'-' {
        let (value, end) = unsafe { parse_unsigned(ptr.add(1), base) };
        if end == unsafe { ptr.add(1) } {
            (0, end)
        } else {
            (-(value as i64), end)
        }
    } else {
        let (value, end) = unsafe { parse_unsigned(ptr, base) };
        (value as i64, end)
    }
}

unsafe fn skip_spaces(mut ptr: *mut u8) -> *mut u8 {
    while matches!(unsafe { *ptr }, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        ptr = unsafe { ptr.add(1) };
    }
    ptr
}

unsafe fn get_range(strp: *mut *mut u8, pint: *mut i32, mut n: i32) -> i32 {
    unsafe { *strp = (*strp).add(1) };
    let (upper, _) = unsafe { parse_signed(*strp, 0) };
    let upper = upper as i32;
    let start = unsafe { *pint };
    let inc_counter = upper - start;
    let mut x = start;
    let mut out = pint;
    while n != 0 && x < upper {
        unsafe { *out = x };
        out = unsafe { out.add(1) };
        x += 1;
        n -= 1;
    }
    inc_counter
}

pub unsafe extern "C" fn get_option(strp: *mut *mut u8, pint: *mut i32) -> i32 {
    if strp.is_null() {
        return 0;
    }
    let mut cur = unsafe { *strp };
    if cur.is_null() || unsafe { *cur } == 0 {
        return 0;
    }

    let value;
    if unsafe { *cur } == b'-' {
        cur = unsafe { cur.add(1) };
        let (parsed, end) = unsafe { parse_unsigned(cur, 0) };
        value = -(parsed as i32);
        unsafe { *strp = end };
    } else {
        let (parsed, end) = unsafe { parse_unsigned(cur, 0) };
        value = parsed as i32;
        unsafe { *strp = end };
    }
    if !pint.is_null() {
        unsafe { *pint = value };
    }
    if cur == unsafe { *strp } {
        return 0;
    }
    match unsafe { **strp } {
        b',' => {
            unsafe { *strp = (*strp).add(1) };
            2
        }
        b'-' => 3,
        _ => 1,
    }
}

pub unsafe extern "C" fn get_options(str: *const u8, nints: i32, ints: *mut i32) -> *mut u8 {
    if str.is_null() || ints.is_null() {
        return str as *mut u8;
    }
    let validate = nints == 0;
    let mut cursor = str as *mut u8;
    let mut i = 1i32;
    while i < nints || validate {
        let pint = if validate {
            ints
        } else {
            unsafe { ints.add(i as usize) }
        };
        let res = unsafe { get_option(&mut cursor, pint) };
        if res == 0 {
            break;
        }
        if res == 3 {
            let n = if validate { 0 } else { nints - i };
            let range_nums = unsafe { get_range(&mut cursor, pint, n) };
            if range_nums < 0 {
                break;
            }
            i += range_nums - 1;
        }
        i += 1;
        if res == 1 {
            break;
        }
    }
    unsafe { *ints = i - 1 };
    cursor
}

pub unsafe extern "C" fn memparse(ptr: *const u8, retptr: *mut *mut u8) -> u64 {
    if ptr.is_null() {
        if !retptr.is_null() {
            unsafe { *retptr = core::ptr::null_mut() };
        }
        return 0;
    }
    let (mut ret, mut endptr) = unsafe { parse_unsigned(ptr as *mut u8, 0) };
    match unsafe { *endptr } {
        b'E' | b'e' => {
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            endptr = unsafe { endptr.add(1) };
        }
        b'P' | b'p' => {
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            endptr = unsafe { endptr.add(1) };
        }
        b'T' | b't' => {
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            endptr = unsafe { endptr.add(1) };
        }
        b'G' | b'g' => {
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            endptr = unsafe { endptr.add(1) };
        }
        b'M' | b'm' => {
            ret = ret.wrapping_shl(10);
            ret = ret.wrapping_shl(10);
            endptr = unsafe { endptr.add(1) };
        }
        b'K' | b'k' => {
            ret = ret.wrapping_shl(10);
            endptr = unsafe { endptr.add(1) };
        }
        _ => {}
    }
    if !retptr.is_null() {
        unsafe { *retptr = endptr };
    }
    ret
}

unsafe fn c_strlen(mut ptr: *const u8) -> usize {
    let mut len = 0usize;
    while unsafe { *ptr } != 0 {
        len += 1;
        ptr = unsafe { ptr.add(1) };
    }
    len
}

pub unsafe extern "C" fn parse_option_str(strp: *const u8, option: *const u8) -> bool {
    if strp.is_null() || option.is_null() {
        return false;
    }
    let option_len = unsafe { c_strlen(option) };
    let mut strp = strp;
    while unsafe { *strp } != 0 {
        let current = unsafe { core::slice::from_raw_parts(strp, option_len) };
        let wanted = unsafe { core::slice::from_raw_parts(option, option_len) };
        if current == wanted {
            strp = unsafe { strp.add(option_len) };
            if unsafe { *strp } == 0 || unsafe { *strp } == b',' {
                return true;
            }
        }
        while unsafe { *strp } != 0 && unsafe { *strp } != b',' {
            strp = unsafe { strp.add(1) };
        }
        if unsafe { *strp } == b',' {
            strp = unsafe { strp.add(1) };
        }
    }
    false
}

pub unsafe extern "C" fn next_arg(
    mut args: *mut u8,
    param: *mut *mut u8,
    val: *mut *mut u8,
) -> *mut u8 {
    if args.is_null() || param.is_null() || val.is_null() {
        return args;
    }
    let mut equals = 0usize;
    let mut in_quote = false;
    let mut quoted = false;
    if unsafe { *args } == b'"' {
        args = unsafe { args.add(1) };
        in_quote = true;
        quoted = true;
    }

    let mut i = 0usize;
    loop {
        let ch = unsafe { *args.add(i) };
        if ch == 0 {
            break;
        }
        if matches!(ch, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) && !in_quote {
            break;
        }
        if equals == 0 && ch == b'=' {
            equals = i;
        }
        if ch == b'"' {
            in_quote = !in_quote;
        }
        i += 1;
    }

    unsafe { *param = args };
    if equals == 0 {
        unsafe { *val = core::ptr::null_mut() };
    } else {
        unsafe { *args.add(equals) = 0 };
        unsafe { *val = args.add(equals + 1) };
        if unsafe { **val } == b'"' {
            unsafe { *val = (*val).add(1) };
            if i > 0 && unsafe { *args.add(i - 1) } == b'"' {
                unsafe { *args.add(i - 1) = 0 };
            }
        }
    }
    if quoted && i > 0 && unsafe { *args.add(i - 1) } == b'"' {
        unsafe { *args.add(i - 1) = 0 };
    }
    if unsafe { *args.add(i) } != 0 {
        unsafe { *args.add(i) = 0 };
        args = unsafe { args.add(i + 1) };
    } else {
        args = unsafe { args.add(i) };
    }
    unsafe { skip_spaces(args) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_cmdline_kunit_get_option_cases_pass() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/cmdline.c"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/cmdline_kunit.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL(get_option);"));
        assert!(kunit.contains("cmdline_test_strings"));

        let strings = [
            "\"\"", "", "=", "\"-", ",", "-,", ",-", "-", "+,", "--", ",,", "''", "\"\",", "\",\"",
            "-\"\"", "\"",
        ];
        let lead_values = [1, 1, 1, 1, 2, 3, 2, 3, 1, 3, 2, 1, 1, 1, 3, 1];
        for (suffix, expected) in strings.iter().zip(lead_values) {
            let mut input = [0u8; 64];
            let prefix = b"7";
            input[..prefix.len()].copy_from_slice(prefix);
            input[prefix.len()..prefix.len() + suffix.len()].copy_from_slice(suffix.as_bytes());
            let mut ptr = input.as_mut_ptr();
            let mut value = 0;
            let ret = unsafe { get_option(&mut ptr, &mut value) };
            assert_eq!(ret, expected, "suffix {suffix}");
        }
    }

    #[test]
    fn linux_cmdline_kunit_range_cases_pass() {
        let cases: [(&[u8], [i32; 16]); 20] = [
            (b"-7\0", [1, -7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"--7\0", [0; 16]),
            (b"-1-2\0", [4, -1, 0, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"7--9\0", [0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"7-\0", [0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (
                b"-7--9\0",
                [0, -7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (b"7-9,\0", [3, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"9-7\0", [0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"5-a\0", [0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"a-5\0", [0; 16]),
            (b"5-8\0", [4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b",8-5\0", [0; 16]),
            (b"+,1\0", [0; 16]),
            (b"-,4\0", [0; 16]),
            (
                b"-3,0-1,6\0",
                [4, -3, 0, 1, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (b"4,-\0", [1, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b" +2\0", [0; 16]),
            (b" -9\0", [0; 16]),
            (
                b"0-1,-3,6\0",
                [4, 0, 1, -3, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (b"- 9\0", [0; 16]),
        ];
        for (input, expected) in cases {
            let mut out = [0i32; 16];
            unsafe { get_options(input.as_ptr(), out.len() as i32, out.as_mut_ptr()) };
            assert_eq!(out, expected, "input {:?}", input);
            out = [0; 16];
            unsafe { get_options(input.as_ptr(), 0, out.as_mut_ptr()) };
            assert_eq!(out[0], expected[0]);
            assert!(out[1..].iter().all(|value| *value == 0));
        }
    }

    #[test]
    fn linux_cmdline_memparse_parse_option_and_next_arg_work() {
        let mut ret = core::ptr::null_mut();
        let value = unsafe { memparse(c"2M".as_ptr() as *const u8, &mut ret) };
        assert_eq!(value, 2 * 1024 * 1024);
        assert_eq!(unsafe { *ret }, 0);

        assert!(unsafe {
            parse_option_str(
                c"root=/dev/vda,quiet,foo=bar".as_ptr() as *const u8,
                c"quiet".as_ptr() as *const u8,
            )
        });
        assert!(!unsafe {
            parse_option_str(
                c"root=/dev/vda,quietness".as_ptr() as *const u8,
                c"quiet".as_ptr() as *const u8,
            )
        });

        let mut input = *b"foo=\"bar baz\" qux=1\0";
        let mut param = core::ptr::null_mut();
        let mut val = core::ptr::null_mut();
        let next = unsafe { next_arg(input.as_mut_ptr(), &mut param, &mut val) };
        let param_len = unsafe { c_strlen(param) };
        let val_len = unsafe { c_strlen(val) };
        assert_eq!(
            unsafe { core::slice::from_raw_parts(param, param_len) },
            b"foo"
        );
        assert_eq!(
            unsafe { core::slice::from_raw_parts(val, val_len) },
            b"bar baz"
        );
        assert_eq!(unsafe { *next }, b'q');

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("memparse"),
            Some(memparse as usize)
        );
    }
}
