//! linux-parity: complete
//! linux-source: vendor/linux/fs/attr.c
//! test-origin: linux:vendor/linux/fs/attr.c
//! Inode attribute changes.
//!
//! Ref: `vendor/linux/fs/attr.c`

use core::sync::atomic::Ordering;

use crate::include::uapi::errno::{EINVAL, EPERM, EROFS};
use crate::include::uapi::stat::S_IFMT;

use super::types::{InodeKind, InodeRef};

pub const ATTR_MODE: u32 = 1 << 0;
pub const ATTR_UID: u32 = 1 << 1;
pub const ATTR_GID: u32 = 1 << 2;
pub const ATTR_SIZE: u32 = 1 << 3;
pub const ATTR_ATIME: u32 = 1 << 4;
pub const ATTR_MTIME: u32 = 1 << 5;
pub const ATTR_CTIME: u32 = 1 << 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IAttr {
    pub valid: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
}

impl IAttr {
    pub const fn mode(mode: u32) -> Self {
        Self {
            valid: ATTR_MODE,
            mode,
            uid: 0,
            gid: 0,
            size: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
        }
    }

    pub const fn size(size: u64) -> Self {
        Self {
            valid: ATTR_SIZE,
            mode: 0,
            uid: 0,
            gid: 0,
            size,
            atime: 0,
            mtime: 0,
            ctime: 0,
        }
    }
}

pub fn setattr_prepare(inode: &InodeRef, attr: &IAttr, readonly: bool) -> Result<(), i32> {
    if readonly && attr.valid != 0 {
        return Err(EROFS);
    }
    if attr.valid & ATTR_MODE != 0 {
        if attr.mode & S_IFMT != 0 && attr.mode & S_IFMT != inode.kind.s_ifmt() {
            return Err(EINVAL);
        }
    }
    if attr.valid & ATTR_SIZE != 0 && inode.kind != InodeKind::Regular {
        return Err(EINVAL);
    }
    if attr.valid
        & !(ATTR_MODE | ATTR_UID | ATTR_GID | ATTR_SIZE | ATTR_ATIME | ATTR_MTIME | ATTR_CTIME)
        != 0
    {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn notify_change(inode: &InodeRef, attr: &IAttr, readonly: bool) -> Result<(), i32> {
    setattr_prepare(inode, attr, readonly)?;
    let evm_metadata_changed = super::xattr::evm_setattr_prepare(inode, attr)?;

    if attr.valid & ATTR_MODE != 0 {
        let kind = inode.kind.s_ifmt();
        inode
            .mode
            .store(kind | (attr.mode & !S_IFMT), Ordering::Release);
    }
    if attr.valid & ATTR_UID != 0 {
        inode.uid.store(attr.uid, Ordering::Release);
    }
    if attr.valid & ATTR_GID != 0 {
        inode.gid.store(attr.gid, Ordering::Release);
    }
    if attr.valid & ATTR_SIZE != 0 {
        super::libfs::ram_file_set_size(inode, attr.size)?;
    }
    if attr.valid & ATTR_ATIME != 0 {
        inode.atime.store(attr.atime, Ordering::Release);
    }
    if attr.valid & ATTR_MTIME != 0 {
        inode.mtime.store(attr.mtime, Ordering::Release);
    }
    if attr.valid & ATTR_CTIME != 0 {
        inode.ctime.store(attr.ctime, Ordering::Release);
    }

    super::xattr::evm_post_setattr(inode, evm_metadata_changed)?;
    Ok(())
}

pub fn inode_owner_or_capable(inode: &InodeRef, uid: u32) -> Result<(), i32> {
    if uid == 0 || inode.uid.load(Ordering::Acquire) == uid {
        Ok(())
    } else {
        Err(EPERM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::ops::{NOOP_FILE_OPS, NOOP_INODE_OPS};
    use crate::fs::types::{Inode, InodeKind, InodePrivate};

    #[test]
    fn setattr_updates_mode_and_size_without_changing_kind() {
        let inode = Inode::new(
            1,
            InodeKind::Regular,
            0o644,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::RamBytes(spin::Mutex::new(alloc::vec![1, 2, 3])),
        );
        notify_change(&inode, &IAttr::mode(0o600), false).unwrap();
        assert_eq!(inode.mode.load(Ordering::Acquire), 0o100600);

        notify_change(&inode, &IAttr::size(5), false).unwrap();
        assert_eq!(inode.size.load(Ordering::Acquire), 5);
        match &inode.private {
            InodePrivate::RamBytes(bytes) => assert_eq!(bytes.lock().len(), 3),
            _ => panic!("expected ram bytes"),
        }
    }

    #[test]
    fn setattr_size_growth_keeps_ram_bytes_sparse() {
        let inode = Inode::new(
            1,
            InodeKind::Regular,
            0o644,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::RamBytes(spin::Mutex::new(alloc::vec![1, 2, 3])),
        );
        notify_change(&inode, &IAttr::size(8 * 1024 * 1024), false).unwrap();
        assert_eq!(inode.size.load(Ordering::Acquire), 8 * 1024 * 1024);
        match &inode.private {
            InodePrivate::RamBytes(bytes) => assert_eq!(bytes.lock().len(), 3),
            _ => panic!("expected ram bytes"),
        }
    }

    #[test]
    fn setattr_rejects_readonly_and_nonregular_truncate() {
        let dir = Inode::new(
            2,
            InodeKind::Directory,
            0o755,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::None,
        );
        assert_eq!(setattr_prepare(&dir, &IAttr::mode(0o700), true), Err(EROFS));
        assert_eq!(setattr_prepare(&dir, &IAttr::size(1), false), Err(EINVAL));
    }
}
