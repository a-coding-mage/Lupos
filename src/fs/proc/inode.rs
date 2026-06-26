//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/inode.c
//! procfs inode helpers.
//!
//! Ref: `vendor/linux/fs/proc/inode.c`

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub fn proc_inode_name(node: &Arc<KernfsNode>) -> &str {
    node.name.as_str()
}
