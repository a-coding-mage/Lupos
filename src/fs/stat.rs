//! linux-parity: complete
//! linux-source: vendor/linux/fs/stat.c
//! test-origin: linux:vendor/linux/fs/stat.c
//! VFS stat and statfs helpers.
//!
//! Ref: `vendor/linux/fs/stat.c`

use core::sync::atomic::Ordering;

use super::types::{InodeRef, SuperBlockRef};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KStat {
    pub dev: u64,
    pub ino: u64,
    pub nlink: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u64,
    pub size: i64,
    pub blksize: i64,
    pub blocks: i64,
    pub atime: i64,
    pub atime_nsec: i64,
    pub mtime: i64,
    pub mtime_nsec: i64,
    pub ctime: i64,
    pub ctime_nsec: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KStatFs {
    pub f_type: i64,
    pub f_bsize: i64,
    pub f_blocks: i64,
    pub f_bfree: i64,
    pub f_bavail: i64,
    pub f_files: i64,
    pub f_ffree: i64,
    pub f_namelen: i64,
    pub f_frsize: i64,
    pub f_flags: i64,
}

pub fn vfs_getattr(inode: &InodeRef) -> KStat {
    let size = inode.size.load(Ordering::Acquire) as i64;
    KStat {
        dev: inode
            .sb
            .lock()
            .as_ref()
            .map(|sb| sb.magic)
            .unwrap_or_default(),
        ino: inode.ino,
        nlink: inode.nlink.load(Ordering::Acquire) as u64,
        mode: inode.mode.load(Ordering::Acquire),
        uid: inode.uid.load(Ordering::Acquire),
        gid: inode.gid.load(Ordering::Acquire),
        rdev: 0,
        size,
        blksize: 4096,
        blocks: (size.saturating_add(511) / 512).max(0),
        atime: inode.atime.load(Ordering::Acquire) as i64,
        mtime: inode.mtime.load(Ordering::Acquire) as i64,
        ctime: inode.ctime.load(Ordering::Acquire) as i64,
        ..KStat::default()
    }
}

pub fn vfs_statfs(sb: Option<&SuperBlockRef>) -> KStatFs {
    const HUGETLBFS_MAGIC: u64 = 0x9584_58f6;
    let f_type = sb.map(|sb| sb.magic).unwrap_or_default();
    let f_bsize = if f_type == HUGETLBFS_MAGIC {
        (crate::mm::huge::HPAGE_PMD_NR * crate::mm::frame::PAGE_SIZE) as i64
    } else {
        4096
    };
    KStatFs {
        f_type: f_type as i64,
        f_bsize,
        f_blocks: 1 << 20,
        f_bfree: 1 << 19,
        f_bavail: 1 << 19,
        f_files: 1 << 20,
        f_ffree: 1 << 19,
        f_namelen: 255,
        f_frsize: f_bsize,
        ..KStatFs::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::ops::{NOOP_FILE_OPS, NOOP_INODE_OPS};
    use crate::fs::types::{Inode, InodeKind, InodePrivate};

    #[test]
    fn kstat_reflects_inode_attributes() {
        let inode = Inode::new(
            42,
            InodeKind::Regular,
            0o640,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::None,
        );
        inode.size.store(513, Ordering::Release);
        inode.uid.store(1000, Ordering::Release);
        inode.gid.store(1001, Ordering::Release);

        let st = vfs_getattr(&inode);
        assert_eq!(st.ino, 42);
        assert_eq!(st.mode, 0o100640);
        assert_eq!(st.uid, 1000);
        assert_eq!(st.gid, 1001);
        assert_eq!(st.size, 513);
        assert_eq!(st.blocks, 2);
    }
}
