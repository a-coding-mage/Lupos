//! linux-parity: partial
//! linux-source: vendor/linux/fs/kernfs/dir.c
//! kernfs directory helpers.
//!
//! Ref: `vendor/linux/fs/kernfs/dir.c`

use alloc::sync::Arc;

use super::{KernfsNode, add_child, lookup};

pub fn kernfs_create_dir(parent: &Arc<KernfsNode>, name: &str, mode: u32) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(name, mode);
    add_child(parent, dir.clone());
    dir
}

pub fn kernfs_find(parent: &Arc<KernfsNode>, name: &str) -> Option<Arc<KernfsNode>> {
    lookup(parent, name)
}
