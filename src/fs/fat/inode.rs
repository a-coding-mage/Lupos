//! linux-parity: partial
//! linux-source: vendor/linux/fs/fat/inode.c
//! FAT inode in-core fields + lookup.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use crate::fs::types::{Inode, InodeKind, InodePrivate, InodeRef, SuperBlockRef};

use super::FatSbi;

pub struct FatInode {
    pub start_cluster: u32,
    pub size: u32,
    pub is_dir: bool,
}

pub fn make_inode(
    sbi: &FatSbi,
    start_cluster: u32,
    size: u32,
    is_dir: bool,
    sb: &SuperBlockRef,
) -> InodeRef {
    let payload = Arc::new(FatInode {
        start_cluster,
        size,
        is_dir,
    });
    let opaque = Arc::into_raw(payload) as usize;
    let kind = if is_dir {
        InodeKind::Directory
    } else {
        InodeKind::Regular
    };
    let ino = (((start_cluster as u64) | 1) << 1) | (is_dir as u64);
    let i = Inode::new(
        ino,
        kind,
        0o644,
        match kind {
            InodeKind::Directory => &super::ops::FAT_DIR_INODE_OPS,
            _ => &super::ops::FAT_FILE_INODE_OPS,
        },
        match kind {
            InodeKind::Directory => &super::ops::FAT_DIR_FILE_OPS,
            _ => &super::ops::FAT_FILE_FILE_OPS,
        },
        InodePrivate::Opaque(opaque),
    );
    *i.sb.lock() = Some(sb.clone());
    i.size.store(size as u64, Ordering::Release);
    let _ = sbi.bdev.id;
    i
}

pub fn root_inode(sbi: &FatSbi, sb: &SuperBlockRef) -> InodeRef {
    make_inode(sbi, sbi.root_cluster, 0, true, sb)
}

pub fn fat_of(inode: &InodeRef) -> Option<Arc<FatInode>> {
    if let InodePrivate::Opaque(p) = &inode.private {
        let raw = *p as *const FatInode;
        unsafe {
            Arc::increment_strong_count(raw);
            Some(Arc::from_raw(raw))
        }
    } else {
        None
    }
}
