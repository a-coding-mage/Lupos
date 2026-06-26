//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/iomem.c
//! test-origin: linux:vendor/linux/arch/x86/lib/iomem.c
//! MMIO byte-copy helpers (`memcpy_fromio`, `memcpy_toio`, `memset_io`).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/iomem.c
//!
//! Linux supports two implementations:
//! - the optimized "string" variant using `REP MOVSL/MOVSW/MOVSB` for
//!   bulk transfers, used on bare metal where every load/store goes
//!   directly to MMIO.
//! - an unrolled byte-at-a-time variant used by confidential guests
//!   (SEV-ES, TDX) where string-IO does not trap into the host correctly
//!   (`CC_ATTR_GUEST_UNROLL_STRING_IO`).
//!
//! The choice is made via a runtime feature query — lupos exposes the
//! same `unroll_string_io_required()` predicate so SEV/TDX builds can
//! pick the unrolled path through the Batch 9 `coco/` ports.

use core::ptr::{read_volatile, write_volatile};

/// Returns true on platforms where REP MOVS string I/O does not work
/// correctly and must be replaced by a byte loop. Mirrors
/// `cc_platform_has(CC_ATTR_GUEST_UNROLL_STRING_IO)` from iomem.c.
///
/// Lupos does not run as a confidential guest yet, so this
/// is always false on bare metal — the predicate exists so the iomem
/// helpers can be flipped over without changing call sites.
#[inline]
pub fn unroll_string_io_required() -> bool {
    false
}

/// Byte-at-a-time unrolled copy from MMIO. Mirrors
/// `unrolled_memcpy_fromio()` in iomem.c lines 67-75.
///
/// # Safety
/// `from` must point to `n` valid bytes of MMIO; reads are volatile.
unsafe fn unrolled_memcpy_fromio(to: *mut u8, from: *const u8, n: usize) {
    for i in 0..n {
        unsafe {
            *to.add(i) = read_volatile(from.add(i));
        }
    }
}

/// Byte-at-a-time unrolled copy to MMIO. Mirrors
/// `unrolled_memcpy_toio()`.
///
/// # Safety
/// `to` must point to `n` valid bytes of MMIO; writes are volatile.
unsafe fn unrolled_memcpy_toio(to: *mut u8, from: *const u8, n: usize) {
    for i in 0..n {
        unsafe {
            write_volatile(to.add(i), *from.add(i));
        }
    }
}

/// Byte-at-a-time unrolled memset to MMIO. Mirrors
/// `unrolled_memset_io()`.
///
/// # Safety
/// `a` must point to `c` valid bytes of MMIO.
unsafe fn unrolled_memset_io(a: *mut u8, b: u8, c: usize) {
    for i in 0..c {
        unsafe {
            write_volatile(a.add(i), b);
        }
    }
}

/// String-IO copy from MMIO. Linux uses REP MOVS with alignment fixups
/// (`movs("b")`, `movs("w")`, then `rep movsl`). The Rust equivalent
/// uses `core::ptr::copy_nonoverlapping` for the bulk; the resulting
/// machine code on x86_64 will lower to ERMS-string instructions when
/// the CPU supports them, matching Linux's intent.
///
/// # Safety
/// `from`/`to` must each cover `n` bytes; regions must not overlap.
unsafe fn string_memcpy_fromio(to: *mut u8, from: *const u8, n: usize) {
    if n == 0 {
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(from, to, n);
    }
}

/// String-IO copy to MMIO. Symmetric to `string_memcpy_fromio`.
///
/// # Safety
/// `to`/`from` must each cover `n` bytes; regions must not overlap.
unsafe fn string_memcpy_toio(to: *mut u8, from: *const u8, n: usize) {
    if n == 0 {
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(from, to, n);
    }
}

/// `memcpy_fromio(dst, src, n)` — copy `n` bytes from MMIO `src` into
/// kernel memory `dst`. Mirrors iomem.c lines 96-103.
///
/// # Safety
/// `to` must point to `n` writable bytes; `from` must point to `n` bytes
/// of MMIO that are safe to read.
pub unsafe fn memcpy_fromio(to: *mut u8, from: *const u8, n: usize) {
    unsafe {
        if unroll_string_io_required() {
            unrolled_memcpy_fromio(to, from, n);
        } else {
            string_memcpy_fromio(to, from, n);
        }
    }
}

/// `memcpy_toio(dst, src, n)` — copy `n` bytes from kernel memory `src`
/// into MMIO `dst`. Mirrors iomem.c lines 105-112.
///
/// # Safety
/// `to` must point to `n` bytes of MMIO; `from` must point to `n`
/// readable bytes of kernel memory.
pub unsafe fn memcpy_toio(to: *mut u8, from: *const u8, n: usize) {
    unsafe {
        if unroll_string_io_required() {
            unrolled_memcpy_toio(to, from, n);
        } else {
            string_memcpy_toio(to, from, n);
        }
    }
}

/// `memset_io(buf, b, n)` — fill `n` bytes of MMIO with byte `b`.
/// Mirrors iomem.c lines 114-126.
///
/// # Safety
/// `a` must point to `c` writable bytes of MMIO.
pub unsafe fn memset_io(a: *mut u8, b: u8, c: usize) {
    unsafe {
        if unroll_string_io_required() {
            unrolled_memset_io(a, b, c);
        } else {
            core::ptr::write_bytes(a, b, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stack-array pseudo-MMIO — the byte-at-a-time and string paths
    /// must produce identical results.
    #[test]
    fn memcpy_fromio_copies_full_buffer() {
        let mut src = [0u8; 64];
        for (i, b) in src.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut dst = [0u8; 64];
        unsafe { memcpy_fromio(dst.as_mut_ptr(), src.as_ptr(), 64) };
        assert_eq!(dst, src);
    }

    #[test]
    fn memcpy_fromio_handles_zero_length() {
        let mut dst = [0xaau8; 4];
        let src = [0u8; 4];
        unsafe { memcpy_fromio(dst.as_mut_ptr(), src.as_ptr(), 0) };
        assert_eq!(dst, [0xaa; 4]);
    }

    #[test]
    fn memcpy_toio_round_trip_matches_input() {
        let mut src = [0u8; 20];
        for (i, b) in src.iter_mut().enumerate() {
            *b = (10 + i) as u8;
        }
        let mut mmio = [0u8; 20];
        unsafe { memcpy_toio(mmio.as_mut_ptr(), src.as_ptr(), src.len()) };
        assert_eq!(mmio, src);
    }

    #[test]
    fn memset_io_fills_every_byte() {
        let mut mmio = [0u8; 33];
        unsafe { memset_io(mmio.as_mut_ptr(), 0x5a, 33) };
        assert!(mmio.iter().all(|&b| b == 0x5a));
    }

    #[test]
    fn unrolled_paths_match_string_paths() {
        // Confirm the byte loop is byte-identical to the string copy —
        // both must produce the same bytes for any input.
        let mut src = [0u8; 17];
        for (i, b) in src.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut a = [0u8; 17];
        let mut b = [0u8; 17];
        unsafe {
            unrolled_memcpy_fromio(a.as_mut_ptr(), src.as_ptr(), 17);
            string_memcpy_fromio(b.as_mut_ptr(), src.as_ptr(), 17);
        }
        assert_eq!(a, b);
    }

    #[test]
    fn unroll_predicate_default_is_false_on_bare_metal() {
        // Confidential-guest code flips this to true.
        assert!(!unroll_string_io_required());
    }
}
