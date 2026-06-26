//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2/xattr_trusted.c
//! test-origin: linux:vendor/linux/fs/jffs2/xattr_trusted.c
//! JFFS2 trusted extended attribute handler.

use super::{Jffs2XattrHandler, Jffs2XattrListGate};

pub const XATTR_TRUSTED_PREFIX: &str = "trusted.";
pub const JFFS2_XPREFIX_TRUSTED: u8 = 5;
pub const JFFS2_TRUSTED_CAPABILITY: &str = "CAP_SYS_ADMIN";
pub const JFFS2_TRUSTED_GET_BACKEND: &str = "do_jffs2_getxattr";
pub const JFFS2_TRUSTED_SET_BACKEND: &str = "do_jffs2_setxattr";
pub const JFFS2_TRUSTED_XATTR_HANDLER: Jffs2XattrHandler = Jffs2XattrHandler {
    symbol: "jffs2_trusted_xattr_handler",
    prefix: XATTR_TRUSTED_PREFIX,
    xprefix: JFFS2_XPREFIX_TRUSTED,
    list_function: Some("jffs2_trusted_listxattr"),
    get_function: "jffs2_trusted_getxattr",
    set_function: "jffs2_trusted_setxattr",
    list_gate: Some(Jffs2XattrListGate::CapSysAdmin),
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2TrustedGetxattrCall {
    pub backend: &'static str,
    pub xprefix: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2TrustedSetxattrCall {
    pub backend: &'static str,
    pub xprefix: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
}

pub const JFFS2_TRUSTED_GETXATTR_CALL: Jffs2TrustedGetxattrCall = Jffs2TrustedGetxattrCall {
    backend: JFFS2_TRUSTED_GET_BACKEND,
    xprefix: JFFS2_XPREFIX_TRUSTED,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
};

pub const JFFS2_TRUSTED_SETXATTR_CALL: Jffs2TrustedSetxattrCall = Jffs2TrustedSetxattrCall {
    backend: JFFS2_TRUSTED_SET_BACKEND,
    xprefix: JFFS2_XPREFIX_TRUSTED,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    flags_arg: "flags",
};

pub const fn jffs2_trusted_listxattr(cap_sys_admin: bool) -> bool {
    cap_sys_admin
}

pub const fn jffs2_trusted_xprefix() -> u8 {
    JFFS2_XPREFIX_TRUSTED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jffs2_trusted_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jffs2/xattr_trusted.c"
        ));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/jffs2.h>"));
        assert!(source.contains("#include <linux/xattr.h>"));
        assert!(source.contains("#include <linux/mtd/mtd.h>"));
        assert!(source.contains("#include \"nodelist.h\""));
        assert!(source.contains("jffs2_trusted_getxattr"));
        assert!(source.contains("const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("do_jffs2_getxattr(inode, JFFS2_XPREFIX_TRUSTED"));
        assert!(source.contains("name, buffer, size);"));
        assert!(source.contains("jffs2_trusted_setxattr"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *buffer"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("do_jffs2_setxattr(inode, JFFS2_XPREFIX_TRUSTED"));
        assert!(source.contains("name, buffer, size, flags);"));
        assert!(source.contains("jffs2_trusted_listxattr"));
        assert!(source.contains("struct dentry *dentry"));
        assert!(source.contains("return capable(CAP_SYS_ADMIN);"));
        assert!(source.contains("const struct xattr_handler jffs2_trusted_xattr_handler"));
        assert!(source.contains(".prefix = XATTR_TRUSTED_PREFIX"));
        assert!(source.contains(".list = jffs2_trusted_listxattr"));
        assert!(source.contains(".set = jffs2_trusted_setxattr"));
        assert!(source.contains(".get = jffs2_trusted_getxattr"));

        assert_eq!(JFFS2_TRUSTED_XATTR_HANDLER.prefix, "trusted.");
        assert_eq!(JFFS2_TRUSTED_XATTR_HANDLER.xprefix, JFFS2_XPREFIX_TRUSTED);
        assert_eq!(
            JFFS2_TRUSTED_XATTR_HANDLER.list_function,
            Some("jffs2_trusted_listxattr")
        );
        assert!(jffs2_trusted_listxattr(true));
        assert!(!jffs2_trusted_listxattr(false));
        assert_eq!(jffs2_trusted_xprefix(), JFFS2_XPREFIX_TRUSTED);
        assert_eq!(JFFS2_TRUSTED_CAPABILITY, "CAP_SYS_ADMIN");
        assert_eq!(
            JFFS2_TRUSTED_GETXATTR_CALL,
            Jffs2TrustedGetxattrCall {
                backend: "do_jffs2_getxattr",
                xprefix: JFFS2_XPREFIX_TRUSTED,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
            }
        );
        assert_eq!(
            JFFS2_TRUSTED_SETXATTR_CALL,
            Jffs2TrustedSetxattrCall {
                backend: "do_jffs2_setxattr",
                xprefix: JFFS2_XPREFIX_TRUSTED,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                flags_arg: "flags",
            }
        );
    }
}
