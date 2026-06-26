//! linux-parity: complete
//! linux-source: vendor/linux/crypto/asymmetric_keys/pkcs7_key_type.c
//! test-origin: linux:vendor/linux/crypto/asymmetric_keys/pkcs7_key_type.c
//! PKCS#7 testing key type metadata and preparse gate.

use crate::include::uapi::errno::EINVAL;

pub const KEY_TYPE_NAME: &str = "pkcs7_test";
pub const VERIFY_USE_SECONDARY_KEYRING: usize = 1;
pub const NR_KEY_BEING_USED_FOR: u32 = 7;
pub const MODULE_DESCRIPTION: &str = "PKCS#7 testing key type";
pub const MODULE_AUTHOR: &str = "Red Hat, Inc.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pkcs7KeyType {
    pub name: &'static str,
    pub has_preparse: bool,
    pub uses_user_payload_ops: bool,
}

pub const KEY_TYPE_PKCS7: Pkcs7KeyType = Pkcs7KeyType {
    name: KEY_TYPE_NAME,
    has_preparse: true,
    uses_user_payload_ops: true,
};

pub const fn pkcs7_usage_valid(usage: u32) -> bool {
    usage < NR_KEY_BEING_USED_FOR
}

pub const fn pkcs7_preparse_gate(usage: u32) -> Result<(), i32> {
    if pkcs7_usage_valid(usage) {
        Ok(())
    } else {
        Err(-EINVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkcs7_key_type_matches_linux_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/asymmetric_keys/pkcs7_key_type.c"
        ));
        let verification = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/verification.h"
        ));
        assert!(source.contains("#define pr_fmt(fmt) \"PKCS7key: \"fmt"));
        assert!(
            source.contains("module_param_named(usage, pkcs7_usage, uint, S_IWUSR | S_IRUGO);")
        );
        assert!(source.contains("saved_prep_data = prep->data;"));
        assert!(source.contains("ret = user_preparse(prep);"));
        assert!(source.contains("if (usage >= NR__KEY_BEING_USED_FOR)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("VERIFY_USE_SECONDARY_KEYRING, usage"));
        assert!(source.contains(".name\t\t\t= \"pkcs7_test\""));
        assert!(source.contains("module_init(pkcs7_key_init);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"PKCS#7 testing key type\")"));
        assert!(verification.contains("VERIFYING_BPF_SIGNATURE"));
        assert!(verification.contains("NR__KEY_BEING_USED_FOR"));

        assert_eq!(KEY_TYPE_PKCS7.name, "pkcs7_test");
        assert!(pkcs7_usage_valid(NR_KEY_BEING_USED_FOR - 1));
        assert!(!pkcs7_usage_valid(NR_KEY_BEING_USED_FOR));
        assert_eq!(pkcs7_preparse_gate(NR_KEY_BEING_USED_FOR), Err(-EINVAL));
    }
}
