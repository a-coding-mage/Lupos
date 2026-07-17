//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/fd.c
//! test-origin: linux:vendor/linux/fs/proc/fd.c
//! `/proc/<pid>/fd`.
//!
//! Ref: `vendor/linux/fs/proc/fd.c`

use alloc::sync::Arc;
use alloc::{format, string::String, vec::Vec};
use core::sync::atomic::Ordering;

use crate::fs::anon_inode::alloc_anon_file_with_kind;
use crate::fs::file::file_path;
use crate::fs::kernfs::KernfsNode;
use crate::fs::ops::{FileOps, InodeOps};
use crate::fs::types::{FileRef, Inode, InodeKind, InodePrivate, InodeRef};
use crate::include::uapi::errno::{EBADF, EINVAL, ENOENT};
use crate::kernel::{files, sched};

pub fn new_fd_dir() -> Arc<KernfsNode> {
    KernfsNode::new_dynamic_dir("fd", 0o555, Some(fd_dir_lookup), Some(fd_dir_readdir))
}

pub fn current_fd_path(fd: i32) -> Result<String, i32> {
    let file = current_fd_file(fd)?;
    Ok(crate::fs::file::path_hint(&file)
        .or_else(|| crate::fs::mount::stable_path_for_dentry(&file.dentry))
        .unwrap_or_else(|| file_path(&file)))
}

pub fn current_fd_file(fd: i32) -> Result<FileRef, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    let files = unsafe { files::get_task_files(task) }.ok_or(EBADF)?;
    files.get(fd)
}

fn current_open_fds() -> Vec<i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Vec::new();
    }
    let Some(files) = (unsafe { files::get_task_files(task) }) else {
        return Vec::new();
    };
    files.open_fds()
}

fn proc_fd_ino(fd: i32) -> u64 {
    0xf000_0000u64 + fd.max(0) as u64
}

fn parse_fd_name(name: &str) -> Result<i32, i32> {
    if name.is_empty() || name.as_bytes().iter().any(|b| !b.is_ascii_digit()) {
        return Err(ENOENT);
    }
    name.parse::<i32>().map_err(|_| ENOENT)
}

fn fd_dir_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let fd = parse_fd_name(name)?;
    current_fd_file(fd).map_err(|_| ENOENT)?;
    let inode = Inode::new(
        proc_fd_ino(fd),
        InodeKind::Symlink,
        0o777,
        &PROC_FD_SYMLINK_INODE_OPS,
        &PROC_FD_SYMLINK_FILE_OPS,
        InodePrivate::Opaque(fd as usize),
    );
    *inode.sb.lock() = dir.sb.lock().clone();
    Ok(inode)
}

fn fd_dir_readdir(file: &FileRef) -> Result<Option<(String, u64, InodeKind)>, i32> {
    if let Some(dot) = crate::fs::libfs::synthetic_readdir_dot_entry(file)? {
        return Ok(Some(dot));
    }
    let fds = current_open_fds();
    let mut idx = file.pos.lock();
    let fd_idx = idx.saturating_sub(2) as usize;
    if fd_idx >= fds.len() {
        return Ok(None);
    }
    let fd = fds[fd_idx];
    *idx += 1;
    Ok(Some((
        format!("{}", fd),
        proc_fd_ino(fd),
        InodeKind::Symlink,
    )))
}

static PROC_FD_SYMLINK_INODE_OPS: InodeOps = InodeOps {
    name: "proc_fd_symlink",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: Some(proc_fd_readlink),
};

static PROC_FD_SYMLINK_FILE_OPS: FileOps = FileOps {
    name: "proc_fd_symlink",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

fn proc_fd_readlink(inode: &InodeRef, buf: &mut [u8]) -> Result<usize, i32> {
    let fd = match inode.private {
        InodePrivate::Opaque(fd) => fd as i32,
        _ => return Err(EINVAL),
    };
    let target = current_fd_path(fd).map_err(|_| ENOENT)?;
    let n = target.len().min(buf.len());
    buf[..n].copy_from_slice(&target.as_bytes()[..n]);
    Ok(n)
}

static FDINFO_FILE_OPS: FileOps = FileOps {
    name: "proc-fdinfo",
    read: Some(fdinfo_read),
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

pub fn current_fdinfo_file(fd: i32, flags: u32, mode: u32) -> Result<FileRef, i32> {
    if let Err(errno) = current_fd_file(fd) {
        trace_fdinfo_open(fd, Err(errno));
        return Err(errno);
    }
    let _ = mode;
    let file = alloc_anon_file_with_kind(
        "fdinfo",
        &FDINFO_FILE_OPS,
        fd as usize,
        InodeKind::Regular,
        0o444,
    );
    file.flags.store(flags, Ordering::Release);
    trace_fdinfo_open(fd, Ok(()));
    Ok(file)
}

pub fn current_fdinfo_file_from_proc_path(
    path: &str,
    flags: u32,
    mode: u32,
) -> Option<Result<FileRef, i32>> {
    let rest = path.strip_prefix("/proc/self/fdinfo/")?;
    if rest.is_empty() || rest.as_bytes().iter().any(|b| !b.is_ascii_digit()) {
        return Some(Err(ENOENT));
    }
    let fd = match rest.parse::<i32>() {
        Ok(fd) => fd,
        Err(_) => return Some(Err(ENOENT)),
    };
    Some(current_fdinfo_file(fd, flags, mode))
}

fn parse_proc_fd_path(path: &str) -> Option<Result<(i32, &str), i32>> {
    let rest = path
        .strip_prefix("/proc/self/fd/")
        .or_else(|| path.strip_prefix("/dev/fd/"))?;
    if rest.is_empty() {
        return Some(Err(ENOENT));
    }
    let digit_len = rest
        .as_bytes()
        .iter()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(rest.len());
    if digit_len == 0 {
        return Some(Err(ENOENT));
    }
    let suffix = &rest[digit_len..];
    if !suffix.is_empty() && !suffix.starts_with('/') {
        return Some(Err(ENOENT));
    }
    let fd = match rest[..digit_len].parse::<i32>() {
        Ok(fd) => fd,
        Err(_) => return Some(Err(ENOENT)),
    };
    Some(Ok((fd, suffix)))
}

fn fdinfo_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let fd = *file.private.lock() as i32;
    let target = match current_fd_file(fd) {
        Ok(target) => target,
        Err(errno) => {
            trace_fdinfo_read(fd, Err(errno), None);
            return Err(errno);
        }
    };
    let pid = crate::fs::pidfd::pid_for_file(&target).ok();
    trace_fdinfo_read(fd, Ok(()), pid);
    let target_flags = target.flags.load(Ordering::Acquire);
    let ino = target.inode().map(|inode| inode.ino).unwrap_or(0);
    let mut text = format!(
        "pos:\t0\nflags:\t0{:o}\nmnt_id:\t0\nino:\t{}\n",
        target_flags, ino
    );
    if let Some(pid) = pid {
        text.push_str(&format!("Pid:\t{}\nNSpid:\t{}\n", pid, pid));
    }
    let bytes = text.as_bytes();
    let start = (*pos as usize).min(bytes.len());
    let n = (bytes.len() - start).min(buf.len());
    buf[..n].copy_from_slice(&bytes[start..start + n]);
    *pos += n as u64;
    Ok(n)
}

#[cfg(not(test))]
fn trace_fdinfo_open(fd: i32, result: Result<(), i32>) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    let task = unsafe { sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    match result {
        Ok(()) => {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-proc-fdinfo-open pid={} fd={} ok",
                pid,
                fd
            )
        }
        Err(errno) => crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-fdinfo-open pid={} fd={} errno={}",
            pid,
            fd,
            errno
        ),
    }
}

#[cfg(test)]
fn trace_fdinfo_open(_fd: i32, _result: Result<(), i32>) {}

#[cfg(not(test))]
fn trace_fdinfo_read(fd: i32, result: Result<(), i32>, target_pid: Option<i32>) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    let task = unsafe { sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    match result {
        Ok(()) => crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-fdinfo-read pid={} fd={} target_pid={}",
            pid,
            fd,
            target_pid.unwrap_or(-1)
        ),
        Err(errno) => crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-fdinfo-read pid={} fd={} errno={}",
            pid,
            fd,
            errno
        ),
    }
}

#[cfg(test)]
fn trace_fdinfo_read(_fd: i32, _result: Result<(), i32>, _target_pid: Option<i32>) {}

pub fn current_fd_path_from_proc_path(path: &str) -> Option<Result<String, i32>> {
    let (fd, suffix) = match parse_proc_fd_path(path)? {
        Ok(parts) => parts,
        Err(errno) => return Some(Err(errno)),
    };
    Some(current_fd_path(fd).map(|mut base| {
        if suffix.is_empty() {
            return base;
        }
        if base == "/" {
            base.push_str(suffix.trim_start_matches('/'));
        } else {
            base.push_str(suffix);
        }
        base
    }))
}

pub fn current_fd_file_from_proc_path(path: &str) -> Option<Result<FileRef, i32>> {
    let (fd, suffix) = match parse_proc_fd_path(path)? {
        Ok(parts) => parts,
        Err(errno) => return Some(Err(errno)),
    };
    if !suffix.is_empty() {
        return None;
    }
    Some(current_fd_file(fd))
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::fs::anon_inode::alloc_anon_file;
    use crate::fs::dcache::d_alloc;
    use crate::fs::fdtable::FilesStruct;
    use crate::fs::file::{alloc_file, set_path_hint};
    use crate::fs::ops::NOOP_FILE_OPS;
    use crate::fs::read_write::vfs_read;
    use crate::fs::types::SuperBlock;
    use crate::include::uapi::fcntl::O_RDONLY;
    use crate::include::uapi::stat::{S_IFMT, S_IFREG};
    use crate::kernel::cred::INIT_CRED;
    use crate::kernel::pid::{INIT_PID_NS, alloc_pid, put_pid};
    use crate::kernel::task::TaskStruct;

    #[test]
    fn fdinfo_for_pidfd_reports_target_pid() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 430;
        current.tgid = 430;
        current.cred = &raw const INIT_CRED;

        let mut target = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        target.pid = 431;
        target.tgid = 431;
        target.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(target.pid)).expect("pid alloc");
        target.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd =
                crate::fs::pidfd::install_pidfd(&mut *target as *mut TaskStruct, false).unwrap();
            let info = current_fdinfo_file(fd, O_RDONLY, 0).unwrap();
            let inode = info.inode().expect("fdinfo inode");
            assert_eq!(
                inode.mode.load(Ordering::Acquire) & S_IFMT,
                S_IFREG,
                "fdinfo should stat as a regular proc-style file"
            );
            let mut buf = [0u8; 128];
            let n = vfs_read(&info, &mut buf).unwrap();
            let text = core::str::from_utf8(&buf[..n]).unwrap();
            assert!(text.contains("Pid:\t431\n"));
            assert!(text.contains("NSpid:\t431\n"));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(target.m26.thread_pid);
            target.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn proc_self_fd_readdir_lists_live_fdtable_entries() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 432;
        current.tgid = 432;
        current.cred = &raw const INIT_CRED;

        unsafe {
            let fdt = FilesStruct::new();
            let held = alloc_anon_file("held", &NOOP_FILE_OPS, 0);
            set_path_hint(&held, String::from("/newroot/usr"));
            assert_eq!(fdt.install_at_or_above(held, 5, false), Ok(5));
            files::set_task_files(&mut *current as *mut TaskStruct, fdt);
            sched::set_current(&mut *current as *mut TaskStruct);

            let sb = SuperBlock::alloc("proc", 0x9fa0, &crate::fs::proc::PROCFS_SUPER_OPS);
            let fd_dir = new_fd_dir();
            let dir_inode = crate::fs::kernfs::inode_for_node(&sb, fd_dir);
            let dir_dentry = d_alloc("fd");
            dir_dentry.instantiate(dir_inode.clone());
            let dir_file = alloc_file(dir_dentry, O_RDONLY, 0, dir_inode.fops);

            let dot = fd_dir_readdir(&dir_file)
                .expect("readdir")
                .expect("dot entry");
            assert_eq!(dot.0, ".");
            assert_eq!(dot.2, InodeKind::Directory);
            let dotdot = fd_dir_readdir(&dir_file)
                .expect("readdir")
                .expect("dotdot entry");
            assert_eq!(dotdot.0, "..");
            assert_eq!(dotdot.2, InodeKind::Directory);
            let fd_entry = fd_dir_readdir(&dir_file)
                .expect("readdir")
                .expect("fd entry");
            assert_eq!(fd_entry.0, "5");
            assert_eq!(fd_entry.2, InodeKind::Symlink);
            assert!(fd_dir_readdir(&dir_file).expect("readdir eof").is_none());

            let link = fd_dir_lookup(&dir_inode, "5").expect("fd link");
            let mut buf = [0u8; 64];
            let n = proc_fd_readlink(&link, &mut buf).expect("readlink");
            assert_eq!(&buf[..n], b"/newroot/usr");
            assert_eq!(fd_dir_lookup(&dir_inode, "6").err(), Some(ENOENT));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
