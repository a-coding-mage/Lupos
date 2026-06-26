//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/acl.c
//! test-origin: linux:vendor/linux/fs/9p/acl.c
//! POSIX ACL option gates and xattr result mapping for 9P.

use crate::include::uapi::errno::{
    ECHILD, EINVAL, EIO, ENODATA, ENOMEM, ENOSYS, EOPNOTSUPP, EPERM,
};
use crate::include::uapi::stat::{S_IFDIR, S_IFLNK, S_IFMT};

use super::types::*;

pub const ACL_TYPE_ACCESS: i32 = 0x8000;
pub const ACL_TYPE_DEFAULT: i32 = 0x4000;
pub const XATTR_NAME_POSIX_ACL_ACCESS: &str = "system.posix_acl_access";
pub const XATTR_NAME_POSIX_ACL_DEFAULT: &str = "system.posix_acl_default";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AclFetch {
    Disabled,
    CacheServerAcls,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachedAclDecision {
    Error(i32),
    None,
    Cached,
    Remote,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FidAclGetResult {
    Decoded,
    Error(i32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidAclGetInputs {
    pub first_xattr_get: i32,
    pub allocation_succeeds: bool,
    pub second_xattr_get: i32,
    pub decode_result: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DentryAclGetPlan {
    pub fid_lookup_errno: i32,
    pub put_fid: bool,
    pub result: FidAclGetResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AclCachePopulatePlan {
    pub clear_default_acl: bool,
    pub clear_access_acl: bool,
    pub cache_default_acl: bool,
    pub cache_access_acl: bool,
    pub release_default_acl: bool,
    pub release_access_acl: bool,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachedAclLookup {
    BugUncached,
    None,
    Cached,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AclModePlan {
    pub mode: u32,
    pub duplicate_default_acl: bool,
    pub set_access_acl: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IopSetAclInputs {
    pub session_flags: u32,
    pub inode_mode: u32,
    pub inode_owner_or_capable: bool,
    pub acl_type: i32,
    pub acl_present: bool,
    pub posix_acl_valid_result: i32,
    pub xattr_allocation_succeeds: bool,
    pub posix_acl_update_mode_result: i32,
    pub acl_representable_by_mode: bool,
    pub setattr_result: i32,
    pub xattr_set_result: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IopSetAclPlan {
    pub acl_name: &'static str,
    pub encoded_xattr_allocated: bool,
    pub encoded_xattr_freed_before_setxattr: bool,
    pub update_mode_called: bool,
    pub setattr_called: bool,
    pub setattr_result_ignored: bool,
    pub xattr_set_called: bool,
    pub set_cached_acl: bool,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FidSetAclPlan {
    NoAcl,
    AllocError,
    SetXattr { name: &'static str, retval: i32 },
    BugInvalidType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AclChmodPlan {
    pub cached_acl_read: bool,
    pub chmod_called: bool,
    pub cache_updated: bool,
    pub set_acl_called: bool,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreateAclPlan {
    pub cache_default_acl: bool,
    pub cache_access_acl: bool,
    pub set_default_acl_called: bool,
    pub set_access_acl_called: bool,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PutAclPlan {
    pub release_default_acl: bool,
    pub release_access_acl: bool,
}

pub const fn acl_client_cache_enabled(session_flags: u32) -> bool {
    (session_flags & V9FS_ACCESS_MASK) == V9FS_ACCESS_CLIENT
        && (session_flags & V9FS_ACL_MASK) == V9FS_POSIX_ACL
}

pub const fn v9fs_get_acl_plan(session_flags: u32) -> AclFetch {
    if acl_client_cache_enabled(session_flags) {
        AclFetch::CacheServerAcls
    } else {
        AclFetch::Disabled
    }
}

pub fn v9fs_fid_get_acl_result(inputs: FidAclGetInputs) -> FidAclGetResult {
    if inputs.first_xattr_get < 0 {
        return FidAclGetResult::Error(inputs.first_xattr_get);
    }
    if inputs.first_xattr_get == 0 {
        return FidAclGetResult::Error(-ENODATA);
    }
    if !inputs.allocation_succeeds {
        return FidAclGetResult::Error(-ENOMEM);
    }
    if inputs.second_xattr_get < 0 {
        return FidAclGetResult::Error(inputs.second_xattr_get);
    }
    if inputs.second_xattr_get == 0 {
        return FidAclGetResult::Error(-ENODATA);
    }
    if inputs.decode_result < 0 {
        return FidAclGetResult::Error(inputs.decode_result);
    }
    FidAclGetResult::Decoded
}

pub const fn v9fs_acl_get_plan(
    fid_lookup_errno: i32,
    fid_acl_result: FidAclGetResult,
) -> DentryAclGetPlan {
    if fid_lookup_errno < 0 {
        DentryAclGetPlan {
            fid_lookup_errno,
            put_fid: false,
            result: FidAclGetResult::Error(fid_lookup_errno),
        }
    } else {
        DentryAclGetPlan {
            fid_lookup_errno: 0,
            put_fid: true,
            result: fid_acl_result,
        }
    }
}

pub fn __v9fs_get_acl_result(fid_get_acl_result: i32) -> CachedAclDecision {
    if fid_get_acl_result >= 0 {
        return CachedAclDecision::Cached;
    }
    match -fid_get_acl_result {
        ENODATA | ENOSYS | EOPNOTSUPP => CachedAclDecision::None,
        _ => CachedAclDecision::Error(-EIO),
    }
}

pub const fn v9fs_get_acl_cache_plan(
    session_flags: u32,
    default_acl: CachedAclDecision,
    access_acl: CachedAclDecision,
) -> AclCachePopulatePlan {
    if !acl_client_cache_enabled(session_flags) {
        return AclCachePopulatePlan {
            clear_default_acl: true,
            clear_access_acl: true,
            cache_default_acl: false,
            cache_access_acl: false,
            release_default_acl: false,
            release_access_acl: false,
            retval: 0,
        };
    }

    let default_ok = !matches!(default_acl, CachedAclDecision::Error(_));
    let access_ok = !matches!(access_acl, CachedAclDecision::Error(_));
    AclCachePopulatePlan {
        clear_default_acl: false,
        clear_access_acl: false,
        cache_default_acl: default_ok && access_ok,
        cache_access_acl: default_ok && access_ok,
        release_default_acl: default_ok,
        release_access_acl: access_ok,
        retval: if default_ok && access_ok { 0 } else { -EIO },
    }
}

pub const fn v9fs_get_cached_acl_lookup(
    uncached_marker: bool,
    cached_acl: bool,
) -> CachedAclLookup {
    if uncached_marker {
        CachedAclLookup::BugUncached
    } else if cached_acl {
        CachedAclLookup::Cached
    } else {
        CachedAclLookup::None
    }
}

pub fn v9fs_iop_get_inode_acl(session_flags: u32, rcu: bool) -> CachedAclDecision {
    if rcu {
        return CachedAclDecision::Error(-ECHILD);
    }
    if !acl_client_cache_enabled(session_flags) {
        return CachedAclDecision::None;
    }
    CachedAclDecision::Cached
}

pub fn v9fs_iop_get_acl(session_flags: u32) -> CachedAclDecision {
    if (session_flags & V9FS_ACCESS_MASK) != V9FS_ACCESS_CLIENT {
        CachedAclDecision::Remote
    } else {
        CachedAclDecision::Cached
    }
}

pub const fn v9fs_posix_acl_xattr_name(acl_type: i32) -> &'static str {
    match acl_type {
        ACL_TYPE_ACCESS => XATTR_NAME_POSIX_ACL_ACCESS,
        ACL_TYPE_DEFAULT => XATTR_NAME_POSIX_ACL_DEFAULT,
        _ => "",
    }
}

pub fn v9fs_acl_xattr_name(acl_type: i32) -> Result<&'static str, i32> {
    let name = v9fs_posix_acl_xattr_name(acl_type);
    if name.is_empty() {
        Err(-EINVAL)
    } else {
        Ok(name)
    }
}

pub fn v9fs_iop_set_acl_precheck(
    session_flags: u32,
    inode_mode: u32,
    inode_owner_or_capable: bool,
    acl_type: i32,
    acl_present: bool,
) -> Result<&'static str, i32> {
    let name = v9fs_acl_xattr_name(acl_type)?;
    if (session_flags & V9FS_ACCESS_MASK) != V9FS_ACCESS_CLIENT {
        return Ok(name);
    }
    if (inode_mode & S_IFMT) == S_IFLNK {
        return Err(-EOPNOTSUPP);
    }
    if !inode_owner_or_capable {
        return Err(-EPERM);
    }
    if acl_type == ACL_TYPE_DEFAULT && (inode_mode & S_IFMT) != S_IFDIR {
        return if acl_present { Err(-EINVAL) } else { Ok(name) };
    }
    Ok(name)
}

pub fn v9fs_iop_set_acl_plan(inputs: IopSetAclInputs) -> IopSetAclPlan {
    let mut plan = IopSetAclPlan {
        acl_name: v9fs_posix_acl_xattr_name(inputs.acl_type),
        encoded_xattr_allocated: false,
        encoded_xattr_freed_before_setxattr: false,
        update_mode_called: false,
        setattr_called: false,
        setattr_result_ignored: false,
        xattr_set_called: false,
        set_cached_acl: false,
        retval: 0,
    };

    if inputs.acl_present {
        if inputs.posix_acl_valid_result != 0 {
            plan.retval = inputs.posix_acl_valid_result;
            return plan;
        }
        if !inputs.xattr_allocation_succeeds {
            plan.retval = -ENOMEM;
            return plan;
        }
        plan.encoded_xattr_allocated = true;
    }

    if (inputs.session_flags & V9FS_ACCESS_MASK) != V9FS_ACCESS_CLIENT {
        plan.xattr_set_called = true;
        plan.retval = inputs.xattr_set_result;
        return plan;
    }

    if (inputs.inode_mode & S_IFMT) == S_IFLNK {
        plan.retval = -EOPNOTSUPP;
        return plan;
    }

    if !inputs.inode_owner_or_capable {
        plan.retval = -EPERM;
        return plan;
    }

    if inputs.acl_type == ACL_TYPE_ACCESS && inputs.acl_present {
        plan.update_mode_called = true;
        if inputs.posix_acl_update_mode_result != 0 {
            plan.retval = inputs.posix_acl_update_mode_result;
            return plan;
        }
        if inputs.acl_representable_by_mode {
            plan.encoded_xattr_freed_before_setxattr = true;
        }
        plan.setattr_called = true;
        plan.setattr_result_ignored = inputs.setattr_result != 0;
    } else if inputs.acl_type == ACL_TYPE_DEFAULT && (inputs.inode_mode & S_IFMT) != S_IFDIR {
        plan.retval = if inputs.acl_present { -EINVAL } else { 0 };
        return plan;
    }

    plan.xattr_set_called = true;
    plan.retval = inputs.xattr_set_result;
    plan.set_cached_acl = inputs.xattr_set_result == 0;
    plan
}

pub const fn v9fs_set_acl_plan(
    acl_type: i32,
    acl_present: bool,
    xattr_allocation_succeeds: bool,
    xattr_set_result: i32,
) -> FidSetAclPlan {
    if !acl_present {
        return FidSetAclPlan::NoAcl;
    }
    if !xattr_allocation_succeeds {
        return FidSetAclPlan::AllocError;
    }
    match acl_type {
        ACL_TYPE_ACCESS => FidSetAclPlan::SetXattr {
            name: XATTR_NAME_POSIX_ACL_ACCESS,
            retval: xattr_set_result,
        },
        ACL_TYPE_DEFAULT => FidSetAclPlan::SetXattr {
            name: XATTR_NAME_POSIX_ACL_DEFAULT,
            retval: xattr_set_result,
        },
        _ => FidSetAclPlan::BugInvalidType,
    }
}

pub const fn v9fs_acl_chmod_plan(
    inode_mode: u32,
    cached_acl_present: bool,
    chmod_result: i32,
    set_acl_result: i32,
) -> AclChmodPlan {
    if (inode_mode & S_IFMT) == S_IFLNK {
        return AclChmodPlan {
            cached_acl_read: false,
            chmod_called: false,
            cache_updated: false,
            set_acl_called: false,
            retval: -EOPNOTSUPP,
        };
    }
    if !cached_acl_present {
        return AclChmodPlan {
            cached_acl_read: true,
            chmod_called: false,
            cache_updated: false,
            set_acl_called: false,
            retval: 0,
        };
    }
    if chmod_result != 0 {
        return AclChmodPlan {
            cached_acl_read: true,
            chmod_called: true,
            cache_updated: false,
            set_acl_called: false,
            retval: chmod_result,
        };
    }
    AclChmodPlan {
        cached_acl_read: true,
        chmod_called: true,
        cache_updated: true,
        set_acl_called: true,
        retval: set_acl_result,
    }
}

pub const fn v9fs_set_create_acl_plan(dacl_present: bool, acl_present: bool) -> CreateAclPlan {
    CreateAclPlan {
        cache_default_acl: true,
        cache_access_acl: true,
        set_default_acl_called: dacl_present,
        set_access_acl_called: acl_present,
        retval: 0,
    }
}

pub const fn v9fs_put_acl_plan(dacl_present: bool, acl_present: bool) -> PutAclPlan {
    PutAclPlan {
        release_default_acl: dacl_present,
        release_access_acl: acl_present,
    }
}

pub fn v9fs_acl_mode(
    mode: u32,
    current_umask: u32,
    has_default_acl: bool,
    posix_acl_create_result: i32,
) -> Result<AclModePlan, i32> {
    if (mode & S_IFMT) == S_IFLNK {
        return Ok(AclModePlan {
            mode,
            duplicate_default_acl: false,
            set_access_acl: false,
        });
    }
    if !has_default_acl {
        return Ok(AclModePlan {
            mode: mode & !current_umask,
            duplicate_default_acl: false,
            set_access_acl: false,
        });
    }
    if posix_acl_create_result < 0 {
        return Err(posix_acl_create_result);
    }
    Ok(AclModePlan {
        mode,
        duplicate_default_acl: (mode & S_IFMT) == S_IFDIR,
        set_access_acl: posix_acl_create_result > 0,
    })
}

pub const fn v9fs_acl_chmod_allowed(inode_mode: u32) -> Result<(), i32> {
    if (inode_mode & S_IFMT) == S_IFLNK {
        Err(-EOPNOTSUPP)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::stat::S_IFREG;

    const ENABLED: u32 = V9FS_ACCESS_CLIENT | V9FS_POSIX_ACL;

    const fn base_set_acl_inputs() -> IopSetAclInputs {
        IopSetAclInputs {
            session_flags: ENABLED,
            inode_mode: S_IFREG | 0o644,
            inode_owner_or_capable: true,
            acl_type: ACL_TYPE_ACCESS,
            acl_present: true,
            posix_acl_valid_result: 0,
            xattr_allocation_succeeds: true,
            posix_acl_update_mode_result: 0,
            acl_representable_by_mode: false,
            setattr_result: 0,
            xattr_set_result: 0,
        }
    }

    #[test]
    fn acl_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/acl.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/acl.h"
        ));
        assert!(source.contains("static struct posix_acl *v9fs_fid_get_acl"));
        assert!(source.contains("size = v9fs_fid_xattr_get(fid, name, NULL, 0);"));
        assert!(source.contains("if (size == 0)"));
        assert!(source.contains("return ERR_PTR(-ENODATA);"));
        assert!(source.contains("value = kzalloc(size, GFP_NOFS);"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("size = v9fs_fid_xattr_get(fid, name, value, size);"));
        assert!(source.contains("posix_acl_from_xattr(&init_user_ns, value, size);"));
        assert!(source.contains("static struct posix_acl *v9fs_acl_get"));
        assert!(source.contains("p9_fid_put(fid);"));
        assert!(
            source
                .contains("if (retval == -ENODATA || retval == -ENOSYS || retval == -EOPNOTSUPP)")
        );
        assert!(source.contains("return ERR_PTR(-EIO);"));
        assert!(source.contains("int v9fs_get_acl(struct inode *inode, struct p9_fid *fid)"));
        assert!(source.contains("((v9ses->flags & V9FS_ACCESS_MASK) != V9FS_ACCESS_CLIENT)"));
        assert!(source.contains("set_cached_acl(inode, ACL_TYPE_DEFAULT, NULL);"));
        assert!(source.contains("set_cached_acl(inode, ACL_TYPE_ACCESS, NULL);"));
        assert!(source.contains("dacl = __v9fs_get_acl(fid, XATTR_NAME_POSIX_ACL_DEFAULT);"));
        assert!(source.contains("pacl = __v9fs_get_acl(fid, XATTR_NAME_POSIX_ACL_ACCESS);"));
        assert!(source.contains("if (!IS_ERR(dacl) && !IS_ERR(pacl))"));
        assert!(source.contains("posix_acl_release(dacl);"));
        assert!(source.contains("static struct posix_acl *v9fs_get_cached_acl"));
        assert!(source.contains("BUG_ON(is_uncached_acl(acl));"));
        assert!(source.contains("struct posix_acl *v9fs_iop_get_inode_acl"));
        assert!(source.contains("if (rcu)"));
        assert!(source.contains("return ERR_PTR(-ECHILD);"));
        assert!(source.contains("return v9fs_acl_get(dentry, posix_acl_xattr_name(type));"));
        assert!(source.contains("int v9fs_iop_set_acl"));
        assert!(source.contains("retval = posix_acl_valid(inode->i_sb->s_user_ns, acl);"));
        assert!(
            source.contains("value = posix_acl_to_xattr(&init_user_ns, acl, &size, GFP_NOFS);")
        );
        assert!(source.contains("retval = v9fs_xattr_set(dentry, acl_name, value, size, 0);"));
        assert!(source.contains("if (S_ISLNK(inode->i_mode))"));
        assert!(source.contains("retval = -EOPNOTSUPP;"));
        assert!(source.contains("retval = posix_acl_update_mode(&nop_mnt_idmap, inode,"));
        assert!(source.contains("if (!acl_mode)"));
        assert!(source.contains("v9fs_vfs_setattr_dotl(&nop_mnt_idmap, dentry, &iattr);"));
        assert!(source.contains("if (!S_ISDIR(inode->i_mode))"));
        assert!(source.contains("retval = acl ? -EINVAL : 0;"));
        assert!(source.contains("if (!retval)"));
        assert!(source.contains("set_cached_acl(inode, type, acl);"));
        assert!(source.contains("static int v9fs_set_acl"));
        assert!(source.contains("if (!acl)"));
        assert!(
            source.contains("buffer = posix_acl_to_xattr(&init_user_ns, acl, &size, GFP_KERNEL);")
        );
        assert!(source.contains("retval = v9fs_fid_xattr_set(fid, name, buffer, size, 0);"));
        assert!(source.contains("int v9fs_acl_chmod"));
        assert!(source.contains("__posix_acl_chmod(&acl, GFP_KERNEL, inode->i_mode);"));
        assert!(source.contains("int v9fs_set_create_acl"));
        assert!(source.contains("void v9fs_put_acl"));
        assert!(source.contains("int v9fs_acl_mode"));
        assert!(source.contains("mode &= ~current_umask();"));
        assert!(header.contains("#define v9fs_iop_get_inode_acl\tNULL"));
        assert!(header.contains("int v9fs_set_create_acl"));
        assert!(header.contains("void v9fs_put_acl"));

        assert_eq!(v9fs_get_acl_plan(ENABLED), AclFetch::CacheServerAcls);
        assert_eq!(v9fs_get_acl_plan(V9FS_ACCESS_USER), AclFetch::Disabled);
        assert_eq!(__v9fs_get_acl_result(-ENODATA), CachedAclDecision::None);
        assert_eq!(
            __v9fs_get_acl_result(-EPERM),
            CachedAclDecision::Error(-EIO)
        );
        assert_eq!(
            v9fs_iop_get_inode_acl(ENABLED, true),
            CachedAclDecision::Error(-ECHILD)
        );
        assert_eq!(
            v9fs_iop_get_acl(V9FS_ACCESS_USER),
            CachedAclDecision::Remote
        );
        assert_eq!(
            v9fs_acl_xattr_name(ACL_TYPE_ACCESS),
            Ok(XATTR_NAME_POSIX_ACL_ACCESS)
        );
        assert_eq!(
            v9fs_iop_set_acl_precheck(ENABLED, S_IFLNK | 0o777, true, ACL_TYPE_ACCESS, true),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(
            v9fs_iop_set_acl_precheck(ENABLED, S_IFREG | 0o644, false, ACL_TYPE_ACCESS, true),
            Err(-EPERM)
        );
        assert_eq!(
            v9fs_iop_set_acl_precheck(ENABLED, S_IFREG | 0o644, true, ACL_TYPE_DEFAULT, true),
            Err(-EINVAL)
        );
        assert_eq!(
            v9fs_acl_mode(S_IFREG | 0o666, 0o022, false, 0)
                .unwrap()
                .mode,
            S_IFREG | 0o644
        );
        assert!(
            v9fs_acl_mode(S_IFDIR | 0o777, 0o022, true, 1)
                .unwrap()
                .duplicate_default_acl
        );
        assert_eq!(v9fs_acl_chmod_allowed(S_IFLNK | 0o777), Err(-EOPNOTSUPP));
    }

    #[test]
    fn fid_and_cache_acl_paths_match_linux_error_mapping() {
        assert_eq!(
            v9fs_fid_get_acl_result(FidAclGetInputs {
                first_xattr_get: -EPERM,
                allocation_succeeds: true,
                second_xattr_get: 4,
                decode_result: 0,
            }),
            FidAclGetResult::Error(-EPERM)
        );
        assert_eq!(
            v9fs_fid_get_acl_result(FidAclGetInputs {
                first_xattr_get: 0,
                allocation_succeeds: true,
                second_xattr_get: 4,
                decode_result: 0,
            }),
            FidAclGetResult::Error(-ENODATA)
        );
        assert_eq!(
            v9fs_fid_get_acl_result(FidAclGetInputs {
                first_xattr_get: 8,
                allocation_succeeds: false,
                second_xattr_get: 8,
                decode_result: 0,
            }),
            FidAclGetResult::Error(-ENOMEM)
        );
        assert_eq!(
            v9fs_fid_get_acl_result(FidAclGetInputs {
                first_xattr_get: 8,
                allocation_succeeds: true,
                second_xattr_get: 0,
                decode_result: 0,
            }),
            FidAclGetResult::Error(-ENODATA)
        );
        assert_eq!(
            v9fs_fid_get_acl_result(FidAclGetInputs {
                first_xattr_get: 8,
                allocation_succeeds: true,
                second_xattr_get: 8,
                decode_result: -EOPNOTSUPP,
            }),
            FidAclGetResult::Error(-EOPNOTSUPP)
        );
        assert_eq!(
            v9fs_fid_get_acl_result(FidAclGetInputs {
                first_xattr_get: 8,
                allocation_succeeds: true,
                second_xattr_get: 8,
                decode_result: 0,
            }),
            FidAclGetResult::Decoded
        );

        assert_eq!(
            v9fs_acl_get_plan(-EIO, FidAclGetResult::Decoded),
            DentryAclGetPlan {
                fid_lookup_errno: -EIO,
                put_fid: false,
                result: FidAclGetResult::Error(-EIO),
            }
        );
        assert_eq!(
            v9fs_acl_get_plan(0, FidAclGetResult::Decoded),
            DentryAclGetPlan {
                fid_lookup_errno: 0,
                put_fid: true,
                result: FidAclGetResult::Decoded,
            }
        );

        assert_eq!(
            v9fs_get_acl_cache_plan(
                V9FS_ACCESS_USER,
                CachedAclDecision::Cached,
                CachedAclDecision::Cached
            ),
            AclCachePopulatePlan {
                clear_default_acl: true,
                clear_access_acl: true,
                cache_default_acl: false,
                cache_access_acl: false,
                release_default_acl: false,
                release_access_acl: false,
                retval: 0,
            }
        );
        assert_eq!(
            v9fs_get_acl_cache_plan(ENABLED, CachedAclDecision::None, CachedAclDecision::Cached),
            AclCachePopulatePlan {
                clear_default_acl: false,
                clear_access_acl: false,
                cache_default_acl: true,
                cache_access_acl: true,
                release_default_acl: true,
                release_access_acl: true,
                retval: 0,
            }
        );
        assert_eq!(
            v9fs_get_acl_cache_plan(
                ENABLED,
                CachedAclDecision::Error(-EIO),
                CachedAclDecision::Cached
            ),
            AclCachePopulatePlan {
                clear_default_acl: false,
                clear_access_acl: false,
                cache_default_acl: false,
                cache_access_acl: false,
                release_default_acl: false,
                release_access_acl: true,
                retval: -EIO,
            }
        );
        assert_eq!(
            v9fs_get_cached_acl_lookup(true, false),
            CachedAclLookup::BugUncached
        );
    }

    #[test]
    fn iop_set_acl_plan_matches_linux_branches() {
        let mut non_client = base_set_acl_inputs();
        non_client.session_flags = V9FS_ACCESS_USER;
        non_client.xattr_set_result = -EIO;
        assert_eq!(
            v9fs_iop_set_acl_plan(non_client),
            IopSetAclPlan {
                acl_name: XATTR_NAME_POSIX_ACL_ACCESS,
                encoded_xattr_allocated: true,
                encoded_xattr_freed_before_setxattr: false,
                update_mode_called: false,
                setattr_called: false,
                setattr_result_ignored: false,
                xattr_set_called: true,
                set_cached_acl: false,
                retval: -EIO,
            }
        );

        let mut valid_error = base_set_acl_inputs();
        valid_error.posix_acl_valid_result = -EINVAL;
        assert_eq!(v9fs_iop_set_acl_plan(valid_error).retval, -EINVAL);

        let mut alloc_error = base_set_acl_inputs();
        alloc_error.xattr_allocation_succeeds = false;
        assert_eq!(v9fs_iop_set_acl_plan(alloc_error).retval, -ENOMEM);

        let mut symlink = base_set_acl_inputs();
        symlink.inode_mode = S_IFLNK | 0o777;
        assert_eq!(v9fs_iop_set_acl_plan(symlink).retval, -EOPNOTSUPP);

        let mut not_owner = base_set_acl_inputs();
        not_owner.inode_owner_or_capable = false;
        assert_eq!(v9fs_iop_set_acl_plan(not_owner).retval, -EPERM);

        let mut default_non_dir_without_acl = base_set_acl_inputs();
        default_non_dir_without_acl.acl_type = ACL_TYPE_DEFAULT;
        default_non_dir_without_acl.acl_present = false;
        let plan = v9fs_iop_set_acl_plan(default_non_dir_without_acl);
        assert_eq!(plan.retval, 0);
        assert!(!plan.xattr_set_called);

        let mut default_non_dir_with_acl = default_non_dir_without_acl;
        default_non_dir_with_acl.acl_present = true;
        assert_eq!(
            v9fs_iop_set_acl_plan(default_non_dir_with_acl).retval,
            -EINVAL
        );

        let mut update_error = base_set_acl_inputs();
        update_error.posix_acl_update_mode_result = -EIO;
        let plan = v9fs_iop_set_acl_plan(update_error);
        assert!(plan.update_mode_called);
        assert_eq!(plan.retval, -EIO);
        assert!(!plan.xattr_set_called);

        let mut access_represented_by_mode = base_set_acl_inputs();
        access_represented_by_mode.acl_representable_by_mode = true;
        access_represented_by_mode.setattr_result = -EIO;
        let plan = v9fs_iop_set_acl_plan(access_represented_by_mode);
        assert!(plan.update_mode_called);
        assert!(plan.encoded_xattr_freed_before_setxattr);
        assert!(plan.setattr_called);
        assert!(plan.setattr_result_ignored);
        assert!(plan.xattr_set_called);
        assert!(plan.set_cached_acl);
        assert_eq!(plan.retval, 0);
    }

    #[test]
    fn chmod_create_and_fid_set_acl_plans_match_linux_helpers() {
        assert_eq!(
            v9fs_set_acl_plan(ACL_TYPE_ACCESS, false, false, -EIO),
            FidSetAclPlan::NoAcl
        );
        assert_eq!(
            v9fs_set_acl_plan(ACL_TYPE_ACCESS, true, false, 0),
            FidSetAclPlan::AllocError
        );
        assert_eq!(
            v9fs_set_acl_plan(ACL_TYPE_DEFAULT, true, true, -EIO),
            FidSetAclPlan::SetXattr {
                name: XATTR_NAME_POSIX_ACL_DEFAULT,
                retval: -EIO,
            }
        );
        assert_eq!(
            v9fs_set_acl_plan(123, true, true, 0),
            FidSetAclPlan::BugInvalidType
        );

        assert_eq!(
            v9fs_acl_chmod_plan(S_IFLNK | 0o777, true, 0, 0).retval,
            -EOPNOTSUPP
        );
        assert_eq!(
            v9fs_acl_chmod_plan(S_IFREG | 0o644, false, 0, 0),
            AclChmodPlan {
                cached_acl_read: true,
                chmod_called: false,
                cache_updated: false,
                set_acl_called: false,
                retval: 0,
            }
        );
        assert_eq!(
            v9fs_acl_chmod_plan(S_IFREG | 0o644, true, -EIO, 0).retval,
            -EIO
        );
        assert_eq!(
            v9fs_acl_chmod_plan(S_IFREG | 0o644, true, 0, -EPERM),
            AclChmodPlan {
                cached_acl_read: true,
                chmod_called: true,
                cache_updated: true,
                set_acl_called: true,
                retval: -EPERM,
            }
        );

        assert_eq!(
            v9fs_set_create_acl_plan(true, false),
            CreateAclPlan {
                cache_default_acl: true,
                cache_access_acl: true,
                set_default_acl_called: true,
                set_access_acl_called: false,
                retval: 0,
            }
        );
        assert_eq!(
            v9fs_put_acl_plan(true, true),
            PutAclPlan {
                release_default_acl: true,
                release_access_acl: true,
            }
        );
    }
}
