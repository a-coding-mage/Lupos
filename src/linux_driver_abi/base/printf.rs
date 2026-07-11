//! linux-parity: partial
//! linux-source: vendor/linux/lib/vsprintf.c
//! x86-64 module-ABI formatting support for C variadic entry points.
//!
//! Rust cannot declare a C-variadic function on the kernel's stable language
//! surface. The exported entry points save the four remaining SysV integer
//! argument registers and pass them, plus the caller's stack argument area,
//! to this cursor. Linux kernel format arguments are integer or pointer values
//! (floating point is forbidden in the kernel), so this is the native x86-64
//! `va_list` ordering used by the vendor objects.

#[derive(Clone, Copy, Eq, PartialEq)]
enum Length {
    Default,
    Char,
    Short,
    Long,
    LongLong,
    Size,
    PtrDiff,
}

struct LinuxVaCursor {
    register_args: *const usize,
    stack_args: *const usize,
    index: usize,
}

impl LinuxVaCursor {
    unsafe fn next(&mut self) -> usize {
        let value = if self.index < 4 {
            unsafe { self.register_args.add(self.index).read() }
        } else {
            unsafe { self.stack_args.add(self.index - 4).read() }
        };
        self.index += 1;
        value
    }
}

struct BufferWriter {
    buf: *mut u8,
    size: usize,
    len: usize,
}

impl BufferWriter {
    unsafe fn new(buf: *mut u8, size: usize) -> Self {
        Self { buf, size, len: 0 }
    }

    fn push(&mut self, byte: u8) {
        if self.len.saturating_add(1) < self.size {
            unsafe {
                self.buf.add(self.len).write(byte);
            }
            self.len += 1;
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes.iter().copied() {
            self.push(byte);
        }
    }

    fn spaces(&mut self, count: usize) {
        for _ in 0..count {
            self.push(b' ');
        }
    }

    fn zeroes(&mut self, count: usize) {
        for _ in 0..count {
            self.push(b'0');
        }
    }

    unsafe fn finish(self) -> usize {
        if self.size != 0 && !self.buf.is_null() {
            unsafe {
                self.buf.add(self.len).write(0);
            }
        }
        self.len
    }
}

unsafe fn c_bytes<'a>(ptr: *const core::ffi::c_char, limit: usize) -> &'a [u8] {
    if ptr.is_null() {
        return &[];
    }
    let mut len = 0usize;
    while len < limit && unsafe { ptr.add(len).read() } != 0 {
        len += 1;
    }
    unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) }
}

fn unsigned_digits(mut value: u64, radix: u64, upper: bool, out: &mut [u8; 64]) -> usize {
    let mut start = out.len();
    loop {
        let digit = (value % radix) as u8;
        start -= 1;
        out[start] = match digit {
            0..=9 => b'0' + digit,
            _ if upper => b'A' + digit - 10,
            _ => b'a' + digit - 10,
        };
        value /= radix;
        if value == 0 {
            break;
        }
    }
    start
}

fn push_number(
    writer: &mut BufferWriter,
    prefix: &[u8],
    digits: &[u8],
    width: usize,
    precision: Option<usize>,
    left: bool,
    zero: bool,
) {
    let precision_zeroes = precision
        .map(|precision| precision.saturating_sub(digits.len()))
        .unwrap_or(0);
    let padding = width.saturating_sub(prefix.len() + precision_zeroes + digits.len());
    if !left && !(zero && precision.is_none()) {
        writer.spaces(padding);
    }
    writer.push_bytes(prefix);
    if !left && zero && precision.is_none() {
        writer.zeroes(padding);
    }
    writer.zeroes(precision_zeroes);
    writer.push_bytes(digits);
    if left {
        writer.spaces(padding);
    }
}

fn signed_value(raw: usize, length: Length) -> i64 {
    match length {
        Length::Char => (raw as u8 as i8) as i64,
        Length::Short => (raw as u16 as i16) as i64,
        Length::Default => (raw as u32 as i32) as i64,
        Length::Long | Length::LongLong | Length::Size | Length::PtrDiff => raw as i64,
    }
}

fn unsigned_value(raw: usize, length: Length) -> u64 {
    match length {
        Length::Char => raw as u8 as u64,
        Length::Short => raw as u16 as u64,
        Length::Default => raw as u32 as u64,
        Length::Long | Length::LongLong | Length::Size | Length::PtrDiff => raw as u64,
    }
}

/// `vscnprintf()` for integer/pointer arguments captured by an x86-64 ABI
/// trampoline. The returned count is capped at `size - 1`, and a non-empty
/// destination is always NUL-terminated, matching Linux `vscnprintf()`.
///
/// This covers the conversion set exercised by the selected vendor virtio PCI
/// and block modules: strings, characters, signed/unsigned integers,
/// octal/hexadecimal, pointers, flags, width, precision, and standard integer
/// length modifiers.
pub(crate) unsafe fn vscnprintf(
    buf: *mut u8,
    size: usize,
    fmt: *const core::ffi::c_char,
    register_args: *const usize,
    stack_args: *const usize,
) -> usize {
    if buf.is_null()
        || size == 0
        || fmt.is_null()
        || register_args.is_null()
        || stack_args.is_null()
    {
        return 0;
    }

    let format = unsafe { c_bytes(fmt, 8192) };
    let mut args = LinuxVaCursor {
        register_args,
        stack_args,
        index: 0,
    };
    let mut writer = unsafe { BufferWriter::new(buf, size) };
    let mut index = 0usize;

    while index < format.len() {
        if format[index] != b'%' {
            writer.push(format[index]);
            index += 1;
            continue;
        }
        index += 1;
        if index < format.len() && format[index] == b'%' {
            writer.push(b'%');
            index += 1;
            continue;
        }

        let mut left = false;
        let mut plus = false;
        let mut space = false;
        let mut alternate = false;
        let mut zero = false;
        while index < format.len() {
            match format[index] {
                b'-' => left = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'#' => alternate = true,
                b'0' => zero = true,
                _ => break,
            }
            index += 1;
        }

        let mut width = 0usize;
        if index < format.len() && format[index] == b'*' {
            let dynamic = unsafe { args.next() } as u32 as i32;
            if dynamic < 0 {
                left = true;
                width = dynamic.wrapping_neg() as usize;
            } else {
                width = dynamic as usize;
            }
            index += 1;
        } else {
            while index < format.len() && format[index].is_ascii_digit() {
                width = width
                    .saturating_mul(10)
                    .saturating_add((format[index] - b'0') as usize);
                index += 1;
            }
        }

        let mut precision = None;
        if index < format.len() && format[index] == b'.' {
            index += 1;
            let mut value = 0usize;
            if index < format.len() && format[index] == b'*' {
                let dynamic = unsafe { args.next() } as u32 as i32;
                if dynamic >= 0 {
                    precision = Some(dynamic as usize);
                }
                index += 1;
            } else {
                while index < format.len() && format[index].is_ascii_digit() {
                    value = value
                        .saturating_mul(10)
                        .saturating_add((format[index] - b'0') as usize);
                    index += 1;
                }
                precision = Some(value);
            }
        }

        let mut length = Length::Default;
        if index < format.len() {
            match format[index] {
                b'h' => {
                    index += 1;
                    if index < format.len() && format[index] == b'h' {
                        length = Length::Char;
                        index += 1;
                    } else {
                        length = Length::Short;
                    }
                }
                b'l' => {
                    index += 1;
                    if index < format.len() && format[index] == b'l' {
                        length = Length::LongLong;
                        index += 1;
                    } else {
                        length = Length::Long;
                    }
                }
                b'z' => {
                    length = Length::Size;
                    index += 1;
                }
                b't' => {
                    length = Length::PtrDiff;
                    index += 1;
                }
                _ => {}
            }
        }

        if index >= format.len() {
            writer.push(b'%');
            break;
        }
        let specifier = format[index];
        index += 1;

        match specifier {
            b'c' => {
                let byte = unsafe { args.next() } as u8;
                let padding = width.saturating_sub(1);
                if !left {
                    writer.spaces(padding);
                }
                writer.push(byte);
                if left {
                    writer.spaces(padding);
                }
            }
            b's' => {
                let ptr = unsafe { args.next() } as *const core::ffi::c_char;
                let bytes = if ptr.is_null() {
                    b"(null)".as_slice()
                } else {
                    unsafe { c_bytes(ptr, 8192) }
                };
                let len = precision.map_or(bytes.len(), |limit| bytes.len().min(limit));
                let bytes = &bytes[..len];
                let padding = width.saturating_sub(bytes.len());
                if !left {
                    writer.spaces(padding);
                }
                writer.push_bytes(bytes);
                if left {
                    writer.spaces(padding);
                }
            }
            b'd' | b'i' => {
                let value = signed_value(unsafe { args.next() }, length);
                let (negative, magnitude) = if value < 0 {
                    (true, value.wrapping_neg() as u64)
                } else {
                    (false, value as u64)
                };
                let mut storage = [0u8; 64];
                let start = unsigned_digits(magnitude, 10, false, &mut storage);
                let digits = if magnitude == 0 && precision == Some(0) {
                    &storage[storage.len()..]
                } else {
                    &storage[start..]
                };
                let prefix = if negative {
                    b"-".as_slice()
                } else if plus {
                    b"+".as_slice()
                } else if space {
                    b" ".as_slice()
                } else {
                    b"".as_slice()
                };
                push_number(&mut writer, prefix, digits, width, precision, left, zero);
            }
            b'u' | b'o' | b'x' | b'X' => {
                let value = unsigned_value(unsafe { args.next() }, length);
                let radix = if specifier == b'o' {
                    8
                } else if matches!(specifier, b'x' | b'X') {
                    16
                } else {
                    10
                };
                let mut storage = [0u8; 64];
                let start = unsigned_digits(value, radix, specifier == b'X', &mut storage);
                let digits = if value == 0 && precision == Some(0) {
                    &storage[storage.len()..]
                } else {
                    &storage[start..]
                };
                let prefix = if alternate && value != 0 {
                    match specifier {
                        b'o' => b"0".as_slice(),
                        b'x' => b"0x".as_slice(),
                        b'X' => b"0X".as_slice(),
                        _ => b"".as_slice(),
                    }
                } else {
                    b"".as_slice()
                };
                push_number(&mut writer, prefix, digits, width, precision, left, zero);
            }
            b'p' => {
                let value = unsafe { args.next() } as u64;
                // `%px` is Linux's explicit unhashed pointer form. The
                // selected virtio modules do not use the other `%p` extension
                // families, so leave their suffix visible rather than
                // pretending to decode an unrelated object type.
                if index < format.len() && format[index] == b'x' {
                    index += 1;
                }
                let mut storage = [0u8; 64];
                let start = unsigned_digits(value, 16, false, &mut storage);
                push_number(
                    &mut writer,
                    b"0x",
                    &storage[start..],
                    width,
                    precision,
                    left,
                    zero,
                );
            }
            b'%' => writer.push(b'%'),
            other => {
                writer.push(b'%');
                writer.push(other);
            }
        }
    }

    unsafe { writer.finish() }
}
