//! linux-parity: partial
//! linux-source: vendor/linux/lib/seq_buf.c
//! Minimal `struct seq_buf` helpers for Linux-built modules.

use core::ffi::c_char;

use crate::kernel::module::{export_symbol, find_symbol};

#[repr(C)]
pub struct SeqBuf {
    buffer: *mut c_char,
    size: usize,
    len: usize,
}

const _: () = assert!(core::mem::size_of::<SeqBuf>() == 24);
const _: () = assert!(core::mem::offset_of!(SeqBuf, buffer) == 0);
const _: () = assert!(core::mem::offset_of!(SeqBuf, size) == 8);
const _: () = assert!(core::mem::offset_of!(SeqBuf, len) == 16);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("seq_buf_printf", linux_seq_buf_printf as usize, true);
    export_symbol_once("seq_buf_puts", linux_seq_buf_puts as usize, true);
    export_symbol_once("seq_buf_putc", linux_seq_buf_putc as usize, true);
}

fn set_overflow(s: &mut SeqBuf) {
    s.len = s.size.saturating_add(1);
}

fn has_overflowed(s: &SeqBuf) -> bool {
    s.len > s.size
}

fn buffer_left(s: &SeqBuf) -> usize {
    if has_overflowed(s) {
        0
    } else {
        s.size.saturating_sub(s.len)
    }
}

unsafe fn c_bytes<'a>(ptr: *const c_char, limit: usize) -> &'a [u8] {
    if ptr.is_null() {
        return &[];
    }
    let mut len = 0usize;
    while len < limit && unsafe { ptr.add(len).read() } != 0 {
        len += 1;
    }
    unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) }
}

unsafe fn terminate_at(s: &mut SeqBuf, index: usize) {
    if !s.buffer.is_null() && index < s.size {
        unsafe { s.buffer.add(index).write(0) };
    }
}

unsafe fn append_bytes(s: &mut SeqBuf, bytes: &[u8]) -> i32 {
    if s.buffer.is_null() || s.size == 0 || bytes.len().saturating_add(1) > buffer_left(s) {
        set_overflow(s);
        return -1;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            s.buffer.add(s.len).cast::<u8>(),
            bytes.len(),
        );
    }
    s.len += bytes.len();
    unsafe { terminate_at(s, s.len) };
    0
}

unsafe extern "C" fn linux_seq_buf_puts(s: *mut SeqBuf, str_: *const c_char) -> i32 {
    if s.is_null() || str_.is_null() {
        return -1;
    }
    let bytes = unsafe { c_bytes(str_, 8192) };
    unsafe { append_bytes(&mut *s, bytes) }
}

unsafe extern "C" fn linux_seq_buf_putc(s: *mut SeqBuf, c: u8) -> i32 {
    if s.is_null() {
        return -1;
    }
    unsafe { append_bytes(&mut *s, &[c]) }
}

unsafe extern "C" fn linux_seq_buf_printf(
    s: *mut SeqBuf,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> i32 {
    if s.is_null() || fmt.is_null() {
        return -1;
    }
    let seq = unsafe { &mut *s };
    if seq.buffer.is_null() || seq.len >= seq.size {
        set_overflow(seq);
        return -1;
    }

    let available = seq.size - seq.len;
    let args = [arg0, arg1, arg2, arg3, arg4, arg5];
    let zero_stack = [0usize; 8];
    let written = unsafe {
        crate::linux_driver_abi::base::printf::vscnprintf_n(
            seq.buffer.add(seq.len).cast::<u8>(),
            available,
            fmt,
            args.as_ptr(),
            args.len(),
            zero_stack.as_ptr(),
        )
    };

    if written < available.saturating_sub(1) {
        seq.len += written;
        return 0;
    }

    set_overflow(seq);
    -1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_buf_layout_matches_vendor() {
        assert_eq!(core::mem::size_of::<SeqBuf>(), 24);
        assert_eq!(core::mem::offset_of!(SeqBuf, buffer), 0);
        assert_eq!(core::mem::offset_of!(SeqBuf, size), 8);
        assert_eq!(core::mem::offset_of!(SeqBuf, len), 16);
    }

    #[test]
    fn seq_buf_puts_appends_and_terminates() {
        let mut storage = [0u8; 16];
        let mut seq = SeqBuf {
            buffer: storage.as_mut_ptr().cast(),
            size: storage.len(),
            len: 0,
        };
        let text = b"abc\0";
        assert_eq!(
            unsafe { linux_seq_buf_puts(&mut seq, text.as_ptr().cast()) },
            0
        );
        assert_eq!(seq.len, 3);
        assert_eq!(&storage[..4], b"abc\0");
    }

    #[test]
    fn seq_buf_printf_formats_integer() {
        let mut storage = [0u8; 32];
        let mut seq = SeqBuf {
            buffer: storage.as_mut_ptr().cast(),
            size: storage.len(),
            len: 0,
        };
        let fmt = b"year %d\0";
        assert_eq!(
            unsafe { linux_seq_buf_printf(&mut seq, fmt.as_ptr().cast(), 2026, 0, 0, 0, 0, 0,) },
            0
        );
        assert_eq!(&storage[..10], b"year 2026\0");
    }

    #[test]
    fn seq_buf_exports_registered() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("seq_buf_printf"),
            Some(linux_seq_buf_printf as usize)
        );
    }
}
