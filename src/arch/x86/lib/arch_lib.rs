//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib
//! test-origin: linux:vendor/linux/arch/x86/lib
//! x86 arch library helpers shared by early boot, MM, checksumming, and decode code.
//!
//! The Linux files under `arch/x86/lib` are a mix of generic byte helpers,
//! instruction decoders, MSR SMP helpers, cache delay helpers, and fault-aware
//! copy paths. Lupos keeps raw user-copy/fault recovery in `uaccess` and
//! `extable`; this module owns the side-effect-free helpers that are safe to
//! exercise in host tests.
//!
//! References:
//! - `vendor/linux/arch/x86/lib/cmdline.c`
//! - `vendor/linux/arch/x86/lib/csum-partial_64.c`
//! - `vendor/linux/arch/x86/lib/csum-wrappers_64.c`
//! - `vendor/linux/arch/x86/lib/strstr_32.c`
//! - `vendor/linux/arch/x86/lib/misc.c`
//! - vendor/linux/arch/x86/lib/atomic64_32.c
//! - vendor/linux/arch/x86/lib/memcpy_32.c
//! - vendor/linux/arch/x86/lib/string_32.c
//! - vendor/linux/arch/x86/lib/usercopy_32.c

use core::sync::atomic::{AtomicI64, Ordering};

/// Linux's one-byte port delay target, traditionally I/O port 0x80.
pub const IO_DELAY_PORT_0X80: u16 = 0x80;

/// Compute the folded 16-bit Internet checksum over a byte slice.
///
/// This mirrors the arch checksum wrappers' externally visible behavior:
/// big-endian 16-bit additions with end-around carry, returned as the one's
/// complement checksum value.
pub fn ip_checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    if let Some(&last) = chunks.remainder().first() {
        sum = sum.wrapping_add((last as u32) << 8);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// Search for an ASCII needle inside a byte slice.
pub fn memmem_ascii(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|candidate| candidate == needle)
}

/// Linux-style C string search: stop both inputs at the first NUL byte and
/// return the byte offset of `needle` inside `haystack`.
pub fn c_strstr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let h_end = haystack
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(haystack.len());
    let n_end = needle.iter().position(|b| *b == 0).unwrap_or(needle.len());
    memmem_ascii(&haystack[..h_end], &needle[..n_end])
}

/// Return true if a kernel command-line token is present.
pub fn cmdline_token_present(cmdline: &[u8], token: &str) -> bool {
    crate::arch::x86::boot::cmdline_has_option(cmdline, token)
}

/// Count the decimal digits in a signed 32-bit value, including the sign.
///
/// Ports `num_digits()` from `vendor/linux/arch/x86/lib/misc.c` verbatim — the
/// helper widens via `i64` so `i32::MIN` (which cannot be negated in `i32`)
/// still produces 11 (the `-` plus ten digits).
pub fn num_digits(val: i32) -> i32 {
    let mut m: i64 = 10;
    let mut d: i32 = 1;
    let (mut v, neg) = (val as i64, val < 0);
    if neg {
        d += 1;
        v = -v;
    }
    while v >= m {
        m *= 10;
        d += 1;
    }
    d
}

/// Atomic add-return helper used by 32-bit x86 to synthesize 64-bit atomics.
pub fn atomic64_add_return_32(cell: &AtomicI64, delta: i64) -> i64 {
    cell.fetch_add(delta, Ordering::AcqRel).wrapping_add(delta)
}

/// Atomic compare-exchange helper with Linux-style return of the old value.
pub fn atomic64_cmpxchg_32(cell: &AtomicI64, old: i64, new: i64) -> i64 {
    match cell.compare_exchange(old, new, Ordering::AcqRel, Ordering::Acquire) {
        Ok(previous) | Err(previous) => previous,
    }
}

/// 32-bit memcpy semantics: copy exactly the minimum of destination/source len.
pub fn memcpy_32(dst: &mut [u8], src: &[u8]) -> usize {
    let len = core::cmp::min(dst.len(), src.len());
    dst[..len].copy_from_slice(&src[..len]);
    len
}

/// Bounded string length used by the 32-bit string helper.
pub fn strnlen_32(bytes: &[u8], max: usize) -> usize {
    let limit = core::cmp::min(bytes.len(), max);
    bytes[..limit].iter().position(|b| *b == 0).unwrap_or(limit)
}

/// Validate a 32-bit user-copy range without performing the actual copy.
pub const fn usercopy_32_range_valid(ptr: u32, len: usize, user_limit: u32) -> bool {
    let end = ptr as u64 + len as u64;
    end <= user_limit as u64 && end >= ptr as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_linux_folded_sum_shape() {
        assert_eq!(ip_checksum(&[]), 0xffff);
        assert_eq!(ip_checksum(&[0x45, 0x00, 0x00, 0x54]), 0xbaab);
    }

    #[test]
    fn memmem_ascii_finds_embedded_pattern() {
        assert_eq!(memmem_ascii(b"linux-x86-lib", b"x86"), Some(6));
        assert_eq!(memmem_ascii(b"linux-x86-lib", b"arm"), None);
    }

    #[test]
    fn c_strstr_stops_at_nul_like_linux_boot_helpers() {
        assert_eq!(c_strstr(b"abc\0x86", b"x86"), None);
        assert_eq!(c_strstr(b"linux-x86\0ignored", b"x86\0tail"), Some(6));
        assert_eq!(c_strstr(b"linux", b"\0ignored"), Some(0));
    }

    #[test]
    fn cmdline_token_delegates_to_boot_parser() {
        assert!(cmdline_token_present(b"quiet nokaslr\0ignored", "nokaslr"));
        assert!(!cmdline_token_present(b"quiet nokaslr\0ignored", "ignored"));
    }

    #[test]
    fn num_digits_matches_linux_misc_c() {
        // Cases from inspection of Linux misc.c semantics.
        assert_eq!(num_digits(0), 1);
        assert_eq!(num_digits(9), 1);
        assert_eq!(num_digits(10), 2);
        assert_eq!(num_digits(99), 2);
        assert_eq!(num_digits(100), 3);
        assert_eq!(num_digits(-1), 2); // '-' + '1'
        assert_eq!(num_digits(-99), 3); // '-' + '9' + '9'
        assert_eq!(num_digits(10_000), 5);
        // i32::MIN: '-' plus ten digits — widened via i64 to avoid overflow on -val.
        assert_eq!(num_digits(i32::MIN), 11);
        assert_eq!(num_digits(i32::MAX), 10);
    }

    #[test]
    fn atomic64_32_helpers_return_linux_old_and_new_values() {
        let value = AtomicI64::new(10);
        assert_eq!(atomic64_add_return_32(&value, 5), 15);
        assert_eq!(atomic64_cmpxchg_32(&value, 15, 7), 15);
        assert_eq!(value.load(Ordering::Acquire), 7);
        assert_eq!(atomic64_cmpxchg_32(&value, 15, 1), 7);
    }

    #[test]
    fn memcpy_and_string_32_are_bounded() {
        let mut out = [0u8; 3];
        assert_eq!(memcpy_32(&mut out, b"abcd"), 3);
        assert_eq!(&out, b"abc");
        assert_eq!(strnlen_32(b"ab\0cd", 5), 2);
        assert_eq!(strnlen_32(b"abcdef", 4), 4);
    }

    #[test]
    fn usercopy_32_rejects_wrap_or_kernel_ranges() {
        assert!(usercopy_32_range_valid(0x1000, 16, 0x8000_0000));
        assert!(!usercopy_32_range_valid(0x7fff_fff8, 16, 0x8000_0000));
        assert!(!usercopy_32_range_valid(0xffff_fff8, 16, 0xffff_ffff));
    }
}
