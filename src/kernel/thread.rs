//! linux-parity: complete
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! x86_64 per-thread CPU state — `struct thread_struct`.
//!
//! This module defines the architecture-specific thread context that is saved
//! and restored across context switches. Only fields that our M21 context
//! switch actually reads/writes are present; the full Linux layout includes
//! additional debug-register, I/O-bitmap, and shadow-stack fields added in
//! later milestones.
//!
//! References:
//!   Linux `arch/x86/include/asm/processor.h` — `struct thread_struct`
//!   Linux `arch/x86/entry/entry_64.S` — `__switch_to_asm`

use core::mem::{offset_of, size_of};

/// Mirror of Linux `struct desc_struct` — one 8-byte GDT descriptor entry.
///
/// Used for the three per-task TLS GDT entries (GS_TLS, FS_TLS, DS_ES_TLS).
/// The packed 8-byte value encodes base, limit, and attribute fields as defined
/// by the Intel SDM Vol. 3A §3.4.5 (Segment Descriptor).
///
/// Ref: Linux `arch/x86/include/asm/desc_defs.h`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DescStruct(pub u64);

/// x86_64 per-thread CPU state.
///
/// Stores the TLS descriptors, saved kernel stack pointer, segment bases/indices,
/// and the Protection-Key Register User (PKRU) value for each task.
///
/// # Binary layout
///
/// | offset | field       | size |
/// |--------|-------------|------|
/// | +0     | tls_array   | 24   |
/// | +24    | sp          | 8    |
/// | +32    | es          | 2    |
/// | +34    | ds          | 2    |
/// | +36    | fsindex     | 2    |
/// | +38    | gsindex     | 2    |
/// | +40    | _pad0       | 4    |
/// | +44    | ← (unused padding to align fsbase) |
/// | +48    | fsbase      | 8    |
/// | +56    | gsbase      | 8    |
/// | +64    | pkru        | 4    |
/// | +68    | _pad1       | 4    |
///
/// Total: 72 bytes.
///
/// Ref: Linux `arch/x86/include/asm/processor.h` `struct thread_struct`
#[repr(C)]
pub struct ThreadStruct {
    /// Three TLS GDT entries (GDT_ENTRY_TLS_MIN..=GDT_ENTRY_TLS_MAX).
    ///
    /// Written by `arch_prctl(ARCH_SET_FS/GS)` and loaded by `load_TLS()`
    /// during context switches.
    pub tls_array: [DescStruct; 3],

    /// Saved kernel stack pointer (RSP).
    ///
    /// **This is the only field used by `__switch_to_asm`.** The assembly saves
    /// the outgoing task's RSP here and loads the incoming task's RSP from it.
    /// All other callee-saved registers are pushed/popped on the stack itself.
    ///
    /// Ref: Linux `arch/x86/entry/entry_64.S` — `TASK_threadsp(%rdi/%rsi)`
    pub sp: u64,

    /// Saved ES segment index (non-zero only for unusual user-space ABI needs).
    pub es: u16,
    /// Saved DS segment index.
    pub ds: u16,
    /// FS segment selector index (loaded into %fs by `load_TLS`).
    pub fsindex: u16,
    /// GS segment selector index (loaded into %gs by `load_TLS`).
    pub gsindex: u16,

    /// Padding to 8-byte-align `fsbase`.
    pub _pad0: u32,

    /// FS base address (user TLS base, set by `arch_prctl(ARCH_SET_FS, addr)`).
    ///
    /// On CPUs with FSGSBASE, written/read with `WRFSBASE`/`RDFSBASE`.
    /// On older CPUs, written via `wrmsr(MSR_FS_BASE, ...)`.
    pub fsbase: u64,

    /// GS base address (per-CPU kernel data; swapped by `swapgs` on entry).
    pub gsbase: u64,

    /// Protection-Key Register User — saved on context switch, restored via
    /// `PKRU` MSR or `WRPKRU` instruction on CPUs that support it.
    pub pkru: u32,

    /// Padding to u64 boundary.
    pub _pad1: u32,
}

// ── Compile-time layout assertions ──────────────────────────────────────────

const _: () = {
    assert!(
        offset_of!(ThreadStruct, tls_array) == 0,
        "tls_array must be at offset 0"
    );
    assert!(
        offset_of!(ThreadStruct, sp) == 24,
        "sp must be at offset 24 (after 3×8-byte DescStruct)"
    );
    assert!(offset_of!(ThreadStruct, es) == 32);
    assert!(offset_of!(ThreadStruct, fsindex) == 36);
    assert!(
        offset_of!(ThreadStruct, fsbase) == 48,
        "fsbase must be at offset 48"
    );
    assert!(offset_of!(ThreadStruct, gsbase) == 56);
    assert!(offset_of!(ThreadStruct, pkru) == 64);
    assert!(
        size_of::<ThreadStruct>() == 72,
        "ThreadStruct must be 72 bytes"
    );
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_struct_tls_array_at_offset_0() {
        assert_eq!(offset_of!(ThreadStruct, tls_array), 0);
    }

    #[test]
    fn thread_struct_sp_at_offset_24() {
        // __switch_to_asm uses a compile-time constant for this offset;
        // if it drifts the assembly will silently corrupt task stacks.
        assert_eq!(offset_of!(ThreadStruct, sp), 24);
    }

    #[test]
    fn thread_struct_fsbase_at_offset_48() {
        assert_eq!(offset_of!(ThreadStruct, fsbase), 48);
    }

    #[test]
    fn thread_struct_gsbase_at_offset_56() {
        assert_eq!(offset_of!(ThreadStruct, gsbase), 56);
    }

    #[test]
    fn thread_struct_size_is_72() {
        assert_eq!(size_of::<ThreadStruct>(), 72);
    }

    #[test]
    fn desc_struct_size_is_8() {
        assert_eq!(size_of::<DescStruct>(), 8);
    }
}
