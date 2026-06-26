//! linux-parity: partial
//! linux-source: vendor/linux/ipc/mqueue.c
//! test-origin: linux:vendor/linux/ipc/mqueue.c
//! POSIX message queue filesystem mount surface.
//!
//! Message queue syscalls are implemented in the IPC layer.  This filesystem
//! type gives systemd's `dev-mqueue.mount` the Linux API filesystem it expects.

use crate::fs::dcache::d_alloc;
use crate::fs::ops::SuperOps;
use crate::fs::ramfs::{RAMFS_DIR_FILE_OPS, RAMFS_DIR_INODE_OPS};
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{Inode, InodeKind, SuperBlock, SuperBlockRef, init_inode_metadata};

const MQUEUE_MAGIC: u64 = 0x1980_0202;

pub static MQUEUE_SUPER_OPS: SuperOps = SuperOps {
    name: "mqueue",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("mqueue", MQUEUE_MAGIC, &MQUEUE_SUPER_OPS);
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
        name: "mqueue",
        mount,
        fs_flags: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mqueue_mount_has_linux_name_and_directory_root() {
        let sb = mount("mqueue", 0, "").expect("mount mqueue");
        assert_eq!(sb.fs_name, "mqueue");
        assert_eq!(sb.magic, MQUEUE_MAGIC);
        assert!(sb.root().unwrap().inode().unwrap().is_dir());
    }
}
