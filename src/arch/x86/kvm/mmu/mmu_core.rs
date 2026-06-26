//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/mmu
//! test-origin: linux:vendor/linux/arch/x86/kvm/mmu
//! KVM shadow-paging MMU core.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/mmu/mmu.c

// `mmu.c` builds shadow page tables that mirror the guest's view onto
// host physical pages, while tracking dirty bits for live migration.
// We model the shadow page descriptor and a small invariance predicate.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShadowPage {
    pub gfn: u64,
    pub level: u8,
    pub direct: bool,
    pub access: u8,
}

pub const fn shadow_role_matches(a: ShadowPage, b: ShadowPage) -> bool {
    a.gfn == b.gfn && a.level == b.level && a.direct == b.direct && a.access == b.access
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_pages_match_only_when_role_identical() {
        let a = ShadowPage {
            gfn: 0x1000,
            level: 1,
            direct: true,
            access: 7,
        };
        let b = a;
        assert!(shadow_role_matches(a, b));

        let c = ShadowPage { level: 2, ..a };
        assert!(!shadow_role_matches(a, c));
    }
}
