//! linux-parity: partial
//! linux-source: vendor/linux/security/inode.c
//! test-origin: linux:vendor/linux/security/inode.c
//! securityfs (M42) â€” skeleton.
//!
//! Mirrors `vendor/linux/security/inode.c`. Mount plus the
//! `securityfs_create_*` helpers back the integrity/IMA/EVM securityfs surface.

extern crate alloc;

use alloc::sync::Arc;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::dcache::d_alloc;
use crate::fs::kernfs::{
    KernfsKind, KernfsNode, OpenReadFn, OpenReleaseFn, OpenWriteFn, ShowFn, StoreFn, add_child,
    inode_for_node, lookup,
};
use crate::fs::ops::SuperOps;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};

const SECURITYFS_MAGIC: u64 = 0x73636673;

pub static SECURITYFS_SUPER_OPS: SuperOps = SuperOps {
    name: "securityfs",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

lazy_static! {
    static ref SECURITYFS_ROOT: Mutex<Option<Arc<KernfsNode>>> = Mutex::new(None);
}

pub fn securityfs_root() -> Arc<KernfsNode> {
    let mut guard = SECURITYFS_ROOT.lock();
    if let Some(root) = guard.clone() {
        return root;
    }

    let root = KernfsNode::new_dir("/", 0o755);
    *guard = Some(root.clone());
    root
}

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("securityfs", SECURITYFS_MAGIC, &SECURITYFS_SUPER_OPS);
    let root = securityfs_root();
    let root_inode = inode_for_node(&sb, root);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    Ok(sb)
}

pub fn securityfs_create_dir(name: &str, parent: Option<&Arc<KernfsNode>>) -> Arc<KernfsNode> {
    let p = parent.cloned().unwrap_or_else(securityfs_root);
    if let Some(existing) = lookup(&p, name)
        && matches!(existing.kind, KernfsKind::Dir)
    {
        return existing;
    }

    let d = KernfsNode::new_dir(name, 0o755);
    add_child(&p, d.clone());
    d
}

pub fn securityfs_create_file(
    name: &str,
    mode: u32,
    parent: Option<&Arc<KernfsNode>>,
    show: Option<ShowFn>,
    store: Option<StoreFn>,
) -> Arc<KernfsNode> {
    let p = parent.cloned().unwrap_or_else(securityfs_root);
    if let Some(existing) = lookup(&p, name)
        && matches!(existing.kind, KernfsKind::File { .. })
    {
        return existing;
    }

    let f = KernfsNode::new_file(name, mode, show, store);
    add_child(&p, f.clone());
    f
}

pub fn securityfs_create_file_with_open_ops(
    name: &str,
    mode: u32,
    parent: Option<&Arc<KernfsNode>>,
    read: Option<OpenReadFn>,
    write: Option<OpenWriteFn>,
    release: Option<OpenReleaseFn>,
) -> Arc<KernfsNode> {
    let p = parent.cloned().unwrap_or_else(securityfs_root);
    if let Some(existing) = lookup(&p, name)
        && matches!(existing.kind, KernfsKind::File { .. })
    {
        return existing;
    }

    let f = KernfsNode::new_file_with_open_ops(name, mode, None, None, read, write, release);
    add_child(&p, f.clone());
    f
}

pub fn securityfs_create_symlink(
    name: &str,
    parent: Option<&Arc<KernfsNode>>,
    target: &str,
) -> Arc<KernfsNode> {
    let p = parent.cloned().unwrap_or_else(securityfs_root);
    if let Some(existing) = lookup(&p, name)
        && matches!(existing.kind, KernfsKind::Symlink { .. })
    {
        return existing;
    }

    let link = KernfsNode::new_symlink(name, target);
    add_child(&p, link.clone());
    link
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "securityfs",
        mount,
        fs_flags: 0,
    });
}

#[cfg(test)]
pub fn reset_for_test() {
    *SECURITYFS_ROOT.lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::{KernfsKind, lookup};

    #[test]
    fn securityfs_helpers_create_linux_style_tree() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        let integrity = securityfs_create_dir("integrity", None);
        let ima = securityfs_create_dir("ima", Some(&integrity));
        securityfs_create_file("runtime_measurements_count", 0o440, Some(&ima), None, None);
        securityfs_create_symlink("ima", None, "integrity/ima");

        let root = securityfs_root();
        assert!(lookup(&root, "integrity").is_some());
        assert!(lookup(&ima, "runtime_measurements_count").is_some());
        let link = lookup(&root, "ima").expect("ima symlink");
        match &link.kind {
            KernfsKind::Symlink { target } => assert_eq!(target, "integrity/ima"),
            _ => panic!("ima must be a securityfs symlink"),
        }
    }

    #[test]
    fn securityfs_create_dir_reuses_existing_integrity_dir() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        let first = securityfs_create_dir("integrity", None);
        securityfs_create_file("ima-marker", 0o440, Some(&first), None, None);
        let second = securityfs_create_dir("integrity", None);

        assert!(Arc::ptr_eq(&first, &second));
        assert!(lookup(&second, "ima-marker").is_some());
    }
}
