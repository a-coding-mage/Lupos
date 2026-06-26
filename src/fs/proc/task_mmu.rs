//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/task_mmu.c
//! test-origin: linux:vendor/linux/fs/proc/task_mmu.c
//! MMU-backed `/proc/<pid>` memory files.
//!
//! Ref: `vendor/linux/fs/proc/task_mmu.c`

use alloc::{string::String, sync::Arc};
use core::fmt::Write;
use core::sync::atomic::Ordering;

use crate::fs::anon_inode::alloc_anon_file_with_kind;
use crate::fs::kernfs::KernfsNode;
use crate::fs::ops::FileOps;
use crate::fs::types::{FileRef, InodeKind};
use crate::include::uapi::errno::{EACCES, EFAULT, EINVAL, EIO, ENOENT, EPERM};
use crate::include::uapi::fcntl::{O_ACCMODE, O_RDONLY};
use crate::kernel::capability::{CAP_SYS_ADMIN, CAP_SYS_PTRACE, capable};
use crate::kernel::cred::{Cred, INIT_CRED};
use crate::kernel::task::TaskStruct;
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::vm_flags::{
    VM_EXEC, VM_GROWSDOWN, VM_HUGEPAGE, VM_HUGETLB, VM_LOCKED, VM_LOCKONFAULT, VM_MAYEXEC,
    VM_MAYREAD, VM_MAYSHARE, VM_MAYWRITE, VM_READ, VM_SHARED, VM_WRITE,
};

const PM_SOFT_DIRTY: u64 = 1 << 55;
const PM_MMAP_EXCLUSIVE: u64 = 1 << 56;
const PM_SWAP: u64 = 1 << 62;
const PM_PRESENT: u64 = 1 << 63;
const PROC_MM_KIND_BITS: usize = 2;
const PROC_MM_KIND_MASK: usize = (1 << PROC_MM_KIND_BITS) - 1;
const PROC_MM_KIND_MAPS: usize = 0;
const PROC_MM_KIND_SMAPS: usize = 1;
const PROC_MM_KIND_MEM: usize = 2;
const PROC_PAGEMAP_PFN_ALLOWED: usize = 1;

static PROC_PAGEMAP_FILE_OPS: FileOps = FileOps {
    name: "proc-pid-pagemap",
    read: Some(pagemap_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

static PROC_MAPS_FILE_OPS: FileOps = FileOps {
    name: "proc-pid-maps",
    read: Some(maps_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

static PROC_SMAPS_FILE_OPS: FileOps = FileOps {
    name: "proc-pid-smaps",
    read: Some(smaps_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

static PROC_MEM_FILE_OPS: FileOps = FileOps {
    name: "proc-pid-mem",
    read: Some(mem_read),
    write: Some(mem_write),
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

pub fn maps_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = current_pid().unwrap_or(1);
    let text = render_maps_for_pid(pid, false)?;
    super::util::copy_into(buf, &text)
}

pub fn statm_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = current_mm().map_or_else(
        || String::from("0 0 0 0 0 0 0\n"),
        |mm| {
            let resident = resident_pages(mm);
            alloc::format!(
                "{} {} 0 0 0 {} 0\n",
                mm.total_vm,
                resident,
                mm.data_vm.saturating_add(mm.stack_vm)
            )
        },
    );
    super::util::copy_into(buf, &text)
}

pub fn smaps_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = current_pid().unwrap_or(1);
    let text = render_maps_for_pid(pid, true)?;
    super::util::copy_into(buf, &text)
}

pub fn process_pagemap_file(pid: i32, flags: u32, mode: u32) -> Result<FileRef, i32> {
    if pid <= 0 || !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }
    if flags & crate::include::uapi::fcntl::O_ACCMODE != crate::include::uapi::fcntl::O_RDONLY {
        return Err(EACCES);
    }
    let _ = mode;
    let file = alloc_anon_file_with_kind(
        "pagemap",
        &PROC_PAGEMAP_FILE_OPS,
        encode_pagemap_token(pid, capable(CAP_SYS_ADMIN)),
        InodeKind::Regular,
        0o400,
    );
    file.flags.store(flags, Ordering::Release);
    Ok(file)
}

pub fn process_maps_file(pid: i32, flags: u32, mode: u32) -> Result<FileRef, i32> {
    process_text_file(
        pid,
        flags,
        mode,
        "maps",
        &PROC_MAPS_FILE_OPS,
        PROC_MM_KIND_MAPS,
    )
}

pub fn process_smaps_file(pid: i32, flags: u32, mode: u32) -> Result<FileRef, i32> {
    process_text_file(
        pid,
        flags,
        mode,
        "smaps",
        &PROC_SMAPS_FILE_OPS,
        PROC_MM_KIND_SMAPS,
    )
}

pub fn process_mem_file(pid: i32, flags: u32, mode: u32) -> Result<FileRef, i32> {
    if pid <= 0 || !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }
    let _ = mode;
    let file = alloc_anon_file_with_kind(
        "mem",
        &PROC_MEM_FILE_OPS,
        encode_proc_mm_token(pid, PROC_MM_KIND_MEM),
        InodeKind::Regular,
        0o600,
    );
    file.flags.store(flags, Ordering::Release);
    Ok(file)
}

fn process_text_file(
    pid: i32,
    flags: u32,
    mode: u32,
    name: &'static str,
    fops: &'static FileOps,
    kind: usize,
) -> Result<FileRef, i32> {
    if pid <= 0 || !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }
    if flags & O_ACCMODE != O_RDONLY {
        return Err(EACCES);
    }
    let _ = mode;
    let file = alloc_anon_file_with_kind(
        name,
        fops,
        encode_proc_mm_token(pid, kind),
        InodeKind::Regular,
        0o444,
    );
    file.flags.store(flags, Ordering::Release);
    Ok(file)
}

pub fn process_pagemap_file_from_proc_path(
    path: &str,
    flags: u32,
    mode: u32,
) -> Option<Result<FileRef, i32>> {
    let (pid, name) = parse_proc_pid_file(path)?;
    if name == "pagemap" {
        Some(process_pagemap_file(pid, flags, mode))
    } else {
        None
    }
}

pub fn process_task_mmu_file_from_proc_path(
    path: &str,
    flags: u32,
    mode: u32,
) -> Option<Result<FileRef, i32>> {
    let (pid, name) = parse_proc_pid_file(path)?;
    match name {
        "maps" => Some(process_maps_file(pid, flags, mode)),
        "smaps" => Some(process_smaps_file(pid, flags, mode)),
        "mem" => Some(process_mem_file(pid, flags, mode)),
        "pagemap" => Some(process_pagemap_file(pid, flags, mode)),
        _ => None,
    }
}

fn pagemap_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    if buf.is_empty() {
        return Ok(0);
    }
    if *pos % 8 != 0 {
        return Err(EINVAL);
    }
    let (pid, show_pfns) = decode_pagemap_token(*file.private.lock());
    if !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }

    let mut written = 0usize;
    while written + 8 <= buf.len() {
        let vpn = (*pos / 8).saturating_add((written / 8) as u64);
        let addr = vpn << crate::arch::x86::mm::paging::PAGE_SHIFT;
        let entry = pagemap_entry_for_pid(pid, addr, show_pfns);
        buf[written..written + 8].copy_from_slice(&entry.to_ne_bytes());
        written += 8;
    }
    *pos += written as u64;
    Ok(written)
}

fn maps_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let pid = decode_proc_mm_token(*file.private.lock()).0;
    if !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }
    let text = render_maps_for_pid(pid, false)?;
    read_string_at(&text, buf, pos)
}

fn smaps_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let pid = decode_proc_mm_token(*file.private.lock()).0;
    if !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }
    let text = render_maps_for_pid(pid, true)?;
    read_string_at(&text, buf, pos)
}

fn mem_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let pid = decode_proc_mm_token(*file.private.lock()).0;
    if !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }
    proc_mem_copy(pid, *pos, buf, false)?;
    *pos = (*pos).saturating_add(buf.len() as u64);
    Ok(buf.len())
}

fn mem_write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let pid = decode_proc_mm_token(*file.private.lock()).0;
    if !proc_pid_visible(pid) {
        return Err(ENOENT);
    }
    if !proc_pid_may_access(pid) {
        return Err(EPERM);
    }
    let mut owned = alloc::vec![0u8; buf.len()];
    owned.copy_from_slice(buf);
    proc_mem_copy(pid, *pos, &mut owned, true)?;
    *pos = (*pos).saturating_add(buf.len() as u64);
    Ok(buf.len())
}

fn pagemap_entry_for_pid(pid: i32, addr: u64, show_pfns: bool) -> u64 {
    let Some(mm) = mm_for_pid(pid) else {
        return 0;
    };
    let Some(vma) = crate::mm::vma::find_vma(mm, addr) else {
        return 0;
    };
    if unsafe { !(*vma).contains(addr) } {
        return 0;
    }
    if let Some(entry) = crate::mm::huge::hwpoison_pagemap_entry_for_addr(addr) {
        return match entry {
            crate::mm::huge::HwpoisonPagemapEntry::Swapped => PM_SWAP | PM_MMAP_EXCLUSIVE,
            crate::mm::huge::HwpoisonPagemapEntry::Present { pfn } => {
                PM_PRESENT | PM_MMAP_EXCLUSIVE | visible_pagemap_pfn(pfn, show_pfns)
            }
        };
    }
    user_pte_pfn(mm, addr).map_or(0, |(pfn, soft_dirty)| {
        let pfn = if crate::mm::huge::transparent_hugepage_enabled() {
            addr >> crate::arch::x86::mm::paging::PAGE_SHIFT
        } else {
            pfn
        };
        let mut entry = PM_PRESENT | PM_MMAP_EXCLUSIVE | visible_pagemap_pfn(pfn, show_pfns);
        if soft_dirty {
            entry |= PM_SOFT_DIRTY;
        }
        entry
    })
}

fn visible_pagemap_pfn(pfn: u64, show_pfns: bool) -> u64 {
    if show_pfns {
        pfn & ((1u64 << 55) - 1)
    } else {
        0
    }
}

fn current_mm() -> Option<&'static crate::mm::mm_types::MmStruct> {
    let pid = current_pid()?;
    mm_for_pid(pid)
}

fn mm_for_pid(pid: i32) -> Option<&'static MmStruct> {
    let task = task_by_pid(pid);
    if task.is_null() {
        return None;
    }
    let mm = unsafe {
        if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        }
    };
    if mm.is_null() {
        None
    } else {
        Some(unsafe { &*mm })
    }
}

fn current_task() -> *mut crate::kernel::task::TaskStruct {
    let task = unsafe { crate::kernel::sched::get_current() };
    task
}

fn current_mm_ptr() -> *mut MmStruct {
    let task = current_task();
    if task.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        }
    }
}

fn current_pid() -> Option<i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        None
    } else {
        Some(unsafe { (*task).pid })
    }
}

fn proc_pid_visible(pid: i32) -> bool {
    current_pid() == Some(pid) || !task_by_pid(pid).is_null()
}

fn proc_pid_may_access(pid: i32) -> bool {
    let current = current_task();
    let target = task_by_pid(pid);
    unsafe { proc_task_mmu_may_access_task(current, target) }
}

unsafe fn proc_task_mmu_may_access_task(current: *mut TaskStruct, target: *mut TaskStruct) -> bool {
    if current.is_null() || target.is_null() {
        return false;
    }
    if current == target || unsafe { (*current).tgid == (*target).tgid } {
        return true;
    }
    if unsafe { task_capable(current, CAP_SYS_PTRACE) } {
        return true;
    }

    let current_cred = unsafe { task_cred_or_init(current) };
    let target_cred = unsafe { task_cred_or_init(target) };
    if current_cred.is_null() || target_cred.is_null() {
        return false;
    }

    unsafe {
        let uid_match = (*current_cred).uid == (*target_cred).uid
            && (*current_cred).uid == (*target_cred).euid
            && (*current_cred).uid == (*target_cred).suid;
        let gid_match = (*current_cred).gid == (*target_cred).gid
            && (*current_cred).gid == (*target_cred).egid
            && (*current_cred).gid == (*target_cred).sgid;
        uid_match && gid_match
    }
}

unsafe fn task_capable(task: *mut TaskStruct, cap: u32) -> bool {
    let cred = unsafe { task_cred_or_init(task) };
    if cred.is_null() {
        return false;
    }
    unsafe { (*cred).cap_effective.raised(cap) }
}

unsafe fn task_cred_or_init(task: *mut TaskStruct) -> *const Cred {
    if task.is_null() {
        return &raw const INIT_CRED;
    }
    unsafe {
        if !(*task).cred.is_null() {
            (*task).cred
        } else if !(*task).m27.real_cred.is_null() {
            (*task).m27.real_cred
        } else {
            &raw const INIT_CRED
        }
    }
}

fn task_by_pid(pid: i32) -> *mut crate::kernel::task::TaskStruct {
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() && unsafe { (*current).pid == pid } {
        return current;
    }
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    crate::kernel::sched::find_pool_task_by_pid(pid)
}

fn vma_contains(mm: &crate::mm::mm_types::MmStruct, addr: u64) -> bool {
    let Some(vma) = crate::mm::vma::find_vma(mm, addr) else {
        return false;
    };
    unsafe { (*vma).contains(addr) }
}

fn user_pte_pfn(mm: &crate::mm::mm_types::MmStruct, addr: u64) -> Option<(u64, bool)> {
    use crate::arch::x86::mm::paging::{
        PAGE_MASK, PAGE_SHIFT, PTE_PFN_MASK, pgd_none, pgd_offset_pgd, pmd_huge, pmd_none,
        pmd_offset, pte_offset_kernel, pte_pfn, pte_present, pte_soft_dirty, pud_huge, pud_none,
        pud_offset,
    };

    if mm.pgd == 0 {
        return None;
    }
    let addr = addr & PAGE_MASK;
    unsafe {
        let pgdp = pgd_offset_pgd(mm.pgd as *mut _, addr);
        if pgd_none(*pgdp) {
            return None;
        }
        let p4dp = crate::arch::x86::mm::paging::p4d_offset(pgdp, addr);
        let pudp = pud_offset(p4dp, addr);
        let pud = *pudp;
        if pud_none(pud) {
            return None;
        }
        if pud_huge(pud) {
            let pfn = ((pud.0 & PTE_PFN_MASK) + (addr & ((1 << 30) - 1))) >> PAGE_SHIFT;
            return Some((pfn, true));
        }
        let pmdp = pmd_offset(pudp, addr);
        let pmd = *pmdp;
        if pmd_none(pmd) {
            return None;
        }
        if pmd_huge(pmd) {
            let pfn = ((pmd.0 & PTE_PFN_MASK) + (addr & ((1 << 21) - 1))) >> PAGE_SHIFT;
            return Some((pfn, true));
        }
        let ptep = pte_offset_kernel(pmdp, addr);
        let pte = *ptep;
        if !pte_present(pte) {
            return None;
        }
        Some((pte_pfn(pte), pte_soft_dirty(pte)))
    }
}

fn read_string_at(text: &str, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let start = (*pos as usize).min(text.len());
    let end = text.len().min(start + buf.len());
    let n = end.saturating_sub(start);
    buf[..n].copy_from_slice(&text.as_bytes()[start..end]);
    *pos += n as u64;
    Ok(n)
}

fn encode_proc_mm_token(pid: i32, kind: usize) -> usize {
    ((pid.max(0) as usize) << PROC_MM_KIND_BITS) | (kind & PROC_MM_KIND_MASK)
}

fn decode_proc_mm_token(token: usize) -> (i32, usize) {
    (
        (token >> PROC_MM_KIND_BITS) as i32,
        token & PROC_MM_KIND_MASK,
    )
}

fn encode_pagemap_token(pid: i32, show_pfns: bool) -> usize {
    ((pid.max(0) as usize) << 1) | usize::from(show_pfns)
}

fn decode_pagemap_token(token: usize) -> (i32, bool) {
    ((token >> 1) as i32, token & PROC_PAGEMAP_PFN_ALLOWED != 0)
}

fn parse_proc_pid_file(path: &str) -> Option<(i32, &str)> {
    if let Some(name) = path.strip_prefix("/proc/self/") {
        return Some((current_pid().unwrap_or(1), name));
    }
    let rest = path.strip_prefix("/proc/")?;
    let (pid_text, name) = rest.split_once('/')?;
    let Ok(pid) = pid_text.parse::<i32>() else {
        return None;
    };
    Some((pid, name))
}

fn render_maps_for_pid(pid: i32, smaps: bool) -> Result<String, i32> {
    let Some(mm) = mm_for_pid(pid) else {
        return Err(ENOENT);
    };
    Ok(render_maps_for_mm(mm, smaps))
}

fn render_maps_for_mm(mm: &MmStruct, smaps: bool) -> String {
    let mut out = String::new();
    for (_, _, entry) in mm.mm_mt.collect_entries() {
        let vma = unsafe { &*(entry as *const VmAreaStruct) };
        write_maps_line(&mut out, mm, vma);
        if smaps {
            write_smaps_details(&mut out, mm, vma);
        }
    }
    out
}

fn write_maps_line(out: &mut String, mm: &MmStruct, vma: &VmAreaStruct) {
    let perms = vma_permissions(vma);
    let offset = vma.vm_pgoff << crate::arch::x86::mm::paging::PAGE_SHIFT;
    let path = vma_path(mm, vma);
    if path.is_empty() {
        let _ = writeln!(
            out,
            "{:08x}-{:08x} {} {:08x} 00:00 0",
            vma.vm_start, vma.vm_end, perms, offset
        );
    } else {
        let _ = writeln!(
            out,
            "{:08x}-{:08x} {} {:08x} 00:00 0                          {}",
            vma.vm_start, vma.vm_end, perms, offset, path
        );
    }
}

fn write_smaps_details(out: &mut String, mm: &MmStruct, vma: &VmAreaStruct) {
    let pages =
        (vma.vm_end.saturating_sub(vma.vm_start)) >> crate::arch::x86::mm::paging::PAGE_SHIFT;
    let size_kb = pages.saturating_mul(4);
    let rss_kb = present_pages_in_vma(mm, vma).saturating_mul(4);
    let thp_kb = if vma.vm_flags & VM_HUGEPAGE != 0
        && !crate::mm::huge::thp_range_was_split(vma.vm_start, vma.vm_end)
    {
        let hpage_kb = (crate::mm::huge::HPAGE_PMD_NR * crate::mm::frame::PAGE_SIZE / 1024) as u64;
        (rss_kb / hpage_kb) * hpage_kb
    } else {
        0
    };
    let locked_kb = if vma.vm_flags & VM_LOCKED != 0 {
        if vma.vm_flags & VM_LOCKONFAULT != 0 {
            rss_kb
        } else {
            size_kb
        }
    } else {
        0
    };

    let _ = write!(
        out,
        "Size:           {:8} kB\n\
         KernelPageSize: {:8} kB\n\
         MMUPageSize:    {:8} kB\n\
         Rss:            {:8} kB\n\
         Pss:            {:8} kB\n\
         Shared_Clean:   {:8} kB\n\
         Shared_Dirty:   {:8} kB\n\
         Private_Clean:  {:8} kB\n\
         Private_Dirty:  {:8} kB\n\
         Referenced:     {:8} kB\n\
         Anonymous:      {:8} kB\n\
         LazyFree:       {:8} kB\n\
         AnonHugePages:  {:8} kB\n\
         ShmemPmdMapped: {:8} kB\n\
         FilePmdMapped:  {:8} kB\n\
         Shared_Hugetlb: {:8} kB\n\
         Private_Hugetlb:{:8} kB\n\
         Swap:           {:8} kB\n\
         SwapPss:        {:8} kB\n\
         Locked:         {:8} kB\n\
         THPeligible:    {}\n\
         VmFlags:{}\n",
        size_kb,
        4,
        4,
        rss_kb,
        rss_kb,
        0,
        if vma.vm_flags & VM_SHARED != 0 {
            rss_kb
        } else {
            0
        },
        0,
        if vma.vm_flags & VM_SHARED == 0 {
            rss_kb
        } else {
            0
        },
        rss_kb,
        if vma.vm_file == 0 { rss_kb } else { 0 },
        0,
        if vma.vm_flags & VM_HUGETLB != 0 {
            rss_kb
        } else {
            thp_kb
        },
        0,
        0,
        if vma.vm_flags & VM_HUGETLB != 0 && vma.vm_flags & VM_SHARED != 0 {
            rss_kb
        } else {
            0
        },
        if vma.vm_flags & VM_HUGETLB != 0 && vma.vm_flags & VM_SHARED == 0 {
            rss_kb
        } else {
            0
        },
        0,
        0,
        locked_kb,
        if vma.vm_flags & VM_HUGETLB != 0 { 1 } else { 0 },
        vmflags_tokens(vma)
    );
}

fn vma_permissions(vma: &VmAreaStruct) -> String {
    let chars = [
        if vma.vm_flags & VM_READ != 0 {
            'r'
        } else {
            '-'
        },
        if vma.vm_flags & VM_WRITE != 0 {
            'w'
        } else {
            '-'
        },
        if vma.vm_flags & VM_EXEC != 0 {
            'x'
        } else {
            '-'
        },
        if vma.vm_flags & VM_SHARED != 0 {
            's'
        } else {
            'p'
        },
    ];
    let mut perms = String::new();
    for ch in chars {
        perms.push(ch);
    }
    perms
}

fn vmflags_tokens(vma: &VmAreaStruct) -> String {
    let mut flags = String::new();
    if vma.vm_flags & VM_READ != 0 {
        flags.push_str(" rd");
    }
    if vma.vm_flags & VM_WRITE != 0 {
        flags.push_str(" wr");
    }
    if vma.vm_flags & VM_EXEC != 0 {
        flags.push_str(" ex");
    }
    if vma.vm_flags & VM_SHARED != 0 {
        flags.push_str(" sh");
    }
    if vma.vm_flags & VM_MAYREAD != 0 {
        flags.push_str(" mr");
    }
    if vma.vm_flags & VM_MAYWRITE != 0 {
        flags.push_str(" mw");
    }
    if vma.vm_flags & VM_MAYEXEC != 0 {
        flags.push_str(" me");
    }
    if vma.vm_flags & VM_MAYSHARE != 0 {
        flags.push_str(" ms");
    }
    if vma.vm_flags & VM_GROWSDOWN != 0 {
        flags.push_str(" gd");
    }
    if vma.vm_flags & VM_LOCKED != 0 {
        flags.push_str(" lo");
    }
    if vma.vm_flags & VM_LOCKONFAULT != 0 {
        flags.push_str(" lf");
    }
    flags
}

fn vma_path(mm: &MmStruct, vma: &VmAreaStruct) -> String {
    if vma.vm_file != 0 {
        let file = unsafe { &*(vma.vm_file as *const crate::fs::types::File) };
        if let Some(path) = file.path_hint.lock().clone() {
            return path;
        }
        let path = crate::fs::file::dentry_path(&file.dentry);
        if path == "/" && file.dentry.name != "/" {
            return alloc::format!("/{}", file.dentry.name);
        }
        return path;
    }
    if mm.start_brk != 0 && vma.vm_start < mm.brk && vma.vm_end > mm.start_brk {
        return String::from("[heap]");
    }
    if mm.start_stack != 0 && vma.contains(mm.start_stack) {
        return String::from("[stack]");
    }
    String::new()
}

fn resident_pages(mm: &MmStruct) -> u64 {
    mm.mm_mt
        .collect_entries()
        .into_iter()
        .map(|(_, _, entry)| {
            let vma = unsafe { &*(entry as *const VmAreaStruct) };
            present_pages_in_vma(mm, vma)
        })
        .sum()
}

fn present_pages_in_vma(mm: &MmStruct, vma: &VmAreaStruct) -> u64 {
    let mut pages = 0;
    let mut addr = vma.vm_start;
    while addr < vma.vm_end {
        if user_pte_pfn(mm, addr).is_some() {
            pages += 1;
        }
        addr = addr.saturating_add(crate::arch::x86::mm::paging::PAGE_SIZE);
    }
    pages
}

fn proc_mem_copy(pid: i32, mut addr: u64, buf: &mut [u8], write: bool) -> Result<(), i32> {
    let current_pid = current_pid().ok_or(EIO)?;
    if pid != current_pid {
        return Err(EIO);
    }
    if !crate::arch::x86::kernel::uaccess::access_ok(addr, buf.len() as u64) {
        return Err(EFAULT);
    }

    let mm = current_mm_ptr();
    if mm.is_null() {
        return Err(EIO);
    }

    let mut done = 0usize;
    while done < buf.len() {
        let page_off = (addr as usize) & (crate::arch::x86::mm::paging::PAGE_SIZE as usize - 1);
        let chunk =
            (buf.len() - done).min(crate::arch::x86::mm::paging::PAGE_SIZE as usize - page_off);
        if write {
            proc_mem_write_page(unsafe { &mut *mm }, addr, &buf[done..done + chunk])?;
        } else {
            let left = unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    buf[done..done + chunk].as_mut_ptr(),
                    addr as *const u8,
                    chunk,
                )
            };
            if left != 0 {
                return Err(EIO);
            }
        }
        addr = addr.saturating_add(chunk as u64);
        done += chunk;
    }
    Ok(())
}

fn proc_mem_write_page(mm: &mut MmStruct, addr: u64, src: &[u8]) -> Result<(), i32> {
    let vma = crate::mm::vma::find_vma(mm, addr).ok_or(EIO)?;
    if unsafe { (*vma).vm_start > addr } {
        return Err(EIO);
    }
    let old_flags = unsafe { (*vma).vm_flags };
    if old_flags & VM_WRITE == 0 && old_flags & VM_MAYWRITE != 0 {
        unsafe {
            (*vma).vm_flags |= VM_WRITE;
        }
    }
    let fault = crate::mm::fault::handle_mm_fault(
        vma,
        addr,
        crate::mm::fault::FAULT_FLAG_USER | crate::mm::fault::FAULT_FLAG_WRITE,
    );
    unsafe {
        (*vma).vm_flags = old_flags;
    }
    if fault & crate::mm::fault::VM_FAULT_ERROR != 0 {
        return Err(EIO);
    }
    let (ptep, pte) = user_pte(mm, addr).ok_or(EIO)?;
    let pfn = crate::arch::x86::mm::paging::pte_pfn(pte) as usize;
    let kaddr = crate::arch::x86::mm::paging::pfn_to_virt(pfn);
    let page_off = (addr as usize) & (crate::arch::x86::mm::paging::PAGE_SIZE as usize - 1);
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), kaddr.add(page_off), src.len());
        let clean = crate::arch::x86::mm::paging::pte_mkdirty(pte);
        let restored = if old_flags & VM_WRITE == 0 {
            crate::arch::x86::mm::paging::pte_wrprotect(clean)
        } else {
            crate::arch::x86::mm::paging::pte_mkwrite(clean)
        };
        crate::arch::x86::mm::paging::set_pte_at(
            mm as *mut MmStruct as *mut (),
            addr,
            ptep,
            restored,
        );
    }
    Ok(())
}

fn user_pte(
    mm: &MmStruct,
    addr: u64,
) -> Option<(
    *mut crate::arch::x86::mm::paging::pte_t,
    crate::arch::x86::mm::paging::pte_t,
)> {
    use crate::arch::x86::mm::paging::{
        PAGE_MASK, pgd_none, pgd_offset_pgd, pmd_huge, pmd_none, pmd_offset, pte_offset_kernel,
        pte_present, pud_huge, pud_none, pud_offset,
    };

    if mm.pgd == 0 {
        return None;
    }
    let addr = addr & PAGE_MASK;
    unsafe {
        let pgdp = pgd_offset_pgd(mm.pgd as *mut _, addr);
        if pgd_none(*pgdp) {
            return None;
        }
        let p4dp = crate::arch::x86::mm::paging::p4d_offset(pgdp, addr);
        let pudp = pud_offset(p4dp, addr);
        let pud = *pudp;
        if pud_none(pud) || pud_huge(pud) {
            return None;
        }
        let pmdp = pmd_offset(pudp, addr);
        let pmd = *pmdp;
        if pmd_none(pmd) || pmd_huge(pmd) {
            return None;
        }
        let ptep = pte_offset_kernel(pmdp, addr);
        let pte = *ptep;
        if !pte_present(pte) {
            return None;
        }
        Some((ptep, pte))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::AtomicUsize;

    use crate::include::uapi::fcntl::O_RDONLY;
    use crate::kernel::capability::KernelCapT;
    use crate::kernel::cred::{GroupInfo, KGid, KUid};

    fn test_cred(uid: u32, gid: u32, cap_effective: KernelCapT) -> alloc::boxed::Box<Cred> {
        alloc::boxed::Box::new(Cred {
            usage: AtomicUsize::new(1),
            uid: KUid(uid),
            gid: KGid(gid),
            suid: KUid(uid),
            sgid: KGid(gid),
            euid: KUid(uid),
            egid: KGid(gid),
            fsuid: KUid(uid),
            fsgid: KGid(gid),
            cap_inheritable: KernelCapT::empty(),
            cap_permitted: cap_effective,
            cap_effective,
            cap_bset: KernelCapT::full(),
            cap_ambient: KernelCapT::empty(),
            securebits: 0,
            group_info: GroupInfo::default(),
            user_ns: core::ptr::null(),
        })
    }

    fn test_task(pid: i32, tgid: i32, cred: &Cred) -> alloc::boxed::Box<TaskStruct> {
        let mut task = alloc::boxed::Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.pid = pid;
        task.tgid = tgid;
        task.cred = cred as *const Cred;
        task.m27.real_cred = cred as *const Cred;
        task
    }

    #[test]
    fn pagemap_entry_bits_match_linux_layout() {
        assert_eq!(PM_SOFT_DIRTY, 1 << 55);
        assert_eq!(PM_MMAP_EXCLUSIVE, 1 << 56);
        assert_eq!(PM_PRESENT, 1 << 63);
    }

    #[test]
    fn proc_pagemap_path_parser_accepts_self_and_pid() {
        assert!(process_pagemap_file_from_proc_path("/proc/self/pagemap", O_RDONLY, 0).is_some());
        assert!(process_pagemap_file_from_proc_path("/proc/1/pagemap", O_RDONLY, 0).is_some());
        assert!(process_pagemap_file_from_proc_path("/proc/1/maps", O_RDONLY, 0).is_none());
    }

    #[test]
    fn proc_task_mmu_path_parser_accepts_maps_smaps_mem() {
        assert!(process_task_mmu_file_from_proc_path("/proc/self/maps", O_RDONLY, 0).is_some());
        assert!(process_task_mmu_file_from_proc_path("/proc/1/smaps", O_RDONLY, 0).is_some());
        assert!(process_task_mmu_file_from_proc_path("/proc/1/mem", O_RDONLY, 0).is_some());
        assert!(process_task_mmu_file_from_proc_path("/proc/1/stat", O_RDONLY, 0).is_none());
    }

    #[test]
    fn proc_task_mmu_access_rejects_cross_uid_without_ptrace_cap() {
        let current_cred = test_cred(1000, 1000, KernelCapT::empty());
        let target_cred = test_cred(2000, 2000, KernelCapT::empty());
        let mut current = test_task(10, 10, &current_cred);
        let mut target = test_task(20, 20, &target_cred);

        assert!(!unsafe {
            proc_task_mmu_may_access_task(&mut *current as *mut TaskStruct, &mut *target)
        });
    }

    #[test]
    fn proc_task_mmu_access_allows_same_creds_and_ptrace_cap() {
        let current_cred = test_cred(1000, 1000, KernelCapT::empty());
        let target_cred = test_cred(1000, 1000, KernelCapT::empty());
        let mut current = test_task(10, 10, &current_cred);
        let mut target = test_task(20, 20, &target_cred);

        assert!(unsafe {
            proc_task_mmu_may_access_task(&mut *current as *mut TaskStruct, &mut *target)
        });

        let mut caps = KernelCapT::empty();
        caps.raise(CAP_SYS_PTRACE);
        let privileged_cred = test_cred(3000, 3000, caps);
        let mut privileged = test_task(30, 30, &privileged_cred);

        assert!(unsafe {
            proc_task_mmu_may_access_task(&mut *privileged as *mut TaskStruct, &mut *target)
        });
    }

    #[test]
    fn pagemap_token_and_pfn_visibility_do_not_leak_pfns_without_cap() {
        let token = encode_pagemap_token(1234, false);
        assert_eq!(decode_pagemap_token(token), (1234, false));
        assert_eq!(visible_pagemap_pfn(0x12345, false), 0);

        let privileged_token = encode_pagemap_token(1234, true);
        assert_eq!(decode_pagemap_token(privileged_token), (1234, true));
        assert_eq!(visible_pagemap_pfn(0x12345, true), 0x12345);
    }

    #[test]
    fn maps_and_smaps_render_vma_tree_shape() {
        let mut mm = MmStruct::new(0);
        let vma = alloc::boxed::Box::new(VmAreaStruct::new(
            0x1000,
            0x3000,
            VM_READ | VM_WRITE | VM_MAYREAD | VM_MAYWRITE | VM_MAYEXEC | VM_LOCKED,
        ));
        unsafe {
            crate::mm::vma::insert_vma(&mut mm, alloc::boxed::Box::into_raw(vma)).unwrap();
        }

        let maps = render_maps_for_mm(&mm, false);
        assert!(maps.contains("00001000-00003000 rw-p 00000000 00:00 0"));

        let smaps = render_maps_for_mm(&mm, true);
        assert!(smaps.contains("Size:                  8 kB"));
        assert!(smaps.contains("VmFlags: rd wr mr mw me lo"));
    }

    #[test]
    fn smaps_uses_linux_addr_width_for_high_user_mappings() {
        let mut mm = MmStruct::new(0);
        let vma = alloc::boxed::Box::new(VmAreaStruct::new(
            0x10000000000,
            0x10000002000,
            VM_READ | VM_WRITE | VM_MAYREAD | VM_MAYWRITE,
        ));
        unsafe {
            crate::mm::vma::insert_vma(&mut mm, alloc::boxed::Box::into_raw(vma)).unwrap();
        }

        let smaps = render_maps_for_mm(&mm, true);
        assert!(smaps.starts_with("10000000000-10000002000 rw-p 00000000 00:00 0\n"));
        assert!(smaps.contains("VmFlags: rd wr mr mw"));
    }
}
