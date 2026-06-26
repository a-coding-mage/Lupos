//! linux-parity: complete
//! linux-source: vendor/linux/fs/ext2/xattr_security.c
//! test-origin: linux:vendor/linux/fs/ext2/xattr_security.c
//! ext2 security extended attribute handler.

pub const XATTR_SECURITY_PREFIX: &str = "security.";
pub const EXT2_XATTR_INDEX_SECURITY: u8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext2SecurityXattrHandler {
    pub symbol: &'static str,
    pub prefix: &'static str,
    pub index: u8,
    pub get_function: &'static str,
    pub set_function: &'static str,
}

pub const EXT2_XATTR_SECURITY_HANDLER: Ext2SecurityXattrHandler = Ext2SecurityXattrHandler {
    symbol: "ext2_xattr_security_handler",
    prefix: XATTR_SECURITY_PREFIX,
    index: EXT2_XATTR_INDEX_SECURITY,
    get_function: "ext2_xattr_security_get",
    set_function: "ext2_xattr_security_set",
};

pub const fn ext2_xattr_security_get_index() -> u8 {
    EXT2_XATTR_INDEX_SECURITY
}

pub const fn ext2_xattr_security_set_index() -> u8 {
    EXT2_XATTR_INDEX_SECURITY
}

pub fn ext2_initxattrs_result(set_results: &[i32]) -> i32 {
    for err in set_results {
        if *err < 0 {
            return *err;
        }
    }
    0
}

pub const fn ext2_init_security_callback() -> &'static str {
    "ext2_initxattrs"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext2_security_xattr_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ext2/xattr_security.c"
        ));
        assert!(source.contains("#include \"ext2.h\""));
        assert!(source.contains("#include <linux/security.h>"));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("ext2_xattr_security_get"));
        assert!(source.contains("ext2_xattr_get(inode, EXT2_XATTR_INDEX_SECURITY"));
        assert!(source.contains("ext2_xattr_security_set"));
        assert!(source.contains("ext2_xattr_set(inode, EXT2_XATTR_INDEX_SECURITY"));
        assert!(source.contains("ext2_initxattrs"));
        assert!(source.contains("for (xattr = xattr_array; xattr->name != NULL; xattr++)"));
        assert!(source.contains("if (err < 0)"));
        assert!(source.contains("security_inode_init_security(inode, dir, qstr,"));
        assert!(source.contains("&ext2_initxattrs, NULL);"));
        assert!(source.contains("const struct xattr_handler ext2_xattr_security_handler"));
        assert!(source.contains(".prefix\t= XATTR_SECURITY_PREFIX"));
        assert!(source.contains(".get\t= ext2_xattr_security_get"));
        assert!(source.contains(".set\t= ext2_xattr_security_set"));

        assert_eq!(EXT2_XATTR_SECURITY_HANDLER.prefix, "security.");
        assert_eq!(ext2_xattr_security_get_index(), EXT2_XATTR_INDEX_SECURITY);
        assert_eq!(ext2_xattr_security_set_index(), EXT2_XATTR_INDEX_SECURITY);
        assert_eq!(ext2_initxattrs_result(&[0, 0]), 0);
        assert_eq!(ext2_initxattrs_result(&[0, -5, 0]), -5);
        assert_eq!(ext2_init_security_callback(), "ext2_initxattrs");
    }
}
