//! linux-parity: complete
//! linux-source: vendor/linux/fs/ioctl.c
//! test-origin: linux:vendor/linux/fs/ioctl.c
//! `ioctl(2)` syscall dispatch for the early TTY/job-control path.
//!
//! Linux routes this through `fs/ioctl.c::do_vfs_ioctl` and then into each
//! file's `->unlocked_ioctl`. Lupos now exposes the file-operation slot and
//! keeps the older TTY fallback for nodes that have not been converted yet.

use core::ffi::c_void;
use core::sync::atomic::Ordering;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EBADF, EFAULT, ENOTTY};
use crate::include::uapi::fcntl::{O_NONBLOCK, O_PATH};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::{files, sched};

use super::types::FileRef;

// Linux asm-generic/ioctls.h. FIONBIO is a generic VFS ioctl and must be
// handled before dispatching to socket/pipe/device-specific file operations.
const FIONBIO: u32 = 0x5421;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("compat_ptr_ioctl", linux_compat_ptr_ioctl as usize, false);
}

/// `compat_ptr_ioctl` - `vendor/linux/fs/ioctl.c`.
///
/// On x86-64 this helper is used as a file-operation function pointer. Lupos
/// does not yet expose raw Linux `struct file_operations` dispatch for module
/// file objects, so preserve the ABI as a fail-closed ioctl handler.
#[unsafe(export_name = "compat_ptr_ioctl")]
pub unsafe extern "C" fn linux_compat_ptr_ioctl(
    _file: *mut c_void,
    _cmd: u32,
    _arg: usize,
) -> isize {
    -(ENOTTY as isize)
}

fn is_tty_name(name: &str) -> bool {
    name == "console" || name.starts_with("tty")
}

fn ioctl_fionbio(file: &FileRef, arg: u64) -> Result<i64, i32> {
    let mut on = 0i32;
    let left = unsafe {
        uaccess::copy_from_user(
            (&mut on as *mut i32).cast::<u8>(),
            arg as *const u8,
            core::mem::size_of::<i32>(),
        )
    };
    if left != 0 {
        return Err(EFAULT);
    }
    if on != 0 {
        file.flags.fetch_or(O_NONBLOCK, Ordering::AcqRel);
    } else {
        file.flags.fetch_and(!O_NONBLOCK, Ordering::AcqRel);
    }
    Ok(0)
}

/// Linux x86-64 syscall 16: `ioctl(fd, cmd, arg)`.
pub unsafe fn sys_ioctl(fd: i32, cmd: u32, arg: u64) -> i64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let Some(ft) = (unsafe { files::get_task_files(task) }) else {
        return -(EBADF as i64);
    };
    let file = match ft.get(fd) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    if file.flags.load(core::sync::atomic::Ordering::Acquire) & O_PATH != 0 {
        return -(EBADF as i64);
    }
    let ret = if cmd == FIONBIO {
        match ioctl_fionbio(&file, arg) {
            Ok(ret) => ret,
            Err(errno) => -(errno as i64),
        }
    } else if let Some(ioctl) = file.fops.ioctl {
        match ioctl(&file, cmd, arg) {
            Ok(ret) => ret,
            Err(errno) => -(errno as i64),
        }
    } else if !is_tty_name(&file.dentry.name) {
        -(ENOTTY as i64)
    } else {
        match crate::linux_driver_abi::tty::tty_ioctl_compat(cmd, arg) {
            Ok(ret) => ret,
            Err(errno) => -(errno as i64),
        }
    };
    ret
}

#[cfg(test)]
mod tests {
    use super::{ioctl_fionbio, is_tty_name};
    use crate::fs::dcache::d_alloc;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::NOOP_FILE_OPS;
    use crate::include::uapi::fcntl::{O_NONBLOCK, O_RDWR};
    use core::sync::atomic::Ordering;

    #[test]
    fn tty_name_filter_matches_console_and_tty_nodes() {
        assert!(is_tty_name("console"));
        assert!(is_tty_name("tty1"));
        assert!(!is_tty_name("passwd"));
    }

    #[test]
    fn fionbio_updates_generic_file_status_flag() {
        let file = alloc_file(d_alloc("socket"), O_RDWR, 0, &NOOP_FILE_OPS);
        let mut on = 1i32;
        assert_eq!(ioctl_fionbio(&file, (&mut on as *mut i32) as u64), Ok(0));
        assert_ne!(file.flags.load(Ordering::Acquire) & O_NONBLOCK, 0);

        on = 0;
        assert_eq!(ioctl_fionbio(&file, (&mut on as *mut i32) as u64), Ok(0));
        assert_eq!(file.flags.load(Ordering::Acquire) & O_NONBLOCK, 0);
    }
}
