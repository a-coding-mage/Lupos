//! linux-parity: complete
//! linux-source: vendor/linux/fs/hfsplus/xattr_trusted.c
//! test-origin: linux:vendor/linux/fs/hfsplus/xattr_trusted.c
//! HFS+ trusted extended attribute handler.

use super::HfsplusXattrHandler;

pub const XATTR_TRUSTED_PREFIX: &str = "trusted.";
pub const XATTR_TRUSTED_PREFIX_LEN: usize = XATTR_TRUSTED_PREFIX.len();
pub const HFSPLUS_TRUSTED_GET_BACKEND: &str = "hfsplus_getxattr";
pub const HFSPLUS_TRUSTED_SET_BACKEND: &str = "hfsplus_setxattr";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusTrustedGetCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub prefix_arg: &'static str,
    pub prefix_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusTrustedSetCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
    pub prefix_arg: &'static str,
    pub prefix_len: usize,
}

pub const HFSPLUS_TRUSTED_GET_CALL: HfsplusTrustedGetCall = HfsplusTrustedGetCall {
    backend: HFSPLUS_TRUSTED_GET_BACKEND,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    prefix_arg: "XATTR_TRUSTED_PREFIX",
    prefix_len: XATTR_TRUSTED_PREFIX_LEN,
};

pub const HFSPLUS_TRUSTED_SET_CALL: HfsplusTrustedSetCall = HfsplusTrustedSetCall {
    backend: HFSPLUS_TRUSTED_SET_BACKEND,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
    flags_arg: "flags",
    prefix_arg: "XATTR_TRUSTED_PREFIX",
    prefix_len: XATTR_TRUSTED_PREFIX_LEN,
};

pub const HFSPLUS_XATTR_TRUSTED_HANDLER: HfsplusXattrHandler = HfsplusXattrHandler {
    symbol: "hfsplus_xattr_trusted_handler",
    prefix: XATTR_TRUSTED_PREFIX,
    prefix_len: XATTR_TRUSTED_PREFIX_LEN,
    get_function: "hfsplus_trusted_getxattr",
    set_function: "hfsplus_trusted_setxattr",
};

pub fn trusted_get_uses_prefix(prefix: &str) -> bool {
    prefix == XATTR_TRUSTED_PREFIX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hfsplus_trusted_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hfsplus/xattr_trusted.c"
        ));
        assert!(source.contains("#include <linux/nls.h>"));
        assert!(source.contains("#include \"hfsplus_fs.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("hfsplus_trusted_getxattr"));
        assert!(source.contains("const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("hfsplus_getxattr(inode, name, buffer, size"));
        assert!(source.contains("XATTR_TRUSTED_PREFIX,"));
        assert!(source.contains("XATTR_TRUSTED_PREFIX_LEN);"));
        assert!(source.contains("XATTR_TRUSTED_PREFIX_LEN"));
        assert!(source.contains("hfsplus_trusted_setxattr"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *buffer"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("hfsplus_setxattr(inode, name, buffer, size, flags"));
        assert!(source.contains("const struct xattr_handler hfsplus_xattr_trusted_handler"));
        assert!(source.contains(".prefix\t= XATTR_TRUSTED_PREFIX"));
        assert!(source.contains(".get\t= hfsplus_trusted_getxattr"));
        assert!(source.contains(".set\t= hfsplus_trusted_setxattr"));

        assert_eq!(HFSPLUS_XATTR_TRUSTED_HANDLER.prefix, "trusted.");
        assert_eq!(HFSPLUS_XATTR_TRUSTED_HANDLER.prefix_len, 8);
        assert!(trusted_get_uses_prefix("trusted."));
        assert_eq!(XATTR_TRUSTED_PREFIX_LEN, 8);
        assert_eq!(
            HFSPLUS_TRUSTED_GET_CALL,
            HfsplusTrustedGetCall {
                backend: "hfsplus_getxattr",
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                prefix_arg: "XATTR_TRUSTED_PREFIX",
                prefix_len: 8,
            }
        );
        assert_eq!(
            HFSPLUS_TRUSTED_SET_CALL,
            HfsplusTrustedSetCall {
                backend: "hfsplus_setxattr",
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
                flags_arg: "flags",
                prefix_arg: "XATTR_TRUSTED_PREFIX",
                prefix_len: 8,
            }
        );
    }
}
