//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Physical frame types and constants, plus the legacy bitmap frame allocator.
///
/// The `PhysFrame` type and `PAGE_SIZE` constant are always available.
/// The `BitmapFrameAllocator` is gated behind the `bitmap-alloc` feature
/// and is superseded by the buddy allocator (`memory::buddy`) as of Milestone 7.
///
/// Ref: Linux include/linux/pfn.h — PFN_UP, PFN_DOWN, PFN_PHYS
///      Linux include/asm-generic/page.h — PAGE_SIZE
///      https://wiki.osdev.org/Page_Frame_Allocation
#[cfg(feature = "bitmap-alloc")]
use crate::mm::region::MemoryMap;

/// Page size — the fundamental unit of physical memory allocation.
///
/// x86/x86_64 supports 4 KiB pages (and 2 MiB / 1 GiB huge pages).
/// We use 4 KiB as the base allocation granularity, matching Linux's PAGE_SIZE.
///
/// Ref: Linux include/asm-generic/page.h — PAGE_SIZE
pub const PAGE_SIZE: usize = 4096;

#[cfg(feature = "bitmap-alloc")]
const MAX_PHYS_MEMORY: usize = 64 * 1024 * 1024 * 1024;
#[cfg(feature = "bitmap-alloc")]
const MAX_FRAMES: usize = MAX_PHYS_MEMORY / PAGE_SIZE;
#[cfg(feature = "bitmap-alloc")]
const BITMAP_SIZE: usize = MAX_FRAMES / 8;

/// A physical frame number (PFN) — an index into physical memory at
/// PAGE_SIZE granularity.
///
/// Equivalent to Linux's PFN concept (include/linux/pfn.h):
///   PFN = physical_address >> PAGE_SHIFT
///
/// Ref: Linux include/linux/pfn.h — PFN_UP, PFN_DOWN, PFN_PHYS
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysFrame(pub u64);

impl PhysFrame {
    /// Get the frame containing the given physical address (round down).
    pub fn containing_address(addr: u64) -> Self {
        PhysFrame(addr / PAGE_SIZE as u64)
    }

    /// Get the physical start address of this frame.
    pub fn start_address(&self) -> u64 {
        self.0 * PAGE_SIZE as u64
    }
}

#[cfg(feature = "bitmap-alloc")]
static mut FRAME_BITMAP: [u8; BITMAP_SIZE] = [0xFF; BITMAP_SIZE];

#[cfg(feature = "bitmap-alloc")]
fn bitmap_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(FRAME_BITMAP) as *mut u8
}

#[cfg(feature = "bitmap-alloc")]
fn bitmap_read(byte_idx: usize) -> u8 {
    unsafe { bitmap_ptr().add(byte_idx).read_volatile() }
}

#[cfg(feature = "bitmap-alloc")]
fn bitmap_write(byte_idx: usize, val: u8) {
    unsafe {
        bitmap_ptr().add(byte_idx).write_volatile(val);
    }
}

/// Legacy bitmap-based physical frame allocator (superseded by buddy allocator).
#[cfg(feature = "bitmap-alloc")]
pub struct BitmapFrameAllocator {
    /// Total number of frames tracked (may be less than MAX_FRAMES
    /// if the system has less than 1 GiB of RAM).
    total_frames: usize,
    /// Number of currently free frames.
    free_count: usize,
}

#[cfg(feature = "bitmap-alloc")]
impl BitmapFrameAllocator {
    /// Create a new (uninitialized) frame allocator.
    pub const fn new() -> Self {
        Self {
            total_frames: 0,
            free_count: 0,
        }
    }

    /// Initialize the frame allocator from the physical memory map.
    ///
    /// Walks the memory map and marks all frames within "available" regions
    /// as free.  Frames in reserved, ACPI, or defective regions remain
    /// allocated (their bits stay as 1).
    ///
    /// After freeing available frames, re-reserves the kernel's own memory
    /// (passed as `kernel_start`/`kernel_end`) and the first 1 MiB (which
    /// contains BIOS data structures, VGA buffer, etc.).
    ///
    /// Ref: Linux mm/memblock.c — memblock_free_all(), __free_pages_memory()
    pub fn init(&mut self, memory_map: &MemoryMap, kernel_start: u64, kernel_end: u64) {
        // Determine the highest physical address to know how many frames to track
        let mut max_addr: u64 = 0;
        for region in memory_map.regions() {
            let end = region.base + region.size;
            if end > max_addr {
                max_addr = end;
            }
        }

        // Cap at our bitmap's maximum coverage
        if max_addr > MAX_PHYS_MEMORY as u64 {
            max_addr = MAX_PHYS_MEMORY as u64;
        }

        self.total_frames = (max_addr as usize) / PAGE_SIZE;
        self.free_count = 0;

        // Free frames in available regions
        for region in memory_map.available_regions() {
            let start_frame = align_up_div(region.base, PAGE_SIZE as u64);
            let end_frame = region.end() / PAGE_SIZE as u64;

            for frame in start_frame..end_frame {
                if (frame as usize) < self.total_frames {
                    self.clear_bit(frame as usize);
                    self.free_count += 1;
                }
            }
        }

        // Re-reserve the first 1 MiB — contains BIOS data, IVT, VGA buffer, etc.
        // Linux does the same: memblock_reserve(0, SZ_1M) in setup_arch().
        self.reserve_range(0, 0x100000);

        // Re-reserve the kernel's own physical memory
        self.reserve_range(kernel_start, kernel_end);
    }

    /// Allocate a single physical frame, returning its PFN.
    ///
    /// Scans the bitmap for the first free bit (0), marks it as allocated (1),
    /// and returns the corresponding PhysFrame.  Returns None if all frames
    /// are in use.
    ///
    /// This is O(n) in the worst case.  A more sophisticated allocator would
    /// maintain a free-list or use the buddy system.
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        for byte_idx in 0..(self.total_frames / 8) {
            let byte = bitmap_read(byte_idx);
            if byte == 0xFF {
                continue; // all 8 frames in this byte are allocated
            }

            // Find the first free bit in this byte
            for bit in 0..8u8 {
                if byte & (1 << bit) == 0 {
                    let frame_idx = byte_idx * 8 + bit as usize;
                    if frame_idx >= self.total_frames {
                        return None;
                    }
                    self.set_bit(frame_idx);
                    self.free_count -= 1;
                    return Some(PhysFrame(frame_idx as u64));
                }
            }
        }
        None
    }

    /// Deallocate a physical frame, returning it to the free pool.
    ///
    /// # Safety (logical)
    /// The caller must ensure that the frame is no longer in use.
    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        let idx = frame.0 as usize;
        if idx < self.total_frames {
            self.clear_bit(idx);
            self.free_count += 1;
        }
    }

    /// Allocate `count` contiguous physical frames.
    ///
    /// Returns the first frame of the contiguous block, or None if no
    /// sufficiently large contiguous free region exists.
    ///
    /// Used by the heap allocator to obtain a contiguous physical region
    /// for the kernel heap.
    pub fn allocate_contiguous(&mut self, count: usize) -> Option<PhysFrame> {
        if count == 0 {
            return None;
        }

        let mut run_start = 0;
        let mut run_length = 0;

        for frame_idx in 0..self.total_frames {
            if self.is_free(frame_idx) {
                if run_length == 0 {
                    run_start = frame_idx;
                }
                run_length += 1;
                if run_length == count {
                    // Found a contiguous block — mark all frames as allocated
                    for i in run_start..run_start + count {
                        self.set_bit(i);
                        self.free_count -= 1;
                    }
                    return Some(PhysFrame(run_start as u64));
                }
            } else {
                run_length = 0;
            }
        }
        None
    }

    /// Number of currently free frames.
    pub fn free_count(&self) -> usize {
        self.free_count
    }

    /// Total number of frames being tracked.
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    /// Reserve a range of physical addresses (mark frames as allocated).
    fn reserve_range(&mut self, start: u64, end: u64) {
        let start_frame = (start / PAGE_SIZE as u64) as usize;
        let end_frame = align_up_div(end, PAGE_SIZE as u64) as usize;

        for frame_idx in start_frame..end_frame.min(self.total_frames) {
            if self.is_free(frame_idx) {
                self.set_bit(frame_idx);
                self.free_count -= 1;
            }
        }
    }

    /// Check if a frame is free (bit = 0).
    fn is_free(&self, frame_idx: usize) -> bool {
        let byte_idx = frame_idx / 8;
        let bit_idx = frame_idx % 8;
        (bitmap_read(byte_idx) & (1 << bit_idx)) == 0
    }

    /// Mark a frame as allocated (set bit to 1).
    fn set_bit(&self, frame_idx: usize) {
        let byte_idx = frame_idx / 8;
        let bit_idx = frame_idx % 8;
        bitmap_write(byte_idx, bitmap_read(byte_idx) | (1 << bit_idx));
    }

    /// Mark a frame as free (clear bit to 0).
    fn clear_bit(&self, frame_idx: usize) {
        let byte_idx = frame_idx / 8;
        let bit_idx = frame_idx % 8;
        bitmap_write(byte_idx, bitmap_read(byte_idx) & !(1 << bit_idx));
    }
}

/// Divide `value` by `divisor`, rounding up.
const fn align_up_div(value: u64, divisor: u64) -> u64 {
    (value + divisor - 1) / divisor
}

#[cfg(all(test, feature = "bitmap-alloc"))]
mod tests {
    use super::*;
    use crate::mm::region::{MemoryMap, PhysRegion, RegionType};

    /// Reset the bitmap to all-allocated before each test.
    fn reset_bitmap() {
        for i in 0..BITMAP_SIZE {
            bitmap_write(i, 0xFF);
        }
    }

    /// Build a simple memory map for testing:
    ///   - 0x000000 - 0x100000: reserved (first 1 MiB)
    ///   - 0x100000 - 0x200000: available (1 MiB = 256 frames)
    fn make_test_map() -> MemoryMap {
        let mut map = MemoryMap::new();
        // First 1 MiB reserved
        map.regions_mut()[0] = PhysRegion {
            base: 0x000000,
            size: 0x100000,
            region_type: RegionType::Reserved,
        };
        // 1 MiB available starting at 1 MiB
        map.regions_mut()[1] = PhysRegion {
            base: 0x100000,
            size: 0x100000,
            region_type: RegionType::Available,
        };
        map.set_count(2);
        map
    }

    #[test]
    fn init_frees_available_frames() {
        reset_bitmap();
        let map = make_test_map();
        let mut alloc = BitmapFrameAllocator::new();
        // Use kernel range that doesn't overlap the available region
        alloc.init(&map, 0x100000, 0x110000);

        // 256 frames available, minus first 1 MiB reserved (re-reserved),
        // minus kernel (0x100000-0x110000 = 16 frames)
        // Available region is 0x100000-0x200000 = 256 frames
        // Kernel occupies 0x100000-0x110000 = 16 frames of those
        // So free_count = 256 - 16 = 240
        assert_eq!(alloc.free_count(), 240);
    }

    #[test]
    fn allocate_returns_frame() {
        reset_bitmap();
        let map = make_test_map();
        let mut alloc = BitmapFrameAllocator::new();
        alloc.init(&map, 0x100000, 0x110000);

        let frame = alloc.allocate_frame();
        assert!(frame.is_some());

        // First free frame should be at 0x110000 (after kernel)
        let frame = frame.unwrap();
        assert_eq!(frame.start_address(), 0x110000);
    }

    #[test]
    fn allocate_and_deallocate() {
        reset_bitmap();
        let map = make_test_map();
        let mut alloc = BitmapFrameAllocator::new();
        alloc.init(&map, 0x100000, 0x110000);

        let initial_free = alloc.free_count();
        let frame = alloc.allocate_frame().unwrap();
        assert_eq!(alloc.free_count(), initial_free - 1);

        alloc.deallocate_frame(frame);
        assert_eq!(alloc.free_count(), initial_free);
    }

    #[test]
    fn allocate_contiguous_returns_block() {
        reset_bitmap();
        let map = make_test_map();
        let mut alloc = BitmapFrameAllocator::new();
        alloc.init(&map, 0x100000, 0x110000);

        let initial_free = alloc.free_count();
        let block = alloc.allocate_contiguous(4);
        assert!(block.is_some());

        let block = block.unwrap();
        assert_eq!(block.start_address(), 0x110000);
        assert_eq!(alloc.free_count(), initial_free - 4);
    }

    #[test]
    fn allocate_contiguous_returns_none_when_exhausted() {
        reset_bitmap();
        let map = make_test_map();
        let mut alloc = BitmapFrameAllocator::new();
        alloc.init(&map, 0x100000, 0x110000);

        // Try to allocate more frames than available
        let block = alloc.allocate_contiguous(300);
        assert!(block.is_none());
    }

    #[test]
    fn phys_frame_conversions() {
        let frame = PhysFrame::containing_address(0x12345);
        assert_eq!(frame.0, 0x12); // 0x12345 / 4096 = 0x12
        assert_eq!(frame.start_address(), 0x12000);
    }
}
