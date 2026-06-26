//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/kcore.c
//! `/proc/kcore`.
//!
//! Ref: `vendor/linux/fs/proc/kcore.c`

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "")
}
