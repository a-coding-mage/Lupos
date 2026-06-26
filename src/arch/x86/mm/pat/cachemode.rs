//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pat
//! test-origin: linux:vendor/linux/arch/x86/mm/pat
//! Page Attribute Table cache-mode helpers.
//!
//! This module maps Linux x86 cache modes to the PTE/PAT bits used by the
//! existing page-table code. It deliberately keeps the reservation interval
//! tree out until the memory-resource layer can enforce conflicts globally.
//!
//! References:
//! - `vendor/linux/arch/x86/mm/pat/memtype.c`

use crate::arch::x86::kernel::cpu::CpuFeatures;
use crate::arch::x86::mm::paging::{
    __pgprot, _PAGE_PAT, _PAGE_PCD, _PAGE_PWT, pgprot_t, pgprot_val,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PageCacheMode {
    WriteBack = 0,
    WriteCombining = 1,
    UncachedMinus = 2,
    Uncached = 3,
    WriteThrough = 4,
    WriteProtected = 5,
}

pub fn pat_enabled(features: CpuFeatures) -> bool {
    features.has_pat()
}

pub const fn cachemode_to_pte_flags(mode: PageCacheMode) -> u64 {
    match mode {
        PageCacheMode::WriteBack => 0,
        PageCacheMode::WriteCombining => _PAGE_PWT,
        PageCacheMode::UncachedMinus => _PAGE_PCD,
        PageCacheMode::Uncached => _PAGE_PCD | _PAGE_PWT,
        PageCacheMode::WriteThrough => _PAGE_PAT | _PAGE_PWT,
        PageCacheMode::WriteProtected => _PAGE_PAT,
    }
}

pub const fn pte_flags_to_cachemode(flags: u64) -> PageCacheMode {
    match flags & (_PAGE_PAT | _PAGE_PCD | _PAGE_PWT) {
        0 => PageCacheMode::WriteBack,
        _PAGE_PWT => PageCacheMode::WriteCombining,
        _PAGE_PCD => PageCacheMode::UncachedMinus,
        x if x == (_PAGE_PCD | _PAGE_PWT) => PageCacheMode::Uncached,
        x if x == (_PAGE_PAT | _PAGE_PWT) => PageCacheMode::WriteThrough,
        _PAGE_PAT => PageCacheMode::WriteProtected,
        _ => PageCacheMode::Uncached,
    }
}

pub const fn pgprot_with_cachemode(prot: pgprot_t, mode: PageCacheMode) -> pgprot_t {
    let cache_mask = _PAGE_PAT | _PAGE_PCD | _PAGE_PWT;
    __pgprot((pgprot_val(prot) & !cache_mask) | cachemode_to_pte_flags(mode))
}

pub const fn pgprot_noncached(prot: pgprot_t) -> pgprot_t {
    pgprot_with_cachemode(prot, PageCacheMode::UncachedMinus)
}

pub const fn pgprot_writecombine(prot: pgprot_t) -> pgprot_t {
    pgprot_with_cachemode(prot, PageCacheMode::WriteCombining)
}

pub const fn pgprot_writethrough(prot: pgprot_t) -> pgprot_t {
    pgprot_with_cachemode(prot, PageCacheMode::WriteThrough)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::PAGE_KERNEL;

    #[test]
    fn cachemode_translation_round_trips_linux_primary_modes() {
        for mode in [
            PageCacheMode::WriteBack,
            PageCacheMode::WriteCombining,
            PageCacheMode::UncachedMinus,
            PageCacheMode::Uncached,
            PageCacheMode::WriteThrough,
            PageCacheMode::WriteProtected,
        ] {
            assert_eq!(pte_flags_to_cachemode(cachemode_to_pte_flags(mode)), mode);
        }
    }

    #[test]
    fn pgprot_helpers_replace_only_cache_bits() {
        let wc = pgprot_writecombine(PAGE_KERNEL);
        assert_eq!(
            pgprot_val(wc) & (_PAGE_PAT | _PAGE_PCD | _PAGE_PWT),
            _PAGE_PWT
        );
        let uc = pgprot_noncached(PAGE_KERNEL);
        assert_eq!(
            pgprot_val(uc) & (_PAGE_PAT | _PAGE_PCD | _PAGE_PWT),
            _PAGE_PCD
        );
    }
}
