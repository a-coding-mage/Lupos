//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry
//! linux-source: vendor/linux/arch/x86/entry/syscall_64.c
//! test-origin: linux:vendor/linux/arch/x86/entry
//! Syscall entry-point wrappers — convert PtRegs to C-ABI args, then call the
//! existing Rust implementations.
//!
//! 370 wrappers, one per implemented syscall, all wired into `SYS_CALL_TABLE`
//! (verified 1:1). Remaining work vs Linux for `complete`: wrappers for the
//! long tail of syscalls Lupos does not yet implement (their slots route to
//! `sys_ni`), and the x32 ABI (`__x32_compat_sys_*`) wrappers.
//!
//! Only wraps the syscalls that the legacy `syscall_dispatch` match currently
//! routes to a real implementation.  Every other slot in `SYS_CALL_TABLE`
//! remains `sys_ni_syscall` (-ENOSYS), exactly mirroring today's behavior.
//!
//! Argument mapping follows the x86-64 Linux syscall ABI:
//!   rdi = a0, rsi = a1, rdx = a2, r10 = a3, r8 = a4, r9 = a5
//!
//! Ref: vendor/linux/arch/x86/entry/syscall_64.c::__SYSCALL

use crate::arch::x86::kernel::ptrace::PtRegs;
use crate::fs::read_write;
use crate::fs::{fdtable, ioctl, mount, openat};
use crate::kernel::{clone, exec, session, signal};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct SchedParam {
    sched_priority: i32,
}

fn errno_result<T>(result: Result<T, i32>, ok: impl FnOnce(T) -> i64) -> i64 {
    match result {
        Ok(value) => ok(value),
        Err(errno) => -(errno as i64),
    }
}

unsafe fn task_for_pid(pid: i32) -> *mut crate::kernel::task::TaskStruct {
    if pid == 0 {
        return unsafe { crate::kernel::sched::get_current() };
    }
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    let pool = crate::kernel::sched::find_pool_task_by_pid(pid);
    if !pool.is_null() {
        return pool;
    }
    let mut by_tgid: *mut crate::kernel::task::TaskStruct = core::ptr::null_mut();
    crate::kernel::fork::for_each_heap_task(|task| unsafe {
        if by_tgid.is_null() && !task.is_null() && (*task).tgid == pid {
            by_tgid = task;
        }
    });
    if !by_tgid.is_null() {
        return by_tgid;
    }
    crate::kernel::sched::find_pool_task_by_tgid(pid)
}

unsafe fn read_user_value<T: Copy>(ptr: *const T) -> Result<T, i32> {
    if ptr.is_null() {
        return Err(crate::include::uapi::errno::EFAULT);
    }
    let mut value = core::mem::MaybeUninit::<T>::uninit();
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            value.as_mut_ptr() as *mut u8,
            ptr as *const u8,
            core::mem::size_of::<T>(),
        )
    };
    if not_copied != 0 {
        return Err(crate::include::uapi::errno::EFAULT);
    }
    Ok(unsafe { value.assume_init() })
}

unsafe fn write_user_value<T>(ptr: *mut T, value: &T) -> Result<(), i32> {
    if ptr.is_null() {
        return Err(crate::include::uapi::errno::EFAULT);
    }
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(
            ptr as *mut u8,
            value as *const T as *const u8,
            core::mem::size_of::<T>(),
        )
    };
    if not_copied != 0 {
        return Err(crate::include::uapi::errno::EFAULT);
    }
    Ok(())
}

unsafe fn read_timespec(ptr: u64) -> Result<crate::kernel::time::Timespec64, i32> {
    if ptr == 0 {
        return Err(crate::kernel::time::posix_clock::EINVAL);
    }
    let mut ts = crate::kernel::time::Timespec64::default();
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            &mut ts as *mut crate::kernel::time::Timespec64 as *mut u8,
            ptr as *const u8,
            core::mem::size_of::<crate::kernel::time::Timespec64>(),
        )
    };
    if not_copied != 0 {
        return Err(crate::include::uapi::errno::EFAULT);
    }
    if ts.is_valid() {
        Ok(ts)
    } else {
        Err(crate::kernel::time::posix_clock::EINVAL)
    }
}

fn futex_absolute_deadline(
    ts: crate::kernel::time::Timespec64,
    clockid: i32,
) -> Result<crate::kernel::futex::FutexDeadline, i32> {
    let deadline = match clockid {
        crate::kernel::time::CLOCK_MONOTONIC => {
            crate::kernel::futex::FutexDeadline::monotonic(ts.to_ns())
        }
        crate::kernel::time::CLOCK_REALTIME => {
            crate::kernel::futex::FutexDeadline::realtime(ts.to_ns())
        }
        _ => return Err(crate::kernel::time::posix_clock::EINVAL),
    };
    Ok(deadline)
}

fn futex_relative_deadline(
    ts: crate::kernel::time::Timespec64,
) -> crate::kernel::futex::FutexDeadline {
    crate::kernel::futex::FutexDeadline::monotonic(
        crate::kernel::time::ktime_get().saturating_add(ts.to_ns()),
    )
}

// The PI/requeue timeout paths still expose their historical relative-duration
// core API.  Keep their conversion separate from the absolute wait API so the
// FUTEX_WAIT/FUTEX_WAIT_BITSET/FUTEX_WAITV fix cannot silently change them.
fn futex_remaining_duration_ns(
    ts: crate::kernel::time::Timespec64,
    clockid: i32,
) -> Result<u64, i32> {
    let now = match clockid {
        crate::kernel::time::CLOCK_MONOTONIC => crate::kernel::time::ktime_get(),
        crate::kernel::time::CLOCK_REALTIME => crate::kernel::time::ktime_get_real(),
        _ => return Err(crate::kernel::time::posix_clock::EINVAL),
    };
    Ok(ts.to_ns().saturating_sub(now))
}

fn legacy_timed_futex_restart_result(result: i64, has_timeout: bool) -> i64 {
    const ERESTARTSYS: i64 = 512;
    const ERESTART_RESTARTBLOCK: i64 = 516;

    if has_timeout && result == -ERESTARTSYS {
        // Linux installs futex_wait_restart and returns this code.  Lupos has
        // the signal-side restart-syscall routing but no materialized per-task
        // restart_block (sys_restart_syscall currently returns EINTR).  Using
        // the existing routing preserves Linux's handler-visible EINTR and,
        // critically, never restarts a relative wait with its full duration.
        return -ERESTART_RESTARTBLOCK;
    }
    result
}

// ── Task — clone / fork / execve ────────────────────────────────────────────

// ── fs/read_write (rootfs bring-up) ─────────────────────────────────────────

/// `write(fd, buf, count)` — Linux syscall 1.
pub unsafe extern "C" fn __x64_sys_write(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { read_write::sys_write(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as usize) }
}

pub unsafe extern "C" fn __x64_sys_read(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { read_write::sys_read(r.arg0() as i32, r.arg1() as *mut u8, r.arg2() as usize) }
}

pub unsafe extern "C" fn __x64_sys_ioctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { ioctl::sys_ioctl(r.arg0() as i32, r.arg1() as u32, r.arg2()) }
}

pub unsafe extern "C" fn __x64_sys_open(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_open(r.arg0() as *const u8, r.arg1() as i32, r.arg2() as u32)
    }
}

/// `close(fd)` — Linux syscall 3.
pub unsafe extern "C" fn __x64_sys_close(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { fdtable::sys_close(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_fstat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fstat(
            r.arg0() as i32,
            r.arg1() as *mut crate::fs::syscalls::LinuxStat,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_stat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_stat(
            r.arg0() as *const u8,
            r.arg1() as *mut crate::fs::syscalls::LinuxStat,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_lstat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_lstat(
            r.arg0() as *const u8,
            r.arg1() as *mut crate::fs::syscalls::LinuxStat,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_newfstatat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_newfstatat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *mut crate::fs::syscalls::LinuxStat,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_statx(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_statx(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as i32,
            r.arg3() as u32,
            r.arg4() as *mut crate::fs::syscalls::LinuxStatx,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_poll(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_poll(
            r.arg0() as *mut crate::fs::syscalls::PollFd,
            r.arg1() as usize,
            r.arg2() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_select(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_select(
            r.arg0() as i32,
            r.arg1() as *mut u64,
            r.arg2() as *mut u64,
            r.arg3() as *mut u64,
            r.arg4() as *mut crate::kernel::syscalls::TimeVal,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_lseek(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_lseek(r.arg0() as i32, r.arg1() as i64, r.arg2() as i32) }
}

pub unsafe extern "C" fn __x64_sys_mmap(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::mm::syscalls::sys_mmap(
            r.arg0(),
            r.arg1(),
            r.arg2() as u32,
            r.arg3() as u32,
            r.arg4() as i32,
            r.arg5(),
        )
    }
}

pub unsafe extern "C" fn __x64_sys_mprotect(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_mprotect(r.arg0(), r.arg1(), r.arg2() as u32) }
}

pub unsafe extern "C" fn __x64_sys_brk(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_brk(r.arg0()) }
}

pub unsafe extern "C" fn __x64_sys_mremap(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::mm::syscalls::sys_mremap(r.arg0(), r.arg1(), r.arg2(), r.arg3() as u32, r.arg4())
    }
}

pub unsafe extern "C" fn __x64_sys_msync(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_msync(r.arg0(), r.arg1(), r.arg2() as i32) }
}

pub unsafe extern "C" fn __x64_sys_madvise(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_madvise(r.arg0(), r.arg1(), r.arg2() as i32) }
}

pub unsafe extern "C" fn __x64_sys_mincore(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_mincore(r.arg0(), r.arg1(), r.arg2() as *mut u8) }
}

pub unsafe extern "C" fn __x64_sys_mlock(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_mlock(r.arg0(), r.arg1()) }
}

pub unsafe extern "C" fn __x64_sys_munlock(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_munlock(r.arg0(), r.arg1()) }
}

pub unsafe extern "C" fn __x64_sys_mlockall(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::mm::syscalls::sys_mlockall(r.arg0() as i32)
}

pub unsafe extern "C" fn __x64_sys_munlockall(_regs: *mut PtRegs) -> i64 {
    crate::mm::syscalls::sys_munlockall()
}

pub unsafe extern "C" fn __x64_sys_mlock2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_mlock2(r.arg0(), r.arg1(), r.arg2() as i32) }
}

pub unsafe extern "C" fn __x64_sys_munmap(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::mm::syscalls::sys_munmap(r.arg0(), r.arg1()) }
}

pub unsafe extern "C" fn __x64_sys_pread64(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_pread64(
            r.arg0() as i32,
            r.arg1() as *mut u8,
            r.arg2() as usize,
            r.arg3() as i64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_pwrite64(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_pwrite64(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as usize,
            r.arg3() as i64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_readv(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_readv(
            r.arg0() as i32,
            r.arg1() as *const crate::fs::syscalls::IoVec,
            r.arg2() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_writev(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_writev(
            r.arg0() as i32,
            r.arg1() as *const crate::fs::syscalls::IoVec,
            r.arg2() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_fdatasync(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fdatasync(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_fsync(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fsync(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_sync(_regs: *mut PtRegs) -> i64 {
    crate::fs::syscalls::sys_sync()
}

pub unsafe extern "C" fn __x64_sys_syncfs(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_syncfs(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_sync_file_range(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_sync_file_range(
            r.arg0() as i32,
            r.arg1() as i64,
            r.arg2() as i64,
            r.arg3() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_sendfile(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_sendfile(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as *mut i64,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_shmget(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_shmget(r.arg0() as i32, r.arg1() as usize, r.arg2() as i32)
}

pub unsafe extern "C" fn __x64_sys_shmat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_shmat(r.arg0() as i32, r.arg1(), r.arg2() as i32)
}

pub unsafe extern "C" fn __x64_sys_shmctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_shmctl(r.arg0() as i32, r.arg1() as i32, r.arg2() as *mut u8)
}

pub unsafe extern "C" fn __x64_sys_semget(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_semget(r.arg0() as i32, r.arg1() as i32, r.arg2() as i32)
}

pub unsafe extern "C" fn __x64_sys_semop(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_semop(r.arg0() as i32, r.arg1() as *mut u8, r.arg2() as usize)
}

pub unsafe extern "C" fn __x64_sys_semctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_semctl(r.arg0() as i32, r.arg1() as i32, r.arg2() as i32, r.arg3())
}

pub unsafe extern "C" fn __x64_sys_shmdt(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_shmdt(r.arg0())
}

pub unsafe extern "C" fn __x64_sys_msgget(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_msgget(r.arg0() as i32, r.arg1() as i32)
}

pub unsafe extern "C" fn __x64_sys_msgsnd(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_msgsnd(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as usize,
        r.arg3() as i32,
    )
}

pub unsafe extern "C" fn __x64_sys_msgrcv(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_msgrcv(
        r.arg0() as i32,
        r.arg1() as *mut u8,
        r.arg2() as usize,
        r.arg3() as i64,
        r.arg4() as i32,
    )
}

pub unsafe extern "C" fn __x64_sys_msgctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_msgctl(r.arg0() as i32, r.arg1() as i32, r.arg2() as *mut u8)
}

pub unsafe extern "C" fn __x64_sys_ftruncate(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_ftruncate(r.arg0() as i32, r.arg1() as i64) }
}

pub unsafe extern "C" fn __x64_sys_truncate(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_truncate(r.arg0() as *const u8, r.arg1() as i64) }
}

pub unsafe extern "C" fn __x64_sys_chmod(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_chmod(r.arg0() as *const u8, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_fchmod(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fchmod(r.arg0() as i32, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_chown(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_chown(r.arg0() as *const u8, r.arg1() as u32, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_fchown(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fchown(r.arg0() as i32, r.arg1() as u32, r.arg2() as u32) }
}

pub unsafe extern "C" fn __x64_sys_lchown(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_lchown(r.arg0() as *const u8, r.arg1() as u32, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_statfs(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_statfs(
            r.arg0() as *const u8,
            r.arg1() as *mut crate::fs::syscalls::LinuxStatFs,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_fstatfs(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fstatfs(
            r.arg0() as i32,
            r.arg1() as *mut crate::fs::syscalls::LinuxStatFs,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_getdents64(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_getdents64(r.arg0() as i32, r.arg1() as *mut u8, r.arg2() as usize)
    }
}

pub unsafe extern "C" fn __x64_sys_getdents(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_getdents(r.arg0() as i32, r.arg1() as *mut u8, r.arg2() as usize)
    }
}

pub unsafe extern "C" fn __x64_sys_getcwd(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_getcwd(r.arg0() as *mut u8, r.arg1() as usize) }
}

pub unsafe extern "C" fn __x64_sys_chdir(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_chdir(r.arg0() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_fchdir(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fchdir(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_rename(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_rename(r.arg0() as *const u8, r.arg1() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_link(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_link(r.arg0() as *const u8, r.arg1() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_symlink(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_symlink(r.arg0() as *const u8, r.arg1() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_readlink(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_readlink(
            r.arg0() as *const u8,
            r.arg1() as *mut u8,
            r.arg2() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_mknod(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_mknod(r.arg0() as *const u8, r.arg1() as u32, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_ustat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_ustat(r.arg0() as u32, r.arg1() as *mut u8)
}

pub unsafe extern "C" fn __x64_sys_pivot_root(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_pivot_root(r.arg0() as *const u8, r.arg1() as *const u8)
}

pub unsafe extern "C" fn __x64_sys_chroot(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_chroot(r.arg0() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_umount2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_umount2(r.arg0() as *const u8, r.arg1() as i32)
}

pub unsafe extern "C" fn __x64_sys_inotify_init(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::fs::syscalls::sys_inotify_init() }
}

pub unsafe extern "C" fn __x64_sys_creat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_creat(r.arg0() as *const u8, r.arg1() as u32) }
}

/// `openat(dirfd, filename, flags, mode)` — Linux syscall 257.
pub unsafe extern "C" fn __x64_sys_openat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        openat::sys_openat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as i32,
            r.arg3() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_dup3(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_dup3(r.arg0() as i32, r.arg1() as i32, r.arg2() as i32) }
}

pub unsafe extern "C" fn __x64_sys_dup(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_dup(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_dup2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_dup2(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_fcntl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fcntl(r.arg0() as i32, r.arg1() as i32, r.arg2()) }
}

pub unsafe extern "C" fn __x64_sys_flock(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_flock(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_close_range(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_close_range(r.arg0() as u32, r.arg1() as u32, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_fallocate(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fallocate(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as i64,
            r.arg3() as i64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_unlinkat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_unlinkat(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as i32)
    }
}

pub unsafe extern "C" fn __x64_sys_unlink(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_unlink(r.arg0() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_rmdir(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_rmdir(r.arg0() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_mkdir(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_mkdir(r.arg0() as *const u8, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_mkdirat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_mkdirat(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_openat2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_openat2(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *const crate::include::uapi::openat2::OpenHow,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_faccessat2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_faccessat2(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as i32,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_access(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_access(r.arg0() as *const u8, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_faccessat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_faccessat(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as i32)
    }
}

pub unsafe extern "C" fn __x64_sys_fchownat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fchownat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as u32,
            r.arg3() as u32,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_fchmodat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fchmodat(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_fchmodat2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fchmodat2(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as u32,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_setxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_setxattr(
            r.arg0() as *const u8,
            r.arg1() as *const u8,
            r.arg2() as *const u8,
            r.arg3() as usize,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_mknodat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_mknodat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as u32,
            r.arg3() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_renameat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_renameat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as i32,
            r.arg3() as *const u8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_linkat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_linkat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as i32,
            r.arg3() as *const u8,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_symlinkat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_symlinkat(
            r.arg0() as *const u8,
            r.arg1() as i32,
            r.arg2() as *const u8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_readlinkat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_readlinkat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *mut u8,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_splice(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_splice(
        r.arg0() as i32,
        r.arg1() as *mut i64,
        r.arg2() as i32,
        r.arg3() as *mut i64,
        r.arg4() as usize,
        r.arg5() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_tee(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_tee(
        r.arg0() as i32,
        r.arg1() as i32,
        r.arg2() as usize,
        r.arg3() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_vmsplice(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_vmsplice(
        r.arg0() as i32,
        r.arg1() as *const crate::fs::syscalls::IoVec,
        r.arg2() as usize,
        r.arg3() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_renameat2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_renameat2(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as i32,
            r.arg3() as *const u8,
            r.arg4() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_memfd_create(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_memfd_create(r.arg0() as *const u8, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_userfaultfd(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_userfaultfd(r.arg0() as i32)
}

pub unsafe extern "C" fn __x64_sys_copy_file_range(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_copy_file_range(
        r.arg0() as i32,
        r.arg1() as *mut i64,
        r.arg2() as i32,
        r.arg3() as *mut i64,
        r.arg4() as usize,
        r.arg5() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_lsetxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_lsetxattr(
            r.arg0() as *const u8,
            r.arg1() as *const u8,
            r.arg2() as *const u8,
            r.arg3() as usize,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_fsetxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fsetxattr(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *const u8,
            r.arg3() as usize,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_getxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_getxattr(
            r.arg0() as *const u8,
            r.arg1() as *const u8,
            r.arg2() as *mut u8,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_lgetxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_lgetxattr(
            r.arg0() as *const u8,
            r.arg1() as *const u8,
            r.arg2() as *mut u8,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_fgetxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fgetxattr(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *mut u8,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_listxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_listxattr(
            r.arg0() as *const u8,
            r.arg1() as *mut u8,
            r.arg2() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_llistxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_llistxattr(
            r.arg0() as *const u8,
            r.arg1() as *mut u8,
            r.arg2() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_flistxattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_flistxattr(r.arg0() as i32, r.arg1() as *mut u8, r.arg2() as usize)
    }
}

pub unsafe extern "C" fn __x64_sys_removexattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_removexattr(r.arg0() as *const u8, r.arg1() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_lremovexattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_lremovexattr(r.arg0() as *const u8, r.arg1() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_fremovexattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fremovexattr(r.arg0() as i32, r.arg1() as *const u8) }
}

pub unsafe extern "C" fn __x64_sys_open_tree(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_open_tree(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as u32)
}

pub unsafe extern "C" fn __x64_sys_move_mount(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_move_mount(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as i32,
        r.arg3() as *const u8,
        r.arg4() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_fsopen(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::syscalls::sys_fsopen(r.arg0() as *const u8, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_fsconfig(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_fsconfig(
        r.arg0() as i32,
        r.arg1() as u32,
        r.arg2() as *const u8,
        r.arg3() as *const u8,
        r.arg4() as i32,
    )
}

pub unsafe extern "C" fn __x64_sys_fsmount(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_fsmount(r.arg0() as i32, r.arg1() as u32, r.arg2() as u32)
}

pub unsafe extern "C" fn __x64_sys_fspick(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_fspick(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_mount_setattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_mount_setattr(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as u32,
        r.arg3() as *const u8,
        r.arg4() as usize,
    )
}

pub unsafe extern "C" fn __x64_sys_quotactl_fd(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_quotactl_fd(
        r.arg0() as i32,
        r.arg1() as u32,
        r.arg2() as i32,
        r.arg3() as *mut u8,
    )
}

pub unsafe extern "C" fn __x64_sys_statmount(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_statmount(
        r.arg0() as *const u8,
        r.arg1() as usize,
        r.arg2() as *mut u8,
        r.arg3() as usize,
        r.arg4() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_listmount(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_listmount(
        r.arg0() as *const u8,
        r.arg1() as usize,
        r.arg2() as *mut u64,
        r.arg3() as usize,
        r.arg4() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_setxattrat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_setxattrat(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as u32,
        r.arg3() as *const u8,
        r.arg4() as *const u8,
        r.arg5() as usize,
        0,
    )
}

pub unsafe extern "C" fn __x64_sys_getxattrat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_getxattrat(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as u32,
        r.arg3() as *const u8,
        r.arg4() as *mut u8,
        r.arg5() as usize,
    )
}

pub unsafe extern "C" fn __x64_sys_listxattrat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_listxattrat(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as u32,
        r.arg3() as *mut u8,
        r.arg4() as usize,
    )
}

pub unsafe extern "C" fn __x64_sys_removexattrat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_removexattrat(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as u32,
        r.arg3() as *const u8,
    )
}

pub unsafe extern "C" fn __x64_sys_open_tree_attr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_open_tree_attr(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as u32,
        r.arg3() as *const u8,
        r.arg4() as usize,
    )
}

pub unsafe extern "C" fn __x64_sys_file_getattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_file_getattr(
        r.arg0() as i32,
        r.arg1() as u32,
        r.arg2() as u32,
        r.arg3() as *mut crate::fs::syscalls::LinuxStatx,
    )
}

pub unsafe extern "C" fn __x64_sys_file_setattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::fs::syscalls::sys_file_setattr(
        r.arg0() as i32,
        r.arg1() as u32,
        r.arg2() as *const u8,
        r.arg3() as usize,
    )
}

pub unsafe extern "C" fn __x64_sys_syslog(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_syslog(r.arg0() as i32, r.arg1() as *mut u8, r.arg2() as i32)
}

pub unsafe extern "C" fn __x64_sys_acct(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_acct(r.arg0() as *const u8)
}

pub unsafe extern "C" fn __x64_sys_io_setup(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_io_setup(r.arg0() as u32, r.arg1() as *mut u64)
}

pub unsafe extern "C" fn __x64_sys_io_destroy(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_io_destroy(r.arg0())
}

pub unsafe extern "C" fn __x64_sys_io_getevents(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_io_getevents(
        r.arg0(),
        r.arg1() as i64,
        r.arg2() as i64,
        r.arg3() as *mut u8,
        r.arg4() as *const crate::kernel::time::Timespec64,
    )
}

pub unsafe extern "C" fn __x64_sys_io_submit(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_io_submit(r.arg0(), r.arg1() as i64, r.arg2() as *mut u8)
}

pub unsafe extern "C" fn __x64_sys_io_cancel(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_io_cancel(r.arg0(), r.arg1() as *mut u8, r.arg2() as *mut u8)
}

pub unsafe extern "C" fn __x64_sys_set_thread_area(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::arch::x86::kernel::tls::sys_set_thread_area(
            r.arg0() as *mut crate::arch::x86::kernel::ldt::UserDesc
        )
    }
}

pub unsafe extern "C" fn __x64_sys_get_thread_area(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::arch::x86::kernel::tls::sys_get_thread_area(
            r.arg0() as *mut crate::arch::x86::kernel::ldt::UserDesc
        )
    }
}

pub unsafe extern "C" fn __x64_sys_mq_open(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mq_open(
        r.arg0() as *const u8,
        r.arg1() as i32,
        r.arg2() as u32,
        r.arg3() as *const u8,
    )
}

pub unsafe extern "C" fn __x64_sys_mq_unlink(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mq_unlink(r.arg0() as *const u8)
}

pub unsafe extern "C" fn __x64_sys_mq_notify(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mq_notify(r.arg0() as i32, r.arg1() as *const u8)
}

pub unsafe extern "C" fn __x64_sys_mq_getsetattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mq_getsetattr(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as *mut u8,
    )
}

pub unsafe extern "C" fn __x64_sys_kexec_load(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_kexec_load(r.arg0(), r.arg1(), r.arg2() as *const u8, r.arg3())
}

pub unsafe extern "C" fn __x64_sys_kexec_file_load(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_kexec_file_load(
        r.arg0() as i32,
        r.arg1() as i32,
        r.arg2(),
        r.arg3() as *const u8,
        r.arg4(),
    )
}

pub unsafe extern "C" fn __x64_sys_io_pgetevents(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_io_pgetevents(
        r.arg0(),
        r.arg1() as i64,
        r.arg2() as i64,
        r.arg3() as *mut u8,
        r.arg4() as *const crate::kernel::time::Timespec64,
        r.arg5() as *const u8,
    )
}

pub unsafe extern "C" fn __x64_sys_pidfd_open(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_pidfd_open(r.arg0() as i32, r.arg1() as u32)
}

pub unsafe extern "C" fn __x64_sys_pidfd_getfd(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_pidfd_getfd(r.arg0() as i32, r.arg1() as i32, r.arg2() as u32)
}

pub unsafe extern "C" fn __x64_sys_lsm_get_self_attr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_lsm_get_self_attr(
        r.arg0() as u32,
        r.arg1() as *mut u8,
        r.arg2() as *mut u32,
        r.arg3() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_lsm_set_self_attr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_lsm_set_self_attr(
        r.arg0() as u32,
        r.arg1() as *const u8,
        r.arg2() as u32,
        r.arg3() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_lsm_list_modules(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_lsm_list_modules(
        r.arg0() as *mut u64,
        r.arg1() as *mut u32,
        r.arg2() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_pipe(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::pipe::sys_pipe(r.arg0() as *mut i32) }
}

pub unsafe extern "C" fn __x64_sys_pipe2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::pipe::sys_pipe2(r.arg0() as *mut i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_preadv(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_preadv(
            r.arg0() as i32,
            r.arg1() as *const crate::fs::syscalls::IoVec,
            r.arg2() as usize,
            r.arg3() as i64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_pwritev(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_pwritev(
            r.arg0() as i32,
            r.arg1() as *const crate::fs::syscalls::IoVec,
            r.arg2() as usize,
            r.arg3() as i64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_preadv2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_preadv2(
            r.arg0() as i32,
            r.arg1() as *const crate::fs::syscalls::IoVec,
            r.arg2() as usize,
            r.arg3() as i64,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_pwritev2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_pwritev2(
            r.arg0() as i32,
            r.arg1() as *const crate::fs::syscalls::IoVec,
            r.arg2() as usize,
            r.arg3() as i64,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_membarrier(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::mm::syscalls::sys_membarrier(r.arg0() as i32, r.arg1() as u32, r.arg2() as i32)
}

pub unsafe extern "C" fn __x64_sys_pkey_mprotect(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::mm::syscalls::sys_pkey_mprotect(r.arg0(), r.arg1(), r.arg2() as u32, r.arg3() as i32)
    }
}

pub unsafe extern "C" fn __x64_sys_pkey_alloc(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::mm::syscalls::sys_pkey_alloc(r.arg0() as u32, r.arg1() as u32)
}

pub unsafe extern "C" fn __x64_sys_pkey_free(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::mm::syscalls::sys_pkey_free(r.arg0() as i32)
}

pub unsafe extern "C" fn __x64_sys_pselect6(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_pselect6(
            r.arg0() as i32,
            r.arg1() as *mut u64,
            r.arg2() as *mut u64,
            r.arg3() as *mut u64,
            r.arg4() as *const crate::kernel::time::Timespec64,
            r.arg5() as *const u8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_ppoll(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::syscalls::sys_ppoll(
            r.arg0() as *mut crate::fs::syscalls::PollFd,
            r.arg1() as usize,
            r.arg2() as *const crate::kernel::time::Timespec64,
            r.arg3() as *const u8,
            r.arg4() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_socket(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::net::syscalls::sys_socket(r.arg0() as i32, r.arg1() as i32, r.arg2() as i32) }
}

pub unsafe extern "C" fn __x64_sys_connect(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_connect(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_accept4(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_accept4(
            r.arg0() as i32,
            r.arg1() as *mut u8,
            r.arg2() as *mut u32,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_accept(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_accept(r.arg0() as i32, r.arg1() as *mut u8, r.arg2() as *mut u32)
    }
}

pub unsafe extern "C" fn __x64_sys_socketpair(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_socketpair(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as i32,
            r.arg3() as *mut i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_sendto(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_sendto(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as usize,
            r.arg3() as i32,
            r.arg4() as *const u8,
            r.arg5() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_recvfrom(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_recvfrom(
            r.arg0() as i32,
            r.arg1() as *mut u8,
            r.arg2() as usize,
            r.arg3() as i32,
            r.arg4() as *mut u8,
            r.arg5() as *mut u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_sendmsg(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_sendmsg(
            r.arg0() as i32,
            r.arg1() as *const crate::net::syscalls::LinuxMsghdr,
            r.arg2() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_recvmsg(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_recvmsg(
            r.arg0() as i32,
            r.arg1() as *mut crate::net::syscalls::LinuxMsghdr,
            r.arg2() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_recvmmsg(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_recvmmsg(
            r.arg0() as i32,
            r.arg1() as *mut crate::net::syscalls::LinuxMmsghdr,
            r.arg2() as u32,
            r.arg3() as i32,
            r.arg4() as *mut crate::kernel::time::Timespec64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_sendmmsg(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_sendmmsg(
            r.arg0() as i32,
            r.arg1() as *mut crate::net::syscalls::LinuxMmsghdr,
            r.arg2() as u32,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_shutdown(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::net::syscalls::sys_shutdown(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_bind(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_bind(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_listen(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::net::syscalls::sys_listen(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_getsockname(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_getsockname(
            r.arg0() as i32,
            r.arg1() as *mut u8,
            r.arg2() as *mut u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_getpeername(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_getpeername(
            r.arg0() as i32,
            r.arg1() as *mut u8,
            r.arg2() as *mut u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_setsockopt(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_setsockopt(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as i32,
            r.arg3() as *const u8,
            r.arg4() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_getsockopt(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::net::syscalls::sys_getsockopt(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as i32,
            r.arg3() as *mut u8,
            r.arg4() as *mut u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_init_module(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::module::syscalls::sys_init_module(
            r.arg0() as *const u8,
            r.arg1() as usize,
            r.arg2() as *const u8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_delete_module(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::module::syscalls::sys_delete_module(r.arg0() as *const u8, r.arg1() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_iopl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_iopl(r.arg0() as u32) }
}

pub unsafe extern "C" fn __x64_sys_ioperm(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_ioperm(r.arg0(), r.arg1(), r.arg2() as i32) }
}

pub unsafe extern "C" fn __x64_sys_ioprio_set(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_ioprio_set(r.arg0() as i32, r.arg1() as i32, r.arg2() as i32)
    }
}

pub unsafe extern "C" fn __x64_sys_ioprio_get(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_ioprio_get(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_finit_module(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::module::syscalls::sys_finit_module(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_setpgid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { session::sys_setpgid(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_setsid(_regs: *mut PtRegs) -> i64 {
    unsafe { session::sys_setsid() }
}

pub unsafe extern "C" fn __x64_sys_mount(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        mount::sys_mount(
            r.arg0() as *const u8,
            r.arg1() as *const u8,
            r.arg2() as *const u8,
            r.arg3(),
            r.arg4() as *const u8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_sethostname(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::utsname::sys_sethostname(r.arg0() as *const u8, r.arg1() as usize) }
}

pub unsafe extern "C" fn __x64_sys_setdomainname(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::utsname::sys_setdomainname(r.arg0() as *const u8, r.arg1() as usize) }
}

pub unsafe extern "C" fn __x64_sys_fork(regs: *mut PtRegs) -> i64 {
    let r = unsafe { *regs };
    unsafe { clone::sys_fork_with_regs(Some(r)) }
}

pub unsafe extern "C" fn __x64_sys_vfork(regs: *mut PtRegs) -> i64 {
    let r = unsafe { *regs };
    unsafe { clone::sys_vfork_with_regs(Some(r)) }
}

pub unsafe extern "C" fn __x64_sys_clone(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        clone::sys_clone_with_regs(
            r.arg0(),
            r.arg1(),
            r.arg2() as *mut i32,
            r.arg3() as *mut i32,
            r.arg4(),
            Some(*r),
        )
    }
}

pub unsafe extern "C" fn __x64_sys_clone3(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        clone::sys_clone3_with_regs(
            r.arg0() as *const crate::kernel::clone::CloneArgs,
            r.arg1() as usize,
            Some(*r),
        )
    }
}

pub unsafe extern "C" fn __x64_sys_execve(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        exec::sys_execve(
            r.arg0() as *const i8,
            r.arg1() as *const *const i8,
            r.arg2() as *const *const i8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_execveat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        exec::sys_execveat(
            r.arg0() as i32,
            r.arg1() as *const i8,
            r.arg2() as *const *const i8,
            r.arg3() as *const *const i8,
            r.arg4() as i32,
        )
    }
}

// ── Signals ─────────────────────────────────────────────────────────────────

pub unsafe extern "C" fn __x64_sys_pause(_regs: *mut PtRegs) -> i64 {
    crate::kernel::syscalls::sys_pause()
}

pub unsafe extern "C" fn __x64_sys_restart_syscall(_regs: *mut PtRegs) -> i64 {
    crate::kernel::syscalls::sys_restart_syscall()
}

pub unsafe extern "C" fn __x64_sys_rt_sigaction(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        signal::sys_rt_sigaction(
            r.arg0() as i32,
            r.arg1() as *const signal::RtSigAction,
            r.arg2() as *mut signal::RtSigAction,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_rt_sigprocmask(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        signal::sys_rt_sigprocmask(
            r.arg0() as i32,
            r.arg1() as *const signal::SigSet,
            r.arg2() as *mut signal::SigSet,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_rt_sigreturn(regs: *mut PtRegs) -> i64 {
    unsafe { signal::sys_rt_sigreturn_impl(regs.cast()) }
}

pub unsafe extern "C" fn __x64_sys_rt_sigpending(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { signal::sys_rt_sigpending(r.arg0() as *mut signal::SigSet, r.arg1() as usize) }
}

pub unsafe extern "C" fn __x64_sys_rt_sigtimedwait(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        signal::sys_rt_sigtimedwait(
            r.arg0() as *const signal::SigSet,
            r.arg1() as *mut signal::SigInfo,
            r.arg2() as *const core::ffi::c_void,
            r.arg3() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_rt_sigsuspend(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { signal::sys_rt_sigsuspend(r.arg0() as *const signal::SigSet, r.arg1() as usize) }
}

pub unsafe extern "C" fn __x64_sys_rt_sigqueueinfo(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        signal::sys_rt_sigqueueinfo(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as *const signal::SigInfo,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_sigaltstack(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        signal::sys_sigaltstack(
            r.arg0() as *const signal::SigAltStack,
            r.arg1() as *mut signal::SigAltStack,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_tkill(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { signal::sys_tkill(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_kill(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_kill(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_tgkill(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { signal::sys_tgkill(r.arg0() as i32, r.arg1() as i32, r.arg2() as i32) }
}

// ── Exit / wait / ptrace (M26) ──────────────────────────────────────────────

pub unsafe extern "C" fn __x64_sys_exit(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::wait::sys_exit(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_exit_group(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::wait::sys_exit_group(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_wait4(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::wait::sys_wait4(
            r.arg0() as i32,
            r.arg1() as *mut i32,
            r.arg2() as i32,
            r.arg3() as *mut crate::kernel::wait::Rusage,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_waitid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::wait::sys_waitid(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as *mut crate::kernel::wait::WaitidSigInfo,
            r.arg3() as i32,
            r.arg4() as *mut crate::kernel::wait::Rusage,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_ptrace(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::ptrace::sys_ptrace(r.arg0() as i64, r.arg1() as i32, r.arg2(), r.arg3())
    }
}

pub unsafe extern "C" fn __x64_sys_arch_prctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::arch::x86::kernel::prctl::sys_arch_prctl(r.arg0() as i32, r.arg1()) }
}

pub unsafe extern "C" fn __x64_sys_prctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::seccomp::sys_prctl(r.arg0() as i32, r.arg1(), r.arg2(), r.arg3(), r.arg4())
    }
}

pub unsafe extern "C" fn __x64_sys_seccomp(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::seccomp::sys_seccomp(
            r.arg0() as u32,
            r.arg1(),
            r.arg2() as *const core::ffi::c_void,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_unshare(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::nsproxy::sys_unshare(r.arg0()) }
}

pub unsafe extern "C" fn __x64_sys_setns(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::nsproxy::sys_setns(r.arg0() as i32, r.arg1() as i32) }
}

// ── M60 — eventfd / signalfd / epoll / inotify / fanotify / io_uring ────────

// M66: scheduler, futex, and POSIX time syscalls.

pub unsafe extern "C" fn __x64_sys_sched_yield(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::sched::syscalls::sys_sched_yield() as i64 }
}

pub unsafe extern "C" fn __x64_sys_sched_setparam(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let param = r.arg1() as *const SchedParam;
    if param.is_null() {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let param = match unsafe { read_user_value(param) } {
        Ok(param) => param,
        Err(errno) => return -(errno as i64),
    };
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    let policy = unsafe { (*task).m29.policy };
    unsafe {
        crate::kernel::sched::syscalls::sys_sched_setscheduler(
            task,
            policy,
            param.sched_priority as u32,
        ) as i64
    }
}

pub unsafe extern "C" fn __x64_sys_sched_setscheduler(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let param = r.arg2() as *const SchedParam;
    if param.is_null() {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let param = match unsafe { read_user_value(param) } {
        Ok(param) => param,
        Err(errno) => return -(errno as i64),
    };
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    unsafe {
        crate::kernel::sched::syscalls::sys_sched_setscheduler(
            task,
            r.arg1() as u32,
            param.sched_priority as u32,
        ) as i64
    }
}

pub unsafe extern "C" fn __x64_sys_sched_getparam(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let param = r.arg1() as *mut SchedParam;
    if param.is_null() {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    let out = SchedParam {
        sched_priority: unsafe { (*task).m29.rt_priority as i32 },
    };
    if let Err(errno) = unsafe { write_user_value(param, &out) } {
        return -(errno as i64);
    }
    0
}

pub unsafe extern "C" fn __x64_sys_sched_getscheduler(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    unsafe { crate::kernel::sched::syscalls::sys_sched_getscheduler(task) as i64 }
}

pub unsafe extern "C" fn __x64_sys_sched_get_priority_max(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::sched::syscalls::sys_sched_get_priority_max(r.arg0() as u32) as i64
}

pub unsafe extern "C" fn __x64_sys_sched_get_priority_min(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::sched::syscalls::sys_sched_get_priority_min(r.arg0() as u32) as i64
}

pub unsafe extern "C" fn __x64_sys_sched_rr_get_interval(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let out = r.arg1() as *mut crate::kernel::time::Timespec64;
    if out.is_null() {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    let ns = unsafe { crate::kernel::sched::syscalls::sys_sched_rr_get_interval(task) };
    let interval = crate::kernel::time::Timespec64::from_ns(ns);
    if let Err(errno) = unsafe { write_user_value(out, &interval) } {
        return -(errno as i64);
    }
    0
}

pub unsafe extern "C" fn __x64_sys_sched_setaffinity(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let cpusetsize = r.arg1() as usize;
    let mask_ptr = r.arg2() as *const u64;
    if mask_ptr.is_null() {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    if cpusetsize < core::mem::size_of::<crate::kernel::sched::entity::CpuMask>() {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let mask = match unsafe { crate::arch::x86::kernel::uaccess::get_user_u64(mask_ptr) } {
        Ok(mask) => {
            crate::kernel::sched::entity::CpuMask(mask & crate::kernel::sched::cpu_active_mask().0)
        }
        Err(errno) => return errno as i64,
    };
    // Linux __sched_setaffinity() intersects the user request with the
    // task's cpuset-allowed CPUs; a request containing no active CPU fails.
    if mask.weight() == 0 {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    unsafe {
        (*task).m29.cpus_mask = mask;
        (*task).m29.nr_cpus_allowed = mask.weight() as i32;
        (*task).m29.cpus_ptr = &(*task).m29.cpus_mask as *const _;
    }
    0
}

pub unsafe extern "C" fn __x64_sys_sched_getaffinity(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let cpusetsize = r.arg1() as usize;
    let mask_ptr = r.arg2() as *mut u64;
    let needed = core::mem::size_of::<crate::kernel::sched::entity::CpuMask>();
    // vendor/linux/kernel/sched/syscalls.c::sys_sched_getaffinity requires an
    // unsigned-long-aligned buffer large enough for nr_cpu_ids.
    if cpusetsize < needed || cpusetsize & (core::mem::size_of::<u64>() - 1) != 0 {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    // Linux sched_getaffinity(): cpumask_and(mask, &p->cpus_mask,
    // cpu_active_mask).  Returning the compile-time NR_CPUS mask here made
    // Mesa/llvmpipe create dozens of workers in a one-vCPU guest.
    let mask = unsafe { (*task).m29.cpus_mask.0 } & crate::kernel::sched::cpu_active_mask().0;
    if let Err(errno) = unsafe { crate::arch::x86::kernel::uaccess::put_user_u64(mask_ptr, mask) } {
        return errno as i64;
    }
    needed as i64
}

pub unsafe extern "C" fn __x64_sys_sched_setattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let attr = r.arg1() as *const crate::kernel::sched::syscalls::SchedAttr;
    if attr.is_null() || r.arg2() != 0 {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let attr = match unsafe { read_user_value(attr) } {
        Ok(attr) => attr,
        Err(errno) => return -(errno as i64),
    };
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    unsafe { crate::kernel::sched::syscalls::sys_sched_setattr(task, &attr) as i64 }
}

pub unsafe extern "C" fn __x64_sys_sched_getattr(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let out = r.arg1() as *mut crate::kernel::sched::syscalls::SchedAttr;
    if out.is_null() || r.arg2() < crate::kernel::sched::syscalls::SCHED_ATTR_SIZE_VER0 as u64 {
        return -(crate::kernel::sched::syscalls::EINVAL as i64);
    }
    let task = unsafe { task_for_pid(r.arg0() as i32) };
    if task.is_null() {
        return -(crate::kernel::sched::syscalls::ESRCH as i64);
    }
    let mut attr = crate::kernel::sched::syscalls::SchedAttr::default();
    let ret = unsafe { crate::kernel::sched::syscalls::sys_sched_getattr(task, &mut attr) };
    if ret != 0 {
        return ret as i64;
    }
    if let Err(errno) = unsafe { write_user_value(out, &attr) } {
        return -(errno as i64);
    }
    0
}

pub unsafe extern "C" fn __x64_sys_futex(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let op = r.arg1() as u32;
    let cmd = op & crate::kernel::futex::FUTEX_CMD_MASK;
    let raw_timeout = r.arg3();
    let clockid = if op & crate::kernel::futex::FUTEX_CLOCK_REALTIME != 0 {
        crate::kernel::time::CLOCK_REALTIME
    } else {
        crate::kernel::time::CLOCK_MONOTONIC
    };
    if matches!(
        cmd,
        crate::kernel::futex::FUTEX_WAIT | crate::kernel::futex::FUTEX_WAIT_BITSET
    ) {
        let deadline = if raw_timeout == 0 {
            None
        } else {
            let ts = match unsafe { read_timespec(raw_timeout) } {
                Ok(ts) => ts,
                Err(errno) => return -(errno as i64),
            };
            Some(if cmd == crate::kernel::futex::FUTEX_WAIT {
                futex_relative_deadline(ts)
            } else {
                match futex_absolute_deadline(ts, clockid) {
                    Ok(deadline) => deadline,
                    Err(errno) => return -(errno as i64),
                }
            })
        };
        if cmd == crate::kernel::futex::FUTEX_WAIT
            && op & crate::kernel::futex::FUTEX_CLOCK_REALTIME != 0
        {
            return -(crate::kernel::futex::ENOSYS as i64);
        }
        let bitset = if cmd == crate::kernel::futex::FUTEX_WAIT {
            crate::kernel::futex::FUTEX_BITSET_MATCH_ANY
        } else {
            r.arg5() as u32
        };
        let result = unsafe {
            crate::kernel::futex::futex_wait_deadline(
                r.arg0(),
                r.arg2() as u32,
                bitset,
                deadline,
                op & crate::kernel::futex::FUTEX_PRIVATE_FLAG != 0,
            )
        };
        return legacy_timed_futex_restart_result(result, deadline.is_some());
    }
    let timeout_clockid = if cmd == crate::kernel::futex::FUTEX_LOCK_PI {
        crate::kernel::time::CLOCK_REALTIME
    } else {
        clockid
    };
    let timeout = match cmd {
        crate::kernel::futex::FUTEX_LOCK_PI
        | crate::kernel::futex::FUTEX_LOCK_PI2
        | crate::kernel::futex::FUTEX_WAIT_REQUEUE_PI
            if raw_timeout != 0 =>
        {
            match unsafe { read_timespec(raw_timeout) }
                .and_then(|ts| futex_remaining_duration_ns(ts, timeout_clockid))
            {
                Ok(0) => 1,
                Ok(ns) => ns,
                Err(errno) => return -(errno as i64),
            }
        }
        _ => raw_timeout,
    };
    unsafe {
        crate::kernel::futex::sys_futex(
            r.arg0(),
            op,
            r.arg2() as u32,
            timeout,
            r.arg4(),
            r.arg5() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_set_robust_list(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::futex::robust::sys_set_robust_list(r.arg0(), r.arg1()) }
}

pub unsafe extern "C" fn __x64_sys_get_robust_list(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    if r.arg1() == 0 || r.arg2() == 0 {
        return -(crate::include::uapi::errno::EFAULT as i64);
    }
    let mut head = 0u64;
    let mut len = 0u64;
    let ret = unsafe {
        crate::kernel::futex::robust::sys_get_robust_list_for_pid(
            r.arg0() as i32,
            &mut head,
            &mut len,
        )
    };
    if ret != 0 {
        return ret;
    }
    if unsafe { crate::arch::x86::kernel::uaccess::put_user_u64(r.arg1() as *mut u64, head) }
        .is_err()
    {
        return -(crate::include::uapi::errno::EFAULT as i64);
    }
    if unsafe { crate::arch::x86::kernel::uaccess::put_user_u64(r.arg2() as *mut u64, len) }
        .is_err()
    {
        return -(crate::include::uapi::errno::EFAULT as i64);
    }
    0
}

pub unsafe extern "C" fn __x64_sys_futex_waitv(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let nr = r.arg1() as usize;
    let clockid = r.arg4() as i32;
    if r.arg2() != 0 || r.arg0() == 0 || nr == 0 || nr > crate::kernel::futex::FUTEX_WAITV_MAX {
        return -(crate::kernel::futex::EINVAL as i64);
    }
    let deadline = if r.arg3() == 0 {
        None
    } else {
        match unsafe { read_timespec(r.arg3()) }.and_then(|ts| futex_absolute_deadline(ts, clockid))
        {
            Ok(deadline) => Some(deadline),
            Err(errno) => return -(errno as i64),
        }
    };
    unsafe {
        crate::kernel::futex::sys_futex_waitv_deadline(
            r.arg0(),
            nr,
            r.arg2() as u32,
            deadline,
            clockid,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_futex_wake(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::futex::sys_futex_wake2(r.arg0(), r.arg1(), r.arg2() as i32, r.arg3() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_futex_wait(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let value = r.arg1();
    let mask = r.arg2();
    let flags = r.arg3() as u32;
    if value > u32::MAX as u64 || mask > u32::MAX as u64 {
        return -(crate::kernel::futex::EINVAL as i64);
    }
    if let Err(errno) =
        unsafe { crate::kernel::futex::core_ops::futex2_prepare_key(r.arg0(), flags) }
    {
        return -(errno as i64);
    }
    let deadline = if r.arg4() == 0 {
        None
    } else {
        match unsafe { read_timespec(r.arg4()) }
            .and_then(|ts| futex_absolute_deadline(ts, r.arg5() as i32))
        {
            Ok(deadline) => Some(deadline),
            Err(errno) => return -(errno as i64),
        }
    };
    unsafe {
        crate::kernel::futex::futex_wait_deadline(
            r.arg0(),
            value as u32,
            mask as u32,
            deadline,
            flags & crate::kernel::futex::FUTEX2_PRIVATE != 0,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_futex_requeue(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::futex::futex_requeue(
            r.arg0(),
            r.arg1(),
            r.arg2() as i32,
            r.arg3() as i32,
            0,
            false,
            r.arg4() as u32 & crate::kernel::futex::FUTEX_PRIVATE_FLAG != 0,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_nanosleep(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let remain = r.arg1() as *mut crate::kernel::time::Timespec64;
    let mut remain_value = crate::kernel::time::Timespec64::default();
    let remain_arg = if remain.is_null() {
        None
    } else {
        Some(&raw mut remain_value)
    };
    match unsafe { read_timespec(r.arg0()) } {
        Ok(request) => {
            let ret = errno_result(
                crate::kernel::time::syscalls::sys_nanosleep(request, remain_arg),
                |_| 0,
            );
            // rem is copied back only when the sleep was interrupted; a
            // completed nanosleep leaves it untouched (Linux
            // nanosleep_copyout runs only on the restart path).
            if ret == -(crate::include::uapi::errno::EINTR as i64)
                && !remain.is_null()
                && unsafe { write_user_value(remain, &remain_value) }.is_err()
            {
                return -(crate::include::uapi::errno::EFAULT as i64);
            }
            ret
        }
        Err(errno) => -(errno as i64),
    }
}

pub unsafe extern "C" fn __x64_sys_clock_gettime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let out = r.arg1() as *mut crate::kernel::time::Timespec64;
    if out.is_null() {
        return -(crate::include::uapi::errno::EFAULT as i64);
    }
    errno_result(
        crate::kernel::time::sys_clock_gettime(r.arg0() as i32),
        |ts| match unsafe { write_user_value(out, &ts) } {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
    )
}

pub unsafe extern "C" fn __x64_sys_gettimeofday(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_gettimeofday(
            r.arg0() as *mut crate::kernel::syscalls::TimeVal,
            r.arg1() as *mut crate::kernel::syscalls::TimeZone,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_getitimer(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_getitimer(
            r.arg0() as i32,
            r.arg1() as *mut crate::kernel::syscalls::ITimerVal,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_setitimer(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_setitimer(
            r.arg0() as i32,
            r.arg1() as *const crate::kernel::syscalls::ITimerVal,
            r.arg2() as *mut crate::kernel::syscalls::ITimerVal,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_alarm(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_alarm(r.arg0() as u32)
}

pub unsafe extern "C" fn __x64_sys_utime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_utime(
            r.arg0() as *const u8,
            r.arg1() as *const crate::kernel::syscalls::TimeVal,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_time(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_time(r.arg0() as *mut i64) }
}

pub unsafe extern "C" fn __x64_sys_getrandom(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_getrandom(
            r.arg0() as *mut u8,
            r.arg1() as usize,
            r.arg2() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_umask(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_umask(r.arg0() as u32)
}

pub unsafe extern "C" fn __x64_sys_getrlimit(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_getrlimit(
            r.arg0() as i32,
            r.arg1() as *mut crate::kernel::syscalls::RLimit,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_getrusage(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_getrusage(
            r.arg0() as i32,
            r.arg1() as *mut crate::kernel::syscalls::RUsage,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_getpriority(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_getpriority(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_setpriority(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_setpriority(r.arg0() as i32, r.arg1() as i32, r.arg2() as i32)
    }
}

pub unsafe extern "C" fn __x64_sys_sysfs(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_sysfs(r.arg0() as i32, r.arg1(), r.arg2()) }
}

pub unsafe extern "C" fn __x64_sys_vhangup(_regs: *mut PtRegs) -> i64 {
    crate::kernel::syscalls::sys_vhangup()
}

pub unsafe extern "C" fn __x64_sys_modify_ldt(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_modify_ldt(
            r.arg0() as i32,
            r.arg1() as *mut u8,
            r.arg2() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_adjtimex(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_adjtimex(r.arg0() as *mut crate::kernel::syscalls::Timex)
    }
}

pub unsafe extern "C" fn __x64_sys_settimeofday(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_settimeofday(
            r.arg0() as *const crate::kernel::syscalls::TimeVal,
            r.arg1() as *const crate::kernel::syscalls::TimeZone,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_swapon(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_swapon(r.arg0() as *const u8, r.arg1() as i32)
}

pub unsafe extern "C" fn __x64_sys_swapoff(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_swapoff(r.arg0() as *const u8)
}

pub unsafe extern "C" fn __x64_sys_reboot(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_reboot(
        r.arg0() as i32,
        r.arg1() as i32,
        r.arg2() as u32,
        r.arg3() as *mut u8,
    )
}

pub unsafe extern "C" fn __x64_sys_sysinfo(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_sysinfo(r.arg0() as *mut crate::kernel::syscalls::SysInfo)
    }
}

pub unsafe extern "C" fn __x64_sys_times(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_times(r.arg0() as *mut crate::kernel::syscalls::Tms) }
}

pub unsafe extern "C" fn __x64_sys_personality(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_personality(r.arg0() as u32)
}

pub unsafe extern "C" fn __x64_sys_prlimit64(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_prlimit64(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as *const crate::kernel::syscalls::RLimit,
            r.arg3() as *mut crate::kernel::syscalls::RLimit,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_set_tid_address(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_set_tid_address(r.arg0() as *mut i32) }
}

pub unsafe extern "C" fn __x64_sys_readahead(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_readahead(r.arg0() as i32, r.arg1() as i64, r.arg2() as usize)
}

pub unsafe extern "C" fn __x64_sys_fadvise64(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_fadvise64(
        r.arg0() as i32,
        r.arg1() as i64,
        r.arg2() as i64,
        r.arg3() as i32,
    )
}

pub unsafe extern "C" fn __x64_sys_getcpu(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_getcpu(r.arg0() as *mut u32, r.arg1() as *mut u32) }
}

pub unsafe extern "C" fn __x64_sys_remap_file_pages(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_remap_file_pages(r.arg0(), r.arg1(), r.arg2(), r.arg3(), r.arg4())
}

pub unsafe extern "C" fn __x64_sys_semtimedop(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_semtimedop(
        r.arg0() as i32,
        r.arg1() as *mut u8,
        r.arg2() as usize,
        r.arg3() as *const u8,
    )
}

pub unsafe extern "C" fn __x64_sys_utimes(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_utimes(
            r.arg0() as *const u8,
            r.arg1() as *const crate::kernel::syscalls::TimeVal,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_mbind(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mbind(
        r.arg0(),
        r.arg1(),
        r.arg2(),
        r.arg3() as *const u64,
        r.arg4(),
        r.arg5() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_set_mempolicy(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_set_mempolicy(r.arg0() as i32, r.arg1() as *const u64, r.arg2())
}

pub unsafe extern "C" fn __x64_sys_get_mempolicy(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_get_mempolicy(
            r.arg0() as *mut i32,
            r.arg1() as *mut u64,
            r.arg2(),
            r.arg3(),
            r.arg4(),
        )
    }
}

pub unsafe extern "C" fn __x64_sys_mq_timedsend(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mq_timedsend(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as usize,
        r.arg3() as u32,
        r.arg4() as *const crate::kernel::time::Timespec64,
    )
}

pub unsafe extern "C" fn __x64_sys_mq_timedreceive(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mq_timedreceive(
        r.arg0() as i32,
        r.arg1() as *mut u8,
        r.arg2() as usize,
        r.arg3() as *mut u32,
        r.arg4() as *const crate::kernel::time::Timespec64,
    )
}

pub unsafe extern "C" fn __x64_sys_migrate_pages(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_migrate_pages(
        r.arg0() as i32,
        r.arg1(),
        r.arg2() as *const u64,
        r.arg3() as *const u64,
    )
}

pub unsafe extern "C" fn __x64_sys_futimesat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_futimesat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *const crate::kernel::syscalls::TimeVal,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_move_pages(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_move_pages(
        r.arg0() as i32,
        r.arg1() as usize,
        r.arg2() as *const u64,
        r.arg3() as *const i32,
        r.arg4() as *mut i32,
        r.arg5() as i32,
    )
}

pub unsafe extern "C" fn __x64_sys_utimensat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_utimensat(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *const crate::kernel::time::Timespec64,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_rt_tgsigqueueinfo(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_rt_tgsigqueueinfo(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as i32,
            r.arg3() as *const u8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_name_to_handle_at(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_name_to_handle_at(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as *mut crate::kernel::syscalls::FileHandle,
            r.arg3() as *mut i32,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_open_by_handle_at(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_open_by_handle_at(
        r.arg0() as i32,
        r.arg1() as *mut crate::kernel::syscalls::FileHandle,
        r.arg2() as i32,
    )
}

pub unsafe extern "C" fn __x64_sys_clock_adjtime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_clock_adjtime(
            r.arg0() as i32,
            r.arg1() as *mut crate::kernel::syscalls::Timex,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_process_vm_readv(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_process_vm_readv(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as usize,
        r.arg3() as *const u8,
        r.arg4() as usize,
        r.arg5(),
    )
}

pub unsafe extern "C" fn __x64_sys_process_vm_writev(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_process_vm_writev(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as usize,
        r.arg3() as *const u8,
        r.arg4() as usize,
        r.arg5(),
    )
}

pub unsafe extern "C" fn __x64_sys_kcmp(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_kcmp(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as i32,
            r.arg3(),
            r.arg4(),
        )
    }
}

pub unsafe extern "C" fn __x64_sys_rseq(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_rseq(
        r.arg0() as *mut u8,
        r.arg1() as u32,
        r.arg2() as i32,
        r.arg3() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_uretprobe(_regs: *mut PtRegs) -> i64 {
    crate::kernel::syscalls::sys_uretprobe()
}

pub unsafe extern "C" fn __x64_sys_uprobe(_regs: *mut PtRegs) -> i64 {
    crate::kernel::syscalls::sys_uprobe()
}

pub unsafe extern "C" fn __x64_sys_pidfd_send_signal(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_pidfd_send_signal(
        r.arg0() as i32,
        r.arg1() as i32,
        r.arg2() as *const u8,
        r.arg3() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_process_madvise(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_process_madvise(
        r.arg0() as i32,
        r.arg1() as *const u8,
        r.arg2() as usize,
        r.arg3() as i32,
        r.arg4() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_memfd_secret(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_memfd_secret(r.arg0() as u32)
}

pub unsafe extern "C" fn __x64_sys_process_mrelease(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_process_mrelease(r.arg0() as i32, r.arg1() as u32)
}

pub unsafe extern "C" fn __x64_sys_set_mempolicy_home_node(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_set_mempolicy_home_node(r.arg0(), r.arg1(), r.arg2(), r.arg3())
}

pub unsafe extern "C" fn __x64_sys_cachestat(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_cachestat(
            r.arg0() as u32,
            r.arg1() as *const crate::kernel::syscalls::CacheStatRange,
            r.arg2() as *mut crate::kernel::syscalls::CacheStat,
            r.arg3() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_map_shadow_stack(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_map_shadow_stack(r.arg0(), r.arg1(), r.arg2() as u32)
}

pub unsafe extern "C" fn __x64_sys_mseal(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_mseal(r.arg0(), r.arg1(), r.arg2())
}

pub unsafe extern "C" fn __x64_sys_listns(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_listns(
        r.arg0() as i32,
        r.arg1(),
        r.arg2() as *mut u8,
        r.arg3() as *mut u32,
        r.arg4() as u32,
    )
}

pub unsafe extern "C" fn __x64_sys_rseq_slice_yield(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    crate::kernel::syscalls::sys_rseq_slice_yield(r.arg0() as i32, r.arg1() as i32, r.arg2())
}

pub unsafe extern "C" fn __x64_sys_clock_settime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    match unsafe { read_timespec(r.arg1()) } {
        Ok(ts) => errno_result(
            crate::kernel::time::sys_clock_settime(r.arg0() as i32, ts),
            |_| 0,
        ),
        Err(errno) => -(errno as i64),
    }
}

pub unsafe extern "C" fn __x64_sys_clock_getres(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let out = r.arg1() as *mut crate::kernel::time::Timespec64;
    if out.is_null() {
        return -(crate::kernel::time::posix_clock::EINVAL as i64);
    }
    errno_result(
        crate::kernel::time::sys_clock_getres(r.arg0() as i32),
        |ts| match unsafe { write_user_value(out, &ts) } {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
    )
}

pub unsafe extern "C" fn __x64_sys_setrlimit(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_setrlimit(
            r.arg0() as i32,
            r.arg1() as *const crate::kernel::syscalls::RLimit,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_quotactl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_quotactl(
            r.arg0() as u32,
            r.arg1() as *const u8,
            r.arg2() as i32,
            r.arg3() as *mut u8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_signalfd(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::signalfd::sys_signalfd(r.arg0() as i32, r.arg1() as *const u8, r.arg2() as usize)
    }
}

pub unsafe extern "C" fn __x64_sys_clock_nanosleep(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let flags = r.arg1() as i32;
    let remain = r.arg3() as *mut crate::kernel::time::Timespec64;
    let mut remain_value = crate::kernel::time::Timespec64::default();
    let remain_arg = if remain.is_null() {
        None
    } else {
        Some(&raw mut remain_value)
    };
    if flags & !crate::kernel::time::posix_timers::TIMER_ABSTIME != 0 {
        return -(crate::kernel::time::posix_clock::EINVAL as i64);
    }
    let abs_time = flags & crate::kernel::time::posix_timers::TIMER_ABSTIME != 0;
    match unsafe { read_timespec(r.arg2()) } {
        Ok(request) => {
            let ret = errno_result(
                crate::kernel::time::sys_clock_nanosleep(
                    r.arg0() as i32,
                    abs_time,
                    request,
                    remain_arg,
                ),
                |_| 0,
            );
            // Linux copies rem back only for interrupted RELATIVE sleeps
            // (vendor/linux/kernel/time/hrtimer.c::nanosleep_copyout via
            // the TASK_INTERRUPTED restart path); absolute sleeps ignore
            // rem and completed sleeps leave it untouched.
            if ret == -(crate::include::uapi::errno::EINTR as i64)
                && !abs_time
                && !remain.is_null()
                && unsafe { write_user_value(remain, &remain_value) }.is_err()
            {
                return -(crate::include::uapi::errno::EFAULT as i64);
            }
            ret
        }
        Err(errno) => -(errno as i64),
    }
}

pub unsafe extern "C" fn __x64_sys_timer_create(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let out = r.arg2() as *mut i32;
    if out.is_null() {
        return -(crate::kernel::time::posix_clock::EINVAL as i64);
    }
    errno_result(
        crate::kernel::time::sys_timer_create(r.arg0() as i32, crate::kernel::signal::SIGALRM, 0),
        |id| match unsafe { write_user_value(out, &id) } {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
    )
}

pub unsafe extern "C" fn __x64_sys_timer_settime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let new_value = r.arg2() as *const crate::kernel::time::Itimerspec64;
    let old_value = r.arg3() as *mut crate::kernel::time::Itimerspec64;
    if new_value.is_null() {
        return -(crate::kernel::time::posix_clock::EINVAL as i64);
    }
    let new_value = match unsafe { read_user_value(new_value) } {
        Ok(value) => value,
        Err(errno) => return -(errno as i64),
    };
    errno_result(
        crate::kernel::time::sys_timer_settime(r.arg0() as i32, r.arg1() as i32, new_value),
        |old| {
            if old_value.is_null() {
                return 0;
            }
            match unsafe { write_user_value(old_value, &old) } {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            }
        },
    )
}

pub unsafe extern "C" fn __x64_sys_timer_gettime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    let out = r.arg1() as *mut crate::kernel::time::Itimerspec64;
    if out.is_null() {
        return -(crate::kernel::time::posix_clock::EINVAL as i64);
    }
    errno_result(
        crate::kernel::time::sys_timer_gettime(r.arg0() as i32),
        |cur| match unsafe { write_user_value(out, &cur) } {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
    )
}

pub unsafe extern "C" fn __x64_sys_timer_getoverrun(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    errno_result(
        crate::kernel::time::posix_timers::sys_timer_getoverrun(r.arg0() as i32),
        |overrun| overrun as i64,
    )
}

pub unsafe extern "C" fn __x64_sys_timer_delete(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    errno_result(
        crate::kernel::time::sys_timer_delete(r.arg0() as i32),
        |_| 0,
    )
}

pub unsafe extern "C" fn __x64_sys_timerfd_create(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::timerfd::sys_timerfd_create(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_timerfd_settime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::timerfd::sys_timerfd_settime(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as *const crate::kernel::time::Itimerspec64,
            r.arg3() as *mut crate::kernel::time::Itimerspec64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_timerfd_gettime(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::timerfd::sys_timerfd_gettime(
            r.arg0() as i32,
            r.arg1() as *mut crate::kernel::time::Itimerspec64,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_eventfd(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::eventfd::sys_eventfd(r.arg0() as u32) }
}

pub unsafe extern "C" fn __x64_sys_eventfd2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::eventfd::sys_eventfd2(r.arg0() as u32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_signalfd4(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::signalfd::sys_signalfd4(
            r.arg0() as i32,
            r.arg1() as *const u8,
            r.arg2() as usize,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_epoll_create1(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::eventpoll::sys_epoll_create1(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_epoll_create(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::eventpoll::sys_epoll_create(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_epoll_ctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::eventpoll::sys_epoll_ctl(
            r.arg0() as i32,
            r.arg1() as i32,
            r.arg2() as i32,
            r.arg3() as *const crate::fs::eventpoll::EpollEvent,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_epoll_pwait(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::eventpoll::sys_epoll_pwait(
            r.arg0() as i32,
            r.arg1() as *mut crate::fs::eventpoll::EpollEvent,
            r.arg2() as i32,
            r.arg3() as i32,
            r.arg4() as *const u8,
            r.arg5() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_epoll_pwait2(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::eventpoll::sys_epoll_pwait2(
            r.arg0() as i32,
            r.arg1() as *mut crate::fs::eventpoll::EpollEvent,
            r.arg2() as i32,
            r.arg3() as *const crate::kernel::time::Timespec64,
            r.arg4() as *const u8,
            r.arg5() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_epoll_wait(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::eventpoll::sys_epoll_wait(
            r.arg0() as i32,
            r.arg1() as *mut crate::fs::eventpoll::EpollEvent,
            r.arg2() as i32,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_inotify_init1(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::inotify::sys_inotify_init1(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_inotify_add_watch(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::inotify::sys_inotify_add_watch(
            r.arg0() as i32,
            r.arg1() as *const i8,
            r.arg2() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_inotify_rm_watch(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::inotify::sys_inotify_rm_watch(r.arg0() as i32, r.arg1() as i32) }
}

pub unsafe extern "C" fn __x64_sys_fanotify_init(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::fs::fanotify::sys_fanotify_init(r.arg0() as u32, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_fanotify_mark(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::fs::fanotify::sys_fanotify_mark(
            r.arg0() as i32,
            r.arg1() as u32,
            r.arg2(),
            r.arg3() as i32,
            r.arg4() as *const i8,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_io_uring_setup(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::io_uring::sys_io_uring_setup(
            r.arg0() as u32,
            r.arg1() as *mut crate::io_uring::IoUringParams,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_io_uring_enter(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::io_uring::sys_io_uring_enter(
            r.arg0() as i32,
            r.arg1() as u32,
            r.arg2() as u32,
            r.arg3() as u32,
            r.arg4() as *const u8,
            r.arg5() as usize,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_io_uring_register(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::io_uring::sys_io_uring_register(
            r.arg0() as i32,
            r.arg1() as u32,
            r.arg2() as *const u8,
            r.arg3() as u32,
        )
    }
}

// ── M63 — perf_event_open + sys_bpf ─────────────────────────────────────────

pub unsafe extern "C" fn __x64_sys_perf_event_open(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::events::sys_perf_event_open(
            r.arg0() as *const crate::kernel::events::PerfEventAttr,
            r.arg1() as i32,
            r.arg2() as i32,
            r.arg3() as i32,
            r.arg4(),
        )
    }
}

pub unsafe extern "C" fn __x64_sys_bpf(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::bpf::syscall::sys_bpf(
            r.arg0() as u32,
            r.arg1() as *const u8,
            r.arg2() as u32,
        )
    }
}

// ── M64 — keyring + landlock ────────────────────────────────────────────────

pub unsafe extern "C" fn __x64_sys_keyctl(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::security::keys::sys_keyctl(r.arg0() as i32, r.arg1(), r.arg2(), r.arg3(), r.arg4())
    }
}

pub unsafe extern "C" fn __x64_sys_add_key(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::security::keys::sys_add_key(
            r.arg0() as *const i8,
            r.arg1() as *const i8,
            r.arg2() as *const u8,
            r.arg3() as usize,
            r.arg4() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_request_key(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::security::keys::sys_request_key(
            r.arg0() as *const i8,
            r.arg1() as *const i8,
            r.arg2() as *const i8,
            r.arg3() as i32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_landlock_create_ruleset(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::security::landlock::sys_landlock_create_ruleset(
            r.arg0() as *const crate::security::landlock::syscalls::LandlockRulesetAttr,
            r.arg1() as usize,
            r.arg2() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_landlock_add_rule(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::security::landlock::sys_landlock_add_rule(
            r.arg0() as i32,
            r.arg1() as u32,
            r.arg2() as *const u8,
            r.arg3() as u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_landlock_restrict_self(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::security::landlock::sys_landlock_restrict_self(r.arg0() as i32, r.arg1() as u32)
    }
}

// M76: process identity, UTS, and session query syscalls.

pub unsafe extern "C" fn __x64_sys_getpid(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_getpid() }
}

pub unsafe extern "C" fn __x64_sys_gettid(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_gettid() }
}

pub unsafe extern "C" fn __x64_sys_getppid(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_getppid() }
}

pub unsafe extern "C" fn __x64_sys_uname(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_uname(r.arg0() as *mut crate::kernel::syscalls::LinuxUtsname)
    }
}

pub unsafe extern "C" fn __x64_sys_getuid(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_getuid() }
}

pub unsafe extern "C" fn __x64_sys_getgid(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_getgid() }
}

pub unsafe extern "C" fn __x64_sys_geteuid(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_geteuid() }
}

pub unsafe extern "C" fn __x64_sys_getegid(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_getegid() }
}

pub unsafe extern "C" fn __x64_sys_setuid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_setuid(r.arg0() as u32) }
}

pub unsafe extern "C" fn __x64_sys_setgid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_setgid(r.arg0() as u32) }
}

pub unsafe extern "C" fn __x64_sys_setreuid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_setreuid(r.arg0() as u32, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_setregid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_setregid(r.arg0() as u32, r.arg1() as u32) }
}

pub unsafe extern "C" fn __x64_sys_getgroups(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_getgroups(r.arg0() as i32, r.arg1() as *mut u32) }
}

pub unsafe extern "C" fn __x64_sys_setgroups(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_setgroups(r.arg0() as i32, r.arg1() as *const u32) }
}

pub unsafe extern "C" fn __x64_sys_setresuid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_setresuid(r.arg0() as u32, r.arg1() as u32, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_getresuid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_getresuid(
            r.arg0() as *mut u32,
            r.arg1() as *mut u32,
            r.arg2() as *mut u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_setresgid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_setresgid(r.arg0() as u32, r.arg1() as u32, r.arg2() as u32)
    }
}

pub unsafe extern "C" fn __x64_sys_getresgid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::syscalls::sys_getresgid(
            r.arg0() as *mut u32,
            r.arg1() as *mut u32,
            r.arg2() as *mut u32,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_setfsuid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_setfsuid(r.arg0() as u32) }
}

pub unsafe extern "C" fn __x64_sys_setfsgid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_setfsgid(r.arg0() as u32) }
}

pub unsafe extern "C" fn __x64_sys_getpgrp(_regs: *mut PtRegs) -> i64 {
    unsafe { crate::kernel::syscalls::sys_getpgrp() }
}

pub unsafe extern "C" fn __x64_sys_getpgid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_getpgid(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_getsid(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe { crate::kernel::syscalls::sys_getsid(r.arg0() as i32) }
}

pub unsafe extern "C" fn __x64_sys_capget(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::capability::sys_capget(
            r.arg0() as *mut crate::kernel::capability::UserCapHeader,
            r.arg1() as *mut crate::kernel::capability::UserCapData,
        )
    }
}

pub unsafe extern "C" fn __x64_sys_capset(regs: *mut PtRegs) -> i64 {
    let r = unsafe { &*regs };
    unsafe {
        crate::kernel::capability::sys_capset(
            r.arg0() as *const crate::kernel::capability::UserCapHeader,
            r.arg1() as *const crate::kernel::capability::UserCapData,
        )
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::kernel::cred::INIT_CRED;
    use crate::kernel::files;
    use crate::kernel::sched;
    use crate::kernel::sched::prio::{SCHED_FIFO, SCHED_NORMAL, SCHED_RR};
    use crate::kernel::task::TaskStruct;

    fn regs(args: [u64; 6]) -> PtRegs {
        PtRegs {
            rdi: args[0],
            rsi: args[1],
            rdx: args[2],
            r10: args[3],
            r8: args[4],
            r9: args[5],
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            r11: 0,
            rax: 0,
            rcx: 0,
            orig_rax: 0,
            rip: 0,
            cs: 0,
            eflags: 0,
            rsp: 0,
            ss: 0,
        }
    }

    #[test]
    fn syscall_m76_scheduler_wrapper_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 61;
        current.tgid = 61;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(__x64_sys_sched_yield(core::ptr::null_mut()), 0);

            let mut param = SchedParam { sched_priority: 0 };
            let mut r = regs([0, &mut param as *mut SchedParam as u64, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_sched_setparam(&mut r), 0);
            let mut r = regs([0, 0, 0, 0, 0, 0]);
            assert_eq!(
                __x64_sys_sched_setparam(&mut r),
                -(crate::include::uapi::errno::EINVAL as i64)
            );

            param.sched_priority = 2;
            let mut r = regs([
                0,
                SCHED_FIFO as u64,
                &param as *const SchedParam as u64,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_sched_setscheduler(&mut r), 0);
            let mut r = regs([0, 0, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_sched_getscheduler(&mut r), SCHED_FIFO as i64);
            let mut out = SchedParam { sched_priority: 0 };
            let mut r = regs([0, &mut out as *mut SchedParam as u64, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_sched_getparam(&mut r), 0);
            assert_eq!(out.sched_priority, 2);

            param.sched_priority = -1;
            let mut r = regs([0, &param as *const SchedParam as u64, 0, 0, 0, 0]);
            assert_eq!(
                __x64_sys_sched_setparam(&mut r),
                -(crate::include::uapi::errno::EINVAL as i64)
            );

            let mut r = regs([SCHED_RR as u64, 0, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_sched_get_priority_max(&mut r), 99);
            assert_eq!(__x64_sys_sched_get_priority_min(&mut r), 1);
            let mut r = regs([SCHED_NORMAL as u64, 0, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_sched_get_priority_max(&mut r), 0);
            let mut r = regs([42, 0, 0, 0, 0, 0]);
            assert_eq!(
                __x64_sys_sched_get_priority_max(&mut r),
                -(crate::include::uapi::errno::EINVAL as i64)
            );

            param.sched_priority = 2;
            let mut r = regs([
                0,
                SCHED_RR as u64,
                &param as *const SchedParam as u64,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_sched_setscheduler(&mut r), 0);
            let mut interval = crate::kernel::time::Timespec64::default();
            let mut r = regs([
                0,
                &mut interval as *mut crate::kernel::time::Timespec64 as u64,
                0,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_sched_rr_get_interval(&mut r), 0);
            assert!(interval.tv_nsec > 0);

            let mask = 1u64;
            let mut r = regs([
                0,
                core::mem::size_of::<crate::kernel::sched::entity::CpuMask>() as u64,
                &mask as *const u64 as u64,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_sched_setaffinity(&mut r), 0);
            let mut out_mask = 0u64;
            let mut r = regs([
                0,
                core::mem::size_of::<crate::kernel::sched::entity::CpuMask>() as u64,
                &mut out_mask as *mut u64 as u64,
                0,
                0,
                0,
            ]);
            assert_eq!(
                __x64_sys_sched_getaffinity(&mut r),
                core::mem::size_of::<crate::kernel::sched::entity::CpuMask>() as i64
            );
            assert_eq!(out_mask, mask);

            let bad_user_ptr = crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX;
            let mut r = regs([
                0,
                core::mem::size_of::<crate::kernel::sched::entity::CpuMask>() as u64,
                bad_user_ptr,
                0,
                0,
                0,
            ]);
            assert_eq!(
                __x64_sys_sched_setaffinity(&mut r),
                -(crate::include::uapi::errno::EFAULT as i64)
            );
            let mut r = regs([
                0,
                core::mem::size_of::<crate::kernel::sched::entity::CpuMask>() as u64,
                bad_user_ptr,
                0,
                0,
                0,
            ]);
            assert_eq!(
                __x64_sys_sched_getaffinity(&mut r),
                -(crate::include::uapi::errno::EFAULT as i64)
            );

            let attr = crate::kernel::sched::syscalls::SchedAttr {
                size: crate::kernel::sched::syscalls::SCHED_ATTR_SIZE_VER1,
                sched_policy: SCHED_NORMAL,
                sched_nice: 5,
                ..crate::kernel::sched::syscalls::SchedAttr::default()
            };
            let mut r = regs([0, &attr as *const _ as u64, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_sched_setattr(&mut r), 0);
            let mut out_attr = crate::kernel::sched::syscalls::SchedAttr::default();
            let mut r = regs([
                0,
                &mut out_attr as *mut _ as u64,
                crate::kernel::sched::syscalls::SCHED_ATTR_SIZE_VER1 as u64,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_sched_getattr(&mut r), 0);
            assert_eq!(out_attr.sched_policy, SCHED_NORMAL);
            assert_eq!(out_attr.sched_nice, 5);

            sched::set_current(previous);
        }
    }

    #[test]
    fn rt_sigreturn_wrapper_restores_pt_regs() {
        let mut stack = [0u8; 2048];
        let frame_base = unsafe { stack.as_mut_ptr().add(256) as u64 };
        let frame =
            unsafe { &mut *(frame_base as *mut crate::arch::x86::kernel::signal::RtSigFrame) };
        frame.uc.uc_sigmask = signal::SigSet { bits: 0x24 };
        frame.uc.uc_mcontext.r8 = 8;
        frame.uc.uc_mcontext.r9 = 9;
        frame.uc.uc_mcontext.r10 = 10;
        frame.uc.uc_mcontext.r11 = 11;
        frame.uc.uc_mcontext.r12 = 12;
        frame.uc.uc_mcontext.r13 = 13;
        frame.uc.uc_mcontext.r14 = 14;
        frame.uc.uc_mcontext.r15 = 15;
        frame.uc.uc_mcontext.rdi = 0x101;
        frame.uc.uc_mcontext.rsi = 0x102;
        frame.uc.uc_mcontext.rbp = 0x103;
        frame.uc.uc_mcontext.rbx = 0x104;
        frame.uc.uc_mcontext.rdx = 0x105;
        frame.uc.uc_mcontext.rax = 0x106;
        frame.uc.uc_mcontext.rcx = 0x107;
        frame.uc.uc_mcontext.rsp = 0x7fff_f000;
        frame.uc.uc_mcontext.rip = 0x401234;
        frame.uc.uc_mcontext.eflags = 0x246;
        frame.uc.uc_mcontext.cs = 0x33;
        frame.uc.uc_mcontext.ss = 0x2b;

        let mut regs = regs([0, 0, 0, 0, 0, 0]);
        regs.rsp = frame_base + core::mem::size_of::<u64>() as u64;
        regs.rip = 0x7000;
        regs.rax = crate::arch::x86::entry::syscall::SYS_RT_SIGRETURN;

        let ret = unsafe { __x64_sys_rt_sigreturn(&mut regs as *mut PtRegs) };

        assert_eq!(ret, 0x106);
        assert_eq!(regs.rip, 0x401234);
        assert_eq!(regs.rsp, 0x7fff_f000);
        assert_eq!(regs.rdi, 0x101);
        assert_eq!(regs.rsi, 0x102);
        assert_eq!(regs.rdx, 0x105);
        assert_eq!(regs.rax, 0x106);
        assert_eq!(regs.cs, 0x33);
        assert_eq!(regs.ss, 0x2b);
    }

    #[test]
    fn syscall_m76_time_timer_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 83;
        current.tgid = 83;
        current.cred = &raw const INIT_CRED;
        current.m29.se.sum_exec_runtime = 123_456;

        unsafe {
            files::set_task_files(
                &mut *current as *mut TaskStruct,
                crate::fs::fdtable::FilesStruct::new(),
            );
            sched::set_current(&mut *current as *mut TaskStruct);

            let zero = crate::kernel::time::Timespec64::default();
            let bad = crate::kernel::time::Timespec64 {
                tv_sec: 0,
                tv_nsec: 1_000_000_000,
            };
            let mut r = regs([&zero as *const _ as u64, 0, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_nanosleep(&mut r), 0);
            let one_ns = crate::kernel::time::Timespec64 {
                tv_sec: 0,
                tv_nsec: 1,
            };
            // A completed sleep never touches rem (Linux nanosleep_copyout
            // only runs on the EINTR restart path), so even an invalid rem
            // pointer returns success — Linux validates it only when used.
            let kernel_remain = 0xffff_8000_0000_0000u64;
            let mut r = regs([&one_ns as *const _ as u64, kernel_remain, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_nanosleep(&mut r), 0);
            let mut r = regs([&bad as *const _ as u64, 0, 0, 0, 0, 0]);
            assert_eq!(
                __x64_sys_nanosleep(&mut r),
                -(crate::include::uapi::errno::EINVAL as i64)
            );

            let mut now = crate::kernel::time::Timespec64::default();
            let mut r = regs([
                crate::kernel::time::CLOCK_MONOTONIC as u64,
                &mut now as *mut _ as u64,
                0,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_clock_gettime(&mut r), 0);
            let mut cpu_time = crate::kernel::time::Timespec64::default();
            let mut r = regs([
                crate::kernel::time::CLOCK_PROCESS_CPUTIME_ID as u64,
                &mut cpu_time as *mut _ as u64,
                0,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_clock_gettime(&mut r), 0);
            assert_eq!(cpu_time.to_ns(), 123_456);
            let mut r = regs([
                crate::kernel::time::CLOCK_PROCESS_CPUTIME_ID as u64,
                0,
                0,
                0,
                0,
                0,
            ]);
            assert_eq!(
                __x64_sys_clock_gettime(&mut r),
                -(crate::include::uapi::errno::EFAULT as i64)
            );
            let mut res = crate::kernel::time::Timespec64::default();
            let mut r = regs([
                crate::kernel::time::CLOCK_MONOTONIC as u64,
                &mut res as *mut _ as u64,
                0,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_clock_getres(&mut r), 0);
            assert!(res.tv_nsec > 0);
            let mut r = regs([
                crate::kernel::time::CLOCK_REALTIME as u64,
                &zero as *const _ as u64,
                0,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_clock_settime(&mut r), 0);
            let mut r = regs([
                crate::kernel::time::CLOCK_MONOTONIC as u64,
                0,
                &zero as *const _ as u64,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_clock_nanosleep(&mut r), 0);

            let mut timer_id = -1;
            let mut r = regs([
                crate::kernel::time::CLOCK_MONOTONIC as u64,
                0,
                &mut timer_id as *mut i32 as u64,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_timer_create(&mut r), 0);
            assert!(timer_id > 0);
            let spec = crate::kernel::time::Itimerspec64 {
                it_interval: zero,
                it_value: zero,
            };
            let mut old = crate::kernel::time::Itimerspec64::default();
            let mut r = regs([
                timer_id as u64,
                0,
                &spec as *const _ as u64,
                &mut old as *mut _ as u64,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_timer_settime(&mut r), 0);
            let mut cur = crate::kernel::time::Itimerspec64::default();
            let mut r = regs([timer_id as u64, &mut cur as *mut _ as u64, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_timer_gettime(&mut r), 0);
            let mut r = regs([timer_id as u64, 0, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_timer_getoverrun(&mut r), 0);
            assert_eq!(__x64_sys_timer_delete(&mut r), 0);

            let mut r = regs([crate::kernel::time::CLOCK_MONOTONIC as u64, 0, 0, 0, 0, 0]);
            let fd = __x64_sys_timerfd_create(&mut r);
            assert!(fd >= 0);
            let mut r = regs([
                fd as u64,
                0,
                &spec as *const _ as u64,
                &mut old as *mut _ as u64,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_timerfd_settime(&mut r), 0);
            let mut r = regs([fd as u64, &mut cur as *mut _ as u64, 0, 0, 0, 0]);
            assert_eq!(__x64_sys_timerfd_gettime(&mut r), 0);

            let mut tx = crate::kernel::syscalls::Timex::default();
            let mut r = regs([
                crate::kernel::time::CLOCK_REALTIME as u64,
                &mut tx as *mut _ as u64,
                0,
                0,
                0,
                0,
            ]);
            assert_eq!(__x64_sys_clock_adjtime(&mut r), 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
