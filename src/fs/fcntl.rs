//! linux-parity: partial
//! linux-source: vendor/linux/fs/fcntl.c
//! test-origin: linux:vendor/linux/fs/fcntl.c
//! `sys_fcntl` (M39).
//!
//! Mirrors `vendor/linux/fs/fcntl.c`.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use crate::include::uapi::errno::{EINVAL, EPERM};
use crate::include::uapi::fcntl::*;

use super::fdtable::{FilesStruct, NR_OPEN_MAX};

const SETFL_MASK: u32 = O_APPEND | O_NONBLOCK | O_NDELAY | O_DIRECT | O_NOATIME;

#[inline]
fn file_status_flags(flags: u32) -> u32 {
    // O_CLOEXEC is an fdtable flag in Linux. It is reported through
    // F_GETFD/FD_CLOEXEC, not through fcntl(F_GETFL).
    flags & !O_CLOEXEC
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
            let f = files.get(fd)?;
            let new = files.install_at_or_above(f, arg as usize, false)?;
            Ok(new as i64)
        }
        F_DUPFD_CLOEXEC => {
            if arg >= NR_OPEN_MAX as u64 {
                return Err(EINVAL);
            }
            let f = files.get(fd)?;
            let new = files.install_at_or_above(f, arg as usize, true)?;
            Ok(new as i64)
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
}
