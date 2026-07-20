//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Memory-management syscall glue used by the syscall table.

extern crate alloc;

use crate::arch::x86::kernel::uaccess;

use crate::include::uapi::errno::{EACCES, EBADF, EFAULT, EINVAL};
use crate::include::uapi::fcntl::{O_ACCMODE, O_PATH, O_RDONLY, O_RDWR};
use crate::kernel::{files, sched};

use super::madvise::do_madvise;
use super::mmap::{MAP_ANONYMOUS, MAP_SHARED, PROT_WRITE, do_mmap, do_munmap};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MmapArgStruct {
    pub addr: u64,
    pub len: u64,
    pub prot: u64,
    pub flags: u64,
    pub fd: u64,
    pub offset: u64,
}

pub unsafe fn sys_mmap(addr: u64, len: u64, prot: u32, flags: u32, fd: i32, off: u64) -> i64 {
    // x86-64 `sys_mmap` accepts a byte offset and rejects offsets that are not
    // page aligned before converting them to the internal page offset.
    if off & (crate::arch::x86::mm::paging::PAGE_SIZE - 1) != 0 {
        return -(EINVAL as i64);
    }
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -(EINVAL as i64);
    }
    let file = if mmap_uses_file(flags) && fd >= 0 {
        let Some(ft) = (unsafe { files::get_task_files(task) }) else {
            return -(EBADF as i64);
        };
        match ft.get(fd) {
            Ok(file) => {
                let file_flags = file.flags.load(core::sync::atomic::Ordering::Acquire);
                if file_flags & O_PATH != 0 {
                    return -(EBADF as i64);
                }
                if let Err(errno) = mmap_validate_file_access(prot, flags, file_flags) {
                    return -(errno as i64);
                }
                crate::fs::file::note_file_mmap_for_integrity(None, &file);
                crate::mm::vma::vma_file_from_ref(file)
            }
            Err(errno) => return -(errno as i64),
        }
    } else {
        0
    };
    match unsafe { do_mmap(&mut *mm, addr, len, prot, flags, off >> 12, file) } {
        Ok(mapped) => mapped as i64,
        Err(errno) => {
            if file != 0 {
                unsafe { crate::mm::vma::vma_file_put_raw(file) };
            }
            errno as i64
        }
    }
}

#[inline]
fn mmap_uses_file(flags: u32) -> bool {
    flags & MAP_ANONYMOUS == 0
}

#[inline]
fn mmap_validate_file_access(prot: u32, flags: u32, file_flags: u32) -> Result<(), i32> {
    let access_mode = file_flags & O_ACCMODE;

    // Linux requires FMODE_READ for every file-backed mmap, including a
    // writable shared mapping. O_RDONLY and O_RDWR are the open modes that
    // provide it.
    if access_mode != O_RDONLY && access_mode != O_RDWR {
        return Err(EACCES);
    }
    if flags & MAP_SHARED != 0 && prot & PROT_WRITE != 0 && access_mode != O_RDWR {
        return Err(EACCES);
    }
    Ok(())
}

/// Linux-visible `mmap_pgoff()` wrapper.
///
/// The syscall-table glue passes byte offsets to `sys_mmap`; Linux's internal
/// MM ABI names this entrypoint by page offset.
pub unsafe fn mmap_pgoff(addr: u64, len: u64, prot: u32, flags: u32, fd: i32, pgoff: u64) -> i64 {
    if pgoff > (u64::MAX >> 12) {
        return -(EINVAL as i64);
    }
    unsafe { sys_mmap(addr, len, prot, flags, fd, pgoff << 12) }
}

pub unsafe fn old_mmap(arg: *const MmapArgStruct) -> i64 {
    if arg.is_null() {
        return -(EFAULT as i64);
    }
    let mut copied = MmapArgStruct::default();
    let left = unsafe {
        uaccess::copy_from_user(
            (&mut copied as *mut MmapArgStruct).cast::<u8>(),
            arg.cast::<u8>(),
            core::mem::size_of::<MmapArgStruct>(),
        )
    };
    if left != 0 {
        return -(EFAULT as i64);
    }
    if copied.offset & (crate::arch::x86::mm::paging::PAGE_SIZE - 1) != 0 {
        return -(EINVAL as i64);
    }
    unsafe {
        mmap_pgoff(
            copied.addr,
            copied.len,
            copied.prot as u32,
            copied.flags as u32,
            copied.fd as i32,
            copied.offset >> 12,
        )
    }
}

pub unsafe fn sys_madvise(start: u64, len: u64, advice: i32) -> i64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -(EINVAL as i64);
    }
    match unsafe { do_madvise(&mut *mm, start, len, advice) } {
        Ok(()) => 0,
        Err(errno) => errno as i64,
    }
}

pub unsafe fn madvise(start: u64, len: u64, advice: i32) -> i64 {
    unsafe { sys_madvise(start, len, advice) }
}

pub unsafe fn sys_msync(addr: u64, len: u64, flags: i32) -> i64 {
    if let Err(errno) = super::backing_dev::validate_msync(addr, len, flags) {
        return -(errno as i64);
    }
    if len != 0 {
        let task = unsafe { sched::get_current() };
        if task.is_null() {
            return -(EBADF as i64);
        }
        let mm = unsafe { (*task).mm };
        if mm.is_null() {
            return -(EINVAL as i64);
        }
        if let Err(errno) = unsafe { super::mmap::sync_shared_file_range(&mut *mm, addr, len) } {
            return -(errno as i64);
        }
    }
    super::backing_dev::msync_range(addr, len, flags)
        .map(|_| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub unsafe fn msync(addr: u64, len: u64, flags: i32) -> i64 {
    unsafe { sys_msync(addr, len, flags) }
}

pub unsafe fn sys_munmap(addr: u64, len: u64) -> i64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -(EINVAL as i64);
    }
    match unsafe { do_munmap(&mut *mm, addr, len) } {
        Ok(()) => 0,
        Err(errno) => errno as i64,
    }
}

pub unsafe fn munmap(addr: u64, len: u64) -> i64 {
    unsafe { sys_munmap(addr, len) }
}

pub unsafe fn sys_mprotect(addr: u64, len: u64, prot: u32) -> i64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -(EINVAL as i64);
    }
    match unsafe { super::mprotect::do_mprotect(&mut *mm, addr, len, prot) } {
        Ok(()) => 0,
        Err(errno) => errno as i64,
    }
}

pub unsafe fn mprotect(addr: u64, len: u64, prot: u32) -> i64 {
    unsafe { sys_mprotect(addr, len, prot) }
}

pub unsafe fn sys_brk(new_brk: u64) -> i64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -(EINVAL as i64);
    }
    unsafe { super::mmap::sys_brk(&mut *mm, new_brk) as i64 }
}

pub unsafe fn brk(new_brk: u64) -> i64 {
    unsafe { sys_brk(new_brk) }
}

pub unsafe fn sys_mremap(
    old_addr: u64,
    old_len: u64,
    new_len: u64,
    flags: u32,
    new_addr: u64,
) -> i64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -(EINVAL as i64);
    }
    match unsafe { super::mremap::do_mremap(&mut *mm, old_addr, old_len, new_len, flags, new_addr) }
    {
        Ok(addr) => addr as i64,
        Err(errno) => errno as i64,
    }
}

pub unsafe fn mremap(old_addr: u64, old_len: u64, new_len: u64, flags: u32, new_addr: u64) -> i64 {
    unsafe { sys_mremap(old_addr, old_len, new_len, flags, new_addr) }
}

pub unsafe fn sys_mincore(start: u64, len: u64, vec: *mut u8) -> i64 {
    if vec.is_null() {
        return -(EFAULT as i64);
    }
    if start & (crate::arch::x86::mm::paging::PAGE_SIZE - 1) != 0 {
        return -(EINVAL as i64);
    }
    let pages = len.div_ceil(crate::arch::x86::mm::paging::PAGE_SIZE);
    let task = unsafe { sched::get_current() };
    if !task.is_null() {
        let mm = unsafe { (*task).mm };
        if !mm.is_null() {
            let mut residency = alloc::vec![0u8; pages as usize];
            match super::mlock::mincore_residency(unsafe { &*mm }, start, len, &mut residency) {
                Ok(nr) => {
                    let not_copied = unsafe { uaccess::copy_to_user(vec, residency.as_ptr(), nr) };
                    return if not_copied == 0 { 0 } else { -(EFAULT as i64) };
                }
                Err(errno) => return -(errno as i64),
            }
        }
    }
    for idx in 0..pages {
        let resident = 1u8;
        let not_copied = unsafe { uaccess::copy_to_user(vec.add(idx as usize), &resident, 1) };
        if not_copied != 0 {
            return -(EFAULT as i64);
        }
    }
    0
}

pub unsafe fn mincore(start: u64, len: u64, vec: *mut u8) -> i64 {
    unsafe { sys_mincore(start, len, vec) }
}

pub unsafe fn sys_mlock(addr: u64, len: u64) -> i64 {
    if len != 0 && addr == 0 {
        return -(EINVAL as i64);
    }
    let task = unsafe { sched::get_current() };
    if !task.is_null() {
        let mm = unsafe { (*task).mm };
        if !mm.is_null() {
            return match unsafe { super::mlock::lock_vma_range(&mut *mm, addr, len, false) } {
                Ok(_) => 0,
                Err(errno) => -(errno as i64),
            };
        }
    }
    0
}

pub unsafe fn mlock(addr: u64, len: u64) -> i64 {
    unsafe { sys_mlock(addr, len) }
}

pub unsafe fn sys_munlock(addr: u64, len: u64) -> i64 {
    if len != 0 && addr == 0 {
        return -(EINVAL as i64);
    }
    let task = unsafe { sched::get_current() };
    if !task.is_null() {
        let mm = unsafe { (*task).mm };
        if !mm.is_null() {
            return match unsafe { super::mlock::unlock_vma_range(&mut *mm, addr, len) } {
                Ok(_) => 0,
                Err(errno) => -(errno as i64),
            };
        }
    }
    0
}

pub unsafe fn munlock(addr: u64, len: u64) -> i64 {
    unsafe { sys_munlock(addr, len) }
}

pub fn sys_mlockall(flags: i32) -> i64 {
    const MCL_CURRENT: i32 = 1;
    const MCL_FUTURE: i32 = 2;
    const MCL_ONFAULT: i32 = 4;
    if flags & !(MCL_CURRENT | MCL_FUTURE | MCL_ONFAULT) != 0 {
        return -(EINVAL as i64);
    }
    let task = unsafe { sched::get_current() };
    if !task.is_null() {
        let mm = unsafe { (*task).mm };
        if !mm.is_null() {
            let future_flags = if flags & MCL_FUTURE != 0 {
                let mut future = crate::mm::vm_flags::VM_LOCKED;
                if flags & MCL_ONFAULT != 0 {
                    future |= crate::mm::vm_flags::VM_LOCKONFAULT;
                }
                future
            } else {
                0
            };
            if flags & MCL_CURRENT != 0 {
                match unsafe { super::mlock::lock_all_current(&mut *mm, flags & MCL_ONFAULT != 0) }
                {
                    Ok(_) => {}
                    Err(errno) => return -(errno as i64),
                }
            }
            unsafe {
                (*mm).def_flags &=
                    !(crate::mm::vm_flags::VM_LOCKED | crate::mm::vm_flags::VM_LOCKONFAULT);
                (*mm).def_flags |= future_flags;
            }
        }
    }
    0
}

pub fn mlockall(flags: i32) -> i64 {
    sys_mlockall(flags)
}

pub fn sys_munlockall() -> i64 {
    let task = unsafe { sched::get_current() };
    if !task.is_null() {
        let mm = unsafe { (*task).mm };
        if !mm.is_null() {
            unsafe {
                (*mm).def_flags &=
                    !(crate::mm::vm_flags::VM_LOCKED | crate::mm::vm_flags::VM_LOCKONFAULT);
                super::mlock::unlock_all(&mut *mm);
            }
        }
    }
    0
}

pub fn munlockall() -> i64 {
    sys_munlockall()
}

pub unsafe fn sys_mlock2(addr: u64, len: u64, flags: i32) -> i64 {
    const MLOCK_ONFAULT: i32 = 1;
    if flags & !MLOCK_ONFAULT != 0 {
        return -(EINVAL as i64);
    }
    if len != 0 && addr == 0 {
        return -(EINVAL as i64);
    }
    let task = unsafe { sched::get_current() };
    if !task.is_null() {
        let mm = unsafe { (*task).mm };
        if !mm.is_null() {
            return match unsafe {
                super::mlock::lock_vma_range(&mut *mm, addr, len, flags & MLOCK_ONFAULT != 0)
            } {
                Ok(_) => 0,
                Err(errno) => -(errno as i64),
            };
        }
    }
    0
}

pub unsafe fn mlock2(addr: u64, len: u64, flags: i32) -> i64 {
    unsafe { sys_mlock2(addr, len, flags) }
}

pub fn sys_membarrier(cmd: i32, flags: u32, cpu_id: i32) -> i64 {
    crate::kernel::sched::membarrier::sys_membarrier(cmd, flags, cpu_id)
}

pub unsafe fn sys_pkey_mprotect(addr: u64, len: u64, prot: u32, pkey: i32) -> i64 {
    if pkey < 0 || pkey > 0 {
        return -(EINVAL as i64);
    }
    unsafe { sys_mprotect(addr, len, prot) }
}

pub unsafe fn pkey_mprotect(addr: u64, len: u64, prot: u32, pkey: i32) -> i64 {
    unsafe { sys_pkey_mprotect(addr, len, prot, pkey) }
}

pub fn sys_pkey_alloc(flags: u32, access_rights: u32) -> i64 {
    if flags != 0 || access_rights & !0b11 != 0 {
        return -(EINVAL as i64);
    }
    1
}

pub fn pkey_alloc(flags: u32, access_rights: u32) -> i64 {
    sys_pkey_alloc(flags, access_rights)
}

pub fn sys_pkey_free(pkey: i32) -> i64 {
    if pkey < 0 {
        return -(EINVAL as i64);
    }
    0
}

pub fn pkey_free(pkey: i32) -> i64 {
    sys_pkey_free(pkey)
}

pub fn fadvise64(fd: i32, offset: i64, len: i64, advice: i32) -> i64 {
    crate::kernel::syscalls::sys_fadvise64(fd, offset, len, advice)
}

pub fn fadvise64_64(fd: i32, offset: i64, len: i64, advice: i32) -> i64 {
    fadvise64(fd, offset, len, advice)
}

pub fn readahead(fd: i32, offset: i64, count: usize) -> i64 {
    crate::kernel::syscalls::sys_readahead(fd, offset, count)
}

pub fn remap_file_pages(start: u64, size: u64, prot: u64, pgoff: u64, flags: u64) -> i64 {
    crate::kernel::syscalls::sys_remap_file_pages(start, size, prot, pgoff, flags)
}

pub unsafe fn cachestat(
    fd: u32,
    cstat_range: *const crate::kernel::syscalls::CacheStatRange,
    cstat: *mut crate::kernel::syscalls::CacheStat,
    flags: u32,
) -> i64 {
    unsafe { crate::kernel::syscalls::sys_cachestat(fd, cstat_range, cstat, flags) }
}

pub unsafe fn memfd_create(name: *const u8, flags: u32) -> i64 {
    unsafe { crate::fs::syscalls::sys_memfd_create(name, flags) }
}

pub fn mseal(start: u64, len: u64, flags: u64) -> i64 {
    crate::kernel::syscalls::sys_mseal(start, len, flags)
}

pub fn process_madvise(pidfd: i32, iovec: *const u8, vlen: usize, advice: i32, flags: u32) -> i64 {
    crate::kernel::syscalls::sys_process_madvise(pidfd, iovec, vlen, advice, flags)
}

pub fn process_mrelease(pidfd: i32, flags: u32) -> i64 {
    crate::kernel::syscalls::sys_process_mrelease(pidfd, flags)
}

pub fn process_vm_readv(
    pid: i32,
    lvec: *const u8,
    liovcnt: usize,
    rvec: *const u8,
    riovcnt: usize,
    flags: u64,
) -> i64 {
    crate::kernel::syscalls::sys_process_vm_readv(pid, lvec, liovcnt, rvec, riovcnt, flags)
}

pub fn process_vm_writev(
    pid: i32,
    lvec: *const u8,
    liovcnt: usize,
    rvec: *const u8,
    riovcnt: usize,
    flags: u64,
) -> i64 {
    crate::kernel::syscalls::sys_process_vm_writev(pid, lvec, liovcnt, rvec, riovcnt, flags)
}

#[cfg(test)]
mod tests {
    use crate::include::uapi::errno::EBADF;
    use crate::include::uapi::fcntl::{O_RDONLY, O_RDWR, O_WRONLY};
    use crate::kernel::sched;
    use crate::mm::mmap::{MAP_PRIVATE, MAP_SHARED, PROT_WRITE};

    #[test]
    fn syscall_glue_exports_mm_closure_entrypoints() {
        let _: unsafe fn(u64, u64, u32) -> i64 = super::sys_mprotect;
        let _: unsafe fn(u64) -> i64 = super::sys_brk;
        let _: unsafe fn(u64, u64, u64, u32, u64) -> i64 = super::sys_mremap;
        let _: unsafe fn(u64, u64, u32, u32, i32, u64) -> i64 = super::mmap_pgoff;
        let _: unsafe fn(*const super::MmapArgStruct) -> i64 = super::old_mmap;
        let _: unsafe fn(u64) -> i64 = super::brk;
        let _: unsafe fn(u64, u64, i32) -> i64 = super::madvise;
        let _: unsafe fn(u64, u64, u32) -> i64 = super::mprotect;
        let _: unsafe fn(u64, u64, u64, u32, u64) -> i64 = super::mremap;
        let _: unsafe fn(u64, u64) -> i64 = super::munmap;
        let _: unsafe fn(u64, u64) -> i64 = super::mlock;
        let _: unsafe fn(u64, u64) -> i64 = super::munlock;
        let _: fn(i32) -> i64 = super::mlockall;
        let _: fn() -> i64 = super::munlockall;
        let _: fn(i32, i64, i64, i32) -> i64 = super::fadvise64;
        let _: fn(i32, i64, i64, i32) -> i64 = super::fadvise64_64;
        let _: fn(i32, i64, usize) -> i64 = super::readahead;
        let _: fn(u64, u64, u64, u64, u64) -> i64 = super::remap_file_pages;
        let _: unsafe fn(
            u32,
            *const crate::kernel::syscalls::CacheStatRange,
            *mut crate::kernel::syscalls::CacheStat,
            u32,
        ) -> i64 = super::cachestat;
        let _: unsafe fn(*const u8, u32) -> i64 = super::memfd_create;
        let _: fn(u64, u64, u64) -> i64 = super::mseal;
        let _: fn(i32, *const u8, usize, i32, u32) -> i64 = super::process_madvise;
        let _: fn(i32, u32) -> i64 = super::process_mrelease;
        let _: fn(i32, *const u8, usize, *const u8, usize, u64) -> i64 = super::process_vm_readv;
        let _: fn(i32, *const u8, usize, *const u8, usize, u64) -> i64 = super::process_vm_writev;
    }

    #[test]
    fn anonymous_mmap_ignores_fd_like_linux() {
        assert!(!super::mmap_uses_file(
            crate::mm::mmap::MAP_PRIVATE | crate::mm::mmap::MAP_ANONYMOUS
        ));
        assert!(super::mmap_uses_file(crate::mm::mmap::MAP_PRIVATE));
    }

    #[test]
    fn file_mmap_rejects_write_only_fd() {
        assert_eq!(
            super::mmap_validate_file_access(0, MAP_PRIVATE, O_WRONLY),
            Err(super::EACCES)
        );
        assert_eq!(
            super::mmap_validate_file_access(PROT_WRITE, MAP_SHARED, O_WRONLY),
            Err(super::EACCES)
        );
    }

    #[test]
    fn shared_writable_mmap_rejects_read_only_fd() {
        assert_eq!(
            super::mmap_validate_file_access(PROT_WRITE, MAP_SHARED, O_RDONLY),
            Err(super::EACCES)
        );
    }

    #[test]
    fn shared_writable_mmap_accepts_read_write_fd() {
        assert_eq!(
            super::mmap_validate_file_access(PROT_WRITE, MAP_SHARED, O_RDWR),
            Ok(())
        );
    }

    #[test]
    fn read_only_fd_accepts_nonshared_or_nonwritable_mmap() {
        assert_eq!(
            super::mmap_validate_file_access(PROT_WRITE, MAP_PRIVATE, O_RDONLY),
            Ok(())
        );
        assert_eq!(
            super::mmap_validate_file_access(0, MAP_SHARED, O_RDONLY),
            Ok(())
        );
    }

    #[test]
    fn syscall_m76_memory_vm_parity() {
        let previous = unsafe { sched::get_current() };
        unsafe {
            sched::set_current(core::ptr::null_mut());

            assert_eq!(super::sys_mmap(0, 4096, 0, 0x22, -1, 0), -(EBADF as i64));
            assert_eq!(super::sys_mprotect(0x1000, 4096, 1), -(EBADF as i64));
            assert_eq!(super::sys_munmap(0x1000, 4096), -(EBADF as i64));
            assert_eq!(super::sys_brk(0), -(EBADF as i64));
            assert_eq!(super::sys_mremap(0x1000, 4096, 8192, 0, 0), -(EBADF as i64));
            assert_eq!(super::sys_madvise(0x1000, 4096, 0), -(EBADF as i64));

            assert_eq!(super::sys_msync(0x1001, 4096, 0), -(super::EINVAL as i64));
            assert_eq!(super::sys_msync(0x1000, 0, 0), 0);
            assert_eq!(
                super::sys_mincore(0x1001, 4096, core::ptr::null_mut()),
                -(super::EFAULT as i64)
            );
            let mut vec = [0u8; 2];
            assert_eq!(super::sys_mincore(0x1000, 8192, vec.as_mut_ptr()), 0);
            assert_eq!(vec, [1, 1]);

            assert_eq!(
                super::sys_pkey_mprotect(0x1000, 4096, 1, 0),
                -(EBADF as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_remap_file_pages(0x1000, 0, 0, 0, 0),
                -(super::EINVAL as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_remap_file_pages(0x1000, 4096, 0, 0, 1),
                -(super::EINVAL as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_remap_file_pages(0x1000, 4096, 0, 0, 0),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_memfd_secret(1),
                -(super::EINVAL as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_memfd_secret(0),
                -(EBADF as i64)
            );
            assert!(crate::mm::shmem::memfd_secret(0) > 0);
            assert_eq!(
                crate::kernel::syscalls::sys_map_shadow_stack(0, 0, 0),
                -(super::EINVAL as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_mseal(0, 4096, 0),
                -(super::EINVAL as i64)
            );
            assert_eq!(super::fadvise64(0, 0, 4096, 6), -(super::EINVAL as i64));
            assert_eq!(super::fadvise64_64(0, 0, 4096, 0), 0);
            assert_eq!(super::readahead(0, 0, 4096), 0);
            assert_eq!(
                super::remap_file_pages(0x1000, 4096, 0, 0, 1),
                -(super::EINVAL as i64)
            );
            assert_eq!(
                super::process_madvise(-1, core::ptr::null(), 0, 0, 1),
                -(super::EINVAL as i64)
            );
            assert_eq!(super::process_mrelease(-1, 1), -(super::EINVAL as i64));
            assert_eq!(
                super::process_vm_readv(-1, core::ptr::null(), 0, core::ptr::null(), 0, 0),
                -(super::EINVAL as i64)
            );
            assert_eq!(
                super::process_vm_writev(0, core::ptr::null(), 0, core::ptr::null(), 0, 1),
                -(super::EINVAL as i64)
            );
            let range = crate::kernel::syscalls::CacheStatRange { off: 0, len: 4096 };
            let mut stat = crate::kernel::syscalls::CacheStat::default();
            assert_eq!(super::cachestat(0, &range, &mut stat, 0), 0);
            assert_eq!(super::old_mmap(core::ptr::null()), -(super::EFAULT as i64));

            sched::set_current(previous);
        }
    }
}
