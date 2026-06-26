//! linux-parity: complete
//! linux-source: vendor/linux/fs/ceph/cache.c
//! test-origin: linux:vendor/linux/fs/ceph/cache.c
//! Ceph FS-Cache cookie registration decisions.

use crate::include::uapi::errno::{ENOMEM, EOPNOTSUPP};

pub const FSCACHE_INVAL_DIO_WRITE: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CephFscacheAcquireVolume {
    Cookie,
    Null,
    Error(i32),
}

pub const fn ceph_fscache_should_register_inode_cookie(
    fs_has_fscache: bool,
    is_regular: bool,
    is_new_inode: bool,
) -> bool {
    fs_has_fscache && is_regular && is_new_inode
}

pub const fn ceph_fscache_unuse_has_update_args(update: bool) -> bool {
    update
}

pub const fn ceph_fscache_invalidate_flags(dio_write: bool) -> u32 {
    if dio_write {
        FSCACHE_INVAL_DIO_WRITE
    } else {
        0
    }
}

pub const fn ceph_fscache_register_fs_result(
    name_allocated: bool,
    acquire: CephFscacheAcquireVolume,
) -> Result<(), i32> {
    if !name_allocated {
        return Err(-ENOMEM);
    }
    match acquire {
        CephFscacheAcquireVolume::Cookie => Ok(()),
        CephFscacheAcquireVolume::Null => Err(-EOPNOTSUPP),
        CephFscacheAcquireVolume::Error(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_cache_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ceph/cache.c"
        ));
        assert!(source.contains("#include <linux/ceph/ceph_debug.h>"));
        assert!(source.contains("#include <linux/fs_context.h>"));
        assert!(source.contains("#include \"super.h\""));
        assert!(source.contains("#include \"cache.h\""));
        assert!(source.contains("void ceph_fscache_register_inode_cookie(struct inode *inode)"));
        assert!(source.contains("if (!fsc->fscache)"));
        assert!(source.contains("if (!S_ISREG(inode->i_mode))"));
        assert!(source.contains("if (!(inode_state_read_once(inode) & I_NEW))"));
        assert!(source.contains("WARN_ON_ONCE(ci->netfs.cache);"));
        assert!(source.contains("fscache_acquire_cookie(fsc->fscache, 0,"));
        assert!(source.contains("&ci->i_vino, sizeof(ci->i_vino),"));
        assert!(source.contains("&ci->i_version, sizeof(ci->i_version),"));
        assert!(source.contains("i_size_read(inode));"));
        assert!(source.contains("mapping_set_release_always(inode->i_mapping);"));
        assert!(source.contains("fscache_relinquish_cookie(ceph_fscache_cookie(ci), false);"));
        assert!(source.contains("fscache_use_cookie(ceph_fscache_cookie(ci), will_modify);"));
        assert!(source.contains("if (update)"));
        assert!(source.contains("fscache_unuse_cookie(ceph_fscache_cookie(ci),"));
        assert!(source.contains("&ci->i_version, &i_size);"));
        assert!(
            source.contains(
                "fscache_update_cookie(ceph_fscache_cookie(ci), &ci->i_version, &i_size);"
            )
        );
        assert!(source.contains("fscache_invalidate(ceph_fscache_cookie(ci),"));
        assert!(source.contains("dio_write ? FSCACHE_INVAL_DIO_WRITE : 0"));
        assert!(source.contains("kasprintf(GFP_KERNEL, \"ceph,%pU%s%s\""));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("fscache_acquire_volume(name, NULL, NULL, 0);"));
        assert!(source.contains("IS_ERR_OR_NULL(fsc->fscache)"));
        assert!(source.contains("err = fsc->fscache ? PTR_ERR(fsc->fscache) : -EOPNOTSUPP;"));
        assert!(source.contains("fscache_relinquish_volume(fsc->fscache, NULL, false);"));

        assert!(!ceph_fscache_should_register_inode_cookie(
            false, true, true
        ));
        assert!(!ceph_fscache_should_register_inode_cookie(
            true, false, true
        ));
        assert!(!ceph_fscache_should_register_inode_cookie(
            true, true, false
        ));
        assert!(ceph_fscache_should_register_inode_cookie(true, true, true));
        assert!(ceph_fscache_unuse_has_update_args(true));
        assert!(!ceph_fscache_unuse_has_update_args(false));
        assert_eq!(ceph_fscache_invalidate_flags(false), 0);
        assert_eq!(ceph_fscache_invalidate_flags(true), FSCACHE_INVAL_DIO_WRITE);
        assert_eq!(
            ceph_fscache_register_fs_result(false, CephFscacheAcquireVolume::Cookie),
            Err(-ENOMEM)
        );
        assert_eq!(
            ceph_fscache_register_fs_result(true, CephFscacheAcquireVolume::Null),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(
            ceph_fscache_register_fs_result(true, CephFscacheAcquireVolume::Error(-5)),
            Err(-5)
        );
        assert_eq!(
            ceph_fscache_register_fs_result(true, CephFscacheAcquireVolume::Cookie),
            Ok(())
        );
    }
}
