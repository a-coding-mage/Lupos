//! linux-parity: partial
//! linux-source: vendor/linux/fs/kernfs
//! linux-source: vendor/linux/fs/kernfs/dir.c
//! test-origin: linux:vendor/linux/fs/kernfs
//! kernfs — pseudo-filesystem skeleton shared by sysfs / cgroupfs / debugfs.
//!
//! Functional kernfs (dir/file nodes, show/store callbacks) backing
//! sysfs/cgroupfs/debugfs, using BTreeMap + per-node lock instead of Linux's
//! radix tree + per-node spinlock. Remaining work vs Linux for `complete`:
//! active references/draining, full inode/attr semantics, and the complete
//! `kernfs_ops`/`kernfs_syscall_ops` surface.
//!
//! Mirrors `vendor/linux/fs/kernfs/`.  Each `KernfsNode` is either a directory
//! (children indexed by name) or a file with `show`/`store` callbacks.  The
//! real Linux design uses radix trees + per-node spinlocks; lupos uses a
//! BTreeMap and one parent lock per node.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EEXIST, EINVAL, ENOENT, ENOSYS, ENOTDIR, EROFS};
use crate::include::uapi::stat::{S_IFDIR, S_IFLNK, S_IFREG};
use crate::kernel::sched::wait::WaitQueueHead;

use super::types::{
    FileRef, Inode, InodeKind, InodePrivate, InodeRef, SuperBlockRef, init_inode_owner,
};

pub mod dir;
pub mod file;
pub mod inode;
pub mod mount;
pub mod symlink;

/// Read-callback: write content into `buf`, return bytes written.
pub type ShowFn = fn(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32>;
/// Write-callback: consume `buf`, return bytes accepted.
pub type StoreFn = fn(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32>;
/// File-local read callback, used by Linux kernfs/debugfs files whose state is
/// stored in `struct file::private_data`.
pub type OpenReadFn =
    fn(file: &FileRef, node: &Arc<KernfsNode>, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32>;
/// File-local write callback.
pub type OpenWriteFn =
    fn(file: &FileRef, node: &Arc<KernfsNode>, buf: &[u8], pos: &mut u64) -> Result<usize, i32>;
/// File-local release callback.
pub type OpenReleaseFn = fn(file: FileRef, node: &Arc<KernfsNode>);
pub type DynamicLookupFn = fn(dir: &InodeRef, name: &str) -> Result<InodeRef, i32>;
pub type DynamicReaddirFn = fn(file: &FileRef) -> Result<Option<(String, u64, InodeKind)>, i32>;
pub type DynamicReadlinkFn = fn(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32>;

pub enum KernfsKind {
    Dir,
    File {
        show: Option<ShowFn>,
        store: Option<StoreFn>,
        read: Option<OpenReadFn>,
        write: Option<OpenWriteFn>,
        release: Option<OpenReleaseFn>,
    },
    Symlink {
        target: String,
    },
    DynamicSymlink {
        readlink: DynamicReadlinkFn,
    },
}

pub struct KernfsNode {
    pub name: String,
    pub kind: KernfsKind,
    pub parent: Mutex<Weak<KernfsNode>>,
    pub children: Mutex<BTreeMap<String, Arc<KernfsNode>>>,
    pub mode: u32,
    pub ino: u64,
    /// Filesystem-private payload (e.g., the cgroup `TaskGroup` pointer).
    pub priv_ptr: AtomicU64,
    /// Vendor `kernfs_open_node::event` generation and poll waitqueue.
    notify_event: AtomicU64,
    notify_wait: WaitQueueHead,
    pub dynamic_lookup: Option<DynamicLookupFn>,
    pub dynamic_readdir: Option<DynamicReaddirFn>,
}

impl KernfsNode {
    pub fn new_dir(name: &str, mode: u32) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            kind: KernfsKind::Dir,
            parent: Mutex::new(Weak::new()),
            children: Mutex::new(BTreeMap::new()),
            mode: mode | S_IFDIR,
            ino: alloc_kn_ino(),
            priv_ptr: AtomicU64::new(0),
            notify_event: AtomicU64::new(0),
            notify_wait: WaitQueueHead::new(),
            dynamic_lookup: None,
            dynamic_readdir: None,
        })
    }
    pub fn new_dynamic_dir(
        name: &str,
        mode: u32,
        lookup: Option<DynamicLookupFn>,
        readdir: Option<DynamicReaddirFn>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            kind: KernfsKind::Dir,
            parent: Mutex::new(Weak::new()),
            children: Mutex::new(BTreeMap::new()),
            mode: mode | S_IFDIR,
            ino: alloc_kn_ino(),
            priv_ptr: AtomicU64::new(0),
            notify_event: AtomicU64::new(0),
            notify_wait: WaitQueueHead::new(),
            dynamic_lookup: lookup,
            dynamic_readdir: readdir,
        })
    }
    pub fn new_file(
        name: &str,
        mode: u32,
        show: Option<ShowFn>,
        store: Option<StoreFn>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            kind: KernfsKind::File {
                show,
                store,
                read: None,
                write: None,
                release: None,
            },
            parent: Mutex::new(Weak::new()),
            children: Mutex::new(BTreeMap::new()),
            mode: mode | S_IFREG,
            ino: alloc_kn_ino(),
            priv_ptr: AtomicU64::new(0),
            notify_event: AtomicU64::new(0),
            notify_wait: WaitQueueHead::new(),
            dynamic_lookup: None,
            dynamic_readdir: None,
        })
    }

    pub fn new_file_with_open_ops(
        name: &str,
        mode: u32,
        show: Option<ShowFn>,
        store: Option<StoreFn>,
        read: Option<OpenReadFn>,
        write: Option<OpenWriteFn>,
        release: Option<OpenReleaseFn>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            kind: KernfsKind::File {
                show,
                store,
                read,
                write,
                release,
            },
            parent: Mutex::new(Weak::new()),
            children: Mutex::new(BTreeMap::new()),
            mode: mode | S_IFREG,
            ino: alloc_kn_ino(),
            priv_ptr: AtomicU64::new(0),
            notify_event: AtomicU64::new(0),
            notify_wait: WaitQueueHead::new(),
            dynamic_lookup: None,
            dynamic_readdir: None,
        })
    }

    pub fn new_symlink(name: &str, target: &str) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            kind: KernfsKind::Symlink {
                target: String::from(target),
            },
            parent: Mutex::new(Weak::new()),
            children: Mutex::new(BTreeMap::new()),
            mode: mode_for_symlink(),
            ino: alloc_kn_ino(),
            priv_ptr: AtomicU64::new(0),
            notify_event: AtomicU64::new(0),
            notify_wait: WaitQueueHead::new(),
            dynamic_lookup: None,
            dynamic_readdir: None,
        })
    }

    pub fn new_dynamic_symlink(name: &str, readlink: DynamicReadlinkFn) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            kind: KernfsKind::DynamicSymlink { readlink },
            parent: Mutex::new(Weak::new()),
            children: Mutex::new(BTreeMap::new()),
            mode: mode_for_symlink(),
            ino: alloc_kn_ino(),
            priv_ptr: AtomicU64::new(0),
            notify_event: AtomicU64::new(0),
            notify_wait: WaitQueueHead::new(),
            dynamic_lookup: None,
            dynamic_readdir: None,
        })
    }
}

fn mode_for_symlink() -> u32 {
    0o777 | S_IFLNK
}

static NEXT_KN_INO: AtomicU64 = AtomicU64::new(1);
fn alloc_kn_ino() -> u64 {
    NEXT_KN_INO.fetch_add(1, Ordering::AcqRel)
}

pub fn add_child(parent: &Arc<KernfsNode>, child: Arc<KernfsNode>) {
    *child.parent.lock() = Arc::downgrade(parent);
    parent.children.lock().insert(child.name.clone(), child);
}

pub fn lookup(parent: &Arc<KernfsNode>, name: &str) -> Option<Arc<KernfsNode>> {
    parent
        .children
        .lock()
        .iter()
        .find(|(child_name, _)| names_eq(child_name.as_str(), name))
        .map(|(_, node)| node.clone())
}

fn names_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Build a `Vec<(name, ino, kind)>` snapshot for readdir.
pub fn list(parent: &Arc<KernfsNode>) -> Vec<(String, u64, InodeKind)> {
    parent
        .children
        .lock()
        .iter()
        .map(|(k, v)| {
            let kind = match &v.kind {
                KernfsKind::Dir => InodeKind::Directory,
                KernfsKind::File { .. } => InodeKind::Regular,
                KernfsKind::Symlink { .. } | KernfsKind::DynamicSymlink { .. } => {
                    InodeKind::Symlink
                }
            };
            (k.clone(), v.ino, kind)
        })
        .collect()
}

/// Bridge into the VFS — wrap a kernfs node in an inode with kernfs-aware ops.
///
/// The inode permanently owns one strong Arc reference to `node`; the inode's
/// private payload holds a raw pointer that we promote via
/// `Arc::increment_strong_count` whenever we need to dispatch through the
/// node.  Inode drop must run `decrement_strong_count` — for M38–M42 inodes
/// live until the FS is unmounted, so we leak that reference deliberately
/// (matches Linux kernfs semantics: nodes only go away when the kernfs root
/// itself is dismantled).
pub fn inode_for_node(sb: &SuperBlockRef, node: Arc<KernfsNode>) -> InodeRef {
    let kind = match &node.kind {
        KernfsKind::Dir => InodeKind::Directory,
        KernfsKind::File { .. } => InodeKind::Regular,
        KernfsKind::Symlink { .. } | KernfsKind::DynamicSymlink { .. } => InodeKind::Symlink,
    };
    let raw = Arc::into_raw(node);
    let inode = Inode::new(
        unsafe { (*raw).ino },
        kind,
        unsafe { (*raw).mode },
        match kind {
            InodeKind::Directory => &KERNFS_DIR_INODE_OPS,
            InodeKind::Symlink => &KERNFS_SYMLINK_INODE_OPS,
            _ => &KERNFS_FILE_INODE_OPS,
        },
        match kind {
            InodeKind::Directory => &KERNFS_DIR_FILE_OPS,
            InodeKind::Symlink => &KERNFS_SYMLINK_FILE_OPS,
            _ => &KERNFS_FILE_FILE_OPS,
        },
        InodePrivate::Opaque(raw as usize),
    );
    *inode.sb.lock() = Some(sb.clone());
    inode
}

pub(crate) fn node_from_inode(inode: &InodeRef) -> Arc<KernfsNode> {
    let raw = match &inode.private {
        InodePrivate::Opaque(p) => *p as *const KernfsNode,
        _ => panic!("kernfs: inode missing kernfs-node payload"),
    };
    unsafe {
        Arc::increment_strong_count(raw);
        Arc::from_raw(raw)
    }
}

// ── Op vtables ────────────────────────────────────────────────────────────

use super::ops::{FileOps, InodeOps};

pub static KERNFS_DIR_INODE_OPS: InodeOps = InodeOps {
    name: "kernfs_dir",
    lookup: Some(kernfs_lookup),
    create: None,
    mkdir: Some(kernfs_mkdir),
    unlink: None,
    rmdir: Some(kernfs_rmdir),
    rename: None,
    symlink: None,
    readlink: None,
    setattr: None,
};

pub static KERNFS_FILE_INODE_OPS: InodeOps = InodeOps {
    name: "kernfs_file",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: None,
    setattr: None,
};

pub static KERNFS_SYMLINK_INODE_OPS: InodeOps = InodeOps {
    name: "kernfs_symlink",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: Some(kernfs_readlink),
    setattr: None,
};

pub static KERNFS_DIR_FILE_OPS: FileOps = FileOps {
    name: "kernfs_dir",
    read: None,
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: Some(kernfs_readdir),
};

pub static KERNFS_FILE_FILE_OPS: FileOps = FileOps {
    name: "kernfs_file",
    read: Some(kernfs_read),
    write: Some(kernfs_write),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: Some(kernfs_poll),
    ioctl: None,
    mmap: None,
    release: Some(kernfs_release),
    readdir: None,
};

pub static KERNFS_SYMLINK_FILE_OPS: FileOps = FileOps {
    name: "kernfs_symlink",
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

fn kernfs_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let node = node_from_inode(dir);
    if let Some(child) = lookup(&node, name) {
        let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
        let inode = inode_for_node(&sb, child);
        if sb.fs_name == "cgroup2" {
            // A delegated cgroup subtree must remain writable by its owner.
            // Kernfs creates every interface inode lazily, so inherit the
            // containing cgroup's ownership instead of recreating control
            // files as root:root on each lookup.
            inode
                .uid
                .store(dir.uid.load(Ordering::Acquire), Ordering::Release);
            inode
                .gid
                .store(dir.gid.load(Ordering::Acquire), Ordering::Release);
        }
        return Ok(inode);
    }
    if let Some(lookup) = node.dynamic_lookup {
        return lookup(dir, name);
    }
    Err(ENOENT)
}

fn kernfs_mkdir(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32> {
    let parent = node_from_inode(dir);
    if lookup(&parent, name).is_some() {
        return Err(EEXIST);
    }
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    if sb.fs_name != "cgroup2" {
        return Err(EROFS);
    }
    let child = crate::kernel::cgroup::new_cgroup_dir(name, mode);
    add_child(&parent, child.clone());
    crate::kernel::cgroup::register_cgroup_dir(&child, &sb);
    let inode = inode_for_node(&sb, child);
    // cgroup mkdir follows normal VFS ownership rules.  This is essential
    // for systemd's user-manager delegation: descendants created by uid 1000
    // and their cgroup.procs files must be writable by uid 1000.
    init_inode_owner(&inode, Some(dir), mode | S_IFDIR);
    Ok(inode)
}

fn kernfs_rmdir(dir: &InodeRef, name: &str) -> Result<(), i32> {
    let parent = node_from_inode(dir);
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    if sb.fs_name != "cgroup2" {
        return Err(EROFS);
    }
    let mut children = parent.children.lock();
    let key = children
        .keys()
        .find(|child_name| names_eq(child_name.as_str(), name))
        .cloned()
        .ok_or(ENOENT)?;
    let child = children.get(&key).cloned().ok_or(ENOENT)?;
    if !matches!(child.kind, KernfsKind::Dir) {
        return Err(ENOTDIR);
    }
    let has_child_cgroup = child
        .children
        .lock()
        .values()
        .any(|node| matches!(node.kind, KernfsKind::Dir));
    if has_child_cgroup {
        return Err(EBUSY);
    }
    crate::kernel::cgroup::unregister_cgroup_dir(&child);
    children.remove(&key);
    Ok(())
}

fn kernfs_readdir(file: &super::types::FileRef) -> Result<Option<(String, u64, InodeKind)>, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let node = node_from_inode(&inode);
    if let Some(readdir) = node.dynamic_readdir {
        return readdir(file);
    }
    if let Some(dot) = crate::fs::libfs::synthetic_readdir_dot_entry(file)? {
        return Ok(Some(dot));
    }
    let snapshot = list(&node);
    let mut idx = file.pos.lock();
    let child_idx = idx.saturating_sub(2) as usize;
    if child_idx >= snapshot.len() {
        return Ok(None);
    }
    let entry = snapshot[child_idx].clone();
    *idx += 1;
    Ok(Some(entry))
}

fn kernfs_read(file: &super::types::FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let node = node_from_inode(&inode);
    if let KernfsKind::File {
        read: Some(read), ..
    } = &node.kind
    {
        return read(file, &node, buf, pos);
    }
    let show = match &node.kind {
        KernfsKind::File { show, .. } => (*show).ok_or(ENOSYS)?,
        _ => return Err(EINVAL),
    };
    // Render full content into a scratch then slice from `pos`.
    let mut scratch = alloc::vec![0u8; kernfs_read_scratch_len(&node)];
    let n = show(&node, &mut scratch)?;
    let start = (*pos as usize).min(n);
    let copy = (n - start).min(buf.len());
    buf[..copy].copy_from_slice(&scratch[start..start + copy]);
    *pos += copy as u64;
    Ok(copy)
}

fn kernfs_read_scratch_len(node: &KernfsNode) -> usize {
    match node.name.as_str() {
        "mounts" | "mountinfo" | "mountstats" | "maps" | "smaps" | "pagetypeinfo"
        | "kpageflags" => 128 * 1024,
        "swaps" => 4 * 1024,
        _ => 16 * 1024,
    }
}

fn kernfs_poll(
    file: &super::types::FileRef,
    table: Option<&mut crate::fs::select::PollTable>,
) -> u32 {
    let Some(inode) = file.inode() else {
        return 0;
    };
    let node = node_from_inode(&inode);
    if matches!(node.name.as_str(), "mounts" | "mountinfo" | "mountstats") {
        return crate::fs::proc_namespace::poll_mount_table(file);
    }
    // vendor/linux/fs/kernfs/file.c::kernfs_generic_poll: register before
    // sampling the generation so notify cannot be lost between the two.
    crate::fs::select::poll_wait(file, &node.notify_wait, table);
    if file.poll_event.load(Ordering::Acquire) != node.notify_event.load(Ordering::Acquire) {
        return (crate::fs::select::POLLERR | crate::fs::select::POLLPRI) as u32;
    }
    0
}

pub(crate) fn initialize_poll_event(file: &FileRef) {
    let Some(inode) = file.inode() else {
        return;
    };
    let node = node_from_inode(&inode);
    file.poll_event
        .store(node.notify_event.load(Ordering::Acquire), Ordering::Release);
}

/// Vendor `kernfs_notify()`: immediately advance the poll generation and wake
/// registered poll/epoll waiters, then emit the matching fsnotify modification
/// event for inotify users.
pub fn notify(dentry: &super::types::DentryRef) {
    let Some(inode) = dentry.inode() else {
        return;
    };
    let node = node_from_inode(&inode);
    let Some(sb) = inode.sb.lock().clone() else {
        return;
    };
    notify_node(&node, &sb);
}

/// Vendor `kernfs_notify()` through the stable node owned by the subsystem.
///
/// Cgroup exit notifications run after task filesystem teardown, so Linux
/// retains the `kernfs_node` instead of resolving the control file by path.
pub fn notify_node(node: &Arc<KernfsNode>, sb: &super::types::SuperBlockRef) {
    node.notify_event.fetch_add(1, Ordering::AcqRel);
    node.notify_wait.wake_up_all();
    crate::fs::inotify::notify_modify_identity(sb, node.ino);
}

pub fn consume_poll_event(file: &super::types::FileRef) {
    if file.fops.name != KERNFS_FILE_FILE_OPS.name {
        return;
    }
    let Some(inode) = file.inode() else {
        return;
    };
    let node = node_from_inode(&inode);
    if matches!(node.name.as_str(), "mounts" | "mountinfo" | "mountstats") {
        crate::fs::proc_namespace::consume_mount_table_poll(file);
    } else {
        file.poll_event
            .store(node.notify_event.load(Ordering::Acquire), Ordering::Release);
    }
}

fn kernfs_write(file: &super::types::FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let node = node_from_inode(&inode);
    if let KernfsKind::File {
        write: Some(write), ..
    } = &node.kind
    {
        return write(file, &node, buf, pos);
    }
    let store = match &node.kind {
        KernfsKind::File { store, .. } => (*store).ok_or(ENOSYS)?,
        _ => return Err(EINVAL),
    };
    let n = store(&node, buf)?;
    *pos += n as u64;
    Ok(n)
}

fn kernfs_release(file: super::types::FileRef) {
    let Some(inode) = file.inode() else {
        return;
    };
    let node = node_from_inode(&inode);
    if let KernfsKind::File {
        release: Some(release),
        ..
    } = &node.kind
    {
        release(file, &node);
    }
}

fn kernfs_readlink(inode: &InodeRef, buf: &mut [u8]) -> Result<usize, i32> {
    let node = node_from_inode(inode);
    match &node.kind {
        KernfsKind::Symlink { target } => {
            let n = target.len().min(buf.len());
            buf[..n].copy_from_slice(&target.as_bytes()[..n]);
            Ok(n)
        }
        KernfsKind::DynamicSymlink { readlink } => readlink(&node, buf),
        _ => Err(EINVAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn large_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
        let n = 8192.min(buf.len());
        buf[..n].fill(b'x');
        Ok(n)
    }

    #[test]
    fn kernfs_dir_file_and_symlink_nodes_round_trip() {
        let root = KernfsNode::new_dir("/", 0o755);
        let file = KernfsNode::new_file("value", 0o444, None, None);
        let link = KernfsNode::new_symlink("link", "value");
        add_child(&root, file);
        add_child(&root, link.clone());

        let entries = list(&root);
        assert!(entries.iter().any(|entry| entry.0 == "value"));
        assert!(entries.iter().any(|entry| entry.2 == InodeKind::Symlink));

        let sb = super::super::types::SuperBlock::alloc(
            "kernfs-test",
            0x6b65726e,
            &super::super::ops::NOOP_SUPER_OPS,
        );
        let inode = inode_for_node(&sb, link);
        let mut buf = [0u8; 16];
        let n = inode.ops.readlink.unwrap()(&inode, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"value");
    }

    #[test]
    fn kernfs_read_buffers_mountinfo_sized_virtual_files() {
        let node = KernfsNode::new_file("large", 0o444, Some(large_show), None);
        let sb = super::super::types::SuperBlock::alloc(
            "kernfs-test",
            0x6b65726e,
            &super::super::ops::NOOP_SUPER_OPS,
        );
        let inode = inode_for_node(&sb, node);
        let dentry = super::super::types::Dentry::new_negative("large");
        dentry.instantiate(inode);
        let file = super::super::types::File::new(dentry, 0, 0, &KERNFS_FILE_FILE_OPS);

        let mut pos = 0;
        let mut buf = [0u8; 5000];
        let n = kernfs_read(&file, &mut buf, &mut pos).expect("read");
        assert_eq!(n, buf.len());
        assert!(buf.iter().all(|b| *b == b'x'));
    }

    #[test]
    fn kernfs_read_uses_compact_scratch_for_proc_swaps() {
        let swaps = KernfsNode::new_file("swaps", 0o444, Some(large_show), None);
        let mountinfo = KernfsNode::new_file("mountinfo", 0o444, Some(large_show), None);

        assert_eq!(kernfs_read_scratch_len(&swaps), 4 * 1024);
        assert_eq!(kernfs_read_scratch_len(&mountinfo), 128 * 1024);
    }

    #[test]
    fn kernfs_notify_wakes_poll_until_open_file_consumes_generation() {
        let node = KernfsNode::new_file("cgroup.events", 0o444, None, None);
        let sb = super::super::types::SuperBlock::alloc(
            "cgroup2",
            0x63677270,
            &super::super::ops::NOOP_SUPER_OPS,
        );
        let inode = inode_for_node(&sb, node);
        let dentry = super::super::types::Dentry::new_negative("cgroup.events");
        dentry.instantiate(inode);
        let file = super::super::types::File::new(dentry.clone(), 0, 0, &KERNFS_FILE_FILE_OPS);

        assert_eq!(kernfs_poll(&file, None), 0);
        notify(&dentry);
        assert_eq!(
            kernfs_poll(&file, None)
                & (crate::fs::select::POLLERR | crate::fs::select::POLLPRI) as u32,
            (crate::fs::select::POLLERR | crate::fs::select::POLLPRI) as u32
        );
        consume_poll_event(&file);
        assert_eq!(kernfs_poll(&file, None), 0);
    }

    #[test]
    fn cgroup_kernfs_rmdir_removes_empty_cgroup_dirs() {
        let root = KernfsNode::new_dir("/", 0o755);
        let service = crate::kernel::cgroup::new_cgroup_dir("systemd-logind.service", 0o755);
        add_child(&root, service);
        let sb = super::super::types::SuperBlock::alloc(
            "cgroup2",
            0x63677270,
            &super::super::ops::NOOP_SUPER_OPS,
        );
        let inode = inode_for_node(&sb, root.clone());

        assert!(lookup(&root, "systemd-logind.service").is_some());
        assert_eq!(kernfs_rmdir(&inode, "systemd-logind.service"), Ok(()));
        assert!(lookup(&root, "systemd-logind.service").is_none());
    }

    #[test]
    fn cgroup_kernfs_rmdir_refuses_child_cgroups() {
        let root = KernfsNode::new_dir("/", 0o755);
        let parent = crate::kernel::cgroup::new_cgroup_dir("system.slice", 0o755);
        add_child(
            &parent,
            crate::kernel::cgroup::new_cgroup_dir("systemd-logind.service", 0o755),
        );
        add_child(&root, parent);
        let sb = super::super::types::SuperBlock::alloc(
            "cgroup2",
            0x63677270,
            &super::super::ops::NOOP_SUPER_OPS,
        );
        let inode = inode_for_node(&sb, root.clone());

        assert_eq!(kernfs_rmdir(&inode, "system.slice"), Err(EBUSY));
        assert!(lookup(&root, "system.slice").is_some());
    }

    #[test]
    fn delegated_cgroup_control_files_inherit_directory_owner() {
        let service = crate::kernel::cgroup::new_cgroup_dir("user-at-1000.service", 0o755);
        let sb = super::super::types::SuperBlock::alloc(
            "cgroup2",
            0x63677270,
            &super::super::ops::NOOP_SUPER_OPS,
        );
        let service_inode = inode_for_node(&sb, service);
        service_inode.uid.store(1000, Ordering::Release);
        service_inode.gid.store(1000, Ordering::Release);

        let procs = kernfs_lookup(&service_inode, "cgroup.procs").expect("cgroup.procs");
        assert_eq!(procs.uid.load(Ordering::Acquire), 1000);
        assert_eq!(procs.gid.load(Ordering::Acquire), 1000);
        assert_eq!(procs.mode.load(Ordering::Acquire) & 0o777, 0o644);
    }
}
