//! linux-parity: complete
//! linux-source: vendor/linux/fs/ramfs
//! test-origin: linux:vendor/linux/fs/ramfs
//! ramfs — in-memory reference filesystem (M38).
//!
//! Mirrors `vendor/linux/fs/ramfs/inode.c`.  Files store bytes in
//! `InodePrivate::RamBytes`; directories use `InodePrivate::RamDir`.  All
//! libfs helpers (`simple_lookup`, `simple_unlink`, `ram_file_*`) plug in
//! straight from `crate::fs::libfs`.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::Ordering;
use spin::Mutex;

use crate::fs::dcache::d_alloc;
use crate::fs::libfs::{
    empty_ram_bytes, empty_ram_dir, ram_file_read, ram_file_write, simple_lookup, simple_readdir,
    simple_rmdir, simple_unlink,
};
use crate::fs::ops::{FileOps, InodeOps, SuperOps};
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{
    Inode, InodeKind, InodePrivate, InodeRef, SuperBlock, SuperBlockRef, init_inode_metadata,
    init_inode_owner, touch_inode_now,
};
use crate::include::uapi::errno::{EEXIST, EINVAL};
use crate::include::uapi::stat::S_IFDIR;

pub mod file_mmu;
pub mod file_nommu;

const RAMFS_MAGIC: u64 = 0x858458f6;

// ── Ops tables ────────────────────────────────────────────────────────────

pub static RAMFS_DIR_INODE_OPS: InodeOps = InodeOps {
    name: "ramfs_dir",
    lookup: Some(simple_lookup),
    create: Some(ramfs_create),
    mkdir: Some(ramfs_mkdir),
    unlink: Some(simple_unlink),
    rmdir: Some(simple_rmdir),
    rename: None,
    symlink: Some(ramfs_symlink),
    readlink: None,
    setattr: None,
};

pub static RAMFS_FILE_INODE_OPS: InodeOps = InodeOps {
    name: "ramfs_file",
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

pub static RAMFS_SYMLINK_INODE_OPS: InodeOps = InodeOps {
    name: "ramfs_symlink",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: Some(ramfs_readlink),
    setattr: None,
};

pub static RAMFS_FILE_OPS: FileOps = FileOps {
    name: "ramfs_file",
    read: Some(ram_file_read),
    write: Some(ram_file_write),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

pub static RAMFS_SYMLINK_FILE_OPS: FileOps = FileOps {
    name: "ramfs_symlink",
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

pub static RAMFS_DIR_FILE_OPS: FileOps = FileOps {
    name: "ramfs_dir",
    read: None,
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: Some(simple_readdir),
};

pub static RAMFS_SUPER_OPS: SuperOps = SuperOps {
    name: "ramfs",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

// ── Inode constructors ────────────────────────────────────────────────────

// Linux tmpfs accounts directory size via `BOGO_DIRENT_SIZE` (20 bytes per
// dentry entry) so `ls -l <dir>` and `stat <dir>` report a non-zero size
// that grows with the number of children.  Match that convention here so
// `ls -li /home/lupos` does not show a misleading `0` size column.  Ref:
// `vendor/linux/mm/shmem.c::BOGO_DIRENT_SIZE` and
// `vendor/linux/mm/shmem.c::shmem_get_inode` (which seeds new dirs with
// `2 * BOGO_DIRENT_SIZE` to account for the implicit `.` and `..` entries).
const BOGO_DIRENT_SIZE: u64 = 20;

fn make_dir_inode(sb: &SuperBlockRef, dir: Option<&InodeRef>, mode: u32) -> InodeRef {
    let ino = sb.alloc_ino();
    let i = Inode::new(
        ino,
        InodeKind::Directory,
        mode | S_IFDIR,
        &RAMFS_DIR_INODE_OPS,
        &RAMFS_DIR_FILE_OPS,
        empty_ram_dir(),
    );
    init_inode_owner(&i, dir, mode | S_IFDIR);
    init_inode_metadata(
        &i,
        i.uid.load(Ordering::Acquire),
        i.gid.load(Ordering::Acquire),
        2,
        0,
    );
    // `.` + `..` self-references → 2 * BOGO_DIRENT_SIZE on fresh dirs.
    i.size.store(2 * BOGO_DIRENT_SIZE, Ordering::Release);
    *i.sb.lock() = Some(sb.clone());
    i
}

/// Bump the directory's BOGO_DIRENT_SIZE accounting for one new entry.
/// `pub` so the rootfs initramfs unpack path (`ensure_static_file`,
/// `create_special_node`) — which inserts inodes directly into the
/// parent `RamDir` map for bulk-load performance — can call it without
/// going through `ramfs_create` / `ramfs_mkdir`.
pub fn dir_account_insert(dir: &InodeRef) {
    let prev = dir.size.load(Ordering::Acquire);
    dir.size
        .store(prev.saturating_add(BOGO_DIRENT_SIZE), Ordering::Release);
}

// Paired with `dir_account_insert` for unlink / rmdir / rename paths.
// Wired by future filesystem ops once they land; keeping the helper here
// next to its sibling so it doesn't drift when those operations port.
#[allow(dead_code)]
pub fn dir_account_remove(dir: &InodeRef) {
    let prev = dir.size.load(Ordering::Acquire);
    dir.size
        .store(prev.saturating_sub(BOGO_DIRENT_SIZE), Ordering::Release);
}

fn make_reg_inode(sb: &SuperBlockRef, dir: Option<&InodeRef>, mode: u32) -> InodeRef {
    let ino = sb.alloc_ino();
    let i = Inode::new(
        ino,
        InodeKind::Regular,
        mode,
        &RAMFS_FILE_INODE_OPS,
        &RAMFS_FILE_OPS,
        empty_ram_bytes(),
    );
    init_inode_owner(&i, dir, mode);
    init_inode_metadata(
        &i,
        i.uid.load(Ordering::Acquire),
        i.gid.load(Ordering::Acquire),
        1,
        0,
    );
    *i.sb.lock() = Some(sb.clone());
    i
}

fn make_symlink_inode(
    sb: &SuperBlockRef,
    dir: Option<&InodeRef>,
    mode: u32,
    target: &str,
) -> InodeRef {
    let ino = sb.alloc_ino();
    let i = Inode::new(
        ino,
        InodeKind::Symlink,
        mode,
        &RAMFS_SYMLINK_INODE_OPS,
        &RAMFS_SYMLINK_FILE_OPS,
        empty_ram_bytes(),
    );
    init_inode_owner(&i, dir, mode);
    init_inode_metadata(
        &i,
        i.uid.load(Ordering::Acquire),
        i.gid.load(Ordering::Acquire),
        1,
        0,
    );
    if let InodePrivate::RamBytes(bytes) = &i.private {
        bytes.lock().extend_from_slice(target.as_bytes());
    }
    i.size.store(target.len() as u64, Ordering::Release);
    *i.sb.lock() = Some(sb.clone());
    i
}

// ── Operations ────────────────────────────────────────────────────────────

fn dir_map(
    dir: &InodeRef,
) -> Result<&Mutex<alloc::collections::BTreeMap<alloc::string::String, InodeRef>>, i32> {
    match &dir.private {
        InodePrivate::RamDir(m) => Ok(m),
        _ => Err(EINVAL),
    }
}

fn ramfs_create(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let inode = make_reg_inode(&sb, Some(dir), mode);
    let map = dir_map(dir)?;
    let mut entries = map.lock();
    if entries.contains_key(name) {
        return Err(EEXIST);
    }
    entries.insert(alloc::string::String::from(name), inode.clone());
    drop(entries);
    dir_account_insert(dir);
    touch_inode_now(dir);
    Ok(inode)
}

fn ramfs_mkdir(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let inode = make_dir_inode(&sb, Some(dir), mode);
    let map = dir_map(dir)?;
    let mut entries = map.lock();
    if entries.contains_key(name) {
        return Err(EEXIST);
    }
    entries.insert(alloc::string::String::from(name), inode.clone());
    drop(entries);
    dir.nlink.fetch_add(1, Ordering::AcqRel);
    dir_account_insert(dir);
    touch_inode_now(dir);
    Ok(inode)
}

pub fn ramfs_symlink(dir: &InodeRef, name: &str, target: &str, mode: u32) -> Result<InodeRef, i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let inode = make_symlink_inode(&sb, Some(dir), mode, target);
    let map = dir_map(dir)?;
    let mut entries = map.lock();
    if entries.contains_key(name) {
        return Err(EEXIST);
    }
    entries.insert(alloc::string::String::from(name), inode.clone());
    drop(entries);
    dir_account_insert(dir);
    touch_inode_now(dir);
    Ok(inode)
}

fn ramfs_readlink(inode: &InodeRef, buf: &mut [u8]) -> Result<usize, i32> {
    let bytes = match &inode.private {
        InodePrivate::RamBytes(bytes) => bytes.lock(),
        _ => return Err(EINVAL),
    };
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    Ok(n)
}

// ── Mount ─────────────────────────────────────────────────────────────────

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("ramfs", RAMFS_MAGIC, &RAMFS_SUPER_OPS);
    let root_inode = make_dir_inode(&sb, None, 0o755);
    let root = d_alloc("/");
    root.instantiate(root_inode);
    *sb.root.lock() = Some(root);
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "ramfs",
        mount,
        fs_flags: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::{d_alloc_child, d_walk};
    use crate::fs::file::{alloc_file, fput};
    use crate::fs::read_write::{vfs_read, vfs_write};
    use crate::include::uapi::fcntl::O_RDWR;
    use crate::include::uapi::stat::S_ISGID;
    use crate::kernel::capability::KernelCapT;
    use crate::kernel::cred::{Cred, GroupInfo, INIT_CRED, KGid, KUid};
    use crate::kernel::{sched, task::TaskStruct};
    use alloc::boxed::Box;

    fn install_current(current: &mut TaskStruct, cred: &Cred) -> *mut TaskStruct {
        let previous = unsafe { sched::get_current() };
        current.pid = 4242;
        current.tgid = 4242;
        current.cred = cred as *const Cred;
        current.m27.real_cred = cred as *const Cred;
        unsafe { sched::set_current(current as *mut TaskStruct) };
        previous
    }

    fn test_cred(uid: u32, gid: u32) -> Cred {
        Cred {
            usage: core::sync::atomic::AtomicUsize::new(1),
            uid: KUid(uid),
            gid: KGid(gid),
            suid: KUid(uid),
            sgid: KGid(gid),
            euid: KUid(uid),
            egid: KGid(gid),
            fsuid: KUid(uid),
            fsgid: KGid(gid),
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
    fn ramfs_round_trip() {
        let sb = mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();

        // Create /foo
        let foo = ramfs_create(&root_inode, "foo", 0o644).unwrap();
        let d_foo = d_alloc_child(&root, "foo");
        d_foo.instantiate(foo.clone());

        // Open + write
        let f = alloc_file(d_foo.clone(), O_RDWR, 0o644, &RAMFS_FILE_OPS);
        let payload = b"hello world";
        let n = vfs_write(&f, payload).unwrap();
        assert_eq!(n, payload.len());

        // Seek back, read
        *f.pos.lock() = 0;
        let mut out = [0u8; 32];
        let r = vfs_read(&f, &mut out).unwrap();
        assert_eq!(r, payload.len());
        assert_eq!(&out[..r], payload);

        fput(f);
        // d_walk from root
        let resolved = d_walk(&root, "foo").unwrap();
        assert_eq!(resolved.name, "foo");
    }

    #[test]
    fn ramfs_directory_metadata_matches_linux_ramfs_basics() {
        let sb = mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();
        assert_eq!(root_inode.nlink.load(Ordering::Acquire), 2);

        let home = ramfs_mkdir(&root_inode, "home", 0o755).unwrap();
        assert_eq!(root_inode.nlink.load(Ordering::Acquire), 3);
        assert_eq!(home.nlink.load(Ordering::Acquire), 2);

        let st = crate::fs::stat::vfs_getattr(&home);
        assert!(st.ino > 0);
        assert!(st.mode & S_IFDIR != 0);
        assert!(st.mtime > 0);
    }

    #[test]
    fn ramfs_mmu_and_nommu_helpers_share_byte_storage() {
        let sb = mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();
        let inode = ramfs_create(&root_inode, "mmap", 0o644).unwrap();
        let dentry = d_alloc_child(&root, "mmap");
        dentry.instantiate(inode);
        let file = alloc_file(dentry, O_RDWR, 0o644, &RAMFS_FILE_OPS);

        let mut pos = 0;
        file_mmu::write(&file, b"abc", &mut pos).unwrap();
        pos = 0;
        let mut out = [0u8; 3];
        file_nommu::read(&file, &mut out, &mut pos).unwrap();
        assert_eq!(&out, b"abc");
        assert!(file_mmu::supports_mmap());
        assert!(file_nommu::supports_mmap());
    }

    /// Linux tmpfs accounts directory metadata via `BOGO_DIRENT_SIZE`:
    /// a fresh directory starts at `2 * 20 = 40` bytes (for `.` and `..`)
    /// and each child entry adds `20`.  Without this, `ls -li /home/lupos`
    /// reports a misleading `0` in the size column.  Ref:
    /// `vendor/linux/mm/shmem.c::BOGO_DIRENT_SIZE`,
    /// `shmem_get_inode` (2 * BOGO_DIRENT_SIZE seed), and
    /// `shmem_mknod` / `shmem_mkdir` (+= BOGO_DIRENT_SIZE).
    #[test]
    fn ramfs_directory_size_tracks_bogo_dirent_growth() {
        let sb = mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();

        // Fresh dir: 2 * BOGO_DIRENT_SIZE = 40.
        assert_eq!(
            root_inode.size.load(Ordering::Acquire),
            2 * BOGO_DIRENT_SIZE
        );

        let _ = ramfs_create(&root_inode, "a", 0o644).unwrap();
        assert_eq!(
            root_inode.size.load(Ordering::Acquire),
            2 * BOGO_DIRENT_SIZE + BOGO_DIRENT_SIZE
        );

        let _ = ramfs_mkdir(&root_inode, "home", 0o755).unwrap();
        assert_eq!(
            root_inode.size.load(Ordering::Acquire),
            2 * BOGO_DIRENT_SIZE + 2 * BOGO_DIRENT_SIZE
        );

        let _ = ramfs_symlink(&root_inode, "init", "/usr/lib/systemd/systemd", 0o777).unwrap();
        assert_eq!(
            root_inode.size.load(Ordering::Acquire),
            2 * BOGO_DIRENT_SIZE + 3 * BOGO_DIRENT_SIZE
        );

        // The new `home` directory is freshly seeded with 40 bytes for
        // its own `.` and `..`, not the parent's running total.
        let home = crate::fs::dcache::d_alloc_child(&root, "home");
        // The mkdir helper already inserted "home" into root's dir map,
        // so the inode reference path goes through the directory entry.
        // Look it up via dir_map for the size assertion.
        let inserted = dir_map(&root_inode)
            .unwrap()
            .lock()
            .get("home")
            .cloned()
            .unwrap();
        home.instantiate(inserted.clone());
        assert_eq!(inserted.size.load(Ordering::Acquire), 2 * BOGO_DIRENT_SIZE);
    }

    #[test]
    fn ramfs_symlink_readlink_round_trip() {
        let sb = mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();
        let inode = ramfs_symlink(&root_inode, "init", "/usr/lib/systemd/systemd", 0o777).unwrap();
        let mut buf = [0u8; 64];
        let n = inode.ops.readlink.unwrap()(&inode, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"/usr/lib/systemd/systemd");
    }

    #[test]
    fn ramfs_create_uses_current_fsuid_and_fsgid() {
        let sb = mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let cred = Box::new(test_cred(1001, 1002));
        let previous = install_current(&mut current, &cred);

        let inode = ramfs_create(&root_inode, "state", 0o640).unwrap();

        unsafe { sched::set_current(previous) };
        current.cred = &raw const INIT_CRED;
        assert_eq!(inode.uid.load(Ordering::Acquire), 1001);
        assert_eq!(inode.gid.load(Ordering::Acquire), 1002);
    }

    #[test]
    fn ramfs_directory_create_inherits_parent_setgid_gid() {
        let sb = mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();
        root_inode.gid.store(77, Ordering::Release);
        root_inode.mode.store(
            root_inode.mode.load(Ordering::Acquire) | S_ISGID,
            Ordering::Release,
        );

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let cred = Box::new(test_cred(1001, 1002));
        let previous = install_current(&mut current, &cred);

        let inode = ramfs_mkdir(&root_inode, "child", 0o755).unwrap();

        unsafe { sched::set_current(previous) };
        current.cred = &raw const INIT_CRED;
        assert_eq!(inode.uid.load(Ordering::Acquire), 1001);
        assert_eq!(inode.gid.load(Ordering::Acquire), 77);
        assert_ne!(inode.mode.load(Ordering::Acquire) & S_ISGID, 0);
    }
}
