//! linux-parity: complete
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs
//! `openat`/`openat2` (M39) â€” kernel-side path open.
//!
//! Mirrors `vendor/linux/fs/open.c::do_filp_open` and the `openat2(2)`
//! syscall path.

extern crate alloc;

use alloc::{string::String, vec::Vec};

use crate::include::uapi::errno::{EACCES, EEXIST, EINVAL, EISDIR, ELOOP, ENOENT, ENXIO};
use crate::include::uapi::fcntl::{
    O_ACCMODE, O_CLOEXEC, O_CREAT, O_DIRECTORY, O_EXCL, O_LARGEFILE, O_NOFOLLOW, O_PATH, O_RDWR,
    O_TMPFILE, O_TRUNC, O_WRONLY,
};
use crate::include::uapi::openat2::OpenHow;

use crate::security;

use super::dcache::d_alloc_child;
use super::file::{alloc_file, note_file_access_for_integrity, path_hint, set_path_hint};
use super::namei::{LookupCtx, path_lookupat, validate_open_how};
use super::ops::PATH_FILE_OPS;
use super::permission::check_file_write_permission;
use super::types::{DentryRef, FileRef, InodeKind};

const O_PATH_FLAGS: u32 = O_DIRECTORY | O_NOFOLLOW | O_PATH | O_CLOEXEC;
const S_IALLUGO: u32 = 0o7777;

#[inline]
fn file_status_flags_for_open(flags: u32) -> u32 {
    flags & !O_CLOEXEC
}

#[inline]
fn will_create(flags: u32) -> bool {
    flags & (O_CREAT | O_TMPFILE) != 0
}

#[inline]
fn open_requests_write(flags: u32) -> bool {
    matches!(flags & O_ACCMODE, O_WRONLY | O_RDWR) || flags & O_TRUNC != 0
}

fn check_existing_open_permissions(
    dentry: &DentryRef,
    inode: &super::types::InodeRef,
    flags: u32,
) -> Result<(), i32> {
    if flags & O_PATH != 0 {
        return Ok(());
    }

    if open_requests_write(flags) {
        check_file_write_permission(dentry, inode)?;
    }

    Ok(())
}

// Mirrors vendor/linux/fs/open.c::build_open_how for legacy open/openat.
fn build_open_how_for_openat(flags: i32, mode: u32) -> OpenHow {
    let mut flags = flags as u32 | O_LARGEFILE;
    if flags & O_PATH != 0 {
        flags &= O_PATH_FLAGS;
    }
    let mode = if will_create(flags) {
        mode & S_IALLUGO
    } else {
        0
    };
    OpenHow {
        flags: flags as u64,
        mode: mode as u64,
        resolve: 0,
    }
}

pub struct OpenResult {
    pub file: FileRef,
    pub cloexec: bool,
}

/// `openat(2)` syscall entry point.
///
/// During early rootfs bring-up we only support `dirfd == AT_FDCWD` and treat the
/// rootfs mount's root dentry as both the root and current working directory.
/// The full `fs_struct`-based root/pwd semantics land later in M39 closure.
///
/// Source of truth: `vendor/linux/fs/open.c`, `vendor/linux/fs/namei.c`,
/// `vendor/linux/arch/x86/entry/syscalls/syscall_64.tbl` (nr 257).
pub unsafe fn sys_openat(dirfd: i32, filename: *const u8, flags: i32, mode: u32) -> i64 {
    use crate::arch::x86::kernel::uaccess;
    use crate::fs::mount;
    use crate::include::uapi::errno::{EBADF, EINVAL};
    use crate::include::uapi::fcntl::AT_FDCWD;
    use crate::kernel::{files, sched};

    if filename.is_null() {
        return -(crate::include::uapi::errno::EFAULT as i64);
    }

    const PATH_MAX: usize = 4096;
    let mut buf = alloc::vec![0u8; PATH_MAX];
    let n = unsafe { uaccess::strncpy_from_user(buf.as_mut_ptr(), filename, buf.len()) };
    if n < 0 {
        return n as i64;
    }
    let n = n as usize;
    let raw_path = trim_copied_user_path(&buf[..n]);
    let path = match core::str::from_utf8(raw_path) {
        Ok(s) => s,
        Err(_) => return -(EINVAL as i64),
    };

    let effective_path;
    let path = if path.starts_with('/') || dirfd == AT_FDCWD {
        effective_path = super::fs_struct::absolute_from_cwd(path);
        effective_path.as_str()
    } else {
        path
    };

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let Some(ft) = (unsafe { files::get_task_files(task) }) else {
        return -(EBADF as i64);
    };

    let Some(root_mnt) = mount::rootfs() else {
        return -(EINVAL as i64);
    };
    let root = root_mnt.root.clone();
    let start = if dirfd == AT_FDCWD {
        root.clone()
    } else {
        match ft.get(dirfd) {
            Ok(file) => mount::mounted_root_for_dentry(&file.dentry)
                .map(|mnt| mnt.root.clone())
                .unwrap_or_else(|| file.dentry.clone()),
            Err(errno) => return -(errno as i64),
        }
    };
    let how = build_open_how_for_openat(flags, mode);

    let opened_path = path_hint_for_open(&ft, dirfd, path);
    let ret = match do_openat2_with_hint(root, start, path, &how, opened_path.as_deref()) {
        Ok(r) => match ft.install(r.file, r.cloexec) {
            Ok(fd) => {
                let integrity_path = opened_path.clone();
                if let Ok(file) = ft.get(fd) {
                    if let Some(path) = opened_path {
                        set_path_hint(&file, path);
                    }
                    note_file_access_for_integrity(integrity_path.as_deref(), &file);
                }
                fd as i64
            }
            Err(errno) => -(errno as i64),
        },
        Err(errno) => -(errno as i64),
    };
    trace_run_openat(dirfd, path, flags, ret);
    ret
}

fn trim_copied_user_path(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|b| *b == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}

#[cfg(not(test))]
fn trace_run_openat(dirfd: i32, path: &str, flags: i32, ret: i64) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    if !(ret < 0
        || path.contains("dev")
        || path.contains("proc")
        || path.contains("sys")
        || path.contains("run")
        || path.contains("systemd")
        || path.contains("journal")
        || path.contains("netif"))
    {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-run-openat pid={} dirfd={} flags={:#x} path={} ret={}",
        pid,
        dirfd,
        flags,
        path,
        ret
    );
}

#[cfg(test)]
fn trace_run_openat(_dirfd: i32, _path: &str, _flags: i32, _ret: i64) {}

fn path_hint_for_open(
    ft: &crate::fs::fdtable::FilesStruct,
    dirfd: i32,
    path: &str,
) -> Option<String> {
    use crate::include::uapi::fcntl::AT_FDCWD;

    if path.is_empty() {
        return None;
    }
    if path.starts_with('/') {
        return Some(String::from(path));
    }
    if dirfd == AT_FDCWD {
        return Some(join_path("/", path));
    }
    if dirfd < 0 {
        return None;
    }
    let dir = ft.get(dirfd).ok()?;
    let base = path_hint(&dir).or_else(|| super::mount::path_for_dentry(&dir.dentry))?;
    Some(join_path(&base, path))
}

/// Open `path` starting at `dir` (resolved against `root`).  M39 in-kernel
/// API; userspace `sys_openat2` will marshal `OpenHow` from user memory once
/// usermode lands.
pub fn do_openat2(
    root: DentryRef,
    dir: DentryRef,
    path: &str,
    how: &OpenHow,
) -> Result<OpenResult, i32> {
    do_openat2_with_hint(root, dir, path, how, None)
}

pub(crate) fn do_openat2_with_hint(
    root: DentryRef,
    dir: DentryRef,
    path: &str,
    how: &OpenHow,
    hinted_path: Option<&str>,
) -> Result<OpenResult, i32> {
    validate_open_how(how)?;
    if absent_lsm_sysfs_probe_path(path) {
        return Err(ENOENT);
    }
    let flags = how.flags as u32;
    let mode = how.mode as u32;
    if flags & O_PATH != 0 && flags & !O_PATH_FLAGS != 0 {
        return Err(EINVAL);
    }
    if flags & O_CREAT == 0 {
        if let Some(result) = proc_special_open_result(path, flags, mode) {
            return result;
        }
        if let Some(hinted_path) = hinted_path
            && hinted_path != path
            && let Some(result) = proc_special_open_result(hinted_path, flags, mode)
        {
            return result;
        }
    }

    let ctx = LookupCtx::new(root, dir.clone(), how.resolve);

    if let Some(dentry) = resolve_existing_open_path(&ctx, path, flags & O_NOFOLLOW == 0)? {
        if flags & O_CREAT != 0 && flags & O_EXCL != 0 {
            return Err(EEXIST);
        }

        let logical_path =
            super::mount::path_for_dentry(&dentry).unwrap_or_else(|| build_dentry_path(&dentry));
        let open_err = security::security_path_open(logical_path.as_bytes(), flags as i32);
        if open_err != 0 {
            return Err(if open_err < 0 { -open_err } else { open_err });
        }

        let inode = dentry.inode().ok_or(ENOENT)?;
        if flags & O_NOFOLLOW != 0 && flags & O_PATH == 0 && inode.kind == InodeKind::Symlink {
            return Err(ELOOP);
        }
        if flags & O_DIRECTORY != 0 && inode.kind != InodeKind::Directory {
            return Err(crate::include::uapi::errno::ENOTDIR);
        }
        if flags & O_PATH == 0 && inode.kind == InodeKind::Socket {
            return Err(ENXIO);
        }
        if inode.kind == InodeKind::Directory
            && (flags & O_ACCMODE == O_WRONLY || flags & O_ACCMODE == O_RDWR)
        {
            return Err(EISDIR);
        }
        check_existing_open_permissions(&dentry, &inode, flags)?;

        if flags & O_TRUNC != 0 {
            if let crate::fs::types::InodePrivate::RamBytes(m) = &inode.private {
                m.lock().clear();
                inode.size.store(0, core::sync::atomic::Ordering::Release);
            }
        }
        let fops = if flags & O_PATH != 0 {
            &PATH_FILE_OPS
        } else {
            inode.fops
        };
        let f = alloc_file(dentry, file_status_flags_for_open(flags), mode, fops);
        return Ok(OpenResult {
            file: f,
            cloexec: flags & O_CLOEXEC != 0,
        });
    }

    // Split into parent path + final component for O_CREAT handling.
    let (parent_path, last) = split_last(path);

    let parent_dentry = resolve_open_parent(&ctx.root, &dir, path, parent_path)?;
    let parent_inode = parent_dentry.inode().ok_or(ENOENT)?;
    if parent_inode.kind != InodeKind::Directory {
        return Err(crate::include::uapi::errno::ENOTDIR);
    }

    let logical_path = build_open_path(&parent_dentry, last);
    let open_err = security::security_path_open(logical_path.as_bytes(), flags as i32);
    if open_err != 0 {
        return Err(if open_err < 0 { -open_err } else { open_err });
    }

    // Resolve / create the final component.
    let dentry: DentryRef = if last.is_empty() || last == "." {
        // path == "/" or empty â€” refers to parent_dentry itself.
        parent_dentry
    } else if last == ".." {
        parent_dentry
            .parent
            .lock()
            .clone()
            .unwrap_or_else(|| parent_dentry.clone())
    } else {
        match super::dcache::d_lookup(&parent_dentry, last) {
            Some(d) => {
                if d.inode().is_some() {
                    if flags & O_CREAT != 0 && flags & O_EXCL != 0 {
                        return Err(EEXIST);
                    }
                    d
                } else {
                    if let Some(lookup) = parent_inode.ops.lookup {
                        match lookup(&parent_inode, last) {
                            Ok(inode) => {
                                if flags & O_CREAT != 0 && flags & O_EXCL != 0 {
                                    return Err(EEXIST);
                                }
                                d.instantiate(inode);
                                d
                            }
                            Err(ENOENT) if flags & O_CREAT != 0 => {
                                let create = parent_inode.ops.create.ok_or(EINVAL)?;
                                let inode = create(&parent_inode, last, mode)?;
                                d.instantiate(inode);
                                super::inotify::notify_create(&parent_dentry, last, false);
                                d
                            }
                            Err(ENOENT) => return Err(ENOENT),
                            Err(errno) => return Err(errno),
                        }
                    } else if flags & O_CREAT != 0 {
                        let create = parent_inode.ops.create.ok_or(EINVAL)?;
                        let inode = create(&parent_inode, last, mode)?;
                        d.instantiate(inode);
                        super::inotify::notify_create(&parent_dentry, last, false);
                        d
                    } else {
                        return Err(ENOENT);
                    }
                }
            }
            None => {
                // Try inode_ops.lookup
                if let Some(lookup) = parent_inode.ops.lookup {
                    match lookup(&parent_inode, last) {
                        Ok(child) => {
                            if flags & O_CREAT != 0 && flags & O_EXCL != 0 {
                                return Err(EEXIST);
                            }
                            let d = d_alloc_child(&parent_dentry, last);
                            d.instantiate(child);
                            d
                        }
                        Err(ENOENT) if flags & O_CREAT != 0 => {
                            let create = parent_inode.ops.create.ok_or(EINVAL)?;
                            let inode = create(&parent_inode, last, mode)?;
                            let d = d_alloc_child(&parent_dentry, last);
                            d.instantiate(inode);
                            super::inotify::notify_create(&parent_dentry, last, false);
                            d
                        }
                        Err(ENOENT) => {
                            super::dcache::d_cache_negative(&parent_dentry, last);
                            return Err(ENOENT);
                        }
                        Err(errno) => return Err(errno),
                    }
                } else if flags & O_CREAT != 0 {
                    let create = parent_inode.ops.create.ok_or(EINVAL)?;
                    let inode = create(&parent_inode, last, mode)?;
                    let d = d_alloc_child(&parent_dentry, last);
                    d.instantiate(inode);
                    super::inotify::notify_create(&parent_dentry, last, false);
                    d
                } else {
                    super::dcache::d_cache_negative(&parent_dentry, last);
                    return Err(ENOENT);
                }
            }
        }
    };

    let inode = dentry.inode().ok_or(ENOENT)?;

    // O_DIRECTORY enforcement.
    if flags & O_DIRECTORY != 0 && inode.kind != InodeKind::Directory {
        return Err(crate::include::uapi::errno::ENOTDIR);
    }
    if flags & O_PATH == 0 && inode.kind == InodeKind::Socket {
        return Err(ENXIO);
    }
    // Writes to directories are forbidden.
    if inode.kind == InodeKind::Directory
        && (flags & O_ACCMODE == O_WRONLY || flags & O_ACCMODE == O_RDWR)
    {
        return Err(EISDIR);
    }

    check_existing_open_permissions(&dentry, &inode, flags)?;

    if flags & O_TRUNC != 0 {
        if let crate::fs::types::InodePrivate::RamBytes(m) = &inode.private {
            m.lock().clear();
            inode.size.store(0, core::sync::atomic::Ordering::Release);
        }
    }

    let fops = if flags & O_PATH != 0 {
        &PATH_FILE_OPS
    } else {
        inode.fops
    };
    let f = alloc_file(dentry, file_status_flags_for_open(flags), mode, fops);
    Ok(OpenResult {
        file: f,
        cloexec: flags & O_CLOEXEC != 0,
    })
}

fn read_only_proc_open_result(result: Result<FileRef, i32>, flags: u32) -> Result<OpenResult, i32> {
    if flags & O_DIRECTORY != 0 {
        return Err(crate::include::uapi::errno::ENOTDIR);
    }
    if flags & O_PATH == 0 && flags & O_ACCMODE != 0 {
        return Err(EACCES);
    }
    result.map(|file| OpenResult {
        file,
        cloexec: flags & O_CLOEXEC != 0,
    })
}

fn proc_file_open_result(result: Result<FileRef, i32>, flags: u32) -> Result<OpenResult, i32> {
    if flags & O_DIRECTORY != 0 {
        return Err(crate::include::uapi::errno::ENOTDIR);
    }
    result.map(|file| OpenResult {
        file,
        cloexec: flags & O_CLOEXEC != 0,
    })
}

fn proc_special_open_result(path: &str, flags: u32, mode: u32) -> Option<Result<OpenResult, i32>> {
    if let Some(result) = crate::fs::proc::fd::current_fdinfo_file_from_proc_path(path, flags, mode)
    {
        return Some(read_only_proc_open_result(result, flags));
    }
    if let Some(result) = crate::fs::proc::base::process_stat_file_from_proc_path(path, flags, mode)
    {
        return Some(read_only_proc_open_result(result, flags));
    }
    if let Some(result) =
        crate::fs::proc::base::process_cgroup_file_from_proc_path(path, flags, mode)
    {
        return Some(read_only_proc_open_result(result, flags));
    }
    if let Some(result) =
        crate::fs::proc::task_mmu::process_task_mmu_file_from_proc_path(path, flags, mode)
    {
        return Some(proc_file_open_result(result, flags));
    }
    if let Some(result) = crate::fs::proc::page::kpageflags_file_from_proc_path(path, flags, mode) {
        return Some(proc_file_open_result(result, flags));
    }
    None
}

fn absent_lsm_sysfs_probe_path(path: &str) -> bool {
    matches!(
        path,
        "/sys/fs/smackfs" | "/sys/fs/smackfs/" | "/sys/fs/selinux" | "/sys/fs/selinux/"
    ) || path.starts_with("/sys/fs/smackfs/")
        || path.starts_with("/sys/fs/selinux/")
}

fn split_last(path: &str) -> (&str, &str) {
    let trimmed = path.trim_end_matches('/');
    if let Some(idx) = trimmed.rfind('/') {
        (&trimmed[..idx + 1], &trimmed[idx + 1..])
    } else {
        ("", trimmed)
    }
}

fn build_open_path(parent: &DentryRef, last: &str) -> String {
    if last.is_empty() {
        return build_dentry_path(parent);
    }

    let mut path = build_dentry_path(parent);
    if path != "/" {
        path.push('/');
    }
    path.push_str(last);
    path
}

fn resolve_existing_open_path(
    ctx: &LookupCtx,
    path: &str,
    follow_final: bool,
) -> Result<Option<DentryRef>, i32> {
    if ctx.resolve != 0 {
        return match super::mount::resolve_path_at(ctx, path, follow_final) {
            Ok((_, dentry)) => Ok(Some(dentry)),
            Err(ENOENT) => Ok(None),
            Err(errno) => Err(errno),
        };
    }

    if !path.starts_with('/') {
        if let Some(base) = super::mount::path_for_dentry(&ctx.start) {
            let full_path = join_path(&base, path);
            return match if follow_final {
                super::mount::resolve_path_follow(&full_path)
            } else {
                super::mount::resolve_path_nofollow(&full_path)
            } {
                Ok((_, dentry)) => Ok(Some(dentry)),
                Err(ENOENT) => Ok(None),
                Err(errno) => Err(errno),
            };
        }

        return match path_lookupat(ctx, path) {
            Ok(dentry) => Ok(Some(dentry)),
            Err(ENOENT) => Ok(None),
            Err(errno) => Err(errno),
        };
    }

    match if follow_final {
        super::mount::resolve_path_follow(path)
    } else {
        super::mount::resolve_path_nofollow(path)
    } {
        Ok((_, dentry)) => Ok(Some(dentry)),
        Err(ENOENT) => Ok(None),
        Err(errno) => Err(errno),
    }
}

fn resolve_open_parent(
    root: &DentryRef,
    dir: &DentryRef,
    path: &str,
    parent_path: &str,
) -> Result<DentryRef, i32> {
    if path.starts_with('/') {
        return super::mount::resolve_path_follow(parent_path).map_or_else(
            |_| path_lookupat(&LookupCtx::new(root.clone(), dir.clone(), 0), parent_path),
            |(_, dentry)| Ok(dentry),
        );
    }

    if let Some(base) = super::mount::path_for_dentry(dir) {
        let full_parent = join_path(&base, parent_path);
        return super::mount::resolve_path_follow(&full_parent).map_or_else(
            |_| path_lookupat(&LookupCtx::new(root.clone(), dir.clone(), 0), parent_path),
            |(_, dentry)| Ok(dentry),
        );
    }

    path_lookupat(&LookupCtx::new(root.clone(), dir.clone(), 0), parent_path)
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

fn build_dentry_path(dentry: &DentryRef) -> String {
    let mut components = Vec::new();
    let mut cur = Some(dentry.clone());

    while let Some(node) = cur {
        let parent = node.parent.lock().clone();
        let is_root = parent.is_none();
        if !is_root && node.name != "/" && !node.name.is_empty() {
            components.push(node.name.clone());
        }
        cur = parent;
    }

    if components.is_empty() {
        return String::from("/");
    }

    let mut path = String::new();
    for component in components.iter().rev() {
        path.push('/');
        path.push_str(component);
    }
    path
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::{boxed::Box, string::String, vec::Vec};
    use std::sync::Mutex;

    use crate::fs;
    use crate::fs::dcache::{d_alloc_child, d_lookup};
    use crate::fs::file::file_path;
    use crate::fs::mount::{self, Mount, set_rootfs};
    use crate::fs::super_block::mount_fs;
    use crate::include::uapi::errno::{EACCES, ENOENT, EROFS};
    use crate::include::uapi::mount::MS_RDONLY;
    use crate::kernel::capability::KernelCapT;
    use crate::kernel::cred::{Cred, GroupInfo, INIT_CRED, KGid, KUid};
    use crate::kernel::{sched, task::TaskStruct};
    use crate::security::hooks::{LsmHooks, NOOP_HOOKS};
    use crate::security::lsm_list::{TEST_LSM_LOCK, reset_for_test};
    use crate::security::register_lsm;

    static OPEN_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

    #[test]
    fn legacy_openat_build_open_how_masks_opath_extras() {
        let how = build_open_how_for_openat(
            (O_PATH
                | O_CLOEXEC
                | O_NOFOLLOW
                | O_TRUNC
                | O_LARGEFILE
                | crate::include::uapi::fcntl::O_NOCTTY) as i32,
            0o17777,
        );

        assert_eq!(how.flags as u32, O_PATH | O_CLOEXEC | O_NOFOLLOW);
        assert_eq!(how.mode, 0);
        assert_eq!(how.resolve, 0);
    }

    #[test]
    fn legacy_openat_keeps_create_mode_inside_linux_umask() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        let how = build_open_how_for_openat((O_CREAT | O_WRONLY) as i32, 0o17777);

        assert_eq!(how.flags as u32, O_CREAT | O_WRONLY | O_LARGEFILE);
        assert_eq!(how.mode as u32, S_IALLUGO);
        assert_eq!(how.resolve, 0);
    }

    #[test]
    fn copied_user_path_stops_at_first_nul() {
        assert_eq!(trim_copied_user_path(b"/usr/bin\0ignored"), b"/usr/bin");
        assert_eq!(trim_copied_user_path(b"/usr/bin"), b"/usr/bin");
    }

    fn reset_mount_state() {
        *mount::MOUNTS.root.lock() = None;
        mount::MOUNTS.by_path.lock().clear();
    }

    fn ensure_dir(root: &DentryRef, path: &str) -> DentryRef {
        let mut cur = root.clone();
        for component in path.trim_matches('/').split('/').filter(|c| !c.is_empty()) {
            if let Some(next) = d_lookup(&cur, component) {
                cur = next;
                continue;
            }

            let parent_inode = cur.inode().expect("parent inode");
            let mkdir = parent_inode.ops.mkdir.expect("mkdir op");
            let inode = mkdir(&parent_inode, component, 0o755).expect("mkdir");
            let child = d_alloc_child(&cur, component);
            child.instantiate(inode);
            cur = child;
        }
        cur
    }

    fn setup_rootfs() -> DentryRef {
        fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("mount ramfs");
        let root = sb.root().expect("root dentry");
        let root_mount = Mount::alloc(sb, root.clone(), 0);
        set_rootfs(root_mount);

        ensure_dir(&root, "/etc");
        ensure_dir(&root, "/tmp");
        root
    }

    fn create_file(parent: &DentryRef, name: &str) -> DentryRef {
        let parent_inode = parent.inode().expect("parent inode");
        let create = parent_inode.ops.create.expect("create op");
        let child_inode = create(&parent_inode, name, 0o644).expect("create file");
        let child = d_alloc_child(parent, name);
        child.instantiate(child_inode);
        child
    }

    fn install_unprivileged_current<'a>(
        current: &'a mut TaskStruct,
        cred: &'a Cred,
    ) -> *mut TaskStruct {
        let previous = unsafe { sched::get_current() };
        current.pid = 4242;
        current.tgid = 4242;
        current.cred = cred as *const Cred;
        current.m27.real_cred = cred as *const Cred;
        unsafe { sched::set_current(current as *mut TaskStruct) };
        previous
    }

    fn unprivileged_cred() -> Cred {
        Cred {
            usage: core::sync::atomic::AtomicUsize::new(1),
            uid: KUid(1000),
            gid: KGid(1000),
            suid: KUid(1000),
            sgid: KGid(1000),
            euid: KUid(1000),
            egid: KGid(1000),
            fsuid: KUid(1000),
            fsgid: KGid(1000),
            cap_inheritable: KernelCapT::empty(),
            cap_permitted: KernelCapT::empty(),
            cap_effective: KernelCapT::empty(),
            cap_bset: KernelCapT::empty(),
            cap_ambient: KernelCapT::empty(),
            securebits: 0,
            group_info: GroupInfo::default(),
            user_ns: core::ptr::null(),
        }
    }

    #[test]
    fn do_openat2_denies_unprivileged_write_to_read_only_inode() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();
        let root = setup_rootfs();
        let etc = d_lookup(&root, "etc").expect("/etc");
        let etc_inode = etc.inode().expect("/etc inode");
        let create = etc_inode.ops.create.expect("create");
        let inode = create(&etc_inode, "shadow", 0o400).expect("shadow");
        inode.uid.store(0, core::sync::atomic::Ordering::Release);
        inode.gid.store(0, core::sync::atomic::Ordering::Release);
        let dentry = d_alloc_child(&etc, "shadow");
        dentry.instantiate(inode);

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let cred = Box::new(unprivileged_cred());
        let previous = install_unprivileged_current(&mut current, &cred);
        let how = OpenHow {
            flags: O_WRONLY as u64,
            ..OpenHow::default()
        };

        let result = do_openat2(root.clone(), root, "/etc/shadow", &how);
        unsafe { sched::set_current(previous) };
        assert!(matches!(result, Err(EACCES)));
        current.cred = &raw const INIT_CRED;
    }

    #[test]
    fn do_openat2_denies_write_on_read_only_mount() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();
        let root = setup_rootfs();
        let tmp = d_lookup(&root, "tmp").expect("/tmp");
        let tmp_inode = tmp.inode().expect("/tmp inode");
        let create = tmp_inode.ops.create.expect("create");
        let inode = create(&tmp_inode, "writable", 0o666).expect("writable");
        let dentry = d_alloc_child(&tmp, "writable");
        dentry.instantiate(inode);
        mount::rootfs()
            .expect("rootfs")
            .flags
            .store(MS_RDONLY as u32, core::sync::atomic::Ordering::Release);

        let how = OpenHow {
            flags: O_WRONLY as u64,
            ..OpenHow::default()
        };
        let result = do_openat2(root.clone(), root, "/tmp/writable", &how);
        assert!(matches!(result, Err(EROFS)));
    }

    #[test]
    fn openat_caches_negative_dentry_and_reuses_it_for_create() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();
        let root = setup_rootfs();
        let how = build_open_how_for_openat(0, 0);

        assert!(matches!(
            do_openat2(root.clone(), root.clone(), "/etc/missing-lib.so", &how),
            Err(ENOENT)
        ));

        let etc = d_lookup(&root, "etc").expect("/etc");
        let negative = d_lookup(&etc, "missing-lib.so").expect("negative dentry");
        assert!(negative.is_negative());

        let create = build_open_how_for_openat((O_CREAT | O_WRONLY) as i32, 0o644);
        let opened = do_openat2(root.clone(), root, "/etc/missing-lib.so", &create)
            .expect("create through cached negative dentry");

        assert!(!negative.is_negative());
        assert!(opened.file.dentry.inode().is_some());
        assert!(
            d_lookup(&etc, "missing-lib.so")
                .and_then(|dentry| dentry.inode())
                .is_some()
        );
    }

    fn log_open_path(path: &[u8], _flags: i32) -> i32 {
        OPEN_LOG
            .lock()
            .unwrap()
            .push(String::from_utf8_lossy(path).into_owned());
        0
    }

    fn deny_etc_paths(path: &[u8], _flags: i32) -> i32 {
        let text = String::from_utf8_lossy(path).into_owned();
        OPEN_LOG.lock().unwrap().push(text.clone());
        if text == "/etc" || text.starts_with("/etc/") {
            -EACCES
        } else {
            0
        }
    }

    #[test]
    fn do_openat2_accepts_legacy_normalized_opath_flags() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let how = build_open_how_for_openat(
            (O_PATH | O_CLOEXEC | crate::include::uapi::fcntl::O_NOCTTY) as i32,
            0,
        );
        let opened = do_openat2(root.clone(), root, "/tmp", &how).expect("open /tmp O_PATH");

        assert_eq!(file_path(&opened.file), "/tmp");
        assert!(opened.cloexec);
        assert_eq!(
            opened
                .file
                .flags
                .load(core::sync::atomic::Ordering::Acquire)
                & crate::include::uapi::fcntl::O_NOCTTY,
            0
        );
    }

    #[test]
    fn do_openat2_runs_path_open_hook_for_existing_file() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();
        register_lsm(LsmHooks {
            name: "openat_existing_path_deny",
            path_open: Some(deny_etc_paths),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        let root = setup_rootfs();
        let etc = d_lookup(&root, "etc").expect("/etc");
        create_file(&etc, "secret");
        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            ..OpenHow::default()
        };

        let result = do_openat2(root.clone(), root, "/etc/secret", &how);
        assert!(matches!(result, Err(EACCES)));
        assert_eq!(&*OPEN_LOG.lock().unwrap(), &[String::from("/etc/secret")]);
    }

    #[test]
    fn do_openat2_enforces_resolve_beneath_for_existing_file() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let tmp = d_lookup(&root, "tmp").expect("/tmp");
        let etc = d_lookup(&root, "etc").expect("/etc");
        create_file(&etc, "beneath_escape");
        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            resolve: crate::include::uapi::openat2::RESOLVE_BENEATH,
            ..OpenHow::default()
        };

        let result = do_openat2(root.clone(), tmp, "../etc/beneath_escape", &how);
        assert!(matches!(result, Err(EINVAL)));
    }

    #[test]
    fn do_openat2_resolve_beneath_rejects_absolute_symlink_target() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let tmp = d_lookup(&root, "tmp").expect("/tmp");
        let tmp_inode = tmp.inode().expect("/tmp inode");
        let etc = d_lookup(&root, "etc").expect("/etc");
        create_file(&etc, "absolute_symlink_escape");
        crate::fs::ramfs::ramfs_symlink(
            &tmp_inode,
            "attacker_link",
            "/etc/absolute_symlink_escape",
            0o777,
        )
        .expect("symlink");
        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            resolve: crate::include::uapi::openat2::RESOLVE_BENEATH,
            ..OpenHow::default()
        };

        let result = do_openat2(root.clone(), tmp, "attacker_link", &how);
        assert!(matches!(result, Err(EINVAL)));
    }

    #[test]
    fn do_openat2_enforces_no_symlinks_for_existing_file() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let etc = d_lookup(&root, "etc").expect("/etc");
        let etc_inode = etc.inode().expect("/etc inode");
        create_file(&etc, "target2");
        crate::fs::ramfs::ramfs_symlink(&etc_inode, "link2", "target2", 0o777).expect("symlink");
        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            resolve: crate::include::uapi::openat2::RESOLVE_NO_SYMLINKS,
            ..OpenHow::default()
        };

        let result = do_openat2(root.clone(), root, "/etc/link2", &how);
        assert!(matches!(result, Err(ELOOP)));
    }

    #[test]
    fn do_openat2_resolve_existing_path_descends_mounts() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let tmp = d_lookup(&root, "tmp").expect("/tmp");
        let hidden = create_file(&tmp, "secret");

        mount::do_mount("tmpfs", "tmpfs", "/tmp", 0, "").expect("mount /tmp");
        let (_, mounted_tmp) = mount::resolve_path_follow("/tmp").expect("mounted /tmp");
        let visible = create_file(&mounted_tmp, "secret");

        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            resolve: crate::include::uapi::openat2::RESOLVE_BENEATH,
            ..OpenHow::default()
        };
        let opened =
            do_openat2(root.clone(), root, "tmp/secret", &how).expect("open mounted /tmp/secret");

        assert!(alloc::sync::Arc::ptr_eq(&opened.file.dentry, &visible));
        assert!(!alloc::sync::Arc::ptr_eq(&opened.file.dentry, &hidden));
    }

    #[test]
    fn do_openat2_runs_path_open_hook_before_enoent() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();
        register_lsm(LsmHooks {
            name: "openat_path_deny",
            path_open: Some(deny_etc_paths),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        let root = setup_rootfs();
        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            ..OpenHow::default()
        };

        let result = do_openat2(root.clone(), root, "/etc/shadow", &how);
        assert!(matches!(result, Err(EACCES)));
        assert_eq!(&*OPEN_LOG.lock().unwrap(), &[String::from("/etc/shadow")]);
    }

    #[test]
    fn do_openat2_passes_canonical_path_to_security_hook() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();
        register_lsm(LsmHooks {
            name: "openat_path_log",
            path_open: Some(log_open_path),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        let root = setup_rootfs();
        let tmp = d_lookup(&root, "tmp").expect("/tmp");
        let how = OpenHow {
            flags: O_CREAT as u64 | O_RDWR as u64,
            mode: 0o644,
            ..OpenHow::default()
        };

        let opened = do_openat2(root.clone(), tmp, "../etc/hosts", &how).expect("open");
        assert_eq!(&*OPEN_LOG.lock().unwrap(), &[String::from("/etc/hosts")]);

        let etc = d_lookup(&root, "etc").expect("/etc");
        assert!(d_lookup(&etc, "hosts").is_some());
        drop(opened);
    }

    #[test]
    fn do_openat2_creates_inside_mounted_parent() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        ensure_dir(&root, "/run");
        mount::do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("mount /run");
        let (_, run_root) = mount::resolve_path_follow("/run").expect("/run");
        let systemd = ensure_dir(&run_root, "systemd");
        let propagate = ensure_dir(&systemd, "propagate");
        ensure_dir(&propagate, ".os-release-stage");

        let how = OpenHow {
            flags: (O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC) as u64,
            mode: 0o600,
            ..OpenHow::default()
        };
        do_openat2(
            root.clone(),
            root,
            "/run/systemd/propagate/.os-release-stage/.#os-release",
            &how,
        )
        .expect("create temp file under mounted /run");

        assert!(
            mount::resolve_path_follow("/run/systemd/propagate/.os-release-stage/.#os-release")
                .is_ok()
        );
    }

    #[test]
    fn do_openat2_o_creat_opens_existing_file_on_cgroupfs() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let sys = ensure_dir(&root, "/sys");
        let fs_dir = ensure_dir(&sys, "fs");
        ensure_dir(&fs_dir, "cgroup");
        mount::do_mount("cgroup2", "cgroup2", "/sys/fs/cgroup", 0, "").expect("mount cgroup2");
        let (_, cg_root) = mount::resolve_path_follow("/sys/fs/cgroup").expect("cgroup root");
        let cg_inode = cg_root.inode().expect("cgroup inode");
        let mkdir = cg_inode.ops.mkdir.expect("cgroup mkdir");
        let system = mkdir(&cg_inode, "system.slice", 0o755).expect("system.slice");
        let system_dentry = d_alloc_child(&cg_root, "system.slice");
        system_dentry.instantiate(system.clone());
        let service = mkdir(&system, "systemd-networkd.service", 0o755).expect("service");
        let service_dentry = d_alloc_child(&system_dentry, "systemd-networkd.service");
        service_dentry.instantiate(service);

        let how = OpenHow {
            flags: (O_CREAT | O_TRUNC | O_WRONLY | O_CLOEXEC) as u64,
            mode: 0o644,
            ..OpenHow::default()
        };
        do_openat2(
            root.clone(),
            root,
            "/sys/fs/cgroup/system.slice/systemd-networkd.service/cgroup.subtree_control",
            &how,
        )
        .expect("open existing cgroup file with O_CREAT");
    }

    #[test]
    fn do_openat2_revalidates_stale_negative_cgroup_events() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let sys = ensure_dir(&root, "/sys");
        let fs_dir = ensure_dir(&sys, "fs");
        ensure_dir(&fs_dir, "cgroup");
        mount::do_mount("cgroup2", "cgroup2", "/sys/fs/cgroup", 0, "").expect("mount cgroup2");
        let (_, cg_root) = mount::resolve_path_follow("/sys/fs/cgroup").expect("cgroup root");
        let cg_inode = cg_root.inode().expect("cgroup inode");
        let mkdir = cg_inode.ops.mkdir.expect("cgroup mkdir");
        let system = mkdir(&cg_inode, "system.slice", 0o755).expect("system.slice");
        let system_dentry = d_alloc_child(&cg_root, "system.slice");
        system_dentry.instantiate(system.clone());
        let service = mkdir(&system, "systemd-remount-fs.service", 0o755).expect("service");
        let service_dentry = d_alloc_child(&system_dentry, "systemd-remount-fs.service");
        service_dentry.instantiate(service);
        let stale = d_alloc_child(&service_dentry, "cgroup.events");
        assert!(stale.is_negative());

        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            ..OpenHow::default()
        };
        let opened = do_openat2(
            root.clone(),
            root,
            "/sys/fs/cgroup/system.slice/systemd-remount-fs.service/cgroup.events",
            &how,
        )
        .expect("stale negative cgroup.events must revalidate");

        assert!(alloc::sync::Arc::ptr_eq(&opened.file.dentry, &stale));
        assert!(!stale.is_negative());
        assert_eq!(opened.file.fops.name, "kernfs_file");
    }

    #[test]
    fn do_openat2_reports_absent_lsm_sysfs_mounts() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        ensure_dir(&root, "/sys");
        mount::do_mount("sysfs", "sysfs", "/sys", 0, "").expect("mount sysfs");

        let how = OpenHow {
            flags: (O_RDWR | O_CLOEXEC) as u64,
            ..OpenHow::default()
        };
        for path in [
            "/sys/fs/smackfs/load2",
            "/sys/fs/smackfs/change-rule",
            "/sys/fs/selinux/enforce",
        ] {
            assert_eq!(
                do_openat2(root.clone(), root.clone(), path, &how).err(),
                Some(ENOENT),
                "{path} must look absent when its LSM filesystem is not mounted"
            );
        }
    }

    #[test]
    fn do_openat2_writes_cgroup_files_when_root_mount_is_readonly() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let sys = ensure_dir(&root, "/sys");
        let fs_dir = ensure_dir(&sys, "fs");
        ensure_dir(&fs_dir, "cgroup");
        mount::rootfs()
            .expect("rootfs")
            .flags
            .store(MS_RDONLY as u32, core::sync::atomic::Ordering::Release);
        mount::do_mount("cgroup2", "cgroup2", "/sys/fs/cgroup", 0, "").expect("mount cgroup2");
        let (_, cg_root) = mount::resolve_path_follow("/sys/fs/cgroup").expect("cgroup root");
        let cg_inode = cg_root.inode().expect("cgroup inode");
        let mkdir = cg_inode.ops.mkdir.expect("cgroup mkdir");
        let system = mkdir(&cg_inode, "system.slice", 0o755).expect("system.slice");
        let system_dentry = d_alloc_child(&cg_root, "system.slice");
        system_dentry.instantiate(system.clone());
        let service = mkdir(&system, "systemd-journald.service", 0o755).expect("service");
        let service_dentry = d_alloc_child(&system_dentry, "systemd-journald.service");
        service_dentry.instantiate(service);

        let how = OpenHow {
            flags: (O_WRONLY | O_CLOEXEC) as u64,
            ..OpenHow::default()
        };
        do_openat2(
            root.clone(),
            root,
            "/sys/fs/cgroup/system.slice/systemd-journald.service/cgroup.procs",
            &how,
        )
        .expect("writable cgroup mount must not inherit rootfs MS_RDONLY");
    }

    #[test]
    fn do_openat2_allows_write_open_of_chardev_on_readonly_root() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let dev = ensure_dir(&root, "/dev");
        let console = d_alloc_child(&dev, "console");
        console.instantiate(crate::fs::types::Inode::new(
            200,
            InodeKind::Chardev,
            0o666,
            &crate::fs::ops::NOOP_INODE_OPS,
            &crate::fs::ops::NOOP_FILE_OPS,
            crate::fs::types::InodePrivate::Opaque(0),
        ));
        mount::rootfs()
            .expect("rootfs")
            .flags
            .store(MS_RDONLY as u32, core::sync::atomic::Ordering::Release);

        let how = OpenHow {
            flags: (O_WRONLY | O_CLOEXEC) as u64,
            ..OpenHow::default()
        };
        do_openat2(root.clone(), root, "/dev/console", &how)
            .expect("readonly mount must not reject writable chardev opens");
    }

    #[test]
    fn do_openat2_opens_dot_and_dotdot_as_directories() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let tmp = d_lookup(&root, "tmp").expect("/tmp");
        let how = OpenHow {
            flags: (crate::include::uapi::fcntl::O_RDONLY | O_DIRECTORY) as u64,
            ..OpenHow::default()
        };

        let dot = do_openat2(root.clone(), tmp.clone(), ".", &how).expect("open dot");
        assert_eq!(file_path(&dot.file), "/tmp");

        let dotdot = do_openat2(root.clone(), tmp, "..", &how).expect("open dotdot");
        assert_eq!(file_path(&dotdot.file), "/");
    }

    #[test]
    fn do_openat2_follows_final_symlink_unless_nofollow() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        let etc = d_lookup(&root, "etc").expect("/etc");
        let etc_inode = etc.inode().expect("/etc inode");
        let create = etc_inode.ops.create.expect("create");
        let target_inode = create(&etc_inode, "target", 0o644).expect("target");
        let target = d_alloc_child(&etc, "target");
        target.instantiate(target_inode);
        crate::fs::ramfs::ramfs_symlink(&etc_inode, "link", "target", 0o777).expect("symlink");

        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            ..OpenHow::default()
        };
        let opened = do_openat2(root.clone(), root.clone(), "/etc/link", &how).expect("open link");
        assert_eq!(file_path(&opened.file), "/etc/target");

        let nofollow = OpenHow {
            flags: (crate::include::uapi::fcntl::O_RDONLY | crate::include::uapi::fcntl::O_NOFOLLOW)
                as u64,
            ..OpenHow::default()
        };
        assert_eq!(
            do_openat2(root.clone(), root, "/etc/link", &nofollow).err(),
            Some(ELOOP)
        );
    }

    #[test]
    fn do_openat2_o_creat_follows_symlinked_parent_with_o_nofollow() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        OPEN_LOG.lock().unwrap().clear();

        let root = setup_rootfs();
        ensure_dir(&root, "/run");
        let var = ensure_dir(&root, "/var");
        let var_inode = var.inode().expect("/var inode");
        crate::fs::ramfs::ramfs_symlink(&var_inode, "run", "/run", 0o777)
            .expect("/var/run symlink");

        let how = OpenHow {
            flags: (O_CREAT | O_TRUNC | O_NOFOLLOW | O_WRONLY) as u64,
            mode: 0o644,
            ..OpenHow::default()
        };
        let opened = do_openat2(root.clone(), root, "/var/run/auditd.pid", &how)
            .expect("create through symlinked parent");

        assert_eq!(file_path(&opened.file), "/run/auditd.pid");
        assert!(mount::resolve_path_follow("/run/auditd.pid").is_ok());
    }

    #[test]
    fn do_openat2_with_hint_opens_relative_proc_pid_cgroup() {
        let _guard = mount::TEST_MOUNT_LOCK.lock();

        reset_for_test();
        let root = setup_rootfs();
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let cred = Box::new(unprivileged_cred());
        let previous = install_unprivileged_current(&mut current, &cred);

        let how = OpenHow {
            flags: crate::include::uapi::fcntl::O_RDONLY as u64,
            ..OpenHow::default()
        };
        let opened = do_openat2_with_hint(
            root.clone(),
            root,
            "cgroup",
            &how,
            Some("/proc/4242/cgroup"),
        )
        .expect("relative proc cgroup");

        unsafe { sched::set_current(previous) };
        assert_eq!(opened.file.fops.name, "proc-pid-cgroup");
        current.cred = &raw const INIT_CRED;
        current.m27.real_cred = &raw const INIT_CRED;
    }
}
