//! linux-parity: complete
//! linux-source: vendor/linux/fs/ext2/xattr_trusted.c
//! test-origin: linux:vendor/linux/fs/ext2/xattr_trusted.c
//! ext2 trusted extended attribute handler.

use super::{Ext2XattrHandler, Ext2XattrListGate};

pub const XATTR_TRUSTED_PREFIX: &str = "trusted.";
pub const EXT2_XATTR_INDEX_TRUSTED: u8 = 4;
pub const EXT2_XATTR_TRUSTED_CAPABILITY: &str = "CAP_SYS_ADMIN";
pub const EXT2_XATTR_TRUSTED_GET_BACKEND: &str = "ext2_xattr_get";
pub const EXT2_XATTR_TRUSTED_SET_BACKEND: &str = "ext2_xattr_set";
pub const EXT2_XATTR_TRUSTED_HANDLER: Ext2XattrHandler = Ext2XattrHandler {
    symbol: "ext2_xattr_trusted_handler",
    prefix: XATTR_TRUSTED_PREFIX,
    index: EXT2_XATTR_INDEX_TRUSTED,
    list_function: "ext2_xattr_trusted_list",
    get_function: "ext2_xattr_trusted_get",
    set_function: "ext2_xattr_trusted_set",
    list_gate: Ext2XattrListGate::CapSysAdmin,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext2TrustedXattrGetCall {
    pub backend: &'static str,
    pub index: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext2TrustedXattrSetCall {
    pub backend: &'static str,
    pub index: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub value_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
}

pub const EXT2_XATTR_TRUSTED_GET_CALL: Ext2TrustedXattrGetCall = Ext2TrustedXattrGetCall {
    backend: EXT2_XATTR_TRUSTED_GET_BACKEND,
    index: EXT2_XATTR_INDEX_TRUSTED,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
};

pub const EXT2_XATTR_TRUSTED_SET_CALL: Ext2TrustedXattrSetCall = Ext2TrustedXattrSetCall {
    backend: EXT2_XATTR_TRUSTED_SET_BACKEND,
    index: EXT2_XATTR_INDEX_TRUSTED,
    inode_arg: "inode",
    name_arg: "name",
    value_arg: "value",
    size_arg: "size",
    flags_arg: "flags",
};

pub const fn ext2_xattr_trusted_list(cap_sys_admin: bool) -> bool {
    cap_sys_admin
}

pub const fn ext2_xattr_trusted_index() -> u8 {
    EXT2_XATTR_INDEX_TRUSTED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext2_trusted_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ext2/xattr_trusted.c"
        ));
        assert!(source.contains("#include \"ext2.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("ext2_xattr_trusted_list"));
        assert!(source.contains("struct dentry *dentry"));
        assert!(source.contains("return capable(CAP_SYS_ADMIN);"));
        assert!(source.contains("ext2_xattr_trusted_get(const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("EXT2_XATTR_INDEX_TRUSTED"));
        assert!(source.contains("ext2_xattr_get(inode, EXT2_XATTR_INDEX_TRUSTED"));
        assert!(source.contains("buffer, size);"));
        assert!(source.contains("ext2_xattr_trusted_set(const struct xattr_handler *handler"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *value"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("ext2_xattr_set(inode, EXT2_XATTR_INDEX_TRUSTED"));
        assert!(source.contains("value, size, flags);"));
        assert!(source.contains("const struct xattr_handler ext2_xattr_trusted_handler"));
        assert!(source.contains(".prefix\t= XATTR_TRUSTED_PREFIX"));
        assert!(source.contains(".list\t= ext2_xattr_trusted_list"));
        assert!(source.contains(".get\t= ext2_xattr_trusted_get"));
        assert!(source.contains(".set\t= ext2_xattr_trusted_set"));

        assert_eq!(EXT2_XATTR_TRUSTED_HANDLER.prefix, "trusted.");
        assert_eq!(EXT2_XATTR_TRUSTED_HANDLER.index, EXT2_XATTR_INDEX_TRUSTED);
        assert!(ext2_xattr_trusted_list(true));
        assert!(!ext2_xattr_trusted_list(false));
        assert_eq!(ext2_xattr_trusted_index(), EXT2_XATTR_INDEX_TRUSTED);
        assert_eq!(EXT2_XATTR_TRUSTED_CAPABILITY, "CAP_SYS_ADMIN");
        assert_eq!(
            EXT2_XATTR_TRUSTED_GET_CALL,
            Ext2TrustedXattrGetCall {
                backend: "ext2_xattr_get",
                index: EXT2_XATTR_INDEX_TRUSTED,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
            }
        );
        assert_eq!(
            EXT2_XATTR_TRUSTED_SET_CALL,
            Ext2TrustedXattrSetCall {
                backend: "ext2_xattr_set",
                index: EXT2_XATTR_INDEX_TRUSTED,
                inode_arg: "inode",
                name_arg: "name",
                value_arg: "value",
                size_arg: "size",
                flags_arg: "flags",
            }
        );
    }
}
