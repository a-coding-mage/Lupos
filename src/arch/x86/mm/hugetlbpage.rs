//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/hugetlbpage.c
//! test-origin: linux:vendor/linux/arch/x86/mm/hugetlbpage.c
//! x86 hugetlb page policy.
//!
//! Mirrors the gigantic-page initialization gate in
//! `vendor/linux/arch/x86/mm/hugetlbpage.c`. The generic huge-page allocator
//! lives under `crate::mm`; this module owns the x86 availability checks.

use crate::include::uapi::errno::EOPNOTSUPP;

pub const PMD_HUGE_PAGE_SHIFT: u32 = 21;
pub const PUD_HUGE_PAGE_SHIFT: u32 = 30;
pub const PMD_HUGE_PAGE_SIZE: u64 = 1 << PMD_HUGE_PAGE_SHIFT;
pub const PUD_HUGE_PAGE_SIZE: u64 = 1 << PUD_HUGE_PAGE_SHIFT;

pub const fn gigantic_pages_supported(has_gbpages: bool, physical_address_bits: u8) -> bool {
    has_gbpages && physical_address_bits >= PUD_HUGE_PAGE_SHIFT as u8
}

pub const fn gigantic_pages_init(has_gbpages: bool, physical_address_bits: u8) -> Result<u64, i32> {
    if gigantic_pages_supported(has_gbpages, physical_address_bits) {
        Ok(PUD_HUGE_PAGE_SIZE)
    } else {
        Err(EOPNOTSUPP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hugepage_sizes_match_x86_levels() {
        assert_eq!(PMD_HUGE_PAGE_SIZE, 2 * 1024 * 1024);
        assert_eq!(PUD_HUGE_PAGE_SIZE, 1024 * 1024 * 1024);
    }

    #[test]
    fn gigantic_pages_require_gbpage_cpu_support() {
        assert_eq!(gigantic_pages_init(false, 52), Err(EOPNOTSUPP));
        assert_eq!(gigantic_pages_init(true, 52), Ok(PUD_HUGE_PAGE_SIZE));
    }
}
