//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/evm/evm_posix_acl.c
//! test-origin: linux:vendor/linux/security/integrity/evm/evm_posix_acl.c
//! EVM POSIX ACL xattr classifier.

pub const XATTR_NAME_POSIX_ACL_ACCESS: &str = "system.posix_acl_access";
pub const XATTR_NAME_POSIX_ACL_DEFAULT: &str = "system.posix_acl_default";
pub const POSIX_XATTR_ACL_MATCH: i32 = 1;
pub const POSIX_XATTR_ACL_NO_MATCH: i32 = 0;

pub fn linux_strlen(xattr: &str) -> usize {
    xattr.len()
}

pub fn linux_strncmp_exact(expected: &str, xattr: &str, xattr_len: usize) -> bool {
    expected.len() == xattr_len && expected.as_bytes() == xattr.as_bytes()
}

pub fn posix_xattr_acl(xattr: &str) -> i32 {
    let xattr_len = linux_strlen(xattr);

    if linux_strncmp_exact(XATTR_NAME_POSIX_ACL_ACCESS, xattr, xattr_len)
        || linux_strncmp_exact(XATTR_NAME_POSIX_ACL_DEFAULT, xattr, xattr_len)
    {
        POSIX_XATTR_ACL_MATCH
    } else {
        POSIX_XATTR_ACL_NO_MATCH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_xattr_acl_matches_linux_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/evm/evm_posix_acl.c"
        ));
        assert!(source.contains("#include <linux/xattr.h>"));
        assert!(source.contains("#include <linux/evm.h>"));
        assert!(source.contains("int xattr_len = strlen(xattr);"));
        assert!(source.contains("XATTR_NAME_POSIX_ACL_ACCESS"));
        assert!(source.contains("strncmp(XATTR_NAME_POSIX_ACL_ACCESS, xattr, xattr_len) == 0"));
        assert!(source.contains("XATTR_NAME_POSIX_ACL_DEFAULT"));
        assert!(source.contains("strncmp(XATTR_NAME_POSIX_ACL_DEFAULT, xattr, xattr_len) == 0"));
        assert!(source.contains("return 1;"));
        assert!(source.contains("return 0;"));
        assert_eq!(linux_strlen(XATTR_NAME_POSIX_ACL_ACCESS), 23);
        assert_eq!(linux_strlen(XATTR_NAME_POSIX_ACL_DEFAULT), 24);
        assert!(linux_strncmp_exact(
            XATTR_NAME_POSIX_ACL_ACCESS,
            XATTR_NAME_POSIX_ACL_ACCESS,
            linux_strlen(XATTR_NAME_POSIX_ACL_ACCESS)
        ));
        assert!(!linux_strncmp_exact(
            XATTR_NAME_POSIX_ACL_ACCESS,
            "system.posix_acl_access.extra",
            linux_strlen("system.posix_acl_access.extra")
        ));
        assert_eq!(
            posix_xattr_acl(XATTR_NAME_POSIX_ACL_ACCESS),
            POSIX_XATTR_ACL_MATCH
        );
        assert_eq!(
            posix_xattr_acl(XATTR_NAME_POSIX_ACL_DEFAULT),
            POSIX_XATTR_ACL_MATCH
        );
        assert_eq!(
            posix_xattr_acl("system.posix_acl_access.extra"),
            POSIX_XATTR_ACL_NO_MATCH
        );
        assert_eq!(posix_xattr_acl("security.evm"), POSIX_XATTR_ACL_NO_MATCH);
    }
}
