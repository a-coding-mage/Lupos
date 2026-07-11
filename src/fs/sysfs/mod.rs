//! linux-parity: complete
//! linux-source: vendor/linux/fs/sysfs
//! test-origin: linux:vendor/linux/fs/sysfs
//! sysfs (M41) — kernfs-backed.
//!
//! Mirrors `vendor/linux/fs/sysfs/`.  Real `kobject` integration lives in
//! `src/kernel/kobject/`; sysfs simply exposes registered kobjects through a
//! kernfs hierarchy.

extern crate alloc;

use crate::fs::dcache::d_alloc;
use crate::fs::kernfs::{KernfsNode, add_child, inode_for_node};
use crate::fs::ops::SuperOps;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};

pub mod dir;
pub mod file;
pub mod group;
#[path = "mount.rs"]
pub mod mount_ops;
pub mod symlink;

const SYSFS_MAGIC: u64 = 0x62656572;

pub static SYSFS_SUPER_OPS: SuperOps = SuperOps {
    name: "sysfs",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("sysfs", SYSFS_MAGIC, &SYSFS_SUPER_OPS);
    let (root, kernel) = mount_ops::build_root();

    // Attach kobjects registered before mount.
    crate::lib::kobject::sysfs_attach_root(&kernel);

    let root_inode = inode_for_node(&sb, root);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "sysfs",
        mount,
        fs_flags: 0,
    });
}

pub fn register_module_exports() {
    file::register_module_exports();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs;

    #[test]
    fn sysfs_group_file_and_symlink_helpers_attach_nodes() {
        let root = kernfs::KernfsNode::new_dir("/", 0o755);
        let group = group::create_group(&root, "power");
        file::create_file(&group, "state", 0o444, None, None);
        symlink::create_link(&root, "power-link", "power");
        assert!(kernfs::lookup(&root, "power").is_some());
        assert!(kernfs::lookup(&group, "state").is_some());
        assert!(kernfs::lookup(&root, "power-link").is_some());
    }
}
