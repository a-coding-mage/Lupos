//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2/xattr_user.c
//! test-origin: linux:vendor/linux/fs/jffs2/xattr_user.c
//! JFFS2 user extended attribute handler.

use super::Jffs2XattrHandler;

pub const XATTR_USER_PREFIX: &str = "user.";
pub const JFFS2_XPREFIX_USER: u8 = 1;
pub const JFFS2_USER_GET_BACKEND: &str = "do_jffs2_getxattr";
pub const JFFS2_USER_SET_BACKEND: &str = "do_jffs2_setxattr";
pub const JFFS2_USER_XATTR_HANDLER: Jffs2XattrHandler = Jffs2XattrHandler {
    symbol: "jffs2_user_xattr_handler",
    prefix: XATTR_USER_PREFIX,
    xprefix: JFFS2_XPREFIX_USER,
    list_function: None,
    get_function: "jffs2_user_getxattr",
    set_function: "jffs2_user_setxattr",
    list_gate: None,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2UserGetxattrCall {
    pub backend: &'static str,
    pub xprefix: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2UserSetxattrCall {
    pub backend: &'static str,
    pub xprefix: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
}

pub const JFFS2_USER_GETXATTR_CALL: Jffs2UserGetxattrCall = Jffs2UserGetxattrCall {
    backend: JFFS2_USER_GET_BACKEND,
    xprefix: JFFS2_XPREFIX_USER,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
};

pub const JFFS2_USER_SETXATTR_CALL: Jffs2UserSetxattrCall = Jffs2UserSetxattrCall {
    backend: JFFS2_USER_SET_BACKEND,
    xprefix: JFFS2_XPREFIX_USER,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    flags_arg: "flags",
};

pub const fn jffs2_user_xprefix() -> u8 {
    JFFS2_XPREFIX_USER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jffs2_user_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jffs2/xattr_user.c"
        ));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/jffs2.h>"));
        assert!(source.contains("#include <linux/xattr.h>"));
        assert!(source.contains("#include <linux/mtd/mtd.h>"));
        assert!(source.contains("#include \"nodelist.h\""));
        assert!(source.contains("jffs2_user_getxattr"));
        assert!(source.contains("const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("do_jffs2_getxattr(inode, JFFS2_XPREFIX_USER"));
        assert!(source.contains("name, buffer, size);"));
        assert!(source.contains("jffs2_user_setxattr"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *buffer"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("do_jffs2_setxattr(inode, JFFS2_XPREFIX_USER"));
        assert!(source.contains("name, buffer, size, flags);"));
        assert!(source.contains("const struct xattr_handler jffs2_user_xattr_handler"));
        assert!(source.contains(".prefix = XATTR_USER_PREFIX"));
        assert!(source.contains(".set = jffs2_user_setxattr"));
        assert!(source.contains(".get = jffs2_user_getxattr"));

        assert_eq!(JFFS2_USER_XATTR_HANDLER.prefix, "user.");
        assert_eq!(JFFS2_USER_XATTR_HANDLER.xprefix, JFFS2_XPREFIX_USER);
        assert_eq!(JFFS2_USER_XATTR_HANDLER.list_function, None);
        assert_eq!(jffs2_user_xprefix(), JFFS2_XPREFIX_USER);
        assert_eq!(
            JFFS2_USER_GETXATTR_CALL,
            Jffs2UserGetxattrCall {
                backend: "do_jffs2_getxattr",
                xprefix: JFFS2_XPREFIX_USER,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
            }
        );
        assert_eq!(
            JFFS2_USER_SETXATTR_CALL,
            Jffs2UserSetxattrCall {
                backend: "do_jffs2_setxattr",
                xprefix: JFFS2_XPREFIX_USER,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                flags_arg: "flags",
            }
        );
    }
}
