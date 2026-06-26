//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Kernel heap allocator — linked-list free-list implementing GlobalAlloc.
///
/// Provides dynamic memory allocation (`Box`, `Vec`, `String`) for kernel code
/// using a simple free-list allocator backed by physical frames from the
/// bitmap frame allocator.
///
/// # Design
///
/// The allocator maintains a singly-linked list of free memory blocks.  Each
/// free block stores its size and a pointer to the next free block.  On
/// allocation, the list is searched for a block large enough to satisfy the
/// request (first-fit).  On deallocation, the block is prepended to the free
/// list.
///
/// This is simpler than Linux's SLUB allocator but adequate for early boot.
/// Linux's early boot uses `memblock_alloc()` (a bump allocator) for initial
/// allocations, then switches to kmalloc/SLUB once the memory subsystem is
/// fully initialized.
///
/// # Identity Mapping
///
/// At this stage, the heap lives in identity-mapped physical memory (the
/// first 1 GiB mapped by arch/x86/boot/header.S).  The heap's physical address IS its
/// virtual address.
///
/// Ref: Linux mm/slub.c — kmalloc()
///      Linux mm/memblock.c — memblock_alloc()
///      https://wiki.osdev.org/Kernel_Heap
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use spin::Mutex;

/// Default kernel heap size (64 KiB).
///
/// Sufficient for early boot allocations.  Can be grown later by
/// requesting more frames from the frame allocator.
pub const INITIAL_HEAP_SIZE: usize = 64 * 1024;

/// A node in the free list.  Each free block starts with this header.
///
/// The minimum allocation size is `size_of::<FreeNode>()` (16 bytes on
/// 64-bit) since every freed block must be large enough to hold this header.
struct FreeNode {
    size: usize,
    next: Option<ptr::NonNull<FreeNode>>,
}

impl FreeNode {
    const fn min_size() -> usize {
        core::mem::size_of::<FreeNode>()
    }
}

/// Linked-list heap allocator.
///
/// The `head` is a dummy node whose `next` points to the first real free block.
/// This avoids special-casing an empty list.
pub struct LinkedListAllocator {
    head: FreeNode,
}

// Safety: The allocator is only accessed behind a spin Mutex which ensures
// exclusive access.  The raw pointers in FreeNode are only dereferenced
// while the lock is held.
unsafe impl Send for LinkedListAllocator {}

impl LinkedListAllocator {
    /// Create a new, uninitialized allocator.
    pub const fn new() -> Self {
        Self {
            head: FreeNode {
                size: 0,
                next: None,
            },
        }
    }

    /// Initialize the allocator with a contiguous memory region.
    ///
    /// `heap_start` is the virtual (= physical, since identity-mapped) address
    /// of the heap region.  `heap_size` is its size in bytes.
    ///
    /// # Safety
    /// The memory region `[heap_start, heap_start + heap_size)` must be valid,
    /// writable, and not used for anything else.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_region(heap_start, heap_size);
        }
    }

    /// Add a free region to the allocator's free list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // Align the address up to FreeNode alignment
        let aligned_addr = align_up(addr, core::mem::align_of::<FreeNode>());
        let adjusted_size = size - (aligned_addr - addr);

        // Don't add regions that are too small to hold a FreeNode
        if adjusted_size < FreeNode::min_size() {
            return;
        }

        // Write a new FreeNode at the start of the region
        let node_ptr = aligned_addr as *mut FreeNode;
        unsafe {
            node_ptr.write(FreeNode {
                size: adjusted_size,
                next: self.head.next,
            });
        }
        self.head.next = Some(unsafe { ptr::NonNull::new_unchecked(node_ptr) });
    }

    /// Find a free block large enough for the given layout.
    ///
    /// Returns a mutable reference to the _previous_ node (so we can unlink
    /// the found node) and the found node's address and size.
    fn find_free_block(
        &mut self,
        size: usize,
        align: usize,
    ) -> Option<(*mut FreeNode, usize, usize)> {
        let mut current = &mut self.head as *mut FreeNode;

        unsafe {
            while let Some(next_ptr) = (*current).next {
                let next = next_ptr.as_ptr();
                let next_addr = next as usize;
                let next_size = (*next).size;

                // Calculate the aligned start within this block
                let alloc_start = align_up(next_addr, align);
                let alloc_end = alloc_start + size;

                if alloc_end <= next_addr + next_size {
                    // This block is large enough — unlink it from the list
                    (*current).next = (*next).next;
                    return Some((next, alloc_start, next_size));
                }

                current = next;
            }
        }
        None
    }
}

/// Wrapper that puts the allocator behind a spin mutex, required by GlobalAlloc.
///
/// GlobalAlloc requires &self (not &mut self), so we wrap in a Mutex for
/// interior mutability.
pub struct Locked<T> {
    inner: Mutex<T>,
}

impl<T> Locked<T> {
    pub const fn new(val: T) -> Self {
        Self {
            inner: Mutex::new(val),
        }
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Ensure minimum allocation size can hold a FreeNode (for dealloc)
        let size = layout.size().max(FreeNode::min_size());
        let align = layout.align().max(core::mem::align_of::<FreeNode>());

        let mut allocator = self.inner.lock();

        if let Some((block_ptr, alloc_start, block_size)) = allocator.find_free_block(size, align) {
            let alloc_end = alloc_start + size;
            let excess = block_size - (alloc_end - block_ptr as usize);

            // If there's enough leftover space, split the block
            if excess >= FreeNode::min_size() {
                unsafe {
                    allocator.add_free_region(alloc_end, excess);
                }
            }

            alloc_start as *mut u8
        } else {
            ptr::null_mut() // OOM
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(FreeNode::min_size());
        let mut allocator = self.inner.lock();
        unsafe {
            allocator.add_free_region(ptr as usize, size);
        }
    }
}

/// The global kernel heap allocator (legacy linked-list allocator).
///
/// Registered with `#[global_allocator]` only when the `slab-alloc` feature
/// is *not* active — once `slab-alloc` is enabled `memory::slab` registers
/// its own `SlabGlobalAlloc` instead.  Host unit tests always use the system
/// allocator via the `alloc` crate's test harness.
///
/// Ref: Linux `mm/memblock.c` — early-boot bump allocator (analogous role)
#[cfg(all(not(test), not(feature = "slab-alloc")))]
#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

#[cfg(any(test, feature = "slab-alloc"))]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

/// Initialize the kernel heap.
///
/// Must be called once during boot after the frame allocator is ready.
/// `heap_start` and `heap_size` define the contiguous physical memory
/// region to use as the heap.
///
/// # Safety
/// The memory region must be valid, writable, and identity-mapped.
pub unsafe fn init(heap_start: usize, heap_size: usize) {
    unsafe {
        ALLOCATOR.inner.lock().init(heap_start, heap_size);
    }
}

/// Round `value` up to the next multiple of `align` (must be a power of two).
const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test the allocator using a static buffer as the heap.
    /// This avoids needing the frame allocator during tests.
    #[repr(align(4096))]
    struct TestHeap([u8; 4096]);
    static mut TEST_HEAP: TestHeap = TestHeap([0; 4096]);

    fn test_heap_ptr() -> *mut u8 {
        core::ptr::addr_of_mut!(TEST_HEAP) as *mut u8
    }

    #[test]
    fn allocate_and_deallocate() {
        let mut alloc = LinkedListAllocator::new();
        unsafe {
            alloc.init(test_heap_ptr() as usize, 4096);
        }

        let layout = Layout::from_size_align(64, 8).unwrap();
        let locked = Locked::new(alloc);

        let ptr = unsafe { GlobalAlloc::alloc(&locked, layout) };
        assert!(!ptr.is_null());

        unsafe {
            GlobalAlloc::dealloc(&locked, ptr, layout);
        }
    }

    #[test]
    fn multiple_allocations() {
        let mut alloc = LinkedListAllocator::new();
        unsafe {
            alloc.init(test_heap_ptr() as usize, 4096);
        }

        let layout = Layout::from_size_align(32, 8).unwrap();
        let locked = Locked::new(alloc);

        let p1 = unsafe { GlobalAlloc::alloc(&locked, layout) };
        let p2 = unsafe { GlobalAlloc::alloc(&locked, layout) };
        let p3 = unsafe { GlobalAlloc::alloc(&locked, layout) };

        assert!(!p1.is_null());
        assert!(!p2.is_null());
        assert!(!p3.is_null());

        // All pointers should be different
        assert_ne!(p1, p2);
        assert_ne!(p2, p3);
        assert_ne!(p1, p3);
    }

    #[test]
    fn oom_returns_null() {
        let mut alloc = LinkedListAllocator::new();
        // Give only 64 bytes of heap space
        unsafe {
            alloc.init(test_heap_ptr() as usize, 64);
        }

        let layout = Layout::from_size_align(128, 8).unwrap();
        let locked = Locked::new(alloc);

        let ptr = unsafe { GlobalAlloc::alloc(&locked, layout) };
        assert!(ptr.is_null());
    }

    #[test]
    fn align_up_works() {
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(1, 8), 8);
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(9, 16), 16);
    }
}
