//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs/nfs4getroot.c
//! test-origin: linux:vendor/linux/fs/nfs/nfs4getroot.c
//! NFSv4 root filehandle validation.

use crate::include::uapi::errno::{ENOMEM, ENOTDIR};
use crate::include::uapi::stat;

pub const NFS_ATTR_FATTR_TYPE: u64 = 1 << 0;
pub const NFSDBG_FACILITY: &str = "NFSDBG_CLIENT";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfsFattr {
    pub valid: u64,
    pub mode: u32,
    pub fsid: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Nfs4GetRootfhOutcome {
    pub ret: i32,
    pub copied_fsid: Option<u64>,
    pub freed_fattr: bool,
}

pub const fn nfs4_get_rootfh_outcome(
    fattr_allocated: bool,
    proc_ret: i32,
    fattr: NfsFattr,
) -> Nfs4GetRootfhOutcome {
    if !fattr_allocated {
        return Nfs4GetRootfhOutcome {
            ret: -ENOMEM,
            copied_fsid: None,
            freed_fattr: false,
        };
    }
    if proc_ret < 0 {
        return Nfs4GetRootfhOutcome {
            ret: proc_ret,
            copied_fsid: None,
            freed_fattr: true,
        };
    }
    if (fattr.valid & NFS_ATTR_FATTR_TYPE) == 0 || !stat::is_dir(fattr.mode) {
        return Nfs4GetRootfhOutcome {
            ret: -ENOTDIR,
            copied_fsid: None,
            freed_fattr: true,
        };
    }
    Nfs4GetRootfhOutcome {
        ret: 0,
        copied_fsid: Some(fattr.fsid),
        freed_fattr: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs4_get_rootfh_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfs/nfs4getroot.c"
        ));
        assert!(source.contains("#include <linux/nfs_fs.h>"));
        assert!(source.contains("#include \"nfs4_fs.h\""));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("#define NFSDBG_FACILITY\t\tNFSDBG_CLIENT"));
        assert!(source.contains("struct nfs_fattr *fattr = nfs_alloc_fattr();"));
        assert!(source.contains("int ret = -ENOMEM;"));
        assert!(source.contains("ret = nfs4_proc_get_rootfh(server, mntfh, fattr, auth_probe);"));
        assert!(source.contains("fattr->valid & NFS_ATTR_FATTR_TYPE"));
        assert!(source.contains("!S_ISDIR(fattr->mode)"));
        assert!(source.contains("ret = -ENOTDIR;"));
        assert!(source.contains("memcpy(&server->fsid, &fattr->fsid, sizeof(server->fsid));"));
        assert!(source.contains("nfs_free_fattr(fattr);"));

        let dir = NfsFattr {
            valid: NFS_ATTR_FATTR_TYPE,
            mode: stat::S_IFDIR,
            fsid: 99,
        };
        assert_eq!(
            nfs4_get_rootfh_outcome(false, 0, dir),
            Nfs4GetRootfhOutcome {
                ret: -ENOMEM,
                copied_fsid: None,
                freed_fattr: false,
            }
        );
        assert_eq!(nfs4_get_rootfh_outcome(true, -5, dir).ret, -5);
        assert_eq!(
            nfs4_get_rootfh_outcome(true, 0, NfsFattr { valid: 0, ..dir }).ret,
            -ENOTDIR
        );
        assert_eq!(
            nfs4_get_rootfh_outcome(
                true,
                0,
                NfsFattr {
                    mode: stat::S_IFREG,
                    ..dir
                },
            )
            .ret,
            -ENOTDIR
        );
        assert_eq!(nfs4_get_rootfh_outcome(true, 0, dir).copied_fsid, Some(99));
    }
}
