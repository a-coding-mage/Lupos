//! linux-parity: complete
//! linux-source: vendor/linux/fs/hfsplus/xattr_security.c
//! test-origin: linux:vendor/linux/fs/hfsplus/xattr_security.c
//! HFS+ security extended attribute handler.

use super::HfsplusXattrHandler;
use crate::include::uapi::errno::ENOMEM;

pub const XATTR_SECURITY_PREFIX: &str = "security.";
pub const XATTR_SECURITY_PREFIX_LEN: usize = XATTR_SECURITY_PREFIX.len();
pub const HFSPLUS_ATTR_MAX_STRLEN: usize = 127;
pub const NLS_MAX_CHARSET_SIZE: usize = 6;
pub const HFSPLUS_SECURITY_XATTR_BUFFER_LEN: usize =
    NLS_MAX_CHARSET_SIZE * HFSPLUS_ATTR_MAX_STRLEN + 1;
pub const HFSPLUS_SECURITY_GET_BACKEND: &str = "hfsplus_getxattr";
pub const HFSPLUS_SECURITY_SET_BACKEND: &str = "hfsplus_setxattr";
pub const HFSPLUS_SECURITY_RAW_SET_BACKEND: &str = "__hfsplus_setxattr";
pub const HFSPLUS_SECURITY_INIT_BACKEND: &str = "security_inode_init_security";
pub const HFSPLUS_INITXATTRS_CALLBACK: &str = "hfsplus_initxattrs";

pub const HFSPLUS_XATTR_SECURITY_HANDLER: HfsplusXattrHandler = HfsplusXattrHandler {
    symbol: "hfsplus_xattr_security_handler",
    prefix: XATTR_SECURITY_PREFIX,
    prefix_len: XATTR_SECURITY_PREFIX_LEN,
    get_function: "hfsplus_security_getxattr",
    set_function: "hfsplus_security_setxattr",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusSecurityGetCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub prefix_arg: &'static str,
    pub prefix_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusSecuritySetCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
    pub prefix_arg: &'static str,
    pub prefix_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusInitxattrNameBuild {
    pub allocation_len: usize,
    pub allocation_flag: &'static str,
    pub empty_name_action: &'static str,
    pub prefix_copy: &'static str,
    pub suffix_copy_offset: usize,
    pub nul_terminator_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusInitxattrSetCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub value_arg: &'static str,
    pub value_len_arg: &'static str,
    pub flags: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusInitSecurityCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub dir_arg: &'static str,
    pub qstr_arg: &'static str,
    pub callback: &'static str,
    pub fs_info_arg: Option<&'static str>,
}

pub const HFSPLUS_SECURITY_GET_CALL: HfsplusSecurityGetCall = HfsplusSecurityGetCall {
    backend: HFSPLUS_SECURITY_GET_BACKEND,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    prefix_arg: "XATTR_SECURITY_PREFIX",
    prefix_len: XATTR_SECURITY_PREFIX_LEN,
};

pub const HFSPLUS_SECURITY_SET_CALL: HfsplusSecuritySetCall = HfsplusSecuritySetCall {
    backend: HFSPLUS_SECURITY_SET_BACKEND,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    flags_arg: "flags",
    prefix_arg: "XATTR_SECURITY_PREFIX",
    prefix_len: XATTR_SECURITY_PREFIX_LEN,
};

pub const HFSPLUS_INITXATTR_NAME_BUILD: HfsplusInitxattrNameBuild = HfsplusInitxattrNameBuild {
    allocation_len: HFSPLUS_SECURITY_XATTR_BUFFER_LEN,
    allocation_flag: "GFP_KERNEL",
    empty_name_action: "continue",
    prefix_copy: "XATTR_SECURITY_PREFIX",
    suffix_copy_offset: XATTR_SECURITY_PREFIX_LEN,
    nul_terminator_len: 1,
};

pub const HFSPLUS_INITXATTR_SET_CALL: HfsplusInitxattrSetCall = HfsplusInitxattrSetCall {
    backend: HFSPLUS_SECURITY_RAW_SET_BACKEND,
    inode_arg: "inode",
    name_arg: "xattr_name",
    value_arg: "xattr->value",
    value_len_arg: "xattr->value_len",
    flags: 0,
};

pub const HFSPLUS_INIT_SECURITY_CALL: HfsplusInitSecurityCall = HfsplusInitSecurityCall {
    backend: HFSPLUS_SECURITY_INIT_BACKEND,
    inode_arg: "inode",
    dir_arg: "dir",
    qstr_arg: "qstr",
    callback: HFSPLUS_INITXATTRS_CALLBACK,
    fs_info_arg: None,
};

pub fn hfsplus_initxattrs_result(xattrs: &[(&str, i32)]) -> i32 {
    hfsplus_initxattrs_result_with_alloc(true, xattrs)
}

pub fn hfsplus_initxattrs_result_with_alloc(allocation_ok: bool, xattrs: &[(&str, i32)]) -> i32 {
    if !allocation_ok {
        return -ENOMEM;
    }
    for (name, result) in xattrs {
        if name.is_empty() {
            continue;
        }
        if *result != 0 {
            return *result;
        }
    }
    0
}

pub const fn hfsplus_init_security_callback() -> &'static str {
    "hfsplus_initxattrs"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hfsplus_security_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hfsplus/xattr_security.c"
        ));
        assert!(source.contains("#include <linux/security.h>"));
        assert!(source.contains("#include <linux/nls.h>"));
        assert!(source.contains("#include \"hfsplus_fs.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("hfsplus_security_getxattr"));
        assert!(source.contains("const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("hfsplus_getxattr(inode, name, buffer, size"));
        assert!(source.contains("XATTR_SECURITY_PREFIX"));
        assert!(source.contains("XATTR_SECURITY_PREFIX_LEN"));
        assert!(source.contains("hfsplus_security_setxattr"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *buffer"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("hfsplus_setxattr(inode, name, buffer, size, flags"));
        assert!(source.contains("static int hfsplus_initxattrs"));
        assert!(source.contains("const struct xattr *xattr;"));
        assert!(source.contains("char *xattr_name;"));
        assert!(source.contains("int err = 0;"));
        assert!(source.contains("kmalloc(NLS_MAX_CHARSET_SIZE * HFSPLUS_ATTR_MAX_STRLEN + 1"));
        assert!(source.contains("GFP_KERNEL"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("for (xattr = xattr_array; xattr->name != NULL; xattr++)"));
        assert!(source.contains("if (!strcmp(xattr->name, \"\"))"));
        assert!(source.contains("continue;"));
        assert!(source.contains("strcpy(xattr_name, XATTR_SECURITY_PREFIX);"));
        assert!(source.contains("XATTR_SECURITY_PREFIX_LEN, xattr->name);"));
        assert!(source.contains("XATTR_SECURITY_PREFIX_LEN + strlen(xattr->name), 0, 1);"));
        assert!(source.contains("__hfsplus_setxattr(inode, xattr_name"));
        assert!(source.contains("xattr->value, xattr->value_len, 0);"));
        assert!(source.contains("if (err)"));
        assert!(source.contains("break;"));
        assert!(source.contains("kfree(xattr_name);"));
        assert!(source.contains("return err;"));
        assert!(source.contains("int hfsplus_init_security(struct inode *inode"));
        assert!(source.contains("security_inode_init_security(inode, dir, qstr"));
        assert!(source.contains("&hfsplus_initxattrs, NULL);"));
        assert!(source.contains("const struct xattr_handler hfsplus_xattr_security_handler"));
        assert!(source.contains(".prefix\t= XATTR_SECURITY_PREFIX"));
        assert!(source.contains(".get\t= hfsplus_security_getxattr"));
        assert!(source.contains(".set\t= hfsplus_security_setxattr"));

        assert_eq!(HFSPLUS_XATTR_SECURITY_HANDLER.prefix, "security.");
        assert_eq!(HFSPLUS_XATTR_SECURITY_HANDLER.prefix_len, 9);
        assert_eq!(HFSPLUS_SECURITY_XATTR_BUFFER_LEN, 763);
        assert_eq!(hfsplus_initxattrs_result_with_alloc(false, &[]), -ENOMEM);
        assert_eq!(hfsplus_initxattrs_result(&[("", -5), ("selinux", 0)]), 0);
        assert_eq!(
            hfsplus_initxattrs_result(&[("selinux", 0), ("smack", -12)]),
            -12
        );
        assert_eq!(hfsplus_init_security_callback(), "hfsplus_initxattrs");
        assert_eq!(
            HFSPLUS_SECURITY_GET_CALL,
            HfsplusSecurityGetCall {
                backend: "hfsplus_getxattr",
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                prefix_arg: "XATTR_SECURITY_PREFIX",
                prefix_len: XATTR_SECURITY_PREFIX_LEN,
            }
        );
        assert_eq!(
            HFSPLUS_SECURITY_SET_CALL,
            HfsplusSecuritySetCall {
                backend: "hfsplus_setxattr",
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                flags_arg: "flags",
                prefix_arg: "XATTR_SECURITY_PREFIX",
                prefix_len: XATTR_SECURITY_PREFIX_LEN,
            }
        );
        assert_eq!(
            HFSPLUS_INITXATTR_NAME_BUILD,
            HfsplusInitxattrNameBuild {
                allocation_len: HFSPLUS_SECURITY_XATTR_BUFFER_LEN,
                allocation_flag: "GFP_KERNEL",
                empty_name_action: "continue",
                prefix_copy: "XATTR_SECURITY_PREFIX",
                suffix_copy_offset: XATTR_SECURITY_PREFIX_LEN,
                nul_terminator_len: 1,
            }
        );
        assert_eq!(
            HFSPLUS_INITXATTR_SET_CALL,
            HfsplusInitxattrSetCall {
                backend: "__hfsplus_setxattr",
                inode_arg: "inode",
                name_arg: "xattr_name",
                value_arg: "xattr->value",
                value_len_arg: "xattr->value_len",
                flags: 0,
            }
        );
        assert_eq!(
            HFSPLUS_INIT_SECURITY_CALL,
            HfsplusInitSecurityCall {
                backend: "security_inode_init_security",
                inode_arg: "inode",
                dir_arg: "dir",
                qstr_arg: "qstr",
                callback: "hfsplus_initxattrs",
                fs_info_arg: None,
            }
        );
    }
}
