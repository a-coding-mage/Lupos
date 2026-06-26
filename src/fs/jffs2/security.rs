//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2/security.c
//! test-origin: linux:vendor/linux/fs/jffs2/security.c
//! JFFS2 security xattr initialization and handler metadata.

use super::Jffs2XattrHandler;

pub const XATTR_SECURITY_PREFIX: &str = "security.";
pub const JFFS2_XPREFIX_SECURITY: u8 = 2;
pub const JFFS2_SECURITY_SET_BACKEND: &str = "do_jffs2_setxattr";
pub const JFFS2_SECURITY_GET_BACKEND: &str = "do_jffs2_getxattr";
pub const JFFS2_SECURITY_INIT_BACKEND: &str = "security_inode_init_security";
pub const JFFS2_INITXATTRS_CALLBACK: &str = "jffs2_initxattrs";
pub const JFFS2_SECURITY_XATTR_HANDLER: Jffs2XattrHandler = Jffs2XattrHandler {
    symbol: "jffs2_security_xattr_handler",
    prefix: XATTR_SECURITY_PREFIX,
    xprefix: JFFS2_XPREFIX_SECURITY,
    list_function: None,
    get_function: "jffs2_security_getxattr",
    set_function: "jffs2_security_setxattr",
    list_gate: None,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2InitxattrsSetCall {
    pub backend: &'static str,
    pub xprefix: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub value_arg: &'static str,
    pub value_len_arg: &'static str,
    pub flags: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2InitSecurityCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub dir_arg: &'static str,
    pub qstr_arg: &'static str,
    pub callback: &'static str,
    pub fs_info_arg: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2SecurityGetxattrCall {
    pub backend: &'static str,
    pub xprefix: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2SecuritySetxattrCall {
    pub backend: &'static str,
    pub xprefix: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
}

pub const JFFS2_INITXATTRS_SET_CALL: Jffs2InitxattrsSetCall = Jffs2InitxattrsSetCall {
    backend: JFFS2_SECURITY_SET_BACKEND,
    xprefix: JFFS2_XPREFIX_SECURITY,
    inode_arg: "inode",
    name_arg: "xattr->name",
    value_arg: "xattr->value",
    value_len_arg: "xattr->value_len",
    flags: 0,
};

pub const JFFS2_INIT_SECURITY_CALL: Jffs2InitSecurityCall = Jffs2InitSecurityCall {
    backend: JFFS2_SECURITY_INIT_BACKEND,
    inode_arg: "inode",
    dir_arg: "dir",
    qstr_arg: "qstr",
    callback: JFFS2_INITXATTRS_CALLBACK,
    fs_info_arg: None,
};

pub const JFFS2_SECURITY_GETXATTR_CALL: Jffs2SecurityGetxattrCall = Jffs2SecurityGetxattrCall {
    backend: JFFS2_SECURITY_GET_BACKEND,
    xprefix: JFFS2_XPREFIX_SECURITY,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
};

pub const JFFS2_SECURITY_SETXATTR_CALL: Jffs2SecuritySetxattrCall = Jffs2SecuritySetxattrCall {
    backend: JFFS2_SECURITY_SET_BACKEND,
    xprefix: JFFS2_XPREFIX_SECURITY,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    flags_arg: "flags",
};

pub fn jffs2_initxattrs_result(setxattr_results: &[i32]) -> i32 {
    for &result in setxattr_results {
        if result < 0 {
            return result;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jffs2_security_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jffs2/security.c"
        ));
        assert!(source.contains("#include <linux/security.h>"));
        assert!(source.contains("#include \"nodelist.h\""));
        assert!(source.contains("static int jffs2_initxattrs"));
        assert!(source.contains("for (xattr = xattr_array; xattr->name != NULL; xattr++)"));
        assert!(source.contains("do_jffs2_setxattr(inode, JFFS2_XPREFIX_SECURITY"));
        assert!(source.contains("xattr->name, xattr->value"));
        assert!(source.contains("xattr->value_len, 0);"));
        assert!(source.contains("if (err < 0)"));
        assert!(source.contains("break;"));
        assert!(source.contains("return err;"));
        assert!(source.contains("int jffs2_init_security(struct inode *inode, struct inode *dir"));
        assert!(source.contains("security_inode_init_security"));
        assert!(source.contains("&jffs2_initxattrs, NULL);"));
        assert!(source.contains("jffs2_security_getxattr"));
        assert!(source.contains("const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("do_jffs2_getxattr(inode, JFFS2_XPREFIX_SECURITY"));
        assert!(source.contains("name, buffer, size);"));
        assert!(source.contains("jffs2_security_setxattr"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *buffer"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("name, buffer, size, flags);"));
        assert!(source.contains("const struct xattr_handler jffs2_security_xattr_handler"));
        assert!(source.contains(".prefix = XATTR_SECURITY_PREFIX"));
        assert!(source.contains(".set = jffs2_security_setxattr"));
        assert!(source.contains(".get = jffs2_security_getxattr"));

        assert_eq!(JFFS2_SECURITY_XATTR_HANDLER.prefix, "security.");
        assert_eq!(JFFS2_SECURITY_XATTR_HANDLER.xprefix, JFFS2_XPREFIX_SECURITY);
        assert_eq!(jffs2_initxattrs_result(&[0, 0, 0]), 0);
        assert_eq!(jffs2_initxattrs_result(&[0, -5, -1]), -5);
        assert_eq!(
            JFFS2_INITXATTRS_SET_CALL,
            Jffs2InitxattrsSetCall {
                backend: "do_jffs2_setxattr",
                xprefix: JFFS2_XPREFIX_SECURITY,
                inode_arg: "inode",
                name_arg: "xattr->name",
                value_arg: "xattr->value",
                value_len_arg: "xattr->value_len",
                flags: 0,
            }
        );
        assert_eq!(
            JFFS2_INIT_SECURITY_CALL,
            Jffs2InitSecurityCall {
                backend: "security_inode_init_security",
                inode_arg: "inode",
                dir_arg: "dir",
                qstr_arg: "qstr",
                callback: "jffs2_initxattrs",
                fs_info_arg: None,
            }
        );
        assert_eq!(
            JFFS2_SECURITY_GETXATTR_CALL,
            Jffs2SecurityGetxattrCall {
                backend: "do_jffs2_getxattr",
                xprefix: JFFS2_XPREFIX_SECURITY,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
            }
        );
        assert_eq!(
            JFFS2_SECURITY_SETXATTR_CALL,
            Jffs2SecuritySetxattrCall {
                backend: "do_jffs2_setxattr",
                xprefix: JFFS2_XPREFIX_SECURITY,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                flags_arg: "flags",
            }
        );
    }
}
