//! linux-parity: complete
//! linux-source: vendor/linux/fs/ext2/xattr_user.c
//! test-origin: linux:vendor/linux/fs/ext2/xattr_user.c
//! ext2 user extended attribute handler.

use super::{Ext2XattrHandler, Ext2XattrListGate};
use crate::include::uapi::errno::EOPNOTSUPP;

pub const XATTR_USER_PREFIX: &str = "user.";
pub const EXT2_XATTR_INDEX_USER: u8 = 1;
pub const EXT2_XATTR_USER_MOUNT_OPTION: &str = "XATTR_USER";
pub const EXT2_XATTR_USER_GET_BACKEND: &str = "ext2_xattr_get";
pub const EXT2_XATTR_USER_SET_BACKEND: &str = "ext2_xattr_set";
pub const EXT2_XATTR_USER_HANDLER: Ext2XattrHandler = Ext2XattrHandler {
    symbol: "ext2_xattr_user_handler",
    prefix: XATTR_USER_PREFIX,
    index: EXT2_XATTR_INDEX_USER,
    list_function: "ext2_xattr_user_list",
    get_function: "ext2_xattr_user_get",
    set_function: "ext2_xattr_user_set",
    list_gate: Ext2XattrListGate::MountOptionXattrUser,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext2UserXattrGetCall {
    pub requires_mount_option: &'static str,
    pub unsupported_error: i32,
    pub backend: &'static str,
    pub index: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext2UserXattrSetCall {
    pub requires_mount_option: &'static str,
    pub unsupported_error: i32,
    pub backend: &'static str,
    pub index: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub value_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
}

pub const EXT2_XATTR_USER_GET_CALL: Ext2UserXattrGetCall = Ext2UserXattrGetCall {
    requires_mount_option: EXT2_XATTR_USER_MOUNT_OPTION,
    unsupported_error: -EOPNOTSUPP,
    backend: EXT2_XATTR_USER_GET_BACKEND,
    index: EXT2_XATTR_INDEX_USER,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
};

pub const EXT2_XATTR_USER_SET_CALL: Ext2UserXattrSetCall = Ext2UserXattrSetCall {
    requires_mount_option: EXT2_XATTR_USER_MOUNT_OPTION,
    unsupported_error: -EOPNOTSUPP,
    backend: EXT2_XATTR_USER_SET_BACKEND,
    index: EXT2_XATTR_INDEX_USER,
    inode_arg: "inode",
    name_arg: "name",
    value_arg: "value",
    size_arg: "size",
    flags_arg: "flags",
};

pub const fn ext2_xattr_user_list(xattr_user_enabled: bool) -> bool {
    xattr_user_enabled
}

pub const fn ext2_xattr_user_index(xattr_user_enabled: bool) -> Result<u8, i32> {
    if xattr_user_enabled {
        Ok(EXT2_XATTR_INDEX_USER)
    } else {
        Err(-EOPNOTSUPP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext2_user_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ext2/xattr_user.c"
        ));
        assert!(source.contains("#include <linux/init.h>"));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include \"ext2.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("ext2_xattr_user_list"));
        assert!(source.contains("return test_opt(dentry->d_sb, XATTR_USER);"));
        assert!(source.contains("ext2_xattr_user_get(const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("if (!test_opt(inode->i_sb, XATTR_USER))"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert!(source.contains("EXT2_XATTR_INDEX_USER"));
        assert!(source.contains("ext2_xattr_get(inode, EXT2_XATTR_INDEX_USER"));
        assert!(source.contains("name, buffer, size);"));
        assert!(source.contains("ext2_xattr_user_set(const struct xattr_handler *handler"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *value"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("ext2_xattr_set(inode, EXT2_XATTR_INDEX_USER"));
        assert!(source.contains("name, value, size, flags);"));
        assert!(source.contains("const struct xattr_handler ext2_xattr_user_handler"));
        assert!(source.contains(".prefix\t= XATTR_USER_PREFIX"));
        assert!(source.contains(".list\t= ext2_xattr_user_list"));
        assert!(source.contains(".get\t= ext2_xattr_user_get"));
        assert!(source.contains(".set\t= ext2_xattr_user_set"));

        assert_eq!(EXT2_XATTR_USER_HANDLER.prefix, "user.");
        assert_eq!(EXT2_XATTR_USER_HANDLER.index, EXT2_XATTR_INDEX_USER);
        assert!(ext2_xattr_user_list(true));
        assert!(!ext2_xattr_user_list(false));
        assert_eq!(ext2_xattr_user_index(true), Ok(EXT2_XATTR_INDEX_USER));
        assert_eq!(ext2_xattr_user_index(false), Err(-EOPNOTSUPP));
        assert_eq!(EXT2_XATTR_USER_MOUNT_OPTION, "XATTR_USER");
        assert_eq!(
            EXT2_XATTR_USER_GET_CALL,
            Ext2UserXattrGetCall {
                requires_mount_option: "XATTR_USER",
                unsupported_error: -EOPNOTSUPP,
                backend: "ext2_xattr_get",
                index: EXT2_XATTR_INDEX_USER,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
            }
        );
        assert_eq!(
            EXT2_XATTR_USER_SET_CALL,
            Ext2UserXattrSetCall {
                requires_mount_option: "XATTR_USER",
                unsupported_error: -EOPNOTSUPP,
                backend: "ext2_xattr_set",
                index: EXT2_XATTR_INDEX_USER,
                inode_arg: "inode",
                name_arg: "name",
                value_arg: "value",
                size_arg: "size",
                flags_arg: "flags",
            }
        );
    }
}
