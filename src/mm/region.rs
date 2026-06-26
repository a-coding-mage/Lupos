//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Physical memory region map — the kernel's view of what physical memory
/// is available, reserved, or otherwise categorized.
///
/// This is the lupos equivalent of Linux's `memblock` subsystem.  Linux's
/// `memblock` maintains two lists of physical memory regions: `memory`
/// (all usable RAM) and `reserved` (kernel, device, firmware reservations).
/// We simplify this into a single sorted list with typed regions, which is
/// sufficient for early boot memory management.
///
/// The memory map is populated from the Linux `boot_params` E820 table, which
/// originates from the firmware memory map gathered by the boot path.
///
/// Key design decision: we use a fixed-size array (no heap allocation needed)
/// just like Linux's initial memblock which uses `INIT_MEMBLOCK_REGIONS = 128`
/// statically-allocated region slots.
///
/// Ref: Linux include/linux/memblock.h — struct memblock_region, memblock_type
///      Linux mm/memblock.c — memblock_add_range(), memblock_reserve()
///      Linux arch/x86/kernel/e820.c — e820__memblock_setup()
use crate::arch::x86::include::uapi::asm::bootparam::BootParams;

/// Maximum number of physical memory regions we can track.
///
/// Linux uses INIT_MEMBLOCK_REGIONS = 128.  Most x86 systems report
/// 10–30 E820 regions, so 128 is generous even after splitting.
pub const MAX_REGIONS: usize = 128;

/// Type of a physical memory region.
///
/// Maps to E820 memory map entry types.
///
/// Ref: Linux include/uapi/asm-generic/e820.h — E820_TYPE_*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RegionType {
    /// Usable RAM — can be allocated by the kernel.
    Available = 1,
    /// Reserved by firmware or hardware — must not be touched.
    Reserved = 2,
    /// ACPI reclaimable — usable after ACPI tables are no longer needed.
    AcpiReclaimable = 3,
    /// ACPI Non-Volatile Storage — must be preserved across sleep states.
    AcpiNvs = 4,
    /// Defective RAM — hardware-reported bad memory.
    Defective = 5,
}

impl RegionType {
    /// Convert from the raw E820 region type integer.
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => RegionType::Available,
            2 => RegionType::Reserved,
            3 => RegionType::AcpiReclaimable,
            4 => RegionType::AcpiNvs,
            5 => RegionType::Defective,
            _ => RegionType::Reserved, // treat unknown types as reserved (safe default)
        }
    }
}

/// A single physical memory region.
///
/// Mirrors Linux's `struct memblock_region` (include/linux/memblock.h):
/// ```c
/// struct memblock_region {
///     phys_addr_t base;
///     phys_addr_t size;
///     enum memblock_flags flags;
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PhysRegion {
    /// Physical base address of the region.
    pub base: u64,
    /// Size of the region in bytes.
    pub size: u64,
    /// Type of the region (available, reserved, etc.).
    pub region_type: RegionType,
}

impl PhysRegion {
    /// End address (exclusive) of this region.
    pub fn end(&self) -> u64 {
        self.base + self.size
    }
}

/// The kernel's physical memory map — a fixed-size collection of regions.
///
/// Mirrors Linux's `struct memblock_type` which holds an array of
/// `memblock_region` entries. Populated from `boot_params` during early boot.
///
/// Ref: Linux include/linux/memblock.h — struct memblock_type
pub struct MemoryMap {
    regions: [PhysRegion; MAX_REGIONS],
    count: usize,
}

impl MemoryMap {
    /// Create an empty memory map.
    pub const fn new() -> Self {
        Self {
            regions: [PhysRegion {
                base: 0,
                size: 0,
                region_type: RegionType::Reserved,
            }; MAX_REGIONS],
            count: 0,
        }
    }

    /// Populate the memory map from a Linux boot_params E820 table.
    pub fn from_boot_params(bp: &BootParams) -> Self {
        let mut map = Self::new();
        for (idx, entry) in bp.e820_iter().enumerate() {
            if idx >= MAX_REGIONS {
                break;
            }
            map.regions[idx] = PhysRegion {
                base: entry.base_addr,
                size: entry.length,
                region_type: RegionType::from_u32(entry.region_type),
            };
            map.count += 1;
        }
        map
    }

    /// Number of regions in the map.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Set the number of valid regions (for test construction).
    pub fn set_count(&mut self, count: usize) {
        self.count = count;
    }

    /// Access all regions as a slice.
    pub fn regions(&self) -> &[PhysRegion] {
        &self.regions[..self.count]
    }

    /// Mutable access to the backing array (for test construction).
    pub fn regions_mut(&mut self) -> &mut [PhysRegion; MAX_REGIONS] {
        &mut self.regions
    }

    /// Iterate over only the available (usable RAM) regions.
    pub fn available_regions(&self) -> impl Iterator<Item = &PhysRegion> {
        self.regions()
            .iter()
            .filter(|r| r.region_type == RegionType::Available)
    }

    /// Total available (usable) physical memory in bytes.
    pub fn total_available(&self) -> u64 {
        self.available_regions().map(|r| r.size).sum()
    }

    /// Mark a range of physical memory as reserved.
    ///
    /// This is used to reserve the kernel's own memory (between `_kernel_start`
    /// and `_kernel_end`) so the frame allocator knows not to hand it out.
    ///
    /// If the reserved range falls within an available region, that region is
    /// split: the overlapping portion becomes Reserved, and the remaining parts
    /// (before and after) stay Available.
    ///
    /// Mirrors Linux's `memblock_reserve()` in mm/memblock.c.
    pub fn mark_reserved(&mut self, base: u64, size: u64) {
        let end = base + size;
        let mut i = 0;

        while i < self.count {
            let region = self.regions[i];

            // Only split available regions that overlap with the reserved range
            if region.region_type != RegionType::Available {
                i += 1;
                continue;
            }

            let r_end = region.end();

            // No overlap: region is entirely before or after the reserved range
            if region.base >= end || r_end <= base {
                i += 1;
                continue;
            }

            // The region overlaps with [base, end).  We may need to split it
            // into up to three parts: [region.base, base) + [base, end) + [end, r_end)
            //
            // Case 1: reserved range covers the entire region → just change type
            // Case 2: reserved range starts after region start → split off front
            // Case 3: reserved range ends before region end → split off back
            // Case 4: both front and back remain → two splits

            let has_front = region.base < base;
            let has_back = r_end > end;

            if !has_front && !has_back {
                // Reserved range covers entire region
                self.regions[i].region_type = RegionType::Reserved;
                i += 1;
            } else if has_front && !has_back {
                // Front portion remains available, rest is reserved
                self.regions[i].size = base - region.base;
                self.insert_at(
                    i + 1,
                    PhysRegion {
                        base,
                        size: r_end - base,
                        region_type: RegionType::Reserved,
                    },
                );
                i += 2;
            } else if !has_front && has_back {
                // Back portion remains available, front is reserved
                self.regions[i] = PhysRegion {
                    base: region.base,
                    size: end - region.base,
                    region_type: RegionType::Reserved,
                };
                self.insert_at(
                    i + 1,
                    PhysRegion {
                        base: end,
                        size: r_end - end,
                        region_type: RegionType::Available,
                    },
                );
                i += 2;
            } else {
                // Both front and back remain — need two new regions
                // [region.base, base) = available (shrink original)
                self.regions[i].size = base - region.base;
                // [base, end) = reserved
                self.insert_at(
                    i + 1,
                    PhysRegion {
                        base,
                        size: end - base,
                        region_type: RegionType::Reserved,
                    },
                );
                // [end, r_end) = available
                self.insert_at(
                    i + 2,
                    PhysRegion {
                        base: end,
                        size: r_end - end,
                        region_type: RegionType::Available,
                    },
                );
                i += 3;
            }
        }
    }

    /// Insert a region at index `idx`, shifting subsequent regions right.
    fn insert_at(&mut self, idx: usize, region: PhysRegion) {
        if self.count >= MAX_REGIONS {
            return; // drop the region if the array is full
        }
        // Shift elements [idx..count) right by one
        let mut j = self.count;
        while j > idx {
            self.regions[j] = self.regions[j - 1];
            j -= 1;
        }
        self.regions[idx] = region;
        self.count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_map_has_zero_available() {
        let map = MemoryMap::new();
        assert_eq!(map.count(), 0);
        assert_eq!(map.total_available(), 0);
    }

    fn make_test_map() -> MemoryMap {
        let mut map = MemoryMap::new();
        // Region 0: 1 MiB available starting at 1 MiB
        map.regions[0] = PhysRegion {
            base: 0x100000,
            size: 0x100000,
            region_type: RegionType::Available,
        };
        // Region 1: 64 KiB reserved at 0xE0000
        map.regions[1] = PhysRegion {
            base: 0xE0000,
            size: 0x10000,
            region_type: RegionType::Reserved,
        };
        // Region 2: 126 MiB available starting at 2 MiB
        map.regions[2] = PhysRegion {
            base: 0x200000,
            size: 126 * 1024 * 1024,
            region_type: RegionType::Available,
        };
        map.count = 3;
        map
    }

    #[test]
    fn total_available_sums_only_available_regions() {
        let map = make_test_map();
        // 1 MiB + 126 MiB = 127 MiB
        let expected = 0x100000u64 + 126 * 1024 * 1024;
        assert_eq!(map.total_available(), expected);
    }

    #[test]
    fn available_regions_skips_reserved() {
        let map = make_test_map();
        let available: usize = map.available_regions().count();
        assert_eq!(available, 2); // regions 0 and 2
    }

    #[test]
    fn mark_reserved_splits_middle_of_region() {
        // Start with one big available region: 0x100000 - 0x200000 (1 MiB)
        let mut map = MemoryMap::new();
        map.regions[0] = PhysRegion {
            base: 0x100000,
            size: 0x100000,
            region_type: RegionType::Available,
        };
        map.count = 1;

        // Reserve 4 KiB in the middle: 0x180000 - 0x181000
        map.mark_reserved(0x180000, 0x1000);

        // Should now have 3 regions:
        // [0x100000, 0x180000) = available (512 KiB)
        // [0x180000, 0x181000) = reserved  (4 KiB)
        // [0x181000, 0x200000) = available (508 KiB - 4 KiB)
        assert_eq!(map.count(), 3);

        assert_eq!(map.regions[0].base, 0x100000);
        assert_eq!(map.regions[0].size, 0x80000);
        assert_eq!(map.regions[0].region_type, RegionType::Available);

        assert_eq!(map.regions[1].base, 0x180000);
        assert_eq!(map.regions[1].size, 0x1000);
        assert_eq!(map.regions[1].region_type, RegionType::Reserved);

        assert_eq!(map.regions[2].base, 0x181000);
        assert_eq!(map.regions[2].size, 0x200000 - 0x181000);
        assert_eq!(map.regions[2].region_type, RegionType::Available);
    }

    #[test]
    fn mark_reserved_at_region_start() {
        let mut map = MemoryMap::new();
        map.regions[0] = PhysRegion {
            base: 0x100000,
            size: 0x100000,
            region_type: RegionType::Available,
        };
        map.count = 1;

        // Reserve the first 8 KiB
        map.mark_reserved(0x100000, 0x2000);

        assert_eq!(map.count(), 2);
        assert_eq!(map.regions[0].region_type, RegionType::Reserved);
        assert_eq!(map.regions[0].base, 0x100000);
        assert_eq!(map.regions[0].size, 0x2000);

        assert_eq!(map.regions[1].region_type, RegionType::Available);
        assert_eq!(map.regions[1].base, 0x102000);
    }

    #[test]
    fn mark_reserved_at_region_end() {
        let mut map = MemoryMap::new();
        map.regions[0] = PhysRegion {
            base: 0x100000,
            size: 0x100000,
            region_type: RegionType::Available,
        };
        map.count = 1;

        // Reserve the last 8 KiB
        map.mark_reserved(0x1FE000, 0x2000);

        assert_eq!(map.count(), 2);
        assert_eq!(map.regions[0].region_type, RegionType::Available);
        assert_eq!(map.regions[0].base, 0x100000);
        assert_eq!(map.regions[0].size, 0xFE000);

        assert_eq!(map.regions[1].region_type, RegionType::Reserved);
        assert_eq!(map.regions[1].base, 0x1FE000);
    }

    #[test]
    fn mark_reserved_entire_region() {
        let mut map = MemoryMap::new();
        map.regions[0] = PhysRegion {
            base: 0x100000,
            size: 0x100000,
            region_type: RegionType::Available,
        };
        map.count = 1;

        map.mark_reserved(0x100000, 0x100000);

        assert_eq!(map.count(), 1);
        assert_eq!(map.regions[0].region_type, RegionType::Reserved);
    }

    #[test]
    fn mark_reserved_skips_already_reserved() {
        let mut map = MemoryMap::new();
        map.regions[0] = PhysRegion {
            base: 0xE0000,
            size: 0x10000,
            region_type: RegionType::Reserved,
        };
        map.count = 1;

        // Try to reserve within an already-reserved region — should be a no-op
        map.mark_reserved(0xE0000, 0x1000);
        assert_eq!(map.count(), 1);
        assert_eq!(map.regions[0].region_type, RegionType::Reserved);
    }

    #[test]
    fn region_type_from_u32_handles_unknown() {
        assert_eq!(RegionType::from_u32(1), RegionType::Available);
        assert_eq!(RegionType::from_u32(2), RegionType::Reserved);
        assert_eq!(RegionType::from_u32(99), RegionType::Reserved); // unknown → reserved
    }
}
