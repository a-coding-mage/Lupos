//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/mmu/page_track.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/mmu/page_track.c
//! Per-page tracking for KVM MMU.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/mmu/page_track.c

// `page_track.c` lets external subsystems (KVMGT GPU passthrough)
// register write-protection callbacks on individual guest pages. We
// model the small reference-count map.

extern crate alloc;

use alloc::collections::BTreeMap;

use crate::include::uapi::errno::EINVAL;

#[derive(Default, Debug)]
pub struct PageTrackRegistry {
    write_protect_refcount: BTreeMap<u64, u32>,
}

impl PageTrackRegistry {
    pub fn add_write_protect(&mut self, gfn: u64) {
        let entry = self.write_protect_refcount.entry(gfn).or_insert(0);
        *entry += 1;
    }

    pub fn remove_write_protect(&mut self, gfn: u64) -> Result<(), i32> {
        let entry = self.write_protect_refcount.get_mut(&gfn).ok_or(EINVAL)?;
        if *entry == 0 {
            return Err(EINVAL);
        }
        *entry -= 1;
        if *entry == 0 {
            self.write_protect_refcount.remove(&gfn);
        }
        Ok(())
    }

    pub fn is_write_protected(&self, gfn: u64) -> bool {
        self.write_protect_refcount.get(&gfn).copied().unwrap_or(0) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refcounted_protection_clears_only_at_zero() {
        let mut r = PageTrackRegistry::default();
        r.add_write_protect(0x1000);
        r.add_write_protect(0x1000);
        assert!(r.is_write_protected(0x1000));
        r.remove_write_protect(0x1000).unwrap();
        assert!(r.is_write_protected(0x1000));
        r.remove_write_protect(0x1000).unwrap();
        assert!(!r.is_write_protected(0x1000));
        assert!(r.remove_write_protect(0x1000).is_err());
    }
}
