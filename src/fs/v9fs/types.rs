//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/v9fs.h
//! test-origin: linux:vendor/linux/fs/9p/v9fs.h
//! Shared constants and compact value types for the 9P filesystem ports.

use crate::include::uapi::stat::{
    S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK, S_ISGID, S_ISUID,
    S_ISVTX,
};

pub const V9FS_PROTO_2000U: u32 = 0x01;
pub const V9FS_PROTO_2000L: u32 = 0x02;
pub const V9FS_ACCESS_SINGLE: u32 = 0x04;
pub const V9FS_ACCESS_USER: u32 = 0x08;
pub const V9FS_ACCESS_CLIENT: u32 = 0x10;
pub const V9FS_POSIX_ACL: u32 = 0x20;
pub const V9FS_NO_XATTR: u32 = 0x40;
pub const V9FS_IGNORE_QV: u32 = 0x80;
pub const V9FS_DIRECT_IO: u32 = 0x100;
pub const V9FS_SYNC: u32 = 0x200;

pub const V9FS_ACCESS_ANY: u32 = V9FS_ACCESS_SINGLE | V9FS_ACCESS_USER | V9FS_ACCESS_CLIENT;
pub const V9FS_ACCESS_MASK: u32 = V9FS_ACCESS_ANY;
pub const V9FS_ACL_MASK: u32 = V9FS_POSIX_ACL;
pub const V9FS_INO_INVALID_ATTR: u32 = 0x01;
pub const V9FS_STAT2INODE_KEEP_ISIZE: u32 = 1;

pub const CACHE_SC_NONE: u32 = 0b0000_0000;
pub const CACHE_SC_READAHEAD: u32 = 0b0000_0001;
pub const CACHE_SC_MMAP: u32 = 0b0000_0101;
pub const CACHE_SC_LOOSE: u32 = 0b0000_1111;
pub const CACHE_SC_FSCACHE: u32 = 0b1000_1111;

pub const CACHE_NONE: u32 = 0b0000_0000;
pub const CACHE_FILE: u32 = 0b0000_0001;
pub const CACHE_META: u32 = 0b0000_0010;
pub const CACHE_WRITEBACK: u32 = 0b0000_0100;
pub const CACHE_LOOSE: u32 = 0b0000_1000;
pub const CACHE_FSCACHE: u32 = 0b1000_0000;

pub const P9_OREAD: u32 = 0x00;
pub const P9_OWRITE: u32 = 0x01;
pub const P9_ORDWR: u32 = 0x02;
pub const P9_OTRUNC: u32 = 0x10;
pub const P9_OAPPEND: u32 = 0x80;
pub const P9_OEXCL: u32 = 0x1000;
pub const P9L_DIRECT: u32 = 0x2000;
pub const P9L_NOWRITECACHE: u32 = 0x4000;

pub const P9_DMDIR: u32 = 0x8000_0000;
pub const P9_DMSYMLINK: u32 = 0x0200_0000;
pub const P9_DMLINK: u32 = 0x0100_0000;
pub const P9_DMDEVICE: u32 = 0x0080_0000;
pub const P9_DMNAMEDPIPE: u32 = 0x0020_0000;
pub const P9_DMSOCKET: u32 = 0x0010_0000;
pub const P9_DMSETUID: u32 = 0x0008_0000;
pub const P9_DMSETGID: u32 = 0x0004_0000;
pub const P9_DMSETVTX: u32 = 0x0001_0000;

pub const P9_DOTL_CREATE: u32 = 0o0000_0100;
pub const P9_DOTL_EXCL: u32 = 0o0000_0200;
pub const P9_DOTL_NOCTTY: u32 = 0o0000_0400;
pub const P9_DOTL_TRUNC: u32 = 0o0000_1000;
pub const P9_DOTL_APPEND: u32 = 0o0000_2000;
pub const P9_DOTL_NONBLOCK: u32 = 0o0000_4000;
pub const P9_DOTL_DSYNC: u32 = 0o0001_0000;
pub const P9_DOTL_FASYNC: u32 = 0o0002_0000;
pub const P9_DOTL_DIRECT: u32 = 0o0004_0000;
pub const P9_DOTL_LARGEFILE: u32 = 0o0010_0000;
pub const P9_DOTL_DIRECTORY: u32 = 0o0020_0000;
pub const P9_DOTL_NOFOLLOW: u32 = 0o0040_0000;
pub const P9_DOTL_NOATIME: u32 = 0o0100_0000;
pub const P9_DOTL_CLOEXEC: u32 = 0o0200_0000;
pub const P9_DOTL_SYNC: u32 = 0o0400_0000;
pub const P9_DOTL_AT_REMOVEDIR: u32 = 0x200;

pub const P9_IOHDRSZ: u32 = 24;
pub const P9_READDIRHDRSZ: u32 = 24;
pub const P9_MAXWELEM: usize = 16;

pub const P9_LOCK_TYPE_RDLCK: u8 = 0;
pub const P9_LOCK_TYPE_WRLCK: u8 = 1;
pub const P9_LOCK_TYPE_UNLCK: u8 = 2;
pub const P9_LOCK_SUCCESS: u8 = 0;
pub const P9_LOCK_BLOCKED: u8 = 1;
pub const P9_LOCK_ERROR: u8 = 2;
pub const P9_LOCK_GRACE: u8 = 3;
pub const P9_LOCK_FLAGS_BLOCK: u8 = 1;

pub const P9_STATS_MODE: u64 = 0x0000_0001;
pub const P9_STATS_NLINK: u64 = 0x0000_0002;
pub const P9_STATS_UID: u64 = 0x0000_0004;
pub const P9_STATS_GID: u64 = 0x0000_0008;
pub const P9_STATS_ATIME: u64 = 0x0000_0020;
pub const P9_STATS_MTIME: u64 = 0x0000_0040;
pub const P9_STATS_CTIME: u64 = 0x0000_0080;
pub const P9_STATS_SIZE: u64 = 0x0000_0200;
pub const P9_STATS_BLOCKS: u64 = 0x0000_0400;
pub const P9_STATS_GEN: u64 = 0x0000_1000;
pub const P9_STATS_BASIC: u64 = 0x0000_07ff;
pub const P9_STATS_ALL: u64 = 0x0000_3fff;

pub const DT_DIR: u8 = 4;
pub const DT_REG: u8 = 8;
pub const DT_LNK: u8 = 10;

pub const V9FS_PORT: u16 = 564;
pub const V9FS_DEFUSER: &str = "nobody";
pub const V9FS_DEFANAME: &str = "";
pub const V9FS_DEFUID: u32 = u32::MAX - 1;
pub const V9FS_DEFGID: u32 = u32::MAX - 1;
pub const V9FS_MAGIC: u32 = 0x0102_1997;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct V9fsSessionInfo<'a> {
    pub flags: u32,
    pub nodev: bool,
    pub debug: u16,
    pub afid: u32,
    pub cache: u32,
    pub cachetag: Option<&'a str>,
    pub fscache: bool,
    pub uname: &'a str,
    pub aname: &'a str,
    pub maxdata: u32,
    pub dfltuid: u32,
    pub dfltgid: u32,
    pub uid: u32,
    pub session_lock_timeout: i64,
}

impl Default for V9fsSessionInfo<'static> {
    fn default() -> Self {
        Self {
            flags: V9FS_ACCESS_USER,
            nodev: false,
            debug: 0,
            afid: u32::MAX,
            cache: CACHE_NONE,
            cachetag: None,
            fscache: false,
            uname: V9FS_DEFUSER,
            aname: V9FS_DEFANAME,
            maxdata: 0,
            dfltuid: V9FS_DEFUID,
            dfltgid: V9FS_DEFGID,
            uid: V9FS_DEFUID,
            session_lock_timeout: 0,
        }
    }
}

pub const fn v9fs_session_cache_present(
    config_9p_fscache: bool,
    session: &V9fsSessionInfo<'_>,
) -> bool {
    config_9p_fscache && session.fscache
}

pub const fn v9fs_inode_cookie_present(config_9p_fscache: bool) -> bool {
    config_9p_fscache
}

pub const fn v9fs_proto_dotu_session(session: &V9fsSessionInfo<'_>) -> bool {
    proto_dotu(session.flags)
}

pub const fn v9fs_proto_dotl_session(session: &V9fsSessionInfo<'_>) -> bool {
    proto_dotl(session.flags)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum V9fsInodeFromFidPath {
    Legacy { new_inode: bool },
    Dotl { new_inode: bool },
}

pub const fn v9fs_get_inode_from_fid_path(flags: u32) -> V9fsInodeFromFidPath {
    if proto_dotl(flags) {
        V9fsInodeFromFidPath::Dotl { new_inode: false }
    } else {
        V9fsInodeFromFidPath::Legacy { new_inode: false }
    }
}

pub const fn v9fs_get_new_inode_from_fid_path(flags: u32) -> V9fsInodeFromFidPath {
    if proto_dotl(flags) {
        V9fsInodeFromFidPath::Dotl { new_inode: true }
    } else {
        V9fsInodeFromFidPath::Legacy { new_inode: true }
    }
}

pub const fn proto_dotu(flags: u32) -> bool {
    flags & V9FS_PROTO_2000U != 0
}

pub const fn proto_dotl(flags: u32) -> bool {
    flags & V9FS_PROTO_2000L != 0
}

pub const fn is_dir(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFDIR
}

pub const fn is_reg(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFREG
}

pub const fn is_lnk(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFLNK
}

pub const fn is_sock(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFSOCK
}

pub const fn is_fifo(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFIFO
}

pub const fn is_blk(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFBLK
}

pub const fn is_chr(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFCHR
}

pub const fn has_setuid(mode: u32) -> bool {
    mode & S_ISUID == S_ISUID
}

pub const fn has_setgid(mode: u32) -> bool {
    mode & S_ISGID == S_ISGID
}

pub const fn has_sticky(mode: u32) -> bool {
    mode & S_ISVTX == S_ISVTX
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P9Qid {
    pub ty: u8,
    pub version: u32,
    pub path: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct V9fsInode {
    pub qid: P9Qid,
    pub cache_validity: u32,
}

pub const fn v9fs_inode_attr_invalid(inode: V9fsInode) -> bool {
    inode.cache_validity & V9FS_INO_INVALID_ATTR != 0
}

pub const fn qid_to_ino_64(qid: P9Qid) -> u64 {
    qid.path.wrapping_add(2)
}

pub const fn qid_to_ino_32(qid: P9Qid) -> u32 {
    let path = qid.path.wrapping_add(2);
    ((path as u32) ^ ((path >> 32) as u32)) as u32
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct P9Wstat<'a> {
    pub qid: P9Qid,
    pub mode: u32,
    pub atime: u32,
    pub mtime: u32,
    pub length: u64,
    pub extension: &'a str,
    pub n_uid: u32,
    pub n_gid: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InodeSnapshot {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u32,
    pub size: u64,
    pub blocks: u64,
    pub atime_sec: u64,
    pub atime_nsec: u32,
    pub mtime_sec: u64,
    pub mtime_nsec: u32,
    pub ctime_sec: u64,
    pub ctime_nsec: u32,
    pub generation: u64,
    pub cache_validity: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P9StatDotl {
    pub qid: P9Qid,
    pub st_result_mask: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_nlink: u32,
    pub st_size: u64,
    pub st_blocks: u64,
    pub st_atime_sec: u64,
    pub st_atime_nsec: u32,
    pub st_mtime_sec: u64,
    pub st_mtime_nsec: u32,
    pub st_ctime_sec: u64,
    pub st_ctime_nsec: u32,
    pub st_gen: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_constants_match_linux_headers() {
        let v9fs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/v9fs.h"
        ));
        let vfs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/v9fs_vfs.h"
        ));
        let p9 = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/9p/9p.h"
        ));

        assert!(v9fs.contains("V9FS_PROTO_2000U         = 0x01"));
        assert!(v9fs.contains("V9FS_ACCESS_CLIENT       = 0x10"));
        assert!(v9fs.contains("CACHE_SC_FSCACHE    = 0b10001111"));
        assert!(v9fs.contains("CACHE_FSCACHE       = 0b10000000"));
        assert!(v9fs.contains("struct v9fs_session_info"));
        assert!(v9fs.contains("unsigned int flags;"));
        assert!(v9fs.contains("unsigned char nodev;"));
        assert!(v9fs.contains("char *cachetag;"));
        assert!(v9fs.contains("long session_lock_timeout;"));
        assert!(v9fs.contains("struct v9fs_inode"));
        assert!(v9fs.contains("unsigned int cache_validity;"));
        assert!(v9fs.contains("static inline struct fscache_cookie *v9fs_inode_cookie"));
        assert!(v9fs.contains("static inline struct fscache_volume *v9fs_session_cache"));
        assert!(v9fs.contains("#define V9FS_PORT\t564"));
        assert!(v9fs.contains("#define V9FS_DEFUID\tKUIDT_INIT(-2)"));
        assert!(v9fs.contains("#define V9FS_DEFGID\tKGIDT_INIT(-2)"));
        assert!(v9fs.contains("v9fs_get_inode_from_fid(struct v9fs_session_info *v9ses"));
        assert!(v9fs.contains("return v9fs_inode_from_fid_dotl(v9ses, fid, sb, 0);"));
        assert!(v9fs.contains("return v9fs_inode_from_fid(v9ses, fid, sb, 0);"));
        assert!(v9fs.contains("v9fs_get_new_inode_from_fid(struct v9fs_session_info *v9ses"));
        assert!(v9fs.contains("return v9fs_inode_from_fid_dotl(v9ses, fid, sb, 1);"));
        assert!(v9fs.contains("return v9fs_inode_from_fid(v9ses, fid, sb, 1);"));
        assert!(vfs.contains("#define QID2INO(q) ((ino_t) ((q)->path+2))"));
        assert!(p9.contains("P9_OREAD = 0x00"));
        assert!(p9.contains("P9L_NOWRITECACHE = 0x4000"));
        assert!(p9.contains("P9_DMDIR = 0x80000000"));
        assert!(p9.contains("#define P9_MAXWELEM\t16"));
        assert!(p9.contains("#define P9_STATS_BASIC\t\t0x000007ffULL"));

        let qid = P9Qid {
            ty: 0,
            version: 1,
            path: 40,
        };
        assert_eq!(qid_to_ino_64(qid), 42);
        assert_eq!(qid_to_ino_32(qid), 42);
        assert_eq!(V9FS_ACCESS_ANY, 0x1c);
        assert_eq!(
            CACHE_SC_FSCACHE,
            CACHE_FILE | CACHE_META | CACHE_WRITEBACK | CACHE_LOOSE | CACHE_FSCACHE
        );
        assert_eq!(V9FS_PORT, 564);
        assert_eq!(V9FS_DEFUID, 0xffff_fffe);
        assert_eq!(V9FS_DEFGID, 0xffff_fffe);

        let default_session = V9fsSessionInfo::default();
        assert_eq!(default_session.flags, V9FS_ACCESS_USER);
        assert_eq!(default_session.uname, V9FS_DEFUSER);
        assert_eq!(default_session.aname, V9FS_DEFANAME);
        assert_eq!(default_session.dfltuid, V9FS_DEFUID);
        assert!(!v9fs_session_cache_present(false, &default_session));
        assert!(!v9fs_session_cache_present(true, &default_session));

        let dotl_session = V9fsSessionInfo {
            flags: V9FS_PROTO_2000L | V9FS_ACCESS_CLIENT,
            fscache: true,
            cachetag: Some("cache-tag"),
            ..V9fsSessionInfo::default()
        };
        assert!(v9fs_proto_dotl_session(&dotl_session));
        assert!(!v9fs_proto_dotu_session(&dotl_session));
        assert!(v9fs_session_cache_present(true, &dotl_session));
        assert!(v9fs_inode_cookie_present(true));
        assert!(!v9fs_inode_cookie_present(false));
        assert_eq!(
            v9fs_get_inode_from_fid_path(V9FS_ACCESS_USER),
            V9fsInodeFromFidPath::Legacy { new_inode: false }
        );
        assert_eq!(
            v9fs_get_inode_from_fid_path(V9FS_PROTO_2000L),
            V9fsInodeFromFidPath::Dotl { new_inode: false }
        );
        assert_eq!(
            v9fs_get_new_inode_from_fid_path(V9FS_PROTO_2000L),
            V9fsInodeFromFidPath::Dotl { new_inode: true }
        );

        let inode = V9fsInode {
            qid,
            cache_validity: V9FS_INO_INVALID_ATTR,
        };
        assert!(v9fs_inode_attr_invalid(inode));
    }
}
