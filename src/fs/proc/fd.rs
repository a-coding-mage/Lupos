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
use crate::kernel::{files, sched, task::TaskStruct};

pub fn new_fd_dir() -> Arc<KernfsNode> {
    KernfsNode::new_dynamic_dir("fd", 0o555, Some(fd_dir_lookup), Some(fd_dir_readdir))
}

pub fn current_fd_path(fd: i32) -> Result<String, i32> {
    let task = unsafe { sched::get_current() };
    task_fd_path(task, fd)
}

#[cfg(not(test))]
macro_rules! trace_procfd {
    ($($arg:tt)*) => {
        if crate::kernel::debug_trace::fs_enabled()
            || crate::kernel::debug_trace::glycin_enabled()
        {
            crate::linux_driver_abi::tty::serial_println!($($arg)*);
        }
    };
}

#[cfg(test)]
macro_rules! trace_procfd {
    ($($arg:tt)*) => {};
}

fn task_fd_path(task: *mut TaskStruct, fd: i32) -> Result<String, i32> {
    let file = task_fd_file(task, fd)?;
    Ok(crate::fs::file::path_hint(&file)
        .or_else(|| crate::fs::mount::stable_path_for_dentry(&file.dentry))
        .or_else(|| pseudo_file_dname(&file))
        .unwrap_or_else(|| file_path(&file)))
}

/// Linux `d_path()` equivalent for files that live on a pseudo filesystem.
///
/// Sockets, pipes and anon-inode files hang off an unparented dentry, so
/// `dentry_path()` walks straight past the name and reports `/` for every one
/// of them.  Linux instead routes these through per-filesystem `d_dname`
/// callbacks, so `/proc/<pid>/fd/N` reads back as `socket:[<ino>]`,
/// `pipe:[<ino>]` or `anon_inode:<name>`
/// (`vendor/linux/net/socket.c::sockfs_dname`,
/// `vendor/linux/fs/pipe.c::pipefs_dname`,
/// `vendor/linux/fs/anon_inodes.c::anon_inodefs_dname`).
///
/// Reporting `/` here is not just cosmetic: gdk-pixbuf resolves descriptors by
/// readlinking `/proc/<pid>/fd/<n>`, and bubblewrap canonicalizes its `O_PATH`
/// bind destinations the same way, so a bogus `/` silently redirects them at a
/// directory.  Returns `None` for files that really do have a path, leaving the
/// normal `dentry_path()` walk in charge.
fn pseudo_file_dname(file: &FileRef) -> Option<String> {
    // A dentry with a parent is reachable in the tree; let d_path handle it.
    if file.dentry.parent.lock().is_some() {
        return None;
    }
    let name = file.dentry.name.clone();
    if name.is_empty() || name == "/" {
        return None;
    }
    let ino = file.inode().map(|inode| inode.ino).unwrap_or(0);
    // memfd files carry their own "memfd:<name>" spelling and read back from
    // Linux as a deleted shmem path.
    if let Some(rest) = name.strip_prefix("memfd:") {
        return Some(format!("/memfd:{rest} (deleted)"));
    }
    if name.starts_with("pipe:") {
        return Some(format!("pipe:[{ino}]"));
    }
    if file.fops.name == "socket" || name == "socket" {
        return Some(format!("socket:[{ino}]"));
    }
    Some(format!("anon_inode:{name}"))
}

pub fn current_fd_file(fd: i32) -> Result<FileRef, i32> {
    let task = unsafe { sched::get_current() };
    task_fd_file(task, fd)
}

fn task_fd_file(task: *mut TaskStruct, fd: i32) -> Result<FileRef, i32> {
    if task.is_null() {
        return Err(EBADF);
    }
    let files = unsafe { files::get_task_files(task) }.ok_or(EBADF)?;
    files.get(fd)
}

fn task_open_fds(task: *mut TaskStruct) -> Vec<i32> {
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
    let (task, pid) = fd_dir_owner(dir)?;
    if let Err(errno) = task_fd_file(task, fd) {
        trace_procfd!(
            "trace-procfd lookup fd={} step=task_fd_file errno={}",
            fd,
            errno
        );
        return Err(ENOENT);
    }
    let inode = Inode::new(
        proc_fd_ino(fd),
        InodeKind::Symlink,
        0o777,
        &PROC_FD_SYMLINK_INODE_OPS,
        &PROC_FD_SYMLINK_FILE_OPS,
        InodePrivate::Opaque(pack_proc_fd(pid, fd)),
    );
    *inode.sb.lock() = dir.sb.lock().clone();
    Ok(inode)
}

fn fd_dir_readdir(file: &FileRef) -> Result<Option<(String, u64, InodeKind)>, i32> {
    if let Some(dot) = crate::fs::libfs::synthetic_readdir_dot_entry(file)? {
        return Ok(Some(dot));
    }
    let inode = file.inode().ok_or(EINVAL)?;
    let task = task_from_fd_dir_inode(&inode)?;
    let fds = task_open_fds(task);
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
    setattr: None,
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
    let (pid, fd) = match inode.private {
        InodePrivate::Opaque(value) => unpack_proc_fd(value),
        _ => return Err(EINVAL),
    };
    let task = task_for_proc_fd_pid(pid);
    if task.is_null() {
        trace_procfd!(
            "trace-procfd readlink pid={} fd={} step=task_by_pid null",
            pid,
            fd
        );
        return Err(ENOENT);
    }
    let target = match task_fd_path(task, fd) {
        Ok(target) => target,
        Err(errno) => {
            trace_procfd!(
                "trace-procfd readlink pid={} fd={} step=task_fd_path errno={}",
                pid,
                fd,
                errno
            );
            return Err(ENOENT);
        }
    };
    let n = target.len().min(buf.len());
    buf[..n].copy_from_slice(&target.as_bytes()[..n]);
    Ok(n)
}

/// Recorded in place of a pid when the `fd` directory is `/proc/self`, meaning
/// "resolve against `current` on every access".  Never a valid task pid.
pub(crate) const PROC_FD_PID_SELF: i32 = 0;

/// Task owning a `/proc/<pid>/fd/N` symlink, honouring [`PROC_FD_PID_SELF`].
fn task_for_proc_fd_pid(pid: i32) -> *mut TaskStruct {
    if pid == PROC_FD_PID_SELF {
        unsafe { sched::get_current() }
    } else {
        crate::fs::proc::base::task_by_pid(pid)
    }
}

fn pack_proc_fd(pid: i32, fd: i32) -> usize {
    ((pid.max(0) as u64) << 32 | fd.max(0) as u32 as u64) as usize
}

fn unpack_proc_fd(value: usize) -> (i32, i32) {
    (((value as u64) >> 32) as i32, (value as u32) as i32)
}

fn task_from_proc_pid_name(name: &str) -> Result<*mut TaskStruct, i32> {
    if name == "self" {
        let task = unsafe { sched::get_current() };
        return if task.is_null() {
            Err(ENOENT)
        } else {
            Ok(task)
        };
    }
    let pid = name.parse::<i32>().map_err(|_| ENOENT)?;
    let task = crate::fs::proc::base::task_by_pid(pid);
    if task.is_null() {
        Err(ENOENT)
    } else {
        Ok(task)
    }
}

fn task_from_fd_dir_inode(dir: &InodeRef) -> Result<*mut TaskStruct, i32> {
    Ok(fd_dir_owner(dir)?.0)
}

/// Resolve the task owning an `fd` directory, plus the pid to record in the
/// symlink inodes below it.
///
/// Linux keeps `/proc/self` as a symlink whose target is regenerated from
/// `current` on every path walk
/// (`vendor/linux/fs/proc/self.c::proc_self_get_link`), so `/proc/self/fd/N`
/// always instantiates under the *calling* process's `/proc/<pid>` directory
/// and `proc_fd_link()` reads that process's file table.  Lupos models
/// `/proc/self` as a directory instead, so a single `fd/N` inode is cached and
/// shared by every process that walks it.  Recording a concrete pid there binds
/// the symlink to whichever task looked it up first; once that task exits,
/// `task_by_pid()` returns NULL and readlink fails with ENOENT even though the
/// caller's own fd is open.  That is what broke bubblewrap: it opens the bind
/// destination with `O_PATH` and canonicalizes it through
/// `readlink("/proc/self/fd/N")`, so every sandboxed glycin image loader died
/// before it could reply and Firefox blocked forever waiting for one.
///
/// Store the sentinel for `/proc/self` and re-resolve against `current` at
/// access time, matching the way the other `/proc/self` consumers in this tree
/// already handle it (`base.rs::proc_pid_from_node`,
/// `namespaces.rs::proc_user_ns_path_pid`).
fn fd_dir_owner(dir: &InodeRef) -> Result<(*mut TaskStruct, i32), i32> {
    let node = crate::fs::kernfs::node_from_inode(dir);
    let parent = node.parent.lock().upgrade().ok_or(EINVAL)?;
    let task = task_from_proc_pid_name(&parent.name)?;
    let pid = if parent.name == "self" {
        PROC_FD_PID_SELF
    } else {
        crate::kernel::pid_namespace::task_pid_vnr(task)
    };
    Ok((task, pid))
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

fn strip_numeric_pid_fd_prefix(path: &str) -> Option<(i32, &str)> {
    let rest = path.strip_prefix("/proc/")?;
    let digit_len = rest
        .as_bytes()
        .iter()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(rest.len());
    if digit_len == 0 {
        return None;
    }
    let pid = rest[..digit_len].parse::<i32>().ok()?;
    let rest = rest[digit_len..].strip_prefix("/fd/")?;
    Some((pid, rest))
}

fn parse_proc_fd_path(path: &str) -> Option<Result<(*mut TaskStruct, i32, &str), i32>> {
    let (task, rest) = if let Some(rest) = path
        .strip_prefix("/proc/self/fd/")
        .or_else(|| path.strip_prefix("/dev/fd/"))
    {
        let task = unsafe { sched::get_current() };
        if task.is_null() {
            return Some(Err(ENOENT));
        }
        (task, rest)
    } else if let Some((pid, rest)) = strip_numeric_pid_fd_prefix(path) {
        let task = crate::fs::proc::base::task_by_pid(pid);
        if task.is_null() {
            return Some(Err(ENOENT));
        }
        (task, rest)
    } else {
        return None;
    };
    if rest.is_empty() {
        return None;
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
    Some(Ok((task, fd, suffix)))
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
    let (task, fd, suffix) = match parse_proc_fd_path(path)? {
        Ok(parts) => parts,
        Err(errno) => return Some(Err(errno)),
    };
    Some(task_fd_path(task, fd).map(|mut base| {
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
    let (task, fd, suffix) = match parse_proc_fd_path(path)? {
        Ok(parts) => parts,
        Err(errno) => return Some(Err(errno)),
    };
    if !suffix.is_empty() {
        return None;
    }
    Some(task_fd_file(task, fd))
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::fs::anon_inode::alloc_anon_file;
    use crate::fs::dcache::d_alloc;
    use crate::fs::fdtable::FilesStruct;
    use crate::fs::file::{alloc_file, set_path_hint};
    use crate::fs::kernfs::lookup;
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

    /// Pseudo-filesystem fds must read back with their Linux `d_dname`
    /// spelling, not as `/`.
    ///
    /// test-origin: linux:vendor/linux/net/socket.c::sockfs_dname,
    /// linux:vendor/linux/fs/pipe.c::pipefs_dname,
    /// linux:vendor/linux/fs/anon_inodes.c::anon_inodefs_dname
    #[test]
    fn proc_fd_links_report_linux_pseudo_filesystem_names() {
        let sockfs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/socket.c"
        ));
        assert!(sockfs.contains("\"socket:[%llu]\""));
        let pipefs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/pipe.c"
        ));
        assert!(pipefs.contains("\"pipe:[%llu]\""));
        let anonfs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/anon_inodes.c"
        ));
        assert!(anonfs.contains("\"anon_inode:%s\""));

        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 4410;
        current.tgid = 4410;
        current.cred = &raw const INIT_CRED;

        unsafe {
            let fdt = FilesStruct::new();
            let sock = crate::fs::anon_inode::alloc_anon_file_with_ino(
                "socket",
                &NOOP_FILE_OPS,
                0,
                4242,
            );
            assert_eq!(fdt.install_at_or_above(sock, 3, false), Ok(3));
            // Same dentry name pipe.rs gives its read end.
            let pipe_r = alloc_anon_file("pipe:[read]", &NOOP_FILE_OPS, 0);
            assert_eq!(fdt.install_at_or_above(pipe_r, 4, false), Ok(4));
            let ev = alloc_anon_file("[eventfd]", &NOOP_FILE_OPS, 0);
            assert_eq!(fdt.install_at_or_above(ev, 5, false), Ok(5));
            files::set_task_files(&mut *current as *mut TaskStruct, fdt);
            sched::set_current(&mut *current as *mut TaskStruct);

            let task = &mut *current as *mut TaskStruct;
            assert_eq!(task_fd_path(task, 3).unwrap(), "socket:[4242]");
            assert!(
                task_fd_path(task, 4).unwrap().starts_with("pipe:["),
                "pipe fd read back as {:?}",
                task_fd_path(task, 4)
            );
            assert_eq!(task_fd_path(task, 5).unwrap(), "anon_inode:[eventfd]");

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    /// A cached `/proc/self/fd/N` inode must follow whoever is reading it.
    ///
    /// Linux regenerates `/proc/self` from `current` on every path walk
    /// (`vendor/linux/fs/proc/self.c::proc_self_get_link`), so `proc_fd_link()`
    /// always reads the calling task's file table.  Lupos shares one cached
    /// `self/fd/N` inode between processes, so it must not bind to the pid that
    /// instantiated it: bubblewrap canonicalizes its `O_PATH` bind destination
    /// through `readlink("/proc/self/fd/N")`, and a stale pid there made every
    /// sandboxed glycin loader fail with ENOENT.
    #[test]
    fn proc_self_fd_link_follows_the_reading_task_not_the_one_that_cached_it() {
        let previous = unsafe { sched::get_current() };

        let mut first = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        first.pid = 4401;
        first.tgid = 4401;
        first.cred = &raw const INIT_CRED;

        let mut second = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        second.pid = 4402;
        second.tgid = 4402;
        second.cred = &raw const INIT_CRED;

        unsafe {
            let fdt = FilesStruct::new();
            let held = alloc_anon_file("held", &NOOP_FILE_OPS, 0);
            set_path_hint(&held, String::from("/newroot"));
            assert_eq!(fdt.install_at_or_above(held, 4, false), Ok(4));
            files::set_task_files(&mut *first as *mut TaskStruct, fdt);
            sched::set_current(&mut *first as *mut TaskStruct);

            let sb = SuperBlock::alloc("proc", 0x9fa0, &crate::fs::proc::PROCFS_SUPER_OPS);
            let self_dir = KernfsNode::new_dir("self", 0o555);
            crate::fs::proc::base::add_tgid_base(&self_dir);
            let fd_dir = lookup(&self_dir, "fd").expect("/proc/self/fd");
            let dir_inode = crate::fs::kernfs::inode_for_node(&sb, fd_dir);

            // The first task instantiates (and caches) the fd symlink inode.
            let link = fd_dir_lookup(&dir_inode, "4").expect("fd link");
            let mut buf = [0u8; 64];
            let n = proc_fd_readlink(&link, &mut buf).expect("readlink");
            assert_eq!(&buf[..n], b"/newroot");

            // The first task goes away, exactly as a finished bwrap child does.
            files::drop_task_files(&mut *first as *mut TaskStruct);

            // A different task reads the same cached inode.  It must see its
            // own fd 4, not ENOENT from a lookup of the departed pid.
            let fdt = FilesStruct::new();
            let held = alloc_anon_file("held", &NOOP_FILE_OPS, 0);
            set_path_hint(&held, String::from("/newroot/usr"));
            assert_eq!(fdt.install_at_or_above(held, 4, false), Ok(4));
            files::set_task_files(&mut *second as *mut TaskStruct, fdt);
            sched::set_current(&mut *second as *mut TaskStruct);

            let mut buf = [0u8; 64];
            let n = proc_fd_readlink(&link, &mut buf).expect("readlink after task switch");
            assert_eq!(&buf[..n], b"/newroot/usr");

            files::drop_task_files(&mut *second as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn proc_numeric_pid_fd_path_resolves_for_current_task() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 434;
        current.tgid = 434;
        current.cred = &raw const INIT_CRED;

        unsafe {
            let fdt = FilesStruct::new();
            let held = alloc_anon_file("held", &NOOP_FILE_OPS, 0);
            set_path_hint(&held, String::from("/newroot/usr"));
            assert_eq!(fdt.install_at_or_above(held, 5, false), Ok(5));
            files::set_task_files(&mut *current as *mut TaskStruct, fdt);
            sched::set_current(&mut *current as *mut TaskStruct);

            // The numeric-pid spelling of the caller's own fd resolves like
            // /proc/self/fd (gdk-pixbuf 2.44 readlinks "/proc/%u/fd/%d").
            assert_eq!(
                current_fd_path_from_proc_path("/proc/434/fd/5"),
                Some(Ok(String::from("/newroot/usr")))
            );
            assert_eq!(
                current_fd_path_from_proc_path("/proc/self/fd/5"),
                Some(Ok(String::from("/newroot/usr")))
            );
            assert_eq!(current_fd_path_from_proc_path("/proc/self/fd/"), None);
            assert!(current_fd_file_from_proc_path("/proc/self/fd/").is_none());
            // A numeric proc-fd path with no live task now fails like procfs
            // lookup instead of silently falling through as a non-proc path.
            assert_eq!(
                current_fd_path_from_proc_path("/proc/435/fd/5"),
                Some(Err(ENOENT))
            );
            // Unknown fds under the caller's own pid report EBADF, matching
            // the existing /proc/self/fd shortcut behaviour.
            assert_eq!(
                current_fd_path_from_proc_path("/proc/434/fd/6"),
                Some(Err(EBADF))
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
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
            let self_dir = KernfsNode::new_dir("self", 0o555);
            crate::fs::proc::base::add_tgid_base(&self_dir);
            let fd_dir = lookup(&self_dir, "fd").expect("/proc/self/fd");
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
