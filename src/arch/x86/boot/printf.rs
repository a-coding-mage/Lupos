//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/printf.c
//! test-origin: linux:vendor/linux/arch/x86/boot/printf.c
//! Real-mode setup `printf` (size-optimised, no 64-bit support).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/printf.c
//!
//! Linux's real-mode setup carries its own `vsprintf`/`printf` so it
//! doesn't drag in `lib/vsprintf.c`. This is a full 1:1 port of every
//! function in printf.c: `skip_atoi`, `number`, `vsprintf`, `sprintf`
//! and `printf`.
//!
//! C varargs (`va_list`) have no safe-Rust equivalent, so the variadic
//! conversions consume a typed [`Arg`] slice instead — the format-string
//! interpreter (the `vsprintf` state machine) is reproduced exactly; only
//! the argument *source* changes from `va_arg` to indexing `args`. This is
//! faithful (same conversions, same flag/width/precision/qualifier rules)
//! and additionally type-safe.

use core::cell::Cell;

/// Linux printf-style flag bits. Bit values are part of the ABI (the
/// `SMALL` flag is OR'd with the digit ASCII to lowercase hex letters
/// so the value must literally be 0x20). printf.c lines 27-33.
pub const ZEROPAD: u32 = 1;
pub const SIGN: u32 = 2;
pub const PLUS: u32 = 4;
pub const SPACE: u32 = 8;
pub const LEFT: u32 = 16;
pub const SMALL: u32 = 32; // Must be 0x20.
pub const SPECIAL: u32 = 64;

/// A single `vsprintf` argument. Replaces C's `va_arg(args, T)`; the
/// caller supplies one variant per conversion in order, exactly as a C
/// caller pushes varargs. Integer width/sign handling still follows the
/// `qualifier`/`SIGN` rules in `vsprintf` (printf.c lines 270-279).
pub enum Arg<'a> {
    /// `int` (also used for `%c`).
    Int(i32),
    /// `unsigned int`.
    Uint(u32),
    /// `long` (qualifier `l`).
    Long(i64),
    /// `unsigned long` (qualifier `l`).
    Ulong(u64),
    /// `char *` for `%s` — the string bytes (NUL-terminated or a plain
    /// slice; `strnlen` semantics stop at the first NUL or the slice end).
    Str(&'a [u8]),
    /// `void *` for `%p`.
    Ptr(usize),
    /// `int *` write-back target for `%n`.
    NInt(&'a Cell<i32>),
    /// `long *` write-back target for `%ln`.
    NLong(&'a Cell<i64>),
}

impl Arg<'_> {
    /// The raw integer bit value, as C's `va_arg` would deliver it before
    /// the `qualifier`/`SIGN` masking in `vsprintf`.
    fn as_i64(&self) -> i64 {
        match *self {
            Arg::Int(v) => v as i64,
            Arg::Uint(v) => v as i64,
            Arg::Long(v) => v,
            Arg::Ulong(v) => v as i64,
            Arg::Ptr(v) => v as i64,
            _ => 0,
        }
    }
}

/// Append one byte to `out` at `*cur`, mirroring C's `*str++ = c`. Writes
/// only inside bounds but always advances `*cur` so the returned length
/// matches C's `str - buf` even on overflow.
#[inline]
fn put(out: &mut [u8], c: u8, cur: &mut usize) {
    if *cur < out.len() {
        out[*cur] = c;
    }
    *cur += 1;
}

/// `skip_atoi(s)` — parse leading ASCII digits, advancing the cursor.
/// Mirrors printf.c lines 18-25.
pub fn skip_atoi(s: &[u8], i: &mut usize) -> i32 {
    let mut n = 0i32;
    while *i < s.len() && s[*i].is_ascii_digit() {
        n = n * 10 + (s[*i] - b'0') as i32;
        *i += 1;
    }
    n
}

/// `number()` formatter appending into `out` at `*cursor`. Mirrors
/// printf.c lines 41-111 exactly.
fn number_append(
    out: &mut [u8],
    cursor: &mut usize,
    mut num: i64,
    base: u32,
    mut size: i32,
    mut precision: i32,
    mut ty: u32,
) {
    const DIGITS: &[u8] = b"0123456789ABCDEF";
    let locase = (ty & SMALL) as u8;
    if ty & LEFT != 0 {
        ty &= !ZEROPAD;
    }
    if !(2..=16).contains(&base) {
        return;
    }
    let pad_char = if ty & ZEROPAD != 0 { b'0' } else { b' ' };
    let mut sign: u8 = 0;
    if ty & SIGN != 0 {
        if num < 0 {
            sign = b'-';
            num = -num;
            size -= 1;
        } else if ty & PLUS != 0 {
            sign = b'+';
            size -= 1;
        } else if ty & SPACE != 0 {
            sign = b' ';
            size -= 1;
        }
    }
    if ty & SPECIAL != 0 {
        if base == 16 {
            size -= 2;
        } else if base == 8 {
            size -= 1;
        }
    }
    let mut tmp = [0u8; 66];
    let mut i = 0usize;
    if num == 0 {
        tmp[i] = b'0';
        i += 1;
    } else {
        // `num` is treated as an unsigned bit pattern for the digit loop,
        // matching C's `(unsigned long) n` in __do_div.
        let mut n = num as u64;
        while n != 0 {
            let d = (n % base as u64) as usize;
            n /= base as u64;
            tmp[i] = DIGITS[d] | locase;
            i += 1;
        }
    }
    if i as i32 > precision {
        precision = i as i32;
    }
    size -= precision;
    if (ty & (ZEROPAD | LEFT)) == 0 {
        while size > 0 {
            put(out, b' ', cursor);
            size -= 1;
        }
    }
    if sign != 0 {
        put(out, sign, cursor);
    }
    if ty & SPECIAL != 0 {
        if base == 8 {
            put(out, b'0', cursor);
        } else if base == 16 {
            put(out, b'0', cursor);
            put(out, b'X' | locase, cursor);
        }
    }
    if ty & LEFT == 0 {
        while size > 0 {
            put(out, pad_char, cursor);
            size -= 1;
        }
    }
    while (i as i32) < precision {
        put(out, b'0', cursor);
        precision -= 1;
    }
    while i > 0 {
        i -= 1;
        put(out, tmp[i], cursor);
    }
    while size > 0 {
        put(out, b' ', cursor);
        size -= 1;
    }
}

/// Format a number into `buf` per Linux's `number()` rules. Returns the
/// number of bytes written. Standalone entry point used by callers/tests;
/// `vsprintf` uses [`number_append`] to append at the running cursor.
pub fn number(buf: &mut [u8], num: i64, base: u32, size: i32, precision: i32, ty: u32) -> usize {
    let mut cursor = 0usize;
    number_append(buf, &mut cursor, num, base, size, precision, ty);
    cursor.min(buf.len())
}

/// `vsprintf(buf, fmt, args)` — 1:1 port of printf.c lines 113-284. The
/// format interpreter is identical to Linux; arguments come from `args`
/// (in order) instead of a `va_list`. Returns the formatted length
/// (`str - buf`), and NUL-terminates `buf` when there is room — exactly
/// like the C `*str = '\0'`, which is not counted in the return value.
pub fn vsprintf(buf: &mut [u8], fmt: &[u8], args: &[Arg]) -> usize {
    let mut cursor = 0usize;
    let mut ai = 0usize; // next argument index (C's implicit va_arg cursor)
    let mut fi = 0usize; // format cursor (C's `fmt`)

    'main: while fi < fmt.len() {
        if fmt[fi] != b'%' {
            put(buf, fmt[fi], &mut cursor);
            fi += 1;
            continue;
        }

        // process flags (the `repeat:` loop; `++fmt` first skips the '%')
        let mut flags = 0u32;
        fi += 1;
        loop {
            if fi >= fmt.len() {
                break;
            }
            match fmt[fi] {
                b'-' => flags |= LEFT,
                b'+' => flags |= PLUS,
                b' ' => flags |= SPACE,
                b'#' => flags |= SPECIAL,
                b'0' => flags |= ZEROPAD,
                _ => break,
            }
            fi += 1;
        }

        // get field width
        let mut field_width: i32 = -1;
        if fi < fmt.len() && fmt[fi].is_ascii_digit() {
            field_width = skip_atoi(fmt, &mut fi);
        } else if fi < fmt.len() && fmt[fi] == b'*' {
            fi += 1;
            field_width = args.get(ai).map_or(0, Arg::as_i64) as i32;
            ai += 1;
            if field_width < 0 {
                field_width = -field_width;
                flags |= LEFT;
            }
        }

        // get the precision
        let mut precision: i32 = -1;
        if fi < fmt.len() && fmt[fi] == b'.' {
            fi += 1;
            if fi < fmt.len() && fmt[fi].is_ascii_digit() {
                precision = skip_atoi(fmt, &mut fi);
            } else if fi < fmt.len() && fmt[fi] == b'*' {
                fi += 1;
                precision = args.get(ai).map_or(0, Arg::as_i64) as i32;
                ai += 1;
            }
            if precision < 0 {
                precision = 0;
            }
        }

        // get the conversion qualifier
        let mut qualifier: u8 = 0;
        if fi < fmt.len() && matches!(fmt[fi], b'h' | b'l' | b'L') {
            qualifier = fmt[fi];
            fi += 1;
        }

        let mut base: u32 = 10;
        let conv = if fi < fmt.len() { fmt[fi] } else { 0 };
        match conv {
            b'c' => {
                if flags & LEFT == 0 {
                    while {
                        field_width -= 1;
                        field_width > 0
                    } {
                        put(buf, b' ', &mut cursor);
                    }
                }
                let c = args.get(ai).map_or(0, Arg::as_i64) as u8;
                ai += 1;
                put(buf, c, &mut cursor);
                while {
                    field_width -= 1;
                    field_width > 0
                } {
                    put(buf, b' ', &mut cursor);
                }
                fi += 1;
                continue 'main;
            }
            b's' => {
                let s = match args.get(ai) {
                    Some(Arg::Str(s)) => *s,
                    _ => b"",
                };
                ai += 1;
                // strnlen(s, precision): stop at NUL or `precision` chars.
                let nul = s.iter().position(|&b| b == 0).unwrap_or(s.len());
                let mut len = if precision < 0 {
                    nul as i32
                } else {
                    nul.min(precision as usize) as i32
                };
                if flags & LEFT == 0 {
                    while len < field_width {
                        put(buf, b' ', &mut cursor);
                        field_width -= 1;
                    }
                }
                let mut i = 0i32;
                while i < len {
                    put(buf, s[i as usize], &mut cursor);
                    i += 1;
                }
                while len < field_width {
                    put(buf, b' ', &mut cursor);
                    field_width -= 1;
                }
                let _ = &mut len;
                fi += 1;
                continue 'main;
            }
            b'p' => {
                if field_width == -1 {
                    field_width = 2 * core::mem::size_of::<usize>() as i32;
                    flags |= ZEROPAD;
                }
                let v = args.get(ai).map_or(0, Arg::as_i64);
                ai += 1;
                number_append(buf, &mut cursor, v, 16, field_width, precision, flags);
                fi += 1;
                continue 'main;
            }
            b'n' => {
                match args.get(ai) {
                    Some(Arg::NLong(cell)) if qualifier == b'l' => cell.set(cursor as i64),
                    Some(Arg::NInt(cell)) => cell.set(cursor as i32),
                    _ => {}
                }
                ai += 1;
                fi += 1;
                continue 'main;
            }
            b'%' => {
                put(buf, b'%', &mut cursor);
                fi += 1;
                continue 'main;
            }
            b'o' => base = 8,
            b'x' => {
                flags |= SMALL;
                base = 16;
            }
            b'X' => base = 16,
            b'd' | b'i' => flags |= SIGN,
            b'u' => {}
            _ => {
                put(buf, b'%', &mut cursor);
                if conv != 0 {
                    put(buf, conv, &mut cursor);
                    fi += 1;
                }
                // else: C does `--fmt;` so the trailing '%' ends the loop.
                continue 'main;
            }
        }

        // integer conversion: pull the value with the right width/sign,
        // matching printf.c lines 270-279.
        let raw = args.get(ai).map_or(0, Arg::as_i64);
        ai += 1;
        let num: i64 = if qualifier == b'l' {
            raw
        } else if qualifier == b'h' {
            if flags & SIGN != 0 {
                (raw as i16) as i64
            } else {
                (raw as u16) as i64
            }
        } else if flags & SIGN != 0 {
            (raw as i32) as i64
        } else {
            (raw as u32) as i64
        };
        number_append(buf, &mut cursor, num, base, field_width, precision, flags);
        fi += 1;
    }

    // `*str = '\0';` — terminate when there is room; not counted.
    if cursor < buf.len() {
        buf[cursor] = 0;
    }
    cursor.min(buf.len())
}

/// `sprintf(buf, fmt, ...)` — printf.c lines 286-295. C varargs become the
/// typed `args` slice; otherwise identical to `vsprintf`.
pub fn sprintf(buf: &mut [u8], fmt: &[u8], args: &[Arg]) -> usize {
    vsprintf(buf, fmt, args)
}

/// `printf(fmt, ...)` — printf.c lines 297-310. Formats into a 1024-byte
/// buffer and hands it to `emit`, which the real boot wires to
/// `crate::arch::x86::boot::tty::puts`. Returns the number of bytes
/// formatted (`printed`).
pub fn printf<F: FnMut(&[u8])>(fmt: &[u8], args: &[Arg], mut emit: F) -> usize {
    let mut printf_buf = [0u8; 1024];
    let printed = vsprintf(&mut printf_buf, fmt, args);
    emit(&printf_buf[..printed.min(printf_buf.len())]);
    printed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(f: &[u8], args: &[Arg]) -> alloc::string::String {
        let mut b = [0u8; 128];
        let n = vsprintf(&mut b, f, args);
        alloc::string::String::from_utf8(b[..n].to_vec()).unwrap()
    }

    extern crate alloc;

    #[test]
    fn skip_atoi_parses_then_advances_cursor() {
        let s = b"123x";
        let mut i = 0;
        assert_eq!(skip_atoi(s, &mut i), 123);
        assert_eq!(i, 3);
    }

    #[test]
    fn flag_constants_match_printf_c() {
        assert_eq!(ZEROPAD, 1);
        assert_eq!(SIGN, 2);
        assert_eq!(PLUS, 4);
        assert_eq!(SPACE, 8);
        assert_eq!(LEFT, 16);
        assert_eq!(SMALL, 32);
        assert_eq!(SPECIAL, 64);
    }

    #[test]
    fn number_formats_negative_signed_decimal() {
        let mut b = [0u8; 16];
        let n = number(&mut b, -42, 10, 0, 0, SIGN);
        assert_eq!(&b[..n], b"-42");
    }

    #[test]
    fn number_emits_special_0x_prefix() {
        let mut b = [0u8; 16];
        let n = number(&mut b, 0x10, 16, 0, 0, SPECIAL | SMALL);
        assert_eq!(&b[..n], b"0x10");
    }

    #[test]
    fn vsprintf_copies_literal_text() {
        assert_eq!(fmt(b"hello", &[]), "hello");
    }

    #[test]
    fn vsprintf_signed_and_unsigned_decimal() {
        assert_eq!(fmt(b"%d", &[Arg::Int(-42)]), "-42");
        assert_eq!(fmt(b"%u", &[Arg::Uint(42)]), "42");
        // %u of a value passed as unsigned int truncates to 32 bits.
        assert_eq!(fmt(b"%u", &[Arg::Uint(0xffff_ffff)]), "4294967295");
    }

    #[test]
    fn vsprintf_hex_case_follows_x_vs_X() {
        assert_eq!(fmt(b"%x", &[Arg::Uint(0xDEAD_BEEF)]), "deadbeef");
        assert_eq!(fmt(b"%X", &[Arg::Uint(0xDEAD_BEEF)]), "DEADBEEF");
        assert_eq!(fmt(b"%#x", &[Arg::Uint(0x10)]), "0x10");
    }

    #[test]
    fn vsprintf_zero_pad_and_left_justify_widths() {
        assert_eq!(fmt(b"%05d", &[Arg::Int(7)]), "00007");
        assert_eq!(fmt(b"%-5d|", &[Arg::Int(7)]), "7    |");
        assert_eq!(fmt(b"%5d", &[Arg::Int(7)]), "    7");
    }

    #[test]
    fn vsprintf_char_and_percent_literal() {
        assert_eq!(
            fmt(b"%c%c", &[Arg::Int(b'O' as i32), Arg::Int(b'k' as i32)]),
            "Ok"
        );
        assert_eq!(fmt(b"100%%", &[]), "100%");
    }

    #[test]
    fn vsprintf_string_with_precision_and_width() {
        assert_eq!(fmt(b"%s", &[Arg::Str(b"boot")]), "boot");
        assert_eq!(fmt(b"%.3s", &[Arg::Str(b"bootloader")]), "boo");
        assert_eq!(fmt(b"%6s", &[Arg::Str(b"hi")]), "    hi");
    }

    #[test]
    fn vsprintf_star_width_consumes_an_argument() {
        assert_eq!(fmt(b"%*d", &[Arg::Int(4), Arg::Int(7)]), "   7");
        // negative star width turns into left-justify.
        assert_eq!(fmt(b"%*d|", &[Arg::Int(-4), Arg::Int(7)]), "7   |");
    }

    #[test]
    fn vsprintf_long_qualifier_keeps_full_value() {
        assert_eq!(fmt(b"%lu", &[Arg::Ulong(4_000_000_000)]), "4000000000");
    }

    #[test]
    fn vsprintf_percent_n_writes_back_byte_count() {
        let count = Cell::new(0i32);
        let s = fmt(b"abc%n", &[Arg::NInt(&count)]);
        assert_eq!(s, "abc");
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn printf_emits_formatted_bytes_to_sink() {
        let mut captured = alloc::vec::Vec::new();
        let n = printf(b"id=%d", &[Arg::Int(5)], |b| captured.extend_from_slice(b));
        assert_eq!(n, 4);
        assert_eq!(&captured, b"id=5");
    }
}
