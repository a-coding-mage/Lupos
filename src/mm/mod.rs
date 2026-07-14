//! linux-parity: complete
//! linux-source: vendor/linux/mm
/// Memory management subsystem — physical memory map, buddy allocator, and heap.
///
/// This module mirrors Linux's memory management:
///
/// | Lupos module      | Linux equivalent            | Reference                      |
/// |-------------------|-----------------------------|--------------------------------|
/// | `region.rs`       | `struct memblock`           | include/linux/memblock.h       |
/// | `frame.rs`        | `PhysFrame`, `PAGE_SIZE`    | include/linux/pfn.h            |
/// | `page_flags.rs`   | `enum pageflags`, `gfp_t`   | include/linux/page-flags.h     |
/// | `page.rs`         | `struct page`               | include/linux/mm_types.h       |
/// | `list.rs`         | `struct list_head`          | include/linux/list.h           |
/// | `zone.rs`         | `struct zone`, `free_area`  | include/linux/mmzone.h         |
/// | `buddy.rs`        | buddy allocator             | mm/page_alloc.c                |
/// | `heap.rs`         | `kmalloc` (early boot)      | mm/slub.c                      |
/// | `maple_tree.rs`   | Maple Tree                  | lib/maple_tree.c               |
/// | `mm_types.rs`     | `mm_struct`, `vm_area_struct`| include/linux/mm_types.h       |
/// | `vm_flags.rs`     | `VM_*` flags                | include/linux/mm.h             |
/// | `vma.rs`          | VMA operations              | mm/vma.c                       |
/// | `mmap.rs`         | mmap operations              | mm/mmap.c                      |
/// | `mprotect.rs`     | mprotect operations          | mm/mprotect.c                   |
/// | `mremap.rs`       | mremap operations            | mm/mremap.c                     |
/// | `madvise.rs`      | madvise operations            | mm/madvise.c                    |
///
/// Early boot still has a temporary identity map, but runtime memory
/// management uses the direct map and higher-half kernel mappings established
/// by `arch/x86/boot/header.S`.
pub mod buddy;
pub mod fault;
pub mod fork;
pub mod frame;
pub mod heap;
pub mod list;
pub mod madvise;
pub mod maple_tree;
pub mod mm_types;
pub mod mmap;
pub mod mprotect;
pub mod mremap;
pub mod msync;
pub mod page;
pub mod page_alloc;
pub mod page_flags;
pub mod pagewalk;
pub mod pgprot;
pub mod region;
pub mod rmap;
pub mod slab;
pub mod syscalls;
#[cfg(any(
    test,
    feature = "test-kunit",
    feature = "test-mm-kselftests",
    feature = "test-entry-kselftests",
    feature = "test-futex-kselftests",
    feature = "test-rcu-kselftests",
    feature = "test-fs-kselftests",
    feature = "test-ipc-kselftests",
    feature = "test-cgroup-kselftests",
    feature = "test-net-kselftests",
    feature = "test-drivers-kselftests",
    feature = "test-security-kselftests",
    feature = "test-block-kselftests",
    feature = "test-userspace-kselftests",
))]
pub mod test_lock;
pub mod vm_flags;
pub mod vma;
pub mod vmalloc;
pub mod zone;
// Milestone 15 — Page Cache
pub mod address_space;
pub mod backing_dev;
pub mod balloon;
pub mod bpf_memcontrol;
pub mod cma_debug;
pub mod cma_sysfs;
pub mod debug;
pub mod filemap;
pub mod lru;
pub mod readahead;
pub mod reclaim;
pub mod shrinker;
pub mod writeback;
pub mod xarray;
// Milestone 17 — Swap Subsystem
pub mod swap;
pub mod tests;
// Milestone 18 — OOM Killer & Pressure Stall Information
pub mod oom;
pub mod psi;
// Milestone 19 — cgroup v2 Memory Controller
pub mod bootmem_info;
pub mod damon;
pub mod debug_alloc;
pub mod debug_page_alloc;
pub mod debug_page_ref;
pub mod debug_vm_pgtable;
pub mod dmapool;
pub mod dmapool_test;
pub mod early_ioremap;
pub mod execmem;
pub mod fadvise;
pub mod fail_page_alloc;
pub mod failslab;
pub mod folio_compat;
pub mod gup;
pub mod gup_test;
pub mod highmem;
pub mod huge;
pub mod init_mm;
pub mod interval_tree;
pub mod ioremap;
pub mod kasan_report_hw_tags;
pub mod kasan_report_sw_tags;
pub mod kasan_report_tags;
pub mod ksm;
#[cfg(any(
    test,
    feature = "test-kunit",
    feature = "test-mm-kselftests",
    feature = "test-entry-kselftests",
    feature = "test-futex-kselftests",
    feature = "test-rcu-kselftests",
    feature = "test-fs-kselftests",
    feature = "test-ipc-kselftests",
    feature = "test-cgroup-kselftests",
    feature = "test-net-kselftests",
    feature = "test-drivers-kselftests",
    feature = "test-security-kselftests",
    feature = "test-block-kselftests",
    feature = "test-userspace-kselftests",
))]
pub mod kunit;
pub mod list_lru;
pub mod memcg;
pub mod mempolicy;
pub mod mempool;
pub mod memremap;
pub mod migration;
pub mod mlock;
pub mod mm_init;
pub mod mm_inline;
pub mod mm_public;
pub mod mmap_lock;
pub mod mmu;
pub mod mmu_notifier;
pub mod mmzone;
pub mod numa;
pub mod page_accounting;
pub mod page_poison;
pub mod percpu;
pub mod process_vm_access;
pub mod rodata_test;
pub mod sanitizers;
pub mod shmem;
pub mod usercopy;
pub mod util;
pub mod vmstat;
pub mod workingset;
pub mod zswap;
