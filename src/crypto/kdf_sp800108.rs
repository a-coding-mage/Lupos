//! linux-parity: complete
//! linux-source: vendor/linux/crypto/kdf_sp800108.c
//! test-origin: linux:vendor/linux/crypto/kdf_sp800108.c
//! SP800-108 CTR-mode KDF over the Linux shash contract.

use crate::include::uapi::errno::EINVAL;

pub const HASH_MAX_DIGESTSIZE: usize = 64;
pub const KDF_CTR_HMAC_SHA256_KEY: [u8; 32] = [
    0xdd, 0x1d, 0x91, 0xb7, 0xd9, 0x0b, 0x2b, 0xd3, 0x13, 0x85, 0x33, 0xce, 0x92, 0xb2, 0x72, 0xfb,
    0xf8, 0xa3, 0x69, 0x31, 0x6a, 0xef, 0xe2, 0x42, 0xe6, 0x59, 0xcc, 0x0a, 0xe2, 0x38, 0xaf, 0xe0,
];
pub const KDF_CTR_HMAC_SHA256_INFO: [u8; 60] = [
    0x01, 0x32, 0x2b, 0x96, 0xb3, 0x0a, 0xcd, 0x19, 0x79, 0x79, 0x44, 0x4e, 0x46, 0x8e, 0x1c, 0x5c,
    0x68, 0x59, 0xbf, 0x1b, 0x1c, 0xf9, 0x51, 0xb7, 0xe7, 0x25, 0x30, 0x3e, 0x23, 0x7e, 0x46, 0xb8,
    0x64, 0xa1, 0x45, 0xfa, 0xb2, 0x5e, 0x51, 0x7b, 0x08, 0xf8, 0x68, 0x3d, 0x03, 0x15, 0xbb, 0x29,
    0x11, 0xd8, 0x0a, 0x0e, 0x8a, 0xba, 0x17, 0xf3, 0xb4, 0x13, 0xfa, 0xac,
];
pub const KDF_CTR_HMAC_SHA256_EXPECTED: [u8; 16] = [
    0x10, 0x62, 0x13, 0x42, 0xbf, 0xb0, 0xfd, 0x40, 0x04, 0x6c, 0x0e, 0x29, 0xf2, 0xcf, 0xdb, 0xf0,
];

pub trait CryptoShash {
    fn digest_size(&self) -> usize;
    fn setkey(&mut self, key: &[u8]) -> Result<(), i32>;
    fn init(&mut self) -> Result<(), i32>;
    fn update(&mut self, data: &[u8]) -> Result<(), i32>;
    fn final_into(&mut self, out: &mut [u8]) -> Result<(), i32>;
    fn zero_desc(&mut self);
}

pub fn crypto_kdf108_ctr_generate<M: CryptoShash>(
    kmd: &mut M,
    info: &[&[u8]],
    dst: &mut [u8],
) -> Result<(), i32> {
    let h = kmd.digest_size();
    let dlen_orig = dst.len();
    let mut counter = 1u32;
    let mut offset = 0usize;
    let mut err = Ok(());

    if h == 0 || h > HASH_MAX_DIGESTSIZE {
        err = Err(-EINVAL);
    }

    while err.is_ok() && offset < dlen_orig {
        err = kmd.init();
        if err.is_err() {
            break;
        }

        err = kmd.update(&counter.to_be_bytes());
        if err.is_err() {
            break;
        }

        for piece in info {
            err = kmd.update(piece);
            if err.is_err() {
                break;
            }
        }
        if err.is_err() {
            break;
        }

        let remaining = dlen_orig - offset;
        if remaining < h {
            let mut tmpbuffer = [0u8; HASH_MAX_DIGESTSIZE];
            err = kmd.final_into(&mut tmpbuffer[..h]);
            if err.is_ok() {
                dst[offset..].copy_from_slice(&tmpbuffer[..remaining]);
                tmpbuffer[..h].fill(0);
            }
            break;
        }

        err = kmd.final_into(&mut dst[offset..offset + h]);
        if err.is_err() {
            break;
        }

        offset += h;
        counter = counter.wrapping_add(1);
    }

    if err.is_err() {
        dst.fill(0);
    }
    kmd.zero_desc();
    err
}

pub fn crypto_kdf108_setkey<M: CryptoShash>(
    kmd: &mut M,
    key: &[u8],
    ikm: Option<&[u8]>,
) -> Result<(), i32> {
    if ikm.is_some() {
        return Err(-EINVAL);
    }
    if kmd.digest_size() > key.len() {
        return Err(-EINVAL);
    }
    kmd.setkey(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::integrity::ima::sha256_digest;
    use alloc::vec::Vec;

    const SHA256_DIGEST_SIZE: usize = 32;
    const SHA256_BLOCK_SIZE: usize = 64;

    #[derive(Clone, Debug)]
    struct HmacSha256 {
        key: [u8; SHA256_BLOCK_SIZE],
        data: Vec<u8>,
        fail_update_after: Option<usize>,
        update_count: usize,
        zeroed: bool,
    }

    impl Default for HmacSha256 {
        fn default() -> Self {
            Self {
                key: [0; SHA256_BLOCK_SIZE],
                data: Vec::new(),
                fail_update_after: None,
                update_count: 0,
                zeroed: false,
            }
        }
    }

    impl HmacSha256 {
        fn with_update_failure(fail_update_after: usize) -> Self {
            Self {
                fail_update_after: Some(fail_update_after),
                ..Self::default()
            }
        }
    }

    impl CryptoShash for HmacSha256 {
        fn digest_size(&self) -> usize {
            SHA256_DIGEST_SIZE
        }

        fn setkey(&mut self, key: &[u8]) -> Result<(), i32> {
            self.key = [0; SHA256_BLOCK_SIZE];
            if key.len() > SHA256_BLOCK_SIZE {
                self.key[..SHA256_DIGEST_SIZE].copy_from_slice(&sha256_digest(key));
            } else {
                self.key[..key.len()].copy_from_slice(key);
            }
            Ok(())
        }

        fn init(&mut self) -> Result<(), i32> {
            self.data.clear();
            self.update_count = 0;
            self.zeroed = false;
            Ok(())
        }

        fn update(&mut self, data: &[u8]) -> Result<(), i32> {
            self.update_count += 1;
            if self.fail_update_after == Some(self.update_count) {
                return Err(-EINVAL);
            }
            self.data.extend_from_slice(data);
            Ok(())
        }

        fn final_into(&mut self, out: &mut [u8]) -> Result<(), i32> {
            let mut ipad = [0x36u8; SHA256_BLOCK_SIZE];
            let mut opad = [0x5cu8; SHA256_BLOCK_SIZE];
            for i in 0..SHA256_BLOCK_SIZE {
                ipad[i] ^= self.key[i];
                opad[i] ^= self.key[i];
            }

            let mut inner = Vec::new();
            inner.extend_from_slice(&ipad);
            inner.extend_from_slice(&self.data);
            let inner_digest = sha256_digest(&inner);

            let mut outer = Vec::new();
            outer.extend_from_slice(&opad);
            outer.extend_from_slice(&inner_digest);
            let digest = sha256_digest(&outer);
            out.copy_from_slice(&digest[..out.len()]);
            Ok(())
        }

        fn zero_desc(&mut self) {
            self.data.fill(0);
            self.data.clear();
            self.zeroed = true;
        }
    }

    #[test]
    fn kdf108_matches_linux_source_and_nist_counter_mode_vector() {
        let source_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/kdf_sp800108.c"
        ));
        let source = source_raw.replace("\r\n", "\n");
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/kdf_sp800108.h"
        ));
        assert!(source.contains("int crypto_kdf108_ctr_generate(struct crypto_shash *kmd"));
        assert!(source.contains("__be32 counter = cpu_to_be32(1);"));
        assert!(source.contains("crypto_shash_update(desc, (u8 *)&counter, sizeof(__be32));"));
        assert!(source.contains("crypto_shash_update(desc, info[i].iov_base,"));
        assert!(source.contains("u8 tmpbuffer[HASH_MAX_DIGESTSIZE];"));
        assert!(source.contains("memzero_explicit(tmpbuffer, h);"));
        assert!(source.contains("if (err)\n\t\tmemzero_explicit(dst_orig, dlen_orig);"));
        assert!(source.contains("if (ikm || ikmlen)\n\t\treturn -EINVAL;"));
        assert!(source.contains("if (ds > keylen)\n\t\treturn -EINVAL;"));
        assert!(source.contains("kdf_ctr_hmac_sha256_tv_template"));
        assert!(source.contains(
            "http://csrc.nist.gov/groups/STM/cavp/documents/KBKDF800-108/CounterMode.zip"
        ));
        assert!(header.contains("crypto_kdf108_ctr_generate"));
        assert!(header.contains("crypto_kdf108_setkey"));

        let mut mac = HmacSha256::default();
        assert_eq!(
            crypto_kdf108_setkey(&mut mac, &KDF_CTR_HMAC_SHA256_KEY, None),
            Ok(())
        );

        let mut out = [0u8; 16];
        assert_eq!(
            crypto_kdf108_ctr_generate(&mut mac, &[&KDF_CTR_HMAC_SHA256_INFO], &mut out),
            Ok(())
        );
        assert_eq!(out, KDF_CTR_HMAC_SHA256_EXPECTED);
        assert!(mac.zeroed);
    }

    #[test]
    fn kdf108_uses_be32_counter_chunks_and_zeros_on_error() {
        let mut mac = HmacSha256::default();
        crypto_kdf108_setkey(&mut mac, &KDF_CTR_HMAC_SHA256_KEY, None).expect("setkey");
        let mut first = [0u8; 32];
        crypto_kdf108_ctr_generate(&mut mac, &[b"context"], &mut first).expect("first");

        let mut mac = HmacSha256::default();
        crypto_kdf108_setkey(&mut mac, &KDF_CTR_HMAC_SHA256_KEY, None).expect("setkey");
        let mut two_blocks = [0u8; 64];
        crypto_kdf108_ctr_generate(&mut mac, &[b"context"], &mut two_blocks).expect("two blocks");
        assert_eq!(&two_blocks[..32], &first);
        assert_ne!(&two_blocks[..32], &two_blocks[32..]);

        let mut failing = HmacSha256::with_update_failure(2);
        crypto_kdf108_setkey(&mut failing, &KDF_CTR_HMAC_SHA256_KEY, None).expect("setkey");
        let mut dst = [0xa5u8; 16];
        assert_eq!(
            crypto_kdf108_ctr_generate(&mut failing, &[b"fail"], &mut dst),
            Err(-EINVAL)
        );
        assert_eq!(dst, [0u8; 16]);
        assert!(failing.zeroed);
    }

    #[test]
    fn kdf108_setkey_rejects_linux_ikm_and_short_keys() {
        let mut mac = HmacSha256::default();
        assert_eq!(
            crypto_kdf108_setkey(&mut mac, &KDF_CTR_HMAC_SHA256_KEY, Some(&[])),
            Err(-EINVAL)
        );
        assert_eq!(
            crypto_kdf108_setkey(&mut mac, &[0u8; 31], None),
            Err(-EINVAL)
        );
        assert_eq!(
            crypto_kdf108_setkey(&mut mac, &KDF_CTR_HMAC_SHA256_KEY, None),
            Ok(())
        );
    }
}
