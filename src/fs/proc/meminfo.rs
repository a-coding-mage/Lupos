//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/meminfo.c
//! `/proc/meminfo`.
//!
//! MemTotal/MemFree/MemAvailable are wired to Lupos's real page accounting
//! (`totalram_pages()` and the global buddy free count) — previously these were
//! hardcoded to 0. Remaining work vs Linux for `complete`: Buffers/Cached and
//! the vmstat-backed fields (Active/Inactive[(anon|file)], Dirty, Writeback,
//! AnonPages, Mapped, Shmem, Slab/SReclaimable/SUnreclaim, KernelStack,
//! PageTables, Committed_AS, …) need Lupos node/zone page-state counters
//! (`NR_*`), which don't exist yet (the same accounting gap stubs
//! `sys_sysinfo`). MemAvailable is a conservative free-pages estimate pending
//! reclaimable-page accounting.
//!
//! Ref: `vendor/linux/fs/proc/meminfo.c`

use alloc::format;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let huge = crate::mm::huge::hugetlb_sysfs_snapshot();
    let swap_total_kb =
        (crate::mm::swap::total_swap_pages() as usize * crate::mm::frame::PAGE_SIZE) / 1024;
    let swap_free_kb =
        (crate::mm::swap::free_swap_pages() as usize * crate::mm::frame::PAGE_SIZE) / 1024;
    let page_size = crate::mm::frame::PAGE_SIZE;
    // Real page accounting: total RAM and the global buddy free count
    // (previously MemTotal/MemFree/MemAvailable were hardcoded to 0).
    let mem_total_kb = crate::mm::mm_public::totalram_pages() as usize * page_size / 1024;
    let mem_free_kb = crate::mm::page_alloc::nr_free_buffer_pages() * page_size / 1024;
    // Lupos lacks reclaimable-page counters, so available == free (conservative).
    let mem_available_kb = mem_free_kb;
    let text = format!(
        "MemTotal:       {:>8} kB\n\
         MemFree:        {:>8} kB\n\
         MemAvailable:   {:>8} kB\n\
         Buffers:        {:>8} kB\n\
         Cached:         {:>8} kB\n\
         SwapTotal:      {:>8} kB\n\
         SwapFree:       {:>8} kB\n\
         HugePages_Total:{:>8}\n\
         HugePages_Free: {:>8}\n\
         HugePages_Rsvd: {:>8}\n\
         HugePages_Surp: {:>8}\n\
         Hugepagesize:   {:>8} kB\n\
         HardwareCorrupted: {:>8} kB\n",
        mem_total_kb,
        mem_free_kb,
        mem_available_kb,
        0,
        0,
        swap_total_kb,
        swap_free_kb,
        huge.nr_hugepages,
        huge.free_hugepages,
        huge.resv_hugepages,
        huge.surplus_hugepages,
        (crate::mm::huge::HPAGE_PMD_NR * crate::mm::frame::PAGE_SIZE) / 1024,
        crate::mm::huge::hwpoison_corrupted_kb(),
    );
    super::util::copy_into(buf, &text)
}
