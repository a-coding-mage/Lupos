//! linux-parity: partial
//! linux-source: vendor/linux/fs/fs_struct.c
//! test-origin: linux:vendor/linux/fs/fs_struct.c
//! `struct fs_struct` — per-task filesystem root + cwd.
//!
//! Faithful port of `vendor/linux/fs/fs_struct.c` onto Lupos's `VfsPath` and
//! task model.  As in Linux, root and pwd retain both the mount and dentry;
//! Linux `path_get` / `path_put` map onto `Arc` clone / drop, and Linux's
//! explicit `int users` reference count is kept exactly (it is *not* the
//! `Arc` strong count).
//!
//! The `fs_struct` is owned by `task_struct` via `M39FsFields::fs` (a raw
//! `*mut FsStruct`). Legacy string helpers are derived from the retained paths
//! and are not used as pathname-lookup identity.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use super::mount::VfsPath;
use super::types::DentryRef;
use crate::include::uapi::errno::EINVAL;
use crate::kernel::task::TaskStruct;

/// `struct fs_struct` (`vendor/linux/include/linux/fs_struct.h`).
pub struct FsStruct {
    /// Linux `int users` — explicit reference count, freed on last `put`.
    pub users: AtomicU32,
    /// Linux `int in_exec`.
    pub in_exec: AtomicBool,
    /// Linux `int umask`.
    pub umask: AtomicU32,
    /// Linux `struct path root`.
    pub root: Mutex<Option<VfsPath>>,
    /// Linux `struct path pwd`.
    pub pwd: Mutex<Option<VfsPath>>,
}

impl FsStruct {
    fn boxed(umask: u32, root: Option<VfsPath>, pwd: Option<VfsPath>) -> *mut FsStruct {
        Box::into_raw(Box::new(FsStruct {
            users: AtomicU32::new(1),
            in_exec: AtomicBool::new(false),
            umask: AtomicU32::new(umask & 0o777),
            root: Mutex::new(root),
            pwd: Mutex::new(pwd),
        }))
    }
}

// ── task_struct ↔ fs_struct plumbing ───────────────────────────────────────

/// Read `task->fs` (`M39FsFields::fs`). Null when the task has no fs_struct yet.
///
/// # Safety
/// `task` must be null or a valid `*mut TaskStruct`.
pub unsafe fn task_fs(task: *mut TaskStruct) -> *mut FsStruct {
    if task.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { (*task).m39_fs.fs as *mut FsStruct }
}

/// Write `task->fs`.
///
/// # Safety
/// `task` must be null or a valid `*mut TaskStruct`; `fs` must be null or owned.
pub unsafe fn set_task_fs(task: *mut TaskStruct, fs: *mut FsStruct) {
    if !task.is_null() {
        unsafe {
            (*task).m39_fs.fs = fs as *mut core::ffi::c_void;
        }
    }
}

fn root_path() -> Option<VfsPath> {
    crate::fs::mount::namespace_root_path()
}

/// Linux `init_fs` — the initial task's fs (umask 022, root/pwd at the VFS root).
pub fn init_fs() -> *mut FsStruct {
    let r = root_path();
    FsStruct::boxed(0o022, r.clone(), r)
}

/// Lazily attach a default `fs_struct` to a task that has none, mirroring the
/// invariant that every Linux task carries an `fs_struct`.
///
/// # Safety
/// `task` must be a valid `*mut TaskStruct`.
pub unsafe fn ensure_task_fs(task: *mut TaskStruct) -> *mut FsStruct {
    let existing = unsafe { task_fs(task) };
    if !existing.is_null() {
        return existing;
    }
    let r = root_path();
    let fresh = FsStruct::boxed(0o022, r.clone(), r);
    unsafe { set_task_fs(task, fresh) };
    fresh
}

/// `current->fs`, lazily initialized. Null only when there is no current task
/// (very early boot) — callers must null-check.
pub fn current_fs() -> *mut FsStruct {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { ensure_task_fs(task) }
}

/// Return `current->fs->umask`.
///
/// Linux keeps the creation mask in the (optionally `CLONE_FS`-shared)
/// `fs_struct`, not in a system-wide variable.  The early-boot fallback is the
/// same 022 used by `init_fs`.
pub fn current_umask() -> u32 {
    let fs = current_fs();
    if fs.is_null() {
        0o022
    } else {
        (unsafe { (*fs).umask.load(Ordering::Acquire) }) & 0o777
    }
}

/// Atomically replace `current->fs->umask`, returning the previous mask.
pub fn set_current_umask(mask: u32) -> u32 {
    let fs = current_fs();
    if fs.is_null() {
        return 0o022;
    }
    unsafe { (*fs).umask.swap(mask & 0o777, Ordering::AcqRel) }
}

pub fn current_root_and_pwd_paths() -> Option<(VfsPath, VfsPath)> {
    let fs = current_fs();
    if fs.is_null() {
        return None;
    }
    let fs = unsafe { &*fs };
    let root = fs.root.lock().clone().or_else(root_path)?;
    let pwd = fs.pwd.lock().clone().unwrap_or_else(|| root.clone());
    Some((root, pwd))
}

/// Dentry-only compatibility accessor for subsystems that have not yet grown
/// a `struct path` parameter.  Pathname lookup must use
/// `current_root_and_pwd_paths()`.
pub fn current_root_and_pwd() -> Option<(DentryRef, DentryRef)> {
    let (root, pwd) = current_root_and_pwd_paths()?;
    Some((root.dentry, pwd.dentry))
}

fn path_components_below_root(root: &DentryRef, dentry: &DentryRef) -> Option<Vec<String>> {
    let mut components = Vec::new();
    let mut cur = Some(dentry.clone());
    while let Some(node) = cur {
        if Arc::ptr_eq(&node, root) {
            components.reverse();
            return Some(components);
        }
        if node.name != "/" && !node.name.is_empty() {
            components.push(node.name.clone());
        }
        cur = node.parent.lock().clone();
    }
    None
}

fn join_components(components: &[String]) -> String {
    if components.is_empty() {
        return String::from("/");
    }
    let mut out = String::new();
    for component in components {
        out.push('/');
        out.push_str(component);
    }
    out
}

pub fn path_from_root(root: &DentryRef, dentry: &DentryRef) -> Option<String> {
    path_components_below_root(root, dentry).map(|components| join_components(&components))
}

pub fn path_from_current_root(dentry: &DentryRef) -> Option<String> {
    let (root, _) = current_root_and_pwd_paths()?;
    let path = VfsPath::new(root.mount.clone(), dentry.clone());
    crate::fs::mount::path_between(&root, &path)
}

pub fn visible_path_for_current_root(path: &VfsPath) -> String {
    current_root_and_pwd_paths()
        .and_then(|(root, _)| crate::fs::mount::path_between(&root, path))
        .unwrap_or_else(|| crate::fs::file::dentry_path(&path.dentry))
}

// ── fs_struct.c functions ───────────────────────────────────────────────────

/// Linux `set_fs_root` — replace `fs->root`, dropping (`path_put`) the old one.
pub fn set_fs_root_path(fs: &FsStruct, path: VfsPath) {
    let old = fs.root.lock().replace(path);
    drop(old);
}

pub fn set_fs_root(fs: &FsStruct, dentry: DentryRef) {
    if let Some(path) = VfsPath::for_dentry(dentry) {
        set_fs_root_path(fs, path);
    }
}

/// Linux `set_fs_pwd` — replace `fs->pwd`, dropping the old one.
pub fn set_fs_pwd_path(fs: &FsStruct, path: VfsPath) {
    let old = fs.pwd.lock().replace(path);
    drop(old);
}

pub fn set_fs_pwd(fs: &FsStruct, dentry: DentryRef) {
    if let Some(path) = VfsPath::for_dentry(dentry) {
        set_fs_pwd_path(fs, path);
    }
}

/// Linux `replace_path` — if `slot` still points at `old`, swap in `new` and
/// report a hit. `Arc` clone/drop balances Linux's `path_get`/`path_put`.
fn replace_path(slot: &Mutex<Option<VfsPath>>, old: &VfsPath, new: &VfsPath) -> bool {
    let mut guard = slot.lock();
    let is_match = guard.as_ref().is_some_and(|cur| cur.equal(old));
    if is_match {
        *guard = Some(new.clone());
    }
    is_match
}

/// Linux `chroot_fs_refs` — after a namespace root transition, rewrite every
/// task whose root or pwd still pointed at `old_root`.  Linux `chroot(2)` does
/// not call this helper.
pub fn chroot_fs_refs(old_root: &VfsPath, new_root: &VfsPath) {
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() {
        let fs = unsafe { task_fs(current) };
        if !fs.is_null() {
            let fs = unsafe { &*fs };
            replace_path(&fs.root, old_root, new_root);
            replace_path(&fs.pwd, old_root, new_root);
        }
    }

    crate::kernel::fork::for_each_heap_task(|task| {
        if task == current {
            return;
        }
        let fs = unsafe { task_fs(task) };
        if fs.is_null() {
            return;
        }
        let fs = unsafe { &*fs };
        replace_path(&fs.root, old_root, new_root);
        replace_path(&fs.pwd, old_root, new_root);
    });
}

/// Linux `copy_fs_struct` — a fresh fs_struct cloning `old`'s root/pwd/umask,
/// with `users = 1` and `in_exec = 0`.
pub fn copy_fs_struct(old: &FsStruct) -> *mut FsStruct {
    let root = old.root.lock().clone();
    let pwd = old.pwd.lock().clone();
    FsStruct::boxed(old.umask.load(Ordering::Relaxed), root, pwd)
}

/// Increment `users` (Linux shared `fs_struct` on `CLONE_FS`).
pub fn get_fs_struct(fs: *mut FsStruct) -> *mut FsStruct {
    if !fs.is_null() {
        unsafe { (*fs).users.fetch_add(1, Ordering::AcqRel) };
    }
    fs
}

/// Linux `free_fs_struct` — release the allocation (after the last `put`).
///
/// # Safety
/// `fs` must be a unique, owned pointer with `users == 0`.
pub unsafe fn free_fs_struct(fs: *mut FsStruct) {
    if !fs.is_null() {
        drop(unsafe { Box::from_raw(fs) });
    }
}

/// Drop one `users` reference; free on the last one (Linux `put_fs_struct`).
///
/// # Safety
/// `fs` must be null or a valid owned `*mut FsStruct`.
pub unsafe fn put_fs_struct(fs: *mut FsStruct) {
    if fs.is_null() {
        return;
    }
    if unsafe { (*fs).users.fetch_sub(1, Ordering::AcqRel) } == 1 {
        unsafe { free_fs_struct(fs) };
    }
}

/// Linux `exit_fs` — detach `task->fs` and drop its reference on task exit.
///
/// # Safety
/// `task` must be a valid `*mut TaskStruct`.
pub unsafe fn exit_fs(task: *mut TaskStruct) {
    let fs = unsafe { task_fs(task) };
    if !fs.is_null() {
        unsafe { set_task_fs(task, core::ptr::null_mut()) };
        unsafe { put_fs_struct(fs) };
    }
}

/// fork `copy_fs`: the child shares the parent's `fs_struct` on `CLONE_FS`
/// (bumping `users`), otherwise it gets a private `copy_fs_struct`.
///
/// # Safety
/// `child` and `parent` must be valid `*mut TaskStruct`.
pub unsafe fn copy_fs(child: *mut TaskStruct, parent: *mut TaskStruct, share: bool) {
    let parent_fs = unsafe { ensure_task_fs(parent) };
    let child_fs = if share {
        get_fs_struct(parent_fs)
    } else {
        copy_fs_struct(unsafe { &*parent_fs })
    };
    unsafe { set_task_fs(child, child_fs) };
}

/// Linux `unshare_fs_struct` — give `current` a private copy of its fs_struct.
///
/// # Safety
/// Must be called with a valid current task.
pub unsafe fn unshare_fs_struct() -> Result<(), i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return Err(EINVAL);
    }
    let old = unsafe { ensure_task_fs(task) };
    let new = copy_fs_struct(unsafe { &*old });
    unsafe { set_task_fs(task, new) };
    unsafe { put_fs_struct(old) };
    Ok(())
}

// ── Lupos string-resolver bridge (path resolution still walks strings) ──────

lazy_static! {
    static ref CURRENT_CWD_PATH: Mutex<String> = Mutex::new(String::from("/"));
}

pub fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            name => parts.push(name),
        }
    }

    if parts.is_empty() {
        return String::from("/");
    }

    let mut out = String::new();
    for part in parts {
        out.push('/');
        out.push_str(part);
    }
    out
}

pub fn current_cwd_path() -> String {
    CURRENT_CWD_PATH.lock().clone()
}

pub fn set_current_cwd_path(path: &str) {
    *CURRENT_CWD_PATH.lock() = normalize_path(path);
}

pub fn absolute_from_cwd(path: &str) -> String {
    if let Some((root, pwd)) = current_root_and_pwd_paths() {
        let mut components = if path.starts_with('/') {
            Vec::new()
        } else {
            crate::fs::mount::path_between(&root, &pwd)
                .map(|path| {
                    path.split('/')
                        .filter(|component| !component.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default()
        };
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    components.pop();
                }
                name => components.push(String::from(name)),
            }
        }

        return join_components(&components);
    }

    if path.starts_with('/') {
        return normalize_path(path);
    }

    let cwd = current_cwd_path();
    let mut joined = if cwd == "/" {
        String::from("/")
    } else {
        let mut s = cwd;
        s.push('/');
        s
    };
    joined.push_str(path);
    normalize_path(&joined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::mount::Mount;
    use crate::fs::ops::NOOP_SUPER_OPS;
    use crate::fs::types::{Dentry, SuperBlock};
    use crate::kernel::sched;
    use crate::kernel::task::TaskStruct;

    fn child(parent: &DentryRef, name: &str) -> DentryRef {
        let dentry = Dentry::new_negative(name);
        *dentry.parent.lock() = Some(parent.clone());
        dentry
    }

    fn test_mount(root: &DentryRef) -> Arc<Mount> {
        let sb = SuperBlock::alloc("fs-struct-test", 0, &NOOP_SUPER_OPS);
        *sb.root.lock() = Some(root.clone());
        Mount::alloc(sb, root.clone(), 0)
    }

    #[test]
    fn normalize_path_collapses_dot_and_dotdot() {
        assert_eq!(normalize_path("/a/./b/../c"), "/a/c");
        assert_eq!(normalize_path("../../"), "/");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn init_fs_defaults_match_linux() {
        let fs = init_fs();
        let f = unsafe { &*fs };
        assert_eq!(f.users.load(Ordering::Relaxed), 1);
        assert_eq!(f.umask.load(Ordering::Relaxed), 0o022);
        assert!(!f.in_exec.load(Ordering::Relaxed));
        unsafe { put_fs_struct(fs) };
    }

    #[test]
    fn set_root_and_pwd_replace_and_release() {
        let fs = init_fs();
        let f = unsafe { &*fs };
        let mount_root = Dentry::new_negative("/");
        let mount = test_mount(&mount_root);
        let a = Dentry::new_negative("a");
        let b = Dentry::new_negative("b");
        set_fs_root_path(f, VfsPath::new(mount.clone(), a.clone()));
        set_fs_pwd_path(f, VfsPath::new(mount.clone(), b.clone()));
        assert!(Arc::ptr_eq(&f.root.lock().as_ref().unwrap().dentry, &a));
        assert!(Arc::ptr_eq(&f.pwd.lock().as_ref().unwrap().dentry, &b));
        // Replacing root drops the old reference (only our `a`/`b` remain).
        let c = Dentry::new_negative("c");
        set_fs_root_path(f, VfsPath::new(mount, c.clone()));
        assert_eq!(Arc::strong_count(&a), 1);
        assert!(Arc::ptr_eq(&f.root.lock().as_ref().unwrap().dentry, &c));
        unsafe { put_fs_struct(fs) };
    }

    #[test]
    fn replace_path_only_swaps_matching_dentry() {
        let old = Dentry::new_negative("old");
        let new = Dentry::new_negative("new");
        let other = Dentry::new_negative("other");
        let mount_root = Dentry::new_negative("/");
        let mount = test_mount(&mount_root);
        let old_path = VfsPath::new(mount.clone(), old.clone());
        let new_path = VfsPath::new(mount.clone(), new.clone());
        let other_path = VfsPath::new(mount, other.clone());
        let slot = Mutex::new(Some(old_path.clone()));
        assert!(replace_path(&slot, &old_path, &new_path));
        assert!(Arc::ptr_eq(&slot.lock().as_ref().unwrap().dentry, &new));
        // A second call with the now-stale `old` must not match.
        assert!(!replace_path(&slot, &old_path, &other_path));
        assert!(Arc::ptr_eq(&slot.lock().as_ref().unwrap().dentry, &new));
    }

    #[test]
    fn copy_fs_struct_clones_state_with_fresh_users() {
        let parent = init_fs();
        let p = unsafe { &*parent };
        p.umask.store(0o027, Ordering::Relaxed);
        let root = Dentry::new_negative("root");
        let mount = test_mount(&root);
        set_fs_root_path(p, VfsPath::new(mount, root.clone()));
        let child = copy_fs_struct(p);
        let c = unsafe { &*child };
        assert_eq!(c.users.load(Ordering::Relaxed), 1);
        assert_eq!(c.umask.load(Ordering::Relaxed), 0o027);
        assert!(Arc::ptr_eq(&c.root.lock().as_ref().unwrap().dentry, &root));
        unsafe { put_fs_struct(child) };
        unsafe { put_fs_struct(parent) };
    }

    #[test]
    fn get_and_put_track_users_and_free_on_last() {
        let fs = init_fs();
        assert_eq!(get_fs_struct(fs), fs);
        assert_eq!(unsafe { (*fs).users.load(Ordering::Relaxed) }, 2);
        unsafe { put_fs_struct(fs) }; // 2 -> 1, not freed
        assert_eq!(unsafe { (*fs).users.load(Ordering::Relaxed) }, 1);
        unsafe { put_fs_struct(fs) }; // 1 -> 0, freed (no UAF below)
    }

    #[test]
    fn copy_fs_shares_or_copies_like_clone_fs() {
        let mut parent = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut shared_child = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut private_child = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let fs = init_fs();
        unsafe {
            set_task_fs(&mut *parent, fs);
            copy_fs(&mut *shared_child, &mut *parent, true);
            assert_eq!(task_fs(&mut *shared_child), fs);
            assert_eq!((*fs).users.load(Ordering::Relaxed), 2);

            copy_fs(&mut *private_child, &mut *parent, false);
            let private = task_fs(&mut *private_child);
            assert_ne!(private, fs);
            assert_eq!((*private).users.load(Ordering::Relaxed), 1);
            assert_eq!((*fs).users.load(Ordering::Relaxed), 2);

            (*fs).umask.store(0o077, Ordering::Release);
            assert_eq!(
                (*task_fs(&mut *shared_child)).umask.load(Ordering::Acquire),
                0o077
            );
            assert_eq!((*private).umask.load(Ordering::Acquire), 0o022);

            exit_fs(&mut *shared_child);
            exit_fs(&mut *private_child);
            exit_fs(&mut *parent);
        }
    }

    #[test]
    fn exit_fs_detaches_and_drops_one_user() {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let fs = init_fs();
        get_fs_struct(fs);
        unsafe {
            set_task_fs(&mut *task, fs);
            exit_fs(&mut *task);
            assert!(task_fs(&mut *task).is_null());
            assert_eq!((*fs).users.load(Ordering::Relaxed), 1);
            put_fs_struct(fs);
        }
    }

    #[test]
    fn unshare_fs_struct_swaps_current_to_private_copy() {
        let previous = unsafe { sched::get_current() };
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let fs = init_fs();
        unsafe { (*fs).umask.store(0o027, Ordering::Release) };
        get_fs_struct(fs);
        unsafe {
            set_task_fs(&mut *task, fs);
            sched::set_current(&mut *task);
            unshare_fs_struct().expect("unshare");
            let new = task_fs(&mut *task);
            assert_ne!(new, fs);
            assert_eq!((*new).users.load(Ordering::Relaxed), 1);
            assert_eq!((*fs).users.load(Ordering::Relaxed), 1);
            assert_eq!((*new).umask.load(Ordering::Acquire), 0o027);
            (*new).umask.store(0o077, Ordering::Release);
            assert_eq!((*fs).umask.load(Ordering::Acquire), 0o027);
            exit_fs(&mut *task);
            put_fs_struct(fs);
            sched::set_current(previous);
        }
    }

    #[test]
    fn chroot_fs_refs_updates_current_root_and_pwd_refs() {
        let previous = unsafe { sched::get_current() };
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let old = Dentry::new_negative("/");
        let new = child(&old, "jail");
        let mount = test_mount(&old);
        let old_path = VfsPath::new(mount.clone(), old.clone());
        let new_path = VfsPath::new(mount, new.clone());
        let fs = FsStruct::boxed(0o022, Some(old_path.clone()), Some(old_path.clone()));
        unsafe {
            set_task_fs(&mut *task, fs);
            sched::set_current(&mut *task);
            chroot_fs_refs(&old_path, &new_path);
            let f = &*fs;
            assert!(Arc::ptr_eq(&f.root.lock().as_ref().unwrap().dentry, &new));
            assert!(Arc::ptr_eq(&f.pwd.lock().as_ref().unwrap().dentry, &new));
            exit_fs(&mut *task);
            sched::set_current(previous);
        }
    }

    #[test]
    fn absolute_from_cwd_stays_beneath_current_root() {
        let previous = unsafe { sched::get_current() };
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let host = Dentry::new_negative("/");
        let jail = child(&host, "jail");
        let work = child(&jail, "work");
        let mount = test_mount(&host);
        let fs = FsStruct::boxed(
            0o022,
            Some(VfsPath::new(mount.clone(), jail.clone())),
            Some(VfsPath::new(mount, work.clone())),
        );
        unsafe {
            set_task_fs(&mut *task, fs);
            sched::set_current(&mut *task);
            assert_eq!(absolute_from_cwd("file"), "/work/file");
            assert_eq!(absolute_from_cwd("../etc"), "/etc");
            assert_eq!(absolute_from_cwd("/../../etc"), "/etc");
            assert_eq!(path_from_current_root(&work).as_deref(), Some("/work"));
            exit_fs(&mut *task);
            sched::set_current(previous);
        }
    }
}
