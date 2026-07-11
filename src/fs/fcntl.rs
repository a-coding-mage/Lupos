//! linux-parity: partial
//! linux-source: vendor/linux/fs/fcntl.c
//! test-origin: linux:vendor/linux/fs/fcntl.c
//! `sys_fcntl` (M39).
//!
//! Mirrors `vendor/linux/fs/fcntl.c`.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EFAULT, EINVAL, EPERM};
use crate::include::uapi::fcntl::*;

use super::fdtable::{FilesStruct, NR_OPEN_MAX};

const SETFL_MASK: u32 = O_APPEND | O_NONBLOCK | O_NDELAY | O_DIRECT | O_NOATIME;

const SEEK_SET: i16 = 0;
const SEEK_CUR: i16 = 1;
const SEEK_END: i16 = 2;

#[inline]
fn file_status_flags(flags: u32) -> u32 {
    // O_CLOEXEC is an fdtable flag in Linux. It is reported through
    // F_GETFD/FD_CLOEXEC, not through fcntl(F_GETFL).
    flags & !O_CLOEXEC
}

fn copy_flock_from_user(arg: u64) -> Result<Flock, i32> {
    let mut lock = Flock::default();
    let left = unsafe {
        uaccess::copy_from_user(
            (&mut lock as *mut Flock).cast::<u8>(),
            arg as *const u8,
            core::mem::size_of::<Flock>(),
        )
    };
    if left != 0 {
        return Err(EFAULT);
    }
    Ok(lock)
}

fn copy_flock_to_user(arg: u64, lock: &Flock) -> Result<(), i32> {
    let left = unsafe {
        uaccess::copy_to_user(
            arg as *mut u8,
            (lock as *const Flock).cast::<u8>(),
            core::mem::size_of::<Flock>(),
        )
    };
    if left != 0 {
        return Err(EFAULT);
    }
    Ok(())
}

fn validate_flock(lock: &Flock) -> Result<(), i32> {
    match lock.l_type {
        F_RDLCK | F_WRLCK | F_UNLCK => {}
        _ => return Err(EINVAL),
    }
    match lock.l_whence {
        SEEK_SET | SEEK_CUR | SEEK_END => {}
        _ => return Err(EINVAL),
    }
    Ok(())
}

fn fcntl_getlk(arg: u64) -> Result<i64, i32> {
    let mut lock = copy_flock_from_user(arg)?;
    validate_flock(&lock)?;
    lock.l_type = F_UNLCK;
    lock.l_pid = 0;
    copy_flock_to_user(arg, &lock)?;
    Ok(0)
}

fn fcntl_setlk(arg: u64) -> Result<i64, i32> {
    let lock = copy_flock_from_user(arg)?;
    validate_flock(&lock)?;
    Ok(0)
}

pub fn sys_fcntl(files: &Arc<FilesStruct>, fd: i32, cmd: i32, arg: u64) -> Result<i64, i32> {
    match cmd {
        F_GETFD => {
            let flags = files.get_fd_flags(fd)?;
            Ok(flags as i64)
        }
        F_SETFD => {
            files.set_fd_flags(fd, arg as u32)?;
            Ok(0)
        }
        F_GETFL => {
            let f = files.get(fd)?;
            Ok(file_status_flags(f.flags.load(core::sync::atomic::Ordering::Acquire)) as i64)
        }
        F_SETFL => {
            let f = files.get(fd)?;
            let current = f.flags.load(core::sync::atomic::Ordering::Acquire);
            let requested = file_status_flags(arg as u32);
            let next = (requested & SETFL_MASK) | (current & !SETFL_MASK);
            f.flags.store(next, core::sync::atomic::Ordering::Release);
            Ok(0)
        }
        F_DUPFD => {
            if arg >= NR_OPEN_MAX as u64 {
                return Err(EINVAL);
            }
            let new = files.dup_at_or_above(fd, arg as usize, false)?;
            Ok(new as i64)
        }
        F_DUPFD_CLOEXEC => {
            if arg >= NR_OPEN_MAX as u64 {
                return Err(EINVAL);
            }
            let new = files.dup_at_or_above(fd, arg as usize, true)?;
            Ok(new as i64)
        }
        F_GETLK | F_GETLK64 | F_OFD_GETLK => {
            let _ = files.get(fd)?;
            fcntl_getlk(arg)
        }
        F_SETLK | F_SETLKW | F_SETLK64 | F_SETLKW64 | F_OFD_SETLK | F_OFD_SETLKW => {
            let _ = files.get(fd)?;
            fcntl_setlk(arg)
        }
        F_ADD_SEALS => {
            let f = files.get(fd)?;
            if f.fops.name != "memfd" {
                return Err(EINVAL);
            }
            let access = f.flags.load(Ordering::Acquire) & O_ACCMODE;
            if access != O_WRONLY && access != O_RDWR {
                return Err(EPERM);
            }
            let id = *f.private.lock() as u64;
            crate::mm::shmem::with_memfd_mut(id, |obj| obj.add_seals(arg as u32))
                .ok_or(EINVAL)??;
            Ok(0)
        }
        F_GET_SEALS => {
            let f = files.get(fd)?;
            if f.fops.name != "memfd" {
                return Err(EINVAL);
            }
            let id = *f.private.lock() as u64;
            crate::mm::shmem::memfd_object(id)
                .map(|obj| obj.seals() as i64)
                .ok_or(EINVAL)
        }
        _ => Err(EINVAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::NOOP_FILE_OPS;

    #[test]
    fn getfl_hides_cloexec_for_systemd_extrinsic_directory_fds() {
        let files = FilesStruct::new();
        let file = alloc_file(
            d_alloc("systemd-network"),
            O_RDONLY | O_DIRECTORY | O_CLOEXEC,
            0,
            &NOOP_FILE_OPS,
        );
        let fd = files.install(file, true).unwrap();

        assert_eq!(
            sys_fcntl(&files, fd, F_GETFD, 0).unwrap() as u32,
            FD_CLOEXEC
        );
        assert_eq!(
            sys_fcntl(&files, fd, F_GETFL, 0).unwrap() as u32,
            O_RDONLY | O_DIRECTORY
        );
    }

    #[test]
    fn setfl_does_not_move_cloexec_into_file_status_flags() {
        let files = FilesStruct::new();
        let file = alloc_file(
            d_alloc("fcntl-setfl"),
            O_WRONLY | O_APPEND,
            0,
            &NOOP_FILE_OPS,
        );
        let fd = files.install(file, false).unwrap();

        sys_fcntl(&files, fd, F_SETFL, (O_NONBLOCK | O_CLOEXEC) as u64).unwrap();

        assert_eq!(sys_fcntl(&files, fd, F_GETFD, 0).unwrap(), 0);
        assert_eq!(
            sys_fcntl(&files, fd, F_GETFL, 0).unwrap() as u32,
            O_WRONLY | O_NONBLOCK
        );
    }

    #[test]
    fn setlkw64_accepts_valid_advisory_lock_for_pwd_lock() {
        let files = FilesStruct::new();
        let file = alloc_file(d_alloc(".pwd.lock"), O_WRONLY, 0, &NOOP_FILE_OPS);
        let fd = files.install(file, true).unwrap();
        let lock = Flock {
            l_type: F_WRLCK,
            l_whence: SEEK_SET,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        assert_eq!(
            sys_fcntl(&files, fd, F_SETLKW64, (&lock as *const Flock) as u64).unwrap(),
            0
        );
    }

    #[test]
    fn getlk64_reports_no_conflicting_lock() {
        let files = FilesStruct::new();
        let file = alloc_file(d_alloc("lock-query"), O_RDWR, 0, &NOOP_FILE_OPS);
        let fd = files.install(file, true).unwrap();
        let mut lock = Flock {
            l_type: F_WRLCK,
            l_whence: SEEK_SET,
            l_start: 0,
            l_len: 0,
            l_pid: 123,
        };

        assert_eq!(
            sys_fcntl(&files, fd, F_GETLK64, (&mut lock as *mut Flock) as u64).unwrap(),
            0
        );
        assert_eq!(lock.l_type, F_UNLCK);
        assert_eq!(lock.l_pid, 0);
    }

    #[test]
    fn setlk_rejects_invalid_lock_type() {
        let files = FilesStruct::new();
        let file = alloc_file(d_alloc("bad-lock"), O_WRONLY, 0, &NOOP_FILE_OPS);
        let fd = files.install(file, true).unwrap();
        let lock = Flock {
            l_type: 99,
            l_whence: SEEK_SET,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        assert_eq!(
            sys_fcntl(&files, fd, F_SETLK, (&lock as *const Flock) as u64),
            Err(EINVAL)
        );
    }
}
