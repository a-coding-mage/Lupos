//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_seq.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_seq.c
//! `struct trace_seq` — bounded write buffer used by trace_output formatters.
//!
//! Ref: vendor/linux/kernel/trace/trace_seq.c

extern crate alloc;
use alloc::string::String;
use core::ffi::c_char;

use crate::kernel::module::{export_symbol, find_symbol};

const TRACE_SEQ_SIZE: usize = 8192;
const TRACE_SEQ_BUFFER_SIZE: usize = TRACE_SEQ_SIZE - 24 - 8 - 4;

/// Exact x86-64 vendor layout of `struct seq_buf`.
#[repr(C)]
struct LinuxSeqBuf {
    buffer: *mut c_char,
    size: usize,
    len: usize,
}

/// Exact configured layout of `struct trace_seq`.  Generated TRACE_EVENT
/// print functions receive this object directly, so this ABI type cannot be
/// replaced by the Rust-native convenience type below.
#[repr(C)]
struct LinuxTraceSeq {
    seq: LinuxSeqBuf,
    readpos: usize,
    full: i32,
    buffer: [u8; TRACE_SEQ_BUFFER_SIZE],
}

const _: () = assert!(core::mem::size_of::<LinuxTraceSeq>() == TRACE_SEQ_SIZE);
const _: () = assert!(core::mem::offset_of!(LinuxTraceSeq, seq) == 0);
const _: () = assert!(core::mem::offset_of!(LinuxTraceSeq, readpos) == 24);
const _: () = assert!(core::mem::offset_of!(LinuxTraceSeq, full) == 32);
const _: () = assert!(core::mem::offset_of!(LinuxTraceSeq, buffer) == 36);

fn export_symbol_once(name: &'static str, address: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, address, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("trace_seq_printf", linux_trace_seq_printf as usize, true);
    export_symbol_once("trace_seq_puts", linux_trace_seq_puts as usize, true);
    export_symbol_once("trace_seq_putc", linux_trace_seq_putc as usize, true);
    export_symbol_once("trace_seq_putmem", linux_trace_seq_putmem as usize, true);
    export_symbol_once(
        "trace_print_hex_seq",
        linux_trace_print_hex_seq as usize,
        false,
    );
}

unsafe fn initialize_if_needed(sequence: &mut LinuxTraceSeq) {
    if sequence.seq.size != 0 {
        return;
    }
    sequence.seq.buffer = sequence.buffer.as_mut_ptr().cast();
    sequence.seq.size = TRACE_SEQ_BUFFER_SIZE;
    sequence.seq.len = 0;
    sequence.readpos = 0;
    sequence.full = 0;
}

unsafe fn c_string_len(mut string: *const c_char, limit: usize) -> usize {
    let mut len = 0;
    while !string.is_null() && len < limit && unsafe { string.read() } != 0 {
        len += 1;
        string = unsafe { string.add(1) };
    }
    len
}

unsafe fn append(sequence: *mut LinuxTraceSeq, bytes: *const u8, len: usize) {
    let Some(sequence) = (unsafe { sequence.as_mut() }) else {
        return;
    };
    if sequence.full != 0 || (bytes.is_null() && len != 0) {
        return;
    }
    unsafe { initialize_if_needed(sequence) };
    if len > sequence.seq.size.saturating_sub(sequence.seq.len) {
        sequence.full = 1;
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes,
            sequence.seq.buffer.cast::<u8>().add(sequence.seq.len),
            len,
        );
    }
    sequence.seq.len += len;
}

/// Linux `trace_seq_printf()`. Rust has no stable C-variadic definitions, so
/// the entry captures the complete SysV integer argument sequence used by the
/// configured module print functions and feeds it to the shared kernel
/// formatter. Floating point is forbidden in Linux kernel format strings.
unsafe extern "C" fn linux_trace_seq_printf(
    sequence: *mut LinuxTraceSeq,
    format: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    arg7: usize,
) {
    let Some(sequence) = (unsafe { sequence.as_mut() }) else {
        return;
    };
    if format.is_null() || sequence.full != 0 {
        return;
    }
    unsafe { initialize_if_needed(sequence) };
    let saved = sequence.seq.len;
    let available = sequence.seq.size.saturating_sub(saved);
    let args = [arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7];
    let extra = [0usize; 8];
    let written = unsafe {
        crate::linux_driver_abi::base::printf::vsnprintf_n(
            sequence.seq.buffer.cast::<u8>().add(saved),
            available,
            format,
            args.as_ptr(),
            args.len(),
            extra.as_ptr(),
        )
    };
    if written >= available {
        sequence.seq.len = saved;
        sequence.full = 1;
    } else {
        sequence.seq.len = saved + written;
    }
}

unsafe extern "C" fn linux_trace_seq_puts(sequence: *mut LinuxTraceSeq, string: *const c_char) {
    let len = unsafe { c_string_len(string, TRACE_SEQ_BUFFER_SIZE) };
    unsafe { append(sequence, string.cast(), len) };
}

unsafe extern "C" fn linux_trace_seq_putc(sequence: *mut LinuxTraceSeq, character: u8) {
    unsafe { append(sequence, &character, 1) };
}

unsafe extern "C" fn linux_trace_seq_putmem(
    sequence: *mut LinuxTraceSeq,
    memory: *const u8,
    len: u32,
) {
    unsafe { append(sequence, memory, len as usize) };
}

unsafe extern "C" fn linux_trace_print_hex_seq(
    sequence: *mut LinuxTraceSeq,
    buffer: *const u8,
    buffer_len: i32,
    concatenate: bool,
) -> *const c_char {
    let Some(sequence_ref) = (unsafe { sequence.as_mut() }) else {
        return core::ptr::null();
    };
    unsafe { initialize_if_needed(sequence_ref) };
    let start = unsafe {
        sequence_ref
            .seq
            .buffer
            .cast::<u8>()
            .add(sequence_ref.seq.len)
            .cast::<c_char>()
    };
    if buffer.is_null() || buffer_len <= 0 {
        unsafe { linux_trace_seq_putc(sequence, 0) };
        return start;
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for index in 0..buffer_len as usize {
        if !concatenate && index != 0 {
            unsafe { linux_trace_seq_putc(sequence, b' ') };
        }
        let byte = unsafe { buffer.add(index).read() };
        let pair = [HEX[(byte >> 4) as usize], HEX[(byte & 0xf) as usize]];
        unsafe { append(sequence, pair.as_ptr(), pair.len()) };
    }
    unsafe { linux_trace_seq_putc(sequence, 0) };
    start
}

pub struct TraceSeq {
    pub buf: String,
    pub full: bool,
}

impl TraceSeq {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            full: false,
        }
    }

    pub fn puts(&mut self, s: &str) -> bool {
        if self.full {
            return false;
        }
        self.buf.push_str(s);
        true
    }

    pub fn putc(&mut self, c: char) -> bool {
        if self.full {
            return false;
        }
        self.buf.push(c);
        true
    }

    pub fn set_full(&mut self) {
        self.full = true;
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puts_appends_when_not_full() {
        let mut s = TraceSeq::new();
        assert!(s.puts("hello "));
        assert!(s.putc('w'));
        assert_eq!(s.buf, "hello w");
    }

    #[test]
    fn puts_drops_when_full() {
        let mut s = TraceSeq::new();
        s.set_full();
        assert!(!s.puts("ignored"));
        assert_eq!(s.buf, "");
    }

    #[test]
    fn linux_trace_seq_layout_and_exports_match_vendor_abi() {
        assert_eq!(core::mem::size_of::<LinuxTraceSeq>(), TRACE_SEQ_SIZE);
        register_module_exports();
        assert_eq!(
            find_symbol("trace_seq_printf"),
            Some(linux_trace_seq_printf as usize)
        );
        assert_eq!(
            find_symbol("trace_seq_puts"),
            Some(linux_trace_seq_puts as usize)
        );
    }

    #[test]
    fn linux_trace_seq_formats_and_appends_without_partial_overflow() {
        let mut sequence: LinuxTraceSeq = unsafe { core::mem::zeroed() };
        let format = b"value=%u\0";
        unsafe {
            linux_trace_seq_printf(
                &mut sequence,
                format.as_ptr().cast(),
                42,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            );
            linux_trace_seq_putc(&mut sequence, b'!');
        }
        assert_eq!(sequence.seq.len, 9);
        assert_eq!(&sequence.buffer[..9], b"value=42!");
        assert_eq!(sequence.full, 0);
    }

    #[test]
    fn linux_trace_print_hex_seq_matches_spaced_and_concatenated_forms() {
        let bytes = [0x01u8, 0xab, 0xff];
        let mut spaced: LinuxTraceSeq = unsafe { core::mem::zeroed() };
        unsafe { linux_trace_print_hex_seq(&mut spaced, bytes.as_ptr(), 3, false) };
        assert_eq!(&spaced.buffer[..9], b"01 ab ff\0");

        let mut joined: LinuxTraceSeq = unsafe { core::mem::zeroed() };
        unsafe { linux_trace_print_hex_seq(&mut joined, bytes.as_ptr(), 3, true) };
        assert_eq!(&joined.buffer[..7], b"01abff\0");
    }
}
