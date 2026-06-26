//! linux-parity: partial
//! linux-source: vendor/linux/fs/isofs/inode.c
//! ISO9660 in-core inode + factory.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use crate::fs::types::{Inode, InodeKind, InodePrivate, InodeRef, SuperBlockRef};

use super::IsoSbi;

pub struct IsoInode {
    pub extent: u32, // first sector
    pub size: u32,
    pub is_dir: bool,
}

pub fn make_inode(extent: u32, size: u32, is_dir: bool, sb: &SuperBlockRef) -> InodeRef {
    let payload = Arc::new(IsoInode {
        extent,
        size,
        is_dir,
    });
    let opaque = Arc::into_raw(payload) as usize;
    let kind = if is_dir {
        InodeKind::Directory
    } else {
        InodeKind::Regular
    };
    let i = Inode::new(
        extent as u64,
        kind,
        0o555,
        match kind {
            InodeKind::Directory => &super::ops::ISO_DIR_INODE_OPS,
            _ => &super::ops::ISO_FILE_INODE_OPS,
        },
        match kind {
            InodeKind::Directory => &super::ops::ISO_DIR_FILE_OPS,
            _ => &super::ops::ISO_FILE_FILE_OPS,
        },
        InodePrivate::Opaque(opaque),
    );
    *i.sb.lock() = Some(sb.clone());
    i.size.store(size as u64, Ordering::Release);
    i
}

pub fn root_inode(sbi: &IsoSbi, sb: &SuperBlockRef) -> InodeRef {
    make_inode(sbi.root_extent, sbi.root_size, true, sb)
}

pub fn iso_of(inode: &InodeRef) -> Option<Arc<IsoInode>> {
    if let InodePrivate::Opaque(p) = &inode.private {
        let raw = *p as *const IsoInode;
        unsafe {
            Arc::increment_strong_count(raw);
            Some(Arc::from_raw(raw))
        }
    } else {
        None
    }
}
