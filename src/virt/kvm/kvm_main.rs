//! linux-parity: partial
//! linux-source: vendor/linux/virt/kvm/kvm_main.c
//! test-origin: linux:vendor/linux/virt/kvm/kvm_main.c
//! Generic KVM memory-slot validation, capability checks, and I/O bus routing.

extern crate alloc;

use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::include::uapi::errno::{E2BIG, EEXIST, EINVAL, ENOENT, ENOSPC, EOPNOTSUPP};

pub const PAGE_SIZE: u64 = 4096;
pub const PAGE_SHIFT: u32 = 12;
pub const KVM_API_VERSION: i32 = 12;

pub const KVM_HALT_POLL_NS_DEFAULT: u32 = 200_000;
pub const KVM_DIRTY_RING_RSVD_ENTRIES: u32 = 64;
pub const KVM_DIRTY_RING_MAX_ENTRIES: u32 = 65_536;
pub const KVM_DIRTY_GFN_SIZE: u32 = 16;

pub const KVM_MEM_LOG_DIRTY_PAGES: u32 = 1 << 0;
pub const KVM_MEM_READONLY: u32 = 1 << 1;
pub const KVM_MEM_GUEST_MEMFD: u32 = 1 << 2;
pub const KVM_SET_USER_MEMORY_REGION_V1_FLAGS: u32 = KVM_MEM_LOG_DIRTY_PAGES | KVM_MEM_READONLY;

pub const KVM_MEM_SLOTS_NUM: u32 = i16::MAX as u32;
pub const KVM_INTERNAL_MEM_SLOTS: u32 = 0;
pub const KVM_USER_MEM_SLOTS: u32 = KVM_MEM_SLOTS_NUM - KVM_INTERNAL_MEM_SLOTS;
pub const KVM_MEM_MAX_NR_PAGES: u64 = (1u64 << 31) - 1;

pub const KVM_CAP_SYNC_MMU: u32 = 16;
pub const KVM_CAP_USER_MEMORY: u32 = 3;
pub const KVM_CAP_USER_MEMORY2: u32 = 231;
pub const KVM_CAP_NR_MEMSLOTS: u32 = 10;
pub const KVM_CAP_HALT_POLL: u32 = 182;
pub const KVM_CAP_DIRTY_LOG_RING: u32 = 192;
pub const KVM_CAP_GUEST_MEMFD: u32 = 234;
pub const KVM_CAP_GUEST_MEMFD_FLAGS: u32 = 244;

pub const GUEST_MEMFD_FLAG_MMAP: u64 = 1 << 0;
pub const GUEST_MEMFD_FLAG_INIT_SHARED: u64 = 1 << 1;

pub const NR_IOBUS_DEVS: usize = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmUserspaceMemoryRegion2 {
    pub slot: u32,
    pub flags: u32,
    pub guest_phys_addr: u64,
    pub memory_size: u64,
    pub userspace_addr: u64,
    pub guest_memfd_offset: u64,
    pub guest_memfd: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmMemorySlot {
    pub as_id: u32,
    pub id: u32,
    pub base_gfn: u64,
    pub npages: u64,
    pub flags: u32,
    pub userspace_addr: u64,
    pub guest_memfd: Option<u32>,
    pub guest_memfd_offset: u64,
}

impl KvmMemorySlot {
    pub const fn end_gfn(&self) -> u64 {
        self.base_gfn + self.npages
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryRegionChange {
    Create,
    Delete,
    Move,
    FlagsOnly,
    Noop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Kvm {
    pub memslots: Vec<KvmMemorySlot>,
    pub nr_memslot_pages: u64,
    pub arch_nr_memslot_as_ids: u32,
    pub arch_has_readonly_mem: bool,
    pub guest_memfd_enabled: bool,
    pub guest_memfd_init_shared_supported: bool,
    pub dirty_ring_size: u32,
    pub dirty_ring_with_bitmap: bool,
    pub created_vcpus: u32,
    pub max_halt_poll_ns: u32,
    pub override_halt_poll_ns: bool,
}

impl Kvm {
    pub fn new() -> Self {
        Self {
            memslots: Vec::new(),
            nr_memslot_pages: 0,
            arch_nr_memslot_as_ids: 1,
            arch_has_readonly_mem: true,
            guest_memfd_enabled: true,
            guest_memfd_init_shared_supported: true,
            dirty_ring_size: 0,
            dirty_ring_with_bitmap: false,
            created_vcpus: 0,
            max_halt_poll_ns: KVM_HALT_POLL_NS_DEFAULT,
            override_halt_poll_ns: false,
        }
    }

    pub fn check_memory_region_flags(&self, mem: &KvmUserspaceMemoryRegion2) -> Result<(), i32> {
        let mut valid_flags = KVM_MEM_LOG_DIRTY_PAGES;
        if self.guest_memfd_enabled {
            valid_flags |= KVM_MEM_GUEST_MEMFD;
        }
        if mem.flags & KVM_MEM_GUEST_MEMFD != 0 {
            valid_flags &= !KVM_MEM_LOG_DIRTY_PAGES;
        }
        if self.arch_has_readonly_mem && mem.flags & KVM_MEM_GUEST_MEMFD == 0 {
            valid_flags |= KVM_MEM_READONLY;
        }
        if mem.flags & !valid_flags != 0 {
            return Err(-EINVAL);
        }
        Ok(())
    }

    pub fn set_memory_region(
        &mut self,
        mem: KvmUserspaceMemoryRegion2,
    ) -> Result<MemoryRegionChange, i32> {
        self.check_memory_region_flags(&mem)?;

        let as_id = mem.slot >> 16;
        let id = mem.slot & 0xffff;
        if mem.memory_size & (PAGE_SIZE - 1) != 0 {
            return Err(-EINVAL);
        }
        if mem.guest_phys_addr & (PAGE_SIZE - 1) != 0 {
            return Err(-EINVAL);
        }
        if mem.userspace_addr & (PAGE_SIZE - 1) != 0 {
            return Err(-EINVAL);
        }
        if mem.flags & KVM_MEM_GUEST_MEMFD != 0
            && (mem.guest_memfd_offset & (PAGE_SIZE - 1) != 0
                || mem
                    .guest_memfd_offset
                    .checked_add(mem.memory_size)
                    .is_none())
        {
            return Err(-EINVAL);
        }
        if as_id >= self.arch_nr_memslot_as_ids || id >= KVM_MEM_SLOTS_NUM {
            return Err(-EINVAL);
        }
        if mem.guest_phys_addr.checked_add(mem.memory_size).is_none() {
            return Err(-EINVAL);
        }

        let npages = mem.memory_size >> PAGE_SHIFT;
        if id < KVM_USER_MEM_SLOTS && npages > KVM_MEM_MAX_NR_PAGES {
            return Err(-EINVAL);
        }

        let old_index = self
            .memslots
            .iter()
            .position(|slot| slot.as_id == as_id && slot.id == id);

        if mem.memory_size == 0 {
            let Some(index) = old_index else {
                return Err(-EINVAL);
            };
            let old = self.memslots.remove(index);
            self.nr_memslot_pages = self
                .nr_memslot_pages
                .checked_sub(old.npages)
                .ok_or(-EINVAL)?;
            return Ok(MemoryRegionChange::Delete);
        }

        let base_gfn = mem.guest_phys_addr >> PAGE_SHIFT;
        let change = if let Some(index) = old_index {
            let old = self.memslots[index];
            if mem.flags & KVM_MEM_GUEST_MEMFD != 0 {
                return Err(-EINVAL);
            }
            if mem.userspace_addr != old.userspace_addr
                || npages != old.npages
                || ((mem.flags ^ old.flags) & (KVM_MEM_READONLY | KVM_MEM_GUEST_MEMFD)) != 0
            {
                return Err(-EINVAL);
            }
            if base_gfn != old.base_gfn {
                MemoryRegionChange::Move
            } else if mem.flags != old.flags {
                MemoryRegionChange::FlagsOnly
            } else {
                return Ok(MemoryRegionChange::Noop);
            }
        } else {
            if self.nr_memslot_pages.checked_add(npages).is_none() {
                return Err(-EINVAL);
            }
            MemoryRegionChange::Create
        };

        if matches!(
            change,
            MemoryRegionChange::Create | MemoryRegionChange::Move
        ) && self.check_memslot_overlap(id, as_id, base_gfn, base_gfn + npages)
        {
            return Err(-EEXIST);
        }

        let new_slot = KvmMemorySlot {
            as_id,
            id,
            base_gfn,
            npages,
            flags: mem.flags,
            userspace_addr: mem.userspace_addr,
            guest_memfd: (mem.flags & KVM_MEM_GUEST_MEMFD != 0).then_some(mem.guest_memfd),
            guest_memfd_offset: mem.guest_memfd_offset,
        };

        match old_index {
            Some(index) => self.memslots[index] = new_slot,
            None => {
                self.nr_memslot_pages += npages;
                self.memslots.push(new_slot);
                self.memslots
                    .sort_by_key(|slot| (slot.as_id, slot.base_gfn));
            }
        }

        Ok(change)
    }

    pub fn vm_ioctl_set_memory_region(
        &mut self,
        mem: KvmUserspaceMemoryRegion2,
    ) -> Result<MemoryRegionChange, i32> {
        if (mem.slot as u16 as u32) >= KVM_USER_MEM_SLOTS {
            return Err(-EINVAL);
        }
        self.set_memory_region(mem)
    }

    pub fn check_memslot_overlap(&self, id: u32, as_id: u32, start: u64, end: u64) -> bool {
        self.memslots.iter().any(|slot| {
            slot.as_id == as_id && slot.id != id && start < slot.end_gfn() && slot.base_gfn < end
        })
    }

    pub fn are_all_memslots_empty(&self) -> bool {
        self.memslots.is_empty()
    }

    pub fn check_extension_generic(&self, cap: u32) -> i32 {
        match cap {
            KVM_CAP_SYNC_MMU | KVM_CAP_USER_MEMORY | KVM_CAP_USER_MEMORY2 | KVM_CAP_HALT_POLL => 1,
            KVM_CAP_NR_MEMSLOTS => KVM_USER_MEM_SLOTS as i32,
            KVM_CAP_DIRTY_LOG_RING => (KVM_DIRTY_RING_MAX_ENTRIES * KVM_DIRTY_GFN_SIZE) as i32,
            KVM_CAP_GUEST_MEMFD if self.guest_memfd_enabled => 1,
            KVM_CAP_GUEST_MEMFD_FLAGS if self.guest_memfd_enabled => {
                let mut flags = GUEST_MEMFD_FLAG_MMAP;
                if self.guest_memfd_init_shared_supported {
                    flags |= GUEST_MEMFD_FLAG_INIT_SHARED;
                }
                flags as i32
            }
            _ => 0,
        }
    }

    pub fn enable_halt_poll(&mut self, flags: u32, value: u64) -> Result<(), i32> {
        if flags != 0 || value != value as u32 as u64 {
            return Err(-EINVAL);
        }
        self.max_halt_poll_ns = value as u32;
        self.override_halt_poll_ns = true;
        Ok(())
    }

    pub fn enable_dirty_log_ring(&mut self, size: u32) -> Result<(), i32> {
        if size == 0 || !size.is_power_of_two() {
            return Err(-EINVAL);
        }
        if size < KVM_DIRTY_RING_RSVD_ENTRIES * KVM_DIRTY_GFN_SIZE || size < PAGE_SIZE as u32 {
            return Err(-EINVAL);
        }
        if size > KVM_DIRTY_RING_MAX_ENTRIES * KVM_DIRTY_GFN_SIZE {
            return Err(-E2BIG);
        }
        if self.dirty_ring_size != 0 || self.created_vcpus != 0 {
            return Err(-EINVAL);
        }
        self.dirty_ring_size = size;
        Ok(())
    }
}

pub const fn kvm_dev_ioctl_get_api_version(arg: u64) -> Result<i32, i32> {
    if arg != 0 {
        Err(-EINVAL)
    } else {
        Ok(KVM_API_VERSION)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmBus {
    Mmio,
    Pio,
    CoalescedMmio,
    FastMmio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmIoRange {
    pub addr: u64,
    pub len: u32,
    pub dev_id: u64,
    pub ioeventfd: bool,
    pub accepts_read: bool,
    pub accepts_write: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KvmIoBus {
    pub ranges: Vec<KvmIoRange>,
    pub ioeventfd_count: usize,
}

impl KvmIoBus {
    pub const fn new() -> Self {
        Self {
            ranges: Vec::new(),
            ioeventfd_count: 0,
        }
    }

    pub fn register_dev(&mut self, range: KvmIoRange) -> Result<(), i32> {
        if self.ranges.len().saturating_sub(self.ioeventfd_count) > NR_IOBUS_DEVS - 1
            && !range.ioeventfd
        {
            return Err(-ENOSPC);
        }

        let index = self
            .ranges
            .iter()
            .position(|existing| kvm_io_bus_cmp(existing, &range) == Ordering::Greater)
            .unwrap_or(self.ranges.len());
        if range.ioeventfd {
            self.ioeventfd_count = self.ioeventfd_count.saturating_add(1);
        }
        self.ranges.insert(index, range);
        Ok(())
    }

    pub fn unregister_dev(&mut self, dev_id: u64) -> Result<(), i32> {
        let Some(index) = self.ranges.iter().position(|range| range.dev_id == dev_id) else {
            return Ok(());
        };
        let range = self.ranges.remove(index);
        if range.ioeventfd {
            self.ioeventfd_count = self.ioeventfd_count.saturating_sub(1);
        }
        Ok(())
    }

    pub fn get_first_dev(&self, addr: u64, len: u32) -> Result<usize, i32> {
        let key = KvmIoRange {
            addr,
            len,
            dev_id: 0,
            ioeventfd: false,
            accepts_read: false,
            accepts_write: false,
        };
        let Some(mut index) = self
            .ranges
            .iter()
            .position(|range| kvm_io_bus_cmp(&key, range) == Ordering::Equal)
        else {
            return Err(-ENOENT);
        };

        while index > 0 && kvm_io_bus_cmp(&key, &self.ranges[index - 1]) == Ordering::Equal {
            index -= 1;
        }
        Ok(index)
    }

    pub fn write(&self, addr: u64, len: u32) -> Result<usize, i32> {
        let key = KvmIoRange {
            addr,
            len,
            dev_id: 0,
            ioeventfd: false,
            accepts_read: false,
            accepts_write: false,
        };
        let mut index = self.get_first_dev(addr, len).map_err(|_| -EOPNOTSUPP)?;
        while index < self.ranges.len()
            && kvm_io_bus_cmp(&key, &self.ranges[index]) == Ordering::Equal
        {
            if self.ranges[index].accepts_write {
                return Ok(index);
            }
            index += 1;
        }
        Err(-EOPNOTSUPP)
    }

    pub fn write_cookie(&self, addr: u64, len: u32, cookie: isize) -> Result<usize, i32> {
        let key = KvmIoRange {
            addr,
            len,
            dev_id: 0,
            ioeventfd: false,
            accepts_read: false,
            accepts_write: false,
        };
        if cookie >= 0 {
            let cookie = cookie as usize;
            if cookie < self.ranges.len()
                && kvm_io_bus_cmp(&key, &self.ranges[cookie]) == Ordering::Equal
                && self.ranges[cookie].accepts_write
            {
                return Ok(cookie);
            }
        }
        self.write(addr, len)
    }

    pub fn read(&self, addr: u64, len: u32) -> Result<usize, i32> {
        let key = KvmIoRange {
            addr,
            len,
            dev_id: 0,
            ioeventfd: false,
            accepts_read: false,
            accepts_write: false,
        };
        let mut index = self.get_first_dev(addr, len).map_err(|_| -EOPNOTSUPP)?;
        while index < self.ranges.len()
            && kvm_io_bus_cmp(&key, &self.ranges[index]) == Ordering::Equal
        {
            if self.ranges[index].accepts_read {
                return Ok(index);
            }
            index += 1;
        }
        Err(-EOPNOTSUPP)
    }
}

pub fn kvm_io_bus_cmp(left: &KvmIoRange, right: &KvmIoRange) -> Ordering {
    let mut addr1 = left.addr;
    let mut addr2 = right.addr;

    if addr1 < addr2 {
        return Ordering::Less;
    }

    if right.len != 0 {
        addr1 = addr1.saturating_add(left.len as u64);
        addr2 = addr2.saturating_add(right.len as u64);
    }

    if addr1 > addr2 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(slot: u32, flags: u32, gpa: u64, size: u64, hva: u64) -> KvmUserspaceMemoryRegion2 {
        KvmUserspaceMemoryRegion2 {
            slot,
            flags,
            guest_phys_addr: gpa,
            memory_size: size,
            userspace_addr: hva,
            guest_memfd_offset: 0,
            guest_memfd: 0,
        }
    }

    #[test]
    fn kvm_main_source_and_selftest_contract_match_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/kvm_main.c"
        ));
        assert!(source.contains("#define KVM_SET_USER_MEMORY_REGION_V1_FLAGS"));
        assert!(source.contains("static int check_memory_region_flags"));
        assert!(source.contains("if (mem->flags & KVM_MEM_GUEST_MEMFD)"));
        assert!(source.contains("if ((mem->memory_size & (PAGE_SIZE - 1))"));
        assert!(
            source.contains("kvm_check_memslot_overlap(slots, id, base_gfn, base_gfn + npages)")
        );
        assert!(source.contains("case KVM_GET_API_VERSION:"));
        assert!(source.contains("r = KVM_API_VERSION;"));
        assert!(source.contains("static inline int kvm_io_bus_cmp"));
        assert!(source.contains("If r2->len == 0, match the exact address"));
        assert!(source.contains("kvm_io_bus_write_cookie"));

        let set_memory_region_test = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/kvm/set_memory_region_test.c"
        ));
        assert!(
            set_memory_region_test.contains("Adding one more memory slot should fail with EINVAL")
        );
        assert!(set_memory_region_test.contains("KVM_MEM_GUEST_MEMFD"));
        assert!(set_memory_region_test.contains("Unaligned offset should fail"));

        let kvm_util = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/kvm/lib/kvm_util.c"
        ));
        assert!(kvm_util.contains("KVM_SET_USER_MEMORY_REGION2"));
        assert!(kvm_util.contains("guest_memfd_offset"));
        assert!(kvm_util.contains("KVM_CHECK_EXTENSION"));
    }

    #[test]
    fn memory_region_rejects_bad_alignment_flags_and_overlaps() {
        let mut kvm = Kvm::new();
        assert_eq!(
            kvm.set_memory_region(region(
                0,
                KVM_MEM_GUEST_MEMFD | KVM_MEM_LOG_DIRTY_PAGES,
                0,
                PAGE_SIZE,
                0
            )),
            Err(-EINVAL)
        );
        assert_eq!(
            kvm.set_memory_region(region(0, 0, 1, PAGE_SIZE, 0)),
            Err(-EINVAL)
        );

        assert_eq!(
            kvm.set_memory_region(region(0, 0, 0, PAGE_SIZE * 2, 0x1000_0000)),
            Ok(MemoryRegionChange::Create)
        );
        assert_eq!(
            kvm.set_memory_region(region(1, 0, PAGE_SIZE, PAGE_SIZE, 0x2000_0000)),
            Err(-EEXIST)
        );
        assert_eq!(kvm.nr_memslot_pages, 2);
    }

    #[test]
    fn memory_region_modify_move_flags_and_delete_follow_linux_rules() {
        let mut kvm = Kvm::new();
        kvm.set_memory_region(region(
            2,
            KVM_MEM_LOG_DIRTY_PAGES,
            0,
            PAGE_SIZE,
            0x3000_0000,
        ))
        .unwrap();
        assert_eq!(
            kvm.set_memory_region(region(
                2,
                KVM_MEM_LOG_DIRTY_PAGES,
                PAGE_SIZE * 4,
                PAGE_SIZE,
                0x3000_0000
            )),
            Ok(MemoryRegionChange::Move)
        );
        assert_eq!(
            kvm.set_memory_region(region(2, 0, PAGE_SIZE * 4, PAGE_SIZE, 0x3000_0000)),
            Ok(MemoryRegionChange::FlagsOnly)
        );
        assert_eq!(
            kvm.set_memory_region(region(2, 0, PAGE_SIZE * 4, PAGE_SIZE, 0x3000_0000)),
            Ok(MemoryRegionChange::Noop)
        );
        assert_eq!(
            kvm.set_memory_region(region(2, 0, PAGE_SIZE * 4, 0, 0)),
            Ok(MemoryRegionChange::Delete)
        );
        assert!(kvm.memslots.is_empty());
    }

    #[test]
    fn private_memslot_creation_uses_region2_offset_validation_and_is_immutable() {
        let mut kvm = Kvm::new();
        let mut private = region(3, KVM_MEM_GUEST_MEMFD, PAGE_SIZE * 8, PAGE_SIZE, 0);
        private.guest_memfd = 11;
        private.guest_memfd_offset = 1;
        assert_eq!(kvm.set_memory_region(private), Err(-EINVAL));

        private.guest_memfd_offset = PAGE_SIZE;
        assert_eq!(
            kvm.set_memory_region(private),
            Ok(MemoryRegionChange::Create)
        );
        assert_eq!(kvm.memslots[0].guest_memfd, Some(11));
        assert_eq!(kvm.set_memory_region(private), Err(-EINVAL));
    }

    #[test]
    fn capabilities_and_dirty_ring_enablement_match_generic_policy() {
        let mut kvm = Kvm::new();
        assert_eq!(kvm_dev_ioctl_get_api_version(0), Ok(12));
        assert_eq!(kvm_dev_ioctl_get_api_version(1), Err(-EINVAL));
        assert_eq!(kvm.check_extension_generic(KVM_CAP_USER_MEMORY2), 1);
        assert_eq!(
            kvm.check_extension_generic(KVM_CAP_NR_MEMSLOTS),
            KVM_USER_MEM_SLOTS as i32
        );
        assert_eq!(
            kvm.check_extension_generic(KVM_CAP_GUEST_MEMFD_FLAGS),
            (GUEST_MEMFD_FLAG_MMAP | GUEST_MEMFD_FLAG_INIT_SHARED) as i32
        );
        assert_eq!(kvm.enable_halt_poll(0, 50_000), Ok(()));
        assert!(kvm.override_halt_poll_ns);
        assert_eq!(kvm.enable_dirty_log_ring(0), Err(-EINVAL));
        assert_eq!(kvm.enable_dirty_log_ring(4096), Ok(()));
        assert_eq!(kvm.enable_dirty_log_ring(4096), Err(-EINVAL));
    }

    #[test]
    fn io_bus_cmp_exact_zero_length_and_overlap_ranges_match_linux() {
        let key = KvmIoRange {
            addr: 0x1004,
            len: 4,
            dev_id: 0,
            ioeventfd: false,
            accepts_read: true,
            accepts_write: true,
        };
        let exact = KvmIoRange { dev_id: 1, ..key };
        let zero_len = KvmIoRange {
            addr: 0x1004,
            len: 0,
            dev_id: 2,
            ioeventfd: false,
            accepts_read: true,
            accepts_write: true,
        };
        let overlap = KvmIoRange {
            addr: 0x1000,
            len: 8,
            dev_id: 3,
            ioeventfd: false,
            accepts_read: true,
            accepts_write: true,
        };

        assert_eq!(kvm_io_bus_cmp(&key, &exact), Ordering::Equal);
        assert_eq!(kvm_io_bus_cmp(&key, &zero_len), Ordering::Equal);
        assert_eq!(kvm_io_bus_cmp(&key, &overlap), Ordering::Equal);
    }

    #[test]
    fn io_bus_registers_sorted_and_cookie_falls_back_to_search() {
        let mut bus = KvmIoBus::new();
        bus.register_dev(KvmIoRange {
            addr: 0x2000,
            len: 4,
            dev_id: 2,
            ioeventfd: false,
            accepts_read: false,
            accepts_write: true,
        })
        .unwrap();
        bus.register_dev(KvmIoRange {
            addr: 0x1000,
            len: 4,
            dev_id: 1,
            ioeventfd: false,
            accepts_read: true,
            accepts_write: false,
        })
        .unwrap();

        assert_eq!(bus.ranges[0].dev_id, 1);
        assert_eq!(bus.read(0x1000, 4), Ok(0));
        assert_eq!(bus.write(0x1000, 4), Err(-EOPNOTSUPP));
        assert_eq!(bus.write_cookie(0x2000, 4, -1), Ok(1));
        assert_eq!(bus.write_cookie(0x2000, 4, 0), Ok(1));
        assert_eq!(bus.unregister_dev(1), Ok(()));
        assert_eq!(bus.get_first_dev(0x1000, 4), Err(-ENOENT));
    }
}
