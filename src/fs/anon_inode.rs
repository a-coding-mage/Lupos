//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! Minimal anon-inode file allocation for fd-backed kernel objects.
//!
//! Ref: `vendor/linux/fs/anon_inodes.c`.  Lupos keeps the object pointer as a
//! small registry token in `file.private`; each subsystem owns the registry.

extern crate alloc;

use crate::fs::dcache::d_alloc;
use crate::fs::file::alloc_file;
use crate::fs::ops::{FileOps, NOOP_INODE_OPS};
use crate::fs::types::{FileRef, Inode, InodeKind, InodePrivate};

pub fn alloc_anon_file(name: &str, fops: &'static FileOps, token: usize) -> FileRef {
    alloc_anon_file_with_kind(name, fops, token, InodeKind::Socket, 0o600)
}

pub fn alloc_anon_file_with_kind(
    name: &str,
    fops: &'static FileOps,
    token: usize,
    kind: InodeKind,
    mode: u32,
) -> FileRef {
    let dentry = d_alloc(name);
    let inode = Inode::new(
        token as u64,
        kind,
        mode,
        &NOOP_INODE_OPS,
        fops,
        InodePrivate::Opaque(token),
    );
    dentry.instantiate(inode);
    let file = alloc_file(dentry, 0, 0, fops);
    *file.private.lock() = token;
    file
}
