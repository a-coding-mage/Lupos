//! linux-parity: complete
//! linux-source: vendor/linux/crypto/dh_helper.c
//! test-origin: linux:vendor/linux/crypto/dh_helper.c
//! DH private-key packet encoder and decoder for KPP.

use crate::include::uapi::errno::EINVAL;

pub const CRYPTO_KPP_SECRET_TYPE_DH: u16 = 1;
pub const KPP_SECRET_SIZE: usize = 4;
pub const DH_KPP_SECRET_MIN_SIZE: usize = KPP_SECRET_SIZE + 3 * core::mem::size_of::<u32>();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DhParams<'a> {
    pub key: &'a [u8],
    pub p: &'a [u8],
    pub g: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedDhKey<'a> {
    pub key_size: u32,
    pub p_size: u32,
    pub g_size: u32,
    pub key: &'a [u8],
    pub p: &'a [u8],
    pub g: &'a [u8],
}

fn dh_data_size(params: DhParams<'_>) -> usize {
    params.key.len() + params.p.len() + params.g.len()
}

pub fn crypto_dh_key_len(params: DhParams<'_>) -> usize {
    DH_KPP_SECRET_MIN_SIZE + dh_data_size(params)
}

pub fn crypto_dh_encode_key(buf: &mut [u8], params: DhParams<'_>) -> Result<(), i32> {
    if buf.is_empty() || buf.len() != crypto_dh_key_len(params) {
        return Err(-EINVAL);
    }

    let mut offset = 0usize;
    pack_u16(buf, &mut offset, CRYPTO_KPP_SECRET_TYPE_DH)?;
    pack_u16(buf, &mut offset, buf.len() as u16)?;
    pack_u32(buf, &mut offset, params.key.len() as u32)?;
    pack_u32(buf, &mut offset, params.p.len() as u32)?;
    pack_u32(buf, &mut offset, params.g.len() as u32)?;
    pack_bytes(buf, &mut offset, params.key)?;
    pack_bytes(buf, &mut offset, params.p)?;
    pack_bytes(buf, &mut offset, params.g)?;

    if offset != buf.len() {
        return Err(-EINVAL);
    }
    Ok(())
}

pub fn __crypto_dh_decode_key(buf: &[u8]) -> Result<DecodedDhKey<'_>, i32> {
    if buf.len() < DH_KPP_SECRET_MIN_SIZE {
        return Err(-EINVAL);
    }

    let mut offset = 0usize;
    let secret_type = unpack_u16(buf, &mut offset)?;
    let secret_len = unpack_u16(buf, &mut offset)? as usize;
    if secret_type != CRYPTO_KPP_SECRET_TYPE_DH {
        return Err(-EINVAL);
    }

    let key_size = unpack_u32(buf, &mut offset)?;
    let p_size = unpack_u32(buf, &mut offset)?;
    let g_size = unpack_u32(buf, &mut offset)?;
    let packet_len = DH_KPP_SECRET_MIN_SIZE
        .checked_add(key_size as usize)
        .and_then(|len| len.checked_add(p_size as usize))
        .and_then(|len| len.checked_add(g_size as usize))
        .ok_or(-EINVAL)?;
    if secret_len != packet_len || buf.len() < packet_len {
        return Err(-EINVAL);
    }

    let key_end = offset + key_size as usize;
    let p_end = key_end + p_size as usize;
    let g_end = p_end + g_size as usize;
    Ok(DecodedDhKey {
        key_size,
        p_size,
        g_size,
        key: &buf[offset..key_end],
        p: &buf[key_end..p_end],
        g: &buf[p_end..g_end],
    })
}

pub fn crypto_dh_decode_key(buf: &[u8]) -> Result<DecodedDhKey<'_>, i32> {
    let params = __crypto_dh_decode_key(buf)?;

    if params.key_size > params.p_size || params.g_size > params.p_size {
        return Err(-EINVAL);
    }
    if params.p.iter().all(|byte| *byte == 0) {
        return Err(-EINVAL);
    }

    Ok(params)
}

fn pack_u16(buf: &mut [u8], offset: &mut usize, value: u16) -> Result<(), i32> {
    pack_bytes(buf, offset, &value.to_le_bytes())
}

fn pack_u32(buf: &mut [u8], offset: &mut usize, value: u32) -> Result<(), i32> {
    pack_bytes(buf, offset, &value.to_le_bytes())
}

fn pack_bytes(buf: &mut [u8], offset: &mut usize, data: &[u8]) -> Result<(), i32> {
    let end = offset.checked_add(data.len()).ok_or(-EINVAL)?;
    if end > buf.len() {
        return Err(-EINVAL);
    }
    buf[*offset..end].copy_from_slice(data);
    *offset = end;
    Ok(())
}

fn unpack_u16(buf: &[u8], offset: &mut usize) -> Result<u16, i32> {
    let bytes = unpack_array::<2>(buf, offset)?;
    Ok(u16::from_le_bytes(bytes))
}

fn unpack_u32(buf: &[u8], offset: &mut usize) -> Result<u32, i32> {
    let bytes = unpack_array::<4>(buf, offset)?;
    Ok(u32::from_le_bytes(bytes))
}

fn unpack_array<const N: usize>(buf: &[u8], offset: &mut usize) -> Result<[u8; N], i32> {
    let end = offset.checked_add(N).ok_or(-EINVAL)?;
    if end > buf.len() {
        return Err(-EINVAL);
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&buf[*offset..end]);
    *offset = end;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn dh_encode_decode_matches_linux_kpp_secret_packet() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/dh_helper.c"
        ));
        let dh_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/dh.h"
        ));
        let kpp_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/kpp.h"
        ));
        assert!(source.contains("#define DH_KPP_SECRET_MIN_SIZE"));
        assert!(source.contains("struct kpp_secret secret = {"));
        assert!(source.contains(".type = CRYPTO_KPP_SECRET_TYPE_DH"));
        assert!(source.contains("ptr = dh_pack_data(ptr, end, &params->key_size"));
        assert!(source.contains("params->key = (void *)ptr;"));
        assert!(source.contains("if (params->key_size > params->p_size ||"));
        assert!(source.contains("if (memchr_inv(params->p, 0, params->p_size) == NULL)"));
        assert!(dh_header.contains("unsigned int crypto_dh_key_len(const struct dh *params);"));
        assert!(dh_header.contains("int crypto_dh_encode_key(char *buf"));
        assert!(kpp_header.contains("CRYPTO_KPP_SECRET_TYPE_DH,"));
        assert!(kpp_header.contains("struct kpp_secret"));

        let params = DhParams {
            key: b"key",
            p: b"\x01prime",
            g: b"gen",
        };
        assert_eq!(crypto_dh_key_len(params), 28);
        let mut buf = vec![0u8; crypto_dh_key_len(params)];
        assert_eq!(crypto_dh_encode_key(&mut buf, params), Ok(()));
        assert_eq!(&buf[0..2], &CRYPTO_KPP_SECRET_TYPE_DH.to_le_bytes());
        assert_eq!(&buf[2..4], &(28u16).to_le_bytes());
        assert_eq!(&buf[4..8], &(3u32).to_le_bytes());
        assert_eq!(&buf[8..12], &(6u32).to_le_bytes());
        assert_eq!(&buf[12..16], &(3u32).to_le_bytes());
        assert_eq!(&buf[16..19], b"key");
        assert_eq!(&buf[19..25], b"\x01prime");
        assert_eq!(&buf[25..28], b"gen");

        let decoded = crypto_dh_decode_key(&buf).expect("decode");
        assert_eq!(decoded.key, b"key");
        assert_eq!(decoded.p, b"\x01prime");
        assert_eq!(decoded.g, b"gen");

        let mut with_trailer = buf.clone();
        with_trailer.extend_from_slice(b"ignored");
        assert_eq!(crypto_dh_decode_key(&with_trailer), Ok(decoded));
        assert_eq!(crypto_dh_encode_key(&mut [0u8; 0], params), Err(-EINVAL));
        assert_eq!(crypto_dh_encode_key(&mut [0u8; 27], params), Err(-EINVAL));
    }

    #[test]
    fn dh_decode_rejects_linux_public_validation_failures() {
        let mut zero_p = vec![
            0u8;
            crypto_dh_key_len(DhParams {
                key: b"k",
                p: b"\0\0",
                g: b"g",
            })
        ];
        crypto_dh_encode_key(
            &mut zero_p,
            DhParams {
                key: b"k",
                p: b"\0\0",
                g: b"g",
            },
        )
        .expect("encode");
        assert_eq!(crypto_dh_decode_key(&zero_p), Err(-EINVAL));

        let mut bad_size = vec![
            0u8;
            crypto_dh_key_len(DhParams {
                key: b"long",
                p: b"p",
                g: b"g",
            })
        ];
        crypto_dh_encode_key(
            &mut bad_size,
            DhParams {
                key: b"long",
                p: b"p",
                g: b"g",
            },
        )
        .expect("encode");
        assert_eq!(
            __crypto_dh_decode_key(&bad_size).expect("internal").key,
            b"long"
        );
        assert_eq!(crypto_dh_decode_key(&bad_size), Err(-EINVAL));

        bad_size[0..2].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(__crypto_dh_decode_key(&bad_size), Err(-EINVAL));
    }
}
