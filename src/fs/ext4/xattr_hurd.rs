//! linux-parity: complete
//! linux-source: vendor/linux/fs/ext4/xattr_hurd.c
//! test-origin: linux:vendor/linux/fs/ext4/xattr_hurd.c
//! ext4 Hurd extended attribute handler.

use crate::include::uapi::errno::EOPNOTSUPP;

pub const XATTR_HURD_PREFIX: &str = "gnu.";
pub const EXT4_XATTR_INDEX_HURD: u8 = 10;
pub const EXT4_XATTR_HURD_HANDLER_SYMBOL: &str = "ext4_xattr_hurd_handler";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4XattrHurdHandler {
    pub prefix: &'static str,
    pub index: u8,
    pub list_function: &'static str,
    pub get_function: &'static str,
    pub set_function: &'static str,
    pub gate: &'static str,
}

pub const EXT4_XATTR_HURD_HANDLER: Ext4XattrHurdHandler = Ext4XattrHurdHandler {
    prefix: XATTR_HURD_PREFIX,
    index: EXT4_XATTR_INDEX_HURD,
    list_function: "ext4_xattr_hurd_list",
    get_function: "ext4_xattr_hurd_get",
    set_function: "ext4_xattr_hurd_set",
    gate: "XATTR_USER",
};

pub const fn ext4_xattr_hurd_list(xattr_user_enabled: bool) -> bool {
    xattr_user_enabled
}

pub fn ext4_xattr_hurd_get_index(xattr_user_enabled: bool) -> Result<u8, i32> {
    if !xattr_user_enabled {
        return Err(-EOPNOTSUPP);
    }
    Ok(EXT4_XATTR_INDEX_HURD)
}

pub fn ext4_xattr_hurd_set_index(xattr_user_enabled: bool) -> Result<u8, i32> {
    ext4_xattr_hurd_get_index(xattr_user_enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext4_xattr_hurd_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ext4/xattr_hurd.c"
        ));
        assert!(source.contains("#include <linux/init.h>"));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include \"ext4.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("ext4_xattr_hurd_list"));
        assert!(source.contains("return test_opt(dentry->d_sb, XATTR_USER);"));
        assert!(source.contains("ext4_xattr_hurd_get"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert!(source.contains("ext4_xattr_get(inode, EXT4_XATTR_INDEX_HURD"));
        assert!(source.contains("ext4_xattr_hurd_set"));
        assert!(source.contains("ext4_xattr_set(inode, EXT4_XATTR_INDEX_HURD"));
        assert!(source.contains(EXT4_XATTR_HURD_HANDLER_SYMBOL));
        assert!(source.contains(".prefix\t= XATTR_HURD_PREFIX"));

        assert_eq!(EXT4_XATTR_HURD_HANDLER.prefix, "gnu.");
        assert_eq!(EXT4_XATTR_HURD_HANDLER.index, 10);
        assert!(ext4_xattr_hurd_list(true));
        assert!(!ext4_xattr_hurd_list(false));
        assert_eq!(ext4_xattr_hurd_get_index(false), Err(-EOPNOTSUPP));
        assert_eq!(ext4_xattr_hurd_set_index(true), Ok(EXT4_XATTR_INDEX_HURD));
    }
}
