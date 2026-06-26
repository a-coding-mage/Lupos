//! linux-parity: partial
//! linux-source: vendor/linux/certs/system_keyring.c
//! test-origin: linux:vendor/linux/certs/system_keyring.c
//! Built-in, secondary, machine, and platform trusted-keyring routing.

use crate::include::uapi::errno::{EBADMSG, ENOKEY};

pub const BUILTIN_TRUSTED_KEYS: &str = ".builtin_trusted_keys";
pub const SECONDARY_TRUSTED_KEYS: &str = ".secondary_trusted_keys";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustedKeyring {
    Builtin,
    Secondary,
    Machine,
    Platform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustedKeysRequest {
    BuiltinOnly,
    SecondaryIfEnabled,
    Platform,
    Explicit(TrustedKeyring),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pkcs7VerifyStep {
    SupplyDetachedData,
    VerifySignature,
    CheckRevocation,
    ValidateTrust,
    ViewContent,
}

pub const PKCS7_VERIFY_STEPS: &[Pkcs7VerifyStep] = &[
    Pkcs7VerifyStep::SupplyDetachedData,
    Pkcs7VerifyStep::VerifySignature,
    Pkcs7VerifyStep::CheckRevocation,
    Pkcs7VerifyStep::ValidateTrust,
    Pkcs7VerifyStep::ViewContent,
];

pub const fn trusted_keyring_for_request(
    request: TrustedKeysRequest,
    secondary_enabled: bool,
    platform_enabled: bool,
) -> Option<TrustedKeyring> {
    match request {
        TrustedKeysRequest::BuiltinOnly => Some(TrustedKeyring::Builtin),
        TrustedKeysRequest::SecondaryIfEnabled if secondary_enabled => {
            Some(TrustedKeyring::Secondary)
        }
        TrustedKeysRequest::SecondaryIfEnabled => Some(TrustedKeyring::Builtin),
        TrustedKeysRequest::Platform if platform_enabled => Some(TrustedKeyring::Platform),
        TrustedKeysRequest::Platform => None,
        TrustedKeysRequest::Explicit(keyring) => Some(keyring),
    }
}

pub const fn detached_data_errno(has_embedded_data: bool) -> Result<(), i32> {
    if has_embedded_data {
        Err(EBADMSG)
    } else {
        Ok(())
    }
}

pub const fn revocation_result_errno(trust_result: i32) -> Result<(), i32> {
    if trust_result == 0 {
        Err(ENOKEY)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_keyring_flow_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/certs/system_keyring.c"
        ));
        assert!(source.contains("restrict_link_by_builtin_trusted"));
        assert!(source.contains("restrict_link_by_digsig_builtin"));
        assert!(source.contains(".builtin_trusted_keys"));
        assert!(source.contains(".secondary_trusted_keys"));
        assert!(source.contains("device_initcall(system_trusted_keyring_init);"));
        assert!(source.contains("late_initcall(load_system_certificate_list);"));
        assert!(source.contains("verify_pkcs7_message_sig"));
        assert!(source.contains("pkcs7_supply_detached_data"));
        assert!(source.contains("ret = pkcs7_verify(pkcs7, usage);"));
        assert!(source.contains("ret = is_key_on_revocation_list(pkcs7);"));
        assert!(source.contains("ret = pkcs7_validate_trust(pkcs7, trusted_keys);"));
        assert!(source.contains("pkcs7_get_content_data"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(verify_pkcs7_signature);"));

        assert_eq!(BUILTIN_TRUSTED_KEYS, ".builtin_trusted_keys");
        assert_eq!(
            trusted_keyring_for_request(TrustedKeysRequest::BuiltinOnly, false, false),
            Some(TrustedKeyring::Builtin)
        );
        assert_eq!(
            trusted_keyring_for_request(TrustedKeysRequest::SecondaryIfEnabled, false, false),
            Some(TrustedKeyring::Builtin)
        );
        assert_eq!(
            trusted_keyring_for_request(TrustedKeysRequest::SecondaryIfEnabled, true, false),
            Some(TrustedKeyring::Secondary)
        );
        assert_eq!(
            trusted_keyring_for_request(TrustedKeysRequest::Platform, false, false),
            None
        );
        assert_eq!(detached_data_errno(true), Err(EBADMSG));
        assert_eq!(revocation_result_errno(0), Err(ENOKEY));
        assert_eq!(PKCS7_VERIFY_STEPS.len(), 5);
    }
}
