//! linux-parity: partial
//! linux-source: vendor/linux/virt/kvm/pfncache.c
//! test-origin: linux:vendor/linux/virt/kvm/pfncache.c
//! KVM gfn-to-pfn cache activation, refresh, check, and invalidation logic.

use crate::include::uapi::errno::{EFAULT, EINVAL, EIO};

pub const PAGE_SIZE: u64 = 4096;
pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_OFFSET: u64 = 0xffff_8000_0000_0000;
pub const INVALID_GPA: u64 = u64::MAX;
pub const KVM_HVA_ERR_BAD: u64 = PAGE_OFFSET;
pub const KVM_PFN_ERR_MASK: u64 = 0x7ffu64 << 52;
pub const KVM_PFN_ERR_NOSLOT_MASK: u64 = 0xfffu64 << 52;
pub const KVM_PFN_ERR_FAULT: u64 = KVM_PFN_ERR_MASK;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PfnMemslot {
    pub base_gfn: u64,
    pub npages: u64,
    pub userspace_addr: u64,
}

impl PfnMemslot {
    pub const fn contains_gfn(&self, gfn: u64) -> bool {
        self.base_gfn <= gfn && gfn < self.base_gfn + self.npages
    }

    pub const fn hva_for_gfn(&self, gfn: u64) -> u64 {
        self.userspace_addr + ((gfn - self.base_gfn) << PAGE_SHIFT)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GfnToPfnCache {
    pub active: bool,
    pub valid: bool,
    pub gpa: u64,
    pub uhva: u64,
    pub generation: u64,
    pub pfn: u64,
    pub khva: Option<u64>,
    pub mapped: bool,
}

impl GfnToPfnCache {
    pub const fn new() -> Self {
        Self {
            active: false,
            valid: false,
            gpa: INVALID_GPA,
            uhva: KVM_HVA_ERR_BAD,
            generation: 0,
            pfn: KVM_PFN_ERR_FAULT,
            khva: None,
            mapped: false,
        }
    }

    pub fn check(&self, slots_generation: u64, len: u64) -> bool {
        if !self.active {
            return false;
        }
        if !kvm_is_error_gpa(self.gpa) && self.generation != slots_generation {
            return false;
        }
        if kvm_is_error_hva(self.uhva) {
            return false;
        }
        if !kvm_gpc_is_valid_len(self.gpa, self.uhva, len) {
            return false;
        }
        self.valid
    }

    pub fn activate_gpa(
        &mut self,
        slots: &[PfnMemslot],
        slots_generation: u64,
        gpa: u64,
        len: u64,
    ) -> Result<(), i32> {
        if kvm_is_error_gpa(gpa) {
            return Err(-EINVAL);
        }
        if !kvm_gpc_is_valid_len(gpa, KVM_HVA_ERR_BAD, len) {
            return Err(-EINVAL);
        }
        if !self.active && self.valid {
            return Err(-EIO);
        }
        self.active = true;
        self.refresh_gpa(slots, slots_generation, gpa)
    }

    pub fn activate_hva(&mut self, uhva: u64, len: u64) -> Result<(), i32> {
        if kvm_is_error_hva(uhva) || !kvm_gpc_is_valid_len(INVALID_GPA, uhva, len) {
            return Err(-EINVAL);
        }
        if !self.active && self.valid {
            return Err(-EIO);
        }
        self.active = true;
        self.gpa = INVALID_GPA;
        self.generation = 0;
        self.uhva = uhva;
        self.install_mapping(uhva);
        Ok(())
    }

    pub fn refresh(
        &mut self,
        slots: &[PfnMemslot],
        slots_generation: u64,
        len: u64,
    ) -> Result<(), i32> {
        if !kvm_gpc_is_valid_len(self.gpa, self.uhva, len) {
            return Err(-EINVAL);
        }
        if !self.active {
            return Err(-EINVAL);
        }
        if kvm_is_error_gpa(self.gpa) {
            if kvm_is_error_hva(self.uhva) {
                self.invalidate_mapping();
                return Err(-EFAULT);
            }
            self.install_mapping(self.uhva);
            Ok(())
        } else {
            self.refresh_gpa(slots, slots_generation, self.gpa)
        }
    }

    fn refresh_gpa(
        &mut self,
        slots: &[PfnMemslot],
        slots_generation: u64,
        gpa: u64,
    ) -> Result<(), i32> {
        let gfn = gpa >> PAGE_SHIFT;
        let page_offset = offset_in_page(gpa);
        let Some(slot) = slots.iter().find(|slot| slot.contains_gfn(gfn)) else {
            self.gpa = gpa;
            self.generation = slots_generation;
            self.uhva = KVM_HVA_ERR_BAD;
            self.invalidate_mapping();
            return Err(-EFAULT);
        };

        self.gpa = gpa;
        self.generation = slots_generation;
        self.uhva = slot.hva_for_gfn(gfn) + page_offset;
        self.install_mapping(self.uhva);
        Ok(())
    }

    fn install_mapping(&mut self, uhva: u64) {
        self.valid = true;
        self.pfn = uhva >> PAGE_SHIFT;
        self.khva = Some(page_align_down(uhva) + offset_in_page(uhva));
        self.mapped = true;
    }

    fn invalidate_mapping(&mut self) {
        self.valid = false;
        self.pfn = KVM_PFN_ERR_FAULT;
        self.khva = None;
        self.mapped = false;
    }

    pub fn deactivate(&mut self) {
        if self.active {
            self.active = false;
            self.invalidate_mapping();
        }
    }
}

pub fn gfn_to_pfn_cache_invalidate_start(
    caches: &mut [GfnToPfnCache],
    start: u64,
    end: u64,
) -> usize {
    let mut invalidated = 0;
    for cache in caches {
        if cache.valid && !is_error_noslot_pfn(cache.pfn) && cache.uhva >= start && cache.uhva < end
        {
            cache.valid = false;
            invalidated += 1;
        }
    }
    invalidated
}

pub const fn kvm_gpc_is_valid_len(gpa: u64, uhva: u64, len: u64) -> bool {
    let offset = if kvm_is_error_gpa(gpa) {
        offset_in_page(uhva)
    } else {
        offset_in_page(gpa)
    };
    offset <= PAGE_SIZE && len <= PAGE_SIZE - offset
}

pub const fn offset_in_page(addr: u64) -> u64 {
    addr & (PAGE_SIZE - 1)
}

pub const fn page_align_down(addr: u64) -> u64 {
    addr & !(PAGE_SIZE - 1)
}

pub const fn kvm_is_error_gpa(gpa: u64) -> bool {
    gpa == INVALID_GPA
}

pub const fn kvm_is_error_hva(hva: u64) -> bool {
    hva >= PAGE_OFFSET
}

pub const fn is_error_noslot_pfn(pfn: u64) -> bool {
    pfn & KVM_PFN_ERR_NOSLOT_MASK != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfncache_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/pfncache.c"
        ));
        assert!(source.contains("void gfn_to_pfn_cache_invalidate_start"));
        assert!(source.contains("gpc->valid && !is_error_noslot_pfn(gpc->pfn)"));
        assert!(source.contains("gpc->uhva >= start && gpc->uhva < end"));
        assert!(source.contains("return offset + len <= PAGE_SIZE;"));
        assert!(source.contains("if (!gpc->active)"));
        assert!(
            source.contains(
                "if (!kvm_is_error_gpa(gpc->gpa) && gpc->generation != slots->generation)"
            )
        );
        assert!(source.contains("gpc->pfn = KVM_PFN_ERR_FAULT;"));
        assert!(source.contains("gpc->gpa = INVALID_GPA;"));
        assert!(source.contains("return __kvm_gpc_activate(gpc, INVALID_GPA, uhva, len);"));

        let host = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/kvm_host.h"
        ));
        assert!(host.contains("#define KVM_PFN_ERR_MASK"));
        assert!(host.contains("#define KVM_HVA_ERR_BAD"));
        assert!(host.contains("int kvm_gpc_activate(struct gfn_to_pfn_cache *gpc"));
    }

    #[test]
    fn valid_len_uses_gpa_for_gpa_cache_and_hva_for_hva_cache() {
        assert!(kvm_gpc_is_valid_len(0x1ff0, KVM_HVA_ERR_BAD, 16));
        assert!(!kvm_gpc_is_valid_len(0x1ff0, KVM_HVA_ERR_BAD, 17));
        assert!(kvm_gpc_is_valid_len(INVALID_GPA, 0x2ff8, 8));
        assert!(!kvm_gpc_is_valid_len(INVALID_GPA, 0x2ff8, 9));
    }

    #[test]
    fn activate_gpa_translates_through_memslot_and_checks_generation() {
        let slots = [PfnMemslot {
            base_gfn: 10,
            npages: 4,
            userspace_addr: 0x4000_0000,
        }];
        let mut cache = GfnToPfnCache::new();
        cache
            .activate_gpa(&slots, 7, (11 << PAGE_SHIFT) + 0x88, 32)
            .unwrap();

        assert!(cache.active);
        assert!(cache.valid);
        assert_eq!(cache.uhva, 0x4000_1088);
        assert_eq!(cache.khva, Some(0x4000_1088));
        assert!(cache.check(7, 32));
        assert!(!cache.check(8, 32));
    }

    #[test]
    fn refresh_invalidates_on_missing_memslot_and_deactivate_drops_mapping() {
        let slots = [PfnMemslot {
            base_gfn: 1,
            npages: 1,
            userspace_addr: 0x8000,
        }];
        let mut cache = GfnToPfnCache::new();
        cache.activate_gpa(&slots, 1, 1 << PAGE_SHIFT, 8).unwrap();

        assert_eq!(cache.refresh(&[], 2, 8), Err(-EFAULT));
        assert!(!cache.valid);
        assert_eq!(cache.pfn, KVM_PFN_ERR_FAULT);

        cache.activate_hva(0x9000, 16).unwrap();
        assert!(cache.check(0, 16));
        cache.deactivate();
        assert!(!cache.active);
        assert!(!cache.valid);
        assert_eq!(cache.khva, None);
    }

    #[test]
    fn invalidate_start_only_invalidates_valid_present_caches_in_hva_range() {
        let mut caches = [
            GfnToPfnCache {
                active: true,
                valid: true,
                gpa: INVALID_GPA,
                uhva: 0x1000,
                generation: 0,
                pfn: 1,
                khva: Some(0x1000),
                mapped: true,
            },
            GfnToPfnCache {
                active: true,
                valid: true,
                gpa: INVALID_GPA,
                uhva: 0x3000,
                generation: 0,
                pfn: 3,
                khva: Some(0x3000),
                mapped: true,
            },
            GfnToPfnCache {
                active: true,
                valid: true,
                gpa: INVALID_GPA,
                uhva: 0x1800,
                generation: 0,
                pfn: KVM_PFN_ERR_FAULT,
                khva: None,
                mapped: false,
            },
        ];

        assert_eq!(
            gfn_to_pfn_cache_invalidate_start(&mut caches, 0x1000, 0x2000),
            1
        );
        assert!(!caches[0].valid);
        assert!(caches[1].valid);
        assert!(caches[2].valid);
    }
}
