//! linux-parity: complete
//! linux-source: vendor/linux/mm/mmap.c
//! test-origin: linux:vendor/linux/mm/mmap.c
/// Anonymous memory mapping — `mmap`, `munmap`, and `brk`.
///
/// Implements the core virtual-memory shaping syscalls for anonymous (non-file-backed)
/// regions.  Every validation step, error code, and flag check is taken verbatim from
/// the Linux reference implementation.
///
/// | Lupos function        | Linux equivalent                  | Source               |
/// |-----------------------|-----------------------------------|----------------------|
/// | `do_mmap`             | `do_mmap()` + `mmap_region()`     | `mm/mmap.c:335,422`  |
/// | `do_munmap`           | `do_munmap()` / `do_vmi_munmap()` | `mm/mmap.c:1061`     |
/// | `unmap_page_range`    | `unmap_page_range()`              | `mm/memory.c`        |
/// | `do_brk_flags`        | `do_brk_flags()`                  | `mm/mmap.c:116`      |
/// | `sys_brk`             | `SYSCALL_DEFINE1(brk)`            | `mm/mmap.c:195`      |
/// | `get_unmapped_area`   | `get_unmapped_area()`             | `arch/x86/mm/mmap.c` |
/// | `calc_vm_prot_bits`   | `calc_vm_prot_bits()`             | `mm/mmap.c`          |
/// | `calc_vm_flag_bits`   | `calc_vm_flag_bits()`             | `mm/mmap.c`          |
///
/// ## References
///
/// - Linux `mm/mmap.c` — primary reference
/// - Linux `tools/testing/selftests/mm/map_fixed_noreplace.c` — parity tests
/// - Linux ABI `include/uapi/asm/mman.h` (x86) — flag values
extern crate alloc;
use crate::mm::list::ListHead;
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::arch::x86::mm::paging::{
    PAGE_MASK, PAGE_SHIFT, PAGE_SIZE, flush_tlb_range, p4d_offset, pfn_to_virt, pgd_none,
    pgd_offset_pgd, pgd_t, pmd_huge, pmd_none, pmd_offset, pte_offset_kernel, pte_pfn, pte_present,
    pte_t, ptep_get, ptep_get_and_clear, pud_huge, pud_none, pud_offset,
};
use crate::include::uapi::fcntl::{O_ACCMODE, O_RDWR, O_WRONLY};
use crate::mm::address_space::{AS_SHARED_ANON, AddressSpace};
use crate::mm::buddy::{pfn_valid, with_global_buddy};
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::pgprot::vm_get_page_prot;
use crate::mm::vm_flags::{
    VM_EXEC, VM_GROWSDOWN, VM_HUGETLB, VM_LOCKED, VM_MAYEXEC, VM_MAYREAD, VM_MAYSHARE, VM_MAYWRITE,
    VM_NORESERVE, VM_READ, VM_SHARED, VM_WRITE, VmFlags,
};
use crate::mm::vma::{
    find_vma, find_vma_prev, insert_vma, remove_vma, vm_area_dup, vm_area_free, vma_file_put_raw,
    vma_merge,
};

// ---------------------------------------------------------------------------
// Linux ABI constants — include/uapi/asm-generic/mman-common.h + x86 overrides
// ---------------------------------------------------------------------------

// PROT_* — memory protection bits
pub const PROT_READ: u32 = 0x1;
pub const PROT_WRITE: u32 = 0x2;
pub const PROT_EXEC: u32 = 0x4;
pub const PROT_NONE: u32 = 0x0;
pub const PROT_GROWSDOWN: u32 = 0x0100_0000;
pub const PROT_GROWSUP: u32 = 0x0200_0000;

// MAP_* — mapping flags (x86_64 values)
pub const MAP_SHARED: u32 = 0x01;
pub const MAP_PRIVATE: u32 = 0x02;
pub const MAP_FIXED: u32 = 0x10;
pub const MAP_ANONYMOUS: u32 = 0x20;
pub const MAP_GROWSDOWN: u32 = 0x100;
pub const MAP_DENYWRITE: u32 = 0x800;
pub const MAP_LOCKED: u32 = 0x2000;
pub const MAP_NORESERVE: u32 = 0x4000;
pub const MAP_POPULATE: u32 = 0x8000;
pub const MAP_NONBLOCK: u32 = 0x10000;
pub const MAP_STACK: u32 = 0x20000;
pub const MAP_HUGETLB: u32 = 0x40000;
pub const MAP_FIXED_NOREPLACE: u32 = 0x100000;

/// Maximum number of VMAs allowed per process.
///
/// Ref: Linux `sysctl_max_map_count` — `mm/mmap.c`
pub const SYSCTL_MAX_MAP_COUNT: i32 = 65530;
pub const DEFAULT_MMAP_BASE: u64 = 0x0000_0100_0000_0000;

/// Userspace virtual address ceiling (exclusive) for x86-64.
///
/// Ref: Linux `TASK_SIZE_MAX` — `arch/x86/include/asm/processor.h`
pub const TASK_SIZE: u64 = 0x0000_7FFF_FFFF_F000;

// ---------------------------------------------------------------------------
// Flag conversion helpers
// ---------------------------------------------------------------------------

/// Convert `PROT_*` bits → `VM_READ | VM_WRITE | VM_EXEC`.
///
/// Ref: Linux `mm/mmap.c` — `calc_vm_prot_bits()`
#[inline]
pub fn calc_vm_prot_bits(prot: u32) -> VmFlags {
    let mut vm: VmFlags = 0;
    if prot & PROT_READ != 0 {
        vm |= VM_READ;
    }
    if prot & PROT_WRITE != 0 {
        vm |= VM_WRITE;
    }
    if prot & PROT_EXEC != 0 {
        vm |= VM_EXEC;
    }
    vm
}

/// Convert `MAP_*` bits → `VM_*` behaviour flags.
///
/// Ref: Linux `mm/mmap.c` — `calc_vm_flag_bits()`
#[inline]
pub fn calc_vm_flag_bits(flags: u32) -> VmFlags {
    let mut vm: VmFlags = 0;
    if flags & MAP_GROWSDOWN != 0 {
        vm |= VM_GROWSDOWN;
    }
    if flags & MAP_LOCKED != 0 {
        vm |= VM_LOCKED;
    }
    if flags & MAP_NORESERVE != 0 {
        vm |= VM_NORESERVE;
    }
    if flags & MAP_HUGETLB != 0 {
        vm |= VM_HUGETLB;
    }
    if flags & MAP_SHARED != 0 {
        vm |= VM_SHARED;
    }
    vm
}

/// Derive `VM_MAY*` from protection flags and mapping type.
///
/// Anonymous MAP_PRIVATE mappings are always allowed to become writable and
/// executable (up to arch limits); MAP_SHARED restricts `VM_MAYWRITE` to
/// regions that were already writable.
///
/// Ref: Linux `mmap_region()` flag calculation
#[inline]
fn vm_may_flags(prot: u32, flags: u32) -> VmFlags {
    let mut may: VmFlags = VM_MAYREAD | VM_MAYWRITE | VM_MAYEXEC;
    if flags & MAP_SHARED != 0 {
        may |= VM_MAYSHARE;
        if prot & PROT_WRITE == 0 {
            may &= !VM_MAYWRITE;
        }
    }
    may
}

// ---------------------------------------------------------------------------
// Address-space gap finder
// ---------------------------------------------------------------------------

/// Find the lowest free gap `[result, result+len)` in `mm`'s address space.
///
/// Scans the Maple Tree for the first range ≥ `len` bytes that does not overlap
/// any existing VMA, starting from `hint`.  Returns the start address on success
/// or `Err(-ENOMEM)` if no gap exists below `TASK_SIZE`.
///
/// Ref: Linux `arch/x86/mm/mmap.c` — `arch_get_unmapped_area()`
///
/// # Safety
/// `mm` must be exclusively accessed (mmap_lock held for write).
pub unsafe fn get_unmapped_area(
    mm: &MmStruct,
    hint: u64,
    len: u64,
    _flags: u32,
) -> Result<u64, i32> {
    const ENOMEM: i32 = -12;

    // Avoid the NULL/low identity-map region for no-hint allocations while
    // still honoring explicit low hints used by MAP_FIXED_NOREPLACE tests.
    let start = if hint == 0 {
        DEFAULT_MMAP_BASE
    } else {
        hint & PAGE_MASK
    };

    let entries = mm.mm_mt.collect_entries();

    let mut search = start;
    for &(vma_start, vma_end_inclusive, _) in &entries {
        // Maple Tree stores inclusive end (vm_end - 1).
        let vma_end = vma_end_inclusive + 1;
        if search + len <= vma_start {
            if search + len <= TASK_SIZE {
                return Ok(search);
            }
            return Err(ENOMEM);
        }
        if vma_end > search {
            search = vma_end;
        }
    }

    // Gap after the last VMA.
    if search + len <= TASK_SIZE {
        Ok(search)
    } else {
        Err(ENOMEM)
    }
}

// ---------------------------------------------------------------------------
// do_mmap
// ---------------------------------------------------------------------------

/// Core mmap — anonymous, file-backed, and hugetlb mappings.
///
/// ## Error codes (matching Linux verbatim)
/// - `-EINVAL` (-22): `len == 0`; both or neither of `MAP_SHARED`/`MAP_PRIVATE`;
///   unaligned `MAP_FIXED` address; unaligned `MAP_HUGETLB` length.
/// - `-ENOMEM` (-12): aligned length overflows; `map_count` ≥ `SYSCTL_MAX_MAP_COUNT`;
///   no free gap.
/// - `-EEXIST` (-17): `MAP_FIXED_NOREPLACE` and range is occupied.
///
/// Ref: Linux `mm/mmap.c` — `do_mmap()` line 335, `mmap_region()` line 422
///
/// # Safety
/// `mm` must be exclusively accessed (mmap_lock held for write).
pub unsafe fn do_mmap(
    mm: &mut MmStruct,
    addr: u64,
    len: u64,
    prot: u32,
    flags: u32,
    pgoff: u64,
    file: usize,
) -> Result<u64, i32> {
    const EINVAL: i32 = -22;
    const ENOMEM: i32 = -12;
    const EEXIST: i32 = -17;

    // 1. Length must be nonzero.
    if len == 0 {
        return Err(EINVAL);
    }

    // 2. Exactly one of MAP_SHARED / MAP_PRIVATE.
    let shared = (flags & MAP_SHARED) != 0;
    let private = (flags & MAP_PRIVATE) != 0;
    if shared == private {
        return Err(EINVAL);
    }

    // 3. Page-align length; check for overflow.
    let len = len.wrapping_add(PAGE_SIZE - 1) & PAGE_MASK;
    if len == 0 {
        return Err(ENOMEM); // wrapped to zero
    }

    let mut hugetlb_private = 0usize;
    if (flags & MAP_HUGETLB) != 0 {
        let huge_len = (crate::mm::huge::HPAGE_PMD_NR as u64) * PAGE_SIZE;
        if len % huge_len != 0 {
            return Err(EINVAL);
        }
        hugetlb_private = crate::mm::huge::allocate_hugetlb_page(crate::mm::huge::HPAGE_PMD_ORDER)
            .map_err(|errno| -errno)? as usize;
    }

    // 4. Resolve the mapping address.
    let addr = if (flags & (MAP_FIXED | MAP_FIXED_NOREPLACE)) != 0 {
        if addr & !PAGE_MASK != 0 {
            if hugetlb_private != 0 {
                let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
            }
            return Err(EINVAL);
        }
        addr
    } else {
        match unsafe { get_unmapped_area(mm, addr, len, flags) } {
            Ok(addr) => addr,
            Err(err) => {
                if hugetlb_private != 0 {
                    let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
                }
                return Err(err);
            }
        }
    };

    // 5. MAP_FIXED_NOREPLACE: reject if any VMA overlaps [addr, addr+len).
    if (flags & MAP_FIXED_NOREPLACE) != 0 {
        if let Some(vma_ptr) = find_vma(mm, addr) {
            let vma = unsafe { &*vma_ptr };
            if vma.vm_start < addr + len {
                if hugetlb_private != 0 {
                    let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
                }
                return Err(EEXIST);
            }
        }
    }

    // 6. MAP_FIXED: unmap whatever is already in the range.
    if (flags & MAP_FIXED) != 0 {
        if let Err(err) = unsafe { do_munmap(mm, addr, len) } {
            if hugetlb_private != 0 {
                let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
            }
            return Err(err);
        }
    }

    // 7. Enforce the VMA count limit.
    if mm.map_count >= SYSCTL_MAX_MAP_COUNT {
        if hugetlb_private != 0 {
            let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
        }
        return Err(ENOMEM);
    }

    // 8. Build vm_flags.
    if unsafe { crate::kernel::seccomp::mdwe_refuses_exec_gain_for_mm(mm) }
        && prot & (PROT_WRITE | PROT_EXEC) == (PROT_WRITE | PROT_EXEC)
    {
        if hugetlb_private != 0 {
            let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
        }
        return Err(-13);
    }

    let mut vm_flags: VmFlags = calc_vm_prot_bits(prot)
        | calc_vm_flag_bits(flags)
        | vm_may_flags(prot, flags)
        | mm.def_flags;
    if unsafe { file_is_secretmem(file) } {
        vm_flags |= VM_LOCKED;
    }
    if unsafe { file_is_hugetlbfs(file) } {
        vm_flags |= VM_HUGETLB;
    }
    let locked_pages = if vm_flags & VM_LOCKED != 0 {
        len >> crate::arch::x86::mm::paging::PAGE_SHIFT
    } else {
        0
    };
    if locked_pages != 0
        && !crate::kernel::capability::capable(crate::kernel::capability::CAP_IPC_LOCK)
    {
        let limit =
            crate::kernel::syscalls::current_rlimit(crate::kernel::syscalls::RLIMIT_MEMLOCK)
                .rlim_cur;
        if limit != u64::MAX
            && mm
                .locked_vm
                .saturating_add(locked_pages)
                .saturating_mul(crate::arch::x86::mm::paging::PAGE_SIZE)
                > limit
        {
            if hugetlb_private != 0 {
                let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
            }
            return Err(ENOMEM);
        }
    }

    // 9. Try to merge with adjacent VMAs. Hugetlb VMAs keep per-range huge-page
    // private state, so they are inserted as distinct VMAs.
    let (_, prev) = find_vma_prev(mm, addr);
    let shared_anonymous = file == 0 && (flags & MAP_SHARED) != 0 && (flags & MAP_ANONYMOUS) != 0;
    if !shared_anonymous && (flags & MAP_HUGETLB) == 0 {
        if let Some(_merged) =
            unsafe { vma_merge(mm, prev, addr, addr + len, vm_flags, file, pgoff) }
        {
            unsafe {
                vma_file_put_raw(file);
            }
            return Ok(addr);
        }
    }

    // 10. Allocate and insert a new VMA.
    let vma_ptr = {
        let vma = Box::new(VmAreaStruct::new(addr, addr + len, vm_flags));
        let raw = Box::into_raw(vma);
        unsafe {
            // Initialize the anon_vma_chain list-head now that the VMA is
            // at a stable heap address.
            ListHead::init(&mut (*raw).anon_vma_chain);
            (*raw).vm_pgoff = pgoff;
            (*raw).vm_file = file;
            if file != 0 {
                (*raw).vm_ops = &crate::mm::fault::LUPOS_FILE_VM_OPS
                    as *const crate::mm::fault::VmOperationsStruct
                    as usize;
            }
            if hugetlb_private != 0 {
                (*raw).vm_private_data = hugetlb_private;
            }
            (*raw).vm_page_prot = vm_get_page_prot(vm_flags);
        }
        raw
    };
    if let Err(err) = unsafe { insert_vma(mm, vma_ptr) } {
        unsafe {
            (*vma_ptr).vm_file = 0;
            (*vma_ptr).vm_ops = 0;
            vm_area_free(vma_ptr);
        }
        if hugetlb_private != 0 {
            let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
        }
        return Err(err);
    }
    if locked_pages != 0 {
        mm.locked_vm = mm.locked_vm.saturating_add(locked_pages);
        if vm_flags & crate::mm::vm_flags::VM_LOCKONFAULT == 0 {
            unsafe { crate::mm::mlock::populate_locked_range(vma_ptr, addr, addr + len) };
        }
    }

    Ok(addr)
}

unsafe fn file_is_secretmem(file: usize) -> bool {
    if file == 0 {
        return false;
    }
    let file = unsafe { &*(file as *const crate::fs::types::File) };
    file.fops.name == crate::fs::syscalls::SECRETMEM_FILE_OPS.name
}

unsafe fn file_is_hugetlbfs(file: usize) -> bool {
    const HUGETLBFS_MAGIC: u64 = 0x9584_58f6;
    if file == 0 {
        return false;
    }
    let file = unsafe { &*(file as *const crate::fs::types::File) };
    let Some(inode) = file.inode() else {
        return false;
    };
    inode
        .sb
        .lock()
        .as_ref()
        .map(|sb| sb.magic == HUGETLBFS_MAGIC)
        .unwrap_or(false)
}

pub(crate) unsafe fn vma_is_secretmem(vma: *const VmAreaStruct) -> bool {
    !vma.is_null() && unsafe { file_is_secretmem((*vma).vm_file) }
}

pub(crate) unsafe fn range_contains_secretmem(mm: &MmStruct, start: u64, len: usize) -> bool {
    if len == 0 {
        return false;
    }
    let Some(end) = start.checked_add(len as u64) else {
        return true;
    };
    mm.mm_mt
        .collect_entries()
        .into_iter()
        .any(|(vstart, vend, entry)| {
            vstart < end
                && vend.saturating_add(1) > start
                && unsafe { vma_is_secretmem(entry as *const VmAreaStruct) }
        })
}

/// Linux-visible `vm_mmap()` wrapper over Lupos' Rust-native `do_mmap`.
///
/// Linux passes a `struct file *` and byte offset here; Lupos keeps the file
/// handle as an opaque kernel address until the VFS layer owns full lifetime
/// integration.
pub unsafe fn vm_mmap(
    mm: &mut MmStruct,
    file: usize,
    addr: u64,
    len: u64,
    prot: u32,
    flags: u32,
    offset: u64,
) -> Result<u64, i32> {
    if offset & (PAGE_SIZE - 1) != 0 {
        return Err(-22);
    }
    unsafe { do_mmap(mm, addr, len, prot, flags, offset >> PAGE_SHIFT, file) }
}

/// Write present `MAP_SHARED` file-backed pages in `[start, start + len)` back
/// to their underlying file. This is the minimal Linux `msync()` behaviour
/// needed by upstream MM selftests until the page cache owns shared mappings
/// directly.
///
/// # Safety
/// `mm` must remain stable for the duration of the walk.
pub(crate) unsafe fn sync_shared_file_range(
    mm: &mut MmStruct,
    start: u64,
    len: u64,
) -> Result<(), i32> {
    use crate::include::uapi::errno::{EINVAL, EIO};
    use alloc::sync::Arc;

    if len == 0 || mm.pgd == 0 {
        return Ok(());
    }
    let end = start.checked_add(len).ok_or(EINVAL)?;
    for (_, _, entry) in mm.mm_mt.collect_entries() {
        let vma = unsafe { &*(entry as *const VmAreaStruct) };
        if vma.vm_file == 0 || vma.vm_flags & (VM_SHARED | VM_WRITE) != (VM_SHARED | VM_WRITE) {
            continue;
        }
        let isect_start = start.max(vma.vm_start) & PAGE_MASK;
        let isect_end = end.min(vma.vm_end);
        if isect_start >= isect_end {
            continue;
        }

        let file_ptr = vma.vm_file as *const crate::fs::types::File;
        unsafe {
            Arc::increment_strong_count(file_ptr);
        }
        let file = unsafe { Arc::from_raw(file_ptr) };
        let file_flags = file.flags.load(core::sync::atomic::Ordering::Acquire);
        let access_mode = file_flags & O_ACCMODE;
        if access_mode != O_WRONLY && access_mode != O_RDWR {
            drop(file);
            continue;
        }
        let Some(write) = file.fops.write else {
            drop(file);
            return Err(EINVAL);
        };

        let mut addr = isect_start;
        while addr < isect_end {
            let Some(pte) = (unsafe { present_pte_for_addr(mm, addr) }) else {
                addr += PAGE_SIZE;
                continue;
            };
            let pfn = pte_pfn(pte) as usize;
            if !pfn_valid(pfn) {
                addr += PAGE_SIZE;
                continue;
            }

            let page_offset = (addr - vma.vm_start) + (vma.vm_pgoff << PAGE_SHIFT);
            let chunk = (isect_end - addr).min(PAGE_SIZE) as usize;
            let src = unsafe { core::slice::from_raw_parts(pfn_to_virt(pfn) as *const u8, chunk) };
            let mut pos = page_offset;
            let written = write(&file, src, &mut pos)?;
            if written != chunk {
                drop(file);
                return Err(EIO);
            }
            addr += PAGE_SIZE;
        }
        drop(file);
    }
    Ok(())
}

/// Flush present `MAP_SHARED` pages for one file offset range back to that
/// file. This gives memfd/tmpfs-style reads Linux-like page-cache coherence
/// before the file cache owns the mapping pages directly.
///
/// # Safety
/// `mm` must remain stable for the duration of the walk, and `file` must be
/// the raw `Arc<File>` pointer stored in matching VMAs.
pub(crate) unsafe fn sync_shared_file_mapping(
    mm: &mut MmStruct,
    file: usize,
    offset: u64,
    len: u64,
) -> Result<(), i32> {
    use crate::include::uapi::errno::{EINVAL, EIO};
    use alloc::sync::Arc;

    if len == 0 || file == 0 || mm.pgd == 0 {
        return Ok(());
    }
    let end = offset.checked_add(len).ok_or(EINVAL)?;
    for (_, _, entry) in mm.mm_mt.collect_entries() {
        let vma = unsafe { &*(entry as *const VmAreaStruct) };
        if vma.vm_file != file || vma.vm_flags & VM_SHARED == 0 {
            continue;
        }
        let vma_file_start = vma.vm_pgoff << PAGE_SHIFT;
        let vma_file_end = vma_file_start.saturating_add(vma.vm_end - vma.vm_start);
        let file_start = offset.max(vma_file_start);
        let file_end = end.min(vma_file_end);
        if file_start >= file_end {
            continue;
        }

        let file_ptr = file as *const crate::fs::types::File;
        unsafe {
            Arc::increment_strong_count(file_ptr);
        }
        let file_ref = unsafe { Arc::from_raw(file_ptr) };
        let Some(write) = file_ref.fops.write else {
            drop(file_ref);
            return Err(EINVAL);
        };

        let mut addr = vma.vm_start + (file_start - vma_file_start);
        let addr_end = vma.vm_start + (file_end - vma_file_start);
        while addr < addr_end {
            let page_addr = addr & PAGE_MASK;
            let Some(pte) = (unsafe { present_pte_for_addr(mm, page_addr) }) else {
                addr = (page_addr + PAGE_SIZE).min(addr_end);
                continue;
            };
            let pfn = pte_pfn(pte) as usize;
            if !pfn_valid(pfn) {
                addr = (page_addr + PAGE_SIZE).min(addr_end);
                continue;
            }

            let in_page = (addr - page_addr) as usize;
            let chunk = (addr_end - addr).min(PAGE_SIZE - in_page as u64) as usize;
            let page_offset = (addr - vma.vm_start) + vma_file_start;
            let src = unsafe {
                core::slice::from_raw_parts((pfn_to_virt(pfn) as *const u8).add(in_page), chunk)
            };
            let mut pos = page_offset;
            let written = write(&file_ref, src, &mut pos)?;
            if written != chunk {
                drop(file_ref);
                return Err(EIO);
            }
            addr += chunk as u64;
        }
        drop(file_ref);
    }
    Ok(())
}

unsafe fn present_pte_for_addr(mm: &MmStruct, addr: u64) -> Option<pte_t> {
    let pgd_base = mm.pgd as *mut pgd_t;
    if pgd_base.is_null() {
        return None;
    }
    let pgdp = unsafe { pgd_offset_pgd(pgd_base, addr) };
    if unsafe { pgd_none(*pgdp) } {
        return None;
    }
    let p4dp = unsafe { p4d_offset(pgdp, addr) };
    let pudp = unsafe { pud_offset(p4dp, addr) };
    if unsafe { pud_none(*pudp) || pud_huge(*pudp) } {
        return None;
    }
    let pmdp = unsafe { pmd_offset(pudp, addr) };
    if unsafe { pmd_none(*pmdp) || pmd_huge(*pmdp) } {
        return None;
    }
    let ptep = unsafe { pte_offset_kernel(pmdp, addr) };
    let pte = unsafe { ptep_get(ptep) };
    pte_present(pte).then_some(pte)
}

// ---------------------------------------------------------------------------
// unmap_page_range
// ---------------------------------------------------------------------------

/// Walk page tables for `[start, end)` and free all present PTEs.
///
/// For anonymous pages the backing `Page` refcount is decremented; if it hits
/// zero the page is returned to the buddy allocator.  File/shared pages are
/// simply cleared (full reclaim deferred to M14/M15).
///
/// Ref: Linux `mm/memory.c` — `unmap_page_range()`
///
/// # Safety
/// `mm` must be exclusively accessible.  `start` and `end` must be page-aligned.
pub unsafe fn unmap_page_range(mm: &mut MmStruct, start: u64, end: u64) {
    unsafe {
        unmap_page_range_inner(mm, start, end, false);
    }
}

unsafe fn unmap_page_range_inner(
    mm: &mut MmStruct,
    start: u64,
    end: u64,
    release_shared_anon: bool,
) {
    let pgd_base = mm.pgd as *mut pgd_t;
    if pgd_base.is_null() {
        return;
    }

    let mut addr = start;
    while addr < end {
        // PGD level
        let pgdp = unsafe { pgd_offset_pgd(pgd_base, addr) };
        if unsafe { pgd_none(*pgdp) } {
            // Advance to next PGD slot (512 GiB).
            let next = ((addr >> 39) + 1) << 39;
            addr = next;
            continue;
        }
        // PUD level
        let p4dp = unsafe { p4d_offset(pgdp, addr) };
        let pudp = unsafe { pud_offset(p4dp, addr) };
        if unsafe { pud_none(*pudp) } {
            let next = ((addr >> 30) + 1) << 30;
            addr = next;
            continue;
        }
        if unsafe { pud_huge(*pudp) } {
            let next = ((addr >> 30) + 1) << 30;
            addr = next;
            continue;
        }
        // PMD level
        let pmdp = unsafe { pmd_offset(pudp, addr) };
        if unsafe { pmd_none(*pmdp) } {
            let next = ((addr >> 21) + 1) << 21;
            addr = next;
            continue;
        }
        if unsafe { pmd_huge(*pmdp) } {
            let next = ((addr >> 21) + 1) << 21;
            addr = next;
            continue;
        }
        // PTE level
        let ptep = unsafe { pte_offset_kernel(pmdp, addr) };
        let pte: pte_t = unsafe { ptep_get_and_clear(core::ptr::null_mut(), addr, ptep) };
        if pte_present(pte) {
            unsafe {
                put_page_from_pte(pte, release_shared_anon);
            }
        }
        addr += PAGE_SIZE;
    }

    unsafe {
        flush_tlb_range(start, end);
    }
}

unsafe fn release_vma_resources(vma: *mut VmAreaStruct) {
    if unsafe { (*vma).vm_flags & VM_HUGETLB != 0 } {
        let huge_id = unsafe { (*vma).vm_private_data as u64 };
        if huge_id != 0 {
            let _ = crate::mm::huge::free_hugetlb_page(huge_id);
            unsafe {
                (*vma).vm_private_data = 0;
            }
        }
    }
}

/// Decrement the `Page` refcount encoded in `pte`; free to buddy when zero.
///
/// Ref: Linux `put_page()` — `include/linux/mm.h`
///
/// # Safety
/// PTE must encode a physical page managed by the buddy allocator.
unsafe fn put_page_from_pte(pte: pte_t, release_shared_anon: bool) {
    use crate::arch::x86::mm::paging::pte_pfn;
    use crate::mm::buddy::{pfn_to_page, pfn_valid};

    let pfn = crate::arch::x86::mm::paging::pte_pfn(pte);
    if pfn == 0 {
        return;
    }
    if !pfn_valid(pfn as usize) {
        return;
    }
    let page_ptr = pfn_to_page(pfn as usize);
    if page_ptr.is_null() {
        return;
    }
    let page = unsafe { &*page_ptr };
    page._mapcount
        .fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
    let prev = page
        ._refcount
        .fetch_sub(1, core::sync::atomic::Ordering::Release);
    if release_shared_anon && prev == 2 {
        let mapping = unsafe { (*page_ptr).mapping as *mut AddressSpace };
        if !mapping.is_null()
            && unsafe {
                (*mapping).flags.load(core::sync::atomic::Ordering::Acquire) & AS_SHARED_ANON
            } != 0
        {
            unsafe {
                (*mapping).i_pages.xa_erase((*page_ptr).index as u64);
                (*page_ptr).mapping = 0;
            }
            let remaining = page.put_page();
            if remaining == 0 {
                unsafe { crate::mm::lru::remove_lru_page(page_ptr) };
                with_global_buddy(|b| unsafe { b.free_pages(page_ptr, 0) });
            }
            return;
        }
    }
    if prev == 1 {
        unsafe { crate::mm::lru::remove_lru_page(page_ptr) };
        with_global_buddy(|b| unsafe { b.free_pages(page_ptr, 0) });
    }
}

// ---------------------------------------------------------------------------
// do_munmap
// ---------------------------------------------------------------------------

/// Unmap the virtual address range `[start, start+len)`.
///
/// Handles three overlap cases for each affected VMA:
/// - Exact match: remove + free the VMA.
/// - Left overlap: keep `[isect_end, vm_end)`.
/// - Right overlap: keep `[vm_start, isect_start)`.
/// - Middle: keep both halves (hole in the middle).
///
/// Ref: Linux `mm/mmap.c` — `do_munmap()` + `do_vmi_munmap()`
///
/// # Safety
/// `mm` must be exclusively accessed (mmap_lock held for write).
pub unsafe fn do_munmap(mm: &mut MmStruct, start: u64, len: u64) -> Result<(), i32> {
    if len == 0 {
        return Ok(());
    }

    let start = start & PAGE_MASK;
    let len = len.wrapping_add(PAGE_SIZE - 1) & PAGE_MASK;
    if len == 0 {
        return Ok(());
    }
    let end = start.checked_add(len).ok_or(-12i32)?;
    crate::mm::shmem::userfaultfd_unregister_range(start, len);

    // Snapshot overlapping VMAs before we start mutating the tree.
    let overlapping: Vec<*mut VmAreaStruct> = {
        mm.mm_mt
            .collect_entries()
            .into_iter()
            .filter_map(|(vstart, vend_incl, ptr)| {
                let vend = vend_incl + 1;
                if vstart < end && vend > start {
                    Some(ptr as *mut VmAreaStruct)
                } else {
                    None
                }
            })
            .collect()
    };

    for vma_ptr in overlapping {
        let vstart = unsafe { (*vma_ptr).vm_start };
        let vend = unsafe { (*vma_ptr).vm_end };

        let isect_start = vstart.max(start);
        let isect_end = vend.min(end);
        let locked_pages = unsafe {
            if (*vma_ptr).vm_flags & VM_LOCKED != 0 {
                (isect_end - isect_start) >> PAGE_SHIFT
            } else {
                0
            }
        };

        // Free pages in the intersection.
        let release_shared_anon = unsafe {
            (*vma_ptr).vm_file == 0
                && (*vma_ptr).vm_ops == 0
                && ((*vma_ptr).vm_flags & VM_SHARED) != 0
        };
        unsafe {
            unmap_page_range_inner(mm, isect_start, isect_end, release_shared_anon);
        }
        if locked_pages != 0 {
            mm.locked_vm = mm.locked_vm.saturating_sub(locked_pages);
        }

        if isect_start == vstart && isect_end == vend {
            // Exact match.
            unsafe {
                remove_vma(mm, vma_ptr);
                release_vma_resources(vma_ptr);
                vm_area_free(vma_ptr);
            }
        } else if isect_start == vstart {
            // Left side removed; keep [isect_end, vend).
            unsafe {
                remove_vma(mm, vma_ptr);
                (*vma_ptr).vm_start = isect_end;
                (*vma_ptr).vm_pgoff += (isect_end - vstart) >> PAGE_SHIFT;
                let _ = insert_vma(mm, vma_ptr);
            }
        } else if isect_end == vend {
            // Right side removed; keep [vstart, isect_start).
            unsafe {
                remove_vma(mm, vma_ptr);
                (*vma_ptr).vm_end = isect_start;
                let _ = insert_vma(mm, vma_ptr);
            }
        } else {
            // Middle removed; keep left [vstart, isect_start) and right [isect_end, vend).
            let right = unsafe { vm_area_dup(vma_ptr) };
            unsafe {
                remove_vma(mm, vma_ptr);
                (*vma_ptr).vm_end = isect_start;
                (*right).vm_start = isect_end;
                (*right).vm_pgoff += (isect_end - vstart) >> PAGE_SHIFT;
                let _ = insert_vma(mm, vma_ptr);
                let _ = insert_vma(mm, right);
            }
        }
    }

    Ok(())
}

/// Linux-visible `vm_munmap()` wrapper over `do_munmap()`.
pub unsafe fn vm_munmap(mm: &mut MmStruct, start: u64, len: u64) -> Result<(), i32> {
    unsafe { do_munmap(mm, start, len) }
}

// ---------------------------------------------------------------------------
// do_brk_flags
// ---------------------------------------------------------------------------

/// Expand the heap by mapping an anonymous private VMA at `[addr, addr+len)`.
///
/// Ref: Linux `mm/mmap.c` — `do_brk_flags()`
///
/// # Safety
/// `mm` must be exclusively accessible.  `addr` and `len` must be page-aligned.
pub unsafe fn do_brk_flags(mm: &mut MmStruct, addr: u64, len: u64) -> Result<(), i32> {
    if mm.map_count >= SYSCTL_MAX_MAP_COUNT {
        return Err(-12); // ENOMEM
    }
    unsafe {
        do_mmap(
            mm,
            addr,
            len,
            PROT_READ | PROT_WRITE,
            MAP_ANONYMOUS | MAP_PRIVATE | MAP_FIXED_NOREPLACE,
            0,
            0,
        )
    }?;
    Ok(())
}

// ---------------------------------------------------------------------------
// sys_brk
// ---------------------------------------------------------------------------

/// Adjust the process heap-end pointer.
///
/// - `new_brk < mm.start_brk` → return `mm.brk` (no-op).
/// - Aligned equal → update `mm.brk` and return.
/// - Grow → `do_brk_flags`; on failure return old `mm.brk`.
/// - Shrink → `do_munmap`; on failure return old `mm.brk`.
///
/// Ref: Linux `mm/mmap.c` — `SYSCALL_DEFINE1(brk)`
///
/// # Safety
/// `mm` must be exclusively accessible (mmap_lock held for write).
pub unsafe fn sys_brk(mm: &mut MmStruct, new_brk: u64) -> u64 {
    if new_brk < mm.start_brk {
        return mm.brk;
    }

    let old_aligned = (mm.brk + PAGE_SIZE - 1) & PAGE_MASK;
    let new_aligned = (new_brk + PAGE_SIZE - 1) & PAGE_MASK;

    if new_aligned == old_aligned {
        mm.brk = new_brk;
        return mm.brk;
    }

    if new_aligned > old_aligned {
        let grow = new_aligned - old_aligned;
        if unsafe { do_brk_flags(mm, old_aligned, grow) }.is_err() {
            return mm.brk;
        }
    } else {
        let shrink = old_aligned - new_aligned;
        if unsafe { do_munmap(mm, new_aligned, shrink) }.is_err() {
            return mm.brk;
        }
    }

    mm.brk = new_brk;
    mm.brk
}

// ---------------------------------------------------------------------------
// Unit tests — ported from vendor/linux/tools/testing/selftests/mm/
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::mm::mm_types::MmStruct;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;
    use crate::mm::vma::find_vma;

    fn make_mm() -> MmStruct {
        MmStruct::new(0)
    }

    #[test]
    fn mmap_applies_mlockall_future_def_flags() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        mm.def_flags = crate::mm::vm_flags::VM_LOCKED | crate::mm::vm_flags::VM_LOCKONFAULT;

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0x50000,
                PAGE_SIZE * 2,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .expect("mmap with def_flags");
        let vma = find_vma(&mm, addr).expect("future-locked vma");
        unsafe {
            assert_eq!(
                (*vma).vm_flags & crate::mm::vm_flags::VM_LOCKED,
                crate::mm::vm_flags::VM_LOCKED
            );
            assert_eq!(
                (*vma).vm_flags & crate::mm::vm_flags::VM_LOCKONFAULT,
                crate::mm::vm_flags::VM_LOCKONFAULT
            );
        }
        assert_eq!(mm.locked_vm, 2);
    }

    #[test]
    fn munmap_locked_range_releases_locked_vm_accounting() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        mm.def_flags = crate::mm::vm_flags::VM_LOCKED | crate::mm::vm_flags::VM_LOCKONFAULT;

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0x70000,
                PAGE_SIZE * 4,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .expect("locked mmap");
        assert_eq!(mm.locked_vm, 4);

        unsafe { do_munmap(&mut mm, addr + PAGE_SIZE, PAGE_SIZE * 2) }.expect("partial munmap");
        assert_eq!(mm.locked_vm, 2);

        unsafe { do_munmap(&mut mm, addr, PAGE_SIZE) }.expect("left munmap");
        unsafe { do_munmap(&mut mm, addr + PAGE_SIZE * 3, PAGE_SIZE) }.expect("right munmap");
        assert_eq!(mm.locked_vm, 0);
    }

    #[test]
    fn put_page_from_pte_ignores_non_mem_map_pfns() {
        let pte = crate::arch::x86::mm::paging::pfn_pte(
            0x44e7_44e7_44,
            crate::arch::x86::mm::paging::__pgprot(crate::arch::x86::mm::paging::_PAGE_PRESENT),
        );

        unsafe {
            put_page_from_pte(pte, false);
        }
    }

    // ── Test 1 ─────────────────────────────────────────────────────────────────
    // Port of: vendor/linux/tools/testing/selftests/mm/map_fixed_noreplace.c
    // Overlap cases must return EEXIST (-17).
    #[test]
    fn map_fixed_noreplace_rejects_overlap() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        // Place anchor VMA at [0x10000, 0x20000).
        let r = unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        };
        assert_eq!(r, Ok(0x10000));

        // Exact overlap → EEXIST.
        assert_eq!(
            unsafe {
                do_mmap(
                    &mut mm,
                    0x10000,
                    0x10000,
                    PROT_READ,
                    MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    0,
                    0,
                )
            },
            Err(-17)
        );
        // Partial overlap at start.
        assert_eq!(
            unsafe {
                do_mmap(
                    &mut mm,
                    0x0F000,
                    0x2000,
                    PROT_READ,
                    MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    0,
                    0,
                )
            },
            Err(-17)
        );
        // Partial overlap at end.
        assert_eq!(
            unsafe {
                do_mmap(
                    &mut mm,
                    0x1F000,
                    0x2000,
                    PROT_READ,
                    MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    0,
                    0,
                )
            },
            Err(-17)
        );
    }

    // ── Test 2 ─────────────────────────────────────────────────────────────────
    // Adjacent non-overlapping MAP_FIXED_NOREPLACE mappings must succeed.
    #[test]
    fn map_fixed_noreplace_allows_adjacent() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        // Immediately before.
        assert!(
            unsafe {
                do_mmap(
                    &mut mm,
                    0x8000,
                    0x8000,
                    PROT_READ,
                    MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    0,
                    0,
                )
            }
            .is_ok()
        );
        // Immediately after.
        assert!(
            unsafe {
                do_mmap(
                    &mut mm,
                    0x20000,
                    0x10000,
                    PROT_READ,
                    MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    0,
                    0,
                )
            }
            .is_ok()
        );
    }

    // ── Test 3 ─────────────────────────────────────────────────────────────────
    // A successful mmap must produce a VMA findable via find_vma.
    #[test]
    fn mmap_anonymous_creates_vma() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0,
                0x3000,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .expect("mmap should succeed");

        let vma = unsafe { &*find_vma(&mm, addr).expect("VMA must exist") };
        assert_eq!(vma.vm_start, addr);
        assert_eq!(vma.vm_end, addr + 0x3000);
        assert_eq!(mm.map_count, 1);
    }

    // ── Test 4 ─────────────────────────────────────────────────────────────────
    // map_count ≥ SYSCTL_MAX_MAP_COUNT must return ENOMEM.
    #[test]
    fn mmap_respects_max_map_count() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        mm.map_count = SYSCTL_MAX_MAP_COUNT;

        let r = unsafe {
            do_mmap(
                &mut mm,
                0x1_0000_0000,
                0x1000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
                0,
                0,
            )
        };
        assert_eq!(r, Err(-12)); // ENOMEM
    }

    // ── Test 5 ─────────────────────────────────────────────────────────────────
    // Adjacent regions with identical flags coalesce into one VMA.
    #[test]
    fn mmap_merges_adjacent_same_flags() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();
        assert_eq!(mm.map_count, 1);

        // Adjacent region with same prot/flags → should merge.
        unsafe {
            do_mmap(
                &mut mm,
                0x20000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();
        assert_eq!(mm.map_count, 1, "adjacent same-flag regions must merge");

        let vma = unsafe { &*find_vma(&mm, 0x10000).unwrap() };
        assert_eq!(vma.vm_start, 0x10000);
        assert_eq!(vma.vm_end, 0x30000);
    }

    #[test]
    fn shared_anonymous_mmap_does_not_merge_adjacent_ranges() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        let first = unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x1000,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_ANONYMOUS,
                0,
                0,
            )
        };
        assert_eq!(first, Ok(0x10000));

        let second = unsafe {
            do_mmap(
                &mut mm,
                0x11000,
                0x1000,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_ANONYMOUS,
                0,
                0,
            )
        };
        assert_eq!(second, Ok(0x11000));
        assert_eq!(
            mm.map_count, 2,
            "adjacent shared-anon mappings must stay distinct"
        );
    }

    #[test]
    fn hugetlb_mmap_allocates_pool_page_and_sets_vma_state() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::mm::huge::reset_for_tests();
        crate::mm::huge::configure_hugetlb_pool(crate::mm::huge::HPAGE_PMD_NR);
        let mut mm = make_mm();

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0,
                (crate::mm::huge::HPAGE_PMD_NR as u64) * PAGE_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB,
                0,
                0,
            )
        }
        .expect("hugetlb mmap");

        let vma = unsafe { &*find_vma(&mm, addr).expect("hugetlb VMA") };
        assert_ne!(vma.vm_private_data, 0);
        assert_ne!(vma.vm_flags & VM_HUGETLB, 0);
        assert_eq!(crate::mm::huge::huge_stats().allocated_hugetlb, 1);
    }

    #[test]
    fn munmap_partial_start_advances_pgoff_for_anonymous_vma() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x3000,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_ANONYMOUS,
                7,
                0,
            )
        }
        .expect("shared-anon mmap");
        assert_eq!(addr, 0x10000);

        unsafe { do_munmap(&mut mm, 0x10000, 0x1000) }.expect("partial munmap");

        let vma = find_vma(&mm, 0x11000).expect("remaining vma");
        unsafe {
            assert_eq!((*vma).vm_start, 0x11000);
            assert_eq!((*vma).vm_pgoff, 8);
        }
    }

    // ── Test 6 ─────────────────────────────────────────────────────────────────
    // Exact-range munmap removes the VMA completely.
    #[test]
    fn munmap_full_removes_vma() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        unsafe { do_munmap(&mut mm, addr, 0x10000) }.unwrap();

        assert_eq!(mm.map_count, 0);
        assert!(find_vma(&mm, addr).is_none());
    }

    #[test]
    fn munmap_full_releases_anon_vma_metadata() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0x20000,
                PAGE_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        let vma = find_vma(&mm, addr).expect("mapped vma");
        unsafe {
            crate::mm::rmap::anon_vma_prepare(vma).expect("anon_vma_prepare");
            assert!(!(*vma).anon_vma.is_null());

            do_munmap(&mut mm, addr, PAGE_SIZE).unwrap();
        }

        assert_eq!(mm.map_count, 0);
        assert!(find_vma(&mm, addr).is_none());
    }

    // ── Test 7 ─────────────────────────────────────────────────────────────────
    // Unmapping the left half leaves the right half intact.
    #[test]
    fn munmap_partial_start_splits() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x20000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        unsafe { do_munmap(&mut mm, 0x10000, 0x8000) }.unwrap();

        assert_eq!(mm.map_count, 1);
        let vma = unsafe { &*find_vma(&mm, 0x18000).unwrap() };
        assert_eq!(vma.vm_start, 0x18000);
        assert_eq!(vma.vm_end, 0x30000);
    }

    // ── Test 8 ─────────────────────────────────────────────────────────────────
    // Unmapping the right half leaves the left half intact.
    #[test]
    fn munmap_partial_end_splits() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x20000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .unwrap();

        unsafe { do_munmap(&mut mm, 0x28000, 0x8000) }.unwrap();

        assert_eq!(mm.map_count, 1);
        let vma = unsafe { &*find_vma(&mm, 0x10000).unwrap() };
        assert_eq!(vma.vm_start, 0x10000);
        assert_eq!(vma.vm_end, 0x28000);
    }

    // ── Test 9 ─────────────────────────────────────────────────────────────────
    // sys_brk grow / shrink round-trip keeps mm.brk consistent.
    #[test]
    fn brk_grow_shrink_roundtrip() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        mm.start_brk = 0x40_0000;
        mm.brk = 0x40_0000;

        let new_brk = unsafe { sys_brk(&mut mm, 0x41_0000) };
        assert_eq!(new_brk, 0x41_0000);
        assert_eq!(mm.brk, 0x41_0000);
        assert!(
            find_vma(&mm, 0x40_0000).is_some(),
            "heap VMA must exist after grow"
        );

        let shrunk = unsafe { sys_brk(&mut mm, 0x40_0000) };
        assert_eq!(shrunk, 0x40_0000);
        assert_eq!(mm.brk, 0x40_0000);
    }

    #[test]
    fn brk_growth_maps_exact_requested_heap_range() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        mm.start_brk = 0x55_5555_720000;
        mm.brk = 0x55_5555_720000;

        let new_brk = unsafe { sys_brk(&mut mm, 0x55_5555_722000) };
        assert_eq!(new_brk, 0x55_5555_722000);

        let vma = unsafe { &*find_vma(&mm, 0x55_5555_720000).unwrap() };
        assert_eq!(vma.vm_start, 0x55_5555_720000);
        assert_eq!(vma.vm_end, 0x55_5555_722000);
        assert!(vma.contains(0x55_5555_720000));
        assert!(vma.contains(0x55_5555_721fff));
    }
}
