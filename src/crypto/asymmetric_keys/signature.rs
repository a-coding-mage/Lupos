//! linux-parity: complete
//! linux-source: vendor/linux/crypto/asymmetric_keys/signature.c
//! test-origin: linux:vendor/linux/crypto/asymmetric_keys/signature.c
//! Asymmetric-key signature query and verify dispatch gates.

use crate::include::uapi::errno::EINVAL;

pub const ENOTSUPP: i32 = 524;
pub const PUBLIC_KEY_SIGNATURE_AUTH_IDS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyType {
    Asymmetric,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsymmetricSubtype {
    pub has_query: bool,
    pub has_verify_signature: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsymmetricKeyState {
    pub key_type: KeyType,
    pub subtype: Option<AsymmetricSubtype>,
    pub has_payload: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicKeySignature {
    pub auth_ids_present: [bool; PUBLIC_KEY_SIGNATURE_AUTH_IDS],
    pub has_s: bool,
    pub m_free: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicKeySignatureFreePlan {
    pub auth_ids_freed: usize,
    pub s_freed: bool,
    pub m_freed: bool,
    pub sig_freed: bool,
}

pub fn public_key_signature_free(sig: Option<PublicKeySignature>) -> PublicKeySignatureFreePlan {
    let Some(sig) = sig else {
        return PublicKeySignatureFreePlan {
            auth_ids_freed: 0,
            s_freed: false,
            m_freed: false,
            sig_freed: false,
        };
    };

    PublicKeySignatureFreePlan {
        auth_ids_freed: sig
            .auth_ids_present
            .iter()
            .filter(|present| **present)
            .count(),
        s_freed: sig.has_s,
        m_freed: sig.m_free,
        sig_freed: true,
    }
}

pub const fn query_asymmetric_key_gate(key: AsymmetricKeyState) -> Result<(), i32> {
    if !matches!(key.key_type, KeyType::Asymmetric) {
        return Err(-EINVAL);
    }
    let Some(subtype) = key.subtype else {
        return Err(-EINVAL);
    };
    if !key.has_payload {
        return Err(-EINVAL);
    }
    if !subtype.has_query {
        return Err(-ENOTSUPP);
    }
    Ok(())
}

pub fn query_asymmetric_key<F>(key: AsymmetricKeyState, subtype_query: F) -> i32
where
    F: FnOnce() -> i32,
{
    match query_asymmetric_key_gate(key) {
        Ok(()) => subtype_query(),
        Err(err) => err,
    }
}

pub const fn verify_signature_gate(key: AsymmetricKeyState) -> Result<(), i32> {
    if !matches!(key.key_type, KeyType::Asymmetric) {
        return Err(-EINVAL);
    }
    let Some(subtype) = key.subtype else {
        return Err(-EINVAL);
    };
    if !key.has_payload {
        return Err(-EINVAL);
    }
    if !subtype.has_verify_signature {
        return Err(-ENOTSUPP);
    }
    Ok(())
}

pub fn verify_signature<F>(
    key: AsymmetricKeyState,
    sig: &PublicKeySignature,
    subtype_verify_signature: F,
) -> i32
where
    F: FnOnce(&PublicKeySignature) -> i32,
{
    match verify_signature_gate(key) {
        Ok(()) => subtype_verify_signature(sig),
        Err(err) => err,
    }
}

pub const fn public_key_signature_free_slots(auth_ids: usize, has_s: bool, m_free: bool) -> usize {
    auth_ids + has_s as usize + m_free as usize + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_dispatch_matches_linux_gates() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/asymmetric_keys/signature.c"
        ));
        assert!(source.contains("#define pr_fmt(fmt) \"SIG: \"fmt"));
        assert!(source.contains("for (i = 0; i < ARRAY_SIZE(sig->auth_ids); i++)"));
        assert!(source.contains("kfree(sig->auth_ids[i]);"));
        assert!(source.contains("if (sig->m_free)"));
        assert!(source.contains("kfree(sig);"));
        assert!(source.contains("if (key->type != &key_type_asymmetric)"));
        assert!(source.contains("if (!subtype ||"));
        assert!(source.contains("!key->payload.data[0])"));
        assert!(source.contains("if (!subtype->query)"));
        assert!(source.contains("return -ENOTSUPP;"));
        assert!(source.contains("if (!subtype->verify_signature)"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(query_asymmetric_key);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(verify_signature);"));

        let usable = AsymmetricKeyState {
            key_type: KeyType::Asymmetric,
            subtype: Some(AsymmetricSubtype {
                has_query: true,
                has_verify_signature: true,
            }),
            has_payload: true,
        };
        assert_eq!(query_asymmetric_key_gate(usable), Ok(()));
        assert_eq!(verify_signature_gate(usable), Ok(()));
        assert_eq!(query_asymmetric_key(usable, || 17), 17);

        let sig = PublicKeySignature {
            auth_ids_present: [true, false, true],
            has_s: true,
            m_free: true,
        };
        assert_eq!(verify_signature(usable, &sig, |_| 23), 23);

        assert_eq!(
            query_asymmetric_key_gate(AsymmetricKeyState {
                key_type: KeyType::Other,
                ..usable
            }),
            Err(-EINVAL)
        );
        assert_eq!(
            verify_signature_gate(AsymmetricKeyState {
                subtype: Some(AsymmetricSubtype {
                    has_query: true,
                    has_verify_signature: false,
                }),
                ..usable
            }),
            Err(-ENOTSUPP)
        );
        assert_eq!(
            query_asymmetric_key(
                AsymmetricKeyState {
                    key_type: KeyType::Other,
                    ..usable
                },
                || 99
            ),
            -EINVAL
        );
        assert_eq!(public_key_signature_free_slots(2, true, true), 5);
        assert_eq!(
            public_key_signature_free(Some(sig)),
            PublicKeySignatureFreePlan {
                auth_ids_freed: 2,
                s_freed: true,
                m_freed: true,
                sig_freed: true,
            }
        );
        assert_eq!(
            public_key_signature_free(None),
            PublicKeySignatureFreePlan {
                auth_ids_freed: 0,
                s_freed: false,
                m_freed: false,
                sig_freed: false,
            }
        );
    }
}
