//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! VMA → PTE protection mapping (`vm_get_page_prot`).
//!
//! Linux stores architecture-specific PTE protection bits in `vma->vm_page_prot`.
//! The values are derived from the low protection bits in `vma->vm_flags`
//! (VM_READ/VM_WRITE/VM_EXEC/VM_SHARED) using an arch-specific protection_map.
//!
//! For x86_64 we mirror the effective mapping from:
//! `vendor/linux/arch/x86/mm/pgprot.c` + `vendor/linux/arch/x86/include/asm/pgtable_types.h`.
//!
//! This is intentionally small and conservative: it only depends on the four
//! low protection bits and emits the same PTE flag combinations Linux uses for
//! PAGE_{NONE,READONLY,COPY,SHARED} (+ EXEC variants).

use crate::arch::x86::mm::paging::{
    _PAGE_ACCESSED, _PAGE_NX, _PAGE_PRESENT, _PAGE_PROTNONE, _PAGE_RW, _PAGE_USER,
};
use crate::mm::vm_flags::{VM_EXEC, VM_READ, VM_SHARED, VM_WRITE, VmFlags};

/// Compute the x86 PTE protection bits for a VMA with the given `vm_flags`.
///
/// Mirrors Linux `vm_get_page_prot()` for the `VM_{READ,WRITE,EXEC,SHARED}` subset.
///
/// Notes:
/// - `VM_WRITE` without `VM_SHARED` maps to a COW-safe prot (no `_PAGE_RW`),
///   matching Linux's `PAGE_COPY*` macros (writes become writable via faults).
/// - Executability is controlled purely via NX (set NX when `!VM_EXEC`).
#[inline]
pub fn vm_get_page_prot(vm_flags: VmFlags) -> u64 {
    let prot = vm_flags & (VM_READ | VM_WRITE | VM_EXEC | VM_SHARED);

    // Linux PAGE_NONE: PRESENT clear + PROTNONE marker (GLOBAL bit) + ACCESSED.
    // Ref: `vendor/linux/arch/x86/include/asm/pgtable_types.h` — PAGE_NONE.
    if (prot & (VM_READ | VM_WRITE | VM_EXEC)) == 0 {
        return _PAGE_ACCESSED | _PAGE_PROTNONE;
    }

    // Base user mapping bits.
    let mut val = _PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED;

    // NX is the only execute control on x86_64.
    if (prot & VM_EXEC) == 0 {
        val |= _PAGE_NX;
    }

    // Writable shared mappings are truly writable; private writable mappings
    // remain COW-protected (no RW bit in vm_page_prot).
    if (prot & VM_SHARED) != 0 && (prot & VM_WRITE) != 0 {
        val |= _PAGE_RW;
    }

    val
}
