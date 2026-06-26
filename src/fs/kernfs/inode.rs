//! linux-parity: partial
//! linux-source: vendor/linux/fs/kernfs/inode.c
//! kernfs inode bridge helpers.
//!
//! Ref: `vendor/linux/fs/kernfs/inode.c`

use alloc::sync::Arc;

use super::KernfsNode;
use crate::fs::types::{InodeRef, SuperBlockRef};

pub fn kernfs_get_inode(sb: &SuperBlockRef, node: Arc<KernfsNode>) -> InodeRef {
    super::inode_for_node(sb, node)
}
