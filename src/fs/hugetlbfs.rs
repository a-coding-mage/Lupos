//! linux-parity: partial
//! linux-source: vendor/linux/fs/hugetlbfs/inode.c
//! test-origin: linux:vendor/linux/fs/hugetlbfs/inode.c
//! Minimal hugetlbfs mount surface.
//!
//! Huge-page allocation policy lives in `crate::mm::huge`; this module exposes
//! the filesystem type systemd expects for `dev-hugepages.mount`.

use crate::fs::dcache::d_alloc;
use crate::fs::ops::SuperOps;
use crate::fs::ramfs::{RAMFS_DIR_FILE_OPS, RAMFS_DIR_INODE_OPS};
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{Inode, InodeKind, SuperBlock, SuperBlockRef, init_inode_metadata};

const HUGETLBFS_MAGIC: u64 = 0x9584_58f6;

pub static HUGETLBFS_SUPER_OPS: SuperOps = SuperOps {
    name: "hugetlbfs",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("hugetlbfs", HUGETLBFS_MAGIC, &HUGETLBFS_SUPER_OPS);
    let root_inode = Inode::new(
        sb.alloc_ino(),
        InodeKind::Directory,
        0o755,
        &RAMFS_DIR_INODE_OPS,
        &RAMFS_DIR_FILE_OPS,
        crate::fs::libfs::empty_ram_dir(),
    );
    init_inode_metadata(&root_inode, 0, 0, 2, 0);
    *root_inode.sb.lock() = Some(sb.clone());
    let root = d_alloc("/");
    root.instantiate(root_inode);
    *sb.root.lock() = Some(root);
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "hugetlbfs",
        mount,
        fs_flags: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hugetlbfs_mount_has_linux_name_and_directory_root() {
        let sb = mount("hugetlbfs", 0, "").expect("mount hugetlbfs");
        assert_eq!(sb.fs_name, "hugetlbfs");
        assert_eq!(sb.magic, HUGETLBFS_MAGIC);
        assert!(sb.root().unwrap().inode().unwrap().is_dir());
    }
}
