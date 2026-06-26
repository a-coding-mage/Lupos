//! linux-parity: complete
//! linux-source: vendor/linux/fs/cachefiles/security.c
//! test-origin: linux:vendor/linux/fs/cachefiles/security.c
//! CacheFiles security credential setup decisions.

use crate::include::uapi::errno::{ENOMEM, EOPNOTSUPP};

pub const CACHEFILES_PREPARE_KERNEL_CRED: &str = "prepare_kernel_cred(current)";
pub const CACHEFILES_SET_SECURITY_OVERRIDE: &str = "set_security_override";
pub const CACHEFILES_PREPARE_CREDS: &str = "prepare_creds";
pub const CACHEFILES_SET_CREATE_FILES_AS: &str = "set_create_files_as";
pub const CACHEFILES_SECURITY_MKDIR: &str = "security_inode_mkdir";
pub const CACHEFILES_SECURITY_CREATE: &str = "security_inode_create";
pub const CACHEFILES_BACKING_INODE_EXPR: &str = "d_backing_inode(root)";
pub const CACHEFILES_SECURE_BEGIN: &str = "cachefiles_begin_secure";
pub const CACHEFILES_SECURE_END: &str = "cachefiles_end_secure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CachefilesGetSecurityIdFlow {
    pub allocate_cred: &'static str,
    pub allocation_error: i32,
    pub secid_guard: &'static str,
    pub override_call: &'static str,
    pub override_failure_cleanup: &'static str,
    pub success_assignment: &'static str,
    pub success_ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CachefilesCheckCacheDirFlow {
    pub mkdir_hook: &'static str,
    pub create_hook: &'static str,
    pub inode_expr: &'static str,
    pub root_arg: &'static str,
    pub mode: u32,
    pub mkdir_error_is_terminal: bool,
    pub create_error_is_terminal: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CachefilesDetermineSecurityFlow {
    pub duplicate_creds: &'static str,
    pub allocation_error: i32,
    pub end_secure_before_mutation: &'static str,
    pub set_create_files_as: &'static str,
    pub create_files_inode_expr: &'static str,
    pub set_create_failure_cleanup: &'static str,
    pub restore_secure_on_failure: &'static str,
    pub replace_cache_cred: &'static str,
    pub restore_secure_before_check: &'static str,
    pub check_cache_dir: &'static str,
    pub eopnotsupp_normalized_to_zero: bool,
}

pub const CACHEFILES_GET_SECURITY_ID_FLOW: CachefilesGetSecurityIdFlow =
    CachefilesGetSecurityIdFlow {
        allocate_cred: CACHEFILES_PREPARE_KERNEL_CRED,
        allocation_error: -ENOMEM,
        secid_guard: "cache->have_secid",
        override_call: CACHEFILES_SET_SECURITY_OVERRIDE,
        override_failure_cleanup: "put_cred(new)",
        success_assignment: "cache->cache_cred = new",
        success_ret: 0,
    };

pub const CACHEFILES_CHECK_CACHE_DIR_FLOW: CachefilesCheckCacheDirFlow =
    CachefilesCheckCacheDirFlow {
        mkdir_hook: CACHEFILES_SECURITY_MKDIR,
        create_hook: CACHEFILES_SECURITY_CREATE,
        inode_expr: CACHEFILES_BACKING_INODE_EXPR,
        root_arg: "root",
        mode: 0,
        mkdir_error_is_terminal: true,
        create_error_is_terminal: true,
    };

pub const CACHEFILES_DETERMINE_SECURITY_FLOW: CachefilesDetermineSecurityFlow =
    CachefilesDetermineSecurityFlow {
        duplicate_creds: CACHEFILES_PREPARE_CREDS,
        allocation_error: -ENOMEM,
        end_secure_before_mutation: CACHEFILES_SECURE_END,
        set_create_files_as: CACHEFILES_SET_CREATE_FILES_AS,
        create_files_inode_expr: CACHEFILES_BACKING_INODE_EXPR,
        set_create_failure_cleanup: "abort_creds(new)",
        restore_secure_on_failure: CACHEFILES_SECURE_BEGIN,
        replace_cache_cred: "put_cred(cache->cache_cred); cache->cache_cred = new",
        restore_secure_before_check: CACHEFILES_SECURE_BEGIN,
        check_cache_dir: "cachefiles_check_cache_dir",
        eopnotsupp_normalized_to_zero: true,
    };

pub const fn cachefiles_get_security_id_result(
    prepare_kernel_cred_ok: bool,
    have_secid: bool,
    set_security_override_ret: i32,
) -> Result<(), i32> {
    if !prepare_kernel_cred_ok {
        return Err(-ENOMEM);
    }
    if have_secid && set_security_override_ret < 0 {
        return Err(set_security_override_ret);
    }
    Ok(())
}

pub const fn cachefiles_check_cache_dir_result(mkdir_ret: i32, create_ret: i32) -> Result<(), i32> {
    if mkdir_ret < 0 {
        return Err(mkdir_ret);
    }
    if create_ret < 0 {
        return Err(create_ret);
    }
    Ok(())
}

pub const fn cachefiles_determine_cache_security_result(
    prepare_creds_ok: bool,
    set_create_files_as_ret: i32,
    check_cache_dir_ret: i32,
) -> Result<(), i32> {
    if !prepare_creds_ok {
        return Err(-ENOMEM);
    }
    if set_create_files_as_ret < 0 {
        return Err(set_create_files_as_ret);
    }
    if check_cache_dir_ret == -EOPNOTSUPP {
        return Ok(());
    }
    if check_cache_dir_ret < 0 {
        return Err(check_cache_dir_ret);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cachefiles_security_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/cachefiles/security.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/cred.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("int cachefiles_get_security_ID(struct cachefiles_cache *cache)"));
        assert!(source.contains("struct cred *new;"));
        assert!(source.contains("int ret;"));
        assert!(source.contains("new = prepare_kernel_cred(current);"));
        assert!(source.contains("if (!new)"));
        assert!(source.contains("ret = -ENOMEM;"));
        assert!(source.contains("goto error;"));
        assert!(source.contains("if (cache->have_secid)"));
        assert!(source.contains("ret = set_security_override(new, cache->secid);"));
        assert!(source.contains("if (ret < 0)"));
        assert!(source.contains("put_cred(new);"));
        assert!(source.contains("goto error;"));
        assert!(source.contains("cache->cache_cred = new;"));
        assert!(source.contains("ret = 0;"));
        assert!(source.contains("_leave(\" = %d\", ret);"));
        assert!(source.contains("static int cachefiles_check_cache_dir"));
        assert!(source.contains("security_inode_mkdir(d_backing_inode(root), root, 0);"));
        assert!(source.contains("if (ret < 0)"));
        assert!(source.contains("return ret;"));
        assert!(source.contains("security_inode_create(d_backing_inode(root), root, 0);"));
        assert!(source.contains("return ret;"));
        assert!(source.contains("int cachefiles_determine_cache_security"));
        assert!(source.contains("const struct cred **_saved_cred"));
        assert!(source.contains("new = prepare_creds();"));
        assert!(source.contains("if (!new)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("cachefiles_end_secure(cache, *_saved_cred);"));
        assert!(source.contains("ret = set_create_files_as(new, d_backing_inode(root));"));
        assert!(source.contains("if (ret < 0)"));
        assert!(source.contains("abort_creds(new);"));
        assert!(source.contains("cachefiles_begin_secure(cache, _saved_cred);"));
        assert!(source.contains("return ret;"));
        assert!(source.contains("put_cred(cache->cache_cred);"));
        assert!(source.contains("cache->cache_cred = new;"));
        assert!(source.contains("cachefiles_begin_secure(cache, _saved_cred);"));
        assert!(source.contains("ret = cachefiles_check_cache_dir(cache, root);"));
        assert!(source.contains("if (ret == -EOPNOTSUPP)"));
        assert!(source.contains("ret = 0;"));

        assert_eq!(
            cachefiles_get_security_id_result(false, false, 0),
            Err(-ENOMEM)
        );
        assert_eq!(cachefiles_get_security_id_result(true, true, -13), Err(-13));
        assert_eq!(cachefiles_get_security_id_result(true, false, -13), Ok(()));
        assert_eq!(cachefiles_check_cache_dir_result(-1, 0), Err(-1));
        assert_eq!(cachefiles_check_cache_dir_result(0, -2), Err(-2));
        assert_eq!(cachefiles_check_cache_dir_result(0, 0), Ok(()));
        assert_eq!(
            cachefiles_determine_cache_security_result(false, 0, 0),
            Err(-ENOMEM)
        );
        assert_eq!(
            cachefiles_determine_cache_security_result(true, -5, 0),
            Err(-5)
        );
        assert_eq!(
            cachefiles_determine_cache_security_result(true, 0, -EOPNOTSUPP),
            Ok(())
        );
        assert_eq!(
            cachefiles_determine_cache_security_result(true, 0, -13),
            Err(-13)
        );
        assert_eq!(
            cachefiles_determine_cache_security_result(true, 0, 0),
            Ok(())
        );
        assert_eq!(
            CACHEFILES_GET_SECURITY_ID_FLOW,
            CachefilesGetSecurityIdFlow {
                allocate_cred: "prepare_kernel_cred(current)",
                allocation_error: -ENOMEM,
                secid_guard: "cache->have_secid",
                override_call: "set_security_override",
                override_failure_cleanup: "put_cred(new)",
                success_assignment: "cache->cache_cred = new",
                success_ret: 0,
            }
        );
        assert_eq!(
            CACHEFILES_CHECK_CACHE_DIR_FLOW,
            CachefilesCheckCacheDirFlow {
                mkdir_hook: "security_inode_mkdir",
                create_hook: "security_inode_create",
                inode_expr: "d_backing_inode(root)",
                root_arg: "root",
                mode: 0,
                mkdir_error_is_terminal: true,
                create_error_is_terminal: true,
            }
        );
        assert_eq!(
            CACHEFILES_DETERMINE_SECURITY_FLOW,
            CachefilesDetermineSecurityFlow {
                duplicate_creds: "prepare_creds",
                allocation_error: -ENOMEM,
                end_secure_before_mutation: "cachefiles_end_secure",
                set_create_files_as: "set_create_files_as",
                create_files_inode_expr: "d_backing_inode(root)",
                set_create_failure_cleanup: "abort_creds(new)",
                restore_secure_on_failure: "cachefiles_begin_secure",
                replace_cache_cred: "put_cred(cache->cache_cred); cache->cache_cred = new",
                restore_secure_before_check: "cachefiles_begin_secure",
                check_cache_dir: "cachefiles_check_cache_dir",
                eopnotsupp_normalized_to_zero: true,
            }
        );
    }
}
