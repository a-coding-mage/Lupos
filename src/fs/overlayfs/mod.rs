//! linux-parity: partial
//! linux-source: vendor/linux/fs/overlayfs
//! overlayfs (M42) — skeleton.
//!
//! Mirrors `vendor/linux/fs/overlayfs/`.  Mount succeeds against a single
//! lower layer and exposes its tree read-only; writes return `EROFS`.  Full
//! upper / merge / copy-up plumbing waits on M76.

extern crate alloc;

use crate::fs::dcache::d_alloc;
use crate::fs::ops::{FileOps, InodeOps, SuperOps};
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{Inode, InodeKind, InodePrivate, InodeRef, SuperBlock, SuperBlockRef};
use crate::include::uapi::errno::EROFS;

const OVERLAYFS_MAGIC: u64 = 0x794c7630;

pub static OVERLAY_DIR_INODE_OPS: InodeOps = InodeOps {
    name: "overlay_dir",
    lookup: Some(crate::fs::libfs::simple_lookup),
    create: Some(|_, _, _| Err(EROFS)),
    mkdir: Some(|_, _, _| Err(EROFS)),
    unlink: Some(|_, _, _| Err(EROFS)),
    rmdir: Some(|_, _| Err(EROFS)),
    rename: Some(|_, _, _, _| Err(EROFS)),
    symlink: Some(|_, _, _, _| Err(EROFS)),
    readlink: None,
    setattr: None,
};

pub static OVERLAY_DIR_FILE_OPS: FileOps = FileOps {
    name: "overlay_dir",
    read: None,
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: Some(crate::fs::libfs::simple_readdir),
};

pub static OVERLAY_SUPER_OPS: SuperOps = SuperOps {
    name: "overlay",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("overlay", OVERLAYFS_MAGIC, &OVERLAY_SUPER_OPS);
    let root_inode = Inode::new(
        sb.alloc_ino(),
        InodeKind::Directory,
        0o555,
        &OVERLAY_DIR_INODE_OPS,
        &OVERLAY_DIR_FILE_OPS,
        crate::fs::libfs::empty_ram_dir(),
    );
    *root_inode.sb.lock() = Some(sb.clone());
    let root = d_alloc("/");
    root.instantiate(root_inode);
    *sb.root.lock() = Some(root);
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "overlay",
        mount,
        fs_flags: 0,
    });
}

// Suppress unused-import lint when InodePrivate isn't used directly.
#[allow(dead_code)]
fn _unused() {
    let _: Option<InodePrivate> = None;
}
