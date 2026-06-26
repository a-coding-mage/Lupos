//! linux-parity: complete
//! linux-source: vendor/linux/crypto/ecdh_helper.c
//! test-origin: linux:vendor/linux/crypto/ecdh_helper.c
//! ECDH private-key packet encoder and decoder for KPP.

extern crate alloc;

use crate::include::uapi::errno::EINVAL;

pub const CRYPTO_KPP_SECRET_TYPE_ECDH: u16 = 2;
pub const KPP_SECRET_SIZE: usize = 4;
pub const ECDH_KPP_SECRET_MIN_SIZE: usize = KPP_SECRET_SIZE + core::mem::size_of::<u16>();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EcdhParams<'a> {
    pub key: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedEcdhKey<'a> {
    pub key_size: u16,
    pub key: &'a [u8],
}

pub fn crypto_ecdh_key_len(params: EcdhParams<'_>) -> usize {
    ECDH_KPP_SECRET_MIN_SIZE + params.key.len()
}

pub fn crypto_ecdh_encode_key(buf: &mut [u8], params: EcdhParams<'_>) -> Result<usize, i32> {
    let len = crypto_ecdh_key_len(params);
    if buf.len() != len || params.key.len() > u16::MAX as usize {
        return Err(-EINVAL);
    }

    buf[0..2].copy_from_slice(&CRYPTO_KPP_SECRET_TYPE_ECDH.to_le_bytes());
    buf[2..4].copy_from_slice(&(len as u16).to_le_bytes());
    buf[4..6].copy_from_slice(&(params.key.len() as u16).to_le_bytes());
    buf[6..len].copy_from_slice(params.key);
    Ok(len)
}

pub fn crypto_ecdh_decode_key(buf: &[u8]) -> Result<DecodedEcdhKey<'_>, i32> {
    if buf.len() < ECDH_KPP_SECRET_MIN_SIZE {
        return Err(-EINVAL);
    }

    let secret_type = u16::from_le_bytes([buf[0], buf[1]]);
    let secret_len = u16::from_le_bytes([buf[2], buf[3]]) as usize;
    if secret_type != CRYPTO_KPP_SECRET_TYPE_ECDH || buf.len() < secret_len {
        return Err(-EINVAL);
    }

    let key_size = u16::from_le_bytes([buf[4], buf[5]]);
    if secret_len != ECDH_KPP_SECRET_MIN_SIZE + key_size as usize {
        return Err(-EINVAL);
    }
    Ok(DecodedEcdhKey {
        key_size,
        key: &buf[6..secret_len],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn ecdh_encode_decode_matches_linux_kpp_secret_packet() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/ecdh_helper.c"
        ));
        assert!(source.contains("ECDH_KPP_SECRET_MIN_SIZE"));
        assert!(source.contains("CRYPTO_KPP_SECRET_TYPE_ECDH"));
        assert!(source.contains("params->key = (void *)ptr"));

        let params = EcdhParams { key: b"private" };
        assert_eq!(crypto_ecdh_key_len(params), 13);
        let mut buf = vec![0u8; crypto_ecdh_key_len(params)];
        assert_eq!(crypto_ecdh_encode_key(&mut buf, params), Ok(13));
        assert_eq!(&buf[0..2], &CRYPTO_KPP_SECRET_TYPE_ECDH.to_le_bytes());
        assert_eq!(&buf[2..4], &(13u16).to_le_bytes());
        assert_eq!(&buf[4..6], &(7u16).to_le_bytes());
        assert_eq!(&buf[6..], b"private");

        let decoded = crypto_ecdh_decode_key(&buf).expect("decode");
        assert_eq!(decoded.key_size, 7);
        assert_eq!(decoded.key, b"private");

        let mut with_trailer = buf.clone();
        with_trailer.extend_from_slice(b"ignored");
        assert_eq!(crypto_ecdh_decode_key(&with_trailer), Ok(decoded));
        assert_eq!(crypto_ecdh_encode_key(&mut [0u8; 12], params), Err(-EINVAL));
    }
}
