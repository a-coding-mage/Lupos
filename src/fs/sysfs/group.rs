//! linux-parity: partial
//! linux-source: vendor/linux/fs/sysfs/group.c
//! sysfs attribute group helpers.
//!
//! Ref: `vendor/linux/fs/sysfs/group.c`

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub fn create_group(parent: &Arc<KernfsNode>, name: &str) -> Arc<KernfsNode> {
    super::dir::create_dir(parent, name)
}
