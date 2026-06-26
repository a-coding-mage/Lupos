//! linux-parity: partial
//! linux-source: vendor/linux/fs/inode.c
//! Inode helpers — `iget`, `iput`, `new_inode`.
//!
//! Ref: `vendor/linux/fs/inode.c`
//!
//! With Arc handling deallocation, `iput` is just a refcount adjustment for
//! Linux-style call-site spelling.  `iget_locked` is unneeded at this scale
//! (no inode hash table yet) — filesystems allocate inodes directly.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use super::ops::{FileOps, InodeOps};
use super::types::{Inode, InodeKind, InodePrivate, InodeRef, SuperBlockRef};

/// Allocate a fresh inode under `sb`, with a freshly minted ino.
pub fn new_inode(
    sb: &SuperBlockRef,
    kind: InodeKind,
    mode: u32,
    ops: &'static InodeOps,
    fops: &'static FileOps,
    private: InodePrivate,
) -> InodeRef {
    let ino = sb.alloc_ino();
    let i = Inode::new(ino, kind, mode, ops, fops, private);
    *i.sb.lock() = Some(sb.clone());
    i
}

/// `iget` — bump refcount.
pub fn iget(inode: &InodeRef) -> InodeRef {
    inode.i_count.fetch_add(1, Ordering::AcqRel);
    inode.clone()
}

/// `iput` — drop refcount.  Arc takes care of free when the last reference
/// goes away.
pub fn iput(inode: InodeRef) {
    inode.i_count.fetch_sub(1, Ordering::AcqRel);
    drop(inode);
}

/// Diagnostic — Arc strong count.
pub fn i_strong_count(i: &InodeRef) -> usize {
    Arc::strong_count(i)
}
