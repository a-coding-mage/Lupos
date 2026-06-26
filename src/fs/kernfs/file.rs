//! linux-parity: partial
//! linux-source: vendor/linux/fs/kernfs/file.c
//! kernfs file helpers.
//!
//! Ref: `vendor/linux/fs/kernfs/file.c`

use alloc::sync::Arc;

use super::{KernfsNode, ShowFn, StoreFn, add_child};

pub fn kernfs_create_file(
    parent: &Arc<KernfsNode>,
    name: &str,
    mode: u32,
    show: Option<ShowFn>,
    store: Option<StoreFn>,
) -> Arc<KernfsNode> {
    let file = KernfsNode::new_file(name, mode, show, store);
    add_child(parent, file.clone());
    file
}
