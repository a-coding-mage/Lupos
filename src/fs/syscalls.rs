//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs
//! VFS syscall glue for the filesystem ABI closure.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{
    E2BIG, EACCES, EBADF, EBUSY, EEXIST, EFAULT, EINVAL, EIO, EISDIR, ENODATA, ENODEV, ENOENT,
    ENOSYS, ENOTDIR, ENOTEMPTY, ENOTTY, EPERM, ERANGE, EXDEV,
};
use crate::include::uapi::fcntl::{
    AT_EMPTY_PATH, AT_FDCWD, AT_NO_AUTOMOUNT, AT_RECURSIVE, AT_REMOVEDIR, AT_SYMLINK_FOLLOW,
    AT_SYMLINK_NOFOLLOW, O_CLOEXEC, O_CREAT, O_PATH, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY,
};
use crate::include::uapi::mount::{
    FSCONFIG_CMD_CREATE, FSCONFIG_CMD_CREATE_EXCL, FSCONFIG_CMD_RECONFIGURE, FSCONFIG_SET_FLAG,
    FSCONFIG_SET_STRING, FSMOUNT_CLOEXEC, FSMOUNT_NAMESPACE, FSOPEN_CLOEXEC, MNT_DETACH,
    MNT_EXPIRE, MNT_FORCE, MOUNT_ATTR__ATIME, MOUNT_ATTR_NOATIME, MOUNT_ATTR_NODEV,
    MOUNT_ATTR_NODIRATIME, MOUNT_ATTR_NOEXEC, MOUNT_ATTR_NOSUID, MOUNT_ATTR_NOSYMFOLLOW,
    MOUNT_ATTR_RDONLY, MOUNT_ATTR_SIZE_VER0, MOUNT_ATTR_STRICTATIME, MOUNT_ATTR_SUPPORTED,
    MOVE_MOUNT_F_EMPTY_PATH, MOVE_MOUNT_MASK, MS_NOATIME, MS_NODEV, MS_NODIRATIME, MS_NOEXEC,
    MS_NOSUID, MS_NOSYMFOLLOW, MS_RDONLY, MS_STRICTATIME, OPEN_TREE_CLOEXEC, OPEN_TREE_CLONE,
    OPEN_TREE_NAMESPACE, UMOUNT_NOFOLLOW,
};
use crate::include::uapi::openat2::OpenHow;
use crate::include::uapi::stat::{S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFMT, S_IFREG, S_IFSOCK};
use crate::kernel::capability::{CAP_DAC_READ_SEARCH, CAP_SYS_ADMIN, CAP_SYS_CHROOT, capable};
use crate::kernel::{files, sched};

use super::mount;
use super::namei::{LookupCtx, path_lookupat};
use super::openat::{do_openat2, do_openat2_with_hint, sys_openat};
use super::ops::{NOOP_FILE_OPS, NOOP_INODE_OPS};
use super::permission::check_inode_write_permission;
use super::read_write::{vfs_fsync, vfs_lseek, vfs_read, vfs_write};
use super::select;
pub use super::select::PollFd;
use super::types::{DentryRef, FileRef, Inode, InodeKind, InodePrivate, InodeRef};

pub const SEEK_SET: i32 = 0;
pub const POLLIN: i16 = select::POLLIN;
pub const POLLOUT: i16 = select::POLLOUT;
pub const POLLERR: i16 = select::POLLERR;

const AT_STATX_FORCE_SYNC: u32 = 0x2000;
const AT_STATX_DONT_SYNC: u32 = 0x4000;
const AT_STATX_SYNC_TYPE: u32 = AT_STATX_FORCE_SYNC | AT_STATX_DONT_SYNC;

const STATX_TYPE: u32 = 0x0000_0001;
const STATX_MODE: u32 = 0x0000_0002;
const STATX_NLINK: u32 = 0x0000_0004;
const STATX_UID: u32 = 0x0000_0008;
const STATX_GID: u32 = 0x0000_0010;
const STATX_ATIME: u32 = 0x0000_0020;
const STATX_MTIME: u32 = 0x0000_0040;
const STATX_CTIME: u32 = 0x0000_0080;
const STATX_INO: u32 = 0x0000_0100;
const STATX_SIZE: u32 = 0x0000_0200;
const STATX_BLOCKS: u32 = 0x0000_0400;
const STATX_BASIC_STATS: u32 = STATX_TYPE
    | STATX_MODE
    | STATX_NLINK
    | STATX_UID
    | STATX_GID
    | STATX_ATIME
    | STATX_MTIME
    | STATX_CTIME
    | STATX_INO
    | STATX_SIZE
    | STATX_BLOCKS;
const STATX_MNT_ID: u32 = 0x0000_1000;
const STATX_MNT_ID_UNIQUE: u32 = 0x0000_4000;
const STATX_RESERVED: u32 = 0x8000_0000;
const STATX_SUPPORTED: u32 = STATX_BASIC_STATS | STATX_MNT_ID;
const STATX_ATTR_MOUNT_ROOT: u64 = 0x0000_2000;
const CLOSE_RANGE_UNSHARE: u32 = 1 << 1;
const CLOSE_RANGE_CLOEXEC: u32 = 1 << 2;
const RENAME_NOREPLACE: u32 = 1 << 0;
const RENAME_EXCHANGE: u32 = 1 << 1;
const RENAME_WHITEOUT: u32 = 1 << 2;
const ERESTARTNOHAND: i32 = 514;

/// Sixth argument to pselect6(2). Linux packs the sigset pointer and size
/// because the native syscall ABI only has six argument registers.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PselectSigsetArg {
    sigmask: *const crate::kernel::signal::SigSet,
    sigsetsize: usize,
}

fn statx_result_mask(request_mask: u32) -> u32 {
    let mnt_id = if request_mask & STATX_MNT_ID_UNIQUE != 0 {
        STATX_MNT_ID_UNIQUE
    } else {
        STATX_MNT_ID
    };
    (request_mask & (STATX_SUPPORTED & !STATX_MNT_ID)) | mnt_id
}

fn statx_mount_id(mount_id: u64, request_mask: u32) -> u64 {
    if request_mask & STATX_MNT_ID_UNIQUE != 0 {
        mount_id | (1u64 << 32)
    } else {
        mount_id
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: i32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub __unused: [i64; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxStatFs {
    pub f_type: i64,
    pub f_bsize: i64,
    pub f_blocks: i64,
    pub f_bfree: i64,
    pub f_bavail: i64,
    pub f_files: i64,
    pub f_ffree: i64,
    pub f_fsid: [i32; 2],
    pub f_namelen: i64,
    pub f_frsize: i64,
    pub f_flags: i64,
    pub f_spare: [i64; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxStatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __reserved: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxStatx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    pub __spare0: u16,
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: LinuxStatxTimestamp,
    pub stx_btime: LinuxStatxTimestamp,
    pub stx_ctime: LinuxStatxTimestamp,
    pub stx_mtime: LinuxStatxTimestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub stx_mnt_id: u64,
    pub stx_dio_mem_align: u32,
    pub stx_dio_offset_align: u32,
    pub __spare3: [u64; 12],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoVec {
    pub iov_base: *mut u8,
    pub iov_len: usize,
}

fn current_files() -> Result<alloc::sync::Arc<super::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn current_mm_mut() -> Option<&'static mut crate::mm::mm_types::MmStruct> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return None;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        None
    } else {
        Some(unsafe { &mut *mm })
    }
}

fn dirfd_base_hint(dirfd: i32, start: &DentryRef) -> Result<Option<String>, i32> {
    if dirfd == AT_FDCWD {
        return Ok(None);
    }
    if dirfd < 0 {
        return Err(EBADF);
    }
    let file = current_files()?.get(dirfd)?;
    Ok(super::file::path_hint(&file).or_else(|| mount::path_for_dentry(start)))
}

fn root_and_start(dirfd: i32) -> Result<(DentryRef, DentryRef), i32> {
    let root_mnt = mount::rootfs().ok_or(EINVAL)?;
    if dirfd == AT_FDCWD {
        if let Some((root, pwd)) = super::fs_struct::current_root_and_pwd() {
            return Ok((root, pwd));
        }
        let root = root_mnt.root.clone();
        return Ok((root.clone(), root));
    }
    let root = super::fs_struct::current_root_and_pwd()
        .map(|(root, _)| root)
        .unwrap_or_else(|| root_mnt.root.clone());
    let file = current_files()?.get(dirfd)?;
    let start = mount::mounted_root_for_dentry(&file.dentry)
        .map(|mnt| mnt.root.clone())
        .unwrap_or_else(|| file.dentry.clone());
    Ok((root, start))
}

unsafe fn user_path(path: *const u8) -> Result<String, i32> {
    if path.is_null() {
        return Err(EFAULT);
    }
    const PATH_MAX: usize = 4096;
    let mut buf = alloc::vec![0u8; PATH_MAX];
    let n = unsafe { uaccess::strncpy_from_user(buf.as_mut_ptr(), path, buf.len()) };
    if n < 0 {
        return Err((-n) as i32);
    }
    core::str::from_utf8(trim_copied_user_path(&buf[..n as usize]))
        .map(String::from)
        .map_err(|_| EINVAL)
}

fn trim_copied_user_path(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|b| *b == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}

fn lookup_path(dirfd: i32, pathname: *const u8) -> Result<DentryRef, i32> {
    lookup_path_with_follow(dirfd, pathname, true)
}

fn lookup_path_with_follow(
    dirfd: i32,
    pathname: *const u8,
    follow_final: bool,
) -> Result<DentryRef, i32> {
    let path = unsafe { user_path(pathname) }?;
    if path.is_empty() {
        return Err(ENOENT);
    }
    lookup_path_str_with_follow(dirfd, &path, follow_final).map(|target| target.dentry)
}

struct StatTarget {
    dentry: DentryRef,
    mount: Arc<mount::Mount>,
}

fn stat_target_from_dentry(dentry: DentryRef) -> Result<StatTarget, i32> {
    if let Some(mount) = mount::mounted_root_for_dentry(&dentry) {
        return Ok(StatTarget {
            dentry: mount.root.clone(),
            mount,
        });
    }
    let mount = mount::containing_mount_for_dentry(&dentry)
        .or_else(mount::rootfs)
        .ok_or(EINVAL)?;
    Ok(StatTarget { dentry, mount })
}

fn lookup_path_str(dirfd: i32, path: &str) -> Result<StatTarget, i32> {
    lookup_path_str_with_follow(dirfd, path, false)
}

fn lookup_path_str_with_follow(
    dirfd: i32,
    path: &str,
    follow_final: bool,
) -> Result<StatTarget, i32> {
    if path.starts_with('/') || dirfd == AT_FDCWD {
        let (mount, dentry) = if follow_final {
            mount::resolve_path_follow(path)?
        } else {
            mount::resolve_path_nofollow(path)?
        };
        return Ok(StatTarget { dentry, mount });
    }

    if dirfd < 0 {
        return Err(EBADF);
    }
    let file = current_files()?.get(dirfd)?;
    let start = mount::mounted_root_for_dentry(&file.dentry)
        .map(|mount| mount.root.clone())
        .unwrap_or_else(|| file.dentry.clone());
    if let Some(base) = super::file::path_hint(&file).or_else(|| mount::path_for_dentry(&start)) {
        let joined = join_path(&base, path);
        let (mount, dentry) = if follow_final {
            mount::resolve_path_follow(&joined)?
        } else {
            mount::resolve_path_nofollow(&joined)?
        };
        return Ok(StatTarget { dentry, mount });
    }

    let root_mount = mount::rootfs().ok_or(EINVAL)?;
    let dentry = path_lookupat(&LookupCtx::new(root_mount.root.clone(), start, 0), path)?;
    let mount = mount::containing_mount_for_dentry(&dentry).unwrap_or_else(|| root_mount.clone());
    let target = if follow_final {
        stat_target_from_dentry(dentry)?
    } else {
        StatTarget { dentry, mount }
    };
    Ok(target)
}

fn lookup_empty_stat_target(dirfd: i32) -> Result<StatTarget, i32> {
    if dirfd == AT_FDCWD {
        let mount = mount::rootfs().ok_or(EINVAL)?;
        return Ok(StatTarget {
            dentry: mount.root.clone(),
            mount,
        });
    }
    let file = current_files()?.get(dirfd)?;
    stat_target_from_dentry(file.dentry.clone())
}

unsafe fn lookup_path_or_empty_target(
    dirfd: i32,
    pathname: *const u8,
    follow_final: bool,
    allow_empty: bool,
) -> Result<StatTarget, i32> {
    if pathname.is_null() {
        return Err(EFAULT);
    }
    let path = unsafe { user_path(pathname) }?;
    if path.is_empty() {
        if !allow_empty {
            trace_run_path("lookup-empty", dirfd, &path, 0, -(ENOENT as i64));
            return Err(ENOENT);
        }
        let result = lookup_empty_stat_target(dirfd);
        trace_run_path(
            "lookup-empty",
            dirfd,
            &path,
            0,
            result.as_ref().map(|_| 0).unwrap_or_else(|e| -(*e as i64)),
        );
        return result;
    }
    let result = lookup_path_str_with_follow(dirfd, &path, follow_final);
    trace_run_path(
        "lookup",
        dirfd,
        &path,
        if follow_final { 1 } else { 0 },
        result.as_ref().map(|_| 0).unwrap_or_else(|e| -(*e as i64)),
    );
    result
}

fn join_path(base: &str, child: &str) -> String {
    if child.is_empty() || child == "." {
        return String::from(base);
    }
    if child.starts_with('/') {
        return String::from(child);
    }
    if base == "/" {
        let mut joined = String::from("/");
        joined.push_str(child);
        return joined;
    }
    let mut joined = String::from(base);
    joined.push('/');
    joined.push_str(child);
    joined
}

#[cfg(not(test))]
fn trace_run_path(op: &str, dirfd: i32, path: &str, flags: i32, ret: i64) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    let interesting_empty = path.is_empty()
        && matches!(
            op,
            "lookup-empty" | "fchmodat" | "fchownat" | "fchmodat-old"
        );
    if !(interesting_empty
        || path.contains("run")
        || path.contains("systemd")
        || path.contains("journal")
        || path.contains("netif")
        || path.contains("resolve"))
    {
        return;
    }
    let task = unsafe { sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-run-{} pid={} dirfd={} flags={:#x} path={} ret={}",
        op,
        pid,
        dirfd,
        flags,
        path,
        ret
    );
}

#[cfg(not(test))]
fn trace_run_symlinkat(dirfd: i32, path: &str, target: &str, parent_ops: &str, ret: i64) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    if !(ret == -(ENOSYS as i64)
        || path.contains("run")
        || path.contains("systemd")
        || path.contains("journal")
        || path.contains("udev")
        || path.starts_with("/dev")
        || path.starts_with("/proc")
        || path.starts_with("/sys"))
    {
        return;
    }
    let task = unsafe { sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-run-symlinkat pid={} dirfd={} path={} target={} parent_ops={} ret={}",
        pid,
        dirfd,
        path,
        target,
        parent_ops,
        ret
    );
}

#[cfg(not(test))]
fn trace_run_fd(op: &str, fd: i32, name: &str, flags: u32, ret: i64) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    let task = unsafe { sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    let logind_getdents = if task.is_null() || op != "getdents64" {
        false
    } else {
        let comm = unsafe { &(*task).comm };
        comm.starts_with(b"systemd-logind")
    };
    if !logind_getdents && !matches!(name, "journal" | "netif" | "resolve" | "credentials") {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-run-{} pid={} fd={} flags={:#x} name={} ret={}",
        op,
        pid,
        fd,
        flags,
        name,
        ret
    );
}

#[cfg(not(test))]
fn trace_xattr_path(op: &str, dirfd: i32, path: &str, ret: i64) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    if !(path.is_empty()
        || path.starts_with("/proc/self/fd/")
        || path.starts_with("/dev/fd/")
        || path.contains("run/systemd")
        || path.contains("systemd/journal")
        || path.contains("systemd/netif")
        || path.contains("systemd/resolve"))
    {
        return;
    }
    let task = unsafe { sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-run-{} pid={} dirfd={} path={} ret={}",
        op,
        pid,
        dirfd,
        path,
        ret
    );
}

#[cfg(test)]
fn trace_run_path(_op: &str, _dirfd: i32, _path: &str, _flags: i32, _ret: i64) {}

#[cfg(test)]
fn trace_run_symlinkat(_dirfd: i32, _path: &str, _target: &str, _parent_ops: &str, _ret: i64) {}

#[cfg(test)]
fn trace_run_fd(_op: &str, _fd: i32, _name: &str, _flags: u32, _ret: i64) {}

#[cfg(test)]
fn trace_xattr_path(_op: &str, _dirfd: i32, _path: &str, _ret: i64) {}

fn copy_ptr_from_user<T: Copy>(src: *const T) -> Result<T, i32> {
    if src.is_null() {
        return Err(EFAULT);
    }
    let mut out = core::mem::MaybeUninit::<T>::uninit();
    let not_copied = unsafe {
        uaccess::copy_from_user(
            out.as_mut_ptr().cast::<u8>(),
            src.cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(unsafe { out.assume_init() })
}

fn copy_ptr_to_user<T: Copy>(dst: *mut T, value: &T) -> Result<(), i32> {
    if dst.is_null() {
        return Err(EFAULT);
    }
    let not_copied = unsafe {
        uaccess::copy_to_user(
            dst.cast::<u8>(),
            (value as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if not_copied != 0 { Err(EFAULT) } else { Ok(()) }
}

fn stat_from_inode(inode: &InodeRef) -> LinuxStat {
    let st = super::stat::vfs_getattr(inode);
    LinuxStat {
        st_dev: st.dev,
        st_ino: st.ino,
        st_nlink: st.nlink,
        st_mode: st.mode,
        st_uid: st.uid,
        st_gid: st.gid,
        st_rdev: st.rdev,
        st_size: st.size,
        st_blksize: st.blksize,
        st_blocks: st.blocks,
        st_atime: st.atime,
        st_atime_nsec: st.atime_nsec,
        st_mtime: st.mtime,
        st_mtime_nsec: st.mtime_nsec,
        st_ctime: st.ctime,
        st_ctime_nsec: st.ctime_nsec,
        ..LinuxStat::default()
    }
}

fn write_stat(out: *mut LinuxStat, inode: &InodeRef) -> i64 {
    let stat = stat_from_inode(inode);
    copy_ptr_to_user(out, &stat)
        .map(|()| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub unsafe fn sys_open(filename: *const u8, flags: i32, mode: u32) -> i64 {
    unsafe { sys_openat(AT_FDCWD, filename, flags, mode) }
}

pub unsafe fn sys_openat2(
    dirfd: i32,
    pathname: *const u8,
    how: *const OpenHow,
    size: usize,
) -> i64 {
    if how.is_null() || size < core::mem::size_of::<OpenHow>() {
        return -(EINVAL as i64);
    }
    let Ok((root, start)) = root_and_start(dirfd) else {
        return -(EBADF as i64);
    };
    let path = match unsafe { user_path(pathname) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    let how = match copy_ptr_from_user(how) {
        Ok(how) => how,
        Err(errno) => return -(errno as i64),
    };
    let hinted_path = if path.starts_with('/') || dirfd == AT_FDCWD {
        Some(super::fs_struct::absolute_from_cwd(&path))
    } else {
        dirfd_base_hint(dirfd, &start)
            .ok()
            .flatten()
            .map(|base| join_path(&base, &path))
    };
    let opened = match do_openat2_with_hint(root, start, &path, &how, hinted_path.as_deref()) {
        Ok(opened) => opened,
        Err(errno) => return -(errno as i64),
    };
    match current_files() {
        Ok(ft) => match ft.install(opened.file, opened.cloexec) {
            Ok(fd) => {
                if let Ok(file) = ft.get(fd) {
                    super::file::note_file_access_for_integrity(None, &file);
                }
                fd as i64
            }
            Err(errno) => -(errno as i64),
        },
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_fstat(fd: i32, out: *mut LinuxStat) -> i64 {
    let ret = (|| {
        let file = match current_files().and_then(|ft| ft.get(fd)) {
            Ok(file) => file,
            Err(errno) => return -(errno as i64),
        };
        let inode = match file.inode() {
            Some(inode) => inode,
            None => return -(EBADF as i64),
        };
        let ret = write_stat(out, &inode);
        trace_run_fd(
            "fstat",
            fd,
            &file.dentry.name,
            file.flags.load(Ordering::Acquire),
            ret,
        );
        ret
    })();
    ret
}

pub unsafe fn sys_stat(pathname: *const u8, out: *mut LinuxStat) -> i64 {
    let dentry = match lookup_path(AT_FDCWD, pathname) {
        Ok(dentry) => dentry,
        Err(errno) => return -(errno as i64),
    };
    match dentry.inode() {
        Some(inode) => write_stat(out, &inode),
        None => -(ENOENT as i64),
    }
}

pub unsafe fn sys_lstat(pathname: *const u8, out: *mut LinuxStat) -> i64 {
    let dentry = match lookup_path_with_follow(AT_FDCWD, pathname, false) {
        Ok(dentry) => dentry,
        Err(errno) => return -(errno as i64),
    };
    match dentry.inode() {
        Some(inode) => write_stat(out, &inode),
        None => -(ENOENT as i64),
    }
}

pub unsafe fn sys_newfstatat(
    dirfd: i32,
    pathname: *const u8,
    out: *mut LinuxStat,
    flags: i32,
) -> i64 {
    let allowed = (AT_SYMLINK_NOFOLLOW | AT_EMPTY_PATH | AT_NO_AUTOMOUNT) as i32;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }
    let mut trace_path: Option<String> = None;
    let ret = (|| {
        let target = if pathname.is_null() {
            if flags & AT_EMPTY_PATH as i32 == 0 {
                return -(EFAULT as i64);
            }
            match lookup_empty_stat_target(dirfd) {
                Ok(target) => target,
                Err(errno) => return -(errno as i64),
            }
        } else {
            let path = match unsafe { user_path(pathname) } {
                Ok(path) => path,
                Err(errno) => return -(errno as i64),
            };
            trace_path = Some(path.clone());
            if path.is_empty() {
                if flags & AT_EMPTY_PATH as i32 == 0 {
                    return -(ENOENT as i64);
                }
                match lookup_empty_stat_target(dirfd) {
                    Ok(target) => target,
                    Err(errno) => return -(errno as i64),
                }
            } else {
                let follow_final = flags & AT_SYMLINK_NOFOLLOW as i32 == 0;
                match lookup_path_str_with_follow(dirfd, &path, follow_final) {
                    Ok(target) => target,
                    Err(errno) => return -(errno as i64),
                }
            }
        };
        match target.dentry.inode() {
            Some(inode) => write_stat(out, &inode),
            None => -(ENOENT as i64),
        }
    })();
    if let Some(path) = trace_path.as_deref() {
        trace_run_path("newfstatat", dirfd, path, flags, ret);
    }
    ret
}

pub unsafe fn sys_statx(
    dirfd: i32,
    pathname: *const u8,
    flags: i32,
    mask: u32,
    out: *mut LinuxStatx,
) -> i64 {
    if out.is_null() {
        return -(EFAULT as i64);
    }
    if mask & STATX_RESERVED != 0 {
        return -(EINVAL as i64);
    }
    let flags_u = flags as u32;
    let allowed = AT_SYMLINK_NOFOLLOW | AT_NO_AUTOMOUNT | AT_EMPTY_PATH | AT_STATX_SYNC_TYPE;
    if flags_u & !allowed != 0 || flags_u & AT_STATX_SYNC_TYPE == AT_STATX_SYNC_TYPE {
        return -(EINVAL as i64);
    }

    let mut trace_path: Option<String> = None;
    let ret = (|| {
        let target = if pathname.is_null() {
            if flags_u & AT_EMPTY_PATH == 0 {
                return -(EFAULT as i64);
            }
            match lookup_empty_stat_target(dirfd) {
                Ok(target) => target,
                Err(errno) => return -(errno as i64),
            }
        } else {
            let path = match unsafe { user_path(pathname) } {
                Ok(path) => path,
                Err(errno) => return -(errno as i64),
            };
            trace_path = Some(path.clone());
            if path.is_empty() {
                if flags_u & AT_EMPTY_PATH == 0 {
                    return -(ENOENT as i64);
                }
                match lookup_empty_stat_target(dirfd) {
                    Ok(target) => target,
                    Err(errno) => return -(errno as i64),
                }
            } else {
                let follow_final = flags_u & AT_SYMLINK_NOFOLLOW == 0;
                match lookup_path_str_with_follow(dirfd, &path, follow_final) {
                    Ok(target) => target,
                    Err(errno) => return -(errno as i64),
                }
            }
        };
        let Some(inode) = target.dentry.inode() else {
            return -(ENOENT as i64);
        };
        let st = stat_from_inode(&inode);
        let result_mask = statx_result_mask(mask);
        let is_mount_root = alloc::sync::Arc::ptr_eq(&target.dentry, &target.mount.root);
        let statx = LinuxStatx {
            stx_mask: result_mask,
            stx_blksize: st.st_blksize as u32,
            stx_attributes: if is_mount_root {
                STATX_ATTR_MOUNT_ROOT
            } else {
                0
            },
            stx_nlink: st.st_nlink as u32,
            stx_uid: st.st_uid,
            stx_gid: st.st_gid,
            stx_mode: st.st_mode as u16,
            stx_ino: st.st_ino,
            stx_size: st.st_size as u64,
            stx_blocks: st.st_blocks as u64,
            stx_attributes_mask: STATX_ATTR_MOUNT_ROOT,
            stx_atime: LinuxStatxTimestamp {
                tv_sec: st.st_atime,
                tv_nsec: st.st_atime_nsec as u32,
                __reserved: 0,
            },
            stx_ctime: LinuxStatxTimestamp {
                tv_sec: st.st_ctime,
                tv_nsec: st.st_ctime_nsec as u32,
                __reserved: 0,
            },
            stx_mtime: LinuxStatxTimestamp {
                tv_sec: st.st_mtime,
                tv_nsec: st.st_mtime_nsec as u32,
                __reserved: 0,
            },
            stx_dev_major: ((st.st_dev >> 8) & 0xfff) as u32,
            stx_dev_minor: (st.st_dev & 0xff) as u32,
            // Device number of a char/block special file (`new_encode_dev` form).
            // Zero for other inode kinds. Userspace (e.g. Xorg `xf86HasTTYs()`)
            // keys behaviour off `major(st_rdev)`.
            stx_rdev_major: ((st.st_rdev >> 8) & 0xfff) as u32,
            stx_rdev_minor: (st.st_rdev & 0xff) as u32,
            stx_mnt_id: statx_mount_id(target.mount.id, mask),
            ..LinuxStatx::default()
        };
        copy_ptr_to_user(out, &statx)
            .map(|()| 0)
            .unwrap_or_else(|errno| -(errno as i64))
    })();
    if let Some(path) = trace_path.as_deref() {
        trace_run_path("statx", dirfd, path, flags, ret);
    }
    ret
}

pub unsafe fn sys_lseek(fd: i32, offset: i64, whence: i32) -> i64 {
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    match vfs_lseek(&file, offset, whence) {
        Ok(pos) => pos as i64,
        Err(errno) => -(errno as i64),
    }
}

fn positioned_io(fd: i32, buf: *mut u8, count: usize, offset: i64, write: bool) -> i64 {
    if count == 0 {
        return 0;
    }
    if buf.is_null() || offset < 0 {
        return -(if buf.is_null() { EFAULT } else { EINVAL } as i64);
    }
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let old_pos = *file.pos.lock();
    *file.pos.lock() = offset as u64;
    const CHUNK: usize = 4096;
    let mut done = 0usize;
    let mut remaining = count;
    let mut user = buf;

    let ret = loop {
        if remaining == 0 {
            break done as i64;
        }
        let this = remaining.min(CHUNK);
        if write {
            let mut kbuf = alloc::vec![0u8; this];
            let not_copied =
                unsafe { uaccess::copy_from_user(kbuf.as_mut_ptr(), user as *const u8, this) };
            let copied = this - not_copied;
            if copied == 0 {
                break if done > 0 {
                    done as i64
                } else {
                    -(EFAULT as i64)
                };
            }
            kbuf.truncate(copied);
            match vfs_write(&file, &kbuf) {
                Ok(n) => {
                    done += n;
                    unsafe {
                        user = user.add(n);
                    }
                    remaining -= n;
                    if n < copied {
                        break done as i64;
                    }
                }
                Err(errno) => {
                    break if done > 0 {
                        done as i64
                    } else {
                        -(errno as i64)
                    };
                }
            }
            if not_copied != 0 {
                break done as i64;
            }
        } else {
            let mut kbuf = alloc::vec![0u8; this];
            let n = match vfs_read(&file, &mut kbuf) {
                Ok(n) => n,
                Err(errno) => {
                    break if done > 0 {
                        done as i64
                    } else {
                        -(errno as i64)
                    };
                }
            };
            if n == 0 {
                break done as i64;
            }
            let not_copied = unsafe { uaccess::copy_to_user(user, kbuf.as_ptr(), n) };
            let copied = n - not_copied;
            done += copied;
            unsafe {
                user = user.add(copied);
            }
            remaining -= copied;
            if copied < n {
                break if done > 0 {
                    done as i64
                } else {
                    -(EFAULT as i64)
                };
            }
            if n < this {
                break done as i64;
            }
        }
    };
    *file.pos.lock() = old_pos;
    ret
}

pub unsafe fn sys_pread64(fd: i32, buf: *mut u8, count: usize, offset: i64) -> i64 {
    positioned_io(fd, buf, count, offset, false)
}

pub unsafe fn sys_pwrite64(fd: i32, buf: *const u8, count: usize, offset: i64) -> i64 {
    positioned_io(fd, buf as *mut u8, count, offset, true)
}

fn rw_iov(fd: i32, iov: *const IoVec, iovcnt: usize, write: bool, offset: Option<i64>) -> i64 {
    const UIO_MAXIOV: usize = 1024;
    if iovcnt > UIO_MAXIOV {
        return -(EINVAL as i64);
    }
    if iovcnt == 0 {
        return 0;
    }
    if iov.is_null() {
        return -(EFAULT as i64);
    }
    if write && offset.is_none() {
        let file = match current_files().and_then(|ft| ft.get(fd)) {
            Ok(file) => file,
            Err(errno) => return -(errno as i64),
        };
        if file.fops.name == "socket" {
            return write_socket_iov(&file, iov, iovcnt);
        }
    }
    let mut total = 0usize;
    let mut pos = offset.unwrap_or_default();
    for idx in 0..iovcnt {
        let entry = unsafe { *iov.add(idx) };
        if entry.iov_len == 0 {
            continue;
        }
        if entry.iov_base.is_null() {
            return if total > 0 {
                total as i64
            } else {
                -(EFAULT as i64)
            };
        }
        let ret = match offset {
            Some(_) if write => unsafe {
                sys_pwrite64(fd, entry.iov_base as *const u8, entry.iov_len, pos)
            },
            Some(_) => unsafe { sys_pread64(fd, entry.iov_base, entry.iov_len, pos) },
            None if write => unsafe {
                super::read_write::sys_write(fd, entry.iov_base, entry.iov_len)
            },
            None => unsafe { super::read_write::sys_read(fd, entry.iov_base, entry.iov_len) },
        };
        if ret < 0 {
            return if total > 0 { total as i64 } else { ret };
        }
        let done = ret as usize;
        total = total.saturating_add(done);
        pos = pos.saturating_add(ret);
        if done < entry.iov_len {
            break;
        }
    }
    total as i64
}

fn write_socket_iov(file: &FileRef, iov: *const IoVec, iovcnt: usize) -> i64 {
    const MAX_RW_COUNT: usize = 0x7fff_f000;
    let mut kbuf = Vec::new();
    for idx in 0..iovcnt {
        let entry = unsafe { *iov.add(idx) };
        if entry.iov_len == 0 {
            continue;
        }
        if entry.iov_base.is_null() {
            return -(EFAULT as i64);
        }
        let Some(new_len) = kbuf.len().checked_add(entry.iov_len) else {
            return -(EINVAL as i64);
        };
        if new_len > MAX_RW_COUNT {
            return -(EINVAL as i64);
        }
        let start = kbuf.len();
        kbuf.resize(new_len, 0);
        let not_copied = unsafe {
            uaccess::copy_from_user(kbuf[start..].as_mut_ptr(), entry.iov_base, entry.iov_len)
        };
        if not_copied != 0 {
            return -(EFAULT as i64);
        }
    }
    if kbuf.is_empty() {
        return 0;
    }
    match vfs_write(file, &kbuf) {
        Ok(n) => n as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_readv(fd: i32, iov: *const IoVec, iovcnt: usize) -> i64 {
    rw_iov(fd, iov, iovcnt, false, None)
}

pub unsafe fn sys_writev(fd: i32, iov: *const IoVec, iovcnt: usize) -> i64 {
    rw_iov(fd, iov, iovcnt, true, None)
}

pub unsafe fn sys_preadv(fd: i32, iov: *const IoVec, iovcnt: usize, offset: i64) -> i64 {
    if offset < 0 {
        return -(EINVAL as i64);
    }
    rw_iov(fd, iov, iovcnt, false, Some(offset))
}

pub unsafe fn sys_pwritev(fd: i32, iov: *const IoVec, iovcnt: usize, offset: i64) -> i64 {
    if offset < 0 {
        return -(EINVAL as i64);
    }
    rw_iov(fd, iov, iovcnt, true, Some(offset))
}

pub unsafe fn sys_preadv2(
    fd: i32,
    iov: *const IoVec,
    iovcnt: usize,
    offset: i64,
    flags: i32,
) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    unsafe { sys_preadv(fd, iov, iovcnt, offset) }
}

pub unsafe fn sys_pwritev2(
    fd: i32,
    iov: *const IoVec,
    iovcnt: usize,
    offset: i64,
    flags: i32,
) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    unsafe { sys_pwritev(fd, iov, iovcnt, offset) }
}

pub unsafe fn sys_fdatasync(fd: i32) -> i64 {
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    if let Some(mm) = current_mm_mut() {
        let len = file
            .inode()
            .map(|inode| inode.size.load(Ordering::Acquire))
            .unwrap_or(0);
        if len != 0 {
            let file_ptr = Arc::as_ptr(&file) as usize;
            if let Err(errno) =
                unsafe { crate::mm::mmap::sync_shared_file_mapping(mm, file_ptr, 0, len) }
            {
                return -(errno as i64);
            }
            crate::mm::huge::mark_file_mapping_clean(file_ptr);
        }
    }
    vfs_fsync(&file).map(|_| 0).unwrap_or_else(|e| -(e as i64))
}

pub unsafe fn sys_fsync(fd: i32) -> i64 {
    unsafe { sys_fdatasync(fd) }
}

pub fn sys_sync() -> i64 {
    0
}

pub unsafe fn sys_syncfs(fd: i32) -> i64 {
    match current_files().and_then(|ft| ft.get(fd)) {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_sync_file_range(fd: i32, offset: i64, nbytes: i64, _flags: u32) -> i64 {
    if offset < 0 || nbytes < 0 {
        return -(EINVAL as i64);
    }
    match current_files().and_then(|ft| ft.get(fd)) {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    }
}

const FALLOC_FL_KEEP_SIZE: i32 = 0x01;
const FALLOC_FL_PUNCH_HOLE: i32 = 0x02;
const FALLOC_FL_ZERO_RANGE: i32 = 0x10;

pub unsafe fn sys_fallocate(fd: i32, mode: i32, offset: i64, len: i64) -> i64 {
    const SUPPORTED_FALLOC_MODES: i32 =
        FALLOC_FL_KEEP_SIZE | FALLOC_FL_PUNCH_HOLE | FALLOC_FL_ZERO_RANGE;
    if mode & !SUPPORTED_FALLOC_MODES != 0 || offset < 0 || len < 0 {
        return -(EINVAL as i64);
    }
    if mode & FALLOC_FL_PUNCH_HOLE != 0 && mode & FALLOC_FL_KEEP_SIZE == 0 {
        return -(EINVAL as i64);
    }
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let inode = match file.inode() {
        Some(inode) => inode,
        None => return -(EBADF as i64),
    };
    let end = match (offset as u64).checked_add(len as u64) {
        Some(end) => end,
        None => return -(EINVAL as i64),
    };
    let offset_usize = if offset as u64 > usize::MAX as u64 {
        return -(EINVAL as i64);
    } else {
        offset as usize
    };
    let len_usize = if len as u64 > usize::MAX as u64 {
        return -(EINVAL as i64);
    } else {
        len as usize
    };
    let keep_size = mode & FALLOC_FL_KEEP_SIZE != 0;
    let zeroing = mode & (FALLOC_FL_PUNCH_HOLE | FALLOC_FL_ZERO_RANGE) != 0;
    if file.fops.name == MEMFD_FILE_OPS.name {
        let id = match memfd_id(&file) {
            Ok(id) => id,
            Err(errno) => return -(errno as i64),
        };
        let ret = if zeroing {
            crate::mm::shmem::with_memfd_mut(id, |obj| {
                obj.zero_range(offset_usize, len_usize, keep_size)
            })
        } else {
            let new_len = if keep_size {
                inode.size.load(Ordering::Acquire) as usize
            } else if end > usize::MAX as u64 {
                return -(EINVAL as i64);
            } else {
                end as usize
            };
            crate::mm::shmem::with_memfd_mut(id, |obj| obj.resize(new_len))
        };
        match ret {
            Some(Ok(())) => {}
            Some(Err(errno)) => return -(errno as i64),
            None => return -(EBADF as i64),
        }
    }
    let ret = if matches!(&inode.private, super::types::InodePrivate::RamBytes(_)) {
        let result = if zeroing {
            super::libfs::ram_file_zero_range(&inode, offset as u64, len as u64, keep_size)
        } else if keep_size {
            Ok(())
        } else {
            super::libfs::ram_file_set_size(&inode, end)
        };
        match result {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        }
    } else {
        -(EINVAL as i64)
    };
    ret
}

pub unsafe fn sys_ftruncate(fd: i32, length: i64) -> i64 {
    let new_size = match crate::mm::backing_dev::truncate_new_size(length) {
        Ok(size) => size,
        Err(errno) => return -(errno as i64),
    };
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let inode = match file.inode() {
        Some(inode) => inode,
        None => return -(EBADF as i64),
    };
    if file.fops.name == MEMFD_FILE_OPS.name {
        let id = match memfd_id(&file) {
            Ok(id) => id,
            Err(errno) => return -(errno as i64),
        };
        let len = if new_size > usize::MAX as u64 {
            return -(EINVAL as i64);
        } else {
            new_size as usize
        };
        match crate::mm::shmem::with_memfd_mut(id, |obj| obj.resize(len)) {
            Some(Ok(())) => {}
            Some(Err(errno)) => return -(errno as i64),
            None => return -(EBADF as i64),
        }
    }
    if file.fops.name == SECRETMEM_FILE_OPS.name {
        let id = match secretmem_id(&file) {
            Ok(id) => id,
            Err(errno) => return -(errno as i64),
        };
        let len = if new_size > usize::MAX as u64 {
            return -(EINVAL as i64);
        } else {
            new_size as usize
        };
        match crate::mm::shmem::with_secretmem_mut(id, |obj| obj.len = len) {
            Some(()) => {}
            None => return -(EBADF as i64),
        }
    }
    match super::attr::notify_change(&inode, &super::attr::IAttr::size(new_size), false) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_truncate(pathname: *const u8, length: i64) -> i64 {
    let new_size = match crate::mm::backing_dev::truncate_new_size(length) {
        Ok(size) => size,
        Err(errno) => return -(errno as i64),
    };
    let dentry = match lookup_path(AT_FDCWD, pathname) {
        Ok(dentry) => dentry,
        Err(errno) => return -(errno as i64),
    };
    let inode = match dentry.inode() {
        Some(inode) => inode,
        None => return -(ENOENT as i64),
    };
    match super::attr::notify_change(&inode, &super::attr::IAttr::size(new_size), false) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

fn chmod_inode(inode: &InodeRef, mode: u32) -> Result<(), i32> {
    super::attr::notify_change(inode, &super::attr::IAttr::mode(mode & 0o7777), false)
}

fn chown_inode(inode: &InodeRef, uid: u32, gid: u32) -> Result<(), i32> {
    let mut attr = super::attr::IAttr::default();
    if uid != u32::MAX {
        attr.valid |= super::attr::ATTR_UID;
        attr.uid = uid;
    }
    if gid != u32::MAX {
        attr.valid |= super::attr::ATTR_GID;
        attr.gid = gid;
    }
    super::attr::notify_change(inode, &attr, false)
}

pub unsafe fn sys_fchmod(fd: i32, mode: u32) -> i64 {
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    match file.inode() {
        Some(inode) => match chmod_inode(&inode, mode) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
        None => -(EBADF as i64),
    }
}

pub unsafe fn sys_chmod(pathname: *const u8, mode: u32) -> i64 {
    unsafe { sys_fchmodat(AT_FDCWD, pathname, mode) }
}

pub unsafe fn sys_fchmodat(dirfd: i32, pathname: *const u8, mode: u32) -> i64 {
    let path = match unsafe { user_path(pathname) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    if path.is_empty() {
        trace_run_path("fchmodat-old", dirfd, &path, 0, -(EINVAL as i64));
        return -(EINVAL as i64);
    }
    unsafe { sys_fchmodat2(dirfd, pathname, mode, 0) }
}

pub unsafe fn sys_fchmodat2(dirfd: i32, pathname: *const u8, mode: u32, flags: i32) -> i64 {
    if flags & !((AT_SYMLINK_NOFOLLOW | AT_EMPTY_PATH) as i32) != 0 {
        return -(EINVAL as i64);
    }
    let trace_path = if pathname.is_null() {
        None
    } else {
        unsafe { user_path(pathname).ok() }
    };
    let follow_final = flags & (AT_SYMLINK_NOFOLLOW as i32) == 0;
    let ret = (|| {
        let target = match unsafe {
            lookup_path_or_empty_target(
                dirfd,
                pathname,
                follow_final,
                flags & AT_EMPTY_PATH as i32 != 0,
            )
        } {
            Ok(target) => target,
            Err(errno) => return -(errno as i64),
        };
        match target.dentry.inode() {
            Some(inode) => match chmod_inode(&inode, mode) {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            },
            None => -(ENOENT as i64),
        }
    })();
    if let Some(path) = trace_path.as_deref() {
        trace_run_path("fchmodat", dirfd, path, flags, ret);
    }
    ret
}

pub unsafe fn sys_fchown(fd: i32, uid: u32, gid: u32) -> i64 {
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    match file.inode() {
        Some(inode) => match chown_inode(&inode, uid, gid) {
            Ok(()) => 0,
            Err(errno) => -(errno as i64),
        },
        None => -(EBADF as i64),
    }
}

pub unsafe fn sys_chown(pathname: *const u8, uid: u32, gid: u32) -> i64 {
    unsafe { sys_fchownat(AT_FDCWD, pathname, uid, gid, 0) }
}

pub unsafe fn sys_lchown(pathname: *const u8, uid: u32, gid: u32) -> i64 {
    unsafe { sys_fchownat(AT_FDCWD, pathname, uid, gid, AT_SYMLINK_NOFOLLOW as i32) }
}

pub unsafe fn sys_fchownat(dirfd: i32, pathname: *const u8, uid: u32, gid: u32, flags: i32) -> i64 {
    if flags & !((AT_SYMLINK_NOFOLLOW | AT_EMPTY_PATH) as i32) != 0 {
        return -(EINVAL as i64);
    }
    let trace_path = if pathname.is_null() {
        None
    } else {
        unsafe { user_path(pathname).ok() }
    };
    let follow_final = flags & (AT_SYMLINK_NOFOLLOW as i32) == 0;
    let ret = (|| {
        let target = match unsafe {
            lookup_path_or_empty_target(
                dirfd,
                pathname,
                follow_final,
                flags & AT_EMPTY_PATH as i32 != 0,
            )
        } {
            Ok(target) => target,
            Err(errno) => return -(errno as i64),
        };
        match target.dentry.inode() {
            Some(inode) => match chown_inode(&inode, uid, gid) {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            },
            None => -(ENOENT as i64),
        }
    })();
    if let Some(path) = trace_path.as_deref() {
        trace_run_path("fchownat", dirfd, path, flags, ret);
    }
    ret
}

pub unsafe fn sys_flock(fd: i32, _operation: i32) -> i64 {
    match current_files().and_then(|ft| ft.get(fd)) {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_dup3(oldfd: i32, newfd: i32, flags: i32) -> i64 {
    if oldfd == newfd || flags & !(O_CLOEXEC as i32) != 0 {
        return -(EINVAL as i64);
    }
    let ft = match current_files() {
        Ok(ft) => ft,
        Err(errno) => return -(errno as i64),
    };
    match ft.dup2(oldfd, newfd) {
        Ok(fd) => {
            if flags & O_CLOEXEC as i32 != 0 {
                let _ = ft.set_fd_flags(fd, crate::include::uapi::fcntl::FD_CLOEXEC);
            }
            fd as i64
        }
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_dup(oldfd: i32) -> i64 {
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    match files.dup_at_or_above(oldfd, 0, false) {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_dup2(oldfd: i32, newfd: i32) -> i64 {
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    match files.dup2(oldfd, newfd) {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_fcntl(fd: i32, cmd: i32, arg: u64) -> i64 {
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    let ret = match super::fcntl::sys_fcntl(&files, fd, cmd, arg) {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    };
    ret
}

pub unsafe fn sys_close_range(first: u32, last: u32, flags: u32) -> i64 {
    if flags & !(CLOSE_RANGE_UNSHARE | CLOSE_RANGE_CLOEXEC) != 0 {
        return -(EINVAL as i64);
    }
    if first > last {
        return -(EINVAL as i64);
    }
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    let first = first as usize;
    let last = last as usize;
    let result = if flags & CLOSE_RANGE_CLOEXEC != 0 {
        files.set_cloexec_range(first, last)
    } else {
        files.close_range(first, last)
    };
    let ret = match result {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    };
    ret
}

pub unsafe fn sys_poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i64 {
    let timeout_ns = if timeout < 0 {
        None
    } else {
        Some((timeout as u64).saturating_mul(1_000_000))
    };
    unsafe { poll_impl(fds, nfds, timeout_ns) }
}

unsafe fn poll_impl(fds: *mut PollFd, nfds: usize, timeout_ns: Option<u64>) -> i64 {
    if nfds != 0 && fds.is_null() {
        return -(EFAULT as i64);
    }
    let ft = match current_files() {
        Ok(ft) => ft,
        Err(errno) => return -(errno as i64),
    };
    let deadline_ns = timeout_ns.map(|ns| crate::kernel::time::ktime_get().saturating_add(ns));
    #[cfg(not(test))]
    let mut wait_state = ConsoleWaitState::default();

    loop {
        #[cfg(not(test))]
        let _ = crate::linux_driver_abi::poll_driver_abi_events_for_wait();
        #[cfg(not(test))]
        crate::init::rootfs::drain_console_control_bytes();

        let current = unsafe { sched::get_current() };
        let mut poll_table = select::PollTable::new(current);
        let ready = match unsafe {
            select::poll_once_with_table(ft.as_ref(), fds, nfds, Some(&mut poll_table))
        } {
            Ok(ready) => ready,
            Err(errno) => {
                poll_table.finish();
                return -(errno as i64);
            }
        };
        if ready != 0 {
            poll_table.finish();
            return ready;
        }
        if crate::kernel::signal::current_has_unblocked_pending_signals() {
            poll_table.finish();
            // Linux do_poll() returns the internal restart code after freeing
            // poll_wqueues. Syscall-exit signal handling then either restarts
            // the call or converts it to EINTR. Keeping the signal queued
            // here also lets every Rust-owned FileRef/Arc unwind normally.
            return -(ERESTARTNOHAND as i64);
        }
        if timeout_ns == Some(0) {
            poll_table.finish();
            return 0;
        }
        if let Some(deadline_ns) = deadline_ns {
            if crate::kernel::time::ktime_get() >= deadline_ns {
                poll_table.finish();
                return 0;
            }
        }

        #[cfg(not(test))]
        {
            wait_state.maintenance();
            poll_schedule(current, &mut poll_table, deadline_ns);
        }
        #[cfg(test)]
        {
            poll_table.finish();
            crate::kernel::time::timekeeping::tick_advance_walltime();
            crate::kernel::time::hrtimer_run_queues();
        }
    }
}

pub unsafe fn sys_select(
    nfds: i32,
    readfds: *mut u64,
    writefds: *mut u64,
    exceptfds: *mut u64,
    timeout: *mut crate::kernel::syscalls::TimeVal,
) -> i64 {
    let timeout_ns = match unsafe { select_timeout_ns(timeout) } {
        Ok(timeout_ns) => timeout_ns,
        Err(errno) => return -(errno as i64),
    };
    unsafe { select_impl(nfds, readfds, writefds, exceptfds, timeout_ns) }
}

unsafe fn select_impl(
    nfds: i32,
    readfds: *mut u64,
    writefds: *mut u64,
    exceptfds: *mut u64,
    timeout_ns: Option<u64>,
) -> i64 {
    if nfds < 0 {
        return -(EINVAL as i64);
    }
    let ft = match current_files() {
        Ok(ft) => ft,
        Err(errno) => return -(errno as i64),
    };

    let words = (nfds as usize).div_ceil(64);
    let read_wants = match unsafe { snapshot_fdset(readfds, words) } {
        Ok(snapshot) => snapshot,
        Err(errno) => return -(errno as i64),
    };
    let write_wants = match unsafe { snapshot_fdset(writefds, words) } {
        Ok(snapshot) => snapshot,
        Err(errno) => return -(errno as i64),
    };
    let except_wants = match unsafe { snapshot_fdset(exceptfds, words) } {
        Ok(snapshot) => snapshot,
        Err(errno) => return -(errno as i64),
    };
    let deadline_ns = match timeout_ns {
        None => None,
        Some(0) => Some(crate::kernel::time::ktime_get()),
        Some(ns) => Some(crate::kernel::time::ktime_get().saturating_add(ns)),
    };
    #[cfg(not(test))]
    let mut wait_state = ConsoleWaitState::default();

    loop {
        #[cfg(not(test))]
        crate::init::rootfs::drain_console_control_bytes();

        let mut read_ready = read_wants.clone();
        let mut write_ready = write_wants.clone();
        let mut except_ready = except_wants.clone();
        let read_ptr = fdset_kernel_ptr(readfds, &mut read_ready);
        let write_ptr = fdset_kernel_ptr(writefds, &mut write_ready);
        let except_ptr = fdset_kernel_ptr(exceptfds, &mut except_ready);
        let current = unsafe { sched::get_current() };
        let mut poll_table = select::PollTable::new(current);
        let ready = match unsafe {
            select::select_once_with_table(
                ft.as_ref(),
                nfds,
                read_ptr,
                write_ptr,
                except_ptr,
                Some(&mut poll_table),
            )
        } {
            Ok(ready) => ready,
            Err(errno) => {
                poll_table.finish();
                return -(errno as i64);
            }
        };
        if let Err(errno) = unsafe { restore_fdset(readfds, &read_ready) } {
            poll_table.finish();
            return -(errno as i64);
        }
        if let Err(errno) = unsafe { restore_fdset(writefds, &write_ready) } {
            poll_table.finish();
            return -(errno as i64);
        }
        if let Err(errno) = unsafe { restore_fdset(exceptfds, &except_ready) } {
            poll_table.finish();
            return -(errno as i64);
        }
        if ready != 0 {
            poll_table.finish();
            return ready;
        }
        if crate::kernel::signal::current_has_unblocked_pending_signals() {
            poll_table.finish();
            // Match core_sys_select(): free all wait registrations before
            // returning the internal restart code with the signal untouched.
            return -(ERESTARTNOHAND as i64);
        }

        if let Some(deadline_ns) = deadline_ns {
            if crate::kernel::time::ktime_get() >= deadline_ns {
                poll_table.finish();
                return 0;
            }
        }

        #[cfg(not(test))]
        {
            wait_state.maintenance();
            poll_schedule(current, &mut poll_table, deadline_ns);
        }
        #[cfg(test)]
        {
            poll_table.finish();
            crate::kernel::time::timekeeping::tick_advance_walltime();
            crate::kernel::time::hrtimer_run_queues();
        }
    }
}

/// Sleep after a poll/select scan while retaining its waitqueue entries.
///
/// Poll registrations do not alter task state.  Linux changes state once, then
/// checks the sticky `poll_wqueues.triggered` bit before scheduling; the same
/// handshake here prevents a wake on an early fd from being overwritten while
/// a later fd is registered.
#[cfg(not(test))]
fn poll_schedule(
    current: *mut crate::kernel::task::TaskStruct,
    table: &mut select::PollTable,
    deadline_ns: Option<u64>,
) {
    if current.is_null() {
        table.finish();
        return;
    }

    let task = current as usize;
    if table.prepare_to_sleep() {
        let timeout = deadline_ns.map(|deadline| {
            let remaining = deadline.saturating_sub(crate::kernel::time::ktime_get());
            crate::kernel::time::timeconv::nsecs_to_jiffies64(remaining).max(1)
        });
        let timeout = if table.has_unregistered_sources() {
            Some(timeout.unwrap_or(1).min(1))
        } else {
            timeout
        };
        if let Some(timeout) = timeout {
            let wake_at = crate::kernel::time::jiffies::jiffies().saturating_add(timeout);
            crate::kernel::time::sleep_timeout::arm_wakeup(task, wake_at);
        }
        unsafe {
            sched::schedule_with_irqs_enabled();
        }
        if timeout.is_some() {
            crate::kernel::time::sleep_timeout::cancel_wakeup(task);
        }
    }
    table.finish();
    unsafe {
        (*current).__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
    }
}

#[cfg(not(test))]
#[derive(Default)]
struct ConsoleWaitState;

#[cfg(not(test))]
impl ConsoleWaitState {
    fn maintenance(&mut self) {
        crate::linux_driver_abi::video::fbdev::core::refresh_cursor_blink();
        core::hint::spin_loop();
    }
}

fn fdset_kernel_ptr(user_fdset: *const u64, fdset: &mut [u64]) -> *mut u64 {
    if user_fdset.is_null() {
        core::ptr::null_mut()
    } else {
        fdset.as_mut_ptr()
    }
}

unsafe fn snapshot_fdset(fdset: *const u64, words: usize) -> Result<Vec<u64>, i32> {
    if fdset.is_null() || words == 0 {
        return Ok(Vec::new());
    }
    let mut snapshot = alloc::vec![0u64; words];
    let bytes = words.saturating_mul(core::mem::size_of::<u64>());
    let not_copied = unsafe {
        uaccess::copy_from_user(
            snapshot.as_mut_ptr().cast::<u8>(),
            fdset.cast::<u8>(),
            bytes,
        )
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(snapshot)
}

unsafe fn restore_fdset(fdset: *mut u64, snapshot: &[u64]) -> Result<(), i32> {
    if fdset.is_null() || snapshot.is_empty() {
        return Ok(());
    }
    let bytes = snapshot.len().saturating_mul(core::mem::size_of::<u64>());
    let not_copied =
        unsafe { uaccess::copy_to_user(fdset.cast::<u8>(), snapshot.as_ptr().cast::<u8>(), bytes) };
    if not_copied != 0 { Err(EFAULT) } else { Ok(()) }
}

unsafe fn select_timeout_ns(
    timeout: *const crate::kernel::syscalls::TimeVal,
) -> Result<Option<u64>, i32> {
    if timeout.is_null() {
        return Ok(None);
    }
    let timeout = unsafe { copy_user_value(timeout)? };
    if timeout.tv_sec < 0 || timeout.tv_usec < 0 || timeout.tv_usec >= 1_000_000 {
        return Err(EINVAL);
    }
    let secs = (timeout.tv_sec as u64).saturating_mul(1_000_000_000);
    let usecs = (timeout.tv_usec as u64).saturating_mul(1_000);
    Ok(Some(secs.saturating_add(usecs)))
}

unsafe fn copy_user_value<T: Copy + Default>(ptr: *const T) -> Result<T, i32> {
    let mut value = T::default();
    let not_copied = unsafe {
        uaccess::copy_from_user(
            (&mut value as *mut T).cast::<u8>(),
            ptr.cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if not_copied != 0 {
        Err(EFAULT)
    } else {
        Ok(value)
    }
}

unsafe fn timespec_timeout_ns(
    timeout: *const crate::kernel::time::Timespec64,
) -> Result<Option<u64>, i32> {
    if timeout.is_null() {
        return Ok(None);
    }
    let timeout = unsafe { copy_user_value(timeout)? };
    if !timeout.is_valid() {
        return Err(EINVAL);
    }
    Ok(Some(timeout.to_ns()))
}

pub unsafe fn sys_pselect6(
    nfds: i32,
    readfds: *mut u64,
    writefds: *mut u64,
    exceptfds: *mut u64,
    timeout: *const crate::kernel::time::Timespec64,
    sig: *const u8,
) -> i64 {
    let sigset = if sig.is_null() {
        PselectSigsetArg::default()
    } else {
        match unsafe { copy_user_value(sig.cast::<PselectSigsetArg>()) } {
            Ok(sigset) => sigset,
            Err(errno) => return -(errno as i64),
        }
    };
    let timeout_ns = match unsafe { timespec_timeout_ns(timeout) } {
        Ok(timeout_ns) => timeout_ns,
        Err(errno) => return -(errno as i64),
    };
    let error =
        unsafe { crate::kernel::signal::set_user_sigmask(sigset.sigmask, sigset.sigsetsize) };
    if error != 0 {
        return error;
    }
    let result = unsafe { select_impl(nfds, readfds, writefds, exceptfds, timeout_ns) };
    crate::kernel::signal::restore_saved_sigmask_unless(result == -(ERESTARTNOHAND as i64));
    result
}

pub unsafe fn sys_ppoll(
    fds: *mut PollFd,
    nfds: usize,
    timeout: *const crate::kernel::time::Timespec64,
    sigmask: *const u8,
    sigsetsize: usize,
) -> i64 {
    let timeout_ns = match unsafe { timespec_timeout_ns(timeout) } {
        Ok(timeout_ns) => timeout_ns,
        Err(errno) => return -(errno as i64),
    };
    let error = unsafe {
        crate::kernel::signal::set_user_sigmask(
            sigmask.cast::<crate::kernel::signal::SigSet>(),
            sigsetsize,
        )
    };
    if error != 0 {
        return error;
    }
    let result = unsafe { poll_impl(fds, nfds, timeout_ns) };
    crate::kernel::signal::restore_saved_sigmask_unless(result == -(ERESTARTNOHAND as i64));
    result
}

fn dirent_type(kind: InodeKind) -> u8 {
    match kind {
        InodeKind::Directory => 4,
        InodeKind::Chardev => 2,
        InodeKind::Blockdev => 6,
        InodeKind::Fifo => 1,
        InodeKind::Symlink => 10,
        InodeKind::Socket => 12,
        InodeKind::Regular => 8,
    }
}

fn getdents_reclen64(name_len: usize) -> usize {
    (19 + name_len + 1 + 7) & !7
}

fn getdents_reclen_legacy(name_len: usize) -> usize {
    (18 + name_len + 2 + (core::mem::size_of::<usize>() - 1)) & !(core::mem::size_of::<usize>() - 1)
}

fn restore_readdir_pos(file: &FileRef, pos: usize) {
    *file.pos.lock() = pos as u64;
}

unsafe fn getdents_common(
    fd: i32,
    dirent: *mut u8,
    count: usize,
    legacy: bool,
) -> (i64, Option<FileRef>) {
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return (-(errno as i64), None),
    };
    if file.flags.load(Ordering::Acquire) & O_PATH != 0 {
        return (-(EBADF as i64), Some(file));
    }
    let Some(readdir) = file.fops.readdir else {
        return (-(ENOTDIR as i64), Some(file));
    };
    let ret = (|| {
        let mut written = 0usize;
        loop {
            let pos_before = *file.pos.lock() as usize;
            let next = match readdir(&file) {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(errno) => return -(errno as i64),
            };
            let name = next.0.as_bytes();
            let reclen = if legacy {
                getdents_reclen_legacy(name.len())
            } else {
                getdents_reclen64(name.len())
            };
            if written + reclen > count {
                restore_readdir_pos(&file, pos_before);
                return if written == 0 {
                    -(EINVAL as i64)
                } else {
                    written as i64
                };
            }
            let pos_after = *file.pos.lock() as i64;
            let reclen_u16 = reclen as u16;
            let mut entry = alloc::vec![0u8; reclen];
            entry[0..8].copy_from_slice(&next.1.to_ne_bytes());
            entry[8..16].copy_from_slice(&pos_after.to_ne_bytes());
            entry[16..18].copy_from_slice(&reclen_u16.to_ne_bytes());
            if legacy {
                entry[18..18 + name.len()].copy_from_slice(name);
                entry[reclen - 1] = dirent_type(next.2);
            } else {
                entry[18] = dirent_type(next.2);
                entry[19..19 + name.len()].copy_from_slice(name);
            }
            let base = dirent.wrapping_add(written);
            let not_copied = unsafe { uaccess::copy_to_user(base, entry.as_ptr(), entry.len()) };
            if not_copied != 0 {
                restore_readdir_pos(&file, pos_before);
                return if written > 0 {
                    written as i64
                } else {
                    -(EFAULT as i64)
                };
            }
            written += reclen as usize;
        }
        written as i64
    })();
    (ret, Some(file))
}

pub unsafe fn sys_getdents64(fd: i32, dirent: *mut u8, count: usize) -> i64 {
    let (ret, file) = unsafe { getdents_common(fd, dirent, count, false) };
    let Some(file) = file else {
        return ret;
    };
    let trace_path = super::file::path_hint(&file);
    trace_run_fd(
        "getdents64",
        fd,
        trace_path.as_deref().unwrap_or(&file.dentry.name),
        file.flags.load(Ordering::Acquire),
        ret,
    );
    ret
}

pub unsafe fn sys_statfs(pathname: *const u8, out: *mut LinuxStatFs) -> i64 {
    if out.is_null() {
        return -(EFAULT as i64);
    }
    let dentry = match lookup_path(AT_FDCWD, pathname) {
        Ok(dentry) => dentry,
        Err(errno) => return -(errno as i64),
    };
    let inode = match dentry.inode() {
        Some(inode) => inode,
        None => return -(ENOENT as i64),
    };
    let sb = inode.sb.lock().clone();
    let st = super::stat::vfs_statfs(sb.as_ref());
    let statfs = LinuxStatFs {
        f_type: st.f_type,
        f_bsize: st.f_bsize,
        f_blocks: st.f_blocks,
        f_bfree: st.f_bfree,
        f_bavail: st.f_bavail,
        f_files: st.f_files,
        f_ffree: st.f_ffree,
        f_namelen: st.f_namelen,
        f_frsize: st.f_frsize,
        f_flags: st.f_flags,
        ..LinuxStatFs::default()
    };
    copy_ptr_to_user(out, &statfs)
        .map(|()| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub unsafe fn sys_fstatfs(fd: i32, out: *mut LinuxStatFs) -> i64 {
    if out.is_null() {
        return -(EFAULT as i64);
    }
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let inode = match file.inode() {
        Some(inode) => inode,
        None => return -(EBADF as i64),
    };
    let sb = inode.sb.lock().clone();
    let st = super::stat::vfs_statfs(sb.as_ref());
    let statfs = LinuxStatFs {
        f_type: st.f_type,
        f_bsize: st.f_bsize,
        f_blocks: st.f_blocks,
        f_bfree: st.f_bfree,
        f_bavail: st.f_bavail,
        f_files: st.f_files,
        f_ffree: st.f_ffree,
        f_namelen: st.f_namelen,
        f_frsize: st.f_frsize,
        f_flags: st.f_flags,
        ..LinuxStatFs::default()
    };
    copy_ptr_to_user(out, &statfs)
        .map(|()| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub unsafe fn sys_creat(pathname: *const u8, mode: u32) -> i64 {
    unsafe { sys_open(pathname, (O_CREAT | O_WRONLY | O_TRUNC) as i32, mode) }
}

pub unsafe fn sys_mkdirat(dirfd: i32, pathname: *const u8, mode: u32) -> i64 {
    let mut path = match unsafe { user_path(pathname) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    if path.starts_with('/') || dirfd == AT_FDCWD {
        path = super::fs_struct::absolute_from_cwd(&path);
    }
    let ret = (|| {
        let (root, start) = match root_and_start(dirfd) {
            Ok(pair) => pair,
            Err(errno) => return -(errno as i64),
        };
        let (parent_path, last) = split_last(&path);
        if last.is_empty() {
            return -(EINVAL as i64);
        }
        let base_hint = match dirfd_base_hint(dirfd, &start) {
            Ok(base) => base,
            Err(errno) => return -(errno as i64),
        };
        let parent = match mkdir_parent_dentry(&root, &start, &path, parent_path, base_hint) {
            Ok(parent) => parent,
            Err(errno) => return -(errno as i64),
        };
        let dir = match parent.inode() {
            Some(inode) => inode,
            None => return -(ENOENT as i64),
        };
        if cached_positive_child(&parent, &dir, last).is_some() {
            return -(EEXIST as i64);
        }
        if let Some(lookup) = dir.ops.lookup {
            match lookup(&dir, last) {
                Ok(_) => return -(EEXIST as i64),
                Err(ENOENT) => {}
                Err(errno) => return -(errno as i64),
            }
        }
        let create_mode = (mode & 0o7777) & !super::fs_struct::current_umask();
        match dir.ops.mkdir {
            Some(mkdir) => match mkdir(&dir, last, create_mode) {
                Ok(inode) => {
                    let dentry = super::dcache::d_lookup(&parent, last)
                        .unwrap_or_else(|| super::dcache::d_alloc_child(&parent, last));
                    dentry.instantiate(inode);
                    super::inotify::notify_create(&parent, last, true);
                    0
                }
                Err(errno) => -(errno as i64),
            },
            None => -(ENOSYS as i64),
        }
    })();
    trace_run_path("mkdirat", dirfd, &path, mode as i32, ret);
    ret
}

pub unsafe fn sys_mkdir(pathname: *const u8, mode: u32) -> i64 {
    unsafe { sys_mkdirat(AT_FDCWD, pathname, mode) }
}

fn mkdir_parent_dentry(
    root: &DentryRef,
    start: &DentryRef,
    path: &str,
    parent_path: &str,
    base_hint: Option<String>,
) -> Result<DentryRef, i32> {
    let full_parent = if path.starts_with('/') {
        String::from(parent_path)
    } else if let Some(base) = base_hint.or_else(|| mount::path_for_dentry(start)) {
        join_path(&base, parent_path)
    } else {
        return path_lookupat(&LookupCtx::new(root.clone(), start.clone(), 0), parent_path);
    };
    if let Some((_, dentry)) = mount::resolve_path(&full_parent) {
        return Ok(dentry);
    }
    path_lookupat(&LookupCtx::new(root.clone(), start.clone(), 0), parent_path)
}

pub unsafe fn sys_unlink(pathname: *const u8) -> i64 {
    unsafe { sys_unlinkat(AT_FDCWD, pathname, 0) }
}

pub unsafe fn sys_rmdir(pathname: *const u8) -> i64 {
    unsafe { sys_unlinkat(AT_FDCWD, pathname, AT_REMOVEDIR as i32) }
}

pub unsafe fn sys_unlinkat(dirfd: i32, pathname: *const u8, flags: i32) -> i64 {
    if flags & !(AT_REMOVEDIR as i32) != 0 {
        return -(EINVAL as i64);
    }
    let mut path = match unsafe { user_path(pathname) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    if path.starts_with('/') || dirfd == AT_FDCWD {
        path = super::fs_struct::absolute_from_cwd(&path);
    }
    let (root, start) = match root_and_start(dirfd) {
        Ok(pair) => pair,
        Err(errno) => return -(errno as i64),
    };
    let (parent_path, last) = split_last(&path);
    if last.is_empty() {
        return -(EINVAL as i64);
    }
    let base_hint = match dirfd_base_hint(dirfd, &start) {
        Ok(base) => base,
        Err(errno) => return -(errno as i64),
    };
    let parent = match mkdir_parent_dentry(&root, &start, &path, parent_path, base_hint) {
        Ok(parent) => parent,
        Err(errno) => return -(errno as i64),
    };
    let dir = match parent.inode() {
        Some(inode) => inode,
        None => return -(ENOENT as i64),
    };
    let (target, target_inode) = match lookup_child(&parent, &dir, last) {
        Ok(target) => target,
        Err(errno) => return -(errno as i64),
    };
    if target.flags.load(Ordering::Acquire) & crate::fs::types::DCACHE_MOUNTED != 0 {
        return -(EBUSY as i64);
    }
    let unbind_unix_path = (target_inode.kind == InodeKind::Socket).then(|| {
        let base =
            mount::path_for_dentry(&parent).unwrap_or_else(|| super::file::dentry_path(&parent));
        join_path(&base, last)
    });
    let result = if flags & AT_REMOVEDIR as i32 != 0 {
        match dir.ops.rmdir {
            Some(op) => op(&dir, last),
            None => Err(ENOSYS),
        }
    } else {
        match dir.ops.unlink {
            Some(op) => op(&dir, last, &target_inode),
            None => Err(ENOSYS),
        }
    };
    match result {
        Ok(()) => {
            super::dcache::d_drop(&parent, last);
            if let Some(path) = unbind_unix_path {
                crate::net::socket::unbind_unix_path(&path);
            }
            0
        }
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_faccessat2(dirfd: i32, pathname: *const u8, _mode: i32, flags: i32) -> i64 {
    // vendor/linux/fs/open.c:do_faccessat() accepts AT_EACCESS (check against
    // real uid/gid vs effective) alongside AT_SYMLINK_NOFOLLOW/AT_EMPTY_PATH.
    // glibc's realpath() canonicalization issues a trailing faccessat2() with
    // AT_EACCESS on the original (possibly slash-terminated) path; rejecting
    // that flag here made every such call fail with EINVAL.
    if flags
        & !((crate::include::uapi::fcntl::AT_SYMLINK_NOFOLLOW
            | crate::include::uapi::fcntl::AT_EMPTY_PATH
            | crate::include::uapi::fcntl::AT_EACCESS) as i32)
        != 0
    {
        return -(EINVAL as i64);
    }
    let follow_final = flags & (AT_SYMLINK_NOFOLLOW as i32) == 0;
    match unsafe {
        lookup_path_or_empty_target(
            dirfd,
            pathname,
            follow_final,
            flags & AT_EMPTY_PATH as i32 != 0,
        )
    } {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_access(pathname: *const u8, mode: i32) -> i64 {
    unsafe { sys_faccessat2(AT_FDCWD, pathname, mode, 0) }
}

pub unsafe fn sys_faccessat(dirfd: i32, pathname: *const u8, mode: i32) -> i64 {
    unsafe { sys_faccessat2(dirfd, pathname, mode, 0) }
}

fn copy_xattr_name(name: *const u8) -> Result<String, i32> {
    let name = unsafe { user_path(name) }?;
    super::xattr::validate_name(&name)?;
    Ok(name)
}

unsafe fn copy_xattr_value(value: *const u8, size: usize) -> Result<Vec<u8>, i32> {
    if size > super::xattr::XATTR_SIZE_MAX {
        return Err(E2BIG);
    }
    if size == 0 {
        return Ok(Vec::new());
    }
    if value.is_null() {
        return Err(EFAULT);
    }
    let mut out = alloc::vec![0u8; size];
    let not_copied = unsafe { uaccess::copy_from_user(out.as_mut_ptr(), value, size) };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(out)
}

fn copy_xattr_bytes_to_user(bytes: &[u8], user: *mut u8, size: usize) -> Result<i64, i32> {
    if size == 0 {
        return Ok(bytes.len() as i64);
    }
    if user.is_null() {
        return Err(EFAULT);
    }
    if size < bytes.len() {
        return Err(ERANGE);
    }
    let not_copied = unsafe { uaccess::copy_to_user(user, bytes.as_ptr(), bytes.len()) };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(bytes.len() as i64)
}

fn inode_from_file(file: &FileRef) -> Result<InodeRef, i32> {
    file.inode().ok_or(ENOENT)
}

unsafe fn xattr_target_from_user_path(
    dirfd: i32,
    pathname: *const u8,
    follow_final: bool,
    allow_empty: bool,
) -> Result<(String, StatTarget), i32> {
    if pathname.is_null() {
        if allow_empty {
            if dirfd < 0 {
                return Err(EBADF);
            }
            let file = current_files()?.get(dirfd)?;
            let target = stat_target_from_dentry(file.dentry.clone())?;
            return Ok((String::new(), target));
        }
        return Err(EFAULT);
    }

    let path = unsafe { user_path(pathname) }?;
    if let Some(file) = crate::fs::proc::fd::current_fd_file_from_proc_path(&path) {
        return file.and_then(|file| {
            stat_target_from_dentry(file.dentry.clone()).map(|target| (path, target))
        });
    }
    let target =
        unsafe { lookup_path_or_empty_target(dirfd, pathname, follow_final, allow_empty) }?;
    Ok((path, target))
}

fn xattr_name_requires_sys_admin(name: &str) -> bool {
    name.starts_with("security.") || name.starts_with("trusted.")
}

fn check_xattr_write_permission(
    target: &StatTarget,
    inode: &InodeRef,
    name: &str,
) -> Result<(), i32> {
    if target.mount.is_readonly() {
        return Err(crate::include::uapi::errno::EROFS);
    }

    if xattr_name_requires_sys_admin(name) {
        if capable(CAP_SYS_ADMIN) {
            return Ok(());
        }
        return Err(EPERM);
    }

    if super::permission::inode_owner_or_capable(inode) {
        return Ok(());
    }

    super::permission::check_inode_write_permission(inode)
}

fn check_xattr_file_write_permission(
    file: &FileRef,
    inode: &InodeRef,
    name: &str,
) -> Result<(), i32> {
    if file.flags.load(Ordering::Acquire) & O_PATH != 0 {
        return Err(EBADF);
    }
    let target = stat_target_from_dentry(file.dentry.clone())?;
    check_xattr_write_permission(&target, inode, name)
}

fn xattr_at_flags(at_flags: u32) -> Result<(bool, bool), i32> {
    let allowed = (AT_SYMLINK_NOFOLLOW | AT_EMPTY_PATH) as u32;
    if at_flags & !allowed != 0 {
        return Err(EINVAL);
    }
    Ok((
        at_flags & AT_SYMLINK_NOFOLLOW as u32 == 0,
        at_flags & AT_EMPTY_PATH as u32 != 0,
    ))
}

unsafe fn do_setxattr_path(
    dirfd: i32,
    pathname: *const u8,
    follow_final: bool,
    allow_empty: bool,
    name: *const u8,
    value: *const u8,
    size: usize,
    flags: i32,
) -> Result<(), i32> {
    let (_, target) =
        unsafe { xattr_target_from_user_path(dirfd, pathname, follow_final, allow_empty) }?;
    let inode = target.dentry.inode().ok_or(ENOENT)?;
    let name = copy_xattr_name(name)?;
    check_xattr_write_permission(&target, &inode, &name)?;
    let value = unsafe { copy_xattr_value(value, size) }?;
    super::xattr::set_inode_xattr(&inode, &name, &value, flags)
}

unsafe fn do_getxattr_path(
    dirfd: i32,
    pathname: *const u8,
    follow_final: bool,
    allow_empty: bool,
    name: *const u8,
    value: *mut u8,
    size: usize,
) -> Result<i64, i32> {
    let (_, target) =
        unsafe { xattr_target_from_user_path(dirfd, pathname, follow_final, allow_empty) }?;
    let inode = target.dentry.inode().ok_or(ENOENT)?;
    let name = copy_xattr_name(name)?;
    let bytes = super::xattr::get_inode_xattr(&inode, &name)?;
    copy_xattr_bytes_to_user(&bytes, value, size)
}

unsafe fn do_listxattr_path(
    dirfd: i32,
    pathname: *const u8,
    follow_final: bool,
    allow_empty: bool,
    list: *mut u8,
    size: usize,
) -> Result<i64, i32> {
    let (_, target) =
        unsafe { xattr_target_from_user_path(dirfd, pathname, follow_final, allow_empty) }?;
    let inode = target.dentry.inode().ok_or(ENOENT)?;
    let bytes = super::xattr::list_inode_xattrs(&inode)?;
    copy_xattr_bytes_to_user(&bytes, list, size)
}

unsafe fn do_removexattr_path(
    dirfd: i32,
    pathname: *const u8,
    follow_final: bool,
    allow_empty: bool,
    name: *const u8,
) -> Result<String, i32> {
    let (path, target) =
        unsafe { xattr_target_from_user_path(dirfd, pathname, follow_final, allow_empty) }?;
    let inode = target.dentry.inode().ok_or(ENOENT)?;
    let name = copy_xattr_name(name)?;
    check_xattr_write_permission(&target, &inode, &name)?;
    super::xattr::remove_inode_xattr(&inode, &name)?;
    Ok(path)
}

pub unsafe fn sys_setxattr(
    pathname: *const u8,
    name: *const u8,
    value: *const u8,
    size: usize,
    flags: i32,
) -> i64 {
    match unsafe { do_setxattr_path(AT_FDCWD, pathname, true, false, name, value, size, flags) } {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_lsetxattr(
    pathname: *const u8,
    name: *const u8,
    value: *const u8,
    size: usize,
    flags: i32,
) -> i64 {
    match unsafe { do_setxattr_path(AT_FDCWD, pathname, false, false, name, value, size, flags) } {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_fsetxattr(
    fd: i32,
    name: *const u8,
    value: *const u8,
    size: usize,
    flags: i32,
) -> i64 {
    let result = current_files().and_then(|ft| ft.get(fd)).and_then(|file| {
        let inode = inode_from_file(&file)?;
        let name = copy_xattr_name(name)?;
        check_xattr_file_write_permission(&file, &inode, &name)?;
        let value = unsafe { copy_xattr_value(value, size) }?;
        super::xattr::set_inode_xattr(&inode, &name, &value, flags)
    });
    match result {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_getxattr(
    pathname: *const u8,
    name: *const u8,
    value: *mut u8,
    size: usize,
) -> i64 {
    match unsafe { do_getxattr_path(AT_FDCWD, pathname, true, false, name, value, size) } {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_lgetxattr(
    pathname: *const u8,
    name: *const u8,
    value: *mut u8,
    size: usize,
) -> i64 {
    match unsafe { do_getxattr_path(AT_FDCWD, pathname, false, false, name, value, size) } {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_fgetxattr(fd: i32, name: *const u8, value: *mut u8, size: usize) -> i64 {
    let result = current_files()
        .and_then(|ft| ft.get(fd))
        .and_then(|file| inode_from_file(&file))
        .and_then(|inode| {
            let name = copy_xattr_name(name)?;
            let bytes = super::xattr::get_inode_xattr(&inode, &name)?;
            copy_xattr_bytes_to_user(&bytes, value, size)
        });
    match result {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_listxattr(pathname: *const u8, list: *mut u8, size: usize) -> i64 {
    match unsafe { do_listxattr_path(AT_FDCWD, pathname, true, false, list, size) } {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_llistxattr(pathname: *const u8, list: *mut u8, size: usize) -> i64 {
    match unsafe { do_listxattr_path(AT_FDCWD, pathname, false, false, list, size) } {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_flistxattr(fd: i32, list: *mut u8, size: usize) -> i64 {
    let result = current_files()
        .and_then(|ft| ft.get(fd))
        .and_then(|file| inode_from_file(&file))
        .and_then(|inode| {
            let bytes = super::xattr::list_inode_xattrs(&inode)?;
            copy_xattr_bytes_to_user(&bytes, list, size)
        });
    match result {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_removexattr(pathname: *const u8, name: *const u8) -> i64 {
    let path = unsafe { user_path(pathname).unwrap_or_else(|_| String::new()) };
    let ret = match unsafe { do_removexattr_path(AT_FDCWD, pathname, true, false, name) } {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    };
    trace_xattr_path("removexattr", AT_FDCWD, &path, ret);
    ret
}

pub unsafe fn sys_lremovexattr(pathname: *const u8, name: *const u8) -> i64 {
    let path = unsafe { user_path(pathname).unwrap_or_else(|_| String::new()) };
    let ret = match unsafe { do_removexattr_path(AT_FDCWD, pathname, false, false, name) } {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    };
    trace_xattr_path("lremovexattr", AT_FDCWD, &path, ret);
    ret
}

pub unsafe fn sys_fremovexattr(fd: i32, name: *const u8) -> i64 {
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let ret = match inode_from_file(&file).and_then(|inode| {
        let name = copy_xattr_name(name)?;
        check_xattr_file_write_permission(&file, &inode, &name)?;
        super::xattr::remove_inode_xattr(&inode, &name)
    }) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    };
    trace_run_fd(
        "fremovexattr",
        fd,
        &file.dentry.name,
        file.flags.load(Ordering::Acquire),
        ret,
    );
    ret
}

pub unsafe fn sys_sendfile(out_fd: i32, in_fd: i32, _offset: *mut i64, count: usize) -> i64 {
    if count == 0 {
        return 0;
    }
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    if files.get(out_fd).is_err() || files.get(in_fd).is_err() {
        return -(EBADF as i64);
    }
    -(ENOSYS as i64)
}

pub unsafe fn sys_getdents(fd: i32, dirent: *mut u8, count: usize) -> i64 {
    let (ret, file) = unsafe { getdents_common(fd, dirent, count, true) };
    let Some(file) = file else {
        return ret;
    };
    trace_run_fd(
        "getdents",
        fd,
        &file.dentry.name,
        file.flags.load(Ordering::Acquire),
        ret,
    );
    ret
}

pub unsafe fn sys_getcwd(buf: *mut u8, size: usize) -> i64 {
    if buf.is_null() {
        return -(EFAULT as i64);
    }
    let mut cwd = {
        let fs = super::fs_struct::current_fs();
        if fs.is_null() {
            super::fs_struct::current_cwd_path()
        } else {
            let fs = unsafe { &*fs };
            let pwd = fs.pwd.lock().clone();
            pwd.map(|pwd| super::fs_struct::visible_path_for_current_root(&pwd))
                .unwrap_or_else(super::fs_struct::current_cwd_path)
        }
    }
    .into_bytes();
    cwd.push(0);
    if size < cwd.len() {
        return -(ERANGE as i64);
    }
    let left = unsafe { uaccess::copy_to_user(buf, cwd.as_ptr(), cwd.len()) };
    if left == 0 {
        cwd.len() as i64
    } else {
        -(EFAULT as i64)
    }
}

pub unsafe fn sys_chdir(pathname: *const u8) -> i64 {
    let path = match unsafe { user_path(pathname) } {
        Ok(path) if !path.is_empty() => path,
        Ok(_) => return -(ENOENT as i64),
        Err(errno) => return -(errno as i64),
    };
    match lookup_path_str_with_follow(AT_FDCWD, &path, true) {
        Ok(target)
            if target
                .dentry
                .inode()
                .map(|i| i.kind == InodeKind::Directory)
                .unwrap_or(false) =>
        {
            let fs = super::fs_struct::current_fs();
            if !fs.is_null() {
                super::fs_struct::set_fs_pwd_path(
                    unsafe { &*fs },
                    mount::VfsPath::new(target.mount.clone(), target.dentry.clone()),
                );
            }
            let visible = super::fs_struct::visible_path_for_current_root(&mount::VfsPath::new(
                target.mount,
                target.dentry,
            ));
            super::fs_struct::set_current_cwd_path(&visible);
            0
        }
        Ok(_) => -(ENOTDIR as i64),
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_fchdir(fd: i32) -> i64 {
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let inode = match file.inode() {
        Some(inode) => inode,
        None => return -(EBADF as i64),
    };
    if inode.kind != InodeKind::Directory {
        return -(ENOTDIR as i64);
    }
    let fs = super::fs_struct::current_fs();
    if !fs.is_null() {
        let path = mount::VfsPath::for_dentry(file.dentry.clone());
        if let Some(path) = path {
            super::fs_struct::set_fs_pwd_path(unsafe { &*fs }, path);
        }
    }
    let visible = mount::VfsPath::for_dentry(file.dentry.clone())
        .map(|path| super::fs_struct::visible_path_for_current_root(&path))
        .unwrap_or_else(|| super::file::dentry_path(&file.dentry));
    super::fs_struct::set_current_cwd_path(&visible);
    0
}

pub unsafe fn sys_renameat2(
    olddfd: i32,
    oldname: *const u8,
    newdfd: i32,
    newname: *const u8,
    flags: u32,
) -> i64 {
    let old_path = unsafe { user_path(oldname) };
    let new_path = unsafe { user_path(newname) };
    match old_path.and_then(|old_path| new_path.map(|new_path| (old_path, new_path))) {
        Ok((old_path, new_path)) => renameat2_path(olddfd, &old_path, newdfd, &new_path, flags)
            .map(|_| 0)
            .unwrap_or_else(|errno| -(errno as i64)),
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_rename(oldname: *const u8, newname: *const u8) -> i64 {
    unsafe { sys_renameat2(AT_FDCWD, oldname, AT_FDCWD, newname, 0) }
}

pub unsafe fn sys_renameat(
    olddfd: i32,
    oldname: *const u8,
    newdfd: i32,
    newname: *const u8,
) -> i64 {
    unsafe { sys_renameat2(olddfd, oldname, newdfd, newname, 0) }
}

fn rename_parent(dirfd: i32, path: &str) -> Result<(DentryRef, InodeRef, String), i32> {
    let effective_path;
    let path = if path.starts_with('/') || dirfd == AT_FDCWD {
        effective_path = super::fs_struct::absolute_from_cwd(path);
        effective_path.as_str()
    } else {
        path
    };
    let (root, start) = root_and_start(dirfd)?;
    let (parent_path, last) = split_last(path);
    if last.is_empty() || last == "." || last == ".." {
        return Err(ENOENT);
    }
    let base_hint = dirfd_base_hint(dirfd, &start)?;
    let parent = mkdir_parent_dentry(&root, &start, path, parent_path, base_hint)?;
    let parent_inode = parent.inode().ok_or(ENOENT)?;
    if parent_inode.kind != InodeKind::Directory {
        return Err(ENOTDIR);
    }
    Ok((parent, parent_inode, String::from(last)))
}

fn lookup_child(
    parent: &DentryRef,
    dir: &InodeRef,
    name: &str,
) -> Result<(DentryRef, InodeRef), i32> {
    if let Some(child) = cached_positive_child(parent, dir, name) {
        return Ok(child);
    }
    let lookup = dir.ops.lookup.ok_or(ENOENT)?;
    let inode = match lookup(dir, name) {
        Ok(inode) => inode,
        Err(ENOENT) => {
            super::dcache::d_cache_negative(parent, name);
            return Err(ENOENT);
        }
        Err(errno) => return Err(errno),
    };
    let dentry = super::dcache::d_alloc_child(parent, name);
    dentry.instantiate(inode.clone());
    Ok((dentry, inode))
}

fn cached_positive_child(
    parent: &DentryRef,
    dir: &InodeRef,
    name: &str,
) -> Option<(DentryRef, InodeRef)> {
    let dentry = super::dcache::d_lookup(parent, name)?;
    let inode = dentry.inode()?;
    if let Some(mapped) = ramdir_child_inode(dir, name) {
        match mapped {
            Some(mapped_inode) if Arc::ptr_eq(&mapped_inode, &inode) => {}
            Some(_) | None => {
                super::dcache::d_drop(parent, name);
                return None;
            }
        }
    }
    Some((dentry, inode))
}

fn lookup_child_optional(
    parent: &DentryRef,
    dir: &InodeRef,
    name: &str,
) -> Result<Option<(DentryRef, InodeRef)>, i32> {
    match lookup_child(parent, dir, name) {
        Ok(child) => Ok(Some(child)),
        Err(ENOENT) => Ok(None),
        Err(errno) => Err(errno),
    }
}

fn ramdir_key(map: &BTreeMap<String, InodeRef>, name: &str) -> Option<String> {
    map.keys().find(|key| key.as_str() == name).cloned()
}

fn ramdir_child_inode(dir: &InodeRef, name: &str) -> Option<Option<InodeRef>> {
    let InodePrivate::RamDir(children) = &dir.private else {
        return None;
    };
    let children = children.lock();
    let child = ramdir_key(&children, name).and_then(|key| children.get(&key).cloned());
    Some(child)
}

fn ensure_empty_directory(inode: &InodeRef) -> Result<(), i32> {
    if inode.kind != InodeKind::Directory {
        return Ok(());
    }
    match &inode.private {
        InodePrivate::RamDir(children) if children.lock().is_empty() => Ok(()),
        InodePrivate::RamDir(_) => Err(ENOTEMPTY),
        _ => Err(ENOTEMPTY),
    }
}

fn same_superblock(a: &InodeRef, b: &InodeRef) -> Result<bool, i32> {
    let a_sb = a.sb.lock().clone().ok_or(EINVAL)?;
    let b_sb = b.sb.lock().clone().ok_or(EINVAL)?;
    Ok(Arc::ptr_eq(&a_sb, &b_sb))
}

fn renameat2_path(
    olddfd: i32,
    old_path: &str,
    newdfd: i32,
    new_path: &str,
    flags: u32,
) -> Result<(), i32> {
    if flags & !(RENAME_NOREPLACE | RENAME_EXCHANGE | RENAME_WHITEOUT) != 0 {
        return Err(EINVAL);
    }
    if flags & RENAME_EXCHANGE != 0 && flags & (RENAME_NOREPLACE | RENAME_WHITEOUT) != 0 {
        return Err(EINVAL);
    }
    if flags & (RENAME_EXCHANGE | RENAME_WHITEOUT) != 0 {
        return Err(ENOSYS);
    }
    if old_path.is_empty() || new_path.is_empty() {
        return Err(ENOENT);
    }

    let (old_parent, old_dir, old_name) = rename_parent(olddfd, old_path)?;
    let (new_parent, new_dir, new_name) = rename_parent(newdfd, new_path)?;
    if !same_superblock(&old_dir, &new_dir)? {
        return Err(EXDEV);
    }

    let (old_dentry, old_inode) = lookup_child(&old_parent, &old_dir, &old_name)?;
    if old_dentry.flags.load(Ordering::Acquire) & crate::fs::types::DCACHE_MOUNTED != 0 {
        return Err(EBUSY);
    }

    let target = lookup_child_optional(&new_parent, &new_dir, &new_name)?;
    if let Some((new_dentry, new_inode)) = &target {
        if Arc::ptr_eq(&old_inode, new_inode) {
            return Ok(());
        }
        if flags & RENAME_NOREPLACE != 0 {
            return Err(EEXIST);
        }
        if new_dentry.flags.load(Ordering::Acquire) & crate::fs::types::DCACHE_MOUNTED != 0 {
            return Err(EBUSY);
        }
        match (
            old_inode.kind == InodeKind::Directory,
            new_inode.kind == InodeKind::Directory,
        ) {
            (true, false) => return Err(ENOTDIR),
            (false, true) => return Err(EISDIR),
            (true, true) => ensure_empty_directory(new_inode)?,
            (false, false) => {}
        }
    }

    if let Some(rename) = old_dir.ops.rename {
        rename(&old_dir, &old_name, &new_dir, &new_name)?;
    } else if Arc::ptr_eq(&old_dir, &new_dir) {
        let InodePrivate::RamDir(children) = &old_dir.private else {
            return Err(ENOSYS);
        };
        let mut children = children.lock();
        let old_key = ramdir_key(&children, &old_name).ok_or(ENOENT)?;
        if old_key == new_name {
            return Ok(());
        }
        if let Some(new_key) = ramdir_key(&children, &new_name) {
            children.remove(&new_key);
        }
        let moved = children.remove(&old_key).ok_or(ENOENT)?;
        children.insert(new_name.clone(), moved);
    } else {
        let InodePrivate::RamDir(old_children) = &old_dir.private else {
            return Err(ENOSYS);
        };
        let InodePrivate::RamDir(new_children) = &new_dir.private else {
            return Err(ENOSYS);
        };
        let moved = {
            let mut old_children = old_children.lock();
            let old_key = ramdir_key(&old_children, &old_name).ok_or(ENOENT)?;
            old_children.remove(&old_key).ok_or(ENOENT)?
        };
        let mut new_children = new_children.lock();
        if let Some(new_key) = ramdir_key(&new_children, &new_name) {
            new_children.remove(&new_key);
        }
        new_children.insert(new_name.clone(), moved);
    }

    super::dcache::d_drop(&old_parent, &old_name);
    super::dcache::d_drop(&new_parent, &new_name);
    let renamed = super::dcache::d_alloc_child(&new_parent, &new_name);
    let moved_is_dir = old_inode.kind == InodeKind::Directory;
    renamed.instantiate(old_inode);
    super::inotify::notify_move(&old_parent, &old_name, &new_parent, &new_name, moved_is_dir);
    Ok(())
}

pub unsafe fn sys_linkat(
    olddfd: i32,
    oldname: *const u8,
    newdfd: i32,
    newname: *const u8,
    flags: i32,
) -> i64 {
    if flags & !((AT_SYMLINK_FOLLOW | AT_EMPTY_PATH) as i32) != 0 {
        return -(EINVAL as i64);
    }
    match unsafe { user_path(oldname).and_then(|old| user_path(newname).map(|new| (old, new))) } {
        Ok((old_path, new_path)) => linkat_path(olddfd, &old_path, newdfd, &new_path, flags)
            .map(|_| 0)
            .unwrap_or_else(|errno| -(errno as i64)),
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_link(oldname: *const u8, newname: *const u8) -> i64 {
    unsafe { sys_linkat(AT_FDCWD, oldname, AT_FDCWD, newname, 0) }
}

fn linkat_path(
    olddfd: i32,
    old_path: &str,
    newdfd: i32,
    new_path: &str,
    flags: i32,
) -> Result<(), i32> {
    let old_target = if old_path.is_empty() {
        if flags & AT_EMPTY_PATH as i32 == 0 {
            return Err(ENOENT);
        }
        if !capable(CAP_DAC_READ_SEARCH) {
            return Err(ENOENT);
        }
        lookup_empty_stat_target(olddfd)?
    } else {
        lookup_path_str_with_follow(olddfd, old_path, flags & AT_SYMLINK_FOLLOW as i32 != 0)?
    };
    let old_inode = old_target.dentry.inode().ok_or(ENOENT)?;
    if old_inode.kind == InodeKind::Directory {
        return Err(EPERM);
    }

    if new_path.is_empty() {
        return Err(ENOENT);
    }
    let (new_parent, new_dir, new_name) = linkat_parent(newdfd, new_path)?;
    if lookup_child_optional(&new_parent.dentry, &new_dir, &new_name)?.is_some() {
        return Err(EEXIST);
    }
    if new_parent.mount.is_readonly() {
        return Err(crate::include::uapi::errno::EROFS);
    }
    if !Arc::ptr_eq(&old_target.mount, &new_parent.mount) {
        return Err(EXDEV);
    }
    check_inode_write_permission(&new_dir)?;

    match &new_dir.private {
        InodePrivate::RamDir(children) => {
            let mut children = children.lock();
            if ramdir_key(&children, &new_name).is_some() {
                return Err(EEXIST);
            }
            children.insert(new_name.clone(), old_inode.clone());
            old_inode.nlink.fetch_add(1, Ordering::AcqRel);
            super::types::touch_inode_now(&new_dir);
            super::types::touch_inode_now(&old_inode);
        }
        _ if new_dir.ops.name == "ext4_dir" => {
            crate::fs::ext4::ops::ext4_link(&new_dir, &new_name, &old_inode)?;
        }
        _ => return Err(ENOSYS),
    }

    super::dcache::d_drop(&new_parent.dentry, &new_name);
    let linked = super::dcache::d_alloc_child(&new_parent.dentry, &new_name);
    linked.instantiate(old_inode);
    super::inotify::notify_create(&new_parent.dentry, &new_name, false);
    Ok(())
}

fn linkat_parent(dirfd: i32, path: &str) -> Result<(StatTarget, InodeRef, String), i32> {
    let (parent_path, last) = split_last(path);
    if last.is_empty() || last == "." || last == ".." {
        return Err(ENOENT);
    }
    let parent = lookup_path_str_with_follow(dirfd, parent_path, true)?;
    let parent_inode = parent.dentry.inode().ok_or(ENOENT)?;
    if parent_inode.kind != InodeKind::Directory {
        return Err(ENOTDIR);
    }
    Ok((parent, parent_inode, String::from(last)))
}

pub unsafe fn sys_symlinkat(oldname: *const u8, newdfd: i32, newname: *const u8) -> i64 {
    let target = match unsafe { user_path(oldname) } {
        Ok(target) if !target.is_empty() => target,
        Ok(_) => return -(ENOENT as i64),
        Err(errno) => return -(errno as i64),
    };
    let path = match unsafe { user_path(newname) } {
        Ok(path) if !path.is_empty() => path,
        Ok(_) => return -(ENOENT as i64),
        Err(errno) => return -(errno as i64),
    };
    let (root, start) = match root_and_start(newdfd) {
        Ok(pair) => pair,
        Err(errno) => return -(errno as i64),
    };
    let (parent_path, last) = split_last(&path);
    if last.is_empty() || last == "." || last == ".." {
        return -(ENOENT as i64);
    }
    let base_hint = match dirfd_base_hint(newdfd, &start) {
        Ok(base) => base,
        Err(errno) => return -(errno as i64),
    };
    let parent = match mkdir_parent_dentry(&root, &start, &path, parent_path, base_hint) {
        Ok(parent) => parent,
        Err(errno) => {
            let ret = -(errno as i64);
            trace_run_symlinkat(newdfd, &path, &target, "<parent-lookup>", ret);
            return ret;
        }
    };
    let parent_inode = match parent.inode() {
        Some(inode) if inode.kind == InodeKind::Directory => inode,
        Some(_) => {
            let ret = -(ENOTDIR as i64);
            trace_run_symlinkat(newdfd, &path, &target, "<not-dir>", ret);
            return ret;
        }
        None => {
            let ret = -(ENOENT as i64);
            trace_run_symlinkat(newdfd, &path, &target, "<negative>", ret);
            return ret;
        }
    };
    if super::dcache::d_lookup(&parent, last)
        .and_then(|dentry| dentry.inode())
        .is_some()
        || parent_inode
            .ops
            .lookup
            .is_some_and(|lookup| lookup(&parent_inode, last).is_ok())
    {
        let ret = -(EEXIST as i64);
        trace_run_symlinkat(newdfd, &path, &target, parent_inode.ops.name, ret);
        return ret;
    }
    let symlink = match parent_inode.ops.symlink {
        Some(symlink) => symlink,
        None => {
            let ret = -(ENOSYS as i64);
            trace_run_symlinkat(newdfd, &path, &target, parent_inode.ops.name, ret);
            return ret;
        }
    };
    let ret = match symlink(&parent_inode, last, &target, 0o777) {
        Ok(inode) => {
            let child = super::dcache::d_lookup(&parent, last)
                .unwrap_or_else(|| super::dcache::d_alloc_child(&parent, last));
            child.instantiate(inode);
            super::inotify::notify_create(&parent, last, false);
            0
        }
        Err(errno) => -(errno as i64),
    };
    trace_run_symlinkat(newdfd, &path, &target, parent_inode.ops.name, ret);
    ret
}

pub unsafe fn sys_symlink(oldname: *const u8, newname: *const u8) -> i64 {
    unsafe { sys_symlinkat(oldname, AT_FDCWD, newname) }
}

pub unsafe fn sys_readlinkat(dirfd: i32, pathname: *const u8, buf: *mut u8, bufsiz: usize) -> i64 {
    if bufsiz == 0 {
        return -(EINVAL as i64);
    }
    if buf.is_null() {
        return -(EFAULT as i64);
    }
    match unsafe { user_path(pathname) } {
        Ok(path) => {
            let result = readlinkat_path(dirfd, &path)
                .and_then(|target| unsafe { copy_readlink_to_user(&target, buf, bufsiz) });
            let ret = result
                .map(|n| n as i64)
                .unwrap_or_else(|errno| -(errno as i64));
            trace_run_readlinkat(dirfd, &path, ret);
            ret
        }
        Err(errno) => -(errno as i64),
    }
}

fn readlinkat_path(dirfd: i32, path: &str) -> Result<String, i32> {
    if path.is_empty() {
        let dentry = readlink_empty_dentry(dirfd)?;
        return readlink_dentry(&dentry, true);
    }

    let dynamic_fd_path = if dirfd == AT_FDCWD {
        crate::fs::proc::fd::current_fd_path_from_proc_path(path)
    } else {
        None
    };
    if let Some(fd_path) = dynamic_fd_path {
        return fd_path;
    }

    let target = lookup_path_str(dirfd, path)?;
    readlink_dentry(&target.dentry, false)
}

fn readlink_empty_dentry(dirfd: i32) -> Result<DentryRef, i32> {
    if dirfd == AT_FDCWD {
        return Ok(mount::rootfs().ok_or(EINVAL)?.root.clone());
    }
    if dirfd < 0 {
        return Err(EBADF);
    }
    Ok(current_files()?.get(dirfd)?.dentry.clone())
}

fn readlink_dentry(dentry: &DentryRef, empty_path: bool) -> Result<String, i32> {
    let inode = dentry.inode().ok_or(ENOENT)?;
    if inode.kind != InodeKind::Symlink && inode.ops.readlink.is_none() {
        return Err(if empty_path { ENOENT } else { EINVAL });
    }
    let readlink = inode.ops.readlink.ok_or(EINVAL)?;
    let mut owned_target = alloc::vec![0u8; 4096];
    let n = readlink(&inode, &mut owned_target)?;
    core::str::from_utf8(&owned_target[..n])
        .map(String::from)
        .map_err(|_| EINVAL)
}

unsafe fn copy_readlink_to_user(target: &str, buf: *mut u8, bufsiz: usize) -> Result<usize, i32> {
    let bytes = target.as_bytes();
    let n = bytes.len().min(bufsiz);
    let not_copied = unsafe { uaccess::copy_to_user(buf, bytes.as_ptr(), n) };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(n)
}

#[cfg(not(test))]
fn trace_run_readlinkat(dirfd: i32, path: &str, ret: i64) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    if !(ret < 0 || path.is_empty() || path.contains("/proc/self/fd") || path.contains("systemd")) {
        return;
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-run-readlinkat pid={} dirfd={} path={} ret={}",
        pid,
        dirfd,
        if path.is_empty() { "<empty>" } else { path },
        ret
    );
}

#[cfg(test)]
fn trace_run_readlinkat(_dirfd: i32, _path: &str, _ret: i64) {}

pub unsafe fn sys_readlink(pathname: *const u8, buf: *mut u8, bufsiz: usize) -> i64 {
    unsafe { sys_readlinkat(AT_FDCWD, pathname, buf, bufsiz) }
}

pub(crate) fn mknod_kind(mode: u32) -> Result<InodeKind, i32> {
    match mode & S_IFMT {
        0 | S_IFREG => Ok(InodeKind::Regular),
        S_IFIFO => Ok(InodeKind::Fifo),
        S_IFSOCK => Ok(InodeKind::Socket),
        S_IFCHR => Ok(InodeKind::Chardev),
        S_IFBLK => Ok(InodeKind::Blockdev),
        S_IFDIR => Err(EPERM),
        _ => Err(EINVAL),
    }
}

fn insert_special_node(
    parent: &DentryRef,
    dir: &InodeRef,
    name: &str,
    mode: u32,
    kind: InodeKind,
    dev: u32,
) -> Result<(), i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let fops = if kind == InodeKind::Blockdev {
        &crate::block::block_device::BLOCK_DEVICE_FILE_OPS
    } else {
        &NOOP_FILE_OPS
    };
    let inode = Inode::new(
        sb.alloc_ino(),
        kind,
        mode & 0o7777,
        &NOOP_INODE_OPS,
        fops,
        InodePrivate::None,
    );
    // Linux `init_special_inode()` stamps `i_rdev` for both S_ISCHR and
    // S_ISBLK. Lupos's block-device lookup is name-based (see
    // `block_device_for_file`), but chardev opens dispatch on rdev via
    // `linux_driver_abi::tty::open_special_tty`, so a `mknod(2)`-created
    // `/dev/ptmx`-alike needs this to actually reach the right driver.
    if matches!(kind, InodeKind::Chardev | InodeKind::Blockdev) {
        inode.rdev.store(dev as u64, Ordering::Release);
    }
    *inode.sb.lock() = Some(sb);
    match &dir.private {
        InodePrivate::RamDir(children) => {
            children.lock().insert(String::from(name), inode.clone());
        }
        _ => return Err(ENOSYS),
    }
    let dentry = super::dcache::d_lookup(parent, name)
        .unwrap_or_else(|| super::dcache::d_alloc_child(parent, name));
    dentry.instantiate(inode);
    Ok(())
}

pub unsafe fn sys_mknodat(dirfd: i32, pathname: *const u8, mode: u32, dev: u32) -> i64 {
    let path = match unsafe { user_path(pathname) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    let ret = (|| {
        let kind = mknod_kind(mode)?;
        let create_mode = mode & !super::fs_struct::current_umask();
        let (root, start) = root_and_start(dirfd)?;
        let (parent_path, last) = split_last(&path);
        if last.is_empty() || last == "." || last == ".." {
            return Err(EINVAL);
        }
        let base_hint = dirfd_base_hint(dirfd, &start)?;
        let parent = mkdir_parent_dentry(&root, &start, &path, parent_path, base_hint)?;
        let dir = parent.inode().ok_or(ENOENT)?;
        if dir.kind != InodeKind::Directory {
            return Err(ENOTDIR);
        }
        if super::dcache::d_lookup(&parent, last)
            .and_then(|dentry| dentry.inode())
            .is_some()
            || dir
                .ops
                .lookup
                .is_some_and(|lookup| lookup(&dir, last).is_ok())
        {
            return Err(EEXIST);
        }
        if kind == InodeKind::Regular {
            let create = dir.ops.create.ok_or(ENOSYS)?;
            let inode = create(&dir, last, create_mode & 0o7777)?;
            let child = super::dcache::d_lookup(&parent, last)
                .unwrap_or_else(|| super::dcache::d_alloc_child(&parent, last));
            child.instantiate(inode);
            super::inotify::notify_create(&parent, last, false);
            return Ok(());
        }
        insert_special_node(&parent, &dir, last, create_mode, kind, dev)?;
        super::inotify::notify_create(&parent, last, kind == InodeKind::Directory);
        Ok(())
    })()
    .map(|_| 0)
    .unwrap_or_else(|errno| -(errno as i64));
    trace_run_path("mknodat", dirfd, &path, mode as i32, ret);
    ret
}

pub unsafe fn sys_mknod(pathname: *const u8, mode: u32, dev: u32) -> i64 {
    unsafe { sys_mknodat(AT_FDCWD, pathname, mode, dev) }
}

pub fn sys_ustat(_dev: u32, ubuf: *mut u8) -> i64 {
    if ubuf.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_pivot_root(new_root: *const u8, put_old: *const u8) -> i64 {
    if new_root.is_null() || put_old.is_null() {
        return -(EFAULT as i64);
    }
    if !capable(CAP_SYS_ADMIN) {
        return -(EPERM as i64);
    }
    let new_root_name = match unsafe { user_path(new_root) } {
        Ok(path) if !path.is_empty() => path,
        Ok(_) => return -(ENOENT as i64),
        Err(errno) => return -(errno as i64),
    };
    let put_old_name = match unsafe { user_path(put_old) } {
        Ok(path) if !path.is_empty() => path,
        Ok(_) => return -(ENOENT as i64),
        Err(errno) => return -(errno as i64),
    };
    let new_root_path = match lookup_path_str_with_follow(AT_FDCWD, &new_root_name, true) {
        Ok(path) => mount::VfsPath::new(path.mount, path.dentry),
        Err(errno) => return -(errno as i64),
    };
    let put_old_path = match lookup_path_str_with_follow(AT_FDCWD, &put_old_name, true) {
        Ok(path) => mount::VfsPath::new(path.mount, path.dentry),
        Err(errno) => return -(errno as i64),
    };
    if new_root_path
        .dentry
        .inode()
        .is_none_or(|inode| inode.kind != InodeKind::Directory)
        || put_old_path
            .dentry
            .inode()
            .is_none_or(|inode| inode.kind != InodeKind::Directory)
    {
        return -(ENOTDIR as i64);
    }
    match mount::pivot_root_paths(&new_root_path, &put_old_path) {
        Ok((old_root, new_root)) => {
            super::fs_struct::chroot_fs_refs(&old_root, &new_root);
            let fs = super::fs_struct::current_fs();
            if !fs.is_null() {
                let pwd = unsafe { &*fs }.pwd.lock().clone();
                let cwd = pwd
                    .map(|pwd| super::fs_struct::visible_path_for_current_root(&pwd))
                    .unwrap_or_else(|| String::from("/"));
                super::fs_struct::set_current_cwd_path(&cwd);
            }
            0
        }
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_chroot(pathname: *const u8) -> i64 {
    if !capable(CAP_SYS_CHROOT) {
        return -(EPERM as i64);
    }
    let path = match unsafe { user_path(pathname) } {
        Ok(path) if !path.is_empty() => path,
        Ok(_) => return -(ENOENT as i64),
        Err(errno) => return -(errno as i64),
    };
    let target = match lookup_path_str_with_follow(AT_FDCWD, &path, true) {
        Ok(target) => target,
        Err(errno) => return -(errno as i64),
    };
    if target
        .dentry
        .inode()
        .map(|inode| inode.kind != InodeKind::Directory)
        .unwrap_or(true)
    {
        return -(ENOTDIR as i64);
    }
    let fs = super::fs_struct::current_fs();
    if fs.is_null() {
        return -(EPERM as i64);
    }
    let fs_ref = unsafe { &*fs };
    // Linux chroot(2) changes only current->fs.  chroot_fs_refs() is the
    // all-task helper used by namespace root transitions (for example
    // pivot_root), not by fs/open.c::chroot.
    super::fs_struct::set_fs_root_path(fs_ref, mount::VfsPath::new(target.mount, target.dentry));
    let pwd = fs_ref.pwd.lock().clone();
    let cwd = pwd
        .map(|pwd| super::fs_struct::visible_path_for_current_root(&pwd))
        .unwrap_or_else(|| String::from("/"));
    super::fs_struct::set_current_cwd_path(&cwd);
    0
}

pub fn sys_umount2(target: *const u8, flags: i32) -> i64 {
    let flags = flags as u32;
    const SUPPORTED_UMOUNT_FLAGS: u32 = MNT_DETACH | UMOUNT_NOFOLLOW;
    if flags & !SUPPORTED_UMOUNT_FLAGS != 0 || flags & (MNT_FORCE | MNT_EXPIRE) != 0 {
        return -(EINVAL as i64);
    }
    if !capable(CAP_SYS_ADMIN) {
        return -(EPERM as i64);
    }
    if target.is_null() {
        return -(EFAULT as i64);
    }
    let path = match unsafe { user_path(target) } {
        Ok(path) if !path.is_empty() => path,
        Ok(_) => return -(ENOENT as i64),
        Err(errno) => return -(errno as i64),
    };
    match mount::do_umount(&path, flags) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_inotify_init() -> i64 {
    crate::fs::inotify::sys_inotify_init1(0)
}

pub fn sys_splice(
    _fd_in: i32,
    _off_in: *mut i64,
    _fd_out: i32,
    _off_out: *mut i64,
    len: usize,
    flags: u32,
) -> i64 {
    if flags & !0xf != 0 {
        return -(EINVAL as i64);
    }
    if len == 0 {
        return 0;
    }
    -(ENOSYS as i64)
}

pub fn sys_tee(_fd_in: i32, _fd_out: i32, len: usize, flags: u32) -> i64 {
    if flags & !0xf != 0 {
        return -(EINVAL as i64);
    }
    if len == 0 {
        return 0;
    }
    -(ENOSYS as i64)
}

pub fn sys_vmsplice(fd: i32, iov: *const IoVec, nr_segs: usize, flags: u32) -> i64 {
    const UIO_MAXIOV: usize = 1024;
    const MAX_RW_COUNT: usize = 0x7fff_f000;
    if flags & !0xf != 0 {
        return -(EINVAL as i64);
    }
    if nr_segs == 0 {
        return 0;
    }
    if iov.is_null() {
        return -(EFAULT as i64);
    }
    if nr_segs > UIO_MAXIOV {
        return -(EINVAL as i64);
    }
    let file = match current_files().and_then(|files| files.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };

    let mut total = 0usize;
    for idx in 0..nr_segs {
        let mut entry = IoVec::default();
        let not_copied = unsafe {
            uaccess::copy_from_user(
                (&mut entry as *mut IoVec).cast::<u8>(),
                unsafe { iov.add(idx) }.cast::<u8>(),
                core::mem::size_of::<IoVec>(),
            )
        };
        if not_copied != 0 {
            return if total > 0 {
                total as i64
            } else {
                -(EFAULT as i64)
            };
        }
        if entry.iov_len == 0 {
            continue;
        }
        if entry.iov_base.is_null() {
            return if total > 0 {
                total as i64
            } else {
                -(EFAULT as i64)
            };
        }
        let task = unsafe { sched::get_current() };
        if !task.is_null() {
            let mm = unsafe { (*task).mm };
            if !mm.is_null()
                && unsafe {
                    crate::mm::mmap::range_contains_secretmem(
                        &*mm,
                        entry.iov_base as u64,
                        entry.iov_len,
                    )
                }
            {
                return if total > 0 {
                    total as i64
                } else {
                    -(EFAULT as i64)
                };
            }
        }
        if total.saturating_add(entry.iov_len) > MAX_RW_COUNT {
            return if total > 0 {
                total as i64
            } else {
                -(EINVAL as i64)
            };
        }

        const CHUNK: usize = 4096;
        let mut done = 0usize;
        while done < entry.iov_len {
            let this = (entry.iov_len - done).min(CHUNK);
            let mut kbuf = alloc::vec![0u8; this];
            let not_copied = unsafe {
                uaccess::copy_from_user(
                    kbuf.as_mut_ptr(),
                    unsafe { entry.iov_base.add(done) },
                    this,
                )
            };
            let copied = this - not_copied;
            if copied == 0 {
                return if total > 0 {
                    total as i64
                } else {
                    -(EFAULT as i64)
                };
            }
            kbuf.truncate(copied);
            match vfs_write(&file, &kbuf) {
                Ok(n) => {
                    total += n;
                    done += n;
                    if n < copied {
                        return total as i64;
                    }
                }
                Err(errno) => {
                    return if total > 0 {
                        total as i64
                    } else {
                        -(errno as i64)
                    };
                }
            }
            if not_copied != 0 {
                break;
            }
        }
    }
    total as i64
}

static MEMFD_FILE_OPS: super::ops::FileOps = super::ops::FileOps {
    name: "memfd",
    read: Some(memfd_file_read),
    write: Some(memfd_file_write),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

pub(crate) static SECRETMEM_FILE_OPS: super::ops::FileOps = super::ops::FileOps {
    name: "secretmem",
    read: Some(secretmem_file_io_blocked),
    write: Some(secretmem_file_write_blocked),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

const UFFD_API: u64 = 0xAA;
const UFFDIO_API: u32 = 0xC018_AA3F;
const UFFDIO_REGISTER: u32 = 0xC020_AA00;
const UFFDIO_UNREGISTER: u32 = 0x8010_AA01;
const UFFDIO_WAKE: u32 = 0x8010_AA02;
const UFFDIO_COPY: u32 = 0xC028_AA03;
const UFFDIO_ZEROPAGE: u32 = 0xC020_AA04;
const UFFDIO_COPY_MODE_DONTWAKE: u64 = 1 << 0;
const UFFDIO_REGISTER_MODE_MISSING: u64 = 1 << 0;
const UFFDIO_REGISTER_MODE_WP: u64 = 1 << 1;
const UFFDIO_REGISTER_MODE_MINOR: u64 = 1 << 2;
const UFFD_API_IOCTLS: u64 = (1u64 << 0) | (1u64 << 1) | (1u64 << 63);
const UFFD_API_SUPPORTED_FEATURES: u64 = 0;
const UFFD_API_RANGE_IOCTLS: u64 = (1u64 << 2) | (1u64 << 3);
const USERFAULTFD_API_INITIALIZED: usize = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct UffdioApi {
    api: u64,
    features: u64,
    ioctls: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct UffdioRange {
    start: u64,
    len: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct UffdioRegister {
    range: UffdioRange,
    mode: u64,
    ioctls: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct UffdioCopy {
    dst: u64,
    src: u64,
    len: u64,
    mode: u64,
    copy: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct UffdioZeropage {
    range: UffdioRange,
    mode: u64,
    zeropage: i64,
}

static USERFAULTFD_FILE_OPS: super::ops::FileOps = super::ops::FileOps {
    name: "userfaultfd",
    read: Some(userfaultfd_file_read),
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: Some(userfaultfd_file_poll),
    ioctl: Some(userfaultfd_ioctl),
    mmap: None,
    release: None,
    readdir: None,
};

fn copy_struct_from_user<T: Default + Copy>(arg: u64) -> Result<T, i32> {
    if arg == 0 {
        return Err(EFAULT);
    }
    let mut value = T::default();
    let left = unsafe {
        uaccess::copy_from_user(
            (&mut value as *mut T).cast::<u8>(),
            arg as *const u8,
            core::mem::size_of::<T>(),
        )
    };
    if left != 0 { Err(EFAULT) } else { Ok(value) }
}

fn copy_struct_to_user<T: Copy>(arg: u64, value: &T) -> Result<(), i32> {
    if arg == 0 {
        return Err(EFAULT);
    }
    let left = unsafe {
        uaccess::copy_to_user(
            arg as *mut u8,
            (value as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if left != 0 { Err(EFAULT) } else { Ok(()) }
}

fn userfaultfd_file_read(_file: &FileRef, _buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    Err(ENOSYS)
}

fn userfaultfd_file_poll(
    _file: &FileRef,
    _table: Option<&mut crate::fs::select::PollTable>,
) -> u32 {
    POLLIN as u32
}

fn userfaultfd_copy(copy: &mut UffdioCopy) -> Result<(), i32> {
    let page_size = crate::arch::x86::mm::paging::PAGE_SIZE;
    if copy.len == 0
        || copy.dst & (page_size - 1) != 0
        || copy.len & (page_size - 1) != 0
        || copy.mode & !UFFDIO_COPY_MODE_DONTWAKE != 0
    {
        copy.copy = -(EINVAL as i64);
        return Err(EINVAL);
    }

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        copy.copy = -(EFAULT as i64);
        return Err(EFAULT);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        copy.copy = -(EFAULT as i64);
        return Err(EFAULT);
    }

    let pages = (copy.len / page_size) as usize;
    let mut copied = 0u64;
    for idx in 0..pages {
        let dst = copy
            .dst
            .checked_add((idx as u64).saturating_mul(page_size))
            .ok_or(EINVAL)?;
        let src = copy
            .src
            .checked_add((idx as u64).saturating_mul(page_size))
            .ok_or(EINVAL)?;

        if crate::mm::gup::get_user_pages_fast(unsafe { &*mm }, dst, 1, 0).is_ok() {
            copy.copy = -(EEXIST as i64);
            return Err(EEXIST);
        }

        let mut page: *mut crate::mm::page::Page = core::ptr::null_mut();
        let got = unsafe {
            crate::mm::gup::get_user_pages_remote(
                mm,
                dst,
                1,
                crate::mm::gup::FOLL_WRITE,
                &mut page as *mut *mut crate::mm::page::Page,
                core::ptr::null_mut(),
            )
        };
        if got != 1 || page.is_null() {
            copy.copy = -(EFAULT as i64);
            return Err(EFAULT);
        }

        let pfn = crate::mm::buddy::page_to_pfn(page);
        let dst_ptr = crate::arch::x86::mm::paging::pfn_to_virt(pfn);
        let left =
            unsafe { uaccess::copy_from_user(dst_ptr, src as *const u8, page_size as usize) };
        unsafe {
            (*page).put_page();
        }
        if left != 0 {
            copy.copy = -(EFAULT as i64);
            return Err(EFAULT);
        }
        copied = copied.saturating_add(page_size);
    }

    copy.copy = copied as i64;
    Ok(())
}

fn userfaultfd_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    match cmd {
        UFFDIO_API => {
            let mut api: UffdioApi = copy_struct_from_user(arg)?;
            if api.api != UFFD_API {
                return Err(EINVAL);
            }
            if *file.private.lock() == USERFAULTFD_API_INITIALIZED {
                return Err(EINVAL);
            }
            if api.features & !UFFD_API_SUPPORTED_FEATURES != 0 {
                return Err(EINVAL);
            }
            api.features = UFFD_API_SUPPORTED_FEATURES;
            api.ioctls = UFFD_API_IOCTLS;
            copy_struct_to_user(arg, &api)?;
            *file.private.lock() = USERFAULTFD_API_INITIALIZED;
            Ok(0)
        }
        UFFDIO_REGISTER => {
            let mut reg: UffdioRegister = copy_struct_from_user(arg)?;
            let known_modes =
                UFFDIO_REGISTER_MODE_MISSING | UFFDIO_REGISTER_MODE_WP | UFFDIO_REGISTER_MODE_MINOR;
            if reg.mode == 0 || reg.mode & !known_modes != 0 {
                return Err(EINVAL);
            }
            crate::mm::shmem::userfaultfd_register(
                reg.range.start,
                reg.range.len,
                reg.mode & UFFDIO_REGISTER_MODE_MISSING != 0,
                reg.mode & UFFDIO_REGISTER_MODE_WP != 0,
            )?;
            reg.ioctls = UFFD_API_RANGE_IOCTLS;
            copy_struct_to_user(arg, &reg)?;
            Ok(0)
        }
        UFFDIO_UNREGISTER => {
            let range: UffdioRange = copy_struct_from_user(arg)?;
            crate::mm::shmem::userfaultfd_unregister_range(range.start, range.len);
            Ok(0)
        }
        UFFDIO_WAKE => {
            let _range: UffdioRange = copy_struct_from_user(arg)?;
            Ok(0)
        }
        UFFDIO_COPY => {
            let mut copy: UffdioCopy = copy_struct_from_user(arg)?;
            let result = userfaultfd_copy(&mut copy);
            copy_struct_to_user(arg, &copy)?;
            result.map(|()| 0)
        }
        UFFDIO_ZEROPAGE => {
            let mut zeropage: UffdioZeropage = copy_struct_from_user(arg)?;
            zeropage.zeropage = -(EINVAL as i64);
            copy_struct_to_user(arg, &zeropage)?;
            Err(EINVAL)
        }
        _ => Err(ENOTTY),
    }
}

fn memfd_id(file: &FileRef) -> Result<u64, i32> {
    if file.fops.name != MEMFD_FILE_OPS.name {
        return Err(EINVAL);
    }
    Ok(*file.private.lock() as u64)
}

fn secretmem_id(file: &FileRef) -> Result<u64, i32> {
    if file.fops.name != SECRETMEM_FILE_OPS.name {
        return Err(EINVAL);
    }
    Ok(*file.private.lock() as u64)
}

pub(crate) unsafe fn zero_file_range_raw(
    file: usize,
    offset: u64,
    len: u64,
    keep_size: bool,
) -> Result<(), i32> {
    if file == 0 {
        return Err(EINVAL);
    }
    let offset_usize = if offset > usize::MAX as u64 {
        return Err(EINVAL);
    } else {
        offset as usize
    };
    let len_usize = if len > usize::MAX as u64 {
        return Err(EINVAL);
    } else {
        len as usize
    };

    let file_ptr = file as *const crate::fs::types::File;
    unsafe {
        Arc::increment_strong_count(file_ptr);
    }
    let file = unsafe { Arc::from_raw(file_ptr) };
    if file.fops.name == MEMFD_FILE_OPS.name {
        let id = memfd_id(&file)?;
        return crate::mm::shmem::with_memfd_mut(id, |obj| {
            obj.zero_range(offset_usize, len_usize, keep_size)
        })
        .ok_or(EBADF)?;
    }

    let inode = file.inode().ok_or(EBADF)?;
    if matches!(&inode.private, super::types::InodePrivate::RamBytes(_)) {
        return super::libfs::ram_file_zero_range(&inode, offset, len, keep_size);
    }

    let Some(write) = file.fops.write else {
        return Err(EINVAL);
    };
    let zeroes = [0u8; crate::arch::x86::mm::paging::PAGE_SIZE as usize];
    let old_size = inode.size.load(Ordering::Acquire);
    let mut done = 0u64;
    while done < len {
        let chunk = (len - done).min(zeroes.len() as u64) as usize;
        let mut pos = offset + done;
        let written = write(&file, &zeroes[..chunk], &mut pos)?;
        if written != chunk {
            return Err(EIO);
        }
        done += chunk as u64;
    }
    if keep_size {
        inode.size.store(old_size, Ordering::Release);
    }
    Ok(())
}

pub(crate) unsafe fn mark_file_hwpoison_raw(file: usize, offset: u64, len: u64) -> Result<(), i32> {
    if file == 0 {
        return Err(EINVAL);
    }
    let offset_usize = if offset > usize::MAX as u64 {
        return Err(EINVAL);
    } else {
        offset as usize
    };
    let len_usize = if len > usize::MAX as u64 {
        return Err(EINVAL);
    } else {
        len as usize
    };

    let file_ptr = file as *const crate::fs::types::File;
    unsafe {
        Arc::increment_strong_count(file_ptr);
    }
    let file = unsafe { Arc::from_raw(file_ptr) };
    if file.fops.name != MEMFD_FILE_OPS.name {
        return Err(EINVAL);
    }
    let id = memfd_id(&file)?;
    crate::mm::shmem::with_memfd_mut(id, |obj| obj.hwpoison_range(offset_usize, len_usize))
        .ok_or(EBADF)?
}

fn memfd_file_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let task = unsafe { sched::get_current() };
    if !task.is_null() {
        let mm = unsafe { (*task).mm };
        if !mm.is_null() {
            let file_ptr = Arc::as_ptr(file) as usize;
            unsafe {
                crate::mm::mmap::sync_shared_file_mapping(
                    &mut *mm,
                    file_ptr,
                    *pos,
                    buf.len() as u64,
                )?;
            }
        }
    }
    let id = memfd_id(file)?;
    let read_len = if let Some(object) = crate::mm::shmem::memfd_object(id) {
        let start = if *pos > usize::MAX as u64 {
            return Err(EINVAL);
        } else {
            *pos as usize
        };
        match object.first_hwpoison_offset(start, buf.len()) {
            Some(poison) if poison <= start => return Ok(0),
            Some(poison) => poison - start,
            None => buf.len(),
        }
    } else {
        buf.len()
    };
    super::libfs::ram_file_read(file, &mut buf[..read_len], pos)
}

fn secretmem_file_io_blocked(
    _file: &FileRef,
    _buf: &mut [u8],
    _pos: &mut u64,
) -> Result<usize, i32> {
    Err(EINVAL)
}

fn secretmem_file_write_blocked(
    _file: &FileRef,
    _buf: &[u8],
    _pos: &mut u64,
) -> Result<usize, i32> {
    Err(EINVAL)
}

fn memfd_file_write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let id = memfd_id(file)?;
    let inode = file.inode().ok_or(EBADF)?;
    let start = *pos as usize;
    let end = start.checked_add(buf.len()).ok_or(EINVAL)?;
    let old_len = inode.size.load(Ordering::Acquire) as usize;
    let seals = crate::mm::shmem::memfd_object(id).ok_or(EBADF)?.seals();
    if seals & (crate::mm::shmem::F_SEAL_WRITE | crate::mm::shmem::F_SEAL_FUTURE_WRITE) != 0 {
        return Err(EPERM);
    }
    if end > old_len && seals & crate::mm::shmem::F_SEAL_GROW != 0 {
        return Err(EPERM);
    }
    let written = super::libfs::ram_file_write(file, buf, pos)?;
    let new_len = inode.size.load(Ordering::Acquire) as usize;
    crate::mm::shmem::with_memfd_mut(id, |obj| obj.resize(new_len)).ok_or(EBADF)??;
    Ok(written)
}

pub fn sys_memfd_secret(flags: u32) -> i64 {
    let id = match crate::mm::shmem::create_secretmem(flags, 0) {
        Ok(id) => id,
        Err(errno) => return -(errno as i64),
    };
    let dentry = super::dcache::d_alloc("secretmem");
    let inode = super::types::Inode::new(
        id,
        super::types::InodeKind::Regular,
        0o600,
        &super::ops::NOOP_INODE_OPS,
        &SECRETMEM_FILE_OPS,
        super::libfs::empty_ram_bytes(),
    );
    dentry.instantiate(inode);
    let file = super::file::alloc_file(dentry, O_RDWR, 0o600, &SECRETMEM_FILE_OPS);
    *file.private.lock() = id as usize;
    match current_files().and_then(|ft| ft.install(file, false)) {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub unsafe fn sys_memfd_create(name: *const u8, flags: u32) -> i64 {
    if let Err(errno) = crate::mm::shmem::validate_memfd_flags(flags) {
        return -(errno as i64);
    }
    let uname = match unsafe { user_path(name) } {
        Ok(name) => name,
        Err(errno) => return -(errno as i64),
    };
    if uname.len() > 249 {
        return -(EINVAL as i64);
    }
    let id = match crate::mm::shmem::create_memfd(flags) {
        Ok(id) => id,
        Err(errno) => return -(errno as i64),
    };
    let dentry_name = if uname.is_empty() {
        String::from("memfd:")
    } else {
        let mut name = String::from("memfd:");
        name.push_str(&uname);
        name
    };
    let dentry = super::dcache::d_alloc(&dentry_name);
    let inode = super::types::Inode::new(
        id,
        super::types::InodeKind::Regular,
        0o600,
        &super::ops::NOOP_INODE_OPS,
        &MEMFD_FILE_OPS,
        super::libfs::empty_ram_bytes(),
    );
    if flags & crate::mm::shmem::MFD_HUGETLB != 0 {
        const HUGETLBFS_MAGIC: u64 = 0x9584_58f6;
        let sb = super::types::SuperBlock::alloc(
            "hugetlbfs",
            HUGETLBFS_MAGIC,
            &super::ops::NOOP_SUPER_OPS,
        );
        *inode.sb.lock() = Some(sb);
    }
    dentry.instantiate(inode);
    let file = super::file::alloc_file(dentry, O_RDWR, 0o600, &MEMFD_FILE_OPS);
    *file.private.lock() = id as usize;
    match current_files()
        .and_then(|ft| ft.install(file, flags & crate::mm::shmem::MFD_CLOEXEC != 0))
    {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_userfaultfd(flags: i32) -> i64 {
    if let Err(errno) = crate::mm::shmem::validate_userfaultfd_flags(flags) {
        return -(errno as i64);
    }
    let file = crate::fs::anon_inode::alloc_anon_file("userfaultfd", &USERFAULTFD_FILE_OPS, 0);
    match current_files()
        .and_then(|ft| ft.install(file, flags & crate::mm::shmem::UFFD_CLOEXEC != 0))
    {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_copy_file_range(
    fd_in: i32,
    _off_in: *mut i64,
    fd_out: i32,
    _off_out: *mut i64,
    len: usize,
    flags: u32,
) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    if len == 0 {
        return 0;
    }
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    if files.get(fd_in).is_err() || files.get(fd_out).is_err() {
        return -(EBADF as i64);
    }
    -(ENOSYS as i64)
}

#[derive(Clone)]
struct MountFsContext {
    fs_name: String,
    source: String,
    params: Vec<(String, Option<String>)>,
    created: bool,
}

enum MountApiObject {
    FsContext(MountFsContext),
    DetachedMount(Arc<mount::Mount>),
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MountAttr {
    attr_set: u64,
    attr_clr: u64,
    propagation: u64,
    userns_fd: u64,
}

static MOUNT_API_TOKEN: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref MOUNT_API_OBJECTS: Mutex<BTreeMap<usize, MountApiObject>> =
        Mutex::new(BTreeMap::new());
}

static FS_CONTEXT_FILE_OPS: super::ops::FileOps = super::ops::FileOps {
    name: "fs_context",
    read: Some(fs_context_file_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: Some(mount_api_file_release),
    readdir: None,
};

static DETACHED_MOUNT_FILE_OPS: super::ops::FileOps = super::ops::FileOps {
    name: "detached_mount",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: Some(mount_api_file_release),
    readdir: None,
};

fn fs_context_file_read(_file: &FileRef, _buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    Err(ENODATA)
}

fn mount_api_file_release(file: FileRef) {
    let token = *file.private.lock();
    if token != 0 {
        MOUNT_API_OBJECTS.lock().remove(&token);
    }
}

fn mount_api_token(file: &FileRef, fops_name: &str) -> Result<usize, i32> {
    if file.fops.name != fops_name {
        return Err(EBADF);
    }
    let token = *file.private.lock();
    if token == 0 {
        return Err(EBADF);
    }
    Ok(token)
}

fn mount_api_context_from_fd(fd: i32) -> Result<(usize, MountFsContext), i32> {
    let file = current_files()?.get(fd)?;
    let token = mount_api_token(&file, FS_CONTEXT_FILE_OPS.name)?;
    let objects = MOUNT_API_OBJECTS.lock();
    match objects.get(&token) {
        Some(MountApiObject::FsContext(ctx)) => Ok((token, ctx.clone())),
        _ => Err(EBADF),
    }
}

fn detached_mount_from_fd(fd: i32) -> Result<Arc<mount::Mount>, i32> {
    let file = current_files()?.get(fd)?;
    let token = mount_api_token(&file, DETACHED_MOUNT_FILE_OPS.name)?;
    let objects = MOUNT_API_OBJECTS.lock();
    match objects.get(&token) {
        Some(MountApiObject::DetachedMount(mount)) => Ok(mount.clone()),
        _ => Err(EBADF),
    }
}

fn context_data_string(ctx: &MountFsContext) -> String {
    let mut out = String::new();
    for (key, value) in ctx.params.iter() {
        if key == "source" {
            continue;
        }
        if !out.is_empty() {
            out.push(',');
        }
        out.push_str(key);
        if let Some(value) = value {
            out.push('=');
            out.push_str(value);
        }
    }
    out
}

fn mount_flags_from_attr_bits(bits: u64) -> u64 {
    let mut flags = 0u64;
    if bits & MOUNT_ATTR_RDONLY != 0 {
        flags |= MS_RDONLY;
    }
    if bits & MOUNT_ATTR_NOSUID != 0 {
        flags |= MS_NOSUID;
    }
    if bits & MOUNT_ATTR_NODEV != 0 {
        flags |= MS_NODEV;
    }
    if bits & MOUNT_ATTR_NOEXEC != 0 {
        flags |= MS_NOEXEC;
    }
    if bits & MOUNT_ATTR_NOATIME != 0 {
        flags |= MS_NOATIME;
    }
    if bits & MOUNT_ATTR_STRICTATIME != 0 {
        flags |= MS_STRICTATIME;
    }
    if bits & MOUNT_ATTR_NODIRATIME != 0 {
        flags |= MS_NODIRATIME;
    }
    if bits & MOUNT_ATTR_NOSYMFOLLOW != 0 {
        flags |= MS_NOSYMFOLLOW;
    }
    flags
}

fn validate_mount_attr(attr: &MountAttr) -> Result<(), i32> {
    if attr.attr_set & !MOUNT_ATTR_SUPPORTED != 0 || attr.attr_clr & !MOUNT_ATTR_SUPPORTED != 0 {
        return Err(EINVAL);
    }
    let atime_clr = attr.attr_clr & MOUNT_ATTR__ATIME;
    if atime_clr != 0 && atime_clr != MOUNT_ATTR__ATIME {
        return Err(EINVAL);
    }
    match attr.attr_set & MOUNT_ATTR__ATIME {
        0 | MOUNT_ATTR_NOATIME | MOUNT_ATTR_STRICTATIME => {}
        _ => return Err(EINVAL),
    }
    if attr.propagation != 0 || attr.userns_fd != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

fn require_sys_admin() -> Result<(), i32> {
    if capable(CAP_SYS_ADMIN) {
        Ok(())
    } else {
        Err(EPERM)
    }
}

fn apply_mount_attr(mount: &Arc<mount::Mount>, attr: &MountAttr) -> Result<(), i32> {
    validate_mount_attr(attr)?;
    let mut clear = mount_flags_from_attr_bits(attr.attr_clr);
    if attr.attr_clr & MOUNT_ATTR__ATIME != 0 {
        clear |= MS_NOATIME | MS_NODIRATIME | MS_STRICTATIME;
    }
    let set = mount_flags_from_attr_bits(attr.attr_set);
    let old = mount.flags.load(Ordering::Acquire) as u64;
    let new = (old & !clear) | set;
    mount.flags.store(new as u32, Ordering::Release);
    Ok(())
}

fn apply_mount_attr_recursive(mount: &Arc<mount::Mount>, attr: &MountAttr) -> Result<(), i32> {
    apply_mount_attr(mount, attr)?;
    let children = mount.children.lock().clone();
    for child in children {
        apply_mount_attr_recursive(&child, attr)?;
    }
    Ok(())
}

fn copy_mount_attr(attr: *const u8, size: usize) -> Result<MountAttr, i32> {
    if size < MOUNT_ATTR_SIZE_VER0 {
        return Err(EINVAL);
    }
    if attr.is_null() {
        return Err(EFAULT);
    }
    let mut out = MountAttr::default();
    let len = core::mem::size_of::<MountAttr>().min(size);
    let not_copied =
        unsafe { uaccess::copy_from_user((&mut out as *mut MountAttr).cast::<u8>(), attr, len) };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(out)
}

fn target_path_from_dirfd(dirfd: i32, path: &str) -> Result<String, i32> {
    if path.starts_with('/') || dirfd == AT_FDCWD {
        return Ok(super::fs_struct::absolute_from_cwd(path));
    }
    let file = current_files()?.get(dirfd)?;
    let base = super::file::path_hint(&file)
        .or_else(|| mount::path_for_dentry(&file.dentry))
        .unwrap_or_else(|| super::file::dentry_path(&file.dentry));
    Ok(join_path(&base, path))
}

fn close_fd_safely(fd: i32) {
    if let Ok(ft) = current_files() {
        let _ = ft.close(fd);
    }
}

fn open_tree_target(dfd: i32, path: &str, flags: u32) -> Result<StatTarget, i32> {
    if path.is_empty() {
        if flags & AT_EMPTY_PATH == 0 {
            return Err(ENOENT);
        }
        return lookup_empty_stat_target(dfd);
    }

    let follow_final = flags & AT_SYMLINK_NOFOLLOW == 0;
    lookup_path_str_with_follow(dfd, path, follow_final)
}

fn install_detached_mount_file(mount: Arc<mount::Mount>, cloexec: bool) -> Result<i32, i32> {
    let token = MOUNT_API_TOKEN.fetch_add(1, Ordering::AcqRel) as usize;
    MOUNT_API_OBJECTS
        .lock()
        .insert(token, MountApiObject::DetachedMount(mount.clone()));
    let file = super::file::alloc_file(mount.root.clone(), O_PATH, 0, &DETACHED_MOUNT_FILE_OPS);
    *file.private.lock() = token;
    match current_files().and_then(|ft| ft.install(file, cloexec)) {
        Ok(fd) => Ok(fd as i32),
        Err(errno) => {
            MOUNT_API_OBJECTS.lock().remove(&token);
            Err(errno)
        }
    }
}

pub fn sys_open_tree(dfd: i32, filename: *const u8, flags: u32) -> i64 {
    let valid_flags = AT_EMPTY_PATH
        | AT_NO_AUTOMOUNT
        | AT_RECURSIVE
        | AT_SYMLINK_NOFOLLOW
        | OPEN_TREE_CLONE
        | OPEN_TREE_NAMESPACE
        | OPEN_TREE_CLOEXEC;
    if flags & !valid_flags != 0 {
        return -(EINVAL as i64);
    }
    if flags & AT_RECURSIVE != 0 && flags & (OPEN_TREE_CLONE | OPEN_TREE_NAMESPACE) == 0 {
        return -(EINVAL as i64);
    }
    if flags & OPEN_TREE_CLONE != 0 && flags & OPEN_TREE_NAMESPACE != 0 {
        return -(EINVAL as i64);
    }
    if flags & (OPEN_TREE_CLONE | OPEN_TREE_NAMESPACE) != 0 {
        if let Err(errno) = require_sys_admin() {
            return -(errno as i64);
        }
    }

    let path = match unsafe { user_path(filename) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    let target = match open_tree_target(dfd, &path, flags) {
        Ok(target) => target,
        Err(errno) => return -(errno as i64),
    };

    if flags & (OPEN_TREE_CLONE | OPEN_TREE_NAMESPACE) != 0 {
        let clone_root = target.dentry.clone();
        let clone_flags = target.mount.flags.load(Ordering::Acquire);
        let mount = mount::Mount::alloc(target.mount.sb.clone(), clone_root, clone_flags);
        return install_detached_mount_file(mount, flags & OPEN_TREE_CLOEXEC != 0)
            .map(|fd| fd as i64)
            .unwrap_or_else(|errno| -(errno as i64));
    }

    let inode = match target.dentry.inode() {
        Some(inode) => inode,
        None => return -(EINVAL as i64),
    };
    let file = super::file::alloc_file(target.dentry.clone(), O_PATH, 0, inode.fops);
    match current_files().and_then(|ft| ft.install(file, flags & OPEN_TREE_CLOEXEC != 0)) {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_move_mount(
    from_dfd: i32,
    from_path: *const u8,
    to_dfd: i32,
    to_path: *const u8,
    flags: u32,
) -> i64 {
    if flags & !MOVE_MOUNT_MASK != 0 {
        return -(EINVAL as i64);
    }
    if from_path.is_null() || to_path.is_null() {
        return -(EFAULT as i64);
    }
    if let Err(errno) = require_sys_admin() {
        return -(errno as i64);
    }
    let from_path = match unsafe { user_path(from_path) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    let to_path = match unsafe { user_path(to_path) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    if from_path.is_empty() && flags & MOVE_MOUNT_F_EMPTY_PATH != 0 {
        let mount = match detached_mount_from_fd(from_dfd) {
            Ok(mount) => mount,
            Err(errno) => return -(errno as i64),
        };
        let target = match target_path_from_dirfd(to_dfd, &to_path) {
            Ok(target) if !target.is_empty() => target,
            Ok(_) => return -(ENOENT as i64),
            Err(errno) => return -(errno as i64),
        };
        return mount::attach_mount(mount, &target)
            .map(|_| 0)
            .unwrap_or_else(|errno| -(errno as i64));
    }
    -(EPERM as i64)
}

/// `fsopen(2)` — open a file-system context handle.
///
/// The new mount API entry point.  Ref:
///   - vendor/linux/fs/fsopen.c::SYSCALL_DEFINE2(fsopen, …)
///   - vendor/linux/Documentation/filesystems/mount_api.rst
///   - vendor/linux/include/uapi/linux/mount.h (`FSOPEN_CLOEXEC`).
///
/// `sys_memfd_create` (below) and the rest of the mount-API surface
/// (`fsmount`, `fsconfig`, `move_mount`, `open_tree`) share the same
/// vendor/linux references — they're all part of the Linux 5.2 mount API
/// rewrite.
pub unsafe fn sys_fsopen(fs_name: *const u8, flags: u32) -> i64 {
    if flags & !FSOPEN_CLOEXEC != 0 {
        return -(EINVAL as i64);
    }
    if let Err(errno) = require_sys_admin() {
        return -(errno as i64);
    }
    let fs_name = match unsafe { user_path(fs_name) } {
        Ok(name) => name,
        Err(errno) => return -(errno as i64),
    };
    if super::super_block::lookup_filesystem(&fs_name).is_none() {
        return -(ENODEV as i64);
    }
    let token = MOUNT_API_TOKEN.fetch_add(1, Ordering::AcqRel) as usize;
    MOUNT_API_OBJECTS.lock().insert(
        token,
        MountApiObject::FsContext(MountFsContext {
            source: fs_name.clone(),
            fs_name,
            params: Vec::new(),
            created: false,
        }),
    );
    let file = crate::fs::anon_inode::alloc_anon_file("fs_context", &FS_CONTEXT_FILE_OPS, token);
    match current_files().and_then(|ft| ft.install(file, flags & FSOPEN_CLOEXEC != 0)) {
        Ok(fd) => fd as i64,
        Err(errno) => {
            MOUNT_API_OBJECTS.lock().remove(&token);
            -(errno as i64)
        }
    }
}

pub fn sys_fsconfig(fd: i32, cmd: u32, key: *const u8, value: *const u8, _aux: i32) -> i64 {
    if cmd > FSCONFIG_CMD_CREATE_EXCL {
        return -(EINVAL as i64);
    }
    if let Err(errno) = require_sys_admin() {
        return -(errno as i64);
    }
    let file = match current_files().and_then(|ft| ft.get(fd)) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let token = match mount_api_token(&file, FS_CONTEXT_FILE_OPS.name) {
        Ok(token) => token,
        Err(errno) => return -(errno as i64),
    };
    let mut objects = MOUNT_API_OBJECTS.lock();
    let Some(MountApiObject::FsContext(ctx)) = objects.get_mut(&token) else {
        return -(EBADF as i64);
    };
    match cmd {
        FSCONFIG_SET_FLAG => {
            let key = match unsafe { user_path(key) } {
                Ok(key) if !key.is_empty() => key,
                Ok(_) => return -(EINVAL as i64),
                Err(errno) => return -(errno as i64),
            };
            ctx.params.push((key, None));
            0
        }
        FSCONFIG_SET_STRING => {
            let key = match unsafe { user_path(key) } {
                Ok(key) if !key.is_empty() => key,
                Ok(_) => return -(EINVAL as i64),
                Err(errno) => return -(errno as i64),
            };
            let value = match unsafe { user_path(value) } {
                Ok(value) => value,
                Err(errno) => return -(errno as i64),
            };
            if key == "source" {
                ctx.source = value.clone();
            }
            ctx.params.push((key, Some(value)));
            0
        }
        FSCONFIG_CMD_CREATE | FSCONFIG_CMD_CREATE_EXCL | FSCONFIG_CMD_RECONFIGURE => {
            ctx.created = true;
            0
        }
        _ => -(EINVAL as i64),
    }
}

pub fn sys_fsmount(_fd: i32, flags: u32, attr_flags: u32) -> i64 {
    if flags & !(FSMOUNT_CLOEXEC | FSMOUNT_NAMESPACE) != 0
        || attr_flags as u64 & !MOUNT_ATTR_SUPPORTED != 0
    {
        return -(EINVAL as i64);
    }
    if let Err(errno) = require_sys_admin() {
        return -(errno as i64);
    }
    let (_token, ctx) = match mount_api_context_from_fd(_fd) {
        Ok(ctx) => ctx,
        Err(errno) => return -(errno as i64),
    };
    if !ctx.created {
        return -(EINVAL as i64);
    }
    let data = context_data_string(&ctx);
    let mount_flags = mount_flags_from_attr_bits(attr_flags as u64);
    let sb = match super::super_block::mount_fs(&ctx.fs_name, &ctx.source, mount_flags, &data) {
        Ok(sb) => sb,
        Err(errno) => return -(errno as i64),
    };
    let root = match sb.root() {
        Some(root) => root,
        None => return -(EINVAL as i64),
    };
    let mount = mount::Mount::alloc(sb, root.clone(), mount_flags as u32);
    let token = MOUNT_API_TOKEN.fetch_add(1, Ordering::AcqRel) as usize;
    MOUNT_API_OBJECTS
        .lock()
        .insert(token, MountApiObject::DetachedMount(mount));
    let file = super::file::alloc_file(root, O_PATH, 0, &DETACHED_MOUNT_FILE_OPS);
    *file.private.lock() = token;
    match current_files().and_then(|ft| ft.install(file, flags & FSMOUNT_CLOEXEC != 0)) {
        Ok(fd) => fd as i64,
        Err(errno) => {
            MOUNT_API_OBJECTS.lock().remove(&token);
            -(errno as i64)
        }
    }
}

pub unsafe fn sys_fspick(_dfd: i32, path: *const u8, flags: u32) -> i64 {
    if flags & !0x7 != 0 {
        return -(EINVAL as i64);
    }
    match unsafe { user_path(path) } {
        Ok(_) => -(ENOSYS as i64),
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_mount_setattr(
    dfd: i32,
    path: *const u8,
    flags: u32,
    attr: *const u8,
    size: usize,
) -> i64 {
    let allowed = AT_EMPTY_PATH | AT_NO_AUTOMOUNT | AT_RECURSIVE | AT_SYMLINK_NOFOLLOW;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }
    if let Err(errno) = require_sys_admin() {
        return -(errno as i64);
    }
    let attr = match copy_mount_attr(attr, size) {
        Ok(attr) => attr,
        Err(errno) => return -(errno as i64),
    };
    let path = match unsafe { user_path(path) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    if path.is_empty() {
        if flags & AT_EMPTY_PATH == 0 {
            return -(ENOENT as i64);
        }
        if let Ok(mount) = detached_mount_from_fd(dfd) {
            let result = if flags & AT_RECURSIVE != 0 {
                apply_mount_attr_recursive(&mount, &attr)
            } else {
                apply_mount_attr(&mount, &attr)
            };
            return result.map(|_| 0).unwrap_or_else(|errno| -(errno as i64));
        }
        let target = match lookup_empty_stat_target(dfd) {
            Ok(target) => target,
            Err(errno) => return -(errno as i64),
        };
        let result = if flags & AT_RECURSIVE != 0 {
            apply_mount_attr_recursive(&target.mount, &attr)
        } else {
            apply_mount_attr(&target.mount, &attr)
        };
        return result.map(|_| 0).unwrap_or_else(|errno| -(errno as i64));
    }
    let follow_final = flags & AT_SYMLINK_NOFOLLOW == 0;
    let target = match lookup_path_str_with_follow(dfd, &path, follow_final) {
        Ok(target) => target,
        Err(errno) => return -(errno as i64),
    };
    let result = if flags & AT_RECURSIVE != 0 {
        apply_mount_attr_recursive(&target.mount, &attr)
    } else {
        apply_mount_attr(&target.mount, &attr)
    };
    result.map(|_| 0).unwrap_or_else(|errno| -(errno as i64))
}

pub fn sys_quotactl_fd(_fd: i32, _cmd: u32, _id: i32, _addr: *mut u8) -> i64 {
    -(EBADF as i64)
}

pub fn sys_statmount(
    _req: *const u8,
    req_size: usize,
    _buf: *mut u8,
    bufsize: usize,
    flags: u32,
) -> i64 {
    if flags != 0 || req_size == 0 || bufsize == 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_listmount(
    _req: *const u8,
    req_size: usize,
    _buf: *mut u64,
    bufsize: usize,
    flags: u32,
) -> i64 {
    if flags != 0 || req_size == 0 || bufsize == 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_setxattrat(
    dfd: i32,
    pathname: *const u8,
    at_flags: u32,
    name: *const u8,
    value: *const u8,
    size: usize,
    flags: i32,
) -> i64 {
    let (follow_final, allow_empty) = match xattr_at_flags(at_flags) {
        Ok(flags) => flags,
        Err(errno) => return -(errno as i64),
    };
    match unsafe {
        do_setxattr_path(
            dfd,
            pathname,
            follow_final,
            allow_empty,
            name,
            value,
            size,
            flags,
        )
    } {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_getxattrat(
    dfd: i32,
    pathname: *const u8,
    at_flags: u32,
    name: *const u8,
    value: *mut u8,
    size: usize,
) -> i64 {
    let (follow_final, allow_empty) = match xattr_at_flags(at_flags) {
        Ok(flags) => flags,
        Err(errno) => return -(errno as i64),
    };
    match unsafe { do_getxattr_path(dfd, pathname, follow_final, allow_empty, name, value, size) } {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_listxattrat(
    dfd: i32,
    pathname: *const u8,
    at_flags: u32,
    list: *mut u8,
    size: usize,
) -> i64 {
    let (follow_final, allow_empty) = match xattr_at_flags(at_flags) {
        Ok(flags) => flags,
        Err(errno) => return -(errno as i64),
    };
    match unsafe { do_listxattr_path(dfd, pathname, follow_final, allow_empty, list, size) } {
        Ok(ret) => ret,
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_removexattrat(dfd: i32, pathname: *const u8, at_flags: u32, name: *const u8) -> i64 {
    let path = unsafe { user_path(pathname).unwrap_or_else(|_| String::new()) };
    let (follow_final, allow_empty) = match xattr_at_flags(at_flags) {
        Ok(flags) => flags,
        Err(errno) => {
            let ret = -(errno as i64);
            trace_xattr_path("removexattrat", dfd, &path, ret);
            return ret;
        }
    };
    let ret = match unsafe { do_removexattr_path(dfd, pathname, follow_final, allow_empty, name) } {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    };
    trace_xattr_path("removexattrat", dfd, &path, ret);
    ret
}

pub fn sys_open_tree_attr(
    dfd: i32,
    filename: *const u8,
    flags: u32,
    attr: *const u8,
    size: usize,
) -> i64 {
    let fd = match sys_open_tree(dfd, filename, flags) {
        fd if fd < 0 => return fd,
        fd => fd as i32,
    };

    if attr.is_null() {
        if size != 0 {
            let _ = close_fd_safely(fd);
            return -(EINVAL as i64);
        }
        return fd as i64;
    }
    if let Err(errno) = require_sys_admin() {
        close_fd_safely(fd);
        return -(errno as i64);
    }

    let attr = match copy_mount_attr(attr, size) {
        Ok(attr) => attr,
        Err(errno) => {
            close_fd_safely(fd);
            return -(errno as i64);
        }
    };

    let mount = match detached_mount_from_fd(fd) {
        Ok(mount) => mount,
        Err(_) => {
            let file = match current_files().and_then(|ft| ft.get(fd)) {
                Ok(file) => file,
                Err(errno) => {
                    close_fd_safely(fd);
                    return -(errno as i64);
                }
            };
            match stat_target_from_dentry(file.dentry.clone()) {
                Ok(target) => target.mount,
                Err(errno) => {
                    close_fd_safely(fd);
                    return -(errno as i64);
                }
            }
        }
    };

    if let Err(errno) = apply_mount_attr(&mount, &attr) {
        close_fd_safely(fd);
        return -(errno as i64);
    }
    fd as i64
}

pub fn sys_file_getattr(fd: i32, request_mask: u32, flags: u32, statxbuf: *mut LinuxStatx) -> i64 {
    if statxbuf.is_null() {
        return -(EFAULT as i64);
    }
    let target = match lookup_empty_stat_target(fd) {
        Ok(target) => target,
        Err(errno) => return -(errno as i64),
    };
    let Some(inode) = target.dentry.inode() else {
        return -(EBADF as i64);
    };
    let st = stat_from_inode(&inode);
    let result_mask = statx_result_mask(request_mask);
    let is_mount_root = alloc::sync::Arc::ptr_eq(&target.dentry, &target.mount.root);
    let _ = flags;
    let statx = LinuxStatx {
        stx_mask: result_mask,
        stx_blksize: st.st_blksize as u32,
        stx_attributes: if is_mount_root {
            STATX_ATTR_MOUNT_ROOT
        } else {
            0
        },
        stx_nlink: st.st_nlink as u32,
        stx_uid: st.st_uid,
        stx_gid: st.st_gid,
        stx_mode: st.st_mode as u16,
        stx_ino: st.st_ino,
        stx_size: st.st_size as u64,
        stx_blocks: st.st_blocks as u64,
        stx_attributes_mask: STATX_ATTR_MOUNT_ROOT,
        stx_atime: LinuxStatxTimestamp {
            tv_sec: st.st_atime,
            tv_nsec: st.st_atime_nsec as u32,
            __reserved: 0,
        },
        stx_ctime: LinuxStatxTimestamp {
            tv_sec: st.st_ctime,
            tv_nsec: st.st_ctime_nsec as u32,
            __reserved: 0,
        },
        stx_mtime: LinuxStatxTimestamp {
            tv_sec: st.st_mtime,
            tv_nsec: st.st_mtime_nsec as u32,
            __reserved: 0,
        },
        stx_dev_major: ((st.st_dev >> 8) & 0xfff) as u32,
        stx_dev_minor: (st.st_dev & 0xff) as u32,
        stx_mnt_id: statx_mount_id(target.mount.id, request_mask),
        ..LinuxStatx::default()
    };
    copy_ptr_to_user(statxbuf, &statx)
        .map(|()| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub fn sys_file_setattr(_fd: i32, _flags: u32, _uattr: *const u8, usize: usize) -> i64 {
    if usize == 0 {
        return -(EINVAL as i64);
    }
    -(EBADF as i64)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::{boxed::Box, string::String, vec::Vec};
    use core::sync::atomic::Ordering;

    use super::*;
    use crate::fs::fdtable::FilesStruct;
    use crate::fs::mount::{self, Mount, set_rootfs};
    use crate::fs::super_block::mount_fs;
    use crate::include::uapi::fcntl::{
        F_ADD_SEALS, F_GET_SEALS, F_SEAL_EXEC, F_SEAL_WRITE, FD_CLOEXEC, O_CLOEXEC, O_CREAT,
        O_DIRECTORY, O_NOATIME, O_NOFOLLOW, O_NONBLOCK, O_PATH, O_RDONLY, O_RDWR,
    };
    use crate::kernel::{
        cred::{INIT_CRED, commit_creds, prepare_creds},
        files, sched,
        task::TaskStruct,
    };

    fn drop_current_cap_sys_admin() {
        let new_cred = prepare_creds().expect("current task has credentials");
        unsafe {
            (*new_cred).cap_effective.lower(CAP_SYS_ADMIN);
            (*new_cred).cap_permitted.lower(CAP_SYS_ADMIN);
        }
        commit_creds(new_cred);
        assert!(!capable(CAP_SYS_ADMIN));
    }

    fn setup_current_with_rootfs(pid: i32) -> (Box<TaskStruct>, *mut TaskStruct) {
        crate::fs::init();
        mount::MOUNTS.root.lock().take();
        mount::MOUNTS.by_path.lock().clear();
        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        set_rootfs(Mount::alloc(sb, root, 0));

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = pid;
        current.tgid = pid;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);
        }
        (current, previous)
    }

    static INTERRUPTED_POLL_QUEUE: crate::kernel::sched::wait::WaitQueueHead =
        crate::kernel::sched::wait::WaitQueueHead::new();

    fn interrupted_poll_mask(
        file: &FileRef,
        table: Option<&mut crate::fs::select::PollTable>,
    ) -> u32 {
        crate::fs::select::poll_wait(file, &INTERRUPTED_POLL_QUEUE, table);
        0
    }

    static INTERRUPTED_POLL_OPS: crate::fs::ops::FileOps = crate::fs::ops::FileOps {
        name: "interrupted-poll",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: Some(interrupted_poll_mask),
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    fn dirents_contain(dirents: &[u8], len: usize, name: &[u8]) -> bool {
        dirent64_names(dirents, len)
            .iter()
            .any(|entry| entry.as_bytes() == name)
    }

    fn dirent64_names(dirents: &[u8], len: usize) -> Vec<String> {
        let mut names = Vec::new();
        let mut off = 0usize;
        while off < len {
            assert!(off + 19 <= len);
            let reclen = u16::from_ne_bytes([dirents[off + 16], dirents[off + 17]]) as usize;
            assert!(reclen >= 20);
            assert!(off + reclen <= len);
            let name_start = off + 19;
            let name_end = dirents[name_start..off + reclen]
                .iter()
                .position(|byte| *byte == 0)
                .map(|pos| name_start + pos)
                .expect("dirent nul");
            names.push(String::from(
                core::str::from_utf8(&dirents[name_start..name_end]).expect("utf8 dirent"),
            ));
            off += reclen;
        }
        assert_eq!(off, len);
        names
    }

    fn legacy_dirent_names(dirents: &[u8], len: usize) -> Vec<(String, u8, usize)> {
        let mut names = Vec::new();
        let mut off = 0usize;
        while off < len {
            assert!(off + 18 <= len);
            let reclen = u16::from_ne_bytes([dirents[off + 16], dirents[off + 17]]) as usize;
            assert!(reclen >= 21);
            assert!(off + reclen <= len);
            let name_start = off + 18;
            let name_end = dirents[name_start..off + reclen - 1]
                .iter()
                .position(|byte| *byte == 0)
                .map(|pos| name_start + pos)
                .expect("dirent nul");
            names.push((
                String::from(
                    core::str::from_utf8(&dirents[name_start..name_end]).expect("utf8 dirent"),
                ),
                dirents[off + reclen - 1],
                reclen,
            ));
            off += reclen;
        }
        assert_eq!(off, len);
        names
    }

    unsafe fn open_dir(path: &[u8]) -> i64 {
        crate::fs::openat::sys_openat(AT_FDCWD, path.as_ptr(), (O_RDONLY | O_DIRECTORY) as i32, 0)
    }

    unsafe fn assert_empty_dir_dot_entries(path: &[u8]) {
        let fd = unsafe { open_dir(path) };
        assert!(fd >= 0);
        let mut dirents = [0u8; 128];
        let len = unsafe { sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len()) };
        assert!(len > 0);
        assert_eq!(
            dirent64_names(&dirents, len as usize),
            [String::from("."), String::from("..")]
        );
        assert_eq!(
            unsafe { sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len()) },
            0
        );
        assert_eq!(crate::fs::fdtable::sys_close(fd as i32), 0);
    }

    #[test]
    fn getdents64_empty_ramfs_and_tmpfs_dirs_emit_dots_before_eof() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(189);
        unsafe {
            assert_eq!(sys_mkdir(b"/empty\0".as_ptr(), 0o755), 0);
            assert_empty_dir_dot_entries(b"/empty\0");

            assert_eq!(sys_mkdir(b"/tmp\0".as_ptr(), 0o755), 0);
            mount::do_mount("tmpfs", "tmpfs", "/tmp", 0, "").expect("tmpfs /tmp");
            assert_empty_dir_dot_entries(b"/tmp\0");

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn getdents64_directory_cursor_is_shared_with_lseek() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(194);
        unsafe {
            assert_eq!(sys_mkdir(b"/rewind\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/rewind/child\0".as_ptr(), 0o755), 0);
            let fd = open_dir(b"/rewind\0");
            assert!(fd >= 0);

            let mut dirents = [0u8; 256];
            let len = sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(len > 0);
            assert_eq!(
                dirent64_names(&dirents, len as usize),
                [String::from("."), String::from(".."), String::from("child")]
            );
            assert_eq!(
                sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len()),
                0
            );

            assert_eq!(sys_lseek(fd as i32, 0, SEEK_SET), 0);
            let len = sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(len > 0);
            assert_eq!(
                dirent64_names(&dirents, len as usize),
                [String::from("."), String::from(".."), String::from("child")]
            );

            assert_eq!(crate::fs::fdtable::sys_close(fd as i32), 0);
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn getdents64_too_small_buffer_returns_einval_without_skipping_entry() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(190);
        unsafe {
            assert_eq!(sys_mkdir(b"/small\0".as_ptr(), 0o755), 0);
            let fd = open_dir(b"/small\0");
            assert!(fd >= 0);

            assert_eq!(
                sys_getdents64(fd as i32, usize::MAX as *mut u8, 24),
                -(EFAULT as i64)
            );

            let mut first = [0u8; 24];
            let len = sys_getdents64(fd as i32, first.as_mut_ptr(), first.len());
            assert_eq!(len, 24);
            assert_eq!(dirent64_names(&first, len as usize), [String::from(".")]);

            let mut tiny = [0u8; 8];
            assert_eq!(
                sys_getdents64(fd as i32, tiny.as_mut_ptr(), tiny.len()),
                -(EINVAL as i64)
            );

            let mut rest = [0u8; 128];
            let len = sys_getdents64(fd as i32, rest.as_mut_ptr(), rest.len());
            assert!(len > 0);
            assert_eq!(dirent64_names(&rest, len as usize)[0], "..");

            assert_eq!(crate::fs::fdtable::sys_close(fd as i32), 0);
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn getdents64_null_pointer_faults_only_when_copying_entry() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(193);
        unsafe {
            assert_eq!(sys_mkdir(b"/nullbuf\0".as_ptr(), 0o755), 0);
            let fd = open_dir(b"/nullbuf\0");
            assert!(fd >= 0);

            assert_eq!(
                sys_getdents64(fd as i32, core::ptr::null_mut(), 0),
                -(EINVAL as i64)
            );

            let mut dirents = [0u8; 128];
            let len = sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(len > 0);
            assert_eq!(dirent64_names(&dirents, len as usize)[0], ".");

            assert_eq!(sys_getdents64(fd as i32, core::ptr::null_mut(), 128), 0);

            assert_eq!(crate::fs::fdtable::sys_close(fd as i32), 0);
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn getdents_uses_legacy_linux_dirent_layout() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(191);
        unsafe {
            assert_eq!(sys_mkdir(b"/legacy\0".as_ptr(), 0o755), 0);
            let fd = open_dir(b"/legacy\0");
            assert!(fd >= 0);

            let mut dirents = [0u8; 128];
            let len = sys_getdents(fd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(len > 0);
            let entries = legacy_dirent_names(&dirents, len as usize);
            assert_eq!(entries[0].0, ".");
            assert_eq!(entries[0].1, dirent_type(InodeKind::Directory));
            assert_eq!(entries[0].2, 24);
            assert_eq!(dirents[18], b'.');
            assert_eq!(dirents[19], 0);
            assert_eq!(dirents[23], dirent_type(InodeKind::Directory));

            assert_eq!(crate::fs::fdtable::sys_close(fd as i32), 0);
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn getcwd_too_small_buffer_returns_erange() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(192);
        unsafe {
            assert_eq!(sys_mkdir(b"/usr\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_chdir(b"/usr\0".as_ptr()), 0);
            let mut cwd = [0u8; 4];
            assert_eq!(sys_getcwd(cwd.as_mut_ptr(), cwd.len()), -(ERANGE as i64));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn at_fdcwd_dot_and_relative_chdir_stay_under_pwd() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(188);
        unsafe {
            for dir in [
                b"/usr\0".as_slice(),
                b"/usr/bin\0".as_slice(),
                b"/home\0".as_slice(),
                b"/home/lupos\0".as_slice(),
                b"/lost+found\0".as_slice(),
            ] {
                assert_eq!(sys_mkdir(dir.as_ptr(), 0o755), 0);
            }

            assert_eq!(sys_chdir(b"/usr\0".as_ptr()), 0);
            let mut cwd = [0u8; 16];
            assert_eq!(sys_getcwd(cwd.as_mut_ptr(), cwd.len()), 5);
            assert_eq!(&cwd[..5], b"/usr\0");

            let dot_fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b".\0".as_ptr(),
                (O_RDONLY | O_DIRECTORY) as i32,
                0,
            );
            assert!(dot_fd >= 0);
            let mut dirents = [0u8; 512];
            let len = sys_getdents64(dot_fd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(len > 0);
            let len = len as usize;
            assert!(
                dirents_contain(&dirents, len, b"."),
                "open('.') after chdir('/usr') must enumerate dot"
            );
            assert!(
                dirents_contain(&dirents, len, b".."),
                "open('.') after chdir('/usr') must enumerate dotdot"
            );
            assert!(
                dirents_contain(&dirents, len, b"bin"),
                "open('.') after chdir('/usr') must enumerate /usr"
            );
            assert!(
                !dirents_contain(&dirents, len, b"home"),
                "open('.') after chdir('/usr') must not enumerate /"
            );
            assert!(
                !dirents_contain(&dirents, len, b"lost+found"),
                "open('.') after chdir('/usr') must not enumerate /"
            );

            assert_eq!(sys_chdir(b"home\0".as_ptr()), -(ENOENT as i64));
            assert_eq!(sys_chdir(b"bin\0".as_ptr()), 0);
            let mut nested_cwd = [0u8; 24];
            assert_eq!(sys_getcwd(nested_cwd.as_mut_ptr(), nested_cwd.len()), 9);
            assert_eq!(&nested_cwd[..9], b"/usr/bin\0");

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn mknodat_creates_systemd_inaccessible_node_shapes() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(83);
        unsafe {
            for dir in [
                b"/run\0".as_slice(),
                b"/run/systemd\0".as_slice(),
                b"/run/systemd/inaccessible\0".as_slice(),
            ] {
                assert_eq!(sys_mkdirat(AT_FDCWD, dir.as_ptr(), 0o755), 0);
            }
            for (name, mode, expected) in [
                (
                    "reg",
                    crate::include::uapi::stat::S_IFREG,
                    crate::include::uapi::stat::S_IFREG,
                ),
                (
                    "fifo",
                    crate::include::uapi::stat::S_IFIFO,
                    crate::include::uapi::stat::S_IFIFO,
                ),
                (
                    "sock",
                    crate::include::uapi::stat::S_IFSOCK,
                    crate::include::uapi::stat::S_IFSOCK,
                ),
                (
                    "chr",
                    crate::include::uapi::stat::S_IFCHR,
                    crate::include::uapi::stat::S_IFCHR,
                ),
                (
                    "blk",
                    crate::include::uapi::stat::S_IFBLK,
                    crate::include::uapi::stat::S_IFBLK,
                ),
            ] {
                let path = std::format!("/run/systemd/inaccessible/{name}\0");
                assert_eq!(sys_mknodat(AT_FDCWD, path.as_ptr(), mode | 0o000, 0), 0);
                let mut st = LinuxStat::default();
                assert_eq!(sys_lstat(path.as_ptr(), &mut st), 0);
                assert_eq!(st.st_mode & crate::include::uapi::stat::S_IFMT, expected);
                if name == "blk" {
                    let path_without_nul =
                        core::str::from_utf8(&path.as_bytes()[..path.len() - 1]).unwrap();
                    let (_, dentry) =
                        mount::resolve_path_follow(path_without_nul).expect("block node dentry");
                    let inode = dentry.inode().expect("block node inode");
                    assert_eq!(inode.fops.name, "block_device");
                }
            }

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn per_task_umask_filters_open_mkdir_and_mknod_creation_modes() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(84);
        unsafe {
            assert_eq!(crate::kernel::syscalls::sys_umask(0o077), 0o022);

            let fd = sys_openat(
                AT_FDCWD,
                b"/private-file\0".as_ptr(),
                (O_CREAT | O_WRONLY) as i32,
                0o666,
            );
            assert!(fd >= 0);
            assert_eq!(crate::fs::fdtable::sys_close(fd as i32), 0);
            let how = OpenHow {
                flags: (O_CREAT | O_WRONLY) as u64,
                mode: 0o666,
                resolve: 0,
            };
            let openat2_fd = sys_openat2(
                AT_FDCWD,
                b"/private-openat2\0".as_ptr(),
                &how,
                core::mem::size_of::<OpenHow>(),
            );
            assert!(openat2_fd >= 0);
            assert_eq!(crate::fs::fdtable::sys_close(openat2_fd as i32), 0);
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/private-dir\0".as_ptr(), 0o777), 0);
            assert_eq!(
                sys_mknodat(AT_FDCWD, b"/private-fifo\0".as_ptr(), S_IFIFO | 0o666, 0,),
                0
            );

            for (path, expected) in [
                (b"/private-file\0".as_slice(), S_IFREG | 0o600),
                (b"/private-openat2\0".as_slice(), S_IFREG | 0o600),
                (b"/private-dir\0".as_slice(), S_IFDIR | 0o700),
                (b"/private-fifo\0".as_slice(), S_IFIFO | 0o600),
            ] {
                let mut st = LinuxStat::default();
                assert_eq!(sys_lstat(path.as_ptr(), &mut st), 0);
                assert_eq!(st.st_mode & (S_IFMT | 0o7777), expected);
            }

            files::drop_task_files(&mut *current as *mut TaskStruct);
            crate::fs::fs_struct::exit_fs(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn mkdir_inside_tmpfs_mount_is_visible_to_open() {
        // Regression shape of the runtime-stress gate on the disk-root boot:
        // /tmp is a mounted tmpfs; `mkdir -p /tmp/lupos-stress` succeeded but
        // the very next `echo > /tmp/lupos-stress/file` failed ENOENT
        // ("Directory nonexistent") — mkdir and open must resolve the path
        // through the SAME (mounted) filesystem, like Linux's mountpoint
        // crossing in link_path_walk (vendor/linux/fs/namei.c).
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(859);
        unsafe {
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/tmp\0".as_ptr(), 0o777), 0);
            mount::do_mount("tmpfs", "tmpfs", "/tmp", 0, "").expect("tmp tmpfs");
            assert!(mount::lookup_mount("/tmp").is_some());

            // The guest gate creates and unlinks a scratch file first
            // (sort.out loop) ...
            let scratch = sys_openat(
                AT_FDCWD,
                b"/tmp/sort.out\0".as_ptr(),
                (crate::include::uapi::fcntl::O_WRONLY | crate::include::uapi::fcntl::O_CREAT)
                    as i32,
                0o644,
            );
            assert!(scratch >= 0);
            assert_eq!(crate::fs::fdtable::sys_close(scratch as i32), 0);
            assert_eq!(sys_unlinkat(AT_FDCWD, b"/tmp/sort.out\0".as_ptr(), 0), 0);

            // ... and `mkdir -p` stats the target before creating it, which
            // caches a negative dentry that the create path must heal.
            let mut st = core::mem::MaybeUninit::<LinuxStat>::uninit();
            assert_eq!(
                sys_newfstatat(
                    AT_FDCWD,
                    b"/tmp/lupos-stress\0".as_ptr(),
                    st.as_mut_ptr(),
                    0
                ),
                -(ENOENT as i64)
            );

            assert_eq!(
                sys_mkdirat(AT_FDCWD, b"/tmp/lupos-stress\0".as_ptr(), 0o755),
                0
            );
            let fd = sys_openat(
                AT_FDCWD,
                b"/tmp/lupos-stress/file\0".as_ptr(),
                (crate::include::uapi::fcntl::O_WRONLY | crate::include::uapi::fcntl::O_CREAT)
                    as i32,
                0o644,
            );
            assert!(
                fd >= 0,
                "open after mkdir inside the tmpfs mount must succeed, got {fd}"
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn linkat_ignores_stale_positive_dentry_missing_from_tmpfs_map() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(860);
        unsafe {
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/tmp\0".as_ptr(), 0o777), 0);
            mount::do_mount("tmpfs", "tmpfs", "/tmp", 0, "").expect("tmp tmpfs");

            let src = sys_openat(
                AT_FDCWD,
                b"/tmp/src\0".as_ptr(),
                (O_RDWR | O_CREAT) as i32,
                0o644,
            );
            assert!(src >= 0);
            assert_eq!(crate::fs::fdtable::sys_close(src as i32), 0);

            let tmp = lookup_path_str(AT_FDCWD, "/tmp").expect("/tmp").dentry;
            let src_dentry = lookup_path_str(AT_FDCWD, "/tmp/src")
                .expect("/tmp/src")
                .dentry;
            let stale = crate::fs::dcache::d_alloc_child(&tmp, "ghost");
            stale.instantiate(src_dentry.inode().expect("src inode"));

            assert_eq!(
                sys_link(b"/tmp/src\0".as_ptr(), b"/tmp/ghost\0".as_ptr()),
                0
            );

            let fd = open_dir(b"/tmp\0");
            assert!(fd >= 0);
            let mut dirents = [0u8; 256];
            let len = sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(len > 0);
            assert!(dirents_contain(&dirents, len as usize, b"ghost"));
            assert_eq!(crate::fs::fdtable::sys_close(fd as i32), 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn linkat_requires_destination_directory_write_permission() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(861);
        unsafe {
            assert!(sys_creat(b"/src\0".as_ptr(), 0o644) >= 0);
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/protected\0".as_ptr(), 0o755), 0);

            drop_current_to_unprivileged(1000);

            assert_eq!(
                sys_link(b"/src\0".as_ptr(), b"/protected/hard\0".as_ptr()),
                -(EACCES as i64)
            );
            let mut stat_buf = LinuxStat::default();
            assert_eq!(
                sys_lstat(b"/protected/hard\0".as_ptr(), &mut stat_buf),
                -(ENOENT as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn linkat_rejects_readonly_destination_mount() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(862);
        unsafe {
            assert!(sys_creat(b"/src\0".as_ptr(), 0o644) >= 0);
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/ro\0".as_ptr(), 0o755), 0);
            mount::do_mount("tmpfs", "tmpfs", "/ro", MS_RDONLY, "").expect("readonly tmpfs");

            assert_eq!(
                sys_link(b"/src\0".as_ptr(), b"/ro/hard\0".as_ptr()),
                -(crate::include::uapi::errno::EROFS as i64)
            );
            let mut stat_buf = LinuxStat::default();
            assert_eq!(
                sys_lstat(b"/ro/hard\0".as_ptr(), &mut stat_buf),
                -(ENOENT as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn linkat_rejects_distinct_bind_mount_of_same_filesystem() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(864);
        unsafe {
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/source\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/alias\0".as_ptr(), 0o755), 0);
            assert!(sys_creat(b"/source/file\0".as_ptr(), 0o644) >= 0);
            assert_eq!(
                mount::sys_mount(
                    b"/source\0".as_ptr(),
                    b"/alias\0".as_ptr(),
                    core::ptr::null(),
                    crate::include::uapi::mount::MS_BIND as u64,
                    core::ptr::null(),
                ),
                0
            );

            assert_eq!(
                sys_link(b"/source/file\0".as_ptr(), b"/alias/hard\0".as_ptr()),
                -(EXDEV as i64)
            );
            let mut stat_buf = LinuxStat::default();
            assert_eq!(
                sys_lstat(b"/alias/hard\0".as_ptr(), &mut stat_buf),
                -(ENOENT as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn linkat_empty_path_requires_cap_dac_read_search() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(863);
        unsafe {
            let fd = sys_openat(
                AT_FDCWD,
                b"/src\0".as_ptr(),
                (O_RDWR | O_CREAT) as i32,
                0o644,
            );
            assert!(fd >= 0);

            drop_current_to_unprivileged(1000);

            assert_eq!(
                sys_linkat(
                    fd as i32,
                    b"\0".as_ptr(),
                    AT_FDCWD,
                    b"/hard\0".as_ptr(),
                    AT_EMPTY_PATH as i32
                ),
                -(ENOENT as i64)
            );
            let mut stat_buf = LinuxStat::default();
            assert_eq!(
                sys_lstat(b"/hard\0".as_ptr(), &mut stat_buf),
                -(ENOENT as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn umount2_requires_cap_sys_admin() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(858);
        unsafe {
            for dir in [
                b"/run\0".as_slice(),
                b"/run/credentials\0".as_slice(),
                b"/run/credentials/poc.service\0".as_slice(),
            ] {
                assert_eq!(sys_mkdirat(AT_FDCWD, dir.as_ptr(), 0o755), 0);
            }

            let target = "/run/credentials/poc.service";
            mount::do_mount("tmpfs", "tmpfs", target, 0, "").expect("credential tmpfs");
            assert!(mount::lookup_mount(target).is_some());

            let unpriv = crate::kernel::cred::prepare_creds().expect("unprivileged cred");
            (*unpriv).uid = crate::kernel::cred::KUid(1000);
            (*unpriv).gid = crate::kernel::cred::KGid(1000);
            (*unpriv).euid = crate::kernel::cred::KUid(1000);
            (*unpriv).egid = crate::kernel::cred::KGid(1000);
            (*unpriv).fsuid = crate::kernel::cred::KUid(1000);
            (*unpriv).fsgid = crate::kernel::cred::KGid(1000);
            (*unpriv).cap_effective = crate::kernel::capability::KernelCapT::empty();
            (*unpriv).cap_permitted = crate::kernel::capability::KernelCapT::empty();
            crate::kernel::cred::commit_creds(unpriv);
            assert!(!capable(CAP_SYS_ADMIN));

            let target_c = b"/run/credentials/poc.service\0";
            assert_eq!(sys_umount2(target_c.as_ptr(), 0), -(EPERM as i64));
            assert!(mount::lookup_mount(target).is_some());

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn umount2_detaches_credentials_mount_and_preserves_mountpoint_dir() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(85);
        unsafe {
            for dir in [
                b"/run\0".as_slice(),
                b"/run/credentials\0".as_slice(),
                b"/run/credentials/systemd-journald.service\0".as_slice(),
            ] {
                assert_eq!(sys_mkdirat(AT_FDCWD, dir.as_ptr(), 0o755), 0);
            }

            let target = "/run/credentials/systemd-journald.service";
            mount::do_mount("tmpfs", "tmpfs", target, 0, "").expect("credential tmpfs");
            assert!(mount::lookup_mount(target).is_some());

            let target_c = b"/run/credentials/systemd-journald.service\0";
            assert_eq!(
                sys_umount2(
                    target_c.as_ptr(),
                    (crate::include::uapi::mount::MNT_DETACH
                        | crate::include::uapi::mount::UMOUNT_NOFOLLOW) as i32,
                ),
                0
            );
            assert!(mount::lookup_mount(target).is_none());
            assert!(mount::resolve_path_follow(target).is_ok());

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn rmdir_refuses_mounted_credentials_directory() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(86);
        unsafe {
            for dir in [
                b"/run\0".as_slice(),
                b"/run/credentials\0".as_slice(),
                b"/run/credentials/systemd-networkd.service\0".as_slice(),
            ] {
                assert_eq!(sys_mkdirat(AT_FDCWD, dir.as_ptr(), 0o755), 0);
            }

            let target = "/run/credentials/systemd-networkd.service";
            mount::do_mount("tmpfs", "tmpfs", target, 0, "").expect("credential tmpfs");

            let target_c = b"/run/credentials/systemd-networkd.service\0";
            assert_eq!(sys_rmdir(target_c.as_ptr()), -(EBUSY as i64));
            assert!(mount::resolve_path_follow(target).is_ok());

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m76_vfs_metadata_parity() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(84);
        unsafe {
            let path = b"/m76-file\0";
            let fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                path.as_ptr(),
                (O_CREAT | O_RDWR) as i32,
                0o644,
            );
            assert!(fd >= 0);

            assert_eq!(sys_access(path.as_ptr(), 0), 0);
            assert_eq!(sys_faccessat(AT_FDCWD, path.as_ptr(), 0), 0);
            assert_eq!(
                sys_faccessat2(AT_FDCWD, path.as_ptr(), 0, AT_SYMLINK_NOFOLLOW as i32),
                0
            );
            // test-origin: linux:vendor/linux/fs/open.c (do_faccessat AT_EACCESS mask).
            // glibc's realpath() canonicalization issues a trailing faccessat2()
            // with AT_EACCESS on the original path; rejecting that flag broke
            // realpath()/pacman's alpm_initialize() for any trailing-slash path.
            assert_eq!(
                sys_faccessat2(
                    AT_FDCWD,
                    path.as_ptr(),
                    0,
                    crate::include::uapi::fcntl::AT_EACCESS as i32
                ),
                0
            );
            assert_eq!(
                sys_faccessat2(
                    AT_FDCWD,
                    path.as_ptr(),
                    0,
                    (crate::include::uapi::fcntl::AT_EACCESS | AT_SYMLINK_NOFOLLOW) as i32
                ),
                0
            );
            assert_eq!(
                sys_faccessat2(AT_FDCWD, path.as_ptr(), 0, 0x4000),
                -(EINVAL as i64)
            );
            assert_eq!(sys_access(core::ptr::null(), 0), -(EFAULT as i64));

            assert_eq!(sys_fallocate(fd as i32, 0, 0, 64), 0);
            assert_eq!(sys_fallocate(fd as i32, FALLOC_FL_KEEP_SIZE, 0, 64), 0);
            assert_eq!(
                sys_fallocate(fd as i32, FALLOC_FL_PUNCH_HOLE, 0, 64),
                -(EINVAL as i64)
            );
            assert_eq!(
                crate::fs::read_write::sys_write(fd as i32, b"abcdef".as_ptr(), 6),
                6
            );
            assert_eq!(
                sys_fallocate(fd as i32, FALLOC_FL_PUNCH_HOLE | FALLOC_FL_KEEP_SIZE, 1, 3),
                0
            );
            let mut punched = [0u8; 6];
            assert_eq!(sys_lseek(fd as i32, 0, 0), 0);
            assert_eq!(
                crate::fs::read_write::sys_read(fd as i32, punched.as_mut_ptr(), punched.len()),
                punched.len() as i64
            );
            assert_eq!(&punched, b"a\0\0\0ef");
            assert_eq!(sys_ftruncate(fd as i32, 32), 0);
            assert_eq!(sys_ftruncate(fd as i32, -1), -(EINVAL as i64));
            assert_eq!(sys_truncate(path.as_ptr(), 16), 0);
            assert_eq!(sys_truncate(path.as_ptr(), -1), -(EINVAL as i64));

            let dentry = lookup_path(AT_FDCWD, path.as_ptr()).expect("created path");
            let inode = dentry.inode().expect("inode");
            assert_eq!(inode.size.load(Ordering::Acquire), 16);

            assert_eq!(sys_fchmod(fd as i32, 0o600), 0);
            assert_eq!(inode.mode.load(Ordering::Acquire) & 0o777, 0o600);
            assert_eq!(sys_chmod(path.as_ptr(), 0o644), 0);
            assert_eq!(sys_fchmodat(AT_FDCWD, path.as_ptr(), 0o640), 0);
            assert_eq!(
                sys_fchmodat2(AT_FDCWD, path.as_ptr(), 0o600, AT_SYMLINK_NOFOLLOW as i32),
                0
            );
            assert_eq!(
                sys_fchmodat2(AT_FDCWD, path.as_ptr(), 0o600, 0x4000),
                -(EINVAL as i64)
            );
            let empty = b"\0";
            assert_eq!(
                sys_fchmodat(fd as i32, empty.as_ptr(), 0o600),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_faccessat2(fd as i32, empty.as_ptr(), 0, AT_EMPTY_PATH as i32),
                0
            );
            assert_eq!(
                sys_fchmodat2(fd as i32, empty.as_ptr(), 0o620, AT_EMPTY_PATH as i32),
                0
            );
            assert_eq!(inode.mode.load(Ordering::Acquire) & 0o777, 0o620);
            assert_eq!(
                sys_fchmodat2(fd as i32, empty.as_ptr(), 0o600, 0),
                -(ENOENT as i64)
            );

            assert_eq!(sys_fchown(fd as i32, 1000, 1001), 0);
            assert_eq!(inode.uid.load(Ordering::Acquire), 1000);
            assert_eq!(inode.gid.load(Ordering::Acquire), 1001);
            assert_eq!(sys_chown(path.as_ptr(), 2000, u32::MAX), 0);
            assert_eq!(sys_lchown(path.as_ptr(), u32::MAX, 2001), 0);
            assert_eq!(
                sys_fchownat(fd as i32, empty.as_ptr(), 3000, 3001, AT_EMPTY_PATH as i32),
                0
            );
            assert_eq!(inode.uid.load(Ordering::Acquire), 3000);
            assert_eq!(inode.gid.load(Ordering::Acquire), 3001);

            let mut tv = [crate::kernel::syscalls::TimeVal::default(); 2];
            assert_eq!(
                crate::kernel::syscalls::sys_utime(path.as_ptr(), core::ptr::null()),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_utimes(path.as_ptr(), tv.as_ptr()),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_futimesat(AT_FDCWD, path.as_ptr(), tv.as_ptr()),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_futimesat(fd as i32, core::ptr::null(), tv.as_ptr()),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_futimesat(AT_FDCWD, core::ptr::null(), tv.as_ptr()),
                -(EFAULT as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_utimensat(
                    AT_FDCWD,
                    path.as_ptr(),
                    core::ptr::null(),
                    0
                ),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_utimensat(
                    fd as i32,
                    core::ptr::null(),
                    core::ptr::null(),
                    0
                ),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_utimensat(
                    fd as i32,
                    core::ptr::null(),
                    core::ptr::null(),
                    AT_EMPTY_PATH as i32
                ),
                -(EINVAL as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_utimensat(
                    fd as i32,
                    empty.as_ptr(),
                    core::ptr::null(),
                    AT_EMPTY_PATH as i32
                ),
                0
            );
            assert_eq!(
                crate::kernel::syscalls::sys_utimensat(
                    AT_FDCWD,
                    path.as_ptr(),
                    core::ptr::null(),
                    0x4000
                ),
                -(EINVAL as i64)
            );

            let mut handle = crate::kernel::syscalls::FileHandle::default();
            let mut mount_id = 0;
            assert_eq!(
                crate::kernel::syscalls::sys_name_to_handle_at(
                    AT_FDCWD,
                    path.as_ptr(),
                    &mut handle,
                    &mut mount_id,
                    0
                ),
                -(crate::include::uapi::errno::ENOTSUP as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_name_to_handle_at(
                    AT_FDCWD,
                    core::ptr::null(),
                    &mut handle,
                    &mut mount_id,
                    0
                ),
                -(EFAULT as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_open_by_handle_at(-1, &mut handle, 0),
                -(EPERM as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_open_by_handle_at(-1, core::ptr::null_mut(), 0),
                -(EFAULT as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn fallocate_large_ram_file_keeps_holes_sparse() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(184);
        unsafe {
            let path = b"/journal-file\0";
            let fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                path.as_ptr(),
                (O_CREAT | O_RDWR) as i32,
                0o644,
            );
            assert!(fd >= 0);

            let journal_len = 8 * 1024 * 1024;
            assert_eq!(sys_fallocate(fd as i32, 0, 0, journal_len), 0);

            let dentry = lookup_path(AT_FDCWD, path.as_ptr()).expect("created path");
            let inode = dentry.inode().expect("inode");
            assert_eq!(inode.size.load(Ordering::Acquire), journal_len as u64);
            match &inode.private {
                InodePrivate::RamBytes(bytes) => assert_eq!(bytes.lock().len(), 0),
                _ => panic!("expected ram bytes"),
            }

            let mut hole = [0xffu8; 16];
            assert_eq!(
                sys_pread64(fd as i32, hole.as_mut_ptr(), hole.len(), 4 * 1024 * 1024),
                hole.len() as i64
            );
            assert_eq!(hole, [0u8; 16]);

            let header = [0x4au8, 0x52, 0x4e, 0x4c];
            assert_eq!(
                sys_pwrite64(fd as i32, header.as_ptr(), header.len(), 0),
                header.len() as i64
            );
            assert_eq!(inode.size.load(Ordering::Acquire), journal_len as u64);
            match &inode.private {
                InodePrivate::RamBytes(bytes) => assert_eq!(bytes.lock().len(), header.len()),
                _ => panic!("expected ram bytes"),
            }

            assert_eq!(sys_ftruncate(fd as i32, 2), 0);
            assert_eq!(inode.size.load(Ordering::Acquire), 2);
            match &inode.private {
                InodePrivate::RamBytes(bytes) => assert_eq!(bytes.lock().len(), 2),
                _ => panic!("expected ram bytes"),
            }

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn opath_handles_metadata_dirfd_and_proc_fd_reopen() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(86);
        unsafe {
            let dir = b"/opath-dir\0";
            let child = b"child\0";
            let child_abs = b"/opath-dir/child\0";
            assert_eq!(sys_mkdirat(AT_FDCWD, dir.as_ptr(), 0o755), 0);
            let created = crate::fs::openat::sys_openat(
                AT_FDCWD,
                child_abs.as_ptr(),
                (O_CREAT | O_RDWR) as i32,
                0o644,
            );
            assert!(created >= 0);

            let dirfd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                dir.as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert!(dirfd >= 0);
            let via_dirfd =
                crate::fs::openat::sys_openat(dirfd as i32, child.as_ptr(), O_RDONLY as i32, 0);
            assert!(via_dirfd >= 0);

            let opath =
                crate::fs::openat::sys_openat(AT_FDCWD, child_abs.as_ptr(), O_PATH as i32, 0);
            assert!(opath >= 0);
            let mut st = LinuxStat::default();
            assert_eq!(sys_fstat(opath as i32, &mut st), 0);
            let empty = b"\0";
            let mut stx = LinuxStatx::default();
            assert_eq!(
                sys_statx(
                    opath as i32,
                    empty.as_ptr(),
                    AT_EMPTY_PATH as i32,
                    0xffff,
                    &mut stx
                ),
                0
            );

            let mut byte = [0u8; 1];
            assert_eq!(
                crate::fs::read_write::sys_read(opath as i32, byte.as_mut_ptr(), 1),
                -(EBADF as i64)
            );
            assert_eq!(sys_lseek(opath as i32, 0, SEEK_SET), -(EBADF as i64));

            let proc_path = std::format!("/proc/self/fd/{}\0", opath);
            let reopened =
                crate::fs::openat::sys_openat(AT_FDCWD, proc_path.as_ptr(), O_RDONLY as i32, 0);
            assert!(reopened >= 0);
            assert_eq!(
                crate::fs::read_write::sys_read(reopened as i32, byte.as_mut_ptr(), 1),
                0
            );

            // vendor/linux/fs/open.c::build_open_how(): legacy open()/openat()
            // masks O_PATH down to O_PATH_FLAGS (O_DIRECTORY|O_NOFOLLOW|O_PATH|
            // O_CLOEXEC), so extra access/create bits are silently ignored rather
            // than rejected. (openat2() with an explicit open_how instead rejects
            // them with EINVAL — see openat.rs do_openat2 tests.)
            let opath_rdwr = crate::fs::openat::sys_openat(
                AT_FDCWD,
                child_abs.as_ptr(),
                (O_PATH | O_RDWR) as i32,
                0,
            );
            assert!(
                opath_rdwr >= 0,
                "O_PATH|O_RDWR must mask O_RDWR and succeed"
            );
            // Still an O_PATH fd: read(2) fails with EBADF regardless of O_RDWR.
            assert_eq!(
                crate::fs::read_write::sys_read(opath_rdwr as i32, byte.as_mut_ptr(), 1),
                -(EBADF as i64)
            );
            let opath_creat = crate::fs::openat::sys_openat(
                AT_FDCWD,
                child_abs.as_ptr(),
                (O_PATH | O_CREAT) as i32,
                0,
            );
            assert!(
                opath_creat >= 0,
                "O_PATH|O_CREAT must mask O_CREAT and succeed"
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn mounted_opath_dirfd_walks_from_mount_root_not_dentry_text() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(87);
        unsafe {
            assert_eq!(sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/systemd\0".as_ptr(), 0o755), 0);
            mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs on /run");

            let runfd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run\0".as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert!(runfd >= 0);
            assert_eq!(sys_mkdirat(runfd as i32, b"systemd\0".as_ptr(), 0o755), 0);

            let systemd_fd = crate::fs::openat::sys_openat(
                runfd as i32,
                b"systemd\0".as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert!(systemd_fd >= 0);
            assert_eq!(
                sys_mkdirat(systemd_fd as i32, b"netif\0".as_ptr(), 0o755),
                0
            );

            let nested = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run/systemd/netif\0".as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert!(nested >= 0);
            assert_eq!(
                crate::fs::proc::fd::current_fd_path(nested as i32).expect("fd path"),
                "/run/systemd/netif"
            );
            let proc_fd_child = std::format!("/proc/self/fd/{}/netif", systemd_fd);
            assert_eq!(
                crate::fs::proc::fd::current_fd_path_from_proc_path(&proc_fd_child)
                    .expect("proc fd path")
                    .expect("resolved proc fd child"),
                "/run/systemd/netif"
            );
            let proc_fd_child_cstr = std::format!("{proc_fd_child}\0");
            let mut st = LinuxStat::default();
            assert_eq!(
                sys_newfstatat(AT_FDCWD, proc_fd_child_cstr.as_ptr(), &mut st, 0,),
                0
            );
            let wrong_root = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/systemd/netif\0".as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert_eq!(wrong_root, -(ENOENT as i64));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn proc_fd_mount_target_records_canonical_mount_path() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(872);
        unsafe {
            assert_eq!(sys_mkdir(b"/mnt\0".as_ptr(), 0o755), 0);
            let mountpoint_fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/mnt\0".as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert!(mountpoint_fd >= 0);
            let target = std::format!("/proc/self/fd/{}\0", mountpoint_fd);
            assert_eq!(
                mount::sys_mount(
                    b"tmpfs\0".as_ptr(),
                    target.as_ptr(),
                    b"tmpfs\0".as_ptr(),
                    0,
                    core::ptr::null(),
                ),
                0
            );
            assert_eq!(sys_mkdir(b"/mnt/child\0".as_ptr(), 0o755), 0);
            let child_fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/mnt/child\0".as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert!(child_fd >= 0);
            assert_eq!(
                crate::fs::proc::fd::current_fd_path(child_fd as i32).expect("fd path"),
                "/mnt/child"
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn credential_dirfd_stat_uses_opened_mount_path_after_overmount() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(874);
        unsafe {
            assert_eq!(sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/dev\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/dev/shm\0".as_ptr(), 0o777), 0);
            mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs on /run");
            assert_eq!(sys_mkdir(b"/run/credentials\0".as_ptr(), 0o700), 0);
            assert_eq!(
                sys_mkdir(
                    b"/run/credentials/systemd-journald.service\0".as_ptr(),
                    0o700
                ),
                0
            );

            let credentials_fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run/credentials\0".as_ptr(),
                (O_PATH | O_DIRECTORY | O_CLOEXEC) as i32,
                0,
            );
            assert!(credentials_fd >= 0);
            let credentials_target = std::format!("/proc/self/fd/{}\0", credentials_fd);
            assert_eq!(
                mount::sys_mount(
                    b"tmpfs\0".as_ptr(),
                    credentials_target.as_ptr(),
                    b"tmpfs\0".as_ptr(),
                    0,
                    core::ptr::null(),
                ),
                0
            );
            assert_eq!(
                sys_mkdir(
                    b"/run/credentials/systemd-resolved.service\0".as_ptr(),
                    0o700
                ),
                0
            );

            let reopened = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run/credentials\0".as_ptr(),
                (O_PATH | O_DIRECTORY | O_CLOEXEC) as i32,
                0,
            );
            assert!(reopened >= 0);
            let mut stx = LinuxStatx::default();
            assert_eq!(
                sys_statx(
                    reopened as i32,
                    b"systemd-resolved.service\0".as_ptr(),
                    (AT_SYMLINK_NOFOLLOW | AT_NO_AUTOMOUNT | AT_STATX_DONT_SYNC) as i32,
                    STATX_BASIC_STATS,
                    &mut stx,
                ),
                0
            );
            assert_eq!(stx.stx_mode & S_IFMT as u16, S_IFDIR as u16);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn proc_fd_mount_target_uses_opened_mount_alias_not_host_dentry() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(873);
        unsafe {
            assert_eq!(sys_mkdir(b"/proc\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            mount::do_mount("proc", "proc", "/proc", 0, "").expect("proc");
            mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs /run");
            assert_eq!(sys_mkdir(b"/run/systemd\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/run/systemd/mount-rootfs\0".as_ptr(), 0o755), 0);
            assert_eq!(
                sys_mkdir(b"/run/systemd/namespace-test\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                mount::sys_mount(
                    b"/\0".as_ptr(),
                    b"/run/systemd/mount-rootfs\0".as_ptr(),
                    core::ptr::null(),
                    (crate::include::uapi::mount::MS_BIND | crate::include::uapi::mount::MS_REC)
                        as u64,
                    core::ptr::null(),
                ),
                0
            );
            mount::do_mount("tmpfs", "tmpfs", "/run/systemd/namespace-test", 0, "")
                .expect("namespace tmpfs");
            assert!(
                crate::fs::openat::sys_openat(
                    AT_FDCWD,
                    b"/run/systemd/namespace-test/marker\0".as_ptr(),
                    (O_CREAT | O_RDWR) as i32,
                    0o644,
                ) >= 0
            );
            let marker_fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run/systemd/namespace-test/marker\0".as_ptr(),
                O_PATH as i32,
                0,
            );
            assert!(marker_fd >= 0);
            let marker_target = std::format!("/proc/self/fd/{}\0", marker_fd);
            let mounts_before = mount::MOUNTS.by_path.lock().len();
            assert_eq!(
                mount::sys_mount(
                    b"/run/systemd/namespace-test/marker\0".as_ptr(),
                    marker_target.as_ptr(),
                    core::ptr::null(),
                    (crate::include::uapi::mount::MS_BIND | crate::include::uapi::mount::MS_REC)
                        as u64,
                    core::ptr::null(),
                ),
                0
            );
            assert_eq!(
                mount::MOUNTS.by_path.lock().len(),
                mounts_before + 1,
                "first exact self bind should make a file visible as a mount point"
            );
            assert_eq!(
                mount::sys_mount(
                    b"/run/systemd/namespace-test/marker\0".as_ptr(),
                    marker_target.as_ptr(),
                    core::ptr::null(),
                    (crate::include::uapi::mount::MS_BIND | crate::include::uapi::mount::MS_REC)
                        as u64,
                    core::ptr::null(),
                ),
                0
            );
            assert_eq!(
                mount::MOUNTS.by_path.lock().len(),
                mounts_before + 1,
                "repeated exact self bind should not stack another mount"
            );

            let mut st = LinuxStat::default();
            assert_eq!(
                sys_newfstatat(
                    AT_FDCWD,
                    b"/run/systemd/mount-rootfs/proc/kmsg\0".as_ptr(),
                    &mut st,
                    0,
                ),
                0,
                "recursive root bind should carry the host /proc submount"
            );

            let target_fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run/systemd/mount-rootfs/proc\0".as_ptr(),
                (O_PATH | O_DIRECTORY) as i32,
                0,
            );
            assert!(target_fd >= 0);
            let target = std::format!("/proc/self/fd/{}\0", target_fd);
            assert_eq!(
                mount::sys_mount(
                    b"/run/systemd/namespace-test\0".as_ptr(),
                    target.as_ptr(),
                    core::ptr::null(),
                    crate::include::uapi::mount::MS_BIND as u64,
                    core::ptr::null(),
                ),
                0
            );

            assert_eq!(
                sys_newfstatat(
                    AT_FDCWD,
                    b"/run/systemd/mount-rootfs/proc/marker\0".as_ptr(),
                    &mut st,
                    0,
                ),
                0
            );
            assert_eq!(
                sys_newfstatat(AT_FDCWD, b"/proc/marker\0".as_ptr(), &mut st, 0,),
                -(ENOENT as i64)
            );
            assert_eq!(
                sys_newfstatat(AT_FDCWD, b"/proc/kmsg\0".as_ptr(), &mut st, 0,),
                0
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn proc_fd_file_bind_mount_is_visible_in_mountinfo() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(884);
        unsafe {
            assert_eq!(sys_mkdir(b"/proc\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            mount::do_mount("proc", "proc", "/proc", 0, "").expect("proc");
            mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs /run");
            assert_eq!(sys_mkdir(b"/run/systemd\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/run/systemd/mount-rootfs\0".as_ptr(), 0o755), 0);
            assert_eq!(
                mount::sys_mount(
                    b"/\0".as_ptr(),
                    b"/run/systemd/mount-rootfs\0".as_ptr(),
                    core::ptr::null(),
                    (crate::include::uapi::mount::MS_BIND | crate::include::uapi::mount::MS_REC)
                        as u64,
                    core::ptr::null(),
                ),
                0
            );

            let fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run/systemd/mount-rootfs/proc/kmsg\0".as_ptr(),
                O_PATH as i32,
                0,
            );
            assert!(fd >= 0);
            let target = std::format!("/proc/self/fd/{}\0", fd);
            assert_eq!(
                mount::sys_mount(
                    b"/run/systemd/mount-rootfs/proc/kmsg\0".as_ptr(),
                    target.as_ptr(),
                    core::ptr::null(),
                    (crate::include::uapi::mount::MS_BIND | crate::include::uapi::mount::MS_REC)
                        as u64,
                    core::ptr::null(),
                ),
                0
            );

            let mountinfo = crate::fs::proc_namespace::render_mountinfo();
            assert!(
                mountinfo
                    .lines()
                    .any(|line| line.contains(" /run/systemd/mount-rootfs/proc/kmsg ")),
                "mountinfo must expose the proc-fd bind target, got:\n{}",
                mountinfo
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn dev_kmsg_is_epollable_for_systemd_journald() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(885);
        unsafe {
            assert_eq!(sys_mkdir(b"/dev\0".as_ptr(), 0o755), 0);
            mount::do_mount("devtmpfs", "devtmpfs", "/dev", 0, "").expect("devtmpfs /dev");

            let kmsg = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/dev/kmsg\0".as_ptr(),
                (O_RDONLY | O_NONBLOCK | O_CLOEXEC) as i32,
                0,
            );
            assert!(kmsg >= 0, "open /dev/kmsg returned {kmsg}");
            let epfd = crate::fs::eventpoll::sys_epoll_create1(0);
            assert!(epfd >= 0, "epoll_create1 returned {epfd}");
            let ev = crate::fs::eventpoll::EpollEvent {
                events: crate::fs::eventpoll::EPOLLIN,
                data: 0x6b6d_7367,
            };
            assert_eq!(
                crate::fs::eventpoll::sys_epoll_ctl(
                    epfd as i32,
                    crate::fs::eventpoll::EPOLL_CTL_ADD,
                    kmsg as i32,
                    &ev,
                ),
                0
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn runtime_directory_chown_sequence_handles_open_dirfd_empty_path_ops() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(881);
        unsafe {
            assert_eq!(sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs on /run");
            assert_eq!(sys_mkdir(b"/run/systemd\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdir(b"/run/systemd/netif\0".as_ptr(), 0o755), 0);

            let fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/run/systemd/netif\0".as_ptr(),
                (O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOATIME) as i32,
                0,
            );
            assert!(fd >= 0);

            let mut st = LinuxStat::default();
            assert_eq!(sys_fstat(fd as i32, &mut st), 0);
            let mut dirents = [0u8; 256];
            let dirent_len = sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(dirent_len > 0);
            assert_eq!(
                dirent64_names(&dirents, dirent_len as usize),
                [String::from("."), String::from("..")]
            );
            assert_eq!(
                sys_getdents64(fd as i32, dirents.as_mut_ptr(), dirents.len()),
                0
            );

            let acl = b"system.posix_acl_access\0";
            assert_eq!(sys_fremovexattr(fd as i32, acl.as_ptr()), -(ENODATA as i64));
            let proc_fd = std::format!("/proc/self/fd/{}\0", fd);
            assert_eq!(
                sys_removexattr(proc_fd.as_ptr(), acl.as_ptr()),
                -(ENODATA as i64)
            );
            let empty = b"\0";
            assert_eq!(
                sys_fchmodat(fd as i32, empty.as_ptr(), 0o755),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_fchmodat2(fd as i32, empty.as_ptr(), 0o755, AT_EMPTY_PATH as i32),
                0
            );
            assert_eq!(
                sys_fchownat(fd as i32, empty.as_ptr(), 995, 995, AT_EMPTY_PATH as i32),
                0
            );
            assert_eq!(sys_fstat(fd as i32, &mut st), 0);
            assert_eq!(st.st_uid, 995);
            assert_eq!(st.st_gid, 995);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn new_mount_api_attaches_tmpfs_detached_mount() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(88);
        unsafe {
            assert_eq!(sys_mkdir(b"/tmp\0".as_ptr(), 0o755), 0);

            let fsfd = sys_fsopen(b"tmpfs\0".as_ptr(), FSOPEN_CLOEXEC);
            assert!(fsfd >= 0);
            let mut log_buf = [0u8; 8];
            assert_eq!(
                crate::fs::read_write::sys_read(fsfd as i32, log_buf.as_mut_ptr(), log_buf.len()),
                -(ENODATA as i64)
            );

            assert_eq!(
                sys_fsconfig(
                    fsfd as i32,
                    FSCONFIG_SET_STRING,
                    b"source\0".as_ptr(),
                    b"tmpfs\0".as_ptr(),
                    0,
                ),
                0
            );
            assert_eq!(
                sys_fsconfig(
                    fsfd as i32,
                    FSCONFIG_SET_STRING,
                    b"mode\0".as_ptr(),
                    b"1777\0".as_ptr(),
                    0,
                ),
                0
            );
            assert_eq!(
                sys_fsconfig(
                    fsfd as i32,
                    FSCONFIG_CMD_CREATE,
                    core::ptr::null(),
                    core::ptr::null(),
                    0,
                ),
                0
            );

            let mfd = sys_fsmount(fsfd as i32, FSMOUNT_CLOEXEC, 0);
            assert!(mfd >= 0);
            let mut stx = LinuxStatx::default();
            assert_eq!(
                sys_statx(
                    mfd as i32,
                    b"\0".as_ptr(),
                    AT_EMPTY_PATH as i32,
                    STATX_TYPE | STATX_INO,
                    &mut stx,
                ),
                0
            );

            let attr = MountAttr {
                attr_set: MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV | MOUNT_ATTR_STRICTATIME,
                attr_clr: MOUNT_ATTR__ATIME,
                propagation: 0,
                userns_fd: 0,
            };
            assert_eq!(
                sys_mount_setattr(
                    mfd as i32,
                    b"\0".as_ptr(),
                    AT_EMPTY_PATH,
                    (&attr as *const MountAttr).cast::<u8>(),
                    core::mem::size_of::<MountAttr>(),
                ),
                0
            );
            assert_eq!(
                sys_move_mount(
                    mfd as i32,
                    b"\0".as_ptr(),
                    AT_FDCWD,
                    b"/tmp\0".as_ptr(),
                    MOVE_MOUNT_F_EMPTY_PATH,
                ),
                0
            );
            let mounted = mount::lookup_mount("/tmp").expect("/tmp mount");
            let flags = mounted.flags.load(Ordering::Acquire) as u64;
            assert_ne!(flags & MS_NOSUID, 0);
            assert_ne!(flags & MS_NODEV, 0);
            assert_ne!(flags & MS_STRICTATIME, 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn mount_setattr_accepts_recursive_nofollow_flags() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(89);
        unsafe {
            assert_eq!(sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs /run");
            assert_eq!(sys_mkdir(b"/run/systemd\0".as_ptr(), 0o755), 0);
            mount::do_mount("tmpfs", "tmpfs", "/run/systemd", 0, "").expect("tmpfs /run/systemd");

            let attr = MountAttr {
                attr_set: MOUNT_ATTR_NODEV | MOUNT_ATTR_NOEXEC,
                ..Default::default()
            };
            assert_eq!(
                sys_mount_setattr(
                    AT_FDCWD,
                    b"/run\0".as_ptr(),
                    AT_RECURSIVE | AT_SYMLINK_NOFOLLOW | AT_NO_AUTOMOUNT,
                    (&attr as *const MountAttr).cast::<u8>(),
                    core::mem::size_of::<MountAttr>(),
                ),
                0
            );

            for path in ["/run", "/run/systemd"] {
                let mounted = mount::lookup_mount(path).expect("mount");
                let flags = mounted.flags.load(Ordering::Acquire) as u64;
                assert_ne!(flags & MS_NODEV, 0, "{path} should inherit NODEV");
                assert_ne!(flags & MS_NOEXEC, 0, "{path} should inherit NOEXEC");
            }

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_statx_reports_mount_root_attributes() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(176);
        unsafe {
            assert_eq!(sys_mkdir(b"/proc\0".as_ptr(), 0o755), 0);
            mount::do_mount("proc", "", "/proc", 0, "").expect("mount procfs");

            let mut stx = LinuxStatx::default();
            let flags = (AT_NO_AUTOMOUNT | AT_STATX_DONT_SYNC) as i32;
            assert_eq!(
                sys_statx(
                    AT_FDCWD,
                    b"/proc\0".as_ptr(),
                    flags,
                    STATX_TYPE | STATX_INO,
                    &mut stx
                ),
                0
            );
            assert_eq!(
                stx.stx_attributes_mask & STATX_ATTR_MOUNT_ROOT,
                STATX_ATTR_MOUNT_ROOT
            );
            assert_eq!(
                stx.stx_attributes & STATX_ATTR_MOUNT_ROOT,
                STATX_ATTR_MOUNT_ROOT
            );
            assert_ne!(stx.stx_mnt_id, 0);

            let fd = sys_open(b"/proc\0".as_ptr(), (O_RDONLY | O_DIRECTORY) as i32, 0);
            assert!(fd >= 0);
            let mut by_fd = LinuxStatx::default();
            assert_eq!(
                sys_statx(
                    fd as i32,
                    b"\0".as_ptr(),
                    AT_EMPTY_PATH as i32,
                    STATX_TYPE | STATX_INO,
                    &mut by_fd
                ),
                0
            );
            assert_eq!(
                by_fd.stx_attributes & STATX_ATTR_MOUNT_ROOT,
                STATX_ATTR_MOUNT_ROOT
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_statx_rejects_raw_symlink_follow_flag() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(179);
        unsafe {
            let mut stx = LinuxStatx::default();
            assert_eq!(
                sys_statx(
                    AT_FDCWD,
                    b"/\0".as_ptr(),
                    AT_SYMLINK_FOLLOW as i32,
                    STATX_TYPE,
                    &mut stx,
                ),
                -(EINVAL as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_statx_reports_unique_mount_id_mask_when_requested() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(180);
        unsafe {
            assert_eq!(sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs /run");

            let mut stx = LinuxStatx::default();
            assert_eq!(
                sys_statx(
                    AT_FDCWD,
                    b"/run\0".as_ptr(),
                    (AT_NO_AUTOMOUNT | AT_STATX_DONT_SYNC) as i32,
                    STATX_TYPE | STATX_INO | STATX_MNT_ID | STATX_MNT_ID_UNIQUE,
                    &mut stx,
                ),
                0
            );
            assert_eq!(stx.stx_mask & STATX_MNT_ID_UNIQUE, STATX_MNT_ID_UNIQUE);
            assert_eq!(stx.stx_mask & STATX_MNT_ID, 0);
            assert_ne!(stx.stx_mnt_id, 0);

            let fd = sys_open(b"/run\0".as_ptr(), (O_RDONLY | O_DIRECTORY) as i32, 0);
            assert!(fd >= 0);
            let mut by_fd = LinuxStatx::default();
            assert_eq!(
                sys_file_getattr(fd as i32, STATX_MNT_ID_UNIQUE, 0, &mut by_fd),
                0
            );
            assert_eq!(by_fd.stx_mask & STATX_MNT_ID_UNIQUE, STATX_MNT_ID_UNIQUE);
            assert_eq!(by_fd.stx_mask & STATX_MNT_ID, 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m77_vfs_rw_fd_parity() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(177);
        unsafe {
            let path = b"/m77-rw\0";
            let fd = sys_open(path.as_ptr(), (O_CREAT | O_RDWR) as i32, 0o644);
            assert!(fd >= 0);
            let payload = b"abcdef";
            assert_eq!(
                crate::fs::read_write::sys_write(fd as i32, payload.as_ptr(), payload.len()),
                6
            );
            assert_eq!(sys_lseek(fd as i32, 0, SEEK_SET), 0);
            let mut out = [0u8; 6];
            assert_eq!(
                crate::fs::read_write::sys_read(fd as i32, out.as_mut_ptr(), out.len()),
                6
            );
            assert_eq!(&out, payload);

            let more = b"XYZ";
            assert_eq!(sys_pwrite64(fd as i32, more.as_ptr(), more.len(), 2), 3);
            let mut positioned = [0u8; 3];
            assert_eq!(
                sys_pread64(fd as i32, positioned.as_mut_ptr(), positioned.len(), 2),
                3
            );
            assert_eq!(&positioned, more);
            assert_eq!(
                sys_pread64(fd as i32, positioned.as_mut_ptr(), positioned.len(), -1),
                -(EINVAL as i64)
            );

            let iov1 = b"12";
            let iov2 = b"34";
            let wiov = [
                IoVec {
                    iov_base: iov1.as_ptr() as *mut u8,
                    iov_len: iov1.len(),
                },
                IoVec {
                    iov_base: iov2.as_ptr() as *mut u8,
                    iov_len: iov2.len(),
                },
            ];
            assert_eq!(sys_pwritev(fd as i32, wiov.as_ptr(), wiov.len(), 0), 4);
            let mut r1 = [0u8; 2];
            let mut r2 = [0u8; 2];
            let riov = [
                IoVec {
                    iov_base: r1.as_mut_ptr(),
                    iov_len: r1.len(),
                },
                IoVec {
                    iov_base: r2.as_mut_ptr(),
                    iov_len: r2.len(),
                },
            ];
            assert_eq!(sys_preadv(fd as i32, riov.as_ptr(), riov.len(), 0), 4);
            assert_eq!(
                (&r1, &r2),
                (
                    &b"12"[..].try_into().unwrap(),
                    &b"34"[..].try_into().unwrap()
                )
            );
            assert_eq!(
                sys_preadv2(fd as i32, riov.as_ptr(), riov.len(), 0, 1),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_pwritev2(fd as i32, wiov.as_ptr(), wiov.len(), 0, 1),
                -(EINVAL as i64)
            );

            let mut st = LinuxStat::default();
            assert_eq!(sys_fstat(fd as i32, &mut st), 0);
            assert_eq!(sys_stat(path.as_ptr(), &mut st), 0);
            assert_eq!(sys_lstat(path.as_ptr(), &mut st), 0);
            assert_eq!(sys_newfstatat(AT_FDCWD, path.as_ptr(), &mut st, 0), 0);
            let mut stx = LinuxStatx::default();
            assert_eq!(sys_statx(AT_FDCWD, path.as_ptr(), 0, 0xffff, &mut stx), 0);
            assert_eq!(
                sys_stat(path.as_ptr(), core::ptr::null_mut()),
                -(EFAULT as i64)
            );

            let dupfd = sys_dup(fd as i32);
            assert!(dupfd >= 0);
            assert_eq!(sys_dup2(fd as i32, 10), 10);
            assert_eq!(sys_dup3(fd as i32, 11, O_CLOEXEC as i32), 11);
            assert_eq!(sys_dup3(fd as i32, fd as i32, 0), -(EINVAL as i64));
            assert_eq!(sys_fcntl(fd as i32, 1, 0), 0);
            assert_eq!(sys_flock(fd as i32, 0), 0);
            assert_eq!(crate::fs::fdtable::sys_close(dupfd as i32), 0);
            assert_eq!(sys_close_range(10, 11, 0), 0);

            let mut pipefds = [0i32; 2];
            assert_eq!(crate::fs::pipe::sys_pipe(pipefds.as_mut_ptr()), 0);
            assert_eq!(
                crate::fs::pipe::sys_pipe2(core::ptr::null_mut(), 0),
                -(EFAULT as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m77_path_metadata_parity() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(178);
        unsafe {
            let dir = b"/m77-dir\0";
            assert_eq!(sys_mkdir(dir.as_ptr(), 0o755), 0);
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/m77-dir/sub\0".as_ptr(), 0o755), 0);
            assert_eq!(
                sys_mkdirat(AT_FDCWD, b"/m77-dir/sub\0".as_ptr(), 0o755),
                -(EEXIST as i64)
            );
            let nested_file = b"/m77-dir/sub/nested\0";
            let nested_fd = sys_creat(nested_file.as_ptr(), 0o644);
            assert!(
                nested_fd >= 0,
                "mkdir-created directories must be visible to later path opens"
            );
            assert_eq!(crate::fs::fdtable::sys_close(nested_fd as i32), 0);
            assert_eq!(sys_unlink(nested_file.as_ptr()), 0);
            let dirfd = sys_open(dir.as_ptr(), (O_RDONLY | O_DIRECTORY) as i32, 0);
            assert!(dirfd >= 0);
            let mut dirents = [0u8; 256];
            let dirent_len = sys_getdents64(dirfd as i32, dirents.as_mut_ptr(), dirents.len());
            assert!(dirent_len > 0);
            let mut off = 0usize;
            let mut saw_sub = false;
            while off < dirent_len as usize {
                let reclen = u16::from_ne_bytes([dirents[off + 16], dirents[off + 17]]) as usize;
                assert!(reclen >= 20);
                let dtype = dirents[off + 18];
                let name_start = off + 19;
                let name_end = dirents[name_start..off + reclen]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map(|pos| name_start + pos)
                    .expect("dirent nul");
                if &dirents[name_start..name_end] == b"sub" {
                    saw_sub = true;
                    assert_eq!(dtype, 4);
                }
                off += reclen;
            }
            assert!(saw_sub, "getdents64 must preserve the first byte of names");
            assert_eq!(sys_chdir(dir.as_ptr()), 0);
            assert_eq!(sys_chdir(b"\0".as_ptr()), -(ENOENT as i64));
            let mut cwd = [0u8; 32];
            assert!(sys_getcwd(cwd.as_mut_ptr(), cwd.len()) > 0);
            assert_eq!(sys_fchdir(-1), -(EBADF as i64));
            crate::fs::fs_struct::set_current_cwd_path("/");

            let file = b"/m77-dir/file\0";
            let fd = sys_creat(file.as_ptr(), 0o644);
            assert!(fd >= 0);
            assert_eq!(
                sys_symlink(b"file\0".as_ptr(), b"/m77-dir/file-link\0".as_ptr()),
                0
            );
            let mut stat_buf = LinuxStat::default();
            assert_eq!(sys_stat(b"/m77-dir/file-link\0".as_ptr(), &mut stat_buf), 0);
            assert_eq!(
                stat_buf.st_mode & crate::include::uapi::stat::S_IFMT,
                crate::include::uapi::stat::S_IFREG
            );
            assert_eq!(
                sys_lstat(b"/m77-dir/file-link\0".as_ptr(), &mut stat_buf),
                0
            );
            assert_eq!(
                stat_buf.st_mode & crate::include::uapi::stat::S_IFMT,
                crate::include::uapi::stat::S_IFLNK
            );
            let renamed = b"/m77-dir/renamed\0";
            assert_eq!(sys_rename(file.as_ptr(), renamed.as_ptr()), 0);
            assert_eq!(sys_stat(renamed.as_ptr(), &mut stat_buf), 0);
            assert_eq!(sys_stat(file.as_ptr(), &mut stat_buf), -(ENOENT as i64));
            assert_eq!(
                sys_renameat(AT_FDCWD, renamed.as_ptr(), AT_FDCWD, file.as_ptr()),
                0
            );
            assert_eq!(
                sys_renameat2(
                    AT_FDCWD,
                    file.as_ptr(),
                    AT_FDCWD,
                    renamed.as_ptr(),
                    RENAME_NOREPLACE
                ),
                0
            );
            let noreplace_target = b"/m77-dir/noreplace-target\0";
            assert!(sys_creat(noreplace_target.as_ptr(), 0o644) >= 0);
            assert_eq!(
                sys_renameat2(
                    AT_FDCWD,
                    renamed.as_ptr(),
                    AT_FDCWD,
                    noreplace_target.as_ptr(),
                    RENAME_NOREPLACE
                ),
                -(EEXIST as i64)
            );
            assert_eq!(sys_rename(renamed.as_ptr(), file.as_ptr()), 0);
            let hard = b"/m77-dir/hard\0";
            assert_eq!(sys_link(file.as_ptr(), hard.as_ptr()), 0);
            let mut hard_st = LinuxStat::default();
            assert_eq!(sys_stat(file.as_ptr(), &mut stat_buf), 0);
            assert_eq!(sys_stat(hard.as_ptr(), &mut hard_st), 0);
            assert_eq!(hard_st.st_ino, stat_buf.st_ino);
            assert_eq!(stat_buf.st_nlink, 2);
            assert_eq!(sys_link(file.as_ptr(), hard.as_ptr()), -(EEXIST as i64));
            assert_eq!(
                sys_link(dir.as_ptr(), b"/m77-dir/dir-hard\0".as_ptr()),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_symlink(b"/target\0".as_ptr(), b"/m77-dir/sym\0".as_ptr()),
                0
            );
            assert_eq!(
                sys_symlink(b"/target\0".as_ptr(), b"/m77-dir/sym\0".as_ptr()),
                -(EEXIST as i64)
            );
            let mut linkbuf = [0u8; 16];
            assert_eq!(
                sys_readlink(
                    b"/m77-dir/sym\0".as_ptr(),
                    linkbuf.as_mut_ptr(),
                    linkbuf.len()
                ),
                7
            );
            assert_eq!(&linkbuf[..7], b"/target");
            linkbuf.fill(0);
            assert_eq!(
                sys_readlinkat(
                    dirfd as i32,
                    b"sym\0".as_ptr(),
                    linkbuf.as_mut_ptr(),
                    linkbuf.len()
                ),
                7
            );
            assert_eq!(&linkbuf[..7], b"/target");
            assert_eq!(
                sys_readlinkat(
                    AT_FDCWD,
                    b"/m77-dir/sym\0".as_ptr(),
                    linkbuf.as_mut_ptr(),
                    linkbuf.len()
                ),
                7
            );
            assert_eq!(
                sys_readlink(b"/m77-dir/sym\0".as_ptr(), linkbuf.as_mut_ptr(), 0),
                -(EINVAL as i64)
            );
            let symfd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"/m77-dir/sym\0".as_ptr(),
                (O_PATH | O_NOFOLLOW) as i32,
                0,
            );
            assert!(symfd >= 0);
            linkbuf.fill(0);
            assert_eq!(
                sys_readlinkat(
                    symfd as i32,
                    b"\0".as_ptr(),
                    linkbuf.as_mut_ptr(),
                    linkbuf.len()
                ),
                7
            );
            assert_eq!(&linkbuf[..7], b"/target");
            let regular_fd =
                crate::fs::openat::sys_openat(AT_FDCWD, file.as_ptr(), O_PATH as i32, 0);
            assert!(regular_fd >= 0);
            assert_eq!(
                sys_readlinkat(
                    regular_fd as i32,
                    b"\0".as_ptr(),
                    linkbuf.as_mut_ptr(),
                    linkbuf.len()
                ),
                -(ENOENT as i64)
            );
            assert_eq!(
                sys_readlink(file.as_ptr(), linkbuf.as_mut_ptr(), linkbuf.len()),
                -(EINVAL as i64)
            );
            assert_eq!(sys_mknod(b"/m77-dir/node\0".as_ptr(), 0o600, 0), 0);
            let mut node_st = LinuxStat::default();
            assert_eq!(sys_stat(b"/m77-dir/node\0".as_ptr(), &mut node_st), 0);
            assert_eq!(
                node_st.st_mode & crate::include::uapi::stat::S_IFMT,
                crate::include::uapi::stat::S_IFREG
            );
            for (path, mode, expected) in [
                (
                    b"/m77-dir/fifo\0".as_slice(),
                    crate::include::uapi::stat::S_IFIFO | 0o000,
                    crate::include::uapi::stat::S_IFIFO,
                ),
                (
                    b"/m77-dir/sock\0".as_slice(),
                    crate::include::uapi::stat::S_IFSOCK | 0o000,
                    crate::include::uapi::stat::S_IFSOCK,
                ),
                (
                    b"/m77-dir/chr\0".as_slice(),
                    crate::include::uapi::stat::S_IFCHR | 0o000,
                    crate::include::uapi::stat::S_IFCHR,
                ),
                (
                    b"/m77-dir/blk\0".as_slice(),
                    crate::include::uapi::stat::S_IFBLK | 0o000,
                    crate::include::uapi::stat::S_IFBLK,
                ),
            ] {
                assert_eq!(sys_mknodat(AT_FDCWD, path.as_ptr(), mode, 0), 0);
                let mut st = LinuxStat::default();
                assert_eq!(sys_lstat(path.as_ptr(), &mut st), 0);
                assert_eq!(st.st_mode & crate::include::uapi::stat::S_IFMT, expected);
            }
            assert_eq!(sys_unlink(file.as_ptr()), 0);
            assert_eq!(sys_unlink(hard.as_ptr()), 0);
            assert_eq!(
                sys_unlinkat(AT_FDCWD, b"/m77-dir/missing\0".as_ptr(), 0),
                -(ENOENT as i64)
            );
            assert_eq!(sys_rmdir(b"/m77-dir/sub\0".as_ptr()), 0);

            let mut sfs = LinuxStatFs::default();
            assert_eq!(sys_statfs(dir.as_ptr(), &mut sfs), 0);
            assert_eq!(sys_fstatfs(fd as i32, &mut sfs), 0);
            assert_eq!(sys_ustat(0, core::ptr::null_mut()), -(EFAULT as i64));

            let how = OpenHow {
                flags: O_RDWR as u64,
                mode: 0,
                resolve: 0,
            };
            let openat2_path = b"/m77-openat2\0";
            assert!(sys_creat(openat2_path.as_ptr(), 0o644) >= 0);
            let fd2 = sys_openat2(
                AT_FDCWD,
                openat2_path.as_ptr(),
                &how,
                core::mem::size_of::<OpenHow>(),
            );
            assert!(fd2 >= 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unlinkat_at_fdcwd_uses_current_working_directory() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(184);
        unsafe {
            assert_eq!(sys_mkdirat(AT_FDCWD, b"/work\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_chdir(b"/work\0".as_ptr()), 0);

            let fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b"tmpfile\0".as_ptr(),
                (O_CREAT | O_RDWR) as i32,
                0o600,
            );
            assert!(fd >= 0, "relative openat(O_CREAT) should use cwd");
            assert_eq!(sys_unlink(b"tmpfile\0".as_ptr()), 0);

            let mut st = LinuxStat::default();
            assert_eq!(sys_stat(b"tmpfile\0".as_ptr(), &mut st), -(ENOENT as i64));

            crate::fs::fs_struct::set_current_cwd_path("/");
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn chroot_rebases_absolute_paths_and_getcwd() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(185);
        unsafe {
            assert_eq!(sys_mkdir(b"/jail\0".as_ptr(), 0o755), 0);
            assert_eq!(sys_chdir(b"/jail\0".as_ptr()), 0);
            assert_eq!(sys_chroot(b".\0".as_ptr()), 0);

            let mut cwd = [0u8; 8];
            assert_eq!(sys_getcwd(cwd.as_mut_ptr(), cwd.len()), 2);
            assert_eq!(&cwd[..2], b"/\0");

            assert_eq!(sys_mkdir(b"/inside\0".as_ptr(), 0o755), 0);
            assert!(mount::resolve_path_follow("/inside").is_ok());
            assert!(mount::resolve_path_follow("/jail/inside").is_err());

            crate::fs::fs_struct::exit_fs(&mut *current as *mut TaskStruct);
            crate::fs::fs_struct::set_current_cwd_path("/");
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn chroot_absolute_symlink_targets_stay_beneath_chroot_root() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(186);
        unsafe {
            assert_eq!(sys_mkdir(b"/etc\0".as_ptr(), 0o755), 0);
            let outside_fd = sys_open(b"/etc/passwd\0".as_ptr(), (O_CREAT | O_RDWR) as i32, 0o644);
            assert!(outside_fd >= 0);
            assert_eq!(sys_mkdir(b"/jail\0".as_ptr(), 0o755), 0);
            assert_eq!(
                sys_symlink(b"/etc/passwd\0".as_ptr(), b"/jail/link\0".as_ptr()),
                0
            );

            assert_eq!(sys_chroot(b"/jail\0".as_ptr()), 0);
            let mut st = LinuxStat::default();
            assert_eq!(sys_stat(b"/link\0".as_ptr(), &mut st), -(ENOENT as i64));

            assert_eq!(sys_mkdir(b"/etc\0".as_ptr(), 0o755), 0);
            let inside_fd = sys_open(b"/etc/passwd\0".as_ptr(), (O_CREAT | O_RDWR) as i32, 0o644);
            assert!(inside_fd >= 0);
            assert_eq!(sys_stat(b"/link\0".as_ptr(), &mut st), 0);

            crate::fs::fs_struct::exit_fs(&mut *current as *mut TaskStruct);
            crate::fs::fs_struct::set_current_cwd_path("/");
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m77_poll_epoll_eventfd_parity() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(179);
        unsafe {
            let fd = crate::fs::eventfd::sys_eventfd(1);
            assert!(fd >= 0);
            let mut pfd = PollFd {
                fd: fd as i32,
                events: POLLIN | POLLOUT,
                revents: 0,
            };
            assert_eq!(sys_poll(&mut pfd, 1, 0), 1);
            assert_ne!(pfd.revents & POLLIN, 0);
            let mut readfds = 1u64 << fd;
            assert_eq!(
                sys_select(
                    fd as i32 + 1,
                    &mut readfds,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut()
                ),
                1
            );
            assert_eq!(
                sys_pselect6(
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    &crate::kernel::time::Timespec64::new(0, 0),
                    core::ptr::null()
                ),
                0
            );
            let bad_user_timeout = (1u64 << 47) as *const crate::kernel::time::Timespec64;
            assert_eq!(
                sys_pselect6(
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    bad_user_timeout,
                    core::ptr::null()
                ),
                -(EFAULT as i64)
            );
            assert_eq!(
                sys_ppoll(
                    core::ptr::null_mut(),
                    0,
                    bad_user_timeout,
                    core::ptr::null(),
                    0
                ),
                -(EFAULT as i64)
            );
            let zero_timeout = crate::kernel::time::Timespec64::new(0, 0);
            assert_eq!(
                sys_pselect6(
                    fd as i32 + 1,
                    &mut readfds,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    &zero_timeout,
                    core::ptr::null()
                ),
                1
            );
            let invalid_timeout = crate::kernel::time::Timespec64::new(0, 1_000_000_000);
            assert_eq!(
                sys_pselect6(
                    fd as i32 + 1,
                    &mut readfds,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    &invalid_timeout,
                    core::ptr::null()
                ),
                -(EINVAL as i64)
            );
            let bad_timeval = (1u64 << 47) as *mut crate::kernel::syscalls::TimeVal;
            assert_eq!(
                sys_select(
                    fd as i32 + 1,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    bad_timeval,
                ),
                -(EFAULT as i64)
            );
            let bad_timespec = (1u64 << 47) as *const crate::kernel::time::Timespec64;
            assert_eq!(
                sys_pselect6(
                    fd as i32 + 1,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    bad_timespec,
                    core::ptr::null()
                ),
                -(EFAULT as i64)
            );
            let bad_fdset = (1u64 << 47) as *mut u64;
            assert_eq!(
                sys_select(
                    fd as i32 + 1,
                    bad_fdset,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                -(EFAULT as i64)
            );
            assert_eq!(
                sys_ppoll(
                    core::ptr::null_mut(),
                    0,
                    &zero_timeout,
                    core::ptr::null(),
                    0
                ),
                0
            );
            assert_eq!(
                sys_ppoll(&mut pfd, 1, &zero_timeout, core::ptr::null(), 0),
                1
            );
            assert_eq!(
                sys_ppoll(&mut pfd, 1, &invalid_timeout, core::ptr::null(), 0),
                -(EINVAL as i64)
            );

            assert_eq!(
                crate::fs::eventfd::sys_eventfd2(0, 0x8000_0000u32 as i32),
                -(EINVAL as i64)
            );
            let epfd = crate::fs::eventpoll::sys_epoll_create1(0);
            assert!(epfd >= 0);
            assert_eq!(crate::fs::eventpoll::sys_epoll_create(1) >= 0, true);
            let ev = crate::fs::eventpoll::EpollEvent {
                events: POLLIN as u32,
                data: 7,
            };
            let regular = sys_open(
                b"/m77-regular-file\0".as_ptr(),
                (O_CREAT | O_RDONLY) as i32,
                0o644,
            );
            assert!(regular >= 0);
            assert_eq!(
                crate::fs::eventpoll::sys_epoll_ctl(epfd as i32, 1, regular as i32, &ev),
                -(EPERM as i64)
            );
            assert_eq!(
                crate::fs::eventpoll::sys_epoll_ctl(epfd as i32, 1, fd as i32, &ev),
                0
            );
            let mut out = [crate::fs::eventpoll::EpollEvent { events: 0, data: 0 }; 4];
            assert!(
                crate::fs::eventpoll::sys_epoll_wait(
                    epfd as i32,
                    out.as_mut_ptr(),
                    out.len() as i32,
                    0
                ) >= 0
            );
            assert!(
                crate::fs::eventpoll::sys_epoll_pwait(
                    epfd as i32,
                    out.as_mut_ptr(),
                    out.len() as i32,
                    0,
                    core::ptr::null(),
                    0
                ) >= 0
            );
            assert!(
                crate::fs::eventpoll::sys_epoll_pwait2(
                    epfd as i32,
                    out.as_mut_ptr(),
                    out.len() as i32,
                    core::ptr::null(),
                    core::ptr::null(),
                    0
                ) >= 0
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn fatal_signal_interrupts_poll_and_select_after_releasing_wait_resources() {
        let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
        crate::kernel::signal::reset_for_tests();
        assert!(INTERRUPTED_POLL_QUEUE.is_empty());

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 31_101;
        current.tgid = 31_101;
        current.cred = &raw const INIT_CRED;

        unsafe {
            let ft = FilesStruct::new();
            let ft_weak = Arc::downgrade(&ft);
            files::set_task_files(&mut *current as *mut TaskStruct, ft.clone());
            sched::set_current(&mut *current as *mut TaskStruct);

            let file = crate::fs::file::alloc_file(
                crate::fs::dcache::d_alloc("interrupted-poll-file"),
                0,
                0,
                &INTERRUPTED_POLL_OPS,
            );
            let file_weak = Arc::downgrade(&file);
            let fd = ft.install(file, false).expect("install poll file");
            drop(ft);

            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *current as *mut TaskStruct,
                    crate::kernel::signal::SIGTERM,
                ),
                0
            );

            let mut pfd = PollFd {
                fd,
                events: POLLIN,
                revents: 0,
            };
            assert_eq!(sys_poll(&mut pfd, 1, -1), -(ERESTARTNOHAND as i64));
            assert!(INTERRUPTED_POLL_QUEUE.is_empty());
            assert_ne!(
                crate::kernel::signal::current_pending_signal_bits()
                    & (1u64 << (crate::kernel::signal::SIGTERM - 1)),
                0,
                "poll must leave the fatal signal queued for syscall-exit"
            );

            let mut readfds = 1u64 << fd;
            assert_eq!(
                sys_select(
                    fd + 1,
                    &mut readfds,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                ),
                -(ERESTARTNOHAND as i64)
            );
            assert!(INTERRUPTED_POLL_QUEUE.is_empty());
            assert_ne!(
                crate::kernel::signal::current_pending_signal_bits()
                    & (1u64 << (crate::kernel::signal::SIGTERM - 1)),
                0,
                "select must leave the fatal signal queued for syscall-exit"
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            assert!(
                ft_weak.upgrade().is_none(),
                "poll/select syscall-local files_struct Arc leaked"
            );
            assert!(
                file_weak.upgrade().is_none(),
                "poll/select wait-table FileRef leaked"
            );
            sched::set_current(previous);
        }
        crate::kernel::signal::reset_for_tests();
    }

    #[test]
    fn pselect_temporary_mask_unblocks_pending_signal_during_wait() {
        let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
        crate::kernel::signal::reset_for_tests();

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 31_102;
        current.tgid = 31_102;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut blocked = crate::kernel::signal::SigSet::default();
            blocked.add(crate::kernel::signal::SIGTERM);
            assert_eq!(
                crate::kernel::signal::sys_rt_sigprocmask(
                    crate::kernel::signal::SIG_SETMASK,
                    &blocked,
                    core::ptr::null_mut(),
                    core::mem::size_of::<crate::kernel::signal::SigSet>(),
                ),
                0
            );
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *current as *mut TaskStruct,
                    crate::kernel::signal::SIGTERM,
                ),
                0
            );
            assert!(
                !crate::kernel::signal::current_has_unblocked_pending_signals(),
                "the persistent mask must keep SIGTERM blocked"
            );

            let wait_mask = crate::kernel::signal::SigSet::default();
            let arg = PselectSigsetArg {
                sigmask: &wait_mask,
                sigsetsize: core::mem::size_of::<crate::kernel::signal::SigSet>(),
            };
            let zero_timeout = crate::kernel::time::Timespec64::new(0, 0);
            assert_eq!(
                sys_pselect6(
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    &zero_timeout,
                    (&raw const arg).cast::<u8>(),
                ),
                -(ERESTARTNOHAND as i64)
            );
            assert!(
                crate::kernel::signal::current_has_unblocked_pending_signals(),
                "pselect's temporary empty mask must expose pending SIGTERM"
            );

            // Model the no-handler cleanup path and prove pselect retained the
            // original mask for restoration.
            crate::kernel::signal::restore_saved_sigmask_unless(false);
            assert!(!crate::kernel::signal::current_has_unblocked_pending_signals());

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
        crate::kernel::signal::reset_for_tests();
    }

    fn drop_current_to_unprivileged(uid: u32) {
        let unpriv = crate::kernel::cred::prepare_creds().expect("unprivileged cred");
        unsafe {
            (*unpriv).uid = crate::kernel::cred::KUid(uid);
            (*unpriv).gid = crate::kernel::cred::KGid(uid);
            (*unpriv).euid = crate::kernel::cred::KUid(uid);
            (*unpriv).egid = crate::kernel::cred::KGid(uid);
            (*unpriv).fsuid = crate::kernel::cred::KUid(uid);
            (*unpriv).fsgid = crate::kernel::cred::KGid(uid);
            (*unpriv).cap_effective = crate::kernel::capability::KernelCapT::empty();
            (*unpriv).cap_permitted = crate::kernel::capability::KernelCapT::empty();
        }
        crate::kernel::cred::commit_creds(unpriv);
        assert!(!capable(CAP_SYS_ADMIN));
    }

    #[test]
    fn xattr_mutation_enforces_security_namespace_permissions() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(181);
        unsafe {
            let path = b"/xattr-security-target\0";
            let fd = sys_open(path.as_ptr(), (O_CREAT | O_RDWR) as i32, 0o600);
            assert!(fd >= 0);

            let name = b"security.ima\0";
            let value = b"trusted-digest";
            assert_eq!(
                sys_setxattr(path.as_ptr(), name.as_ptr(), value.as_ptr(), value.len(), 0),
                0
            );

            drop_current_to_unprivileged(1000);

            let forged = b"forged-ima";
            assert_eq!(
                sys_setxattr(
                    path.as_ptr(),
                    name.as_ptr(),
                    forged.as_ptr(),
                    forged.len(),
                    0
                ),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_fsetxattr(fd as i32, name.as_ptr(), forged.as_ptr(), forged.len(), 0),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_removexattr(path.as_ptr(), name.as_ptr()),
                -(EPERM as i64)
            );
            assert_eq!(sys_fremovexattr(fd as i32, name.as_ptr()), -(EPERM as i64));

            let mut readback = [0u8; 16];
            assert_eq!(
                sys_getxattr(
                    path.as_ptr(),
                    name.as_ptr(),
                    readback.as_mut_ptr(),
                    readback.len()
                ),
                value.len() as i64
            );
            assert_eq!(&readback[..value.len()], value);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m77_notify_xattr_mount_parity() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(180);
        unsafe {
            let path = b"/m77-xattr\0";
            let fd = sys_open(path.as_ptr(), (O_CREAT | O_RDWR) as i32, 0o644);
            assert!(fd >= 0);
            let name = b"user.test\0";
            assert_eq!(
                sys_setxattr(path.as_ptr(), name.as_ptr(), core::ptr::null(), 0, 0),
                0
            );
            assert_eq!(
                sys_lsetxattr(path.as_ptr(), name.as_ptr(), core::ptr::null(), 0, 0),
                0
            );
            assert_eq!(
                sys_fsetxattr(fd as i32, name.as_ptr(), core::ptr::null(), 0, 0),
                0
            );
            assert_eq!(
                sys_getxattr(path.as_ptr(), name.as_ptr(), core::ptr::null_mut(), 0),
                0
            );
            assert_eq!(
                sys_lgetxattr(path.as_ptr(), name.as_ptr(), core::ptr::null_mut(), 0),
                0
            );
            assert_eq!(
                sys_fgetxattr(fd as i32, name.as_ptr(), core::ptr::null_mut(), 0),
                0
            );
            let mut list = [0u8; 32];
            assert_eq!(
                sys_listxattr(path.as_ptr(), list.as_mut_ptr(), list.len()),
                name.len() as i64
            );
            assert_eq!(&list[..name.len()], name);
            assert_eq!(
                sys_llistxattr(path.as_ptr(), core::ptr::null_mut(), 0),
                name.len() as i64
            );
            assert_eq!(
                sys_flistxattr(fd as i32, core::ptr::null_mut(), 0),
                name.len() as i64
            );
            assert_eq!(sys_removexattr(path.as_ptr(), name.as_ptr()), 0);
            assert_eq!(
                sys_lremovexattr(path.as_ptr(), name.as_ptr()),
                -(ENODATA as i64)
            );
            assert_eq!(
                sys_fremovexattr(fd as i32, name.as_ptr()),
                -(ENODATA as i64)
            );
            assert_eq!(
                sys_setxattrat(
                    AT_FDCWD,
                    path.as_ptr(),
                    0,
                    name.as_ptr(),
                    core::ptr::null(),
                    0,
                    0
                ),
                0
            );
            assert_eq!(
                sys_getxattrat(
                    AT_FDCWD,
                    path.as_ptr(),
                    0,
                    name.as_ptr(),
                    core::ptr::null_mut(),
                    0
                ),
                0
            );
            assert_eq!(
                sys_listxattrat(AT_FDCWD, path.as_ptr(), 0, core::ptr::null_mut(), 0),
                name.len() as i64
            );
            assert_eq!(
                sys_removexattrat(AT_FDCWD, path.as_ptr(), 0, name.as_ptr()),
                0
            );

            assert!(crate::fs::inotify::sys_inotify_init1(0) >= 0);
            assert_eq!(
                crate::fs::inotify::sys_inotify_add_watch(-1, path.as_ptr() as *const i8, 0),
                -(EINVAL as i64)
            );
            assert_eq!(
                crate::fs::inotify::sys_inotify_rm_watch(-1, 1),
                -(EBADF as i64)
            );
            assert!(crate::fs::fanotify::sys_fanotify_init(0x8000_0000, 0) >= 0);
            assert_eq!(
                crate::fs::fanotify::sys_fanotify_mark(
                    -1,
                    0,
                    0,
                    AT_FDCWD,
                    path.as_ptr() as *const i8,
                ),
                -(EBADF as i64)
            );

            assert_eq!(
                crate::fs::mount::sys_mount(
                    b"ramfs\0".as_ptr(),
                    b"/\0".as_ptr(),
                    b"ramfs\0".as_ptr(),
                    0,
                    core::ptr::null()
                ),
                0
            );
            assert_eq!(sys_umount2(core::ptr::null(), 0), -(EFAULT as i64));
            assert_eq!(
                sys_pivot_root(core::ptr::null(), b"/\0".as_ptr()),
                -(EFAULT as i64)
            );
            assert_eq!(sys_chroot(path.as_ptr()), -(ENOTDIR as i64));
            assert_eq!(
                sys_open_tree(AT_FDCWD, path.as_ptr(), 0x8000),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_open_tree(AT_FDCWD, path.as_ptr(), AT_RECURSIVE),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_open_tree(
                    AT_FDCWD,
                    path.as_ptr(),
                    OPEN_TREE_CLONE | OPEN_TREE_NAMESPACE
                ),
                -(EINVAL as i64)
            );
            let run_dir = b"/run\0";
            let run_credentials = b"/run/credentials\0";
            assert!(
                sys_mkdir(run_dir.as_ptr(), 0o755) == 0
                    || sys_mkdir(run_dir.as_ptr(), 0o755) == -(EEXIST as i64)
            );
            assert!(
                sys_mkdir(run_credentials.as_ptr(), 0o700) == 0
                    || sys_mkdir(run_credentials.as_ptr(), 0o700) == -(EEXIST as i64)
            );
            let clone_fd = sys_open_tree(
                AT_FDCWD,
                run_credentials.as_ptr(),
                OPEN_TREE_CLONE | OPEN_TREE_CLOEXEC,
            );
            assert!(clone_fd >= 0);
            let mut bad_attr = MountAttr {
                userns_fd: 1,
                ..Default::default()
            };
            assert_eq!(
                sys_open_tree_attr(
                    AT_FDCWD,
                    run_credentials.as_ptr(),
                    OPEN_TREE_CLONE,
                    (&mut bad_attr as *mut _ as *const u8),
                    core::mem::size_of::<MountAttr>()
                ),
                -(EINVAL as i64)
            );
            let bad_attr_size = sys_open_tree_attr(
                AT_FDCWD,
                run_credentials.as_ptr(),
                OPEN_TREE_CLONE,
                core::ptr::null(),
                core::mem::size_of::<MountAttr>(),
            );
            assert_eq!(bad_attr_size, -(EINVAL as i64));
            assert_eq!(
                sys_move_mount(AT_FDCWD, core::ptr::null(), AT_FDCWD, path.as_ptr(), 0),
                -(EFAULT as i64)
            );
            assert_eq!(sys_fsopen(core::ptr::null(), 0), -(EFAULT as i64));
            let fsconfig_bad_fd = sys_fsconfig(-1, 8, core::ptr::null(), core::ptr::null(), 0);
            assert!(fsconfig_bad_fd == -(EINVAL as i64) || fsconfig_bad_fd == -(EBADF as i64));
            let fsmount_bad_fd = sys_fsmount(-1, 2, 0);
            assert!(fsmount_bad_fd == -(EINVAL as i64) || fsmount_bad_fd == -(EBADF as i64));
            assert_eq!(sys_fspick(AT_FDCWD, core::ptr::null(), 0), -(EFAULT as i64));
            assert_eq!(
                sys_mount_setattr(AT_FDCWD, path.as_ptr(), 0x4000, core::ptr::null(), 1),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_statmount(core::ptr::null(), 0, core::ptr::null_mut(), 0, 0),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_listmount(core::ptr::null(), 0, core::ptr::null_mut(), 0, 0),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_open_tree_attr(AT_FDCWD, core::ptr::null(), 0, core::ptr::null(), 0),
                -(EFAULT as i64)
            );
            let mut good_attr = MountAttr {
                attr_set: MOUNT_ATTR_NOSUID,
                ..Default::default()
            };
            let attr_fd = sys_open_tree_attr(
                AT_FDCWD,
                run_credentials.as_ptr(),
                OPEN_TREE_CLONE,
                (&mut good_attr as *mut _ as *const u8),
                core::mem::size_of::<MountAttr>(),
            );
            assert!(attr_fd >= 0);
            assert_eq!(crate::fs::fdtable::sys_close(attr_fd as i32), 0);
            assert_eq!(crate::fs::fdtable::sys_close(clone_fd as i32), 0);
            assert_eq!(
                sys_quotactl_fd(-1, 0, 0, core::ptr::null_mut()),
                -(EBADF as i64)
            );
            assert_eq!(
                crate::kernel::syscalls::sys_quotactl(
                    0,
                    core::ptr::null(),
                    0,
                    core::ptr::null_mut()
                ),
                0
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn unprivileged_mount_api_requires_cap_sys_admin() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(187);
        unsafe {
            let path = b"/cap-mount-api\0";
            assert_eq!(sys_mkdir(path.as_ptr(), 0o755), 0);
            let clone_fd =
                sys_open_tree(AT_FDCWD, path.as_ptr(), OPEN_TREE_CLONE | OPEN_TREE_CLOEXEC);
            assert!(clone_fd >= 0);
            let fsfd = sys_fsopen(b"tmpfs\0".as_ptr(), FSOPEN_CLOEXEC);
            assert!(fsfd >= 0);
            assert_eq!(
                sys_fsconfig(
                    fsfd as i32,
                    FSCONFIG_CMD_CREATE,
                    core::ptr::null(),
                    core::ptr::null(),
                    0,
                ),
                0
            );
            let mfd = sys_fsmount(fsfd as i32, FSMOUNT_CLOEXEC, 0);
            assert!(mfd >= 0);

            drop_current_cap_sys_admin();

            assert_eq!(
                sys_open_tree(AT_FDCWD, path.as_ptr(), OPEN_TREE_CLONE),
                -(EPERM as i64)
            );
            assert_eq!(
                mount::sys_mount(
                    b"tmpfs\0".as_ptr(),
                    path.as_ptr(),
                    b"tmpfs\0".as_ptr(),
                    0,
                    core::ptr::null(),
                ),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_fsopen(b"tmpfs\0".as_ptr(), FSOPEN_CLOEXEC),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_fsconfig(
                    fsfd as i32,
                    FSCONFIG_SET_FLAG,
                    b"ro\0".as_ptr(),
                    core::ptr::null(),
                    0,
                ),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_fsmount(fsfd as i32, FSMOUNT_CLOEXEC, 0),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_move_mount(
                    clone_fd as i32,
                    b"\0".as_ptr(),
                    AT_FDCWD,
                    path.as_ptr(),
                    MOVE_MOUNT_F_EMPTY_PATH,
                ),
                -(EPERM as i64)
            );

            let mut attr = MountAttr {
                attr_set: MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV,
                ..Default::default()
            };
            assert_eq!(
                sys_open_tree_attr(
                    AT_FDCWD,
                    path.as_ptr(),
                    0,
                    (&mut attr as *mut _ as *const u8),
                    core::mem::size_of::<MountAttr>(),
                ),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_mount_setattr(
                    AT_FDCWD,
                    path.as_ptr(),
                    0,
                    (&mut attr as *mut _ as *const u8),
                    core::mem::size_of::<MountAttr>(),
                ),
                -(EPERM as i64)
            );

            assert_eq!(crate::fs::fdtable::sys_close(mfd as i32), 0);
            assert_eq!(crate::fs::fdtable::sys_close(fsfd as i32), 0);
            assert_eq!(crate::fs::fdtable::sys_close(clone_fd as i32), 0);
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m77_sync_splice_memfd_parity() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let (mut current, previous) = setup_current_with_rootfs(181);
        unsafe {
            let a = b"/m77-a\0";
            let b = b"/m77-b\0";
            let fda = sys_open(a.as_ptr(), (O_CREAT | O_RDWR) as i32, 0o644);
            let fdb = sys_open(b.as_ptr(), (O_CREAT | O_RDWR) as i32, 0o644);
            assert!(fda >= 0 && fdb >= 0);
            assert_eq!(sys_fsync(fda as i32), 0);
            assert_eq!(sys_fdatasync(fda as i32), 0);
            assert_eq!(sys_sync(), 0);
            assert_eq!(sys_syncfs(fda as i32), 0);
            assert_eq!(sys_sync_file_range(fda as i32, -1, 0, 0), -(EINVAL as i64));
            assert_eq!(
                sys_sendfile(fdb as i32, fda as i32, core::ptr::null_mut(), 0),
                0
            );
            assert_eq!(
                sys_copy_file_range(
                    fda as i32,
                    core::ptr::null_mut(),
                    fdb as i32,
                    core::ptr::null_mut(),
                    0,
                    0
                ),
                0
            );
            assert_eq!(
                sys_splice(
                    fda as i32,
                    core::ptr::null_mut(),
                    fdb as i32,
                    core::ptr::null_mut(),
                    0,
                    0
                ),
                0
            );
            assert_eq!(sys_tee(fda as i32, fdb as i32, 0, 0), 0);
            assert_eq!(sys_vmsplice(fda as i32, core::ptr::null(), 0, 0), 0);
            let mut pipefds = [0i32; 2];
            assert_eq!(crate::fs::pipe::sys_pipe(pipefds.as_mut_ptr()), 0);
            let vmsplice_payload = b"vmsplice";
            let iov = IoVec {
                iov_base: vmsplice_payload.as_ptr() as *mut u8,
                iov_len: vmsplice_payload.len(),
            };
            assert_eq!(
                sys_vmsplice(pipefds[1], &iov as *const IoVec, 1, 0),
                vmsplice_payload.len() as i64
            );
            let mut vmsplice_out = [0u8; 8];
            assert_eq!(
                crate::fs::read_write::sys_read(
                    pipefds[0],
                    vmsplice_out.as_mut_ptr(),
                    vmsplice_out.len()
                ),
                vmsplice_out.len() as i64
            );
            assert_eq!(&vmsplice_out, vmsplice_payload);
            assert_eq!(
                sys_splice(
                    fda as i32,
                    core::ptr::null_mut(),
                    fdb as i32,
                    core::ptr::null_mut(),
                    1,
                    0x10
                ),
                -(EINVAL as i64)
            );
            let memfd = sys_memfd_create(
                b"m77\0".as_ptr(),
                crate::mm::shmem::MFD_CLOEXEC
                    | crate::mm::shmem::MFD_ALLOW_SEALING
                    | crate::mm::shmem::MFD_NOEXEC_SEAL,
            );
            assert!(memfd >= 0);
            assert_eq!(
                current_files().unwrap().get_fd_flags(memfd as i32).unwrap() & FD_CLOEXEC,
                FD_CLOEXEC
            );
            assert_eq!(
                crate::fs::read_write::sys_write(memfd as i32, b"memfd".as_ptr(), 5),
                5
            );
            assert_eq!(sys_lseek(memfd as i32, 0, SEEK_SET), 0);
            let mut memfd_out = [0u8; 5];
            assert_eq!(
                crate::fs::read_write::sys_read(
                    memfd as i32,
                    memfd_out.as_mut_ptr(),
                    memfd_out.len()
                ),
                5
            );
            assert_eq!(&memfd_out, b"memfd");
            assert_ne!(
                sys_fcntl(memfd as i32, F_GET_SEALS, 0) as u32 & F_SEAL_EXEC,
                0
            );
            assert_eq!(sys_fcntl(memfd as i32, F_ADD_SEALS, F_SEAL_WRITE as u64), 0);
            assert_eq!(
                crate::fs::read_write::sys_write(memfd as i32, b"x".as_ptr(), 1),
                -(EPERM as i64)
            );
            assert_eq!(
                sys_memfd_create(
                    b"bad\0".as_ptr(),
                    crate::mm::shmem::MFD_EXEC | crate::mm::shmem::MFD_NOEXEC_SEAL
                ),
                -(EINVAL as i64)
            );
            assert_eq!(sys_memfd_create(core::ptr::null(), 0), -(EFAULT as i64));
            let uffd = sys_userfaultfd(0);
            assert!(uffd >= 0);
            let mut unsupported_uffdio_api = UffdioApi {
                api: UFFD_API,
                features: 1u64 << 63,
                ioctls: 0,
            };
            assert_eq!(
                unsafe {
                    crate::fs::ioctl::sys_ioctl(
                        uffd as i32,
                        UFFDIO_API,
                        &mut unsupported_uffdio_api as *mut UffdioApi as u64,
                    )
                },
                -(EINVAL as i64)
            );
            let mut uffdio_api = UffdioApi {
                api: UFFD_API,
                features: 0,
                ioctls: 0,
            };
            assert_eq!(
                unsafe {
                    crate::fs::ioctl::sys_ioctl(
                        uffd as i32,
                        UFFDIO_API,
                        &mut uffdio_api as *mut UffdioApi as u64,
                    )
                },
                0
            );
            assert_eq!(uffdio_api.api, UFFD_API);
            assert_ne!(uffdio_api.ioctls & (1u64 << 63), 0);
            assert_eq!(
                unsafe {
                    crate::fs::ioctl::sys_ioctl(
                        uffd as i32,
                        UFFDIO_API,
                        &mut uffdio_api as *mut UffdioApi as u64,
                    )
                },
                -(EINVAL as i64)
            );
            let mut uffdio_register = UffdioRegister {
                range: UffdioRange {
                    start: 0x4000,
                    len: 0x1000,
                },
                mode: UFFDIO_REGISTER_MODE_MISSING,
                ioctls: 0,
            };
            assert_eq!(
                unsafe {
                    crate::fs::ioctl::sys_ioctl(
                        uffd as i32,
                        UFFDIO_REGISTER,
                        &mut uffdio_register as *mut UffdioRegister as u64,
                    )
                },
                0
            );
            assert!(crate::mm::shmem::userfaultfd_range_registered(
                0x4000, 0x1000
            ));
            assert_eq!(uffdio_register.ioctls & (1u64 << 4), 0);
            let mut uffdio_zeropage = UffdioZeropage {
                range: UffdioRange {
                    start: 0x4000,
                    len: 0x1000,
                },
                mode: 0,
                zeropage: 0,
            };
            assert_eq!(
                unsafe {
                    crate::fs::ioctl::sys_ioctl(
                        uffd as i32,
                        UFFDIO_ZEROPAGE,
                        &mut uffdio_zeropage as *mut UffdioZeropage as u64,
                    )
                },
                -(EINVAL as i64)
            );
            assert_eq!(uffdio_zeropage.zeropage, -(EINVAL as i64));
            crate::mm::shmem::userfaultfd_unregister_range(0x4000, 0x1000);
            assert_eq!(sys_userfaultfd(0x8000_0000u32 as i32), -(EINVAL as i64));
            let mut stx = LinuxStatx::default();
            assert_eq!(sys_file_getattr(-1, 0, 0, &mut stx), -(EBADF as i64));
            assert_eq!(
                sys_file_getattr(-1, 0, 0, core::ptr::null_mut()),
                -(EFAULT as i64)
            );
            assert_eq!(
                sys_file_setattr(-1, 0, core::ptr::null(), 0),
                -(EINVAL as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}

fn split_last(path: &str) -> (&str, &str) {
    let trimmed = path.trim_end_matches('/');
    if let Some(idx) = trimmed.rfind('/') {
        (&trimmed[..idx + 1], &trimmed[idx + 1..])
    } else {
        ("", trimmed)
    }
}
