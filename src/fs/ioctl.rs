//! linux-parity: complete
//! linux-source: vendor/linux/fs/ioctl.c
//! test-origin: linux:vendor/linux/fs/ioctl.c
//! `ioctl(2)` syscall dispatch for the early TTY/job-control path.
//!
//! Linux routes this through `fs/ioctl.c::do_vfs_ioctl` and then into each
//! file's `->unlocked_ioctl`. Lupos now exposes the file-operation slot and
//! keeps the older TTY fallback for nodes that have not been converted yet.

use crate::include::uapi::errno::{EBADF, ENOTTY};
use crate::include::uapi::fcntl::O_PATH;
use crate::kernel::{files, sched};

fn is_tty_name(name: &str) -> bool {
    name == "console" || name.starts_with("tty")
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
    let ret = if let Some(ioctl) = file.fops.ioctl {
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
    use super::is_tty_name;

    #[test]
    fn tty_name_filter_matches_console_and_tty_nodes() {
        assert!(is_tty_name("console"));
        assert!(is_tty_name("tty1"));
        assert!(!is_tty_name("passwd"));
    }
}
