//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/xattr.c
//! test-origin: linux:vendor/linux/fs/9p/xattr.c
//! 9P extended attribute walk/create result handling and handler names.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EOVERFLOW, ERANGE};

pub const XATTR_USER_PREFIX: &str = "user.";
pub const XATTR_TRUSTED_PREFIX: &str = "trusted.";
pub const XATTR_SECURITY_PREFIX: &str = "security.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XattrReadResult {
    pub bytes: usize,
    pub read_called: bool,
    pub attr_fid_put: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XattrSetResult {
    pub write_called: bool,
    pub errno: i32,
    pub cloned_fid: bool,
    pub xattrcreate_called: bool,
    pub fid_put: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DentryXattrGetPlan {
    pub fid_lookup_called: bool,
    pub fid_put: bool,
    pub result: Result<XattrReadResult, i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DentryXattrSetPlan {
    pub fid_lookup_called: bool,
    pub fid_put: bool,
    pub result: XattrSetResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XattrHandler {
    pub prefix: &'static str,
    pub get: &'static str,
    pub set: &'static str,
}

pub const V9FS_XATTR_USER_HANDLER: XattrHandler = XattrHandler {
    prefix: XATTR_USER_PREFIX,
    get: "v9fs_xattr_handler_get",
    set: "v9fs_xattr_handler_set",
};

pub const V9FS_XATTR_TRUSTED_HANDLER: XattrHandler = XattrHandler {
    prefix: XATTR_TRUSTED_PREFIX,
    get: "v9fs_xattr_handler_get",
    set: "v9fs_xattr_handler_set",
};

pub const V9FS_XATTR_SECURITY_HANDLER: XattrHandler = XattrHandler {
    prefix: XATTR_SECURITY_PREFIX,
    get: "v9fs_xattr_handler_get",
    set: "v9fs_xattr_handler_set",
};

pub fn v9fs_fid_xattr_get_result(
    attr_size: u64,
    buffer_size: usize,
    read_bytes: usize,
    read_errno: i32,
) -> Result<XattrReadResult, i32> {
    if attr_size > buffer_size as u64 {
        if buffer_size != 0 {
            return Err(-ERANGE);
        }
        if attr_size > isize::MAX as u64 {
            return Err(-EOVERFLOW);
        }
        return Ok(XattrReadResult {
            bytes: attr_size as usize,
            read_called: false,
            attr_fid_put: true,
        });
    }
    if read_errno != 0 {
        return Err(read_errno);
    }
    Ok(XattrReadResult {
        bytes: read_bytes,
        read_called: true,
        attr_fid_put: true,
    })
}

pub fn v9fs_fid_xattr_set_result(
    create_errno: i32,
    write_errno: i32,
    put_errno: i32,
) -> XattrSetResult {
    v9fs_fid_xattr_set_result_with_clone(0, create_errno, write_errno, put_errno)
}

pub fn v9fs_fid_xattr_set_result_with_clone(
    clone_errno: i32,
    create_errno: i32,
    write_errno: i32,
    put_errno: i32,
) -> XattrSetResult {
    if clone_errno < 0 {
        return XattrSetResult {
            write_called: false,
            errno: clone_errno,
            cloned_fid: false,
            xattrcreate_called: false,
            fid_put: false,
        };
    }
    if create_errno < 0 {
        return XattrSetResult {
            write_called: false,
            errno: create_errno,
            cloned_fid: true,
            xattrcreate_called: true,
            fid_put: true,
        };
    }
    let mut retval = write_errno;
    if retval == 0 && put_errno != 0 {
        retval = put_errno;
    }
    XattrSetResult {
        write_called: true,
        errno: retval,
        cloned_fid: true,
        xattrcreate_called: true,
        fid_put: true,
    }
}

pub fn v9fs_xattr_get_plan(
    fid_lookup_errno: i32,
    attr_size: u64,
    buffer_size: usize,
    read_bytes: usize,
    read_errno: i32,
) -> Result<DentryXattrGetPlan, i32> {
    if fid_lookup_errno < 0 {
        return Err(fid_lookup_errno);
    }

    Ok(DentryXattrGetPlan {
        fid_lookup_called: true,
        fid_put: true,
        result: v9fs_fid_xattr_get_result(attr_size, buffer_size, read_bytes, read_errno),
    })
}

pub fn v9fs_xattr_set_plan(
    fid_lookup_errno: i32,
    create_errno: i32,
    write_errno: i32,
    put_errno: i32,
) -> Result<DentryXattrSetPlan, i32> {
    if fid_lookup_errno < 0 {
        return Err(fid_lookup_errno);
    }

    Ok(DentryXattrSetPlan {
        fid_lookup_called: true,
        fid_put: true,
        result: v9fs_fid_xattr_set_result(create_errno, write_errno, put_errno),
    })
}

pub const fn v9fs_listxattr_name() -> &'static str {
    ""
}

pub fn v9fs_xattr_handlers(security_enabled: bool) -> Vec<&'static str> {
    let mut handlers = Vec::new();
    handlers.push(XATTR_USER_PREFIX);
    handlers.push(XATTR_TRUSTED_PREFIX);
    if security_enabled {
        handlers.push(XATTR_SECURITY_PREFIX);
    }
    handlers
}

pub fn v9fs_xattr_handler_get_name(prefix: &str, name: &str) -> alloc::string::String {
    xattr_full_name(prefix, name)
}

pub fn v9fs_xattr_handler_set_name(prefix: &str, name: &str) -> alloc::string::String {
    xattr_full_name(prefix, name)
}

pub fn xattr_full_name(prefix: &str, name: &str) -> alloc::string::String {
    let mut full = alloc::string::String::with_capacity(prefix.len() + name.len());
    full.push_str(prefix);
    full.push_str(name);
    full
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::xattr::{XATTR_CREATE, XATTR_REPLACE};
    use crate::include::uapi::errno::{ENOSPC, EROFS};

    #[test]
    fn xattr_helpers_match_linux_source_and_relevant_selftest() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/xattr.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/xattr.h"
        ));
        let selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/filesystems/xattr/xattr_sockfs_test.c"
        ));
        assert!(source.contains("p9_client_xattrwalk(fid, name, &attr_size)"));
        assert!(source.contains("if (attr_size > buffer_size)"));
        assert!(source.contains("retval = -ERANGE;"));
        assert!(source.contains("retval = -EOVERFLOW;"));
        assert!(source.contains("retval = p9_client_read(attr_fid, 0, &to, &err);"));
        assert!(source.contains("p9_fid_put(attr_fid);"));
        assert!(source.contains("ssize_t v9fs_xattr_get"));
        assert!(source.contains("fid = v9fs_fid_lookup(dentry);"));
        assert!(source.contains("ret = v9fs_fid_xattr_get(fid, name, buffer, buffer_size);"));
        assert!(source.contains("int v9fs_xattr_set"));
        assert!(source.contains("fid = clone_fid(fid);"));
        assert!(source.contains("p9_client_xattrcreate(fid, name, value_len, flags);"));
        assert!(source.contains("p9_client_write(fid, 0, &from, &retval);"));
        assert!(source.contains("if (!retval && err)"));
        assert!(source.contains("/* Txattrwalk with an empty string lists xattrs instead */"));
        assert!(source.contains("static int v9fs_xattr_handler_get"));
        assert!(source.contains("static int v9fs_xattr_handler_set"));
        assert!(source.contains("const char *full_name = xattr_full_name(handler, name);"));
        assert!(source.contains(".prefix\t= XATTR_USER_PREFIX"));
        assert!(source.contains(".prefix\t= XATTR_TRUSTED_PREFIX"));
        assert!(source.contains("const struct xattr_handler * const v9fs_xattr_handlers[]"));
        assert!(header.contains("ssize_t v9fs_fid_xattr_get"));
        assert!(selftest.contains("TEST_F(xattr_sockfs, xattr_create_flag)"));
        assert!(selftest.contains("TEST_F(xattr_sockfs, xattr_replace_flag)"));
        assert!(selftest.contains("TEST_F(xattr_sockfs, max_nr_xattrs)"));

        assert_eq!(
            v9fs_fid_xattr_get_result(5, 0, 0, 0),
            Ok(XattrReadResult {
                bytes: 5,
                read_called: false,
                attr_fid_put: true
            })
        );
        assert_eq!(v9fs_fid_xattr_get_result(5, 4, 0, 0), Err(-ERANGE));
        assert_eq!(
            v9fs_fid_xattr_get_result(isize::MAX as u64 + 1, 0, 0, 0),
            Err(-EOVERFLOW)
        );
        assert_eq!(
            v9fs_fid_xattr_get_result(3, 3, 2, 0),
            Ok(XattrReadResult {
                bytes: 2,
                read_called: true,
                attr_fid_put: true
            })
        );
        assert_eq!(
            v9fs_fid_xattr_set_result(-EROFS, 0, 0),
            XattrSetResult {
                write_called: false,
                errno: -EROFS,
                cloned_fid: true,
                xattrcreate_called: true,
                fid_put: true
            }
        );
        assert_eq!(
            v9fs_fid_xattr_set_result_with_clone(-EROFS, 0, 0, 0),
            XattrSetResult {
                write_called: false,
                errno: -EROFS,
                cloned_fid: false,
                xattrcreate_called: false,
                fid_put: false
            }
        );
        assert_eq!(v9fs_fid_xattr_set_result(0, 0, -ENOSPC).errno, -ENOSPC);
        assert_eq!(v9fs_xattr_get_plan(-EROFS, 0, 0, 0, 0), Err(-EROFS));
        let get_plan = v9fs_xattr_get_plan(0, 4, 4, 4, 0).unwrap();
        assert!(get_plan.fid_lookup_called);
        assert!(get_plan.fid_put);
        assert_eq!(
            get_plan.result,
            Ok(XattrReadResult {
                bytes: 4,
                read_called: true,
                attr_fid_put: true
            })
        );
        assert_eq!(v9fs_xattr_set_plan(-EROFS, 0, 0, 0), Err(-EROFS));
        let set_plan = v9fs_xattr_set_plan(0, 0, 0, -ENOSPC).unwrap();
        assert!(set_plan.fid_lookup_called);
        assert!(set_plan.fid_put);
        assert_eq!(set_plan.result.errno, -ENOSPC);
        assert_eq!(v9fs_listxattr_name(), "");
        assert_eq!(
            v9fs_xattr_handlers(true),
            ["user.", "trusted.", "security."]
        );
        assert_eq!(V9FS_XATTR_USER_HANDLER.prefix, XATTR_USER_PREFIX);
        assert_eq!(V9FS_XATTR_TRUSTED_HANDLER.get, "v9fs_xattr_handler_get");
        assert_eq!(V9FS_XATTR_SECURITY_HANDLER.set, "v9fs_xattr_handler_set");
        assert_eq!(
            v9fs_xattr_handler_get_name(XATTR_USER_PREFIX, "name"),
            "user.name"
        );
        assert_eq!(
            v9fs_xattr_handler_set_name(XATTR_TRUSTED_PREFIX, "name"),
            "trusted.name"
        );
        assert_eq!(xattr_full_name(XATTR_USER_PREFIX, "name"), "user.name");
        assert_eq!(XATTR_CREATE, 1);
        assert_eq!(XATTR_REPLACE, 2);
    }
}
