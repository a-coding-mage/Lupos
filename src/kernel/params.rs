//! linux-parity: partial
//! linux-source: vendor/linux/kernel/params.c
//! test-origin: linux:vendor/linux/kernel/params.c
//! Module parameter ABI exports.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::{EINVAL, ERANGE};
use crate::kernel::module::{export_symbol, find_symbol};

const PARAM_GET_BUFFER_SIZE: usize = 4096;

static LINUX_PARAM_OPS_BOOL: usize = 0;
static LINUX_PARAM_OPS_BINT: usize = 0;
static LINUX_PARAM_OPS_INT: usize = 0;
static LINUX_PARAM_OPS_UINT: usize = 0;
static LINUX_PARAM_OPS_ULONG: usize = 0;
static LINUX_PARAM_OPS_ULLONG: usize = 0;
static LINUX_PARAM_OPS_STRING: usize = 0;
static LINUX_PARAM_OPS_CHARP: usize = 0;
static LINUX_PARAM_ARRAY_OPS: usize = 0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "param_ops_bool",
        core::ptr::addr_of!(LINUX_PARAM_OPS_BOOL) as usize,
        true,
    );
    export_symbol_once(
        "param_ops_bint",
        core::ptr::addr_of!(LINUX_PARAM_OPS_BINT) as usize,
        false,
    );
    export_symbol_once(
        "param_ops_int",
        core::ptr::addr_of!(LINUX_PARAM_OPS_INT) as usize,
        true,
    );
    export_symbol_once(
        "param_ops_uint",
        core::ptr::addr_of!(LINUX_PARAM_OPS_UINT) as usize,
        true,
    );
    export_symbol_once(
        "param_ops_ulong",
        core::ptr::addr_of!(LINUX_PARAM_OPS_ULONG) as usize,
        true,
    );
    export_symbol_once(
        "param_ops_ullong",
        core::ptr::addr_of!(LINUX_PARAM_OPS_ULLONG) as usize,
        true,
    );
    export_symbol_once(
        "param_ops_string",
        core::ptr::addr_of!(LINUX_PARAM_OPS_STRING) as usize,
        true,
    );
    export_symbol_once(
        "param_ops_charp",
        core::ptr::addr_of!(LINUX_PARAM_OPS_CHARP) as usize,
        true,
    );
    export_symbol_once(
        "param_array_ops",
        core::ptr::addr_of!(LINUX_PARAM_ARRAY_OPS) as usize,
        true,
    );
    export_symbol_once("param_set_int", linux_param_set_int as usize, false);
    export_symbol_once("param_get_int", linux_param_get_int as usize, false);
}

#[repr(C)]
struct LinuxKernelParam {
    name: *const c_char,
    mod_: *mut c_void,
    ops: *const c_void,
    perm: u16,
    level: i8,
    flags: u8,
    arg: *mut c_void,
}

unsafe fn c_str_bytes<'a>(ptr: *const c_char, max: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    while len < max {
        if unsafe { *ptr.add(len) } == 0 {
            return Some(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) });
        }
        len += 1;
    }
    None
}

fn hex_digit(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some((byte - b'0') as u32),
        b'a'..=b'f' => Some((byte - b'a' + 10) as u32),
        b'A'..=b'F' => Some((byte - b'A' + 10) as u32),
        _ => None,
    }
}

fn parse_i32_base0(mut bytes: &[u8]) -> Result<i32, i32> {
    let mut negative = false;
    match bytes.first().copied() {
        Some(b'-') => {
            negative = true;
            bytes = &bytes[1..];
        }
        Some(b'+') => bytes = &bytes[1..],
        _ => {}
    }
    if bytes.is_empty() {
        return Err(EINVAL);
    }

    let radix = if bytes[0] == b'0' {
        if bytes.len() >= 3 && matches!(bytes[1], b'x' | b'X') && hex_digit(bytes[2]).is_some() {
            bytes = &bytes[2..];
            16u32
        } else {
            8u32
        }
    } else {
        10u32
    };

    let limit = i32::MAX as u64 + u64::from(negative);
    let mut value = 0u64;
    let mut parsed = 0usize;
    while parsed < bytes.len() {
        let Some(digit) = hex_digit(bytes[parsed]) else {
            break;
        };
        if digit >= radix {
            break;
        }
        value = value
            .checked_mul(radix as u64)
            .and_then(|v| v.checked_add(digit as u64))
            .ok_or(ERANGE)?;
        if value > limit {
            return Err(ERANGE);
        }
        parsed += 1;
    }
    if parsed == 0 {
        return Err(EINVAL);
    }

    let tail = &bytes[parsed..];
    if !(tail.is_empty() || tail == b"\n") {
        return Err(EINVAL);
    }

    if negative {
        Ok(-(value as i64) as i32)
    } else {
        Ok(value as i32)
    }
}

fn write_decimal_i32(buf: *mut c_char, value: i32) -> i32 {
    if buf.is_null() {
        return -EINVAL;
    }

    let mut out = [0u8; 16];
    let mut len = 0usize;
    let mut number = if value < 0 {
        out[len] = b'-';
        len += 1;
        -(value as i64) as u64
    } else {
        value as u64
    };

    let mut digits = [0u8; 10];
    let mut digit_len = 0usize;
    loop {
        digits[digit_len] = b'0' + (number % 10) as u8;
        digit_len += 1;
        number /= 10;
        if number == 0 {
            break;
        }
    }
    while digit_len != 0 {
        digit_len -= 1;
        out[len] = digits[digit_len];
        len += 1;
    }
    out[len] = b'\n';
    len += 1;

    let copy_len = len.min(PARAM_GET_BUFFER_SIZE - 1);
    unsafe {
        core::ptr::copy_nonoverlapping(out.as_ptr(), buf.cast::<u8>(), copy_len);
        *buf.add(copy_len) = 0;
    }
    copy_len as i32
}

/// `param_set_int` - `vendor/linux/kernel/params.c`.
#[unsafe(export_name = "param_set_int")]
pub unsafe extern "C" fn linux_param_set_int(
    val: *const c_char,
    kp: *const LinuxKernelParam,
) -> i32 {
    if kp.is_null() {
        return -EINVAL;
    }
    let arg = unsafe { (*kp).arg };
    if arg.is_null() {
        return -EINVAL;
    }
    let Some(bytes) = (unsafe { c_str_bytes(val, 128) }) else {
        return -EINVAL;
    };
    match parse_i32_base0(bytes) {
        Ok(value) => {
            unsafe { *arg.cast::<i32>() = value };
            0
        }
        Err(err) => -err,
    }
}

/// `param_get_int` - `vendor/linux/kernel/params.c`.
#[unsafe(export_name = "param_get_int")]
pub unsafe extern "C" fn linux_param_get_int(
    buffer: *mut c_char,
    kp: *const LinuxKernelParam,
) -> i32 {
    if kp.is_null() {
        return -EINVAL;
    }
    let arg = unsafe { (*kp).arg };
    if arg.is_null() {
        return -EINVAL;
    }
    write_decimal_i32(buffer, unsafe { *arg.cast::<i32>() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_ops_exports_register_for_modules() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("param_ops_bint").is_some());
        assert!(crate::kernel::module::find_symbol("param_ops_uint").is_some());
        assert!(crate::kernel::module::find_symbol("param_ops_ulong").is_some());
        assert!(crate::kernel::module::find_symbol("param_ops_string").is_some());
        assert!(crate::kernel::module::find_symbol("param_array_ops").is_some());
        assert_eq!(
            crate::kernel::module::find_symbol("param_set_int"),
            Some(linux_param_set_int as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("param_get_int"),
            Some(linux_param_get_int as usize)
        );
    }

    #[test]
    fn kernel_param_layout_matches_linux_x86_64() {
        assert_eq!(core::mem::size_of::<LinuxKernelParam>(), 40);
        assert_eq!(core::mem::offset_of!(LinuxKernelParam, arg), 32);
    }

    #[test]
    fn param_set_int_parses_linux_base0_values() {
        let mut value = 0i32;
        let kp = LinuxKernelParam {
            name: core::ptr::null(),
            mod_: core::ptr::null_mut(),
            ops: core::ptr::null(),
            perm: 0,
            level: 0,
            flags: 0,
            arg: (&mut value as *mut i32).cast(),
        };

        unsafe {
            assert_eq!(linux_param_set_int(b"+0x10\0".as_ptr().cast(), &kp), 0);
            assert_eq!(value, 16);
            assert_eq!(linux_param_set_int(b"010\n\0".as_ptr().cast(), &kp), 0);
            assert_eq!(value, 8);
            assert_eq!(
                linux_param_set_int(b"-2147483648\0".as_ptr().cast(), &kp),
                0
            );
            assert_eq!(value, i32::MIN);
            assert_eq!(
                linux_param_set_int(b"2147483648\0".as_ptr().cast(), &kp),
                -ERANGE
            );
            assert_eq!(value, i32::MIN);
            assert_eq!(linux_param_set_int(b"12x\0".as_ptr().cast(), &kp), -EINVAL);
        }
    }

    #[test]
    fn param_get_int_writes_value_and_newline() {
        let mut value = -42i32;
        let kp = LinuxKernelParam {
            name: core::ptr::null(),
            mod_: core::ptr::null_mut(),
            ops: core::ptr::null(),
            perm: 0,
            level: 0,
            flags: 0,
            arg: (&mut value as *mut i32).cast(),
        };

        unsafe {
            let mut buf = [0i8; 16];
            assert_eq!(linux_param_get_int(buf.as_mut_ptr(), &kp), 4);
            let out = core::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), 5);
            assert_eq!(out, b"-42\n\0");
        }
    }
}
