//! linux-parity: complete
//! linux-source: vendor/linux/lib/kasprintf.c
//! test-origin: linux:vendor/linux/lib/kasprintf.c
//! Kernel asprintf helpers with `%s` const-string fast paths.

extern crate alloc;

use alloc::string::String;
use core::ffi::c_char;

use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::GfpFlags;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KvasprintfConst<'a> {
    Const(&'a str),
    Allocated(String),
}

pub fn kvasprintf(fmt: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut arg_index = 0usize;
    let mut chars = fmt.chars();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('%') => out.push('%'),
            Some('s') => {
                if let Some(arg) = args.get(arg_index) {
                    out.push_str(arg);
                    arg_index += 1;
                }
            }
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }

    out
}

pub fn kvasprintf_const<'a>(fmt: &'a str, args: &'a [&'a str]) -> KvasprintfConst<'a> {
    if !fmt.as_bytes().contains(&b'%') {
        return KvasprintfConst::Const(fmt);
    }
    if fmt == "%s" {
        return KvasprintfConst::Const(args.first().copied().unwrap_or(""));
    }
    KvasprintfConst::Allocated(kvasprintf(fmt, args))
}

pub fn kasprintf(fmt: &str, args: &[&str]) -> String {
    kvasprintf(fmt, args)
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("kasprintf", linux_kasprintf as usize, false);
    export_symbol_once("kvasprintf", linux_kvasprintf as usize, false);
    export_symbol_once("kvasprintf_const", linux_kvasprintf_const as usize, false);
}

unsafe fn c_str_bytes<'a>(ptr: *const c_char, limit: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return None;
    }
    let len = unsafe { crate::lib::string::c_strlen(ptr, limit) };
    Some(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) })
}

#[derive(Clone, Copy)]
enum FormatLength {
    Default,
    Char,
    Short,
    Long,
    LongLong,
    Size,
    PtrDiff,
}

fn signed_arg(raw: usize, length: FormatLength) -> i64 {
    match length {
        FormatLength::Char => raw as u8 as i8 as i64,
        FormatLength::Short => raw as u16 as i16 as i64,
        FormatLength::Default => raw as u32 as i32 as i64,
        FormatLength::Long
        | FormatLength::LongLong
        | FormatLength::Size
        | FormatLength::PtrDiff => raw as i64,
    }
}

fn unsigned_arg(raw: usize, length: FormatLength) -> u64 {
    match length {
        FormatLength::Char => raw as u8 as u64,
        FormatLength::Short => raw as u16 as u64,
        FormatLength::Default => raw as u32 as u64,
        FormatLength::Long
        | FormatLength::LongLong
        | FormatLength::Size
        | FormatLength::PtrDiff => raw as u64,
    }
}

fn format_c_with<F>(fmt: &[u8], mut next_arg: F) -> String
where
    F: FnMut() -> usize,
{
    let mut out = String::new();
    let mut index = 0usize;
    while index < fmt.len() {
        if fmt[index] != b'%' {
            out.push(fmt[index] as char);
            index += 1;
            continue;
        }

        index += 1;
        if index >= fmt.len() {
            out.push('%');
            break;
        }
        if fmt[index] == b'%' {
            out.push('%');
            index += 1;
            continue;
        }

        while index < fmt.len() && matches!(fmt[index], b'#' | b'0' | b'-' | b'+' | b' ') {
            index += 1;
        }
        if index < fmt.len() && fmt[index] == b'*' {
            let _ = next_arg();
            index += 1;
        } else {
            while index < fmt.len() && fmt[index].is_ascii_digit() {
                index += 1;
            }
        }
        if index < fmt.len() && fmt[index] == b'.' {
            index += 1;
            if index < fmt.len() && fmt[index] == b'*' {
                let _ = next_arg();
                index += 1;
            } else {
                while index < fmt.len() && fmt[index].is_ascii_digit() {
                    index += 1;
                }
            }
        }

        let mut length = FormatLength::Default;
        if index < fmt.len() {
            match fmt[index] {
                b'h' => {
                    index += 1;
                    length = if index < fmt.len() && fmt[index] == b'h' {
                        index += 1;
                        FormatLength::Char
                    } else {
                        FormatLength::Short
                    };
                }
                b'l' => {
                    index += 1;
                    length = if index < fmt.len() && fmt[index] == b'l' {
                        index += 1;
                        FormatLength::LongLong
                    } else {
                        FormatLength::Long
                    };
                }
                b'z' => {
                    index += 1;
                    length = FormatLength::Size;
                }
                b't' => {
                    index += 1;
                    length = FormatLength::PtrDiff;
                }
                _ => {}
            }
        }
        if index >= fmt.len() {
            break;
        }

        let spec = fmt[index];
        index += 1;
        match spec {
            b'c' => out.push((next_arg() as u8) as char),
            b'd' | b'i' => out.push_str(&alloc::format!("{}", signed_arg(next_arg(), length))),
            b'u' => out.push_str(&alloc::format!("{}", unsigned_arg(next_arg(), length))),
            b'o' => out.push_str(&alloc::format!("{:o}", unsigned_arg(next_arg(), length))),
            b'x' => out.push_str(&alloc::format!("{:x}", unsigned_arg(next_arg(), length))),
            b'X' => out.push_str(&alloc::format!("{:X}", unsigned_arg(next_arg(), length))),
            b'p' => out.push_str(&alloc::format!("{:#x}", next_arg())),
            b's' => {
                let ptr = next_arg() as *const c_char;
                match unsafe { c_str_bytes(ptr, 4096) } {
                    Some(bytes) => out.push_str(core::str::from_utf8(bytes).unwrap_or("")),
                    None => out.push_str("(null)"),
                }
            }
            other => {
                out.push('%');
                out.push(other as char);
            }
        }
    }
    out
}

#[repr(C)]
#[derive(Clone, Copy)]
struct X86VaList {
    gp_offset: u32,
    fp_offset: u32,
    overflow_arg_area: *const usize,
    reg_save_area: *const u8,
}

impl X86VaList {
    unsafe fn next_usize(&mut self) -> usize {
        if self.gp_offset < 48 && !self.reg_save_area.is_null() {
            let ptr = unsafe { self.reg_save_area.add(self.gp_offset as usize) }.cast::<usize>();
            self.gp_offset = self.gp_offset.saturating_add(8);
            unsafe { ptr.read() }
        } else if !self.overflow_arg_area.is_null() {
            let ptr = self.overflow_arg_area;
            self.overflow_arg_area = unsafe { self.overflow_arg_area.add(1) };
            unsafe { ptr.read() }
        } else {
            0
        }
    }
}

unsafe fn format_va(fmt: *const c_char, args: *const X86VaList) -> String {
    let fmt = unsafe { c_str_bytes(fmt, 8192) }.unwrap_or(b"");
    if args.is_null() {
        return format_c_with(fmt, || 0);
    }
    let mut cursor = unsafe { args.read() };
    format_c_with(fmt, || unsafe { cursor.next_usize() })
}

unsafe fn kmalloc_string(text: &str, gfp: GfpFlags) -> *mut c_char {
    let Some(size) = text.len().checked_add(1) else {
        return core::ptr::null_mut();
    };
    let ptr = unsafe { crate::mm::slab::kmalloc(size, gfp) };
    if ptr.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        core::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len());
        *ptr.add(text.len()) = 0;
    }
    ptr.cast()
}

pub unsafe extern "C" fn linux_kasprintf(
    gfp: GfpFlags,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> *mut c_char {
    let fmt = unsafe { c_str_bytes(fmt, 8192) }.unwrap_or(b"");
    let args = [arg0, arg1, arg2, arg3];
    let mut index = 0usize;
    let text = format_c_with(fmt, || {
        let value = args.get(index).copied().unwrap_or(0);
        index += 1;
        value
    });
    unsafe { kmalloc_string(&text, gfp) }
}

pub unsafe extern "C" fn linux_kvasprintf(
    gfp: GfpFlags,
    fmt: *const c_char,
    args: *const X86VaList,
) -> *mut c_char {
    let text = unsafe { format_va(fmt, args) };
    unsafe { kmalloc_string(&text, gfp) }
}

pub unsafe extern "C" fn linux_kvasprintf_const(
    gfp: GfpFlags,
    fmt: *const c_char,
    args: *const X86VaList,
) -> *const c_char {
    let Some(fmt_bytes) = (unsafe { c_str_bytes(fmt, 8192) }) else {
        return core::ptr::null();
    };
    if !fmt_bytes.contains(&b'%') {
        return fmt;
    }
    if fmt_bytes == b"%s" {
        if args.is_null() {
            return core::ptr::null();
        }
        let mut cursor = unsafe { args.read() };
        return unsafe { cursor.next_usize() as *const c_char };
    }
    unsafe { linux_kvasprintf(gfp, fmt, args) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kasprintf_helpers_match_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/kasprintf.c"
        ));
        assert!(source.contains("first = vsnprintf(NULL, 0, fmt, aq);"));
        assert!(source.contains("p = kmalloc_track_caller(first+1, gfp);"));
        assert!(source.contains("second = vsnprintf(p, first+1, fmt, ap);"));
        assert!(source.contains("WARN(first != second"));
        assert!(source.contains("if (!strchr(fmt, '%'))"));
        assert!(source.contains("if (!strcmp(fmt, \"%s\"))"));
        assert!(source.contains("return kstrdup_const(va_arg(ap, const char*), gfp);"));
        assert!(source.contains("EXPORT_SYMBOL(kvasprintf);"));
        assert!(source.contains("EXPORT_SYMBOL(kvasprintf_const);"));
        assert!(source.contains("EXPORT_SYMBOL(kasprintf);"));

        assert_eq!(kvasprintf("net/%s/%s", &["eth0", "rx"]), "net/eth0/rx");
        assert_eq!(kvasprintf("100%% %s", &["ok"]), "100% ok");
        assert_eq!(
            kvasprintf_const("literal", &[]),
            KvasprintfConst::Const("literal")
        );
        assert_eq!(
            kvasprintf_const("%s", &["already-const"]),
            KvasprintfConst::Const("already-const")
        );
        assert_eq!(
            kvasprintf_const("%s/%s", &["a", "b"]),
            KvasprintfConst::Allocated(String::from("a/b"))
        );
        assert_eq!(kasprintf("%s", &["value"]), "value");
    }

    #[test]
    fn kasprintf_module_exports_register() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("kasprintf"),
            Some(linux_kasprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kvasprintf"),
            Some(linux_kvasprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kvasprintf_const"),
            Some(linux_kvasprintf_const as usize)
        );
    }

    #[test]
    fn c_formatter_handles_driver_name_formats() {
        let name = b"card\0";
        let args = [name.as_ptr() as usize, 7usize, 0, 0];
        let mut index = 0usize;
        assert_eq!(
            format_c_with(b"%s-%d", || {
                let value = args[index];
                index += 1;
                value
            }),
            "card-7"
        );
        let mut index = 0usize;
        assert_eq!(
            format_c_with(b"pcm%u.%x", || {
                let value = [3usize, 0x2a_usize][index];
                index += 1;
                value
            }),
            "pcm3.2a"
        );
    }
}
