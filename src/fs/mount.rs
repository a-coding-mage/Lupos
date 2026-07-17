//! linux-parity: partial
//! linux-source: vendor/linux/fs/namespace.c, vendor/linux/fs/mount.h
//! test-origin: linux:vendor/linux/fs/namespace.c
//! `struct mount` / `struct vfsmount` — M39.
//!
//! Mirrors `vendor/linux/fs/namespace.c` and `vendor/linux/fs/mount.h`.
//! Mount tree is a parent-pointer tree rooted at the namespace root.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ELOOP, ENOENT, EPERM, EROFS, EXDEV};
use crate::include::uapi::mount::{
    MS_BIND, MS_MOVE, MS_NOATIME, MS_NODEV, MS_NODIRATIME, MS_NOEXEC, MS_NOSUID, MS_NOSYMFOLLOW,
    MS_PRIVATE, MS_RDONLY, MS_REC, MS_RELATIME, MS_REMOUNT, MS_SHARED, MS_SILENT, MS_SLAVE,
    MS_STRICTATIME, MS_UNBINDABLE,
};
use crate::include::uapi::openat2::{
    RESOLVE_BENEATH, RESOLVE_IN_ROOT, RESOLVE_NO_MAGICLINKS, RESOLVE_NO_SYMLINKS, RESOLVE_NO_XDEV,
};
use crate::kernel::capability::{CAP_SYS_ADMIN, capable};
use crate::kernel::nsproxy::INIT_NSPROXY;

use super::namei::LookupCtx;
use super::types::DCACHE_MOUNTED;
use super::types::{DentryRef, InodeKind, SuperBlockRef};

const PATH_MAX: usize = 4096;
const MAX_SYMLINK_FOLLOWS: usize = 40;

#[cfg(not(test))]
macro_rules! trace_mount {
    ($($arg:tt)*) => {
        if crate::kernel::debug_trace::fs_enabled()
            || crate::kernel::debug_trace::glycin_enabled()
        {
            crate::linux_driver_abi::tty::serial_println!($($arg)*);
        }
    };
}

#[cfg(test)]
macro_rules! trace_mount {
    ($($arg:tt)*) => {};
}

unsafe fn copy_mount_string(ptr: *const u8, max_len: usize) -> Result<String, i32> {
    if ptr.is_null() {
        return Ok(String::new());
    }
    let mut buf = alloc::vec![0u8; max_len];
    let n = unsafe {
        crate::arch::x86::kernel::uaccess::strncpy_from_user(buf.as_mut_ptr(), ptr, max_len)
    };
    if n < 0 {
        return Err(-n);
    }
    core::str::from_utf8(&buf[..n as usize])
        .map(String::from)
        .map_err(|_| EINVAL)
}

pub type MountId = u64;

pub struct Mount {
    pub id: MountId,
    pub sb: SuperBlockRef,
    pub mountpoint: Mutex<Option<DentryRef>>, // dentry on the parent
    pub root: DentryRef,                      // root dentry of this fs
    pub parent: Mutex<Option<Arc<Mount>>>,
    pub flags: AtomicU32,
    pub mnt_count: AtomicU32,
    /// Children keyed by *mountpoint dentry name path component*.
    pub children: Mutex<Vec<Arc<Mount>>>,
}

/// Linux `struct path`: the mount and dentry identities are inseparable.
///
/// Two mounts may expose the same dentry (bind mounts), and a mounted
/// filesystem root is not a child dentry of the mountpoint it covers.  Keeping
/// only the dentry therefore cannot implement Linux pathname or `fs_struct`
/// semantics across mount transitions.
#[derive(Clone)]
pub struct VfsPath {
    pub mount: Arc<Mount>,
    pub dentry: DentryRef,
}

impl VfsPath {
    pub fn new(mount: Arc<Mount>, dentry: DentryRef) -> Self {
        Self { mount, dentry }
    }

    pub fn equal(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.mount, &other.mount) && Arc::ptr_eq(&self.dentry, &other.dentry)
    }

    /// Recover a path for legacy dentry-only kernel callers.  New pathname
    /// lookup must carry the `VfsPath` returned by the resolver instead.
    pub fn for_dentry(dentry: DentryRef) -> Option<Self> {
        let mount = containing_mount_for_dentry(&dentry)
            .or_else(|| mounted_root_for_dentry(&dentry))
            .or_else(rootfs)?;
        Some(Self { mount, dentry })
    }
}

static NEXT_MOUNT_ID: AtomicU64 = AtomicU64::new(1);
static MOUNT_EVENT: AtomicU64 = AtomicU64::new(1);

impl Mount {
    pub fn alloc(sb: SuperBlockRef, root: DentryRef, flags: u32) -> Arc<Self> {
        Arc::new(Self {
            id: NEXT_MOUNT_ID.fetch_add(1, Ordering::AcqRel),
            sb,
            mountpoint: Mutex::new(None),
            root,
            parent: Mutex::new(None),
            flags: AtomicU32::new(flags),
            mnt_count: AtomicU32::new(1),
            children: Mutex::new(Vec::new()),
        })
    }
    pub fn is_readonly(&self) -> bool {
        self.flags.load(Ordering::Acquire) & 1 != 0
    }
}

/// Mount registry keyed by (parent_mount_id, mountpoint_name) — small enough
/// for one big lock at this scale.
pub struct MountTable {
    pub root: Mutex<Option<Arc<Mount>>>,
    /// Name → mount, walked relative to the namespace root, slash-separated.
    pub by_path: Mutex<BTreeMap<alloc::string::String, Arc<Mount>>>,
}

lazy_static::lazy_static! {
    pub static ref MOUNTS: Arc<MountTable> = Arc::new(MountTable {
        root: Mutex::new(None),
        by_path: Mutex::new(BTreeMap::new()),
    });
    static ref MOUNT_NAMESPACES: Mutex<BTreeMap<usize, Arc<MountTable>>> =
        Mutex::new(BTreeMap::new());
}

#[cfg(test)]
pub static TEST_MOUNT_LOCK: Mutex<()> = Mutex::new(());

fn table_for_namespace(ns: *const crate::fs::namespace::MntNamespace) -> Arc<MountTable> {
    if ns.is_null() || core::ptr::eq(ns, &raw const crate::fs::namespace::INIT_MNT_NS as *const _) {
        return MOUNTS.clone();
    }
    MOUNT_NAMESPACES
        .lock()
        .get(&(ns as usize))
        .cloned()
        .unwrap_or_else(|| MOUNTS.clone())
}

fn current_mount_table() -> Arc<MountTable> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return MOUNTS.clone();
    }
    let nsproxy = unsafe { (*task).m28_nsproxy.nsproxy };
    let nsproxy = if nsproxy.is_null() {
        &raw const INIT_NSPROXY as *mut crate::kernel::nsproxy::Nsproxy
    } else {
        nsproxy
    };
    table_for_namespace(unsafe { (*nsproxy).mnt_ns })
}

pub fn current_mount_entries() -> Vec<(String, Arc<Mount>)> {
    current_mount_table()
        .by_path
        .lock()
        .iter()
        .map(|(path, mount)| (path.clone(), mount.clone()))
        .collect()
}

pub fn namespace_root_for(ns: *const crate::fs::namespace::MntNamespace) -> Option<VfsPath> {
    let mount = table_for_namespace(ns).root.lock().clone()?;
    Some(VfsPath::new(mount.clone(), mount.root.clone()))
}

pub fn resolve_from_namespace_root(root: &VfsPath, path: &str) -> Result<VfsPath, i32> {
    walk_path(root, root.clone(), path, true, 0)
}

fn clone_mount_subtree(
    source: &Arc<Mount>,
    parent: Option<Arc<Mount>>,
    by_id: &mut BTreeMap<MountId, Arc<Mount>>,
) -> Arc<Mount> {
    let cloned = Mount::alloc(
        source.sb.clone(),
        source.root.clone(),
        source.flags.load(Ordering::Acquire),
    );
    *cloned.mountpoint.lock() = source.mountpoint.lock().clone();
    *cloned.parent.lock() = parent;
    by_id.insert(source.id, cloned.clone());
    for source_child in source.children.lock().clone() {
        let child = clone_mount_subtree(&source_child, Some(cloned.clone()), by_id);
        cloned.children.lock().push(child);
    }
    cloned
}

/// Clone the complete mount tree for a new mount namespace and publish it
/// before the child can run. Superblocks and dentries remain shared, while
/// mount topology, flags, and later attach/detach operations are private.
pub fn register_mount_namespace(
    ns: *mut crate::fs::namespace::MntNamespace,
    old: *const crate::fs::namespace::MntNamespace,
) -> *mut Mount {
    let source = table_for_namespace(old);
    let Some(source_root) = source.root.lock().clone() else {
        MOUNT_NAMESPACES.lock().insert(
            ns as usize,
            Arc::new(MountTable {
                root: Mutex::new(None),
                by_path: Mutex::new(BTreeMap::new()),
            }),
        );
        return core::ptr::null_mut();
    };
    let mut by_id = BTreeMap::new();
    let root = clone_mount_subtree(&source_root, None, &mut by_id);
    let mut by_path = BTreeMap::new();
    for (path, source_mount) in source.by_path.lock().iter() {
        if let Some(cloned) = by_id.get(&source_mount.id) {
            by_path.insert(path.clone(), cloned.clone());
        }
    }
    let root_ptr = Arc::as_ptr(&root) as *mut Mount;
    MOUNT_NAMESPACES.lock().insert(
        ns as usize,
        Arc::new(MountTable {
            root: Mutex::new(Some(root)),
            by_path: Mutex::new(by_path),
        }),
    );
    root_ptr
}

pub fn unregister_mount_namespace(ns: *const crate::fs::namespace::MntNamespace) {
    if !ns.is_null() {
        MOUNT_NAMESPACES.lock().remove(&(ns as usize));
    }
}

/// Set the rootfs mount (init time).
pub fn set_rootfs(m: Arc<Mount>) {
    let table = current_mount_table();
    *table.root.lock() = Some(m.clone());
    table
        .by_path
        .lock()
        .insert(alloc::string::String::from("/"), m);
    notify_mount_change();
}

pub fn rootfs() -> Option<Arc<Mount>> {
    current_mount_table().root.lock().clone()
}

pub fn namespace_root_path() -> Option<VfsPath> {
    let mount = rootfs()?;
    Some(VfsPath::new(mount.clone(), mount.root.clone()))
}

pub fn mount_event() -> u64 {
    MOUNT_EVENT.load(Ordering::Acquire)
}

pub fn notify_mount_change() {
    MOUNT_EVENT.fetch_add(1, Ordering::AcqRel);
}

/// Mount `fs_name` at `mountpoint_path` (relative to rootfs root).  Simplified
/// model: the resolved dentry must already exist; the mount supersedes it.
pub fn do_mount(
    fs_name: &str,
    source: &str,
    mountpoint_path: &str,
    flags: u64,
    data: &str,
) -> Result<Arc<Mount>, i32> {
    let sb = super::super_block::mount_fs(fs_name, source, flags, data)?;
    let new_root = sb.root().ok_or(EINVAL)?;
    let m = Mount::alloc(sb, new_root, flags as u32);
    attach_mount(m.clone(), mountpoint_path)?;
    Ok(m)
}

fn do_bind_mount(source_path: &str, mountpoint_path: &str, flags: u64) -> Result<Arc<Mount>, i32> {
    if source_path.is_empty() {
        return Err(ENOENT);
    }
    let (source_mount, source_root) = match resolve_path_follow(source_path) {
        Ok(source) => source,
        Err(errno) => {
            trace_mount!(
                "trace-bind-resolve source={} target={} failed=source errno={}",
                source_path,
                mountpoint_path,
                errno
            );
            return Err(errno);
        }
    };
    if bind_source_is_target_mount_root(&source_mount, &source_root, mountpoint_path) {
        return Ok(source_mount);
    }
    if bind_source_is_target(&source_mount, &source_root, mountpoint_path) {
        return Ok(source_mount);
    }
    let source_root_for_walk = source_root.clone();
    let source_root_path = proc_self_fd_path_hint(source_path)
        .or_else(|| absolute_mount_path(source_path))
        .or_else(|| stable_path_for_dentry(&source_root_for_walk))
        .unwrap_or_else(|| normalize_mount_path(source_path));
    let m = Mount::alloc(source_mount.sb.clone(), source_root, flags as u32);
    if let Err(errno) = attach_mount(m.clone(), mountpoint_path) {
        trace_mount!(
            "trace-bind-resolve source={} target={} failed=target errno={}",
            source_path,
            mountpoint_path,
            errno
        );
        return Err(errno);
    }
    if flags & MS_REC != 0 {
        // Drop the mountpoint guard before canonicalization.  Relative bind
        // targets fall through to path_for_dentry(), which walks every mount
        // and locks this same mountpoint field.  Keeping the guard alive
        // across the closure self-deadlocks bubblewrap's recursive self-bind.
        let bind_mountpoint = m.mountpoint.lock().clone();
        let bind_root_path = bind_mountpoint
            .as_ref()
            .map(|mp| canonical_mountpoint_path(mountpoint_path, mp))
            .or_else(|| path_for_dentry(&m.root))
            .unwrap_or_else(|| normalize_mount_path(mountpoint_path));
        clone_child_mounts_recursive(
            &source_mount,
            &source_root_for_walk,
            &source_root_path,
            &bind_root_path,
            m.id,
            &bind_root_path,
        )?;
    }
    Ok(m)
}

fn bind_source_is_target(
    source_mount: &Arc<Mount>,
    source_root: &DentryRef,
    mountpoint_path: &str,
) -> bool {
    let Some((target_parent, target_mountpoint)) = resolve_mount_target(mountpoint_path) else {
        return false;
    };
    target_mountpoint.flags.load(Ordering::Acquire) & DCACHE_MOUNTED != 0
        && Arc::ptr_eq(source_mount, &target_parent)
        && Arc::ptr_eq(source_root, &target_mountpoint)
}

fn bind_source_is_target_mount_root(
    source_mount: &Arc<Mount>,
    source_root: &DentryRef,
    mountpoint_path: &str,
) -> bool {
    if !Arc::ptr_eq(source_root, &source_mount.root) {
        return false;
    }
    let Some((target_parent, target_mountpoint)) = resolve_mount_target(mountpoint_path) else {
        return false;
    };
    source_mount
        .parent
        .lock()
        .as_ref()
        .is_some_and(|parent| Arc::ptr_eq(parent, &target_parent))
        && source_mount
            .mountpoint
            .lock()
            .as_ref()
            .is_some_and(|mountpoint| Arc::ptr_eq(mountpoint, &target_mountpoint))
}

fn clone_child_mounts_recursive(
    source_parent: &Arc<Mount>,
    source_root: &DentryRef,
    source_root_path: &str,
    dest_root_path: &str,
    exclude_mount_id: MountId,
    exclude_source_prefix: &str,
) -> Result<(), i32> {
    let children = source_parent.children.lock().clone();
    for child in children {
        if child.id == exclude_mount_id {
            continue;
        }
        let Some(source_mountpoint) = child.mountpoint.lock().clone() else {
            continue;
        };
        let Some(components) = dentry_components_below_root(&source_mountpoint, source_root) else {
            continue;
        };

        let source_child_path = join_mount_path(source_root_path, &components);
        if path_is_same_or_below(&source_child_path, exclude_source_prefix) {
            continue;
        }
        let dest_path = join_mount_path(dest_root_path, &components);
        let cloned = Mount::alloc(
            child.sb.clone(),
            child.root.clone(),
            child.flags.load(Ordering::Acquire),
        );
        attach_mount(cloned, &dest_path)?;
        clone_child_mounts_recursive(
            &child,
            &child.root,
            &source_child_path,
            &dest_path,
            exclude_mount_id,
            exclude_source_prefix,
        )?;
    }
    Ok(())
}

fn path_is_same_or_below(path: &str, prefix: &str) -> bool {
    let path = normalize_mount_path(path);
    let prefix = normalize_mount_path(prefix);
    path == prefix
        || (prefix != "/"
            && path.len() > prefix.len()
            && path.starts_with(&prefix)
            && path.as_bytes().get(prefix.len()) == Some(&b'/'))
}

fn do_move_mount(source_path: &str, mountpoint_path: &str, flags: u64) -> Result<(), i32> {
    if flags & !MS_MOVE != 0 {
        return Err(EINVAL);
    }
    if source_path.is_empty() {
        return Err(EINVAL);
    }

    // Linux resolves `old_name` with `kern_path()` and then requires that the
    // resulting struct path is the root of a mounted tree.  In particular,
    // MS_MOVE never degrades into MS_BIND when the spelling is relative.
    let source = resolve_path_follow(source_path)?;
    let source = VfsPath::new(source.0, source.1);
    if !Arc::ptr_eq(&source.dentry, &source.mount.root) || source.mount.parent.lock().is_none() {
        return Err(EINVAL);
    }

    let target = resolve_path_follow(mountpoint_path)?;
    let target = VfsPath::new(target.0, target.1);
    let source_is_dir = source
        .dentry
        .inode()
        .is_some_and(|inode| inode.kind == InodeKind::Directory);
    let target_is_dir = target
        .dentry
        .inode()
        .is_some_and(|inode| inode.kind == InodeKind::Directory);
    if source_is_dir != target_is_dir {
        return Err(EINVAL);
    }

    // `mount_is_ancestor(old, target_parent)` is rejected by Linux: attaching
    // a tree below itself would create a mount-cycle.
    let mut ancestor = Some(target.mount.clone());
    while let Some(mount) = ancestor {
        if Arc::ptr_eq(&mount, &source.mount) {
            return Err(ELOOP);
        }
        ancestor = mount.parent.lock().clone();
    }

    let old_parent = source.mount.parent.lock().clone().ok_or(EINVAL)?;
    let old_mountpoint = source.mount.mountpoint.lock().clone().ok_or(EINVAL)?;
    old_parent
        .children
        .lock()
        .retain(|mount| !Arc::ptr_eq(mount, &source.mount));
    if !old_parent.children.lock().iter().any(|mount| {
        mount
            .mountpoint
            .lock()
            .as_ref()
            .is_some_and(|mountpoint| Arc::ptr_eq(mountpoint, &old_mountpoint))
    }) {
        old_mountpoint
            .flags
            .fetch_and(!DCACHE_MOUNTED, Ordering::AcqRel);
    }

    unregister_mount_tree(&source.mount);
    attach_mount_at(source.mount, &target)
}

fn collect_mount_tree_ids(mount: &Arc<Mount>, ids: &mut Vec<MountId>) {
    ids.push(mount.id);
    for child in mount.children.lock().clone() {
        collect_mount_tree_ids(&child, ids);
    }
}

fn unregister_mount_tree(mount: &Arc<Mount>) {
    let mut ids = Vec::new();
    collect_mount_tree_ids(mount, &mut ids);
    current_mount_table()
        .by_path
        .lock()
        .retain(|_, candidate| !ids.contains(&candidate.id));
}

fn register_mount_tree(mount: &Arc<Mount>, path: &str, table: &mut BTreeMap<String, Arc<Mount>>) {
    table.insert(String::from(path), mount.clone());
    for child in mount.children.lock().clone() {
        let Some(mountpoint) = child.mountpoint.lock().clone() else {
            continue;
        };
        let Some(components) = dentry_components_below_root(&mountpoint, &mount.root) else {
            continue;
        };
        let child_path = join_mount_path(path, &components);
        register_mount_tree(&child, &child_path, table);
    }
}

fn attach_mount_at(mount: Arc<Mount>, target: &VfsPath) -> Result<(), i32> {
    let canonical_path = namespace_path(target).ok_or(EINVAL)?;
    *mount.mountpoint.lock() = Some(target.dentry.clone());
    *mount.parent.lock() = Some(target.mount.clone());
    target
        .dentry
        .flags
        .fetch_or(DCACHE_MOUNTED, Ordering::AcqRel);
    target.mount.children.lock().push(mount.clone());
    let mount_table = current_mount_table();
    let mut table = mount_table.by_path.lock();
    register_mount_tree(&mount, &canonical_path, &mut table);
    drop(table);
    notify_mount_change();
    Ok(())
}

pub fn attach_mount(m: Arc<Mount>, mountpoint_path: &str) -> Result<(), i32> {
    let (parent, mp) = resolve_mount_target(mountpoint_path).ok_or(ENOENT)?;
    let canonical_path = canonical_mountpoint_path(mountpoint_path, &mp);
    *m.mountpoint.lock() = Some(mp.clone());
    *m.parent.lock() = Some(parent.clone());
    mp.flags.fetch_or(DCACHE_MOUNTED, Ordering::AcqRel);
    parent.children.lock().push(m.clone());
    current_mount_table()
        .by_path
        .lock()
        .insert(canonical_path, m.clone());
    notify_mount_change();
    Ok(())
}

fn canonical_mountpoint_path(requested: &str, mp: &DentryRef) -> String {
    proc_self_fd_path_hint(requested)
        .or_else(|| absolute_mount_path(requested))
        .or_else(|| path_for_dentry(mp))
        .unwrap_or_else(|| super::file::dentry_path(mp))
}

fn proc_self_fd_path_hint(path: &str) -> Option<String> {
    let file = proc_self_fd_file(path)?.ok()?;
    super::file::path_hint(&file)
        .or_else(|| path_for_dentry(&file.dentry))
        .map(|path| normalize_mount_path(&path))
}

fn absolute_mount_path(path: &str) -> Option<String> {
    path.starts_with('/').then(|| normalize_mount_path(path))
}

fn normalize_mount_path(path: &str) -> String {
    let mut out = String::from("/");
    let mut first = true;
    for comp in path
        .split('/')
        .filter(|comp| !comp.is_empty() && *comp != ".")
    {
        if comp == ".." {
            continue;
        }
        if !first {
            out.push('/');
        }
        out.push_str(comp);
        first = false;
    }
    out
}

/// `mount(2)` — Linux x86-64 syscall 165.
///
/// Lupos currently supports the in-kernel pseudo filesystems used by early
/// SysV init (`proc`, `sysfs`, `devtmpfs`, `tmpfs`, ramfs family).  The ABI
/// shape mirrors Linux: user strings are copied first, then VFS mount dispatch
/// decides whether the filesystem exists.
fn resolve_mountpoint(path: &str) -> Option<(Arc<Mount>, DentryRef)> {
    if let Some(resolved) = resolve_proc_self_fd_mountpoint(path) {
        return resolved.ok();
    }
    resolve_path_follow(path).ok()
}

fn resolve_proc_self_fd_mountpoint(path: &str) -> Option<Result<(Arc<Mount>, DentryRef), i32>> {
    let file = match proc_self_fd_file(path)? {
        Ok(file) => file,
        Err(errno) => return Some(Err(errno)),
    };
    if let Some(path) = super::file::path_hint(&file) {
        return Some(resolve_mountpoint(&path).ok_or(ENOENT));
    }
    if let Some(path) = path_for_dentry(&file.dentry) {
        return Some(resolve_mountpoint(&path).ok_or(ENOENT));
    }
    if let Some(mount) = mounted_root_for_dentry_by_path(&file.dentry, true) {
        return Some(Ok((mount.clone(), mount.root.clone())));
    }

    let dentry = file.dentry.clone();
    let parent = containing_mount_for_dentry_by_path(&dentry, true)
        .or_else(rootfs)
        .ok_or(EINVAL);
    Some(parent.map(|parent| (parent, dentry)))
}

fn resolve_mount_target(path: &str) -> Option<(Arc<Mount>, DentryRef)> {
    if let Some(resolved) = resolve_proc_self_fd_mount_target(path) {
        return resolved.ok();
    }

    resolve_path_follow(path).ok()
}

fn resolve_proc_self_fd_mount_target(path: &str) -> Option<Result<(Arc<Mount>, DentryRef), i32>> {
    let file = match proc_self_fd_file(path)? {
        Ok(file) => file,
        Err(errno) => return Some(Err(errno)),
    };
    if let Some(path) = super::file::path_hint(&file) {
        return Some(resolve_mount_target(&path).ok_or(ENOENT));
    }
    if let Some(path) = path_for_dentry(&file.dentry) {
        return Some(resolve_mount_target(&path).ok_or(ENOENT));
    }
    if let Some(mount) = mounted_root_for_dentry_by_path(&file.dentry, true) {
        if let (Some(parent), Some(mountpoint)) =
            (mount.parent.lock().clone(), mount.mountpoint.lock().clone())
        {
            return Some(Ok((parent, mountpoint)));
        }
        return Some(Ok((mount.clone(), mount.root.clone())));
    }
    let dentry = file.dentry.clone();
    let parent = containing_mount_for_dentry_by_path(&dentry, true)
        .or_else(rootfs)
        .ok_or(EINVAL);
    Some(parent.map(|parent| (parent, dentry)))
}

fn proc_self_fd_file(path: &str) -> Option<Result<super::types::FileRef, i32>> {
    let rest = path.strip_prefix("/proc/self/fd/")?;
    if rest.is_empty() || rest.as_bytes().iter().any(|b| !b.is_ascii_digit()) {
        return Some(Err(ENOENT));
    }
    let fd = match rest.parse::<i32>() {
        Ok(fd) => fd,
        Err(_) => return Some(Err(ENOENT)),
    };
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return Some(Err(crate::include::uapi::errno::EBADF));
    }
    let files = unsafe { crate::kernel::files::get_task_files(task) };
    let Some(files) = files else {
        return Some(Err(crate::include::uapi::errno::EBADF));
    };
    let file = match files.get(fd) {
        Ok(file) => file,
        Err(errno) => return Some(Err(errno)),
    };
    Some(Ok(file))
}

pub(crate) fn containing_mount_for_dentry(dentry: &DentryRef) -> Option<Arc<Mount>> {
    containing_mount_for_dentry_by_path(dentry, false)
}

fn containing_mount_for_dentry_by_path(
    dentry: &DentryRef,
    prefer_longest_path_on_tie: bool,
) -> Option<Arc<Mount>> {
    let table = current_mount_table();
    let mounts = table.by_path.lock();
    let mut best: Option<(usize, usize, Arc<Mount>)> = None;
    for (path, mount) in mounts.iter() {
        if let Some(depth) = dentry_depth_below_root(dentry, &mount.root) {
            let path_len = path.len();
            let is_better = best.as_ref().is_none_or(|(best_depth, best_len, _)| {
                depth > *best_depth
                    || (depth == *best_depth
                        && mount_path_len_is_better(
                            Some(*best_len),
                            path_len,
                            prefer_longest_path_on_tie,
                        ))
            });
            if is_better {
                best = Some((depth, path_len, mount.clone()));
            }
        }
    }
    best.map(|(_, _, mount)| mount)
}

fn dentry_depth_below_root(dentry: &DentryRef, root: &DentryRef) -> Option<usize> {
    let mut depth = 0;
    let mut cur = Some(dentry.clone());
    while let Some(node) = cur {
        if Arc::ptr_eq(&node, root) {
            return Some(depth);
        }
        cur = node.parent.lock().clone();
        depth += 1;
    }
    None
}

const REMOUNT_UPDATABLE_FLAGS: u64 = MS_RDONLY
    | MS_NOSUID
    | MS_NODEV
    | MS_NOEXEC
    | MS_NOSYMFOLLOW
    | MS_NOATIME
    | MS_NODIRATIME
    | MS_RELATIME
    | MS_STRICTATIME;

fn remount_existing(mount: &Arc<Mount>, flags: u64) {
    let mut old = mount.flags.load(Ordering::Acquire);
    loop {
        let new =
            (old & !(REMOUNT_UPDATABLE_FLAGS as u32)) | ((flags & REMOUNT_UPDATABLE_FLAGS) as u32);
        match mount
            .flags
            .compare_exchange(old, new, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => break,
            Err(next) => old = next,
        }
    }
    notify_mount_change();
}

pub fn remount_mountpoint(target: &str, flags: u64) -> Result<(), i32> {
    let Some((mount, _)) = resolve_mountpoint(target) else {
        return Err(ENOENT);
    };
    remount_existing(&mount, flags);
    Ok(())
}

pub unsafe fn sys_mount(
    source: *const u8,
    target: *const u8,
    fstype: *const u8,
    flags: u64,
    data: *const u8,
) -> i64 {
    if !capable(CAP_SYS_ADMIN) {
        return -(EPERM as i64);
    }

    if target.is_null() {
        return -(crate::include::uapi::errno::EFAULT as i64);
    }
    if !capable(CAP_SYS_ADMIN) {
        return -(EPERM as i64);
    }

    let source = match unsafe { copy_mount_string(source, PATH_MAX) } {
        Ok(s) => s,
        Err(errno) => return -(errno as i64),
    };
    let target = match unsafe { copy_mount_string(target, PATH_MAX) } {
        Ok(s) => s,
        Err(errno) => return -(errno as i64),
    };
    let fstype = match unsafe { copy_mount_string(fstype, 256) } {
        Ok(s) => s,
        Err(errno) => return -(errno as i64),
    };
    let data = match unsafe { copy_mount_string(data, PATH_MAX) } {
        Ok(s) => s,
        Err(errno) => return -(errno as i64),
    };

    trace_mount!(
        "trace-mount-enter source={} target={} fstype={} flags={:#x} data=<redacted>",
        source,
        target,
        fstype,
        flags
    );

    if flags & MS_REMOUNT != 0 && flags & MS_BIND == 0 {
        let Some((mount, _)) = resolve_mountpoint(&target) else {
            trace_mount!("trace-mount-ret errno={}", ENOENT);
            return -(ENOENT as i64);
        };
        remount_existing(&mount, flags);
        trace_mount!("trace-mount-ret ok");
        return 0;
    }

    if fstype.is_empty() {
        let propagation_flags = MS_SHARED | MS_PRIVATE | MS_SLAVE | MS_UNBINDABLE;
        let propagation = flags & propagation_flags;
        if propagation != 0 && (propagation & (propagation - 1)) == 0 {
            // MS_SILENT is an accepted no-op request modifier. bubblewrap
            // combines it with MS_SLAVE|MS_REC for its namespace root.
            if flags & !(propagation_flags | MS_REC | MS_SILENT) != 0 {
                trace_mount!("trace-mount-ret errno={}", EINVAL);
                return -(EINVAL as i64);
            }
            if resolve_mountpoint(&target).is_none() {
                trace_mount!("trace-mount-ret errno={}", ENOENT);
                return -(ENOENT as i64);
            }
            trace_mount!("trace-mount-ret ok");
            return 0;
        }
        if flags & (MS_REMOUNT | MS_BIND) == (MS_REMOUNT | MS_BIND) {
            let Some((mount, _)) = resolve_mountpoint(&target) else {
                trace_mount!("trace-mount-ret errno={}", ENOENT);
                return -(ENOENT as i64);
            };
            remount_existing(&mount, flags);
            trace_mount!("trace-mount-ret ok");
            return 0;
        }
        if flags & MS_MOVE != 0 {
            match do_move_mount(&source, &target, flags) {
                Ok(()) => {
                    trace_mount!("trace-mount-ret ok");
                    return 0;
                }
                Err(errno) => {
                    trace_mount!("trace-mount-ret errno={}", errno);
                    return -(errno as i64);
                }
            }
        }
        if flags & MS_BIND != 0 {
            match do_bind_mount(&source, &target, flags) {
                Ok(_) => {
                    trace_mount!("trace-mount-ret ok");
                    return 0;
                }
                Err(errno) => {
                    trace_mount!("trace-mount-ret errno={}", errno);
                    return -(errno as i64);
                }
            }
        }
        trace_mount!("trace-mount-ret errno={}", EINVAL);
        return -(EINVAL as i64);
    }
    match do_mount(&fstype, &source, &target, flags, &data) {
        Ok(_) => {
            trace_mount!("trace-mount-ret ok");
            0
        }
        Err(errno) => {
            trace_mount!("trace-mount-ret errno={}", errno);
            -(errno as i64)
        }
    }
}

pub fn do_umount(mountpoint_path: &str, _flags: u32) -> Result<(), i32> {
    let table = current_mount_table();
    let keyed_mount = table.by_path.lock().get(mountpoint_path).cloned();
    let m = keyed_mount
        .or_else(|| {
            let (mount, dentry) = resolve_path_follow(mountpoint_path).ok()?;
            if Arc::ptr_eq(&dentry, &mount.root) && mount.parent.lock().is_some() {
                return Some(mount);
            }
            mount.children.lock().iter().rev().find_map(|child| {
                child
                    .mountpoint
                    .lock()
                    .as_ref()
                    .is_some_and(|mp| Arc::ptr_eq(mp, &dentry))
                    .then(|| child.clone())
            })
        })
        .ok_or(EINVAL)?;
    if m.parent.lock().is_none() {
        return Err(EBUSY);
    }
    unregister_mount_tree(&m);
    let mountpoint = m.mountpoint.lock().clone();
    if let Some(parent) = m.parent.lock().clone() {
        parent.children.lock().retain(|c| !Arc::ptr_eq(c, &m));
        if let Some(mp) = mountpoint.as_ref() {
            let replacement = parent
                .children
                .lock()
                .iter()
                .rev()
                .find(|child| {
                    child
                        .mountpoint
                        .lock()
                        .as_ref()
                        .is_some_and(|child_mp| Arc::ptr_eq(child_mp, mp))
                })
                .cloned();
            if let Some(replacement) = replacement {
                table
                    .by_path
                    .lock()
                    .insert(String::from(mountpoint_path), replacement);
            } else {
                mp.flags.fetch_and(!DCACHE_MOUNTED, Ordering::AcqRel);
            }
        }
    }
    notify_mount_change();
    Ok(())
}

pub fn lookup_mount(path: &str) -> Option<Arc<Mount>> {
    current_mount_table().by_path.lock().get(path).cloned()
}

pub fn mounted_root_for_dentry(dentry: &DentryRef) -> Option<Arc<Mount>> {
    mounted_root_for_dentry_by_path(dentry, false)
}

fn mounted_root_for_dentry_by_path(dentry: &DentryRef, prefer_longest: bool) -> Option<Arc<Mount>> {
    let table = current_mount_table();
    let mounts = table.by_path.lock();
    let mut best: Option<(usize, Arc<Mount>)> = None;
    for (path, mount) in mounts.iter() {
        if Arc::ptr_eq(&mount.root, dentry) {
            if mount_path_len_is_better(
                best.as_ref().map(|(best_len, _)| *best_len),
                path.len(),
                prefer_longest,
            ) {
                best = Some((path.len(), mount.clone()));
            }
        }
        if mount
            .mountpoint
            .lock()
            .as_ref()
            .is_some_and(|mp| Arc::ptr_eq(mp, dentry))
        {
            if mount_path_len_is_better(
                best.as_ref().map(|(best_len, _)| *best_len),
                path.len(),
                prefer_longest,
            ) {
                best = Some((path.len(), mount.clone()));
            }
        }
    }
    best.map(|(_, mount)| mount)
}

pub fn path_for_dentry(dentry: &DentryRef) -> Option<String> {
    path_for_dentry_by_mount_path(dentry, true)
}

pub fn stable_path_for_dentry(dentry: &DentryRef) -> Option<String> {
    path_for_dentry_by_mount_path(dentry, false)
}

fn path_for_dentry_by_mount_path(dentry: &DentryRef, prefer_longest: bool) -> Option<String> {
    let table = current_mount_table();
    let mounts = table.by_path.lock();
    let mut best: Option<(usize, String)> = None;
    for (path, mount) in mounts.iter() {
        if let Some(components) = dentry_components_below_root(dentry, &mount.root) {
            let candidate = join_mount_path(path, &components);
            if mount_path_is_better(best.as_ref(), path.len(), prefer_longest) {
                best = Some((path.len(), candidate));
            }
        }
        if mount
            .mountpoint
            .lock()
            .as_ref()
            .is_some_and(|mp| Arc::ptr_eq(mp, dentry))
        {
            if mount_path_is_better(best.as_ref(), path.len(), prefer_longest) {
                best = Some((path.len(), path.clone()));
            }
        }
    }
    best.map(|(_, path)| path)
}

fn mount_path_is_better(
    best: Option<&(usize, String)>,
    candidate_len: usize,
    prefer_longest: bool,
) -> bool {
    mount_path_len_is_better(
        best.map(|(best_len, _)| *best_len),
        candidate_len,
        prefer_longest,
    )
}

fn mount_path_len_is_better(
    best_len: Option<usize>,
    candidate_len: usize,
    prefer_longest: bool,
) -> bool {
    best_len.is_none_or(|best_len| {
        if prefer_longest {
            candidate_len > best_len
        } else {
            candidate_len < best_len
        }
    })
}

fn dentry_components_below_root(dentry: &DentryRef, root: &DentryRef) -> Option<Vec<String>> {
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

/// Render a namespace-global pathname from a Linux-style struct path.
/// Mount-root transitions use the covered mountpoint dentry rather than a
/// nonexistent dentry parent link.
pub fn namespace_path(path: &VfsPath) -> Option<String> {
    let root = namespace_root_path()?;
    path_between(&root, path)
}

/// Return `path` relative to `root`, following mount-parent transitions.
pub fn path_between(root: &VfsPath, path: &VfsPath) -> Option<String> {
    let mut components = Vec::new();
    let mut current = path.clone();
    loop {
        if current.equal(root) {
            components.reverse();
            return Some(join_mount_path("/", &components));
        }

        if Arc::ptr_eq(&current.dentry, &current.mount.root) {
            let parent = current.mount.parent.lock().clone()?;
            let mountpoint = current.mount.mountpoint.lock().clone()?;
            current = VfsPath::new(parent, mountpoint);
            continue;
        }

        if current.dentry.name != "/" && !current.dentry.name.is_empty() {
            components.push(current.dentry.name.clone());
        }
        let parent = current.dentry.parent.lock().clone()?;
        current.dentry = parent;
    }
}

fn join_mount_path(mount_path: &str, components: &[String]) -> String {
    if components.is_empty() {
        return String::from(mount_path);
    }
    let mut path = if mount_path == "/" {
        String::from("/")
    } else {
        let mut path = String::from(mount_path);
        path.push('/');
        path
    };
    for (idx, component) in components.iter().enumerate() {
        if idx > 0 {
            path.push('/');
        }
        path.push_str(component);
    }
    path
}

pub fn resolve_path(path: &str) -> Option<(Arc<Mount>, DentryRef)> {
    resolve_path_with_links(path, false).ok()
}

pub fn resolve_path_follow(path: &str) -> Result<(Arc<Mount>, DentryRef), i32> {
    resolve_path_with_links(path, true)
}

pub fn resolve_path_nofollow(path: &str) -> Result<(Arc<Mount>, DentryRef), i32> {
    resolve_path_with_links(path, false)
}

/// Resolve a path from a lookup context while honoring openat2 resolve flags
/// and performing mount traversal after every component.
pub fn resolve_path_at(
    ctx: &LookupCtx,
    path: &str,
    follow_final: bool,
) -> Result<(Arc<Mount>, DentryRef), i32> {
    let absolute = path.starts_with('/');
    if absolute && ctx.resolve & RESOLVE_BENEATH != 0 {
        return Err(EINVAL);
    }
    let root = match ctx.root_path.clone() {
        Some(path) => path,
        None => lookup_start_path(ctx.root.clone())?,
    };
    let start = if absolute {
        root.clone()
    } else {
        match ctx.start_path.clone() {
            Some(path) => path,
            None => lookup_start_path(ctx.start.clone())?,
        }
    };
    walk_path(&root, start, path, follow_final, ctx.resolve).map(|path| (path.mount, path.dentry))
}

fn lookup_start_mount(dentry: DentryRef) -> Result<(Arc<Mount>, DentryRef), i32> {
    if let Some(mount) = mounted_root_for_dentry(&dentry) {
        return Ok((mount.clone(), mount.root.clone()));
    }
    let mount = containing_mount_for_dentry(&dentry)
        .or_else(rootfs)
        .ok_or(EINVAL)?;
    Ok((mount, dentry))
}

fn lookup_start_path(dentry: DentryRef) -> Result<VfsPath, i32> {
    let (mount, dentry) = lookup_start_mount(dentry)?;
    Ok(VfsPath::new(mount, dentry))
}

fn resolve_path_with_links(path: &str, follow_final: bool) -> Result<(Arc<Mount>, DentryRef), i32> {
    if let Some(fd_path) = crate::fs::proc::fd::current_fd_path_from_proc_path(path) {
        let fd_path = fd_path?;
        return resolve_path_with_links(&fd_path, follow_final);
    }

    let namespace_root = namespace_root_path().ok_or(EINVAL)?;
    let (lookup_root, lookup_pwd) = crate::fs::fs_struct::current_root_and_pwd_paths()
        .filter(|(root, pwd)| {
            mount_is_in_namespace(&root.mount, &namespace_root.mount)
                && mount_is_in_namespace(&pwd.mount, &namespace_root.mount)
        })
        .unwrap_or_else(|| (namespace_root.clone(), namespace_root));
    let start = if path.starts_with('/') {
        lookup_root.clone()
    } else {
        lookup_pwd
    };
    walk_path(&lookup_root, start, path, follow_final, 0).map(|path| (path.mount, path.dentry))
}

fn mount_is_in_namespace(mount: &Arc<Mount>, namespace_root: &Arc<Mount>) -> bool {
    let mut current = Some(mount.clone());
    while let Some(candidate) = current {
        if Arc::ptr_eq(&candidate, namespace_root) {
            return true;
        }
        current = candidate.parent.lock().clone();
    }
    false
}

fn walk_path(
    root: &VfsPath,
    mut current: VfsPath,
    path: &str,
    follow_final: bool,
    resolve: u64,
) -> Result<VfsPath, i32> {
    let beneath_boundary = (resolve & RESOLVE_BENEATH != 0).then(|| current.clone());
    let mut parts = path_components(path);
    let mut index = 0usize;
    let mut symlink_follows = 0usize;

    while index < parts.len() {
        let comp = parts[index].as_str();
        index += 1;
        if comp == "." {
            continue;
        }
        if comp == ".." {
            if beneath_boundary
                .as_ref()
                .is_some_and(|boundary| current.equal(boundary))
            {
                return Err(EINVAL);
            }
            if current.equal(root) {
                continue;
            }
            let old_mount = current.mount.clone();
            current = follow_dotdot(current);
            if resolve & RESOLVE_NO_XDEV != 0 && !Arc::ptr_eq(&old_mount, &current.mount) {
                return Err(EXDEV);
            }
            continue;
        }

        let parent = current.clone();
        let next = lookup_child(&parent.dentry, comp)?;
        let is_last = index == parts.len();

        if let Some(inode) = next.inode() {
            if inode.kind == InodeKind::Symlink {
                if resolve & (RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS) != 0 {
                    return Err(ELOOP);
                }
                if is_last && !follow_final {
                    current = VfsPath::new(parent.mount, next);
                    continue;
                }
                symlink_follows += 1;
                if symlink_follows > MAX_SYMLINK_FOLLOWS {
                    return Err(ELOOP);
                }
                let target = read_symlink_target(&inode)?;
                let mut next_parts = path_components(&target);
                next_parts.extend_from_slice(&parts[index..]);
                if target.starts_with('/') {
                    if resolve & RESOLVE_BENEATH != 0 {
                        return Err(EINVAL);
                    }
                    current = root.clone();
                } else {
                    current = parent;
                }
                parts = next_parts;
                index = 0;
                continue;
            }
        }

        let (mount, dentry) = descend_mount(parent.mount.clone(), next);
        if resolve & RESOLVE_NO_XDEV != 0 && !Arc::ptr_eq(&mount, &parent.mount) {
            return Err(EXDEV);
        }
        current = VfsPath::new(mount, dentry);
    }

    Ok(current)
}

fn follow_dotdot(mut path: VfsPath) -> VfsPath {
    while Arc::ptr_eq(&path.dentry, &path.mount.root) {
        let Some(parent) = path.mount.parent.lock().clone() else {
            return path;
        };
        let Some(mountpoint) = path.mount.mountpoint.lock().clone() else {
            return path;
        };
        path = VfsPath::new(parent, mountpoint);
    }
    let parent = path.dentry.parent.lock().clone();
    if let Some(parent) = parent {
        path.dentry = parent;
    }
    path
}

fn path_components(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|c| !c.is_empty())
        .map(String::from)
        .collect()
}

fn lookup_child(parent: &DentryRef, name: &str) -> Result<DentryRef, i32> {
    if let Some(dentry) = super::dcache::d_lookup(parent, name) {
        if dentry.inode().is_none() {
            let dir_inode = parent.inode().ok_or(ENOENT)?;
            let lookup = dir_inode.ops.lookup.ok_or(ENOENT)?;
            match lookup(&dir_inode, name) {
                Ok(inode) => {
                    dentry.instantiate(inode);
                    return Ok(dentry);
                }
                Err(ENOENT) => return Err(ENOENT),
                Err(errno) => return Err(errno),
            }
        }
        return Ok(dentry);
    }
    let dir_inode = parent.inode().ok_or(ENOENT)?;
    let lookup = dir_inode.ops.lookup.ok_or(ENOENT)?;
    let child_inode = match lookup(&dir_inode, name) {
        Ok(inode) => inode,
        Err(ENOENT) => {
            super::dcache::d_cache_negative(parent, name);
            return Err(ENOENT);
        }
        Err(errno) => return Err(errno),
    };
    let new_d = super::dcache::d_alloc_child(parent, name);
    new_d.instantiate(child_inode);
    Ok(new_d)
}

fn descend_mount(mut cur_mount: Arc<Mount>, mut cur_dentry: DentryRef) -> (Arc<Mount>, DentryRef) {
    loop {
        let mut found = None;
        let children = cur_mount.children.lock().clone();
        for child in children.iter().rev() {
            if let Some(mp) = child.mountpoint.lock().clone() {
                if Arc::ptr_eq(&mp, &cur_dentry) {
                    found = Some(child.clone());
                    break;
                }
            }
        }
        let Some(child) = found else {
            return (cur_mount, cur_dentry);
        };
        cur_dentry = child.root.clone();
        cur_mount = child;
    }
}

fn read_symlink_target(inode: &super::types::InodeRef) -> Result<String, i32> {
    let readlink = inode.ops.readlink.ok_or(EINVAL)?;
    let mut buf = alloc::vec![0u8; PATH_MAX];
    let len = readlink(inode, &mut buf)?;
    core::str::from_utf8(&buf[..len])
        .map(String::from)
        .map_err(|_| EINVAL)
}

/// Resolve a path through the mount tree.  Returns the dentry at the end of
/// the walk *after* mount-traversal substitutions (sub-mounts replace their
/// mountpoint dentry with the sub-mount's root dentry).
pub fn path_walk(path: &str) -> Option<DentryRef> {
    resolve_path_follow(path).ok().map(|(_, dentry)| dentry)
}

pub fn pivot_root_paths(new_root: &VfsPath, put_old: &VfsPath) -> Result<(VfsPath, VfsPath), i32> {
    let table = current_mount_table();
    let old_mount = table.root.lock().clone().ok_or(EINVAL)?;
    if Arc::ptr_eq(&old_mount, &new_root.mount)
        || !Arc::ptr_eq(&new_root.dentry, &new_root.mount.root)
        || !Arc::ptr_eq(&put_old.mount, &new_root.mount)
        || new_root.mount.parent.lock().is_none()
    {
        return Err(EINVAL);
    }
    let old_root = VfsPath::new(old_mount.clone(), old_mount.root.clone());
    let new_root = VfsPath::new(new_root.mount.clone(), new_root.mount.root.clone());

    let old_parent = new_root.mount.parent.lock().clone().ok_or(EINVAL)?;
    let old_mountpoint = new_root.mount.mountpoint.lock().clone().ok_or(EINVAL)?;
    old_parent
        .children
        .lock()
        .retain(|mount| !Arc::ptr_eq(mount, &new_root.mount));
    if !old_parent.children.lock().iter().any(|mount| {
        mount
            .mountpoint
            .lock()
            .as_ref()
            .is_some_and(|mountpoint| Arc::ptr_eq(mountpoint, &old_mountpoint))
    }) {
        old_mountpoint
            .flags
            .fetch_and(!DCACHE_MOUNTED, Ordering::AcqRel);
    }

    *new_root.mount.parent.lock() = None;
    *new_root.mount.mountpoint.lock() = None;
    *old_mount.parent.lock() = Some(new_root.mount.clone());
    *old_mount.mountpoint.lock() = Some(put_old.dentry.clone());
    put_old
        .dentry
        .flags
        .fetch_or(DCACHE_MOUNTED, Ordering::AcqRel);

    // The pivot_root(".", ".") container-runtime idiom keeps the old root
    // reachable through an open fd/current path until MNT_DETACH, but it must
    // not cover the new namespace root during ordinary absolute lookup.
    let dot_pivot = put_old.equal(&new_root);
    if !dot_pivot {
        new_root.mount.children.lock().push(old_mount.clone());
    }

    let mut by_path = BTreeMap::new();
    register_mount_tree(&new_root.mount, "/", &mut by_path);
    if dot_pivot {
        // Keep the detached tree discoverable by dentry-only open file
        // descriptions until fchdir()+MNT_DETACH drops it.  The synthetic key
        // is internal to the mount registry and is never pathname-resolved.
        let hidden_path = alloc::format!("/.lupos-pivot-old-{}", old_mount.id);
        register_mount_tree(&old_mount, &hidden_path, &mut by_path);
    }
    *table.root.lock() = Some(new_root.mount.clone());
    *table.by_path.lock() = by_path;
    notify_mount_change();
    Ok((old_root, new_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::{d_alloc_child, d_lookup};
    use crate::fs::super_block::mount_fs;
    use crate::include::uapi::mount::{MS_BIND, MS_MOVE, MS_RDONLY, MS_REC, MS_REMOUNT};

    fn reset_mount_state() {
        *MOUNTS.root.lock() = None;
        MOUNTS.by_path.lock().clear();
    }

    fn mkdir_dentry(parent: &DentryRef, name: &str) -> DentryRef {
        let parent_inode = parent.inode().expect("parent inode");
        let mkdir = parent_inode.ops.mkdir.expect("mkdir op");
        let child_inode = mkdir(&parent_inode, name, 0o755).expect("mkdir");
        let child = d_alloc_child(parent, name);
        child.instantiate(child_inode);
        child
    }

    #[test]
    fn kernel_remount_mountpoint_updates_root_without_syscall_usercopy() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", MS_RDONLY, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root, MS_RDONLY as u32));

        assert!(rootfs().expect("rootfs").is_readonly());
        remount_mountpoint("/", MS_REMOUNT).expect("remount root");
        assert!(!rootfs().expect("rootfs").is_readonly());
    }

    #[test]
    fn recursive_slave_propagation_accepts_silent_modifier() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root, 0));

        assert_eq!(
            unsafe {
                sys_mount(
                    core::ptr::null(),
                    b"/\0".as_ptr(),
                    core::ptr::null(),
                    MS_SLAVE | MS_REC | MS_SILENT,
                    core::ptr::null(),
                )
            },
            0
        );
    }

    #[test]
    fn bind_remount_with_empty_fstype_succeeds() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root, 0));

        let target = b"/\0";
        assert_eq!(
            unsafe {
                sys_mount(
                    core::ptr::null(),
                    target.as_ptr(),
                    core::ptr::null(),
                    MS_REMOUNT | MS_BIND | MS_RDONLY,
                    core::ptr::null(),
                )
            },
            0
        );
        assert!(rootfs().expect("rootfs").is_readonly());
    }

    #[test]
    fn resolve_path_revalidates_negative_dentry_after_filesystem_create() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        let etc = mkdir_dentry(&root, "etc");

        assert_eq!(resolve_path_follow("/etc/ld.so.cache").err(), Some(ENOENT));
        let stale = d_lookup(&etc, "ld.so.cache").expect("negative dentry");
        assert!(stale.is_negative());

        let etc_inode = etc.inode().expect("/etc inode");
        let create = etc_inode.ops.create.expect("create op");
        create(&etc_inode, "ld.so.cache", 0o644).expect("create behind stale dentry");

        let (_mount, resolved) = resolve_path_follow("/etc/ld.so.cache").expect("revalidated");
        assert!(alloc::sync::Arc::ptr_eq(&resolved, &stale));
        assert!(!stale.is_negative());
    }

    #[test]
    fn plain_bind_mount_with_empty_fstype_attaches_source_subtree() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        mkdir_dentry(&root, "source");
        mkdir_dentry(&root, "target");

        let source = b"/source\0";
        let target = b"/target\0";
        assert_eq!(
            unsafe {
                sys_mount(
                    source.as_ptr(),
                    target.as_ptr(),
                    core::ptr::null(),
                    MS_BIND,
                    core::ptr::null(),
                )
            },
            0
        );

        let (_mount, dentry) = resolve_path_follow("/target").expect("target");
        assert_eq!(dentry.name, "source");
    }

    #[test]
    fn move_mount_rejects_a_source_that_is_not_a_mount_root() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        mkdir_dentry(&root, "source");
        mkdir_dentry(&root, "target");

        let source = b"/source\0";
        let target = b"/target\0";
        assert_eq!(
            unsafe {
                sys_mount(
                    source.as_ptr(),
                    target.as_ptr(),
                    core::ptr::null(),
                    MS_MOVE,
                    core::ptr::null(),
                )
            },
            -(EINVAL as i64)
        );
    }

    #[test]
    fn remount_updates_existing_mount_without_replacing_tree() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        mkdir_dentry(&root, "run");

        assert_eq!(
            unsafe {
                sys_mount(
                    b"tmpfs\0".as_ptr(),
                    b"/run\0".as_ptr(),
                    b"tmpfs\0".as_ptr(),
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        let (_run_mount, run_root) = resolve_path_follow("/run").expect("/run");
        let systemd = mkdir_dentry(&run_root, "systemd");
        mkdir_dentry(&systemd, "mount-rootfs");

        assert_eq!(
            unsafe {
                sys_mount(
                    b"tmpfs\0".as_ptr(),
                    b"/run\0".as_ptr(),
                    b"tmpfs\0".as_ptr(),
                    MS_REMOUNT | MS_RDONLY,
                    core::ptr::null(),
                )
            },
            0
        );

        assert!(
            resolve_path_follow("/run/systemd/mount-rootfs").is_ok(),
            "remount must preserve tmpfs contents"
        );
        assert!(lookup_mount("/run").expect("/run mount").is_readonly());
    }

    #[test]
    fn cgroup_remount_preserves_service_dirs_and_files() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        let sys = mkdir_dentry(&root, "sys");
        let fs = mkdir_dentry(&sys, "fs");
        mkdir_dentry(&fs, "cgroup");

        assert_eq!(
            unsafe {
                sys_mount(
                    b"cgroup2\0".as_ptr(),
                    b"/sys/fs/cgroup\0".as_ptr(),
                    b"cgroup2\0".as_ptr(),
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        let (_cg_mount, cg_root) = resolve_path_follow("/sys/fs/cgroup").expect("cgroup root");
        let system = mkdir_dentry(&cg_root, "system.slice");
        mkdir_dentry(&system, "systemd-networkd.service");

        assert_eq!(
            unsafe {
                sys_mount(
                    b"cgroup2\0".as_ptr(),
                    b"/sys/fs/cgroup\0".as_ptr(),
                    b"cgroup2\0".as_ptr(),
                    MS_REMOUNT | MS_RDONLY,
                    core::ptr::null(),
                )
            },
            0
        );

        assert!(
            resolve_path_follow(
                "/sys/fs/cgroup/system.slice/systemd-networkd.service/cgroup.subtree_control"
            )
            .is_ok(),
            "remount must not replace the cgroupfs hierarchy"
        );
        assert!(
            lookup_mount("/sys/fs/cgroup")
                .expect("cgroup mount")
                .is_readonly()
        );
    }

    #[test]
    fn repeated_mounts_stack_on_mountpoint_not_previous_root() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        mkdir_dentry(&root, "sys");

        do_mount("sysfs", "sysfs", "/sys", 0, "").expect("first sysfs");
        do_mount("sysfs", "sysfs", "/sys", 0, "").expect("stacked sysfs");
        do_mount("cgroup2", "cgroup2", "/sys/fs/cgroup", 0, "").expect("first cgroup");
        do_mount("cgroup2", "cgroup2", "/sys/fs/cgroup", 0, "").expect("second cgroup");
        do_mount("cgroup2", "cgroup2", "/sys/fs/cgroup", 0, "").expect("third cgroup");

        let (_mount, cgroup_root) = resolve_path_follow("/sys/fs/cgroup").expect("cgroup root");
        let inode = cgroup_root.inode().expect("cgroup inode");
        let sb = inode.sb.lock().clone().expect("cgroup sb");
        assert_eq!(sb.fs_name, "cgroup2");
        assert!(inode.ops.mkdir.expect("cgroup mkdir")(&inode, "systemd", 0o755).is_ok());
    }

    #[test]
    fn relative_recursive_self_bind_does_not_relock_mountpoint() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        mkdir_dentry(&root, "sandbox-root");

        let mount = do_bind_mount("sandbox-root", "sandbox-root", MS_BIND | MS_REC)
            .expect("recursive self-bind");
        assert_eq!(
            stable_path_for_dentry(&mount.root).as_deref(),
            Some("/sandbox-root")
        );
    }

    #[test]
    fn pivot_root_reparents_old_tree_below_new_root() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        let old_mount = Mount::alloc(sb, root.clone(), 0);
        set_rootfs(old_mount.clone());
        mkdir_dentry(&root, "tmp");
        mkdir_dentry(&root, "dev");
        do_mount("tmpfs", "tmpfs", "/dev", 0, "").expect("old devtmpfs");
        let (_, dev_root) = resolve_path_follow("/dev").expect("devtmpfs root");
        mkdir_dentry(&dev_root, "full");
        do_mount("tmpfs", "tmpfs", "/tmp", 0, "").expect("sandbox tmpfs");
        let (_, tmp_root) = resolve_path_follow("/tmp").expect("tmpfs root");
        mkdir_dentry(&tmp_root, "newroot");
        mkdir_dentry(&tmp_root, "oldroot");

        let (new_mount, new_dentry) = resolve_path_follow("/tmp").expect("new root");
        let (put_mount, put_dentry) = resolve_path_follow("/tmp/oldroot").expect("put_old");
        let (old_path, new_path) = pivot_root_paths(
            &VfsPath::new(new_mount, new_dentry),
            &VfsPath::new(put_mount, put_dentry),
        )
        .expect("pivot root");

        assert!(Arc::ptr_eq(&old_path.mount, &old_mount));
        assert!(Arc::ptr_eq(
            &rootfs().expect("namespace root"),
            &new_path.mount
        ));
        let (mounted_old, _) = resolve_path_follow("/oldroot").expect("old root visible");
        assert!(Arc::ptr_eq(&mounted_old, &old_mount));
        assert!(
            resolve_path_follow("/oldroot/dev/full").is_ok(),
            "old-root submounts remain reachable after pivot"
        );
    }

    #[test]
    fn dot_dot_pivot_keeps_old_root_fd_reachable_but_not_path_covering() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        let old_mount = Mount::alloc(sb, root.clone(), 0);
        set_rootfs(old_mount.clone());
        let newroot = mkdir_dentry(&root, "newroot");
        let new_mount =
            do_bind_mount("/newroot", "/newroot", MS_BIND | MS_REC).expect("newroot bind");
        let pivot = VfsPath::new(new_mount.clone(), newroot);

        let (old_path, new_path) = pivot_root_paths(&pivot, &pivot).expect("dot pivot");
        assert!(Arc::ptr_eq(&old_path.mount, &old_mount));
        assert!(Arc::ptr_eq(
            &rootfs().expect("namespace root"),
            &new_path.mount
        ));
        assert!(new_path.mount.children.lock().is_empty());
        assert!(old_path.mount.parent.lock().is_some());
        assert!(Arc::ptr_eq(
            &VfsPath::for_dentry(root)
                .expect("open old-root dentry remains mount-qualified")
                .mount,
            &old_mount
        ));
    }

    #[test]
    fn recursive_root_bind_clones_run_submount_without_hiding_host_run() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        let usr = mkdir_dentry(&root, "usr");
        let lib = mkdir_dentry(&usr, "lib");
        let systemd_dir = mkdir_dentry(&lib, "systemd");
        let executor = mkdir_dentry(&systemd_dir, "systemd-executor");
        mkdir_dentry(&root, "run");

        do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs /run");
        let (_run_mount, run_root) = resolve_path_follow("/run").expect("/run");
        let systemd = mkdir_dentry(&run_root, "systemd");
        mkdir_dentry(&systemd, "mount-rootfs");
        let credentials = mkdir_dentry(&run_root, "credentials");
        mkdir_dentry(&credentials, "systemd-networkd.service");

        assert_eq!(
            unsafe {
                sys_mount(
                    b"/\0".as_ptr(),
                    b"/run/systemd/mount-rootfs\0".as_ptr(),
                    core::ptr::null(),
                    MS_BIND | MS_REC,
                    core::ptr::null(),
                )
            },
            0
        );
        assert!(
            resolve_path_follow(
                "/run/systemd/mount-rootfs/run/credentials/systemd-networkd.service"
            )
            .is_ok(),
            "recursive bind must carry the /run submount into the service root"
        );

        do_mount("tmpfs", "tmpfs", "/run/systemd/mount-rootfs/run", 0, "")
            .expect("private service /run");
        assert!(
            resolve_path_follow("/run/credentials/systemd-networkd.service").is_ok(),
            "overmounting the service root's /run must not hide host /run credentials"
        );
        assert!(
            resolve_path_follow(
                "/run/systemd/mount-rootfs/run/credentials/systemd-networkd.service"
            )
            .is_err(),
            "the service root gets a fresh /run after the overmount"
        );
        assert_eq!(
            stable_path_for_dentry(&executor).as_deref(),
            Some("/usr/lib/systemd/systemd-executor"),
            "proc-fd exec paths should prefer the stable host alias"
        );
    }

    #[test]
    fn recursive_root_bind_skips_existing_target_submounts() {
        let _guard = TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs");
        let root = sb.root().expect("root");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        mkdir_dentry(&root, "tmp");
        mkdir_dentry(&root, "run");

        do_mount("tmpfs", "tmpfs", "/run", 0, "").expect("tmpfs /run");
        let (_run_mount, run_root) = resolve_path_follow("/run").expect("/run");
        let systemd = mkdir_dentry(&run_root, "systemd");
        mkdir_dentry(&systemd, "mount-rootfs");

        assert_eq!(
            unsafe {
                sys_mount(
                    b"/\0".as_ptr(),
                    b"/run/systemd/mount-rootfs\0".as_ptr(),
                    core::ptr::null(),
                    MS_BIND | MS_REC,
                    core::ptr::null(),
                )
            },
            0
        );
        do_mount("tmpfs", "tmpfs", "/run/systemd/mount-rootfs/tmp", 0, "")
            .expect("leaked service submount");

        assert_eq!(
            unsafe {
                sys_mount(
                    b"/\0".as_ptr(),
                    b"/run/systemd/mount-rootfs\0".as_ptr(),
                    core::ptr::null(),
                    MS_BIND | MS_REC,
                    core::ptr::null(),
                )
            },
            0,
            "a repeated root bind should ignore stale submounts below its target"
        );
        let (bound_run_mount, bound_run_root) =
            resolve_path_follow("/run/systemd/mount-rootfs/run").expect("bound /run");
        assert!(
            Arc::ptr_eq(&bound_run_mount.root, &bound_run_root),
            "the host /run submount must still be cloned into the service root"
        );
    }
}

#[allow(unused)]
fn _ensure_unused_errno() {
    let _ = EROFS;
}
