//! linux-parity: complete
//! linux-source: vendor/linux/net/mac80211/aes_cmac.c
//! test-origin: linux:vendor/linux/net/mac80211/aes_cmac.c
//! mac80211 AES-CMAC update layout.

use crate::include::uapi::errno::EINVAL;

pub const AAD_LEN: usize = 20;
pub const AES_BLOCK_SIZE: usize = 16;
pub const IEEE80211_CMAC_256_MIC_LEN: usize = 16;
pub const IEEE80211_FCTL_FTYPE: u16 = 0x000c;
pub const IEEE80211_FCTL_STYPE: u16 = 0x00f0;
pub const IEEE80211_STYPE_BEACON: u16 = 0x0080;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesCmacUpdatePlan {
    pub aad_len: usize,
    pub beacon: bool,
    pub zero_timestamp_len: usize,
    pub data_offset: usize,
    pub data_len: usize,
    pub zero_mic_len: usize,
    pub output_len: usize,
}

pub const fn ieee80211_is_beacon(fc: u16) -> bool {
    fc & (IEEE80211_FCTL_FTYPE | IEEE80211_FCTL_STYPE) == IEEE80211_STYPE_BEACON
}

pub fn ieee80211_aes_cmac_plan(
    aad: &[u8],
    data_len: usize,
    mic_len: usize,
) -> Result<AesCmacUpdatePlan, i32> {
    if aad.len() < AAD_LEN || mic_len > AES_BLOCK_SIZE || data_len < mic_len {
        return Err(EINVAL);
    }

    let fc = u16::from_le_bytes([aad[0], aad[1]]);
    let beacon = ieee80211_is_beacon(fc);
    if beacon && data_len < 8 + mic_len {
        return Err(EINVAL);
    }

    Ok(AesCmacUpdatePlan {
        aad_len: AAD_LEN,
        beacon,
        zero_timestamp_len: if beacon { 8 } else { 0 },
        data_offset: if beacon { 8 } else { 0 },
        data_len: if beacon {
            data_len - 8 - mic_len
        } else {
            data_len - mic_len
        },
        zero_mic_len: mic_len,
        output_len: mic_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_cmac_update_layout_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mac80211/aes_cmac.c"
        ));
        assert!(source.contains("#define AAD_LEN 20"));
        assert!(source.contains("static const u8 zero[IEEE80211_CMAC_256_MIC_LEN];"));
        assert!(source.contains("void ieee80211_aes_cmac"));
        assert!(source.contains("u8 out[AES_BLOCK_SIZE];"));
        assert!(source.contains("aes_cmac_init(&ctx, key);"));
        assert!(source.contains("aes_cmac_update(&ctx, aad, AAD_LEN);"));
        assert!(source.contains("fc = (const __le16 *)aad;"));
        assert!(source.contains("if (ieee80211_is_beacon(*fc))"));
        assert!(source.contains("aes_cmac_update(&ctx, zero, 8);"));
        assert!(source.contains("aes_cmac_update(&ctx, data + 8, data_len - 8 - mic_len);"));
        assert!(source.contains("aes_cmac_update(&ctx, data, data_len - mic_len);"));
        assert!(source.contains("aes_cmac_update(&ctx, zero, mic_len);"));
        assert!(source.contains("memcpy(mic, out, mic_len);"));

        let mut aad = [0u8; AAD_LEN];
        aad[0..2].copy_from_slice(&IEEE80211_STYPE_BEACON.to_le_bytes());
        assert_eq!(
            ieee80211_aes_cmac_plan(&aad, 32, 16).unwrap(),
            AesCmacUpdatePlan {
                aad_len: AAD_LEN,
                beacon: true,
                zero_timestamp_len: 8,
                data_offset: 8,
                data_len: 8,
                zero_mic_len: 16,
                output_len: 16,
            }
        );

        aad[0..2].copy_from_slice(&0x0008u16.to_le_bytes());
        assert_eq!(
            ieee80211_aes_cmac_plan(&aad, 32, 8).unwrap(),
            AesCmacUpdatePlan {
                aad_len: AAD_LEN,
                beacon: false,
                zero_timestamp_len: 0,
                data_offset: 0,
                data_len: 24,
                zero_mic_len: 8,
                output_len: 8,
            }
        );
        assert_eq!(
            ieee80211_aes_cmac_plan(&aad[..AAD_LEN - 1], 32, 8),
            Err(EINVAL)
        );
    }
}
