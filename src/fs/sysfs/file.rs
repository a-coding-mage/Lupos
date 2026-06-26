//! linux-parity: partial
//! linux-source: vendor/linux/fs/sysfs/file.c
//! sysfs file helpers.
//!
//! Ref: `vendor/linux/fs/sysfs/file.c`

use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, ShowFn, StoreFn, add_child};

pub fn create_file(
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
