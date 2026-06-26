//! linux-parity: complete
//! linux-source: vendor/linux/fs/debugfs
//! test-origin: linux:vendor/linux/fs/debugfs
//! debugfs (M42) — kernfs-backed.
//!
//! Mirrors `vendor/linux/fs/debugfs/`.  Helpers (`debugfs_create_u32`,
//! `debugfs_create_file`) attach kernfs nodes to the active root.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::dcache::d_alloc;
use crate::fs::kernfs::{KernfsNode, ShowFn, StoreFn, add_child, inode_for_node};
use crate::fs::ops::SuperOps;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};

pub mod file;
pub mod inode;

const DEBUGFS_MAGIC: u64 = 0x64626720;

pub static DEBUGFS_SUPER_OPS: SuperOps = SuperOps {
    name: "debugfs",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

lazy_static! {
    pub(super) static ref DEBUGFS_ROOT: Mutex<Option<Arc<KernfsNode>>> = Mutex::new(None);
}

pub(super) fn root_node() -> Arc<KernfsNode> {
    if let Some(root) = DEBUGFS_ROOT.lock().clone() {
        return root;
    }
    let mut guard = DEBUGFS_ROOT.lock();
    if let Some(root) = guard.clone() {
        return root;
    }
    let root = KernfsNode::new_dir("/", 0o755);
    *guard = Some(root.clone());
    root
}

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("debugfs", DEBUGFS_MAGIC, &DEBUGFS_SUPER_OPS);
    let root = {
        let mut g = DEBUGFS_ROOT.lock();
        if let Some(r) = g.clone() {
            r
        } else {
            let r = KernfsNode::new_dir("/", 0o755);
            *g = Some(r.clone());
            r
        }
    };
    let root_inode = inode_for_node(&sb, root);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    Ok(sb)
}

/// Top-level dir creation (e.g., `debugfs_create_dir("lupos", NULL)`).
pub fn debugfs_create_dir(name: &str, parent: Option<&Arc<KernfsNode>>) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(name, 0o755);
    let p = match parent {
        Some(p) => p.clone(),
        None => root_node(),
    };
    add_child(&p, dir.clone());
    dir
}

pub fn debugfs_create_file(
    name: &str,
    mode: u32,
    parent: &Arc<KernfsNode>,
    show: Option<ShowFn>,
    store: Option<StoreFn>,
) -> Arc<KernfsNode> {
    let f = KernfsNode::new_file(name, mode, show, store);
    add_child(parent, f.clone());
    f
}

// `debugfs_create_u32(name, mode, parent, &value)` analogue.
static U32_BACKING: AtomicU64 = AtomicU64::new(0);

fn u32_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let raw = node.priv_ptr.load(Ordering::Acquire) as *const AtomicU64;
    let v = if raw.is_null() {
        U32_BACKING.load(Ordering::Acquire) as u32
    } else {
        unsafe { (*raw).load(Ordering::Acquire) as u32 }
    };
    let s = alloc::format!("{}\n", v);
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}
fn u32_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| crate::include::uapi::errno::EINVAL)?;
    let v: u32 = s
        .trim()
        .parse()
        .map_err(|_| crate::include::uapi::errno::EINVAL)?;
    let raw = node.priv_ptr.load(Ordering::Acquire) as *const AtomicU64;
    if raw.is_null() {
        U32_BACKING.store(v as u64, Ordering::Release);
    } else {
        unsafe {
            (*raw).store(v as u64, Ordering::Release);
        }
    }
    Ok(buf.len())
}

pub fn debugfs_create_u32(
    name: &str,
    mode: u32,
    parent: &Arc<KernfsNode>,
    backing: &'static AtomicU64,
) -> Arc<KernfsNode> {
    let f = KernfsNode::new_file(name, mode, Some(u32_show), Some(u32_store));
    f.priv_ptr
        .store(backing as *const _ as u64, Ordering::Release);
    add_child(parent, f.clone());
    f
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "debugfs",
        mount,
        fs_flags: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debugfs_file_and_inode_helpers_share_root() {
        *DEBUGFS_ROOT.lock() = None;
        let root = inode::root();
        let dir = debugfs_create_dir("lupos", Some(&root));
        file::create_file("state", 0o444, &dir, None, None);
        assert!(crate::fs::kernfs::lookup(&root, "lupos").is_some());
        assert!(crate::fs::kernfs::lookup(&dir, "state").is_some());
    }
}
