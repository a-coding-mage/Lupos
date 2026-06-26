//! linux-parity: complete
//! linux-source: vendor/linux/fs/hfsplus/xattr_user.c
//! test-origin: linux:vendor/linux/fs/hfsplus/xattr_user.c
//! HFS+ user extended attribute handler.

use super::HfsplusXattrHandler;

pub const XATTR_USER_PREFIX: &str = "user.";
pub const XATTR_USER_PREFIX_LEN: usize = XATTR_USER_PREFIX.len();
pub const HFSPLUS_USER_GET_BACKEND: &str = "hfsplus_getxattr";
pub const HFSPLUS_USER_SET_BACKEND: &str = "hfsplus_setxattr";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusUserGetCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub prefix_arg: &'static str,
    pub prefix_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusUserSetCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
    pub prefix_arg: &'static str,
    pub prefix_len: usize,
}

pub const HFSPLUS_USER_GET_CALL: HfsplusUserGetCall = HfsplusUserGetCall {
    backend: HFSPLUS_USER_GET_BACKEND,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    prefix_arg: "XATTR_USER_PREFIX",
    prefix_len: XATTR_USER_PREFIX_LEN,
};

pub const HFSPLUS_USER_SET_CALL: HfsplusUserSetCall = HfsplusUserSetCall {
    backend: HFSPLUS_USER_SET_BACKEND,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    flags_arg: "flags",
    prefix_arg: "XATTR_USER_PREFIX",
    prefix_len: XATTR_USER_PREFIX_LEN,
};

pub const HFSPLUS_XATTR_USER_HANDLER: HfsplusXattrHandler = HfsplusXattrHandler {
    symbol: "hfsplus_xattr_user_handler",
    prefix: XATTR_USER_PREFIX,
    prefix_len: XATTR_USER_PREFIX_LEN,
    get_function: "hfsplus_user_getxattr",
    set_function: "hfsplus_user_setxattr",
};

pub fn user_get_uses_prefix(prefix: &str) -> bool {
    prefix == XATTR_USER_PREFIX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hfsplus_user_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hfsplus/xattr_user.c"
        ));
        assert!(source.contains("#include <linux/nls.h>"));
        assert!(source.contains("#include \"hfsplus_fs.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("hfsplus_user_getxattr"));
        assert!(source.contains("const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("hfsplus_getxattr(inode, name, buffer, size"));
        assert!(source.contains("XATTR_USER_PREFIX, XATTR_USER_PREFIX_LEN);"));
        assert!(source.contains("XATTR_USER_PREFIX_LEN"));
        assert!(source.contains("hfsplus_user_setxattr"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *buffer"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("hfsplus_setxattr(inode, name, buffer, size, flags"));
        assert!(source.contains("const struct xattr_handler hfsplus_xattr_user_handler"));
        assert!(source.contains(".prefix\t= XATTR_USER_PREFIX"));
        assert!(source.contains(".get\t= hfsplus_user_getxattr"));
        assert!(source.contains(".set\t= hfsplus_user_setxattr"));

        assert_eq!(HFSPLUS_XATTR_USER_HANDLER.prefix, "user.");
        assert_eq!(HFSPLUS_XATTR_USER_HANDLER.prefix_len, 5);
        assert!(user_get_uses_prefix("user."));
        assert_eq!(XATTR_USER_PREFIX_LEN, 5);
        assert_eq!(
            HFSPLUS_USER_GET_CALL,
            HfsplusUserGetCall {
                backend: "hfsplus_getxattr",
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                prefix_arg: "XATTR_USER_PREFIX",
                prefix_len: 5,
            }
        );
        assert_eq!(
            HFSPLUS_USER_SET_CALL,
            HfsplusUserSetCall {
                backend: "hfsplus_setxattr",
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                flags_arg: "flags",
                prefix_arg: "XATTR_USER_PREFIX",
                prefix_len: 5,
            }
        );
    }
}
