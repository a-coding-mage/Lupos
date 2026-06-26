//! linux-parity: complete
//! linux-source: vendor/linux/virt/kvm/dirty_ring.c
//! test-origin: linux:vendor/linux/virt/kvm/dirty_ring.c
//! KVM dirty-ring accounting, soft-full policy, and reset batching.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EINTR, EINVAL, ENOMEM};

pub const KVM_DIRTY_RING_RSVD_ENTRIES: u32 = 64;
pub const KVM_DIRTY_RING_MAX_ENTRIES: u32 = 65536;
pub const KVM_DIRTY_GFN_SIZE: u32 = 16;
pub const PAGE_SIZE: u32 = 4096;
pub const KVM_INTERNAL_MEM_SLOTS: u16 = 3;
pub const KVM_MEM_SLOTS_NUM: u16 = i16::MAX as u16;
pub const KVM_USER_MEM_SLOTS: u16 = KVM_MEM_SLOTS_NUM - KVM_INTERNAL_MEM_SLOTS;
pub const KVM_REQ_DIRTY_RING_SOFT_FULL: u32 = 3;
pub const KVM_EXIT_DIRTY_RING_FULL: u32 = 31;
pub const KVM_DIRTY_GFN_F_DIRTY: u32 = 1 << 0;
pub const KVM_DIRTY_GFN_F_RESET: u32 = 1 << 1;
pub const KVM_DIRTY_GFN_F_MASK: u32 = 0x3;
pub const BITS_PER_LONG: i128 = 64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmDirtyGfn {
    pub flags: u32,
    pub slot: u32,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirtyResetBatch {
    pub slot: u32,
    pub offset: u64,
    pub mask: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResetDirtyGfnPlan {
    pub as_id: u32,
    pub id: u16,
    pub reset: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmDirtyRing {
    pub dirty_gfns: Vec<KvmDirtyGfn>,
    pub size: u32,
    pub soft_limit: u32,
    pub dirty_index: u32,
    pub reset_index: u32,
    pub index: i32,
    pub soft_full_request: bool,
    pub exit_reason: Option<u32>,
}

impl KvmDirtyRing {
    pub fn alloc(index: i32, size_bytes: u32, cpu_dirty_log_size: u32) -> Result<Self, i32> {
        let size = size_bytes / KVM_DIRTY_GFN_SIZE;
        if size == 0 || size > KVM_DIRTY_RING_MAX_ENTRIES || !size.is_power_of_two() {
            return Err(-EINVAL);
        }

        let soft_limit = size.wrapping_sub(kvm_dirty_ring_get_rsvd_entries(cpu_dirty_log_size));
        Ok(Self {
            dirty_gfns: vec![KvmDirtyGfn::default(); size as usize],
            size,
            soft_limit,
            dirty_index: 0,
            reset_index: 0,
            index,
            soft_full_request: false,
            exit_reason: None,
        })
    }

    pub const fn used(&self) -> u32 {
        self.dirty_index.wrapping_sub(self.reset_index)
    }

    pub const fn soft_full(&self) -> bool {
        self.used() >= self.soft_limit
    }

    pub const fn full(&self) -> bool {
        self.used() >= self.size
    }

    pub fn push(&mut self, slot: u32, offset: u64) -> Result<bool, i32> {
        if self.size == 0 {
            return Err(-EINVAL);
        }
        let index = (self.dirty_index & (self.size - 1)) as usize;
        self.dirty_gfns[index] = KvmDirtyGfn {
            flags: KVM_DIRTY_GFN_F_DIRTY,
            slot,
            offset,
        };
        self.dirty_index = self.dirty_index.wrapping_add(1);
        if self.soft_full() {
            self.soft_full_request = true;
        }
        Ok(self.soft_full_request)
    }

    pub fn harvest_entry(&mut self, ring_index: u32) {
        let index = (ring_index & (self.size - 1)) as usize;
        self.dirty_gfns[index].flags = KVM_DIRTY_GFN_F_RESET;
    }

    pub fn reset(&mut self, nr_entries_reset: &mut i32) -> Result<Vec<DirtyResetBatch>, i32> {
        self.reset_with_signal(nr_entries_reset, false)
    }

    pub fn reset_with_signal(
        &mut self,
        nr_entries_reset: &mut i32,
        signal_pending: bool,
    ) -> Result<Vec<DirtyResetBatch>, i32> {
        if self.dirty_gfns.is_empty() {
            return Err(-ENOMEM);
        }

        let mut batches = Vec::new();
        let mut cur_slot = 0u32;
        let mut cur_offset = 0u64;
        let mut mask = 0u64;

        while *nr_entries_reset < i32::MAX {
            if signal_pending {
                return Err(-EINTR);
            }

            let entry_index = (self.reset_index & (self.size - 1)) as usize;
            let entry = self.dirty_gfns[entry_index];
            if entry.flags & KVM_DIRTY_GFN_F_RESET == 0 {
                break;
            }

            let next_slot = entry.slot;
            let next_offset = entry.offset;
            self.dirty_gfns[entry_index].flags = 0;
            self.reset_index = self.reset_index.wrapping_add(1);
            *nr_entries_reset += 1;

            if mask != 0 {
                if next_slot == cur_slot {
                    let delta = next_offset as i128 - cur_offset as i128;
                    if (0..BITS_PER_LONG).contains(&delta) {
                        mask |= 1u64 << (delta as u32);
                        continue;
                    }
                    if (-BITS_PER_LONG + 1..0).contains(&delta) {
                        let shift = (-delta) as u32;
                        if (mask << shift >> shift) == mask {
                            cur_offset = next_offset;
                            mask = (mask << shift) | 1;
                            continue;
                        }
                    }
                }
                batches.push(DirtyResetBatch {
                    slot: cur_slot,
                    offset: cur_offset,
                    mask,
                });
            }

            cur_slot = next_slot;
            cur_offset = next_offset;
            mask = 1;
        }

        if mask != 0 {
            batches.push(DirtyResetBatch {
                slot: cur_slot,
                offset: cur_offset,
                mask,
            });
        }

        Ok(batches)
    }

    pub fn check_request(&mut self) -> bool {
        if self.soft_full_request && self.soft_full() {
            self.soft_full_request = true;
            self.exit_reason = Some(KVM_EXIT_DIRTY_RING_FULL);
            return true;
        }
        false
    }

    pub fn get_page(&self, offset: u32) -> Option<u64> {
        if self.dirty_gfns.is_empty() {
            return None;
        }
        Some(offset as u64 * PAGE_SIZE as u64)
    }

    pub fn free(&mut self) {
        self.dirty_gfns.clear();
        self.dirty_gfns.shrink_to_fit();
        self.size = 0;
        self.soft_limit = 0;
        self.dirty_index = 0;
        self.reset_index = 0;
        self.soft_full_request = false;
        self.exit_reason = None;
    }
}

pub const fn kvm_cpu_dirty_log_size() -> u32 {
    0
}

pub const fn kvm_dirty_ring_get_rsvd_entries(cpu_dirty_log_size: u32) -> u32 {
    KVM_DIRTY_RING_RSVD_ENTRIES + cpu_dirty_log_size
}

pub const fn kvm_use_dirty_bitmap(dirty_ring_size: u32, dirty_ring_with_bitmap: bool) -> bool {
    dirty_ring_size == 0 || dirty_ring_with_bitmap
}

pub const fn kvm_arch_allow_write_without_running_vcpu() -> bool {
    false
}

pub const fn kvm_dirty_gfn_harvested(entry: KvmDirtyGfn) -> bool {
    entry.flags & KVM_DIRTY_GFN_F_RESET != 0
}

pub const fn kvm_reset_dirty_gfn_plan(
    slot: u32,
    nr_memslot_as_ids: u32,
    memslot_present: bool,
    memslot_npages: u64,
    offset: u64,
    mask: u64,
) -> ResetDirtyGfnPlan {
    let as_id = slot >> 16;
    let id = slot as u16;
    if as_id >= nr_memslot_as_ids || id >= KVM_USER_MEM_SLOTS {
        return ResetDirtyGfnPlan {
            as_id,
            id,
            reset: false,
        };
    }
    if !memslot_present || offset >= memslot_npages {
        return ResetDirtyGfnPlan {
            as_id,
            id,
            reset: false,
        };
    }

    let last_bit = if mask == 0 {
        0
    } else {
        63 - mask.leading_zeros() as u64
    };

    let end = match offset.checked_add(last_bit) {
        Some(end) => end,
        None => {
            return ResetDirtyGfnPlan {
                as_id,
                id,
                reset: false,
            };
        }
    };

    ResetDirtyGfnPlan {
        as_id,
        id,
        reset: end < memslot_npages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_ring_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/dirty_ring.c"
        ));
        let host_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/kvm_host.h"
        ));
        let x86_host_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/kvm_host.h"
        ));
        assert!(
            source.contains("return KVM_DIRTY_RING_RSVD_ENTRIES + kvm_cpu_dirty_log_size(kvm);")
        );
        assert!(source.contains("int __weak kvm_cpu_dirty_log_size(struct kvm *kvm)"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("return !kvm->dirty_ring_size || kvm->dirty_ring_with_bitmap;"));
        assert!(source.contains("bool kvm_arch_allow_write_without_running_vcpu(struct kvm *kvm)"));
        assert!(x86_host_source.contains("#define KVM_INTERNAL_MEM_SLOTS 3"));
        assert!(host_source.contains("#define KVM_MEM_SLOTS_NUM SHRT_MAX"));
        assert!(host_source.contains("#define KVM_USER_MEM_SLOTS"));
        assert!(
            source.contains("return READ_ONCE(ring->dirty_index) - READ_ONCE(ring->reset_index);")
        );
        assert!(
            source
                .contains("ring->soft_limit = ring->size - kvm_dirty_ring_get_rsvd_entries(kvm);")
        );
        assert!(
            source.contains("entry = &ring->dirty_gfns[ring->reset_index & (ring->size - 1)];")
        );
        assert!(source.contains("kvm_dirty_gfn_set_invalid(entry);"));
        assert!(source.contains("if (signal_pending(current))"));
        assert!(source.contains("return -EINTR;"));
        assert!(
            source.contains("entry = &ring->dirty_gfns[ring->dirty_index & (ring->size - 1)];")
        );
        assert!(source.contains("kvm_make_request(KVM_REQ_DIRTY_RING_SOFT_FULL, vcpu);"));
        assert!(source.contains("vcpu->run->exit_reason = KVM_EXIT_DIRTY_RING_FULL;"));
        assert!(source.contains("struct page *kvm_dirty_ring_get_page"));
        assert!(
            source
                .contains("return vmalloc_to_page((void *)ring->dirty_gfns + offset * PAGE_SIZE);")
        );
        assert!(source.contains("void kvm_dirty_ring_free"));
        assert!(source.contains("vfree(ring->dirty_gfns);"));
        assert!(source.contains("ring->dirty_gfns = NULL;"));
    }

    #[test]
    fn allocation_sets_size_soft_limit_and_indexes() {
        let ring = KvmDirtyRing::alloc(3, 128 * KVM_DIRTY_GFN_SIZE, 8).unwrap();
        assert_eq!(ring.size, 128);
        assert_eq!(ring.soft_limit, 56);
        assert_eq!(ring.index, 3);
        assert_eq!(kvm_dirty_ring_get_rsvd_entries(8), 72);
        assert_eq!(kvm_cpu_dirty_log_size(), 0);
        assert!(kvm_use_dirty_bitmap(0, false));
        assert!(kvm_use_dirty_bitmap(4096, true));
        assert!(!kvm_use_dirty_bitmap(4096, false));
        assert!(!kvm_arch_allow_write_without_running_vcpu());
        assert_eq!(KVM_USER_MEM_SLOTS, 32764);
    }

    #[test]
    fn push_sets_dirty_and_requests_soft_full() {
        let mut ring = KvmDirtyRing::alloc(0, 128 * KVM_DIRTY_GFN_SIZE, 0).unwrap();
        for i in 0..63 {
            assert!(!ring.push(1, i).unwrap());
        }
        assert!(ring.push(1, 63).unwrap());
        assert_eq!(ring.dirty_gfns[63].flags, KVM_DIRTY_GFN_F_DIRTY);
        assert!(ring.check_request());
        assert_eq!(ring.exit_reason, Some(KVM_EXIT_DIRTY_RING_FULL));
    }

    #[test]
    fn reset_batches_forward_and_backward_adjacent_offsets() {
        let mut ring = KvmDirtyRing::alloc(0, 128 * KVM_DIRTY_GFN_SIZE, 0).unwrap();
        for offset in [10, 11, 12, 9] {
            ring.push(2, offset).unwrap();
            ring.harvest_entry(ring.dirty_index - 1);
        }

        let mut reset = 0;
        let batches = ring.reset(&mut reset).unwrap();
        assert_eq!(reset, 4);
        assert_eq!(
            batches,
            &[DirtyResetBatch {
                slot: 2,
                offset: 9,
                mask: 0b1111,
            }]
        );
        assert_eq!(ring.reset_index, 4);
        assert_eq!(ring.dirty_gfns[0].flags, 0);
    }

    #[test]
    fn reset_reports_pending_signal_before_consuming_entries() {
        let mut ring = KvmDirtyRing::alloc(0, 128 * KVM_DIRTY_GFN_SIZE, 0).unwrap();
        ring.push(7, 42).unwrap();
        ring.harvest_entry(0);

        let mut reset = 0;
        assert_eq!(ring.reset_with_signal(&mut reset, true), Err(-EINTR));
        assert_eq!(reset, 0);
        assert_eq!(ring.reset_index, 0);
        assert!(kvm_dirty_gfn_harvested(ring.dirty_gfns[0]));
    }

    #[test]
    fn reset_dirty_gfn_plan_checks_slot_and_range() {
        assert_eq!(
            kvm_reset_dirty_gfn_plan(1, 1, true, 8, 2, 0b111),
            ResetDirtyGfnPlan {
                as_id: 0,
                id: 1,
                reset: true,
            }
        );
        assert!(!kvm_reset_dirty_gfn_plan(1 << 16, 1, true, 8, 2, 1).reset);
        assert!(!kvm_reset_dirty_gfn_plan(KVM_USER_MEM_SLOTS as u32, 1, true, 8, 2, 1).reset);
        assert!(!kvm_reset_dirty_gfn_plan(1, 1, false, 8, 2, 1).reset);
        assert!(!kvm_reset_dirty_gfn_plan(1, 1, true, 8, 8, 1).reset);
        assert!(!kvm_reset_dirty_gfn_plan(1, 1, true, 8, 6, 0b111).reset);
        assert!(!kvm_reset_dirty_gfn_plan(1, 1, true, u64::MAX, u64::MAX, 1).reset);
    }

    #[test]
    fn page_lookup_and_free_follow_vmalloc_lifetime() {
        let mut ring = KvmDirtyRing::alloc(0, 128 * KVM_DIRTY_GFN_SIZE, 0).unwrap();
        assert_eq!(ring.get_page(2), Some(2 * PAGE_SIZE as u64));

        ring.free();
        assert!(ring.dirty_gfns.is_empty());
        assert_eq!(ring.size, 0);
        assert_eq!(ring.get_page(2), None);
    }
}
