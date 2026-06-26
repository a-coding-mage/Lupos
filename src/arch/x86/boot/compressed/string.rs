//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/string.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/string.c
//! Compressed-kernel `memcpy/memset/memmove` plus parent string shim.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/string.c
//!
//! The decompression stub provides its own `memcpy/memset/memmove`
//! because the kernel's `lib/string.c` isn't linked yet and gcc's
//! built-ins may pull in FPU operations the stub can't tolerate. The
//! optimised path uses `rep movsq` (x86_64) or `rep movsl` (x86_32);
//! we mirror the dispatch shape and use `core::ptr::copy_nonoverlapping`
//! for the non-asm fallback so host tests are exercisable.

/// `memset(s, c, n)` — byte fill. Mirrors string.c lines 43-51.
///
/// # Safety
/// `s` must point to `n` writable bytes.
pub unsafe fn memset(s: *mut u8, c: u8, n: usize) -> *mut u8 {
    unsafe { core::ptr::write_bytes(s, c, n) };
    s
}

/// Internal optimised forward copy. Mirrors `____memcpy()` in
/// string.c lines 27-41. The `rep movsq` path is gated on the
/// production target.
///
/// # Safety
/// Source and destination must each cover `n` bytes; regions must not
/// overlap (callers go through `memcpy`/`memmove` which check).
#[inline]
unsafe fn forward_memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        // rep movsq for the quad-word bulk, rep movsb for the trailing
        // bytes — same shape as the C inline asm.
        let qwords = n >> 3;
        let bytes = n & 7;
        core::arch::asm!(
            "rep movsq",
            inout("rcx") qwords => _,
            inout("rdi") dest => _,
            inout("rsi") src => _,
            options(nostack, preserves_flags),
        );
        // After rep movsq, rdi/rsi point past the last copied qword.
        let tail_dst = dest.add(n - bytes);
        let tail_src = src.add(n - bytes);
        core::arch::asm!(
            "rep movsb",
            inout("rcx") bytes => _,
            inout("rdi") tail_dst => _,
            inout("rsi") tail_src => _,
            options(nostack, preserves_flags),
        );
    }
    #[cfg(any(not(target_arch = "x86_64"), test))]
    unsafe {
        core::ptr::copy_nonoverlapping(src, dest, n);
    }
    dest
}

/// `memmove(dest, src, n)` — overlap-safe copy. Mirrors string.c
/// lines 53-65.
///
/// # Safety
/// Both pointers must cover `n` bytes; overlapping is *allowed* (this
/// is the whole point of `memmove`).
pub unsafe fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // Linux's overlap check: forward-copy is safe iff dest <= src OR
    // dest - src >= n.
    if (dest as usize) <= (src as usize) || ((dest as usize) - (src as usize)) >= n {
        return unsafe { forward_memcpy(dest, src, n) };
    }
    // Backward copy.
    unsafe {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}

/// `memcpy(dest, src, n)` — overlap-detecting copy. Mirrors string.c
/// lines 68-75. Linux warns and falls through to `memmove` on overlap.
///
/// # Safety
/// Both pointers must cover `n` bytes. Overlap detection here matches
/// the Linux predicate; we *do not* warn (no console here) but still
/// fall through to `memmove`.
pub unsafe fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dest as usize) > (src as usize) && ((dest as usize) - (src as usize)) < n {
        return unsafe { memmove(dest, src, n) };
    }
    unsafe { forward_memcpy(dest, src, n) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memset_fills_buffer_with_byte() {
        let mut buf = [0u8; 16];
        unsafe { memset(buf.as_mut_ptr(), 0x5a, buf.len()) };
        assert!(buf.iter().all(|&b| b == 0x5a));
    }

    #[test]
    fn memcpy_copies_disjoint_regions() {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        unsafe { memcpy(dst.as_mut_ptr(), src.as_ptr(), 5) };
        assert_eq!(dst, src);
    }

    #[test]
    fn memmove_supports_overlap_going_right() {
        // [A B C D E] → shift right by 1 ⇒ [A A B C D].
        let mut buf = [b'A', b'B', b'C', b'D', b'E'];
        unsafe {
            let src = buf.as_ptr();
            let dst = buf.as_mut_ptr().add(1);
            memmove(dst, src, 4);
        }
        assert_eq!(&buf, b"AABCD");
    }

    #[test]
    fn memcpy_falls_through_to_memmove_when_overlapping() {
        let mut buf = [b'X', b'Y', b'Z'];
        unsafe {
            let src = buf.as_ptr();
            let dst = buf.as_mut_ptr().add(1);
            memcpy(dst, src, 2);
        }
        // dst[0..2] = src[0..2] via memmove → [X, X, Y].
        assert_eq!(&buf, b"XXY");
    }
}
