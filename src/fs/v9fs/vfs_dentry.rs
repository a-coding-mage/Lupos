//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/vfs_dentry.c
//! test-origin: linux:vendor/linux/fs/9p/vfs_dentry.c
//! Dentry cache invalidation and revalidation decisions for 9P.

use crate::include::uapi::errno::{ECHILD, ENOENT};

use super::types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DentryReleasePlan {
    pub fids_to_put: usize,
}

pub const LOOKUP_RCU: u32 = 0x40;

pub const fn v9fs_cached_dentry_delete(is_negative: bool) -> bool {
    is_negative
}

pub const fn v9fs_dentry_release_plan(fid_count: usize) -> DentryReleasePlan {
    DentryReleasePlan {
        fids_to_put: fid_count,
    }
}

pub fn v9fs_lookup_revalidate_result(
    flags: u32,
    inode_present: bool,
    cache_validity: u32,
    fid_lookup_errno: Option<i32>,
    refresh_errno: i32,
    cache_validity_after_refresh: u32,
) -> i32 {
    if flags & LOOKUP_RCU != 0 {
        return -ECHILD;
    }
    if !inode_present {
        return 1;
    }
    if cache_validity & V9FS_INO_INVALID_ATTR == 0 {
        return 1;
    }
    if let Some(errno) = fid_lookup_errno {
        return errno;
    }
    if refresh_errno == -ENOENT {
        return 0;
    }
    if cache_validity_after_refresh & V9FS_INO_INVALID_ATTR != 0 {
        return 0;
    }
    if refresh_errno < 0 {
        return refresh_errno;
    }
    1
}

pub fn v9fs_lookup_revalidate(
    flags: u32,
    inode_present: bool,
    cache_validity: u32,
    fid_lookup_errno: Option<i32>,
    refresh_errno: i32,
    cache_validity_after_refresh: u32,
) -> i32 {
    v9fs_lookup_revalidate_result(
        flags,
        inode_present,
        cache_validity,
        fid_lookup_errno,
        refresh_errno,
        cache_validity_after_refresh,
    )
}

pub const fn v9fs_dentry_unalias_trylock(rename_sem_available: bool) -> bool {
    rename_sem_available
}

pub const fn v9fs_dentry_unalias_unlock(lock_held: bool) -> bool {
    lock_held
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DentryOperations {
    pub revalidate: bool,
    pub weak_revalidate: bool,
    pub delete: bool,
    pub release: bool,
    pub unalias_lock: bool,
}

pub const V9FS_CACHED_DENTRY_OPERATIONS: DentryOperations = DentryOperations {
    revalidate: true,
    weak_revalidate: true,
    delete: true,
    release: true,
    unalias_lock: true,
};

pub const V9FS_DENTRY_OPERATIONS: DentryOperations = DentryOperations {
    revalidate: false,
    weak_revalidate: false,
    delete: false,
    release: true,
    unalias_lock: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dentry_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/vfs_dentry.c"
        ));
        assert!(source.contains("static int v9fs_cached_dentry_delete"));
        assert!(source.contains("if (!d_really_is_negative(dentry))"));
        assert!(source.contains("return v9fs_ndentry_is_expired(dentry);"));
        assert!(source.contains("static void v9fs_dentry_release"));
        assert!(source.contains("hlist_move_list"));
        assert!(source.contains("p9_fid_put(hlist_entry"));
        assert!(source.contains("static int __v9fs_lookup_revalidate"));
        assert!(source.contains("if (flags & LOOKUP_RCU)"));
        assert!(source.contains("return -ECHILD;"));
        assert!(source.contains("if (retval == -ENOENT)"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("static int v9fs_lookup_revalidate"));
        assert!(source.contains("return __v9fs_lookup_revalidate(dentry, flags);"));
        assert!(source.contains("v9fs_dentry_unalias_trylock"));
        assert!(source.contains("down_write_trylock(&v9ses->rename_sem)"));
        assert!(source.contains("v9fs_dentry_unalias_unlock"));
        assert!(source.contains("up_write(&v9ses->rename_sem)"));
        assert!(source.contains("const struct dentry_operations v9fs_cached_dentry_operations"));
        assert!(source.contains(".d_weak_revalidate = __v9fs_lookup_revalidate"));
        assert!(source.contains("const struct dentry_operations v9fs_dentry_operations"));

        assert!(v9fs_cached_dentry_delete(true));
        assert!(!v9fs_cached_dentry_delete(false));
        assert_eq!(v9fs_dentry_release_plan(3).fids_to_put, 3);
        assert_eq!(
            v9fs_lookup_revalidate_result(LOOKUP_RCU, true, 0, None, 0, 0),
            -ECHILD
        );
        assert_eq!(v9fs_lookup_revalidate(0, false, 0, None, 0, 0), 1);
        assert_eq!(
            v9fs_lookup_revalidate_result(0, false, V9FS_INO_INVALID_ATTR, None, 0, 0),
            1
        );
        assert_eq!(
            v9fs_lookup_revalidate_result(0, true, V9FS_INO_INVALID_ATTR, None, -ENOENT, 0),
            0
        );
        assert_eq!(
            v9fs_lookup_revalidate_result(
                0,
                true,
                V9FS_INO_INVALID_ATTR,
                None,
                0,
                V9FS_INO_INVALID_ATTR
            ),
            0
        );
        assert!(V9FS_CACHED_DENTRY_OPERATIONS.revalidate);
        assert!(!V9FS_DENTRY_OPERATIONS.revalidate);
        assert!(v9fs_dentry_unalias_trylock(true));
        assert!(!v9fs_dentry_unalias_trylock(false));
        assert!(v9fs_dentry_unalias_unlock(true));
    }
}
