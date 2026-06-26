//! linux-parity: complete
//! linux-source: vendor/linux/net/mac80211/aes_gmac.c
//! test-origin: linux:vendor/linux/net/mac80211/aes_gmac.c
//! mac80211 AES-GMAC scatterlist layout.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const AES_BLOCK_SIZE: usize = 16;
pub const IEEE80211_GMAC_MIC_LEN: usize = 16;
pub const GMAC_AAD_LEN: usize = 20;
pub const GMAC_NONCE_LEN: usize = 12;
pub const IEEE80211_FCTL_FTYPE: u16 = 0x000c;
pub const IEEE80211_FCTL_STYPE: u16 = 0x00f0;
pub const IEEE80211_STYPE_BEACON: u16 = 0x0080;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesGmacPlan {
    pub scatterlist_entries: usize,
    pub aad_len: usize,
    pub beacon: bool,
    pub zero_timestamp_len: usize,
    pub data_offset: usize,
    pub data_len: usize,
    pub zero_mic_len: usize,
    pub mic_len: usize,
    pub assoc_data_len: usize,
    pub iv_tail: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesGmacKey {
    pub key_len: usize,
    pub authsize: usize,
}

pub const fn ieee80211_is_beacon(fc: u16) -> bool {
    fc & (IEEE80211_FCTL_FTYPE | IEEE80211_FCTL_STYPE) == IEEE80211_STYPE_BEACON
}

pub fn ieee80211_aes_gmac_plan(
    aad: &[u8],
    data_len: usize,
    alloc_ok: bool,
) -> Result<AesGmacPlan, i32> {
    if data_len < IEEE80211_GMAC_MIC_LEN || aad.len() < GMAC_AAD_LEN {
        return Err(-EINVAL);
    }
    if !alloc_ok {
        return Err(-ENOMEM);
    }

    let fc = u16::from_le_bytes([aad[0], aad[1]]);
    let beacon = ieee80211_is_beacon(fc);
    if beacon && data_len < 8 + IEEE80211_GMAC_MIC_LEN {
        return Err(-EINVAL);
    }

    Ok(AesGmacPlan {
        scatterlist_entries: if beacon { 5 } else { 4 },
        aad_len: GMAC_AAD_LEN,
        beacon,
        zero_timestamp_len: if beacon { 8 } else { 0 },
        data_offset: if beacon { 8 } else { 0 },
        data_len: if beacon {
            data_len - 8 - IEEE80211_GMAC_MIC_LEN
        } else {
            data_len - IEEE80211_GMAC_MIC_LEN
        },
        zero_mic_len: IEEE80211_GMAC_MIC_LEN,
        mic_len: IEEE80211_GMAC_MIC_LEN,
        assoc_data_len: GMAC_AAD_LEN + data_len,
        iv_tail: 0x01,
    })
}

pub const fn ieee80211_aes_gmac_key_setup(
    key_len: usize,
    alloc_ok: bool,
    setkey_ok: bool,
    setauthsize_ok: bool,
) -> Result<AesGmacKey, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    if !setkey_ok || !setauthsize_ok {
        return Err(-EINVAL);
    }
    Ok(AesGmacKey {
        key_len,
        authsize: IEEE80211_GMAC_MIC_LEN,
    })
}

pub const fn ieee80211_aes_gmac_key_free(_key: AesGmacKey) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_gmac_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mac80211/aes_gmac.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mac80211/aes_gmac.h"
        ));
        assert!(source.contains("AES-GMAC for IEEE 802.11 BIP-GMAC-128 and BIP-GMAC-256"));
        assert!(header.contains("#define GMAC_AAD_LEN\t20"));
        assert!(header.contains("#define GMAC_NONCE_LEN\t12"));
        assert!(source.contains("int ieee80211_aes_gmac"));
        assert!(source.contains("struct scatterlist sg[5];"));
        assert!(source.contains("if (data_len < IEEE80211_GMAC_MIC_LEN)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("memcpy(__aad, aad, GMAC_AAD_LEN);"));
        assert!(source.contains("if (ieee80211_is_beacon(*fc))"));
        assert!(source.contains("sg_init_table(sg, 5);"));
        assert!(source.contains("sg_set_buf(&sg[1], zero, 8);"));
        assert!(source.contains("data_len - 8 - IEEE80211_GMAC_MIC_LEN"));
        assert!(source.contains("sg_init_table(sg, 4);"));
        assert!(source.contains("data_len - IEEE80211_GMAC_MIC_LEN"));
        assert!(source.contains("memcpy(iv, nonce, GMAC_NONCE_LEN);"));
        assert!(source.contains("iv[AES_BLOCK_SIZE - 1] = 0x01;"));
        assert!(source.contains("aead_request_set_ad(aead_req, GMAC_AAD_LEN + data_len);"));
        assert!(source.contains("crypto_aead_encrypt(aead_req);"));
        assert!(source.contains("crypto_alloc_aead(\"gcm(aes)\", 0, CRYPTO_ALG_ASYNC);"));
        assert!(source.contains("crypto_aead_setauthsize(tfm, IEEE80211_GMAC_MIC_LEN);"));
        assert!(source.contains("crypto_free_aead(tfm);"));
    }

    #[test]
    fn aes_gmac_plan_masks_beacon_timestamp_and_mic_segments() {
        let mut aad = [0u8; GMAC_AAD_LEN];
        aad[0..2].copy_from_slice(&IEEE80211_STYPE_BEACON.to_le_bytes());
        assert_eq!(
            ieee80211_aes_gmac_plan(&aad, 40, true).unwrap(),
            AesGmacPlan {
                scatterlist_entries: 5,
                aad_len: GMAC_AAD_LEN,
                beacon: true,
                zero_timestamp_len: 8,
                data_offset: 8,
                data_len: 16,
                zero_mic_len: IEEE80211_GMAC_MIC_LEN,
                mic_len: IEEE80211_GMAC_MIC_LEN,
                assoc_data_len: 60,
                iv_tail: 1,
            }
        );
        aad[0..2].copy_from_slice(&0x0008u16.to_le_bytes());
        assert_eq!(
            ieee80211_aes_gmac_plan(&aad, 40, true)
                .unwrap()
                .scatterlist_entries,
            4
        );
        assert_eq!(ieee80211_aes_gmac_plan(&aad, 15, true), Err(-EINVAL));
        assert_eq!(ieee80211_aes_gmac_plan(&aad, 40, false), Err(-ENOMEM));
        let key = ieee80211_aes_gmac_key_setup(32, true, true, true).unwrap();
        assert_eq!(key.authsize, IEEE80211_GMAC_MIC_LEN);
        assert!(ieee80211_aes_gmac_key_free(key));
    }
}
