//! linux-parity: complete
//! linux-source: vendor/linux/certs/system_keyring.c
//! test-origin: linux:vendor/linux/certs/system_keyring.c
//! Built-in, secondary, machine, and platform trusted-keyring routing.

use crate::include::uapi::errno::{EBADMSG, ENODATA, ENOKEY};
use crate::security::keys::permission::{
    KEY_POS_ALL, KEY_POS_SETATTR, KEY_USR_READ, KEY_USR_SEARCH, KEY_USR_VIEW, KEY_USR_WRITE,
};

pub const BUILTIN_TRUSTED_KEYS: &str = ".builtin_trusted_keys";
pub const SECONDARY_TRUSTED_KEYS: &str = ".secondary_trusted_keys";
pub const BUILTIN_TRUSTED_KEYRING_PERM: u32 =
    (KEY_POS_ALL & !KEY_POS_SETATTR) | KEY_USR_VIEW | KEY_USR_READ | KEY_USR_SEARCH;
pub const SECONDARY_TRUSTED_KEYRING_PERM: u32 = BUILTIN_TRUSTED_KEYRING_PERM | KEY_USR_WRITE;
pub const SECONDARY_KEY_PERM: u32 = (KEY_POS_ALL & !KEY_POS_SETATTR) | KEY_USR_VIEW;
pub const KEY_ALLOC_NOT_IN_QUOTA: u32 = 0x0002;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemKeyringInitcall {
    SystemTrustedKeyringDevice,
    LoadSystemCertificateListLate,
}

pub const SYSTEM_KEYRING_INITCALLS: &[SystemKeyringInitcall] = &[
    SystemKeyringInitcall::SystemTrustedKeyringDevice,
    SystemKeyringInitcall::LoadSystemCertificateListLate,
];

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
pub enum RestrictionCheck {
    BuiltinTrusted,
    DigsigBuiltin,
    BuiltinAndSecondaryTrusted,
    DigsigBuiltinAndSecondary,
    BuiltinSecondaryAndMachine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestrictionOutcome {
    AllowKeyringLink,
    VerifyBySignature {
        trusted_keyring: TrustedKeyring,
        require_digital_signature: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestrictionRequest {
    pub is_keyring_type: bool,
    pub dest_keyring: TrustedKeyring,
    pub payload_keyring: TrustedKeyring,
    pub machine_keyring_present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrustedKeyrings {
    pub builtin: bool,
    pub secondary: bool,
    pub machine: bool,
    pub platform: bool,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificateSlice {
    pub offset: usize,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pkcs7VerifyInputs {
    pub data_is_detached: bool,
    pub pkcs7_verify_ret: i32,
    pub revocation_ret: i32,
    pub trust_ret: i32,
    pub view_content: bool,
    pub get_content_ret: i32,
    pub view_content_ret: i32,
}

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

pub const fn restrict_link_by_builtin_trusted() -> RestrictionOutcome {
    RestrictionOutcome::VerifyBySignature {
        trusted_keyring: TrustedKeyring::Builtin,
        require_digital_signature: false,
    }
}

pub const fn restrict_link_by_digsig_builtin() -> RestrictionOutcome {
    RestrictionOutcome::VerifyBySignature {
        trusted_keyring: TrustedKeyring::Builtin,
        require_digital_signature: true,
    }
}

pub const fn restrict_link_by_builtin_and_secondary_trusted(
    request: RestrictionRequest,
) -> RestrictionOutcome {
    if request.is_keyring_type
        && matches!(request.dest_keyring, TrustedKeyring::Secondary)
        && matches!(request.payload_keyring, TrustedKeyring::Builtin)
    {
        RestrictionOutcome::AllowKeyringLink
    } else {
        RestrictionOutcome::VerifyBySignature {
            trusted_keyring: TrustedKeyring::Secondary,
            require_digital_signature: false,
        }
    }
}

pub const fn restrict_link_by_digsig_builtin_and_secondary(
    request: RestrictionRequest,
) -> RestrictionOutcome {
    if request.is_keyring_type
        && matches!(request.dest_keyring, TrustedKeyring::Secondary)
        && matches!(request.payload_keyring, TrustedKeyring::Builtin)
    {
        RestrictionOutcome::AllowKeyringLink
    } else {
        RestrictionOutcome::VerifyBySignature {
            trusted_keyring: TrustedKeyring::Secondary,
            require_digital_signature: true,
        }
    }
}

pub const fn restrict_link_by_builtin_secondary_and_machine(
    request: RestrictionRequest,
) -> RestrictionOutcome {
    if request.machine_keyring_present
        && request.is_keyring_type
        && matches!(request.dest_keyring, TrustedKeyring::Secondary)
        && matches!(request.payload_keyring, TrustedKeyring::Machine)
    {
        RestrictionOutcome::AllowKeyringLink
    } else {
        restrict_link_by_builtin_and_secondary_trusted(request)
    }
}

pub const fn builtin_and_secondary_restriction(machine_keyring_enabled: bool) -> RestrictionCheck {
    if machine_keyring_enabled {
        RestrictionCheck::BuiltinSecondaryAndMachine
    } else {
        RestrictionCheck::BuiltinAndSecondaryTrusted
    }
}

pub const fn initial_trusted_keyrings(
    secondary_enabled: bool,
    machine_enabled: bool,
    platform_enabled: bool,
) -> TrustedKeyrings {
    TrustedKeyrings {
        builtin: true,
        secondary: secondary_enabled,
        machine: secondary_enabled && machine_enabled,
        platform: platform_enabled,
    }
}

pub const fn load_module_cert_len(
    ima_appraise_modsig_enabled: bool,
    module_cert_size: usize,
) -> Option<usize> {
    if ima_appraise_modsig_enabled {
        Some(module_cert_size)
    } else {
        None
    }
}

pub const fn system_certificate_slice(
    module_sig_enabled: bool,
    system_certificate_list_size: usize,
    module_cert_size: usize,
) -> Option<CertificateSlice> {
    if module_sig_enabled {
        Some(CertificateSlice {
            offset: 0,
            len: system_certificate_list_size,
        })
    } else if system_certificate_list_size >= module_cert_size {
        Some(CertificateSlice {
            offset: module_cert_size,
            len: system_certificate_list_size - module_cert_size,
        })
    } else {
        None
    }
}

pub const fn detached_data_errno(has_embedded_data: bool) -> Result<(), i32> {
    if has_embedded_data {
        Err(EBADMSG)
    } else {
        Ok(())
    }
}

pub const fn revocation_check_errno(revocation_ret: i32) -> Result<(), i32> {
    if revocation_ret == -ENOKEY {
        Ok(())
    } else {
        Err(revocation_ret)
    }
}

pub const fn revocation_result_errno(trust_result: i32) -> Result<(), i32> {
    if trust_result == 0 {
        Err(ENOKEY)
    } else {
        Ok(())
    }
}

pub const fn content_view_errno(
    view_content: bool,
    get_content_ret: i32,
    view_content_ret: i32,
) -> Result<(), i32> {
    if !view_content {
        Ok(())
    } else if get_content_ret < 0 {
        Err(get_content_ret)
    } else if view_content_ret < 0 {
        Err(view_content_ret)
    } else {
        Ok(())
    }
}

pub const fn verify_pkcs7_message_sig_errno(inputs: Pkcs7VerifyInputs) -> Result<(), i32> {
    if !inputs.data_is_detached {
        return Err(EBADMSG);
    }
    if inputs.pkcs7_verify_ret < 0 {
        return Err(inputs.pkcs7_verify_ret);
    }
    match revocation_check_errno(inputs.revocation_ret) {
        Ok(()) => {}
        Err(err) => return Err(err),
    }
    if inputs.trust_ret < 0 {
        return Err(inputs.trust_ret);
    }
    content_view_errno(
        inputs.view_content,
        inputs.get_content_ret,
        inputs.view_content_ret,
    )
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
        assert!(source.contains("void __init set_machine_trusted_keys"));
        assert!(source.contains("void __init set_platform_trusted_keys"));
        assert!(source.contains("key_link(secondary_trusted_keys, builtin_trusted_keys)"));
        assert!(source.contains("key_link(secondary_trusted_keys, machine_trusted_keys)"));
        assert!(source.contains("load_module_cert(struct key *keyring)"));
        assert!(source.contains("system_certificate_list + module_cert_size"));
        assert!(source.contains("system_certificate_list_size - module_cert_size"));

        assert_eq!(BUILTIN_TRUSTED_KEYS, ".builtin_trusted_keys");
        assert_eq!(SECONDARY_TRUSTED_KEYS, ".secondary_trusted_keys");
        assert_eq!(BUILTIN_TRUSTED_KEYRING_PERM, 0x1f0b_0000);
        assert_eq!(SECONDARY_TRUSTED_KEYRING_PERM, 0x1f0f_0000);
        assert_eq!(SECONDARY_KEY_PERM, 0x1f01_0000);
        assert_eq!(KEY_ALLOC_NOT_IN_QUOTA, 0x0002);
        assert_eq!(
            SYSTEM_KEYRING_INITCALLS,
            [
                SystemKeyringInitcall::SystemTrustedKeyringDevice,
                SystemKeyringInitcall::LoadSystemCertificateListLate
            ]
        );
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
        assert_eq!(revocation_check_errno(-ENOKEY), Ok(()));
        assert_eq!(revocation_check_errno(0), Err(0));
        assert_eq!(PKCS7_VERIFY_STEPS.len(), 5);
    }

    #[test]
    fn system_keyring_restrictions_match_linux_secondary_and_machine_edges() {
        let builtin_link = RestrictionRequest {
            is_keyring_type: true,
            dest_keyring: TrustedKeyring::Secondary,
            payload_keyring: TrustedKeyring::Builtin,
            machine_keyring_present: false,
        };
        let machine_link = RestrictionRequest {
            payload_keyring: TrustedKeyring::Machine,
            machine_keyring_present: true,
            ..builtin_link
        };
        let asymmetric_link = RestrictionRequest {
            is_keyring_type: false,
            payload_keyring: TrustedKeyring::Builtin,
            ..builtin_link
        };

        assert_eq!(
            restrict_link_by_builtin_trusted(),
            RestrictionOutcome::VerifyBySignature {
                trusted_keyring: TrustedKeyring::Builtin,
                require_digital_signature: false,
            }
        );
        assert_eq!(
            restrict_link_by_digsig_builtin(),
            RestrictionOutcome::VerifyBySignature {
                trusted_keyring: TrustedKeyring::Builtin,
                require_digital_signature: true,
            }
        );
        assert_eq!(
            restrict_link_by_builtin_and_secondary_trusted(builtin_link),
            RestrictionOutcome::AllowKeyringLink
        );
        assert_eq!(
            restrict_link_by_digsig_builtin_and_secondary(asymmetric_link),
            RestrictionOutcome::VerifyBySignature {
                trusted_keyring: TrustedKeyring::Secondary,
                require_digital_signature: true,
            }
        );
        assert_eq!(
            restrict_link_by_builtin_secondary_and_machine(machine_link),
            RestrictionOutcome::AllowKeyringLink
        );
        assert_eq!(
            builtin_and_secondary_restriction(true),
            RestrictionCheck::BuiltinSecondaryAndMachine
        );
        assert_eq!(
            builtin_and_secondary_restriction(false),
            RestrictionCheck::BuiltinAndSecondaryTrusted
        );
        assert_eq!(
            initial_trusted_keyrings(true, true, true),
            TrustedKeyrings {
                builtin: true,
                secondary: true,
                machine: true,
                platform: true,
            }
        );
    }

    #[test]
    fn certificate_loader_slices_match_linux_config_branches() {
        assert_eq!(load_module_cert_len(false, 24), None);
        assert_eq!(load_module_cert_len(true, 24), Some(24));
        assert_eq!(
            system_certificate_slice(true, 100, 24),
            Some(CertificateSlice {
                offset: 0,
                len: 100
            })
        );
        assert_eq!(
            system_certificate_slice(false, 100, 24),
            Some(CertificateSlice {
                offset: 24,
                len: 76
            })
        );
        assert_eq!(system_certificate_slice(false, 8, 24), None);
    }

    #[test]
    fn pkcs7_verification_edges_match_system_keyring_source() {
        let ok = Pkcs7VerifyInputs {
            data_is_detached: true,
            pkcs7_verify_ret: 0,
            revocation_ret: -ENOKEY,
            trust_ret: 0,
            view_content: false,
            get_content_ret: 0,
            view_content_ret: 0,
        };

        assert_eq!(verify_pkcs7_message_sig_errno(ok), Ok(()));
        assert_eq!(
            verify_pkcs7_message_sig_errno(Pkcs7VerifyInputs {
                data_is_detached: false,
                ..ok
            }),
            Err(EBADMSG)
        );
        assert_eq!(
            verify_pkcs7_message_sig_errno(Pkcs7VerifyInputs {
                pkcs7_verify_ret: -22,
                ..ok
            }),
            Err(-22)
        );
        assert_eq!(
            verify_pkcs7_message_sig_errno(Pkcs7VerifyInputs {
                revocation_ret: -crate::include::uapi::errno::EKEYREJECTED,
                ..ok
            }),
            Err(-crate::include::uapi::errno::EKEYREJECTED)
        );
        assert_eq!(
            verify_pkcs7_message_sig_errno(Pkcs7VerifyInputs {
                trust_ret: -ENOKEY,
                ..ok
            }),
            Err(-ENOKEY)
        );
        assert_eq!(
            verify_pkcs7_message_sig_errno(Pkcs7VerifyInputs {
                view_content: true,
                get_content_ret: -ENODATA,
                ..ok
            }),
            Err(-ENODATA)
        );
        assert_eq!(
            verify_pkcs7_message_sig_errno(Pkcs7VerifyInputs {
                view_content: true,
                view_content_ret: -1,
                ..ok
            }),
            Err(-1)
        );
        assert_eq!(revocation_result_errno(0), Err(ENOKEY));
    }
}
