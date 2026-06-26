//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/mmu/tdp_mmu.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/mmu/tdp_mmu.c
//! Two-Dimensional Paging (TDP) MMU implementation.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/mmu/tdp_mmu.c

// The TDP MMU manages EPT/NPT root tables per VM with RCU-protected
// page-table reads. We model the root list and a few invariance helpers.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdpRoot {
    pub root_id: u32,
    pub generation: u32,
    pub flushed: bool,
}

#[derive(Default, Debug)]
pub struct TdpMmu {
    pub roots: Vec<TdpRoot>,
    pub current_generation: u32,
}

impl TdpMmu {
    pub fn install_root(&mut self, id: u32) {
        self.current_generation += 1;
        self.roots.push(TdpRoot {
            root_id: id,
            generation: self.current_generation,
            flushed: false,
        });
    }

    pub fn flush_all(&mut self) {
        for root in self.roots.iter_mut() {
            root.flushed = true;
        }
    }

    pub fn live_roots(&self) -> usize {
        self.roots.iter().filter(|r| !r.flushed).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_marks_all_roots_as_flushed() {
        let mut mmu = TdpMmu::default();
        mmu.install_root(1);
        mmu.install_root(2);
        assert_eq!(mmu.live_roots(), 2);
        mmu.flush_all();
        assert_eq!(mmu.live_roots(), 0);
    }

    #[test]
    fn install_root_increments_generation() {
        let mut mmu = TdpMmu::default();
        mmu.install_root(7);
        assert_eq!(mmu.roots[0].generation, 1);
        mmu.install_root(8);
        assert_eq!(mmu.roots[1].generation, 2);
    }
}
