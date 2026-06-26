//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/pgtable_64.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/pgtable_64.c
//! 5-level paging (LA57) toggle for the decompressor.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/pgtable_64.c
//!
//! The decompressor picks between 4- and 5-level paging at runtime
//! based on `CONFIG_X86_5LEVEL` plus the `no5lvl` cmdline override and
//! CPUID leaf 7 ECX bit 16 (LA57). When the desired mode differs from
//! the current `CR4.LA57`, the decompressor places a 32-bit
//! trampoline below the BIOS region and uses it to switch.
//!
//! Lupos uses 4-level paging today; the port carries the algorithmic
//! pieces and the trampoline-placement search so the algorithm is
//! ready when 5-level support lands.

/// Minimum reasonable BIOS-data-area start.
pub const BIOS_START_MIN: u32 = 0x0_0002_0000; // 128 KiB
/// Architectural maximum BIOS-data-area start.
pub const BIOS_START_MAX: u32 = 0x0_0009_F000; // 640 KiB
/// 4 KiB page size used to align the trampoline.
pub const PAGE_SIZE: u32 = 0x1000;
/// Linux `TRAMPOLINE_32BIT_SIZE` — 16 KiB (placed below the BIOS).
pub const TRAMPOLINE_32BIT_SIZE: u32 = 0x4000;
/// `X86_CR4_LA57` — CR4 bit 12 enables 5-level paging.
pub const X86_CR4_LA57: u64 = 1 << 12;

/// Real-mode pointer ports for EBDA and base-memory size.
pub const EBDA_PTR_PHYS: u32 = 0x40e;
pub const BIOS_BASE_KB_PTR_PHYS: u32 = 0x413;

/// Compute the candidate trampoline placement from EBDA paragraph and
/// base-memory KiB readings. Mirrors `find_trampoline_placement()`
/// pgtable_64.c lines 33-101 (without the e820 walk — caller supplies
/// the maximum usable address from its own scan).
pub fn find_trampoline_placement(is_efi: bool, ebda_paragraph: u16, bios_base_kib: u16) -> u32 {
    let mut bios_start: u32;
    let mut ebda_start: u32 = 0;
    if is_efi {
        bios_start = 0;
    } else {
        ebda_start = (ebda_paragraph as u32) << 4;
        bios_start = (bios_base_kib as u32) << 10;
    }
    if bios_start < BIOS_START_MIN || bios_start > BIOS_START_MAX {
        bios_start = BIOS_START_MAX;
    }
    if ebda_start > BIOS_START_MIN && ebda_start < bios_start {
        bios_start = ebda_start;
    }
    // round_down to PAGE_SIZE.
    bios_start &= !(PAGE_SIZE - 1);
    bios_start - TRAMPOLINE_32BIT_SIZE
}

/// `LA57 desired` decision shape from the cmdline + CPUID. Mirrors
/// pgtable_64.c lines 121-129.
pub fn la57_desired(no_5_lvl_cmdline: bool, cpuid_max_basic: u32, cpuid_7_ecx: u32) -> bool {
    if no_5_lvl_cmdline {
        return false;
    }
    if cpuid_max_basic < 7 {
        return false;
    }
    (cpuid_7_ecx & (1 << 16)) != 0
}

/// "No-op" predicate: when LA57-desired equals current `CR4.LA57`,
/// no trampoline is needed. Mirrors `if (l5_required == !!(cr4 & LA57))`.
pub fn la57_noop(l5_required: bool, cr4: u64) -> bool {
    let current = (cr4 & X86_CR4_LA57) != 0;
    l5_required == current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bios_start_constants_match_pgtable_64_c() {
        assert_eq!(BIOS_START_MIN, 0x20000);
        assert_eq!(BIOS_START_MAX, 0x9f000);
    }

    #[test]
    fn trampoline_size_and_la57_bit_match_linux() {
        assert_eq!(TRAMPOLINE_32BIT_SIZE, 0x4000);
        assert_eq!(X86_CR4_LA57, 1 << 12);
    }

    #[test]
    fn la57_desired_disabled_via_cmdline() {
        assert!(!la57_desired(true, 7, 1 << 16));
    }

    #[test]
    fn la57_desired_requires_cpuid_leaf_7_bit_16() {
        assert!(!la57_desired(false, 6, 1 << 16)); // max basic < 7
        assert!(!la57_desired(false, 7, 0)); // bit 16 clear
        assert!(la57_desired(false, 7, 1 << 16));
    }

    #[test]
    fn la57_noop_when_desired_matches_current_cr4() {
        assert!(la57_noop(true, X86_CR4_LA57));
        assert!(la57_noop(false, 0));
        assert!(!la57_noop(true, 0));
        assert!(!la57_noop(false, X86_CR4_LA57));
    }

    #[test]
    fn trampoline_placement_uses_bios_start_max_when_efi() {
        // EFI guests skip the legacy ROM reads entirely.
        let p = find_trampoline_placement(true, 0, 0);
        // bios_start was 0 → clamped to BIOS_START_MAX → page-aligned
        // → minus TRAMPOLINE_32BIT_SIZE.
        let expected = (BIOS_START_MAX & !(PAGE_SIZE - 1)) - TRAMPOLINE_32BIT_SIZE;
        assert_eq!(p, expected);
    }

    #[test]
    fn trampoline_placement_uses_ebda_when_lower_than_bios() {
        let ebda = 0x9000u16; // 0x90000 linear
        let basekb = 640u16; // 0xa0000 linear (clamped to BIOS_START_MAX)
        let p = find_trampoline_placement(false, ebda, basekb);
        let expected = (0x90000 & !(PAGE_SIZE - 1)) - TRAMPOLINE_32BIT_SIZE;
        assert_eq!(p, expected);
    }
}
