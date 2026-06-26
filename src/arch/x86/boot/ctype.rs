//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/ctype.h
//! test-origin: linux:vendor/linux/arch/x86/boot/ctype.h
//! Boot-local ctype predicates for the real-mode setup stub.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/ctype.h
//!
//! `setup.bin` cannot pull in the kernel's full `<linux/ctype.h>` table,
//! so it carries these two tiny inline predicates. They are deliberately
//! kept separate from `src/lib/ctype.rs`: the boot copies have the exact,
//! self-contained ASCII semantics the Linux setup code relies on. Linux
//! declares them returning `int` (0/1); we return `bool`, which is the
//! same observable truth value at every call site in the setup stub.

/// `isdigit(ch)` — true for ASCII decimal digits `'0'..='9'`.
///
/// Mirrors ctype.h lines 5-8 exactly: `(ch >= '0') && (ch <= '9')`.
/// Takes `i32` because the C function takes `int` (the argument may be a
/// promoted byte or `EOF`-like sentinel), and the comparison is done in
/// signed `int` space upstream.
#[inline]
pub fn isdigit(ch: i32) -> bool {
    ch >= '0' as i32 && ch <= '9' as i32
}

/// `isxdigit(ch)` — true for ASCII hexadecimal digits: `0-9`, `a-f`,
/// `A-F`.
///
/// Mirrors ctype.h lines 10-19: first defer to `isdigit`, then test the
/// lowercase `a..f` range, then the uppercase `A..F` range. The order is
/// irrelevant to the result but kept to match the Linux source.
#[inline]
pub fn isxdigit(ch: i32) -> bool {
    if isdigit(ch) {
        return true;
    }
    if ch >= 'a' as i32 && ch <= 'f' as i32 {
        return true;
    }
    ch >= 'A' as i32 && ch <= 'F' as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isdigit_accepts_ascii_decimal_digits() {
        for c in b'0'..=b'9' {
            assert!(isdigit(c as i32), "{} should be a digit", c as char);
        }
    }

    #[test]
    fn isdigit_rejects_boundaries_and_letters() {
        // One below '0' and one above '9' must fail.
        assert!(!isdigit('0' as i32 - 1)); // '/'
        assert!(!isdigit('9' as i32 + 1)); // ':'
        assert!(!isdigit('a' as i32));
        assert!(!isdigit('A' as i32));
        assert!(!isdigit(-1)); // EOF-like sentinel
    }

    #[test]
    fn isxdigit_accepts_digits_and_both_letter_cases() {
        for c in b'0'..=b'9' {
            assert!(isxdigit(c as i32));
        }
        for c in b'a'..=b'f' {
            assert!(isxdigit(c as i32));
        }
        for c in b'A'..=b'F' {
            assert!(isxdigit(c as i32));
        }
    }

    #[test]
    fn isxdigit_rejects_non_hex_boundaries() {
        // 'g'/'G' are just past the hex letter range; '`'/'@' just before.
        assert!(!isxdigit('g' as i32));
        assert!(!isxdigit('G' as i32));
        assert!(!isxdigit('a' as i32 - 1)); // '`'
        assert!(!isxdigit('A' as i32 - 1)); // '@'
        assert!(!isxdigit('f' as i32 + 1)); // 'g'
        assert!(!isxdigit('F' as i32 + 1)); // 'G'
    }
}
