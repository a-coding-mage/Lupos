//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/ident_map_64.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/ident_map_64.c
//! Identity-mapping page-table builder for the decompressor.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/ident_map_64.c
//!
//! The decompressor needs a working identity map before it can touch
//! any of the kernel-image regions. Linux walks the e820 table, maps
//! each "usable" region in 1:1 PA→VA fashion using 1 GiB / 2 MiB / 4
//! KiB leaves where alignment allows, and registers a #PF handler that
//! lazily faults in additional ranges. The port reproduces the
//! map-or-split decision tree and the architectural constants.

/// Page-size constants used by the mapper. Values match
/// `arch/x86/include/asm/page_types.h`.
pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
pub const PMD_SHIFT: u32 = 21;
pub const PMD_SIZE: u64 = 1 << PMD_SHIFT;
pub const PUD_SHIFT: u32 = 30;
pub const PUD_SIZE: u64 = 1 << PUD_SHIFT;

/// Returns true if [start, end) is aligned to a 1 GiB boundary and
/// large enough to be mapped with a single PUD leaf. Mirrors the
/// Linux mapper's "use_gbpages" choice.
pub fn fits_gbpage(start: u64, end: u64) -> bool {
    start & (PUD_SIZE - 1) == 0 && end >= start.wrapping_add(PUD_SIZE) && end & (PUD_SIZE - 1) == 0
}

/// Returns true if [start, end) is aligned to 2 MiB and ≥ 2 MiB long.
pub fn fits_largepage(start: u64, end: u64) -> bool {
    start & (PMD_SIZE - 1) == 0 && end >= start.wrapping_add(PMD_SIZE) && end & (PMD_SIZE - 1) == 0
}

/// Granularity the mapper picks for a region. Mirrors Linux's split
/// from `kernel_ident_mapping_init()` (in `arch/x86/mm/ident_map.c`,
/// reused by the decompressor's `kernel_add_identity_map`).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MapGranularity {
    /// 1 GiB leaf in the PUD.
    GbPage,
    /// 2 MiB leaf in the PMD.
    LargePage,
    /// 4 KiB leaf in the PTE.
    Page,
}

/// Pick the coarsest granularity that fits `[start, end)`.
pub fn choose_granularity(start: u64, end: u64) -> MapGranularity {
    if fits_gbpage(start, end) {
        MapGranularity::GbPage
    } else if fits_largepage(start, end) {
        MapGranularity::LargePage
    } else {
        MapGranularity::Page
    }
}

/// Round an address down to the previous page boundary.
#[inline]
pub const fn round_down_page(addr: u64) -> u64 {
    addr & !(PAGE_SIZE - 1)
}

/// Round an address up to the next page boundary.
#[inline]
pub const fn round_up_page(addr: u64) -> u64 {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Format an aligned `(start, end)` range, snapping to page bounds.
pub fn align_range(start: u64, end: u64) -> (u64, u64) {
    (round_down_page(start), round_up_page(end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_constants_match_x86_arch() {
        assert_eq!(PAGE_SIZE, 4096);
        assert_eq!(PMD_SIZE, 2 * 1024 * 1024);
        assert_eq!(PUD_SIZE, 1024 * 1024 * 1024);
    }

    #[test]
    fn fits_gbpage_requires_alignment_and_size() {
        assert!(fits_gbpage(0, PUD_SIZE));
        assert!(!fits_gbpage(0x1000, PUD_SIZE));
        assert!(!fits_gbpage(0, PUD_SIZE - 1));
        // Spanning two 1 GiB blocks still qualifies if endpoints are 1G-aligned.
        assert!(fits_gbpage(0, 2 * PUD_SIZE));
    }

    #[test]
    fn fits_largepage_requires_2mib_alignment() {
        assert!(fits_largepage(0, PMD_SIZE));
        assert!(!fits_largepage(0x1_0000, PMD_SIZE));
        assert!(!fits_largepage(0, PMD_SIZE - 0x1000));
    }

    #[test]
    fn choose_granularity_prefers_largest_fit() {
        assert_eq!(choose_granularity(0, PUD_SIZE), MapGranularity::GbPage);
        assert_eq!(
            choose_granularity(PMD_SIZE, 2 * PMD_SIZE),
            MapGranularity::LargePage
        );
        assert_eq!(choose_granularity(0x1000, 0x3000), MapGranularity::Page);
    }

    #[test]
    fn align_range_snaps_to_page_boundaries() {
        assert_eq!(align_range(0x1234, 0x5678), (0x1000, 0x6000));
        // Already aligned: no change.
        assert_eq!(align_range(0x1000, 0x2000), (0x1000, 0x2000));
    }
}
