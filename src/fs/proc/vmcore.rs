//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/vmcore.c
//! `/proc/vmcore`.
//!
//! Ref: `vendor/linux/fs/proc/vmcore.c`

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "")
}
