//! linux-parity: partial
//! linux-source: vendor/linux/fs/9p/vfs_inode.c
//! test-origin: linux:vendor/linux/fs/9p/vfs_inode.c
//! 9P2000 and 9P2000.u inode mode, setattr, and refresh helpers.

use crate::include::uapi::errno::EINVAL;
use crate::include::uapi::fcntl::{
    AT_REMOVEDIR, O_ACCMODE, O_APPEND, O_EXCL, O_RDWR, O_TRUNC, O_WRONLY,
};
use crate::include::uapi::stat::{
    S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK,
};

use super::types::*;

pub const INVALID_UID: u32 = u32::MAX;
pub const INVALID_GID: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum V9fsInodeOps {
    DotlFile,
    LegacyFile,
    DotuDir,
    LegacyDir,
    DotlDir,
    DotuSymlink,
    DotlSymlink,
    Special,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P9UnixMode {
    pub mode: u32,
    pub rdev: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlankWstat {
    pub type_: u16,
    pub dev: u32,
    pub qid: P9Qid,
    pub mode: u32,
    pub atime: u32,
    pub mtime: u32,
    pub length: u64,
    pub name_is_null: bool,
    pub uid_is_null: bool,
    pub gid_is_null: bool,
    pub muid_is_null: bool,
    pub n_uid: u32,
    pub n_gid: u32,
    pub n_muid: u32,
    pub extension_is_null: bool,
}

pub fn unixmode2p9mode(session_flags: u32, nodev: bool, mode: u32) -> u32 {
    let mut res = mode & 0o777;
    if is_dir(mode) {
        res |= P9_DMDIR;
    }
    if proto_dotu(session_flags) {
        if !nodev {
            if is_sock(mode) {
                res |= P9_DMSOCKET;
            }
            if is_fifo(mode) {
                res |= P9_DMNAMEDPIPE;
            }
            if is_blk(mode) || is_chr(mode) {
                res |= P9_DMDEVICE;
            }
        }
        if has_setuid(mode) {
            res |= P9_DMSETUID;
        }
        if has_setgid(mode) {
            res |= P9_DMSETGID;
        }
        if has_sticky(mode) {
            res |= P9_DMSETVTX;
        }
    }
    res
}

pub fn p9mode2perm(session_flags: u32, stat_mode: u32) -> u32 {
    let mut res = stat_mode & 0o777;
    if proto_dotu(session_flags) {
        if stat_mode & P9_DMSETUID == P9_DMSETUID {
            res |= crate::include::uapi::stat::S_ISUID;
        }
        if stat_mode & P9_DMSETGID == P9_DMSETGID {
            res |= crate::include::uapi::stat::S_ISGID;
        }
        if stat_mode & P9_DMSETVTX == P9_DMSETVTX {
            res |= crate::include::uapi::stat::S_ISVTX;
        }
    }
    res
}

pub fn p9mode2unixmode(
    session_flags: u32,
    nodev: bool,
    stat_mode: u32,
    extension: &str,
) -> P9UnixMode {
    let mut res = p9mode2perm(session_flags, stat_mode);
    let mut rdev = None;

    if stat_mode & P9_DMDIR == P9_DMDIR {
        res |= S_IFDIR;
    } else if stat_mode & P9_DMSYMLINK != 0 && proto_dotu(session_flags) {
        res |= S_IFLNK;
    } else if stat_mode & P9_DMSOCKET != 0 && proto_dotu(session_flags) && !nodev {
        res |= S_IFSOCK;
    } else if stat_mode & P9_DMNAMEDPIPE != 0 && proto_dotu(session_flags) && !nodev {
        res |= S_IFIFO;
    } else if stat_mode & P9_DMDEVICE != 0 && proto_dotu(session_flags) && !nodev {
        if let Some((kind, major, minor)) = parse_device_extension(extension) {
            match kind {
                'c' => res |= S_IFCHR,
                'b' => res |= S_IFBLK,
                _ => {}
            }
            if kind == 'c' || kind == 'b' {
                rdev = Some((major, minor));
            }
        }
    } else {
        res |= S_IFREG;
    }

    P9UnixMode { mode: res, rdev }
}

fn parse_device_extension(extension: &str) -> Option<(char, u32, u32)> {
    let mut parts = extension.split_whitespace();
    let kind = parts.next()?.chars().next()?;
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((kind, major, minor))
}

pub fn v9fs_uflags2omode(uflags: u32, extended: bool) -> u32 {
    let mut ret = match uflags & O_ACCMODE {
        O_WRONLY => P9_OWRITE,
        O_RDWR => P9_ORDWR,
        _ => P9_OREAD,
    };
    if uflags & O_TRUNC != 0 {
        ret |= P9_OTRUNC;
    }
    if extended {
        if uflags & O_EXCL != 0 {
            ret |= P9_OEXCL;
        }
        if uflags & O_APPEND != 0 {
            ret |= P9_OAPPEND;
        }
    }
    ret
}

pub fn v9fs_blank_wstat() -> BlankWstat {
    BlankWstat {
        type_: !0,
        dev: !0,
        qid: P9Qid {
            ty: !0,
            version: !0,
            path: !0,
        },
        mode: !0,
        atime: !0,
        mtime: !0,
        length: !0,
        name_is_null: true,
        uid_is_null: true,
        gid_is_null: true,
        muid_is_null: true,
        n_uid: INVALID_UID,
        n_gid: INVALID_GID,
        n_muid: INVALID_UID,
        extension_is_null: true,
    }
}

pub fn init_inode_ops(session_flags: u32, mode: u32) -> Result<V9fsInodeOps, i32> {
    match mode & S_IFMT {
        S_IFIFO | S_IFBLK | S_IFCHR | S_IFSOCK => {
            if proto_dotl(session_flags) || proto_dotu(session_flags) {
                Ok(V9fsInodeOps::Special)
            } else {
                Err(-EINVAL)
            }
        }
        S_IFREG => {
            if proto_dotl(session_flags) {
                Ok(V9fsInodeOps::DotlFile)
            } else {
                Ok(V9fsInodeOps::LegacyFile)
            }
        }
        S_IFLNK => {
            if proto_dotl(session_flags) {
                Ok(V9fsInodeOps::DotlSymlink)
            } else if proto_dotu(session_flags) {
                Ok(V9fsInodeOps::DotuSymlink)
            } else {
                Err(-EINVAL)
            }
        }
        S_IFDIR => {
            if proto_dotl(session_flags) {
                Ok(V9fsInodeOps::DotlDir)
            } else if proto_dotu(session_flags) {
                Ok(V9fsInodeOps::DotuDir)
            } else {
                Ok(V9fsInodeOps::LegacyDir)
            }
        }
        _ => Err(-EINVAL),
    }
}

pub fn at_to_dotl_flags(flags: u32) -> u32 {
    if flags & AT_REMOVEDIR != 0 {
        P9_DOTL_AT_REMOVEDIR
    } else {
        0
    }
}

pub const fn dec_count_nlink(mode: u32, nlink: u32) -> u32 {
    if (mode & S_IFMT) != S_IFDIR || nlink > 2 {
        nlink.saturating_sub(1)
    } else {
        nlink
    }
}

pub fn v9fs_stat2inode(
    inode: &mut InodeSnapshot,
    session_flags: u32,
    dfltuid: u32,
    dfltgid: u32,
    stat: &P9Wstat<'_>,
    flags: u32,
) {
    inode.atime_sec = stat.atime as u64;
    inode.atime_nsec = 0;
    inode.mtime_sec = stat.mtime as u64;
    inode.mtime_nsec = 0;
    inode.ctime_sec = stat.mtime as u64;
    inode.ctime_nsec = 0;
    inode.uid = dfltuid;
    inode.gid = dfltgid;
    if proto_dotu(session_flags) {
        inode.uid = stat.n_uid;
        inode.gid = stat.n_gid;
    }
    let mode = p9mode2perm(session_flags, stat.mode);
    inode.mode = mode | (inode.mode & !0o777);
    if flags & V9FS_STAT2INODE_KEEP_ISIZE == 0 {
        inode.size = stat.length;
    }
    inode.blocks = stat.length.div_ceil(512);
    inode.cache_validity &= !V9FS_INO_INVALID_ATTR;
}

pub fn refresh_inode_should_update(old_mode: u32, new_mode: u32) -> bool {
    (old_mode & S_IFMT) == (new_mode & S_IFMT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::fcntl::{O_APPEND, O_EXCL, O_RDONLY};

    #[test]
    fn vfs_inode_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/vfs_inode.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/9p/9p.h"
        ));
        assert!(source.contains("static u32 unixmode2p9mode"));
        assert!(source.contains("if (S_ISDIR(mode))"));
        assert!(source.contains("res |= P9_DMDIR;"));
        assert!(source.contains("if (v9fs_proto_dotu(v9ses))"));
        assert!(source.contains("if (S_ISSOCK(mode))"));
        assert!(source.contains("res |= P9_DMSOCKET;"));
        assert!(source.contains("static umode_t p9mode2unixmode"));
        assert!(source.contains("sscanf(stat->extension, \"%c %i %i\""));
        assert!(source.contains("int v9fs_uflags2omode(int uflags, int extended)"));
        assert!(source.contains("case O_WRONLY:"));
        assert!(source.contains("ret |= P9_OTRUNC;"));
        assert!(source.contains("v9fs_blank_wstat(struct p9_wstat *wstat)"));
        assert!(source.contains("wstat->qid.type = ~0;"));
        assert!(source.contains("static int v9fs_at_to_dotl_flags"));
        assert!(source.contains("if (flags & AT_REMOVEDIR)"));
        assert!(source.contains("v9fs_stat2inode(struct p9_wstat *stat"));
        assert!(source.contains("mode = p9mode2perm(v9ses, stat);"));
        assert!(header.contains("P9_DMDEVICE = 0x00800000"));

        let dotu = V9FS_PROTO_2000U;
        assert_eq!(
            unixmode2p9mode(dotu, false, S_IFDIR | 0o755),
            P9_DMDIR | 0o755
        );
        assert_eq!(
            unixmode2p9mode(dotu, false, S_IFSOCK | 0o600),
            P9_DMSOCKET | 0o600
        );
        assert_eq!(
            p9mode2unixmode(dotu, false, P9_DMDEVICE | 0o644, "c 1 3"),
            P9UnixMode {
                mode: S_IFCHR | 0o644,
                rdev: Some((1, 3))
            }
        );
        assert_eq!(
            v9fs_uflags2omode(O_RDONLY | O_TRUNC | O_EXCL | O_APPEND, true),
            P9_OREAD | P9_OTRUNC | P9_OEXCL | P9_OAPPEND
        );
        assert_eq!(v9fs_uflags2omode(O_WRONLY, false), P9_OWRITE);
        assert_eq!(init_inode_ops(0, S_IFLNK | 0o777), Err(-EINVAL));
        assert_eq!(
            init_inode_ops(V9FS_PROTO_2000L, S_IFDIR | 0o755),
            Ok(V9fsInodeOps::DotlDir)
        );
        assert_eq!(at_to_dotl_flags(AT_REMOVEDIR), P9_DOTL_AT_REMOVEDIR);
        assert_eq!(dec_count_nlink(S_IFDIR, 2), 2);
        assert_eq!(dec_count_nlink(S_IFREG, 1), 0);

        let blank = v9fs_blank_wstat();
        assert_eq!(blank.qid.ty, u8::MAX);
        assert_eq!(blank.n_uid, INVALID_UID);

        let mut inode = InodeSnapshot {
            mode: S_IFREG | 0o600,
            cache_validity: V9FS_INO_INVALID_ATTR,
            ..InodeSnapshot::default()
        };
        let stat = P9Wstat {
            qid: P9Qid {
                ty: 0,
                version: 1,
                path: 2,
            },
            mode: 0o644,
            atime: 1,
            mtime: 2,
            length: 513,
            extension: "",
            n_uid: 7,
            n_gid: 8,
        };
        v9fs_stat2inode(&mut inode, V9FS_PROTO_2000U, 99, 100, &stat, 0);
        assert_eq!(inode.mode, S_IFREG | 0o644);
        assert_eq!(inode.uid, 7);
        assert_eq!(inode.gid, 8);
        assert_eq!(inode.blocks, 2);
        assert_eq!(inode.cache_validity, 0);
    }
}
