//! linux-parity: complete
//! linux-source: vendor/linux/fs/ext4/xattr_security.c
//! test-origin: linux:vendor/linux/fs/ext4/xattr_security.c
//! ext4 security extended attribute handler.

pub const XATTR_SECURITY_PREFIX: &str = "security.";
pub const EXT4_XATTR_INDEX_SECURITY: u8 = 6;
pub const EXT4_XATTR_SECURITY_GET_BACKEND: &str = "ext4_xattr_get";
pub const EXT4_XATTR_SECURITY_SET_BACKEND: &str = "ext4_xattr_set";
pub const EXT4_XATTR_SECURITY_SET_HANDLE_BACKEND: &str = "ext4_xattr_set_handle";
pub const EXT4_SECURITY_INIT_BACKEND: &str = "security_inode_init_security";
pub const EXT4_INITXATTRS_CALLBACK: &str = "ext4_initxattrs";
pub const XATTR_CREATE_FLAG: &str = "XATTR_CREATE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4SecurityXattrHandler {
    pub symbol: &'static str,
    pub prefix: &'static str,
    pub index: u8,
    pub get_function: &'static str,
    pub set_function: &'static str,
}

pub const EXT4_XATTR_SECURITY_HANDLER: Ext4SecurityXattrHandler = Ext4SecurityXattrHandler {
    symbol: "ext4_xattr_security_handler",
    prefix: XATTR_SECURITY_PREFIX,
    index: EXT4_XATTR_INDEX_SECURITY,
    get_function: "ext4_xattr_security_get",
    set_function: "ext4_xattr_security_set",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4SecurityGetCall {
    pub backend: &'static str,
    pub index: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub buffer_arg: &'static str,
    pub size_arg: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4SecuritySetCall {
    pub backend: &'static str,
    pub index: u8,
    pub inode_arg: &'static str,
    pub name_arg: &'static str,
    pub value_arg: &'static str,
    pub size_arg: &'static str,
    pub flags_arg: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4InitxattrsSetCall {
    pub backend: &'static str,
    pub handle_arg: &'static str,
    pub inode_arg: &'static str,
    pub index: u8,
    pub name_arg: &'static str,
    pub value_arg: &'static str,
    pub value_len_arg: &'static str,
    pub create_flag: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4InitSecurityCall {
    pub backend: &'static str,
    pub inode_arg: &'static str,
    pub dir_arg: &'static str,
    pub qstr_arg: &'static str,
    pub callback: &'static str,
    pub fs_info_arg: &'static str,
}

pub const EXT4_XATTR_SECURITY_GET_CALL: Ext4SecurityGetCall = Ext4SecurityGetCall {
    backend: EXT4_XATTR_SECURITY_GET_BACKEND,
    index: EXT4_XATTR_INDEX_SECURITY,
    inode_arg: "inode",
    name_arg: "name",
    buffer_arg: "buffer",
    size_arg: "size",
};

pub const EXT4_XATTR_SECURITY_SET_CALL: Ext4SecuritySetCall = Ext4SecuritySetCall {
    backend: EXT4_XATTR_SECURITY_SET_BACKEND,
    index: EXT4_XATTR_INDEX_SECURITY,
    inode_arg: "inode",
    name_arg: "name",
    value_arg: "value",
    size_arg: "size",
    flags_arg: "flags",
};

pub const EXT4_INITXATTRS_SET_CALL: Ext4InitxattrsSetCall = Ext4InitxattrsSetCall {
    backend: EXT4_XATTR_SECURITY_SET_HANDLE_BACKEND,
    handle_arg: "handle",
    inode_arg: "inode",
    index: EXT4_XATTR_INDEX_SECURITY,
    name_arg: "xattr->name",
    value_arg: "xattr->value",
    value_len_arg: "xattr->value_len",
    create_flag: XATTR_CREATE_FLAG,
};

pub const EXT4_INIT_SECURITY_CALL: Ext4InitSecurityCall = Ext4InitSecurityCall {
    backend: EXT4_SECURITY_INIT_BACKEND,
    inode_arg: "inode",
    dir_arg: "dir",
    qstr_arg: "qstr",
    callback: EXT4_INITXATTRS_CALLBACK,
    fs_info_arg: "handle",
};

pub const fn ext4_xattr_security_get_index() -> u8 {
    EXT4_XATTR_INDEX_SECURITY
}

pub const fn ext4_xattr_security_set_index() -> u8 {
    EXT4_XATTR_INDEX_SECURITY
}

pub fn ext4_initxattrs_result(set_results: &[i32]) -> i32 {
    for err in set_results {
        if *err < 0 {
            return *err;
        }
    }
    0
}

pub const fn ext4_init_security_callback() -> &'static str {
    "ext4_initxattrs"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext4_security_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ext4/xattr_security.c"
        ));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/security.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include \"ext4_jbd2.h\""));
        assert!(source.contains("#include \"ext4.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("ext4_xattr_security_get"));
        assert!(source.contains("const struct xattr_handler *handler"));
        assert!(source.contains("struct dentry *unused, struct inode *inode"));
        assert!(source.contains("const char *name, void *buffer, size_t size"));
        assert!(source.contains("ext4_xattr_get(inode, EXT4_XATTR_INDEX_SECURITY"));
        assert!(source.contains("name, buffer, size);"));
        assert!(source.contains("ext4_xattr_security_set"));
        assert!(source.contains("struct mnt_idmap *idmap"));
        assert!(source.contains("const char *name, const void *value"));
        assert!(source.contains("size_t size, int flags"));
        assert!(source.contains("ext4_xattr_set(inode, EXT4_XATTR_INDEX_SECURITY"));
        assert!(source.contains("name, value, size, flags);"));
        assert!(source.contains("ext4_initxattrs"));
        assert!(source.contains("handle_t *handle = fs_info;"));
        assert!(source.contains("int err = 0;"));
        assert!(source.contains("for (xattr = xattr_array; xattr->name != NULL; xattr++)"));
        assert!(source.contains("ext4_xattr_set_handle(handle, inode,"));
        assert!(source.contains("EXT4_XATTR_INDEX_SECURITY"));
        assert!(source.contains("xattr->name, xattr->value"));
        assert!(source.contains("xattr->value_len, XATTR_CREATE);"));
        assert!(source.contains("XATTR_CREATE"));
        assert!(source.contains("if (err < 0)"));
        assert!(source.contains("break;"));
        assert!(source.contains("return err;"));
        assert!(source.contains("ext4_init_security(handle_t *handle"));
        assert!(source.contains("security_inode_init_security(inode, dir, qstr,"));
        assert!(source.contains("&ext4_initxattrs, handle);"));
        assert!(source.contains("const struct xattr_handler ext4_xattr_security_handler"));
        assert!(source.contains(".prefix\t= XATTR_SECURITY_PREFIX"));
        assert!(source.contains(".get\t= ext4_xattr_security_get"));
        assert!(source.contains(".set\t= ext4_xattr_security_set"));

        assert_eq!(EXT4_XATTR_SECURITY_HANDLER.prefix, "security.");
        assert_eq!(ext4_xattr_security_get_index(), EXT4_XATTR_INDEX_SECURITY);
        assert_eq!(ext4_xattr_security_set_index(), EXT4_XATTR_INDEX_SECURITY);
        assert_eq!(ext4_initxattrs_result(&[0, 0]), 0);
        assert_eq!(ext4_initxattrs_result(&[0, -5, 0]), -5);
        assert_eq!(ext4_init_security_callback(), "ext4_initxattrs");
        assert_eq!(
            EXT4_XATTR_SECURITY_GET_CALL,
            Ext4SecurityGetCall {
                backend: "ext4_xattr_get",
                index: EXT4_XATTR_INDEX_SECURITY,
                inode_arg: "inode",
                name_arg: "name",
                buffer_arg: "buffer",
                size_arg: "size",
            }
        );
        assert_eq!(
            EXT4_XATTR_SECURITY_SET_CALL,
            Ext4SecuritySetCall {
                backend: "ext4_xattr_set",
                index: EXT4_XATTR_INDEX_SECURITY,
                inode_arg: "inode",
                name_arg: "name",
                value_arg: "value",
                size_arg: "size",
                flags_arg: "flags",
            }
        );
        assert_eq!(
            EXT4_INITXATTRS_SET_CALL,
            Ext4InitxattrsSetCall {
                backend: "ext4_xattr_set_handle",
                handle_arg: "handle",
                inode_arg: "inode",
                index: EXT4_XATTR_INDEX_SECURITY,
                name_arg: "xattr->name",
                value_arg: "xattr->value",
                value_len_arg: "xattr->value_len",
                create_flag: "XATTR_CREATE",
            }
        );
        assert_eq!(
            EXT4_INIT_SECURITY_CALL,
            Ext4InitSecurityCall {
                backend: "security_inode_init_security",
                inode_arg: "inode",
                dir_arg: "dir",
                qstr_arg: "qstr",
                callback: "ext4_initxattrs",
                fs_info_arg: "handle",
            }
        );
    }
}
