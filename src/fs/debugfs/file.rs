//! linux-parity: partial
//! linux-source: vendor/linux/fs/debugfs/file.c
//! debugfs file helpers.
//!
//! Ref: `vendor/linux/fs/debugfs/file.c`

use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, ShowFn, StoreFn};

pub fn create_file(
    name: &str,
    mode: u32,
    parent: &Arc<KernfsNode>,
    show: Option<ShowFn>,
    store: Option<StoreFn>,
) -> Arc<KernfsNode> {
    super::debugfs_create_file(name, mode, parent, show, store)
}
