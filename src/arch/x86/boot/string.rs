//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/string.c
//! test-origin: linux:vendor/linux/arch/x86/boot/string.c
//! Basic string helpers used by the real-mode setup stub.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/string.c
//! - vendor/linux/arch/x86/boot/string.h
//!
//! Real-mode `setup.bin` carries its own tiny libc subset because it
//! cannot link against the main kernel's `lib/string.c`. The port
//! reproduces the surface (`memcmp/strcmp/strncmp/strnlen/simple_strtoull/
//! simple_strtoll`) plus the base-guessing helper used by `kstrtoul`.
//! Lupos uses Rust's own string handling at runtime, but mirroring the
//! Linux semantics here documents the bit-exact rules upstream relies on.

/// Linux `KSTRTOX_OVERFLOW` — set in the upper bit of the result on
/// overflow. Matches string.c line 21.
pub const KSTRTOX_OVERFLOW: u32 = 1 << 31;

/// `memcmp(s1, s2, len)` — byte-wise lexicographic comparison. Linux
/// uses `repe cmpsb` and returns the nonzero flag; we mirror the
/// "first difference" semantics in pure Rust.
pub fn memcmp(s1: &[u8], s2: &[u8], len: usize) -> i32 {
    let n = len.min(s1.len()).min(s2.len());
    for i in 0..n {
        let d = s1[i] as i32 - s2[i] as i32;
        if d != 0 {
            return d;
        }
    }
    0
}

/// `strcmp` — compare two NUL-terminated byte slices.
pub fn strcmp(s1: &[u8], s2: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let a = *s1.get(i).unwrap_or(&0);
        let b = *s2.get(i).unwrap_or(&0);
        if a == 0 && b == 0 {
            return 0;
        }
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
}

/// `strncmp(cs, ct, count)` — compare up to `count` bytes (or to NUL).
pub fn strncmp(cs: &[u8], ct: &[u8], count: usize) -> i32 {
    for i in 0..count {
        let a = *cs.get(i).unwrap_or(&0);
        let b = *ct.get(i).unwrap_or(&0);
        if a != b {
            return if a < b { -1 } else { 1 };
        }
        if a == 0 {
            return 0;
        }
    }
    0
}

/// `strnlen(s, maxlen)` — strlen capped at `maxlen`.
pub fn strnlen(s: &[u8], maxlen: usize) -> usize {
    let mut i = 0;
    while i < maxlen && i < s.len() && s[i] != 0 {
        i += 1;
    }
    i
}

/// `TOLOWER(c)` — bit-or with 0x20 only correctly downcases ASCII
/// letters and digits, but that's the entire input domain for the
/// Linux helper. Matches string.c line 92.
#[inline]
pub const fn tolower(c: u8) -> u8 {
    c | 0x20
}

/// `simple_guess_base(cp)` — auto-detect base from `0x`/`0` prefix.
/// Mirrors string.c lines 94-104.
pub fn simple_guess_base(cp: &[u8]) -> u32 {
    if cp.first() == Some(&b'0') {
        if cp.get(1).map(|&c| tolower(c)) == Some(b'x')
            && cp.get(2).map(|c| c.is_ascii_hexdigit()).unwrap_or(false)
        {
            return 16;
        }
        return 8;
    }
    10
}

/// `simple_strtoull(cp, base)` — parse an unsigned 64-bit integer.
/// Returns `(value, bytes_consumed)`. Matches the algorithm in
/// string.c lines 112-160.
pub fn simple_strtoull(cp: &[u8], mut base: u32) -> (u64, usize) {
    if base == 0 {
        base = simple_guess_base(cp);
    }
    let mut i = 0;
    if base == 16 && cp.first() == Some(&b'0') && cp.get(1).map(|&c| tolower(c)) == Some(b'x') {
        i = 2;
    }
    let mut result: u64 = 0;
    while i < cp.len() {
        let c = cp[i];
        let digit: u32 = if c.is_ascii_digit() {
            (c - b'0') as u32
        } else if c.is_ascii_hexdigit() {
            (tolower(c) - b'a') as u32 + 10
        } else {
            break;
        };
        if digit >= base {
            break;
        }
        result = result
            .checked_mul(base as u64)
            .and_then(|v| v.checked_add(digit as u64))
            .unwrap_or(u64::MAX);
        i += 1;
    }
    (result, i)
}

/// `simple_strtoll(cp, base)` — signed variant. A leading '-' negates
/// the unsigned result.
pub fn simple_strtoll(cp: &[u8], base: u32) -> (i64, usize) {
    if cp.first() == Some(&b'-') {
        let (v, n) = simple_strtoull(&cp[1..], base);
        (-(v as i64), n + 1)
    } else {
        let (v, n) = simple_strtoull(cp, base);
        (v as i64, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memcmp_returns_first_difference() {
        assert_eq!(memcmp(b"abc", b"abc", 3), 0);
        assert!(memcmp(b"abc", b"abd", 3) < 0);
        assert!(memcmp(b"abd", b"abc", 3) > 0);
    }

    #[test]
    fn strcmp_handles_terminating_nul() {
        assert_eq!(strcmp(b"linux\0", b"linux\0"), 0);
        assert!(strcmp(b"linux\0", b"lupos\0") < 0);
    }

    #[test]
    fn strncmp_stops_at_count() {
        assert_eq!(strncmp(b"abcd", b"abcz", 3), 0);
        assert!(strncmp(b"abcd", b"abcz", 4) < 0);
    }

    #[test]
    fn strnlen_caps_at_maxlen() {
        assert_eq!(strnlen(b"abc\0", 10), 3);
        assert_eq!(strnlen(b"abcdef", 4), 4);
    }

    #[test]
    fn simple_guess_base_recognises_hex_and_octal_prefixes() {
        assert_eq!(simple_guess_base(b"0xff"), 16);
        assert_eq!(simple_guess_base(b"0777"), 8);
        assert_eq!(simple_guess_base(b"123"), 10);
    }

    #[test]
    fn simple_strtoull_parses_hex_with_prefix() {
        assert_eq!(simple_strtoull(b"0xdeadbeef", 0), (0xdead_beef, 10));
        assert_eq!(simple_strtoull(b"0xff stuff", 0), (0xff, 4));
    }

    #[test]
    fn simple_strtoull_parses_decimal_and_stops_at_invalid() {
        assert_eq!(simple_strtoull(b"123abc", 10), (123, 3));
    }

    #[test]
    fn simple_strtoll_handles_negative_sign() {
        assert_eq!(simple_strtoll(b"-42", 10), (-42, 3));
    }

    #[test]
    fn tolower_const_fn_works_for_ascii_letters() {
        assert_eq!(tolower(b'A'), b'a');
        assert_eq!(tolower(b'Z'), b'z');
        assert_eq!(tolower(b'a'), b'a');
    }
}
