//! linux-parity: partial
//! linux-source: vendor/linux/fs/debugfs/inode.c
//! debugfs inode and root helpers.
//!
//! Ref: `vendor/linux/fs/debugfs/inode.c`

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub fn root() -> Arc<KernfsNode> {
    super::root_node()
}
