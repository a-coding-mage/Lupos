//! linux-parity: partial
//! linux-source: vendor/linux/lib/vsprintf.c
//! test-origin: linux:vendor/linux/lib/vsprintf.c
//! Minimal exported printf formatting used by Linux-built modules.

use core::ffi::{c_char, c_void};

use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::string::c_strlen;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("snprintf", linux_snprintf as usize, false);
    export_symbol_once("sprintf", linux_sprintf as usize, false);
    export_symbol_once("vsnprintf", linux_vsnprintf as usize, false);
    export_symbol_once("vsprintf", linux_vsprintf as usize, false);
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

unsafe fn format_one_arg(buf: *mut c_char, size: usize, fmt: *const c_char, arg0: usize) -> i32 {
    if buf.is_null() || fmt.is_null() {
        return 0;
    }
    let mut pos = 0usize;
    let mut idx = 0usize;
    while unsafe { *fmt.add(idx) } != 0 {
        let ch = unsafe { *fmt.add(idx) } as u8;
        if ch != b'%' {
            unsafe { write_byte(buf, size, &mut pos, ch) };
            idx += 1;
            continue;
        }
        idx += 1;
        let spec = unsafe { *fmt.add(idx) } as u8;
        match spec {
            b'%' => unsafe { write_byte(buf, size, &mut pos, b'%') },
            b'c' => unsafe { write_byte(buf, size, &mut pos, arg0 as u8) },
            b'd' | b'i' | b'u' => unsafe { write_decimal(buf, size, &mut pos, arg0) },
            b's' => {
                let s = arg0 as *const c_char;
                // Mirror Linux vsnprintf: a NULL `%s` argument prints "(null)"
                // rather than dereferencing it. `slice::from_raw_parts` also
                // forbids a null base even for a zero length, so guard here.
                if s.is_null() {
                    unsafe { write_bytes(buf, size, &mut pos, b"(null)") };
                } else {
                    let len = unsafe { c_strlen(s, 4096) };
                    let bytes = unsafe { core::slice::from_raw_parts(s.cast::<u8>(), len) };
                    unsafe { write_bytes(buf, size, &mut pos, bytes) };
                }
            }
            _ => {
                unsafe { write_byte(buf, size, &mut pos, b'%') };
                unsafe { write_byte(buf, size, &mut pos, spec) };
            }
        }
        idx += 1;
    }
    unsafe { terminate(buf, size, pos) };
    pos as i32
}

/// `snprintf` - `vendor/linux/lib/vsprintf.c:3036`.
///
/// Current module coverage only needs the one-argument formats used by
/// `vendor/linux/drivers/block/virtio_blk.c` and
/// `vendor/linux/drivers/virtio/virtio_pci_common.c` for queue names.
pub unsafe extern "C" fn linux_snprintf(
    buf: *mut c_char,
    size: usize,
    fmt: *const c_char,
    arg0: usize,
) -> i32 {
    unsafe { format_one_arg(buf, size, fmt, arg0) }
}

/// `sprintf` - `vendor/linux/lib/vsprintf.c:3105`.
pub unsafe extern "C" fn linux_sprintf(buf: *mut c_char, fmt: *const c_char, arg0: usize) -> i32 {
    unsafe { format_one_arg(buf, i32::MAX as usize, fmt, arg0) }
}

/// `vsprintf` - `vendor/linux/lib/vsprintf.c:3088`.
#[unsafe(export_name = "vsprintf")]
pub unsafe extern "C" fn linux_vsprintf(
    buf: *mut c_char,
    fmt: *const c_char,
    args: *const c_void,
) -> i32 {
    unsafe {
        crate::linux_driver_abi::base::printf::vscnprintf_va_list(
            buf.cast::<u8>(),
            i32::MAX as usize,
            fmt,
            args,
        )
        .min(i32::MAX as usize) as i32
    }
}

/// `vsnprintf` - `vendor/linux/lib/vsprintf.c:2860`.
#[unsafe(export_name = "vsnprintf")]
pub unsafe extern "C" fn linux_vsnprintf(
    buf: *mut c_char,
    size: usize,
    fmt: *const c_char,
    args: *const c_void,
) -> i32 {
    if size > i32::MAX as usize {
        return 0;
    }
    unsafe {
        crate::linux_driver_abi::base::printf::vsnprintf_va_list(buf.cast::<u8>(), size, fmt, args)
            .min(i32::MAX as usize) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snprintf_export_registers_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("snprintf"),
            Some(linux_snprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("sprintf"),
            Some(linux_sprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vsnprintf"),
            Some(linux_vsnprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vsprintf"),
            Some(linux_vsprintf as usize)
        );
    }

    #[test]
    fn vsnprintf_export_matches_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/vsprintf.c"
        ));

        assert!(
            source.contains(
                "int vsnprintf(char *buf, size_t size, const char *fmt_str, va_list args)"
            )
        );
        assert!(source.contains("EXPORT_SYMBOL(vsnprintf);"));
    }

    #[test]
    fn snprintf_formats_queue_name_integer() {
        unsafe {
            let mut buf = [0i8; 16];
            let fmt = b"req.%u\0";
            assert_eq!(
                linux_snprintf(buf.as_mut_ptr(), buf.len(), fmt.as_ptr().cast(), 7),
                5
            );
            let out = core::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), 5);
            assert_eq!(out, b"req.7");
        }
    }

    #[test]
    fn sprintf_formats_virtio_admin_queue_name() {
        unsafe {
            let mut buf = [0i8; 16];
            let fmt = b"avq.%u\0";
            assert_eq!(linux_sprintf(buf.as_mut_ptr(), fmt.as_ptr().cast(), 12), 6);
            let out = core::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), 6);
            assert_eq!(out, b"avq.12");
        }
    }
}
