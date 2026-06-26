//! linux-parity: partial
//! linux-source: vendor/linux/mm/swapfile.c
//! `/proc/swaps` renderer.

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &crate::mm::swap::proc_swaps())
}
