//! linux-parity: partial
//! linux-source: vendor/linux/fs/fcntl.c
//! test-origin: linux:vendor/linux/fs/fcntl.c
//! `sys_fcntl` (M39).
//!
//! Mirrors `vendor/linux/fs/fcntl.c`.

extern crate alloc;

use alloc::sync::Arc;
use core::ffi::c_void;
use core::sync::atomic::Ordering;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EFAULT, EINVAL, EPERM};
use crate::include::uapi::fcntl::*;
use crate::kernel::module::{export_symbol, find_symbol};

use super::fdtable::{FilesStruct, NR_OPEN_MAX};

const SETFL_MASK: u32 = O_APPEND | O_NONBLOCK | O_NDELAY | O_DIRECT | O_NOATIME;

const SEEK_SET: i16 = 0;
const SEEK_CUR: i16 = 1;
const SEEK_END: i16 = 2;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("fasync_helper", linux_fasync_helper as usize, false);
    export_symbol_once("kill_fasync", linux_kill_fasync as usize, false);
}

/// `fasync_helper` - `vendor/linux/fs/fcntl.c`.
///
/// Lupos does not yet model Linux `struct fasync_struct` ownership. Preserve
/// the ABI for modules that install fasync file operations, reporting "no
/// change" without mutating the caller's list.
pub unsafe extern "C" fn linux_fasync_helper(
    _fd: i32,
    _filp: *mut c_void,
    _on: i32,
    _fapp: *mut *mut c_void,
) -> i32 {
    0
}

/// `kill_fasync` - `vendor/linux/fs/fcntl.c`.
pub unsafe extern "C" fn linux_kill_fasync(_fp: *mut *mut c_void, _sig: i32, _band: i32) {}

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
            // Linux mutates the same `struct file::f_flags` that a character
            // driver's ->read()/->write() callback subsequently observes.
            // Lupos keeps a native File plus a configured-vendor `struct file`
            // adapter for module-backed character devices, so keep both views
            // coherent. In particular, ALSA sequencer reads use this bit to
            // return -EAGAIN after fcntl(F_SETFL, O_NONBLOCK).
            super::char_dev::sync_linux_module_chardev_flags(&f);
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
    use crate::fs::char_dev::LINUX_CHAR_FILE_OPS;
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
    fn setfl_updates_vendor_linux_file_flags_for_module_chardevs() {
        let files = FilesStruct::new();
        // Configured vendor `struct file` is 176 bytes with f_flags at byte
        // 40. Keep the backing aligned exactly as the module adapter does.
        let mut linux_file = [0u64; 22];
        let file = alloc_file(d_alloc("snd-seq"), O_RDWR, 0, &LINUX_CHAR_FILE_OPS);
        *file.private.lock() = linux_file.as_mut_ptr() as usize;
        let fd = files.install(file.clone(), false).unwrap();

        sys_fcntl(&files, fd, F_SETFL, O_NONBLOCK as u64).unwrap();

        let vendor_flags = unsafe {
            linux_file
                .as_ptr()
                .cast::<u8>()
                .add(40)
                .cast::<u32>()
                .read()
        };
        assert_eq!(vendor_flags, O_RDWR | O_NONBLOCK);

        // The test-owned array must not be released through the module
        // allocator when the final FileRef drops.
        *file.private.lock() = 0;
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
