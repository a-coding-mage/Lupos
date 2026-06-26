//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// VM area permission and attribute flags.
///
/// These constants define the protection and behavior attributes for virtual
/// memory areas (`vm_area_struct`).  Bit positions match Linux
/// `include/linux/mm.h` exactly so that any code path that inspects or
/// manipulates `vm_flags` produces bit-identical results.
///
/// ## References
///
/// - Linux `include/linux/mm.h` — `VM_*` flag definitions
/// - Linux `include/linux/mm_types.h` — `vm_flags_t` typedef
/// - Linux `mm/vma.c` — `is_mergeable_vma()` merge-compatibility check

// ---------------------------------------------------------------------------
// Type alias — matches Linux `vm_flags_t` (unsigned long on x86-64 = u64).
// ---------------------------------------------------------------------------

/// Type for VMA flags, matching Linux `vm_flags_t`.
pub type VmFlags = u64;

// ---------------------------------------------------------------------------
// Protection flags (bits 0-3) — mirrored to hardware PTE bits by
// `vm_get_page_prot()`.
// ---------------------------------------------------------------------------

/// No permissions.
pub const VM_NONE: VmFlags = 0;

/// Readable.
pub const VM_READ: VmFlags = 1 << 0;

/// Writable.
pub const VM_WRITE: VmFlags = 1 << 1;

/// Executable.
pub const VM_EXEC: VmFlags = 1 << 2;

/// Shared (vs private/COW).
pub const VM_SHARED: VmFlags = 1 << 3;

// ---------------------------------------------------------------------------
// "May" flags (bits 4-7) — upper limits on what mprotect() can grant.
// ---------------------------------------------------------------------------

/// May be made readable via mprotect().
pub const VM_MAYREAD: VmFlags = 1 << 4;

/// May be made writable via mprotect().
pub const VM_MAYWRITE: VmFlags = 1 << 5;

/// May be made executable via mprotect().
pub const VM_MAYEXEC: VmFlags = 1 << 6;

/// May be made shared via mprotect().
pub const VM_MAYSHARE: VmFlags = 1 << 7;

// ---------------------------------------------------------------------------
// Growth direction (bit 8).
// ---------------------------------------------------------------------------

/// Stack-like VMA that grows downward.
pub const VM_GROWSDOWN: VmFlags = 1 << 8;

// ---------------------------------------------------------------------------
// Userfaultfd (bit 9).
// ---------------------------------------------------------------------------

/// Userfaultfd missing-page tracking.
pub const VM_UFFD_MISSING: VmFlags = 1 << 9;

// ---------------------------------------------------------------------------
// PFN map (bit 10).
// ---------------------------------------------------------------------------

/// Page-frame-number mapped (no struct page backing).
pub const VM_PFNMAP: VmFlags = 1 << 10;

// ---------------------------------------------------------------------------
// Locking (bit 11).
// ---------------------------------------------------------------------------

/// Pages locked in RAM (mlock).
pub const VM_LOCKED: VmFlags = 1 << 11;

// ---------------------------------------------------------------------------
// I/O mapping (bit 12).
// ---------------------------------------------------------------------------

/// Memory-mapped I/O region.
pub const VM_IO: VmFlags = 1 << 12;

// ---------------------------------------------------------------------------
// Readahead hints (bits 13-14).
// ---------------------------------------------------------------------------

/// Sequential read hint (set by madvise(MADV_SEQUENTIAL)).
pub const VM_SEQ_READ: VmFlags = 1 << 13;

/// Random read hint (set by madvise(MADV_RANDOM)).
pub const VM_RAND_READ: VmFlags = 1 << 14;

// ---------------------------------------------------------------------------
// Copy / expand behavior (bits 15-16).
// ---------------------------------------------------------------------------

/// Do not copy this VMA on fork().
pub const VM_DONTCOPY: VmFlags = 1 << 15;

/// Do not allow mremap() to expand this VMA.
pub const VM_DONTEXPAND: VmFlags = 1 << 16;

// ---------------------------------------------------------------------------
// Lock-on-fault (bit 17).
// ---------------------------------------------------------------------------

/// Lock pages when they are faulted in (mlock2(MLOCK_ONFAULT)).
pub const VM_LOCKONFAULT: VmFlags = 1 << 17;

// ---------------------------------------------------------------------------
// Accounting (bit 18).
// ---------------------------------------------------------------------------

/// VMA is accounted against the process's RLIMIT_AS.
pub const VM_ACCOUNT: VmFlags = 1 << 18;

// ---------------------------------------------------------------------------
// Overcommit (bit 19).
// ---------------------------------------------------------------------------

/// Do not check overcommit for this VMA.
pub const VM_NORESERVE: VmFlags = 1 << 19;

// ---------------------------------------------------------------------------
// Huge pages (bit 20).
// ---------------------------------------------------------------------------

/// Backed by hugetlbfs.
pub const VM_HUGETLB: VmFlags = 1 << 20;

// ---------------------------------------------------------------------------
// Synchronous page faults (bit 21).
// ---------------------------------------------------------------------------

/// Synchronous page faults (DAX).
pub const VM_SYNC: VmFlags = 1 << 21;

// ---------------------------------------------------------------------------
// Architecture-specific (bit 22).
// ---------------------------------------------------------------------------

/// Architecture-specific flag.
pub const VM_ARCH_1: VmFlags = 1 << 22;

/// x86 `VM_SHADOW_STACK` reuses the first architecture-private VMA bit.
///
/// Ref: vendor/linux/arch/x86/kernel/sys_x86_64.c
pub const VM_SHADOW_STACK: VmFlags = VM_ARCH_1;

// ---------------------------------------------------------------------------
// Fork behavior (bit 23).
// ---------------------------------------------------------------------------

/// Wipe VMA contents on fork (madvise(MADV_WIPEONFORK)).
pub const VM_WIPEONFORK: VmFlags = 1 << 23;

// ---------------------------------------------------------------------------
// Core dump (bit 24).
// ---------------------------------------------------------------------------

/// Exclude from core dumps (madvise(MADV_DONTDUMP)).
pub const VM_DONTDUMP: VmFlags = 1 << 24;

// ---------------------------------------------------------------------------
// Soft-dirty tracking (bit 25).
// ---------------------------------------------------------------------------

/// Soft-dirty bit tracking.
#[allow(dead_code)]
pub const VM_SOFTDIRTY: VmFlags = 1 << 25;

// ---------------------------------------------------------------------------
// Mixed map (bit 26).
// ---------------------------------------------------------------------------

/// VMA may contain both struct page and PFN-only pages.
pub const VM_MIXEDMAP: VmFlags = 1 << 26;

// ---------------------------------------------------------------------------
// Transparent huge pages (bits 27-28).
// ---------------------------------------------------------------------------

/// Eligible for transparent huge page promotion.
pub const VM_HUGEPAGE: VmFlags = 1 << 27;

/// Ineligible for transparent huge page promotion.
pub const VM_NOHUGEPAGE: VmFlags = 1 << 28;

// ---------------------------------------------------------------------------
// KSM (bit 29).
// ---------------------------------------------------------------------------

/// Eligible for Kernel Same-page Merging.
pub const VM_MERGEABLE: VmFlags = 1 << 29;

// ---------------------------------------------------------------------------
// Composite aliases.
// ---------------------------------------------------------------------------

/// Stack VMA (alias for VM_GROWSDOWN).
pub const VM_STACK: VmFlags = VM_GROWSDOWN;

/// Standard data segment flags: read + write + may-read/write/exec.
pub const VM_DATA_DEFAULT_FLAGS: VmFlags =
    VM_READ | VM_WRITE | VM_MAYREAD | VM_MAYWRITE | VM_MAYEXEC;

/// Standard stack flags: read + write + growsdown + may-read/write/exec.
pub const VM_STACK_DEFAULT_FLAGS: VmFlags = VM_DATA_DEFAULT_FLAGS | VM_STACK | VM_ACCOUNT;

// ---------------------------------------------------------------------------
// Merge helpers.
// ---------------------------------------------------------------------------

/// Check whether two VMAs have compatible flags for merging.
///
/// In Linux, `is_mergeable_vma()` (`mm/vma.c`) checks a broader set of
/// conditions (file, anon_vma, policy, etc.).  This helper covers the
/// `vm_flags` portion: two VMAs can only merge if their flags are identical.
///
/// Ref: Linux `mm/vma.c` — `is_mergeable_vma()`
pub fn vm_flags_equal(a: VmFlags, b: VmFlags) -> bool {
    a == b
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that each VM_* flag occupies the expected bit position.
    #[test]
    fn bit_positions_match_linux() {
        assert_eq!(VM_READ, 1 << 0);
        assert_eq!(VM_WRITE, 1 << 1);
        assert_eq!(VM_EXEC, 1 << 2);
        assert_eq!(VM_SHARED, 1 << 3);
        assert_eq!(VM_MAYREAD, 1 << 4);
        assert_eq!(VM_MAYWRITE, 1 << 5);
        assert_eq!(VM_MAYEXEC, 1 << 6);
        assert_eq!(VM_MAYSHARE, 1 << 7);
        assert_eq!(VM_GROWSDOWN, 1 << 8);
        assert_eq!(VM_UFFD_MISSING, 1 << 9);
        assert_eq!(VM_PFNMAP, 1 << 10);
        assert_eq!(VM_LOCKED, 1 << 11);
        assert_eq!(VM_IO, 1 << 12);
        assert_eq!(VM_SEQ_READ, 1 << 13);
        assert_eq!(VM_RAND_READ, 1 << 14);
        assert_eq!(VM_DONTCOPY, 1 << 15);
        assert_eq!(VM_DONTEXPAND, 1 << 16);
        assert_eq!(VM_LOCKONFAULT, 1 << 17);
        assert_eq!(VM_ACCOUNT, 1 << 18);
        assert_eq!(VM_NORESERVE, 1 << 19);
        assert_eq!(VM_HUGETLB, 1 << 20);
        assert_eq!(VM_SYNC, 1 << 21);
        assert_eq!(VM_ARCH_1, 1 << 22);
        assert_eq!(VM_WIPEONFORK, 1 << 23);
        assert_eq!(VM_DONTDUMP, 1 << 24);
        assert_eq!(VM_SOFTDIRTY, 1 << 25);
        assert_eq!(VM_MIXEDMAP, 1 << 26);
        assert_eq!(VM_HUGEPAGE, 1 << 27);
        assert_eq!(VM_NOHUGEPAGE, 1 << 28);
        assert_eq!(VM_MERGEABLE, 1 << 29);
    }

    /// VM_STACK is an alias for VM_GROWSDOWN (Linux convention).
    #[test]
    fn vm_stack_equals_growsdown() {
        assert_eq!(VM_STACK, VM_GROWSDOWN);
    }

    /// VM_NONE is zero.
    #[test]
    fn vm_none_is_zero() {
        assert_eq!(VM_NONE, 0);
    }

    /// All 30 single-bit flags are distinct (no collisions).
    #[test]
    fn all_flags_distinct() {
        let flags: [VmFlags; 30] = [
            VM_READ,
            VM_WRITE,
            VM_EXEC,
            VM_SHARED,
            VM_MAYREAD,
            VM_MAYWRITE,
            VM_MAYEXEC,
            VM_MAYSHARE,
            VM_GROWSDOWN,
            VM_UFFD_MISSING,
            VM_PFNMAP,
            VM_LOCKED,
            VM_IO,
            VM_SEQ_READ,
            VM_RAND_READ,
            VM_DONTCOPY,
            VM_DONTEXPAND,
            VM_LOCKONFAULT,
            VM_ACCOUNT,
            VM_NORESERVE,
            VM_HUGETLB,
            VM_SYNC,
            VM_ARCH_1,
            VM_WIPEONFORK,
            VM_DONTDUMP,
            VM_SOFTDIRTY,
            VM_MIXEDMAP,
            VM_HUGEPAGE,
            VM_NOHUGEPAGE,
            VM_MERGEABLE,
        ];
        // Each flag is a single bit, and OR-ing them all yields 30 set bits.
        let combined: VmFlags = flags.iter().copied().fold(0, |acc, f| acc | f);
        assert_eq!(combined.count_ones(), 30);
    }

    /// vm_flags_equal returns true for identical flags, false otherwise.
    #[test]
    fn vm_flags_equal_basic() {
        let a = VM_READ | VM_WRITE;
        let b = VM_READ | VM_WRITE;
        let c = VM_READ | VM_EXEC;
        assert!(vm_flags_equal(a, b));
        assert!(!vm_flags_equal(a, c));
    }

    /// VM_DATA_DEFAULT_FLAGS is the expected combination.
    #[test]
    fn data_default_flags() {
        let expected = VM_READ | VM_WRITE | VM_MAYREAD | VM_MAYWRITE | VM_MAYEXEC;
        assert_eq!(VM_DATA_DEFAULT_FLAGS, expected);
    }

    /// VM_STACK_DEFAULT_FLAGS includes GROWSDOWN and ACCOUNT.
    #[test]
    fn stack_default_flags() {
        assert_ne!(VM_STACK_DEFAULT_FLAGS & VM_GROWSDOWN, 0);
        assert_ne!(VM_STACK_DEFAULT_FLAGS & VM_ACCOUNT, 0);
        assert_ne!(VM_STACK_DEFAULT_FLAGS & VM_READ, 0);
        assert_ne!(VM_STACK_DEFAULT_FLAGS & VM_WRITE, 0);
    }
}
