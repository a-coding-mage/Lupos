//! linux-parity: partial
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
    PAGE_MASK, PAGE_SHIFT, PAGE_SIZE, PMD_SIZE, PUD_SIZE, p4d_offset, pfn_to_virt, pgd_none,
    pgd_offset_pgd, pgd_t, pmd_huge, pmd_none, pmd_offset, pte_offset_kernel, pte_pfn, pte_present,
    pte_special, pte_t, ptep_get, ptep_get_and_clear, pud_huge, pud_none, pud_offset,
};
use crate::arch::x86::mm::tlb::flush_tlb_mm_range;
use crate::include::uapi::errno::EINVAL;
use crate::include::uapi::fcntl::{O_ACCMODE, O_RDWR, O_WRONLY};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::address_space::{AS_SHARED_ANON, AddressSpace};
#[cfg(not(test))]
use crate::mm::buddy::page_to_pfn;
use crate::mm::buddy::{pfn_valid, with_global_buddy};
use crate::mm::maple_tree::MapleRangeEdit;
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::mmap_lock::MmapWriteGuard;
use crate::mm::pgprot::vm_get_page_prot;
use crate::mm::vm_flags::{
    VM_EXEC, VM_GROWSDOWN, VM_HUGETLB, VM_LOCKED, VM_MAYEXEC, VM_MAYREAD, VM_MAYSHARE, VM_MAYWRITE,
    VM_NORESERVE, VM_READ, VM_SHARED, VM_WRITE, VmFlags,
};
use crate::mm::vma::{
    find_vma, find_vma_prev, insert_vma, vm_area_free, vm_area_try_dup, vma_file_put_raw, vma_merge,
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("vm_mmap", linux_vm_mmap as usize, false);
}

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
            (*raw).vm_mm = mm as *mut MmStruct;
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
    if let Err(err) = unsafe { call_file_mmap(file, vma_ptr) } {
        unsafe {
            // The syscall wrapper still owns the raw Arc on failure.
            (*vma_ptr).vm_file = 0;
            (*vma_ptr).vm_ops = 0;
            vm_area_free(vma_ptr);
        }
        if hugetlb_private != 0 {
            let _ = crate::mm::huge::free_hugetlb_page(hugetlb_private as u64);
        }
        return Err(err);
    }
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

/// Invoke `file_operations::mmap` once on the initialized VMA, matching
/// Linux `__mmap_new_file_vma()` in `mm/vma.c`.
unsafe fn call_file_mmap(file: usize, vma: *mut VmAreaStruct) -> Result<(), i32> {
    use alloc::sync::Arc;

    if file == 0 {
        return Ok(());
    }
    let file_ptr = file as *const crate::fs::types::File;
    let Some(mmap_fn) = (unsafe { (*file_ptr).fops.mmap }) else {
        return Ok(());
    };

    unsafe {
        Arc::increment_strong_count(file_ptr);
    }
    let file_ref = unsafe { Arc::from_raw(file_ptr) };
    let result = mmap_fn(&file_ref, unsafe { &mut *vma });
    drop(file_ref);
    result
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
/// integration. Like Linux `vm_mmap_pgoff()`, this wrapper owns the mmap write
/// lock; lock-holding internal paths call `do_mmap()` directly.
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
    let mm_ptr = mm as *mut MmStruct;
    let _mmap_guard = unsafe { MmapWriteGuard::lock(mm_ptr) };
    unsafe { do_mmap(mm, addr, len, prot, flags, offset >> PAGE_SHIFT, file) }
}

/// `vm_mmap()` — `vendor/linux/mm/util.c:608`.
///
/// Linux maps into `current->mm` and accepts a byte offset, validating overflow
/// and page alignment before converting to `pgoff`.
pub unsafe extern "C" fn linux_vm_mmap(
    file: *mut core::ffi::c_void,
    addr: u64,
    len: u64,
    prot: u64,
    flag: u64,
    offset: u64,
) -> u64 {
    let Some(page_aligned_len) = len
        .checked_add(PAGE_SIZE - 1)
        .map(|value| value & PAGE_MASK)
    else {
        return (-(EINVAL as i64)) as u64;
    };
    if offset.checked_add(page_aligned_len).is_none() || offset & (PAGE_SIZE - 1) != 0 {
        return (-(EINVAL as i64)) as u64;
    }

    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return (-(EINVAL as i64)) as u64;
    }

    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return (-(EINVAL as i64)) as u64;
    }

    match unsafe {
        vm_mmap(
            &mut *mm,
            file as usize,
            addr,
            len,
            prot as u32,
            flag as u32,
            offset,
        )
    } {
        Ok(mapped) => mapped,
        Err(errno) => errno as u64,
    }
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

/// Number of cleared PTEs retained on the kernel stack before a shootdown.
///
/// Linux starts with an eight-pointer fallback and then links GFP_NOWAIT pages
/// until roughly 10K folios are pending. Lupos follows the same inline bundle,
/// linked-page, and allocation-failure fallback policy.
///
/// Ref: Linux `MMU_GATHER_BUNDLE` and `mmu_gather_batch` —
/// `include/asm-generic/tlb.h`; `tlb_next_batch()` — `mm/mmu_gather.c`.
const UNMAP_PTE_BATCH_SIZE: usize = 8;
const UNMAP_SHARED_ANON_WORDS: usize = UNMAP_PTE_BATCH_SIZE.div_ceil(64);
const UNMAP_GATHER_PAGE_CAPACITY: usize = 496;
const UNMAP_GATHER_PAGE_WORDS: usize = UNMAP_GATHER_PAGE_CAPACITY.div_ceil(64);
const UNMAP_GATHER_MAX_PAGES: usize = 10_000 / 510;

/// One GFP_NOWAIT-backed continuation page in the unmap gather.
///
/// Keep the complete object below 4 KiB so it can live directly in one buddy
/// page, like Linux `struct mmu_gather_batch`.
#[repr(C)]
struct UnmapPteBatchPage {
    next: *mut UnmapPteBatchPage,
    backing_page: *mut crate::mm::page::Page,
    len: usize,
    ptes: [core::mem::MaybeUninit<pte_t>; UNMAP_GATHER_PAGE_CAPACITY],
    shared_anon: [u64; UNMAP_GATHER_PAGE_WORDS],
}

const _: () = assert!(core::mem::size_of::<UnmapPteBatchPage>() <= PAGE_SIZE as usize);

/// Linux-style `mmu_gather` subset used by the PTE zap path.
///
/// The batch retains the cleared PTE values until the target address space has
/// been invalidated.  Its range deliberately spans from the first through the
/// last collected present PTE, including any holes between them, just as
/// Linux's `__tlb_adjust_range()` widens `mmu_gather::{start,end}`.
struct UnmapPteBatch {
    // Like Linux's C array, only the `[..len]` prefix is initialized. Avoid
    // zeroing even the small fallback payload for an empty munmap.
    ptes: [core::mem::MaybeUninit<pte_t>; UNMAP_PTE_BATCH_SIZE],
    // Preserve the VMA-specific release mode while entries from several VMAs
    // share one gather. One bit corresponds to each slot in `ptes`.
    shared_anon: [u64; UNMAP_SHARED_ANON_WORDS],
    len: usize,
    head: *mut UnmapPteBatchPage,
    tail: *mut UnmapPteBatchPage,
    page_count: usize,
    needs_flush: bool,
    start: u64,
    end: u64,
}

impl UnmapPteBatch {
    const fn new() -> Self {
        Self {
            ptes: [core::mem::MaybeUninit::uninit(); UNMAP_PTE_BATCH_SIZE],
            shared_anon: [0; UNMAP_SHARED_ANON_WORDS],
            len: 0,
            head: core::ptr::null_mut(),
            tail: core::ptr::null_mut(),
            page_count: 0,
            needs_flush: false,
            start: 0,
            end: 0,
        }
    }

    /// Extend the invalidation range for a cleared leaf which owns no queued
    /// order-0 page release (for example a PMD/PUD huge entry).
    #[inline]
    fn note_cleared(&mut self, start: u64, end: u64) {
        debug_assert!(start < end);
        if self.needs_flush {
            self.start = self.start.min(start);
            self.end = self.end.max(end);
        } else {
            self.start = start;
            self.end = end;
            self.needs_flush = true;
        }
    }

    #[cfg(not(test))]
    fn try_add_page(&mut self) -> bool {
        use crate::mm::page_flags::GFP_NOWAIT;

        if self.page_count >= UNMAP_GATHER_MAX_PAGES {
            return false;
        }
        let Some(backing_page) = with_global_buddy(|buddy| buddy.alloc_pages(0, GFP_NOWAIT)) else {
            return false;
        };
        let page_addr = pfn_to_virt(page_to_pfn(backing_page));
        unsafe {
            core::ptr::write_bytes(page_addr, 0, PAGE_SIZE as usize);
        }
        let page = page_addr.cast::<UnmapPteBatchPage>();
        unsafe {
            (*page).backing_page = backing_page;
            if self.tail.is_null() {
                self.head = page;
            } else {
                (*self.tail).next = page;
            }
        }
        self.tail = page;
        self.page_count += 1;
        true
    }

    #[cfg(test)]
    fn try_add_page(&mut self) -> bool {
        // Host page-table tests use identity "physical" addresses without a
        // direct-map allocation backing them. Exercise the guaranteed inline
        // fallback there; QEMU runtime gates cover continuation pages.
        false
    }

    /// Queue one already-cleared present PTE.
    ///
    /// Returns whether storage cannot grow beyond the just-queued entry and
    /// the caller must flush before clearing another PTE. This matches Linux's
    /// `__tlb_remove_page_size()` contract: the page which made the gather full
    /// is already owned by the gather.
    #[inline]
    fn push(&mut self, addr: u64, pte: pte_t, release_shared_anon: bool) -> bool {
        debug_assert!(pte_present(pte));
        self.note_cleared(addr, addr + PAGE_SIZE);

        if self.len < UNMAP_PTE_BATCH_SIZE {
            self.ptes[self.len].write(pte);
            let word = self.len / 64;
            let mask = 1u64 << (self.len % 64);
            if release_shared_anon {
                self.shared_anon[word] |= mask;
            } else {
                self.shared_anon[word] &= !mask;
            }
            self.len += 1;
            return self.len == UNMAP_PTE_BATCH_SIZE && !self.try_add_page();
        }

        debug_assert!(!self.tail.is_null());
        let page_became_full = {
            let tail = unsafe { &mut *self.tail };
            debug_assert!(tail.len < UNMAP_GATHER_PAGE_CAPACITY);
            tail.ptes[tail.len].write(pte);
            let word = tail.len / 64;
            let mask = 1u64 << (tail.len % 64);
            if release_shared_anon {
                tail.shared_anon[word] |= mask;
            } else {
                tail.shared_anon[word] &= !mask;
            }
            tail.len += 1;
            tail.len == UNMAP_GATHER_PAGE_CAPACITY
        };
        page_became_full && !self.try_add_page()
    }

    /// Invalidate the accumulated range before releasing any queued page.
    ///
    /// Ref: Linux generic MMU-gather ordering in
    /// `include/asm-generic/tlb.h`: unhook, invalidate, then free.
    #[inline]
    fn drain_with<F, R>(&mut self, mut flush: F, mut release: R)
    where
        F: FnMut(u64, u64),
        R: FnMut(pte_t, bool),
    {
        if !self.needs_flush {
            return;
        }

        flush(self.start, self.end);
        for (index, pte) in self.ptes[..self.len].iter().enumerate() {
            // `push()` initializes every slot below `len`, and `len` is reset
            // only after all of them have been consumed.
            let release_shared_anon = self.shared_anon[index / 64] & (1u64 << (index % 64)) != 0;
            release(unsafe { pte.assume_init_read() }, release_shared_anon);
        }

        let mut page = self.head;
        while !page.is_null() {
            let next = unsafe { (*page).next };
            let page_len = unsafe { (*page).len };
            for index in 0..page_len {
                let pte = unsafe { (*page).ptes[index].assume_init_read() };
                let release_shared_anon =
                    unsafe { (*page).shared_anon[index / 64] } & (1u64 << (index % 64)) != 0;
                release(pte, release_shared_anon);
            }
            let backing_page = unsafe { (*page).backing_page };
            if !backing_page.is_null() {
                with_global_buddy(|buddy| unsafe { buddy.free_pages(backing_page, 0) });
            }
            page = next;
        }

        self.len = 0;
        self.head = core::ptr::null_mut();
        self.tail = core::ptr::null_mut();
        self.page_count = 0;
        self.needs_flush = false;
        self.start = 0;
        self.end = 0;
    }

    /// Drain the gather, then run teardown that may invalidate VMA-owned
    /// metadata. Keeping this ordering in one helper makes it impossible for
    /// the munmap completion path to close a detached VMA while one of its
    /// queued PTEs still awaits a shootdown.
    #[inline]
    fn drain_with_then<F, R, A>(&mut self, flush: F, release: R, after_drain: A)
    where
        F: FnMut(u64, u64),
        R: FnMut(pte_t, bool),
        A: FnOnce(),
    {
        self.drain_with(flush, release);
        after_drain();
    }
}

#[inline]
unsafe fn flush_and_release_pte_batch(batch: &mut UnmapPteBatch, mm: *mut MmStruct) {
    batch.drain_with(
        |batch_start, batch_end| {
            assert!(
                unsafe { flush_tlb_mm_range(mm, batch_start, batch_end) },
                "TLB shootdown failed before releasing unmapped pages"
            );
        },
        |pte, release_shared_anon| unsafe {
            put_page_from_pte(pte, release_shared_anon);
        },
    );
}

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
    let mm_ptr = mm as *mut MmStruct;
    let mut batch = UnmapPteBatch::new();
    unsafe {
        unmap_page_range_inner(mm, start, end, false, &mut batch);
        flush_and_release_pte_batch(&mut batch, mm_ptr);
    }
}

unsafe fn unmap_page_range_inner(
    mm: &mut MmStruct,
    start: u64,
    end: u64,
    release_shared_anon: bool,
    batch: &mut UnmapPteBatch,
) {
    let pgd_base = mm.pgd as *mut pgd_t;
    if pgd_base.is_null() {
        return;
    }

    let mm_ptr = mm as *mut MmStruct;
    let mut addr = start;
    while addr < end {
        // PGD level
        let pgdp = unsafe { pgd_offset_pgd(pgd_base, addr) };
        if unsafe { pgd_none(*pgdp) } {
            // Advance to next PGD slot (512 GiB).
            let next = ((((addr >> 39) + 1) << 39).min(end)).max(addr + PAGE_SIZE);
            addr = next;
            continue;
        }
        // PUD level
        let p4dp = unsafe { p4d_offset(pgdp, addr) };
        let pudp = unsafe { pud_offset(p4dp, addr) };
        if unsafe { pud_none(*pudp) } {
            let next = ((addr & !(PUD_SIZE - 1)) + PUD_SIZE).min(end);
            addr = next;
            continue;
        }
        if unsafe { pud_huge(*pudp) } {
            // A correct user huge-leaf zap needs Linux's hugetlb/THP locks,
            // notifier interval, compound-folio release, and rmap accounting.
            // Preserve the leaf until that protocol exists instead of
            // releasing its tails as unrelated order-0 pages.
            addr = ((addr & !(PUD_SIZE - 1)) + PUD_SIZE).min(end);
            continue;
        }
        // PMD level
        let pmdp = unsafe { pmd_offset(pudp, addr) };
        if unsafe { pmd_none(*pmdp) } {
            let next = ((addr & !(PMD_SIZE - 1)) + PMD_SIZE).min(end);
            addr = next;
            continue;
        }
        if unsafe { pmd_huge(*pmdp) } {
            addr = ((addr & !(PMD_SIZE - 1)) + PMD_SIZE).min(end);
            continue;
        }
        // PTE level
        let ptep = unsafe { pte_offset_kernel(pmdp, addr) };
        let pte: pte_t = ptep_get_and_clear(mm_ptr.cast::<()>(), addr, ptep);
        if pte_present(pte) && batch.push(addr, pte, release_shared_anon) {
            unsafe {
                flush_and_release_pte_batch(batch, mm_ptr);
            }
        }
        addr += PAGE_SIZE;
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

    // `remap_pfn_range()` PTEs are raw PFNs even when the PFN happens to lie
    // in managed RAM. They do not own a `struct page` reference.
    if pte_special(pte) {
        return;
    }
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
    page._mapcount()
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
    // Linux do_vmi_munmap() rejects an unaligned start and any range outside
    // the userspace limit before consulting the VMA tree.
    if start & !PAGE_MASK != 0 || start > TASK_SIZE || len > TASK_SIZE - start {
        return Err(-EINVAL);
    }

    let len = len.wrapping_add(PAGE_SIZE - 1) & PAGE_MASK;
    if len == 0 {
        return Err(-EINVAL);
    }
    let end = start + len;
    let tree_end = end - 1;

    // Linux preallocates the VMA iterator edit and a possible right-hand VMA
    // before clearing page tables. Lupos's sorted-Vec Maple Tree needs at most
    // one extra slot: only one VMA can straddle both munmap boundaries.
    mm.mm_mt.prepare_edit_range(start, tree_end)?;

    // Linux retains removed VMAs in a detached Maple Tree until after the
    // primary tree is cleared and the page-table/TLB work completes.  Lupos
    // needs the same detached lifetime for close callbacks, but its compact
    // sorted-Vec tree cannot embed a second tree node in each VMA.  Reserve the
    // maximum possible pointer count before the point of no return so the edit
    // callback below cannot allocate.
    let mut affected_vmas = 0usize;
    let mut split_error = None;
    mm.mm_mt
        .for_each_range(start, tree_end, |vstart, vend_inclusive, entry| {
            affected_vmas += 1;
            let vend = vend_inclusive.saturating_add(1);
            let vma = unsafe { &*(entry as *const VmAreaStruct) };
            if vma.vm_flags & VM_HUGETLB != 0 {
                let huge_mask = crate::mm::huge::HPAGE_PMD_NR as u64 * PAGE_SIZE - 1;
                if (vstart < start && start < vend && start & huge_mask != 0)
                    || (vstart < end && end < vend && end & huge_mask != 0)
                {
                    split_error = Some(-EINVAL);
                }
            }
        });
    if let Some(errno) = split_error {
        return Err(errno);
    }
    if affected_vmas == 0 {
        // Linux do_vmi_munmap() returns immediately when vma_find() finds no
        // overlap; no detached-VMA storage or userfault notification is
        // needed.
        return Ok(());
    }
    // Linux's detached tree keeps its root entry inline. Keep the common
    // small munmap allocation-free as well, and reserve any overflow before
    // page-table clears.
    const INLINE_DETACHED_VMAS: usize = 8;
    let mut removed_inline: [*mut VmAreaStruct; INLINE_DETACHED_VMAS] =
        [core::ptr::null_mut(); INLINE_DETACHED_VMAS];
    let mut removed_overflow: Vec<*mut VmAreaStruct> = Vec::new();
    removed_overflow
        .try_reserve_exact(affected_vmas.saturating_sub(INLINE_DETACHED_VMAS))
        .map_err(|_| -12i32)?;
    let mut removed_count = 0usize;

    let mut prepared_right = core::ptr::null_mut();
    if let Some((vstart, vend_inclusive, entry)) = mm.mm_mt.find(start, tree_end) {
        if vstart < start && vend_inclusive >= end {
            let source = entry as *const VmAreaStruct;
            prepared_right = unsafe { vm_area_try_dup(source) }?;
            let huge_id = unsafe {
                if (*source).vm_flags & VM_HUGETLB != 0 {
                    (*source).vm_private_data as u64
                } else {
                    0
                }
            };
            if huge_id != 0
                && let Err(errno) = crate::mm::huge::retain_hugetlb_page(huge_id)
            {
                unsafe { vm_area_free(prepared_right) };
                return Err(-errno);
            }
        }
    }

    let mm_ptr = mm as *mut MmStruct;
    let tree_ptr = &mm.mm_mt as *const crate::mm::maple_tree::MapleTree;
    crate::mm::shmem::userfaultfd_unregister_range(start, len);

    // Like Linux's `mmu_gather`, retain one batch for the entire VMA walk.
    // The write lock keeps the still-published VMA descriptors inaccessible to
    // fault-side readers until every queued leaf has passed through
    // clear -> shootdown -> page release. Tree edits and VMA close callbacks
    // happen only after the final drain.
    let mut batch = UnmapPteBatch::new();
    unsafe {
        (*tree_ptr).for_each_range(start, tree_end, |vstart, vend_inclusive, entry| {
            let vma_ptr = entry as *mut VmAreaStruct;
            let vend = vend_inclusive.saturating_add(1);
            let isect_start = vstart.max(start);
            let isect_end = vend.min(end);
            let release_shared_anon = (*vma_ptr).vm_file == 0
                && (*vma_ptr).vm_ops == 0
                && ((*vma_ptr).vm_flags & VM_SHARED) != 0;
            unmap_page_range_inner(
                &mut *mm_ptr,
                isect_start,
                isect_end,
                release_shared_anon,
                &mut batch,
            );
            if (*vma_ptr).vm_flags & VM_LOCKED != 0 {
                let locked_pages = (isect_end - isect_start) >> PAGE_SHIFT;
                (*mm_ptr).locked_vm = (*mm_ptr).locked_vm.saturating_sub(locked_pages);
            }
        });
    }
    unsafe {
        flush_and_release_pte_batch(&mut batch, mm_ptr);
    }

    // With stale translations gone, compact the affected VMA interval in one
    // pass. Completely removed VMAs are retained in the inline/overflow
    // detached buffers: invoking vm_ops.close while edit_range holds the
    // backing Vec's exclusive borrow would leave the VMA published during
    // close and permit callback reentry to invalidate that borrow.
    let right_slot = &mut prepared_right as *mut *mut VmAreaStruct;
    let edited = unsafe {
        (*tree_ptr).edit_range(start, tree_end, |vstart, vend_inclusive, entry| {
            let vma_ptr = entry as *mut VmAreaStruct;
            let vend = vend_inclusive.saturating_add(1);
            let isect_start = vstart.max(start);
            let isect_end = vend.min(end);
            let removed_pages = (isect_end - isect_start) >> PAGE_SHIFT;
            (*mm_ptr).total_vm = (*mm_ptr).total_vm.saturating_sub(removed_pages);

            if isect_start == vstart && isect_end == vend {
                (*mm_ptr).map_count = (*mm_ptr).map_count.saturating_sub(1);
                if removed_count < INLINE_DETACHED_VMAS {
                    removed_inline[removed_count] = vma_ptr;
                } else {
                    removed_overflow.push(vma_ptr);
                }
                removed_count += 1;
                MapleRangeEdit::Remove
            } else if isect_start == vstart {
                (*vma_ptr).vm_start = isect_end;
                (*vma_ptr).vm_pgoff += (isect_end - vstart) >> PAGE_SHIFT;
                MapleRangeEdit::Keep {
                    start: isect_end,
                    end: vend - 1,
                    value: entry,
                }
            } else if isect_end == vend {
                (*vma_ptr).vm_end = isect_start;
                MapleRangeEdit::Keep {
                    start: vstart,
                    end: isect_start - 1,
                    value: entry,
                }
            } else {
                let right = *right_slot;
                assert!(
                    !right.is_null(),
                    "munmap middle split missing preallocated VMA"
                );
                *right_slot = core::ptr::null_mut();
                (*vma_ptr).vm_end = isect_start;
                (*right).vm_start = isect_end;
                (*right).vm_pgoff += (isect_end - vstart) >> PAGE_SHIFT;
                (*mm_ptr).map_count = (*mm_ptr).map_count.saturating_add(1);
                MapleRangeEdit::Split {
                    left_start: vstart,
                    left_end: isect_start - 1,
                    left_value: entry,
                    right_start: isect_end,
                    right_end: vend - 1,
                    right_value: right as usize,
                }
            }
        })
    }?;
    debug_assert!(edited > 0 || prepared_right.is_null());
    debug_assert!(prepared_right.is_null());

    // Linux's vms_complete_munmap_vmas() likewise invokes remove_vma() only
    // after the VMAs have been detached from the primary Maple Tree.
    for &vma_ptr in &removed_inline[..removed_count.min(INLINE_DETACHED_VMAS)] {
        unsafe {
            release_vma_resources(vma_ptr);
            vm_area_free(vma_ptr);
        }
    }
    for vma_ptr in removed_overflow {
        unsafe {
            release_vma_resources(vma_ptr);
            vm_area_free(vma_ptr);
        }
    }

    Ok(())
}

/// Linux-visible `vm_munmap()` wrapper over `do_munmap()`.
///
/// This is the lock-owning entry point. Internal MAP_FIXED, brk, and mremap
/// paths call `do_munmap()` while their outer syscall guard remains held.
pub unsafe fn vm_munmap(mm: &mut MmStruct, start: u64, len: u64) -> Result<(), i32> {
    let mm_ptr = mm as *mut MmStruct;
    let _mmap_guard = unsafe { MmapWriteGuard::lock(mm_ptr) };
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
    use core::sync::atomic::{AtomicBool, Ordering};

    fn make_mm() -> MmStruct {
        MmStruct::new(0)
    }

    static CLOSE_SAW_DETACHED_VMA: AtomicBool = AtomicBool::new(false);
    static CLOSE_REENTERED_MAPLE_TREE: AtomicBool = AtomicBool::new(false);
    static CLOSE_ORDER: std::sync::Mutex<std::vec::Vec<usize>> =
        std::sync::Mutex::new(std::vec::Vec::new());

    unsafe extern "C" fn check_detached_vma_on_close(vma: *mut VmAreaStruct) {
        let mm = unsafe { &mut *(*vma).vm_mm };
        let start = unsafe { (*vma).vm_start };
        let detached = mm.mm_mt.load(start) != Some(vma as usize);
        CLOSE_SAW_DETACHED_VMA.store(detached, Ordering::SeqCst);

        // Only reenter after confirming detachment, so the regression fails
        // cleanly on the old ordering instead of deliberately aliasing the
        // edit_range Vec borrow.
        if detached {
            let inserted = mm.mm_mt.insert_range(start, start, vma as usize).is_ok();
            let erased = mm.mm_mt.erase(start) == Some(vma as usize);
            CLOSE_REENTERED_MAPLE_TREE.store(inserted && erased, Ordering::SeqCst);
        }
    }

    static DETACH_CHECK_VM_OPS: crate::mm::fault::VmOperationsStruct =
        crate::mm::fault::VmOperationsStruct {
            open: None,
            close: Some(check_detached_vma_on_close),
            fault: None,
            map_pages: None,
            pfn_mkwrite: None,
            access: None,
        };

    unsafe extern "C" fn record_close_order(vma: *mut VmAreaStruct) {
        let tag = unsafe { (*vma).vm_private_data };
        CLOSE_ORDER
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(tag);
    }

    static CLOSE_ORDER_VM_OPS: crate::mm::fault::VmOperationsStruct =
        crate::mm::fault::VmOperationsStruct {
            open: None,
            close: Some(record_close_order),
            fault: None,
            map_pages: None,
            pfn_mkwrite: None,
            access: None,
        };

    /// Linux do_vmi_munmap() returns immediately when vma_find() finds no
    /// overlap. The no-op must not instantiate Lupos's lazy tree backing.
    ///
    /// test-origin: linux:vendor/linux/mm/vma.c:do_vmi_munmap
    #[test]
    fn munmap_empty_mm_does_not_allocate_maple_storage() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        unsafe { do_munmap(&mut mm, 0x10000, PAGE_SIZE) }.expect("empty munmap");

        assert_eq!(
            mm.mm_mt.ma_root.load(core::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    /// test-origin: linux:vendor/linux/mm/vma.c:do_vmi_munmap
    #[test]
    fn munmap_rejects_zero_unaligned_and_out_of_range_requests() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();

        assert_eq!(unsafe { do_munmap(&mut mm, 0x1000, 0) }, Err(-EINVAL));
        assert_eq!(
            unsafe { do_munmap(&mut mm, 0x1001, PAGE_SIZE) },
            Err(-EINVAL)
        );
        assert_eq!(
            unsafe { do_munmap(&mut mm, TASK_SIZE, PAGE_SIZE) },
            Err(-EINVAL)
        );
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

    /// Linux's generic MMU-gather contract requires unhook -> invalidate ->
    /// free. This host test exercises Lupos's representation of that contract
    /// because Linux's pointer-based gather cannot run directly in the Rust
    /// host harness.
    ///
    /// test-origin: linux:vendor/linux/include/asm-generic/tlb.h
    #[test]
    fn unmap_batch_aggregates_vmas_and_preserves_release_metadata() {
        use core::cell::{Cell, RefCell};

        let first = pte_t(crate::arch::x86::mm::paging::_PAGE_PRESENT | 0x1000);
        let second = pte_t(crate::arch::x86::mm::paging::_PAGE_PRESENT | 0x2000);
        let mut batch = UnmapPteBatch::new();
        // Model two VMAs separated by a hole. The second VMA is shared
        // anonymous, so its page needs the extra address-space release.
        assert!(!batch.push(0x4000, first, false));
        assert!(!batch.push(0xa000, second, true));

        let flushed = Cell::new(false);
        let flushed_range = Cell::new((0, 0));
        let released = RefCell::new(std::vec::Vec::new());
        batch.drain_with(
            |start, end| {
                assert!(released.borrow().is_empty());
                flushed_range.set((start, end));
                flushed.set(true);
            },
            |pte, release_shared_anon| {
                assert!(flushed.get(), "page released before TLB invalidation");
                released.borrow_mut().push((pte, release_shared_anon));
            },
        );

        assert_eq!(flushed_range.get(), (0x4000, 0xb000));
        assert_eq!(
            released.borrow().as_slice(),
            &[(first, false), (second, true)]
        );

        // A second drain must not emit a spurious flush for an empty batch.
        flushed.set(false);
        batch.drain_with(
            |_, _| flushed.set(true),
            |_, _| panic!("empty batch released a PTE"),
        );
        assert!(!flushed.get());
    }

    /// test-origin: linux:vendor/linux/mm/vma.c:vms_complete_munmap_vmas
    #[test]
    fn unmap_batch_drains_before_detached_vma_resources() {
        use core::cell::RefCell;

        let pte = pte_t(crate::arch::x86::mm::paging::_PAGE_PRESENT | 0x3000);
        let mut batch = UnmapPteBatch::new();
        assert!(!batch.push(0x8000, pte, false));
        let events = RefCell::new(std::vec::Vec::new());

        batch.drain_with_then(
            |_, _| events.borrow_mut().push("flush"),
            |_, _| events.borrow_mut().push("page-release"),
            || events.borrow_mut().push("vma-resource-release"),
        );

        assert_eq!(
            events.borrow().as_slice(),
            &["flush", "page-release", "vma-resource-release"]
        );
    }

    /// If a GFP_NOWAIT continuation page is unavailable, Linux falls back to
    /// its eight-pointer on-stack bundle and asks the caller to drain at that
    /// exact boundary. Host tests deliberately exercise that fallback.
    ///
    /// test-origin: linux:vendor/linux/mm/mmu_gather.c:tlb_next_batch
    #[test]
    fn unmap_batch_reports_full_at_inline_fallback_capacity() {
        use core::cell::Cell;

        assert_eq!(UNMAP_PTE_BATCH_SIZE, 8);
        assert!(core::mem::size_of::<UnmapPteBatchPage>() <= PAGE_SIZE as usize);
        assert_eq!(UNMAP_GATHER_MAX_PAGES, 10_000 / 510);
        let mut batch = UnmapPteBatch::new();
        for index in 0..UNMAP_PTE_BATCH_SIZE {
            let addr = 0x1000 + index as u64 * PAGE_SIZE;
            let pte = pte_t(
                crate::arch::x86::mm::paging::_PAGE_PRESENT | ((index as u64 + 1) << PAGE_SHIFT),
            );
            assert_eq!(
                batch.push(addr, pte, index % 2 == 0),
                index + 1 == UNMAP_PTE_BATCH_SIZE
            );
        }

        let flush_count = Cell::new(0);
        let release_count = Cell::new(0);
        let shared_release_count = Cell::new(0);
        batch.drain_with(
            |start, end| {
                assert_eq!(start, 0x1000);
                assert_eq!(end, 0x1000 + UNMAP_PTE_BATCH_SIZE as u64 * PAGE_SIZE);
                flush_count.set(flush_count.get() + 1);
            },
            |_, release_shared_anon| {
                release_count.set(release_count.get() + 1);
                if release_shared_anon {
                    shared_release_count.set(shared_release_count.get() + 1);
                }
            },
        );
        assert_eq!(flush_count.get(), 1);
        assert_eq!(release_count.get(), UNMAP_PTE_BATCH_SIZE);
        assert_eq!(shared_release_count.get(), UNMAP_PTE_BATCH_SIZE / 2);
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

    #[test]
    fn map_fixed_replacement_uses_one_outer_mmap_write_lock() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        let mm_ptr = &mut mm as *mut MmStruct;
        let _mmap_guard = unsafe { MmapWriteGuard::lock(mm_ptr) };

        unsafe {
            do_mmap(
                &mut mm,
                0x40000,
                PAGE_SIZE,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .expect("initial mapping");
        let replaced = unsafe {
            do_mmap(
                &mut mm,
                0x40000,
                PAGE_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
                0,
                0,
            )
        }
        .expect("MAP_FIXED replacement under syscall-level write lock");

        assert_eq!(replaced, 0x40000);
        assert_eq!(mm.map_count, 1);
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

    /// Linux takes a reservation-map reference for the new VMA when munmap
    /// splits a private hugetlb VMA. Closing either surviving fragment must
    /// not return the reservation to the pool while the other remains.
    ///
    /// test-origin: linux:vendor/linux/mm/vma.c:vms_gather_munmap_vmas
    /// test-origin: linux:vendor/linux/mm/hugetlb.c:hugetlb_vm_op_open
    #[test]
    fn hugetlb_middle_munmap_retains_each_surviving_vma_fragment() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::mm::huge::reset_for_tests();
        crate::mm::huge::configure_hugetlb_pool(crate::mm::huge::HPAGE_PMD_NR);
        let mut mm = make_mm();
        let huge_size = crate::mm::huge::HPAGE_PMD_NR as u64 * PAGE_SIZE;
        let base = 0x4000_0000;

        unsafe {
            do_mmap(
                &mut mm,
                base,
                3 * huge_size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB | MAP_FIXED_NOREPLACE,
                0,
                0,
            )
        }
        .expect("three-page hugetlb VMA");
        let original = find_vma(&mm, base).expect("hugetlb VMA");
        let huge_id = unsafe { (*original).vm_private_data as u64 };

        unsafe { do_munmap(&mut mm, base + huge_size, huge_size) }.expect("middle hugetlb munmap");
        assert_eq!(mm.map_count, 2);
        assert_eq!(crate::mm::huge::huge_page(huge_id).unwrap().refcount, 2);
        assert_eq!(crate::mm::huge::huge_stats().pool_pages, 0);

        unsafe { do_munmap(&mut mm, base, huge_size) }.expect("left fragment munmap");
        assert_eq!(crate::mm::huge::huge_page(huge_id).unwrap().refcount, 1);
        assert_eq!(crate::mm::huge::huge_stats().pool_pages, 0);

        unsafe { do_munmap(&mut mm, base + 2 * huge_size, huge_size) }
            .expect("right fragment munmap");
        assert_eq!(crate::mm::huge::huge_page(huge_id), None);
        assert_eq!(
            crate::mm::huge::huge_stats().pool_pages,
            crate::mm::huge::HPAGE_PMD_NR
        );
    }

    /// Linux's hugetlb VMA split callback rejects boundaries which are not
    /// aligned to the mapping's huge-page size.
    ///
    /// test-origin: linux:vendor/linux/mm/hugetlb.c:hugetlb_vm_op_split
    #[test]
    fn hugetlb_munmap_rejects_unaligned_vma_split() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::mm::huge::reset_for_tests();
        crate::mm::huge::configure_hugetlb_pool(crate::mm::huge::HPAGE_PMD_NR);
        let mut mm = make_mm();
        let huge_size = crate::mm::huge::HPAGE_PMD_NR as u64 * PAGE_SIZE;
        let base = 0x6000_0000;

        unsafe {
            do_mmap(
                &mut mm,
                base,
                huge_size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB | MAP_FIXED_NOREPLACE,
                0,
                0,
            )
        }
        .expect("hugetlb VMA");
        let vma = find_vma(&mm, base).expect("hugetlb VMA");
        let huge_id = unsafe { (*vma).vm_private_data as u64 };

        assert_eq!(
            unsafe { do_munmap(&mut mm, base + PAGE_SIZE, PAGE_SIZE) },
            Err(-EINVAL)
        );
        assert_eq!(mm.map_count, 1);
        assert_eq!(crate::mm::huge::huge_page(huge_id).unwrap().refcount, 1);

        unsafe { do_munmap(&mut mm, base, huge_size) }.expect("hugetlb cleanup");
        assert_eq!(crate::mm::huge::huge_page(huge_id), None);
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

    /// Linux clears the primary VMA Maple Tree before remove_vma() invokes a
    /// driver's vm_ops.close callback. The callback may therefore inspect or
    /// mutate that tree without observing the closing VMA or aliasing an
    /// in-progress tree edit.
    ///
    /// test-origin: linux:vendor/linux/mm/vma.c:vms_complete_munmap_vmas
    #[test]
    fn munmap_detaches_vma_before_close_callback_reenters_tree() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        CLOSE_SAW_DETACHED_VMA.store(false, Ordering::SeqCst);
        CLOSE_REENTERED_MAPLE_TREE.store(false, Ordering::SeqCst);

        let addr = unsafe {
            do_mmap(
                &mut mm,
                0x180000,
                PAGE_SIZE,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        }
        .expect("mmap for close callback");
        let vma = find_vma(&mm, addr).expect("mapped VMA");
        unsafe {
            (*vma).vm_ops = &DETACH_CHECK_VM_OPS as *const _ as usize;
            do_munmap(&mut mm, addr, PAGE_SIZE).expect("munmap with close callback");
        }

        assert!(CLOSE_SAW_DETACHED_VMA.load(Ordering::SeqCst));
        assert!(CLOSE_REENTERED_MAPLE_TREE.load(Ordering::SeqCst));
        assert_eq!(mm.map_count, 0);
        assert!(find_vma(&mm, addr).is_none());
    }

    /// Linux walks its detached Maple Tree in ascending VMA order when
    /// remove_vma() invokes close callbacks. Exercise both Lupos's eight-entry
    /// inline detached buffer and its preallocated overflow without changing
    /// that order.
    ///
    /// test-origin: linux:vendor/linux/mm/vma.c:vms_complete_munmap_vmas
    #[test]
    fn munmap_overflow_preserves_detached_close_order() {
        const VMA_COUNT: usize = 10;

        let _g = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let mut mm = make_mm();
        CLOSE_ORDER
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clear();

        let base = 0x20_0000;
        for index in 0..VMA_COUNT {
            let addr = base + index as u64 * 2 * PAGE_SIZE;
            unsafe {
                do_mmap(
                    &mut mm,
                    addr,
                    PAGE_SIZE,
                    PROT_READ,
                    MAP_SHARED | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    index as u64,
                    0,
                )
            }
            .expect("distinct VMA for detached overflow");
            let vma = find_vma(&mm, addr).expect("new detached-overflow VMA");
            unsafe {
                (*vma).vm_ops = &CLOSE_ORDER_VM_OPS as *const _ as usize;
                (*vma).vm_private_data = index + 1;
            }
        }

        unsafe { do_munmap(&mut mm, base, (VMA_COUNT as u64 * 2 - 1) * PAGE_SIZE) }
            .expect("munmap spanning inline and overflow detached VMAs");

        let expected: std::vec::Vec<usize> = (1..=VMA_COUNT).collect();
        let close_order = CLOSE_ORDER
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(close_order.as_slice(), expected.as_slice());
        assert_eq!(mm.map_count, 0);
        assert!(mm.mm_mt.is_empty());
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

    /// Linux splits a VMA at both boundaries before clearing a hole from its
    /// middle, preserving both residual ranges and their page offsets.
    ///
    /// test-origin: linux:vendor/linux/mm/vma.c:vms_gather_munmap_vmas
    #[test]
    fn munmap_middle_keeps_two_preallocated_vma_halves() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        let base = 0x50_0000;

        unsafe {
            do_mmap(
                &mut mm,
                base,
                4 * PAGE_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                9,
                0,
            )
        }
        .expect("four-page VMA");

        unsafe { do_munmap(&mut mm, base + PAGE_SIZE, 2 * PAGE_SIZE) }.expect("middle munmap");

        assert_eq!(mm.map_count, 2);
        assert_eq!(mm.total_vm, 2);
        let left = find_vma(&mm, base).expect("left VMA");
        let right = find_vma(&mm, base + PAGE_SIZE).expect("right VMA after hole");
        assert_ne!(left, right);
        unsafe {
            assert_eq!(((*left).vm_start, (*left).vm_end), (base, base + PAGE_SIZE));
            assert_eq!(
                ((*right).vm_start, (*right).vm_end),
                (base + 3 * PAGE_SIZE, base + 4 * PAGE_SIZE)
            );
            assert_eq!((*left).vm_pgoff, 9);
            assert_eq!((*right).vm_pgoff, 12);
        }
    }

    /// A single munmap over many VMAs must walk only the intersecting native
    /// tree range and bulk-remove it; this is the behavioral counterpart of
    /// Linux's `for_each_vma_range()`/`vma_iter_clear_gfp()` path.
    ///
    /// test-origin: linux:vendor/linux/mm/vma.c:do_vmi_align_munmap
    #[test]
    fn munmap_many_distinct_vmas_removes_the_native_range() {
        const VMA_COUNT: usize = 128;

        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut mm = make_mm();
        let base = 0x1_0000_0000;
        for index in 0..VMA_COUNT {
            let addr = base + index as u64 * PAGE_SIZE;
            unsafe {
                do_mmap(
                    &mut mm,
                    addr,
                    PAGE_SIZE,
                    PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    index as u64,
                    0,
                )
            }
            .expect("distinct shared-anonymous VMA");
        }
        assert_eq!(mm.map_count, VMA_COUNT as i32);
        assert_eq!(mm.total_vm, VMA_COUNT as u64);

        unsafe { do_munmap(&mut mm, base, VMA_COUNT as u64 * PAGE_SIZE) }.expect("bulk munmap");

        assert_eq!(mm.map_count, 0);
        assert_eq!(mm.total_vm, 0);
        assert_eq!(mm.mm_mt.count(), 0);
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
