//! linux-parity: complete
//! linux-source: vendor/linux/crypto/krb5/rfc3962_aes.c
//! test-origin: linux:vendor/linux/crypto/krb5/rfc3962_aes.c
//! RFC3962 AES Kerberos enctype descriptors.

pub const KRB5_ENCTYPE_AES128_CTS_HMAC_SHA1_96: u32 = 0x0011;
pub const KRB5_ENCTYPE_AES256_CTS_HMAC_SHA1_96: u32 = 0x0012;
pub const KRB5_CKSUMTYPE_HMAC_SHA1_96_AES128: u32 = 0x000f;
pub const KRB5_CKSUMTYPE_HMAC_SHA1_96_AES256: u32 = 0x0010;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Krb5Enctype {
    pub etype: u32,
    pub ctype: u32,
    pub name: &'static str,
    pub encrypt_name: &'static str,
    pub cksum_name: &'static str,
    pub hash_name: &'static str,
    pub derivation_enc: &'static str,
    pub key_bytes: usize,
    pub key_len: usize,
    pub kc_len: usize,
    pub ke_len: usize,
    pub ki_len: usize,
    pub block_len: usize,
    pub conf_len: usize,
    pub cksum_len: usize,
    pub hash_len: usize,
    pub prf_len: usize,
    pub keyed_cksum: bool,
    pub random_to_key_is_identity: bool,
    pub profile: &'static str,
}

pub const KRB5_AES128_CTS_HMAC_SHA1_96: Krb5Enctype = Krb5Enctype {
    etype: KRB5_ENCTYPE_AES128_CTS_HMAC_SHA1_96,
    ctype: KRB5_CKSUMTYPE_HMAC_SHA1_96_AES128,
    name: "aes128-cts-hmac-sha1-96",
    encrypt_name: "krb5enc(hmac(sha1),cts(cbc(aes)))",
    cksum_name: "hmac(sha1)",
    hash_name: "sha1",
    derivation_enc: "cts(cbc(aes))",
    key_bytes: 16,
    key_len: 16,
    kc_len: 16,
    ke_len: 16,
    ki_len: 16,
    block_len: 16,
    conf_len: 16,
    cksum_len: 12,
    hash_len: 20,
    prf_len: 16,
    keyed_cksum: true,
    random_to_key_is_identity: true,
    profile: "rfc3961_simplified_profile",
};

pub const KRB5_AES256_CTS_HMAC_SHA1_96: Krb5Enctype = Krb5Enctype {
    etype: KRB5_ENCTYPE_AES256_CTS_HMAC_SHA1_96,
    ctype: KRB5_CKSUMTYPE_HMAC_SHA1_96_AES256,
    name: "aes256-cts-hmac-sha1-96",
    encrypt_name: "krb5enc(hmac(sha1),cts(cbc(aes)))",
    cksum_name: "hmac(sha1)",
    hash_name: "sha1",
    derivation_enc: "cts(cbc(aes))",
    key_bytes: 32,
    key_len: 32,
    kc_len: 32,
    ke_len: 32,
    ki_len: 32,
    block_len: 16,
    conf_len: 16,
    cksum_len: 12,
    hash_len: 20,
    prf_len: 16,
    keyed_cksum: true,
    random_to_key_is_identity: true,
    profile: "rfc3961_simplified_profile",
};

pub const RFC3962_AES_ENCTYPES: &[Krb5Enctype] =
    &[KRB5_AES128_CTS_HMAC_SHA1_96, KRB5_AES256_CTS_HMAC_SHA1_96];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3962_aes_enctypes_match_linux_descriptors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/krb5/rfc3962_aes.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/krb5.h"
        ));
        assert!(source.contains("const struct krb5_enctype krb5_aes128_cts_hmac_sha1_96"));
        assert!(source.contains(".name\t\t= \"aes128-cts-hmac-sha1-96\""));
        assert!(source.contains(".encrypt_name\t= \"krb5enc(hmac(sha1),cts(cbc(aes)))\""));
        assert!(source.contains(".cksum_len\t= 12"));
        assert!(source.contains(".hash_len\t= 20"));
        assert!(source.contains(".random_to_key\t= NULL, /* Identity */"));
        assert!(source.contains("const struct krb5_enctype krb5_aes256_cts_hmac_sha1_96"));
        assert!(source.contains(".key_bytes\t= 32"));
        assert!(header.contains("#define KRB5_ENCTYPE_AES128_CTS_HMAC_SHA1_96\t0x0011"));
        assert!(header.contains("#define KRB5_CKSUMTYPE_HMAC_SHA1_96_AES256\t0x0010"));

        assert_eq!(RFC3962_AES_ENCTYPES.len(), 2);
        assert_eq!(KRB5_AES128_CTS_HMAC_SHA1_96.key_len, 16);
        assert_eq!(KRB5_AES256_CTS_HMAC_SHA1_96.key_len, 32);
        assert_eq!(KRB5_AES256_CTS_HMAC_SHA1_96.block_len, 16);
        assert!(KRB5_AES128_CTS_HMAC_SHA1_96.keyed_cksum);
        assert!(KRB5_AES256_CTS_HMAC_SHA1_96.random_to_key_is_identity);
    }
}
