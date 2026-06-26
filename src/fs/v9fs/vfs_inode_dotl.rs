//! linux-parity: partial
//! linux-source: vendor/linux/fs/9p/vfs_inode_dotl.c
//! test-origin: linux:vendor/linux/fs/9p/vfs_inode_dotl.c
//! 9P2000.L open flag, setattr, stat, and create-mode helpers.

use crate::fs::attr::{ATTR_CTIME, ATTR_GID, ATTR_MODE, ATTR_MTIME, ATTR_SIZE, ATTR_UID};
use crate::include::uapi::fcntl::{
    O_ACCMODE, O_APPEND, O_CLOEXEC, O_CREAT, O_DIRECT, O_DIRECTORY, O_DSYNC, O_EXCL, O_LARGEFILE,
    O_NOATIME, O_NOCTTY, O_NOFOLLOW, O_NONBLOCK, O_SYNC,
};
use crate::include::uapi::stat::S_ISGID;

use super::types::*;

pub const FASYNC: u32 = 0x0002_0000;
pub const ATTR_ATIME_SET: u32 = 1 << 7;
pub const ATTR_MTIME_SET: u32 = 1 << 8;

pub const P9_ATTR_MODE: u32 = 1 << 0;
pub const P9_ATTR_UID: u32 = 1 << 1;
pub const P9_ATTR_GID: u32 = 1 << 2;
pub const P9_ATTR_SIZE: u32 = 1 << 3;
pub const P9_ATTR_ATIME: u32 = 1 << 4;
pub const P9_ATTR_MTIME: u32 = 1 << 5;
pub const P9_ATTR_CTIME: u32 = 1 << 6;
pub const P9_ATTR_ATIME_SET: u32 = 1 << 7;
pub const P9_ATTR_MTIME_SET: u32 = 1 << 8;

pub fn v9fs_mapped_dotl_flags(flags: u32) -> u32 {
    let mappings = [
        (O_CREAT, P9_DOTL_CREATE),
        (O_EXCL, P9_DOTL_EXCL),
        (O_NOCTTY, P9_DOTL_NOCTTY),
        (O_APPEND, P9_DOTL_APPEND),
        (O_NONBLOCK, P9_DOTL_NONBLOCK),
        (O_DSYNC, P9_DOTL_DSYNC),
        (FASYNC, P9_DOTL_FASYNC),
        (O_DIRECT, P9_DOTL_DIRECT),
        (O_LARGEFILE, P9_DOTL_LARGEFILE),
        (O_DIRECTORY, P9_DOTL_DIRECTORY),
        (O_NOFOLLOW, P9_DOTL_NOFOLLOW),
        (O_NOATIME, P9_DOTL_NOATIME),
        (O_CLOEXEC, P9_DOTL_CLOEXEC),
        (O_SYNC, P9_DOTL_SYNC),
    ];
    mappings.iter().fold(
        0,
        |out, (open, dotl)| if flags & *open != 0 { out | *dotl } else { out },
    )
}

pub fn v9fs_open_to_dotl_flags(flags: u32) -> u32 {
    (flags & O_ACCMODE) | v9fs_mapped_dotl_flags(flags)
}

pub fn v9fs_get_fsgid_for_create(dir_mode: u32, dir_gid: u32, current_fsgid: u32) -> u32 {
    if dir_mode & S_ISGID != 0 {
        dir_gid
    } else {
        current_fsgid
    }
}

pub fn v9fs_mapped_iattr_valid(iattr_valid: u32) -> u32 {
    let mappings = [
        (ATTR_MODE, P9_ATTR_MODE),
        (ATTR_UID, P9_ATTR_UID),
        (ATTR_GID, P9_ATTR_GID),
        (ATTR_SIZE, P9_ATTR_SIZE),
        (crate::fs::attr::ATTR_ATIME, P9_ATTR_ATIME),
        (ATTR_MTIME, P9_ATTR_MTIME),
        (ATTR_CTIME, P9_ATTR_CTIME),
        (ATTR_ATIME_SET, P9_ATTR_ATIME_SET),
        (ATTR_MTIME_SET, P9_ATTR_MTIME_SET),
    ];
    mappings.iter().fold(0, |out, (attr, p9)| {
        if iattr_valid & *attr != 0 {
            out | *p9
        } else {
            out
        }
    })
}

pub fn writeback_create_omode(cache: u32, p9_omode: u32) -> u32 {
    if cache & CACHE_WRITEBACK != 0 && p9_omode & P9_OWRITE != 0 {
        (p9_omode & !(P9_OWRITE | P9_DOTL_APPEND)) | P9_ORDWR
    } else {
        p9_omode
    }
}

pub fn v9fs_stat2inode_dotl(inode: &mut InodeSnapshot, stat: &P9StatDotl, flags: u32) {
    if stat.st_result_mask & P9_STATS_BASIC == P9_STATS_BASIC {
        inode.atime_sec = stat.st_atime_sec;
        inode.atime_nsec = stat.st_atime_nsec;
        inode.mtime_sec = stat.st_mtime_sec;
        inode.mtime_nsec = stat.st_mtime_nsec;
        inode.ctime_sec = stat.st_ctime_sec;
        inode.ctime_nsec = stat.st_ctime_nsec;
        inode.uid = stat.st_uid;
        inode.gid = stat.st_gid;
        inode.nlink = stat.st_nlink;
        inode.mode = (stat.st_mode & 0o7777) | (inode.mode & !0o7777);
        if flags & V9FS_STAT2INODE_KEEP_ISIZE == 0 {
            inode.size = stat.st_size;
        }
        inode.blocks = stat.st_blocks;
    } else {
        if stat.st_result_mask & P9_STATS_ATIME != 0 {
            inode.atime_sec = stat.st_atime_sec;
            inode.atime_nsec = stat.st_atime_nsec;
        }
        if stat.st_result_mask & P9_STATS_MTIME != 0 {
            inode.mtime_sec = stat.st_mtime_sec;
            inode.mtime_nsec = stat.st_mtime_nsec;
        }
        if stat.st_result_mask & P9_STATS_CTIME != 0 {
            inode.ctime_sec = stat.st_ctime_sec;
            inode.ctime_nsec = stat.st_ctime_nsec;
        }
        if stat.st_result_mask & P9_STATS_UID != 0 {
            inode.uid = stat.st_uid;
        }
        if stat.st_result_mask & P9_STATS_GID != 0 {
            inode.gid = stat.st_gid;
        }
        if stat.st_result_mask & P9_STATS_NLINK != 0 {
            inode.nlink = stat.st_nlink;
        }
        if stat.st_result_mask & P9_STATS_MODE != 0 {
            inode.mode = (stat.st_mode & 0o7777) | (inode.mode & !0o7777);
        }
        if flags & V9FS_STAT2INODE_KEEP_ISIZE == 0 && stat.st_result_mask & P9_STATS_SIZE != 0 {
            inode.size = stat.st_size;
        }
        if stat.st_result_mask & P9_STATS_BLOCKS != 0 {
            inode.blocks = stat.st_blocks;
        }
    }
    if stat.st_result_mask & P9_STATS_GEN != 0 {
        inode.generation = stat.st_gen;
    }
    inode.cache_validity &= !V9FS_INO_INVALID_ATTR;
}

pub fn refresh_dotl_keep_isize(cache: u32) -> u32 {
    if cache & CACHE_LOOSE != 0 {
        V9FS_STAT2INODE_KEEP_ISIZE
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::fcntl::{O_RDONLY, O_RDWR, O_WRONLY};

    #[test]
    fn vfs_inode_dotl_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/vfs_inode_dotl.c"
        ));
        assert!(source.contains("static kgid_t v9fs_get_fsgid_for_create"));
        assert!(source.contains("if (dir_inode->i_mode & S_ISGID)"));
        assert!(source.contains("struct dotl_openflag_map"));
        assert!(source.contains("{ O_CREAT,\tP9_DOTL_CREATE }"));
        assert!(source.contains("int v9fs_open_to_dotl_flags(int flags)"));
        assert!(source.contains("rflags |= flags & O_ACCMODE;"));
        assert!(source.contains("struct dotl_iattr_map"));
        assert!(source.contains("{ ATTR_SIZE,\t\tP9_ATTR_SIZE }"));
        assert!(source.contains("v9fs_stat2inode_dotl(struct p9_stat_dotl *stat"));
        assert!(source.contains("if ((stat->st_result_mask & P9_STATS_BASIC) == P9_STATS_BASIC)"));
        assert!(source.contains("if (stat->st_result_mask & P9_STATS_GEN)"));

        assert_eq!(
            v9fs_open_to_dotl_flags(O_RDWR | O_CREAT | O_APPEND | O_CLOEXEC),
            O_RDWR | P9_DOTL_CREATE | P9_DOTL_APPEND | P9_DOTL_CLOEXEC
        );
        assert_eq!(v9fs_open_to_dotl_flags(O_RDONLY), 0);
        assert_eq!(v9fs_open_to_dotl_flags(O_WRONLY), P9_OWRITE);
        assert_eq!(v9fs_get_fsgid_for_create(S_ISGID | 0o755, 44, 55), 44);
        assert_eq!(v9fs_get_fsgid_for_create(0o755, 44, 55), 55);
        assert_eq!(
            v9fs_mapped_iattr_valid(ATTR_MODE | ATTR_SIZE | ATTR_MTIME_SET),
            P9_ATTR_MODE | P9_ATTR_SIZE | P9_ATTR_MTIME_SET
        );
        assert_eq!(
            writeback_create_omode(CACHE_WRITEBACK, P9_OWRITE | P9_DOTL_APPEND),
            P9_ORDWR
        );

        let mut inode = InodeSnapshot {
            mode: crate::include::uapi::stat::S_IFREG | 0o600,
            cache_validity: V9FS_INO_INVALID_ATTR,
            ..InodeSnapshot::default()
        };
        let stat = P9StatDotl {
            qid: P9Qid {
                ty: 0,
                version: 1,
                path: 1,
            },
            st_result_mask: P9_STATS_BASIC | P9_STATS_GEN,
            st_mode: 0o644,
            st_uid: 1,
            st_gid: 2,
            st_nlink: 3,
            st_size: 4,
            st_blocks: 5,
            st_atime_sec: 6,
            st_atime_nsec: 7,
            st_mtime_sec: 8,
            st_mtime_nsec: 9,
            st_ctime_sec: 10,
            st_ctime_nsec: 11,
            st_gen: 12,
        };
        v9fs_stat2inode_dotl(&mut inode, &stat, 0);
        assert_eq!(inode.mode, crate::include::uapi::stat::S_IFREG | 0o644);
        assert_eq!(inode.uid, 1);
        assert_eq!(inode.nlink, 3);
        assert_eq!(inode.size, 4);
        assert_eq!(inode.generation, 12);
        assert_eq!(inode.cache_validity, 0);
    }
}
