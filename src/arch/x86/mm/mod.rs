//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm
//! test-origin: linux:vendor/linux/arch/x86/mm
//! x86 architecture memory-management policy.
//!
//! Existing modules own the live low-level pieces: `paging`, `fault`, `tlb`,
//! `pat`, and `ioremap`. This module ties together the architecture policies
//! that Linux keeps in `arch/x86/mm`: direct-map ranges, optional hardening
//! features, and fail-closed decisions for memory modes not configured in Lupos.

use crate::include::uapi::errno::{EINVAL, ENODEV, EOPNOTSUPP};

use self::paging::{PAGE_MASK, PAGE_SIZE};

pub mod amdtopology;
pub mod cpu_entry_area;
pub mod debug_pagetables;
pub mod dump_pagetables;
pub mod hugetlbpage;
pub mod ident_map;
pub mod init;
pub mod init_32;
pub mod init_64;
pub mod iomap_32;
pub mod kasan_init_64;
pub mod kaslr;
pub mod kmmio;
pub mod kmsan_shadow;
pub mod maccess;
pub mod mem_encrypt;
pub mod mem_encrypt_amd;
pub mod mmap;
pub mod mmio_mod;
pub mod numa;
pub mod pat;
pub mod pf_in;
pub mod pgprot;
pub mod pgtable;
pub mod pgtable_32;
pub mod physaddr;
pub mod pkeys;
pub mod pti;
pub mod srat;
pub mod testmmiotrace;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchMmFeature {
    HugeTlb,
    Kasan,
    Kmsan,
    MemoryEncryption,
    Numa,
    ProtectionKeys,
    PageTableIsolation,
    MmioTrace,
}

pub const fn feature_enabled(feature: ArchMmFeature) -> bool {
    match feature {
        ArchMmFeature::HugeTlb
        | ArchMmFeature::Kasan
        | ArchMmFeature::Kmsan
        | ArchMmFeature::MemoryEncryption
        | ArchMmFeature::Numa
        | ArchMmFeature::ProtectionKeys
        | ArchMmFeature::PageTableIsolation
        | ArchMmFeature::MmioTrace => false,
    }
}

pub const fn feature_errno(feature: ArchMmFeature) -> i32 {
    match feature {
        ArchMmFeature::HugeTlb | ArchMmFeature::ProtectionKeys => EOPNOTSUPP,
        ArchMmFeature::Kasan | ArchMmFeature::Kmsan | ArchMmFeature::MmioTrace => ENODEV,
        ArchMmFeature::MemoryEncryption
        | ArchMmFeature::Numa
        | ArchMmFeature::PageTableIsolation => EOPNOTSUPP,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageAlignedRange {
    pub start: u64,
    pub end: u64,
}

pub const fn align_range_down_up(start: u64, size: u64) -> Result<PageAlignedRange, i32> {
    if size == 0 {
        return Err(EINVAL);
    }
    let end = match start.checked_add(size) {
        Some(end) => end,
        None => return Err(EINVAL),
    };
    Ok(PageAlignedRange {
        start: start & PAGE_MASK,
        end: (end + (PAGE_SIZE - 1)) & PAGE_MASK,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_mm_features_fail_closed() {
        assert!(!feature_enabled(ArchMmFeature::MemoryEncryption));
        assert_eq!(feature_errno(ArchMmFeature::Kasan), ENODEV);
    }

    #[test]
    fn range_alignment_matches_page_boundaries() {
        assert_eq!(
            align_range_down_up(0x1234, 0x100).unwrap(),
            PageAlignedRange {
                start: 0x1000,
                end: 0x2000
            }
        );
        assert_eq!(align_range_down_up(u64::MAX, 2), Err(EINVAL));
    }
}

pub mod fault;
pub mod ioremap;
pub mod paging;
pub mod tlb;
