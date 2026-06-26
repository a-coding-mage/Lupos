//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4/inline.c
//! ext4 inline_data feature.  Small files store their bytes inside the
//! inode itself (in `i_block` + `i_xattr` overflow region).
//!
//! Mirrors `vendor/linux/fs/ext4/inline.c`.  M45 only consumes the in-inode
//! portion (`i_block`, 60 bytes) — the xattr overflow region is rare on
//! freshly mkfs'd images and supports up to inode-size minus header.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use super::Ext4Inode;

/// Pull the inline payload out of an inode (up to 60 bytes from `i_block`).
pub fn inline_payload(ext4_inode: &Ext4Inode, max: usize) -> Vec<u8> {
    let i_block_copy = { ext4_inode.raw.lock().i_block };
    let buf: &[u8] = unsafe { core::slice::from_raw_parts(i_block_copy.as_ptr() as *const u8, 60) };
    let n = max
        .min(buf.len())
        .min(ext4_inode.i_size.load(Ordering::Acquire) as usize);
    buf[..n].to_vec()
}
