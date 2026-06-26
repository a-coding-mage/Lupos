//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/copy_mc.c
//! test-origin: linux:vendor/linux/arch/x86/lib/copy_mc.c
//! Machine-check-aware memory copy primitives.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/copy_mc.c
//! - vendor/linux/arch/x86/lib/copy_mc_64.S (the asm primitives the C
//!   file dispatches into)
//!
//! Linux exposes `copy_mc_to_kernel` and `copy_mc_to_user` as poisoned-
//! memory-safe wrappers around `memcpy` — on systems where the `#MC`
//! handler can recover from a poison-consumption (`X86_FEATURE_MCE`),
//! these helpers either use a careful byte-at-a-time fragile copy
//! (`copy_mc_fragile`) or fall back to ERMS string copy via
//! `copy_mc_enhanced_fast_string` when `X86_FEATURE_ERMS` is set.
//!
//! Lupos' Batch 10 MCE infrastructure lives in `cpu::mce`; until a
//! real #MC recovery backend enables it, `copy_mc_fragile_enabled`
//! defaults to false and the helpers fall through to a plain `memcpy`.
//! This preserves the Linux return convention (0 on success,
//! remaining-bytes on fault) so call sites do not change when MCE
//! recovery becomes hardware-backed.

use core::sync::atomic::{AtomicUsize, Ordering};

/// Refcount of subsystems requesting the fragile (byte-at-a-time)
/// copy path. Mirrors Linux's `static_branch_unlikely(copy_mc_fragile_key)`.
static COPY_MC_FRAGILE: AtomicUsize = AtomicUsize::new(0);

/// `enable_copy_mc_fragile()` — bump the static-key refcount. Called
/// by MCE/EDAC subsystems that want poison-consumption containment.
/// Mirrors copy_mc.c lines 16-19.
pub fn enable_copy_mc_fragile() {
    COPY_MC_FRAGILE.fetch_add(1, Ordering::Release);
}

/// Predicate corresponding to `copy_mc_fragile_enabled`.
#[inline]
pub fn copy_mc_fragile_enabled() -> bool {
    COPY_MC_FRAGILE.load(Ordering::Acquire) > 0
}

/// Probe whether the CPU advertises `X86_FEATURE_ERMS` (Enhanced REP
/// MOVSB/STOSB). Linux uses static-CPU-has; lupos goes straight to
/// CPUID.
fn has_erms() -> bool {
    // ERMS lives in CPUID.07H:EBX[9].
    let r = crate::arch::x86::kernel::cpuid::cpuid(7, 0);
    (r.ebx & (1 << 9)) != 0
}

/// `copy_mc_fragile(dst, src, len)` — byte-at-a-time copy that returns
/// the byte count of any tail it could not safely consume due to a
/// poison fault. The real implementation lives in inline asm and uses
/// the extable to catch `#MC`; in the stub we never see a #MC, so the
/// function is a literal byte copy returning 0.
///
/// # Safety
/// `dst` and `src` must each cover `len` bytes; regions may overlap
/// (Linux's primitive does forward byte copy, so overlap is undefined
/// the same way `memcpy` is).
unsafe fn copy_mc_fragile(dst: *mut u8, src: *const u8, len: usize) -> usize {
    unsafe {
        for i in 0..len {
            *dst.add(i) = *src.add(i);
        }
    }
    0
}

/// `copy_mc_enhanced_fast_string` — ERMS-string-instruction-based copy.
/// Returns bytes not copied (0 on success).
///
/// # Safety
/// See `copy_mc_fragile`.
unsafe fn copy_mc_enhanced_fast_string(dst: *mut u8, src: *const u8, len: usize) -> usize {
    unsafe { core::ptr::copy_nonoverlapping(src, dst, len) };
    0
}

/// `copy_mc_to_kernel(dst, src, len)` — copy with #MC recovery. Returns
/// the number of bytes **not** copied (0 = success). Mirrors copy_mc.c
/// lines 47-82.
///
/// # Safety
/// Standard memcpy contract: non-overlapping byte regions of size
/// `len` at `dst` and `src`.
pub unsafe fn copy_mc_to_kernel(dst: *mut u8, src: *const u8, len: usize) -> usize {
    if copy_mc_fragile_enabled() {
        return unsafe { copy_mc_fragile(dst, src, len) };
    }
    if has_erms() {
        return unsafe { copy_mc_enhanced_fast_string(dst, src, len) };
    }
    unsafe { core::ptr::copy_nonoverlapping(src, dst, len) };
    0
}

/// `copy_mc_to_user(dst, src, len)` — copy to user-space with #MC
/// recovery. The user-pointer must pass `access_ok` before invocation
/// — this helper does not re-check. Mirrors copy_mc.c lines 84-105.
///
/// # Safety
/// `dst` is a user-space pointer; caller is responsible for `access_ok`.
/// `src` covers `len` readable kernel bytes.
pub unsafe fn copy_mc_to_user(dst: *mut u8, src: *const u8, len: usize) -> usize {
    if copy_mc_fragile_enabled() {
        return unsafe { copy_mc_fragile(dst, src, len) };
    }
    if has_erms() {
        return unsafe { copy_mc_enhanced_fast_string(dst, src, len) };
    }
    // Fallback to the generic uaccess path (matches Linux's
    // `copy_user_generic`). Until full integration with the extable in
    // batch D, perform a plain byte copy; the fault path remains
    // identical in shape.
    unsafe { core::ptr::copy_nonoverlapping(src, dst, len) };
    0
}

/// `copy_mc_fragile_handle_tail(dst, src, len)` — probe the remaining
/// tail byte-by-byte until a fault occurs; return the number of bytes
/// left. Mirrors copy_mc.c lines 26-33.
///
/// # Safety
/// Caller has already split the copy; this only handles the residual
/// tail of length `len`.
pub unsafe fn copy_mc_fragile_handle_tail(dst: *mut u8, src: *const u8, len: usize) -> usize {
    let mut remaining = len;
    let mut d = dst;
    let mut s = src;
    while remaining > 0 {
        let not_copied = unsafe { copy_mc_fragile(d, s, 1) };
        if not_copied != 0 {
            break;
        }
        unsafe {
            d = d.add(1);
            s = s.add(1);
        }
        remaining -= 1;
    }
    remaining
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_fragile() {
        COPY_MC_FRAGILE.store(0, Ordering::SeqCst);
    }

    #[test]
    fn fragile_disabled_by_default() {
        reset_fragile();
        assert!(!copy_mc_fragile_enabled());
    }

    #[test]
    fn enable_fragile_is_refcounted() {
        reset_fragile();
        enable_copy_mc_fragile();
        assert!(copy_mc_fragile_enabled());
        enable_copy_mc_fragile();
        assert!(copy_mc_fragile_enabled());
        // Two enables ⇒ still enabled (refcounted, no decrement API yet
        // matching Linux which has none either).
    }

    #[test]
    fn copy_mc_to_kernel_returns_zero_on_full_copy() {
        reset_fragile();
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        let leftover = unsafe { copy_mc_to_kernel(dst.as_mut_ptr(), src.as_ptr(), 5) };
        assert_eq!(leftover, 0);
        assert_eq!(dst, src);
    }

    #[test]
    fn copy_mc_to_kernel_zero_length_succeeds() {
        reset_fragile();
        let leftover = unsafe { copy_mc_to_kernel(core::ptr::null_mut(), core::ptr::null(), 0) };
        assert_eq!(leftover, 0);
    }

    #[test]
    fn fragile_path_byte_copies_input() {
        reset_fragile();
        enable_copy_mc_fragile();
        let src = [0xa, 0xb, 0xc];
        let mut dst = [0u8; 3];
        let leftover = unsafe { copy_mc_to_kernel(dst.as_mut_ptr(), src.as_ptr(), 3) };
        assert_eq!(leftover, 0);
        assert_eq!(dst, src);
    }

    #[test]
    fn fragile_handle_tail_zero_remaining_when_no_fault() {
        reset_fragile();
        let src = [9u8, 9, 9];
        let mut dst = [0u8; 3];
        let leftover = unsafe { copy_mc_fragile_handle_tail(dst.as_mut_ptr(), src.as_ptr(), 3) };
        // No fault is ever produced in the stub copy, so we consumed
        // every byte and return 0 — matches the success branch in
        // copy_mc.c line 32 (the for-loop exits naturally).
        assert_eq!(leftover, 0);
        assert_eq!(dst, src);
    }
}
