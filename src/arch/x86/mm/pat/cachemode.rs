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
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("pat_enabled", linux_pat_enabled as usize, true);
    export_symbol_once("arch_phys_wc_add", linux_arch_phys_wc_add as usize, false);
    export_symbol_once("arch_phys_wc_del", linux_arch_phys_wc_del as usize, false);
    export_symbol_once(
        "pgprot_writecombine",
        linux_pgprot_writecombine as usize,
        true,
    );
    export_symbol_once(
        "pgprot_writethrough",
        linux_pgprot_writethrough as usize,
        true,
    );
}

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

/// `pat_enabled` - `vendor/linux/arch/x86/mm/pat/memtype.c:92`.
pub extern "C" fn linux_pat_enabled() -> bool {
    pat_enabled(CpuFeatures::current())
}

/// `arch_phys_wc_add` - `vendor/linux/arch/x86/kernel/cpu/mtrr/mtrr.c:483`.
///
/// With PAT enabled Linux returns 0 and does not reserve an MTRR. Lupos uses
/// PAT-style page attributes for WC mappings and has no dynamic MTRR allocator,
/// so the exported ABI follows the PAT-enabled success path.
pub extern "C" fn linux_arch_phys_wc_add(_base: usize, _size: usize) -> i32 {
    0
}

/// `arch_phys_wc_del` - `vendor/linux/arch/x86/kernel/cpu/mtrr/mtrr.c:509`.
pub extern "C" fn linux_arch_phys_wc_del(_handle: i32) {}

pub const fn cachemode_to_pte_flags(mode: PageCacheMode) -> u64 {
    match mode {
        PageCacheMode::WriteBack => 0,
        PageCacheMode::WriteCombining => _PAGE_PWT,
        PageCacheMode::UncachedMinus => _PAGE_PCD,
        PageCacheMode::Uncached => _PAGE_PCD | _PAGE_PWT,
        PageCacheMode::WriteThrough => _PAGE_PAT | _PAGE_PCD | _PAGE_PWT,
        PageCacheMode::WriteProtected => _PAGE_PAT | _PAGE_PWT,
    }
}

pub const fn pte_flags_to_cachemode(flags: u64) -> PageCacheMode {
    match flags & (_PAGE_PAT | _PAGE_PCD | _PAGE_PWT) {
        0 => PageCacheMode::WriteBack,
        _PAGE_PWT => PageCacheMode::WriteCombining,
        _PAGE_PCD => PageCacheMode::UncachedMinus,
        x if x == (_PAGE_PCD | _PAGE_PWT) => PageCacheMode::Uncached,
        _PAGE_PAT => PageCacheMode::WriteBack,
        x if x == (_PAGE_PAT | _PAGE_PWT) => PageCacheMode::WriteProtected,
        x if x == (_PAGE_PAT | _PAGE_PCD) => PageCacheMode::UncachedMinus,
        x if x == (_PAGE_PAT | _PAGE_PCD | _PAGE_PWT) => PageCacheMode::WriteThrough,
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

/// `pgprot_writecombine` - `vendor/linux/arch/x86/mm/pat/memtype.c:944`.
pub extern "C" fn linux_pgprot_writecombine(prot: pgprot_t) -> pgprot_t {
    pgprot_writecombine(prot)
}

/// `pgprot_writethrough` - `vendor/linux/arch/x86/mm/pat/memtype.c:951`.
pub extern "C" fn linux_pgprot_writethrough(prot: pgprot_t) -> pgprot_t {
    pgprot_writethrough(prot)
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

    #[test]
    fn cachemode_translation_matches_linux_full_pat_slots() {
        assert_eq!(cachemode_to_pte_flags(PageCacheMode::WriteBack), 0);
        assert_eq!(
            cachemode_to_pte_flags(PageCacheMode::WriteCombining),
            _PAGE_PWT
        );
        assert_eq!(
            cachemode_to_pte_flags(PageCacheMode::UncachedMinus),
            _PAGE_PCD
        );
        assert_eq!(
            cachemode_to_pte_flags(PageCacheMode::Uncached),
            _PAGE_PCD | _PAGE_PWT
        );
        assert_eq!(
            cachemode_to_pte_flags(PageCacheMode::WriteProtected),
            _PAGE_PAT | _PAGE_PWT
        );
        assert_eq!(
            cachemode_to_pte_flags(PageCacheMode::WriteThrough),
            _PAGE_PAT | _PAGE_PCD | _PAGE_PWT
        );
    }

    #[test]
    fn cachemode_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(find_symbol("pat_enabled"), Some(linux_pat_enabled as usize));
        assert_eq!(
            find_symbol("arch_phys_wc_add"),
            Some(linux_arch_phys_wc_add as usize)
        );
        assert_eq!(
            find_symbol("arch_phys_wc_del"),
            Some(linux_arch_phys_wc_del as usize)
        );
        assert_eq!(
            find_symbol("pgprot_writecombine"),
            Some(linux_pgprot_writecombine as usize)
        );
        assert_eq!(
            find_symbol("pgprot_writethrough"),
            Some(linux_pgprot_writethrough as usize)
        );
    }
}
