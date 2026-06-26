//! linux-parity: partial
//! linux-source: vendor/linux/fs/kernfs/mount.c
//! kernfs mount helpers.
//!
//! Ref: `vendor/linux/fs/kernfs/mount.c`

use alloc::sync::Arc;

use super::KernfsNode;

pub fn kernfs_root(name: &str, mode: u32) -> Arc<KernfsNode> {
    KernfsNode::new_dir(name, mode)
}
