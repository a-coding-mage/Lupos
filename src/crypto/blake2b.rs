//! linux-parity: complete
//! linux-source: vendor/linux/crypto/blake2b.c
//! test-origin: linux:vendor/linux/crypto/blake2b.c
//! Crypto API registration wrapper for BLAKE2b.

use crate::include::uapi::errno::EINVAL;

pub const BLAKE2B_BLOCK_SIZE: usize = 128;
pub const BLAKE2B_HASH_SIZE: usize = 64;
pub const BLAKE2B_KEY_SIZE: usize = 64;
pub const BLAKE2B_160_HASH_SIZE: usize = 20;
pub const BLAKE2B_256_HASH_SIZE: usize = 32;
pub const BLAKE2B_384_HASH_SIZE: usize = 48;
pub const BLAKE2B_512_HASH_SIZE: usize = 64;
pub const CRA_PRIORITY: u32 = 300;
pub const MODULE_DESCRIPTION: &str = "Crypto API support for BLAKE2b";
pub const MODULE_ALIAS_CRYPTO: [&str; 8] = [
    "blake2b-160",
    "blake2b-160-lib",
    "blake2b-256",
    "blake2b-256-lib",
    "blake2b-384",
    "blake2b-384-lib",
    "blake2b-512",
    "blake2b-512-lib",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Blake2bTfmCtx {
    pub keylen: usize,
    pub key: [u8; BLAKE2B_KEY_SIZE],
}

impl Default for Blake2bTfmCtx {
    fn default() -> Self {
        Self {
            keylen: 0,
            key: [0; BLAKE2B_KEY_SIZE],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Blake2bAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub digestsize: usize,
    pub priority: u32,
    pub blocksize: usize,
    pub ctxsize: usize,
    pub descsize_name: &'static str,
    pub optional_key: bool,
}

pub const BLAKE2B_ALGS: &[Blake2bAlg] = &[
    Blake2bAlg {
        cra_name: "blake2b-160",
        cra_driver_name: "blake2b-160-lib",
        digestsize: BLAKE2B_160_HASH_SIZE,
        priority: CRA_PRIORITY,
        blocksize: BLAKE2B_BLOCK_SIZE,
        ctxsize: core::mem::size_of::<Blake2bTfmCtx>(),
        descsize_name: "struct blake2b_ctx",
        optional_key: true,
    },
    Blake2bAlg {
        cra_name: "blake2b-256",
        cra_driver_name: "blake2b-256-lib",
        digestsize: BLAKE2B_256_HASH_SIZE,
        priority: CRA_PRIORITY,
        blocksize: BLAKE2B_BLOCK_SIZE,
        ctxsize: core::mem::size_of::<Blake2bTfmCtx>(),
        descsize_name: "struct blake2b_ctx",
        optional_key: true,
    },
    Blake2bAlg {
        cra_name: "blake2b-384",
        cra_driver_name: "blake2b-384-lib",
        digestsize: BLAKE2B_384_HASH_SIZE,
        priority: CRA_PRIORITY,
        blocksize: BLAKE2B_BLOCK_SIZE,
        ctxsize: core::mem::size_of::<Blake2bTfmCtx>(),
        descsize_name: "struct blake2b_ctx",
        optional_key: true,
    },
    Blake2bAlg {
        cra_name: "blake2b-512",
        cra_driver_name: "blake2b-512-lib",
        digestsize: BLAKE2B_512_HASH_SIZE,
        priority: CRA_PRIORITY,
        blocksize: BLAKE2B_BLOCK_SIZE,
        ctxsize: core::mem::size_of::<Blake2bTfmCtx>(),
        descsize_name: "struct blake2b_ctx",
        optional_key: true,
    },
];

pub fn crypto_blake2b_setkey(ctx: &mut Blake2bTfmCtx, key: &[u8]) -> Result<(), i32> {
    if key.len() > BLAKE2B_KEY_SIZE {
        return Err(-EINVAL);
    }
    ctx.key = [0; BLAKE2B_KEY_SIZE];
    ctx.key[..key.len()].copy_from_slice(key);
    ctx.keylen = key.len();
    Ok(())
}

pub fn crypto_blake2b_digest(ctx: &Blake2bTfmCtx, data: &[u8], out: &mut [u8]) -> Result<(), i32> {
    blake2b_digest(&ctx.key[..ctx.keylen], data, out)
}

pub fn blake2b_digest(key: &[u8], data: &[u8], out: &mut [u8]) -> Result<(), i32> {
    if out.is_empty() || out.len() > BLAKE2B_HASH_SIZE || key.len() > BLAKE2B_KEY_SIZE {
        return Err(-EINVAL);
    }
    let mut ctx = Blake2bCtx::new(out.len(), key)?;
    ctx.update(data);
    ctx.finalize(out);
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Blake2bCtx {
    h: [u64; 8],
    t: [u64; 2],
    f: [u64; 2],
    buf: [u8; BLAKE2B_BLOCK_SIZE],
    buflen: usize,
    outlen: usize,
}

const BLAKE2B_IV: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];

const BLAKE2B_SIGMA: [[usize; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

impl Blake2bCtx {
    pub fn new(outlen: usize, key: &[u8]) -> Result<Self, i32> {
        if outlen == 0 || outlen > BLAKE2B_HASH_SIZE || key.len() > BLAKE2B_KEY_SIZE {
            return Err(-EINVAL);
        }
        let mut ctx = Self {
            h: BLAKE2B_IV,
            t: [0; 2],
            f: [0; 2],
            buf: [0; BLAKE2B_BLOCK_SIZE],
            buflen: 0,
            outlen,
        };
        ctx.h[0] ^= 0x0101_0000 ^ ((key.len() as u64) << 8) ^ outlen as u64;
        if !key.is_empty() {
            ctx.buf[..key.len()].copy_from_slice(key);
            ctx.buflen = BLAKE2B_BLOCK_SIZE;
        }
        Ok(ctx)
    }

    pub fn update(&mut self, mut input: &[u8]) {
        if input.is_empty() {
            return;
        }

        if self.buflen > 0 {
            let fill = BLAKE2B_BLOCK_SIZE - self.buflen;
            if input.len() > fill {
                self.buf[self.buflen..].copy_from_slice(&input[..fill]);
                self.increment_counter(BLAKE2B_BLOCK_SIZE as u64);
                let block = self.buf;
                self.compress(&block);
                self.buflen = 0;
                input = &input[fill..];
            }
        }

        while input.len() > BLAKE2B_BLOCK_SIZE {
            self.increment_counter(BLAKE2B_BLOCK_SIZE as u64);
            self.compress(&input[..BLAKE2B_BLOCK_SIZE]);
            input = &input[BLAKE2B_BLOCK_SIZE..];
        }

        self.buf[self.buflen..self.buflen + input.len()].copy_from_slice(input);
        self.buflen += input.len();
    }

    pub fn finalize(&mut self, out: &mut [u8]) {
        assert!(out.len() >= self.outlen);
        self.increment_counter(self.buflen as u64);
        self.f[0] = u64::MAX;
        for byte in &mut self.buf[self.buflen..] {
            *byte = 0;
        }
        let block = self.buf;
        self.compress(&block);

        let mut full = [0u8; BLAKE2B_HASH_SIZE];
        for (index, word) in self.h.iter().enumerate() {
            full[index * 8..index * 8 + 8].copy_from_slice(&word.to_le_bytes());
        }
        out[..self.outlen].copy_from_slice(&full[..self.outlen]);
    }

    fn increment_counter(&mut self, inc: u64) {
        let old = self.t[0];
        self.t[0] = self.t[0].wrapping_add(inc);
        if self.t[0] < old {
            self.t[1] = self.t[1].wrapping_add(1);
        }
    }

    fn compress(&mut self, block: &[u8]) {
        let mut m = [0u64; 16];
        for (index, chunk) in block[..BLAKE2B_BLOCK_SIZE].chunks_exact(8).enumerate() {
            m[index] = u64::from_le_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
        }

        let mut v = [0u64; 16];
        v[..8].copy_from_slice(&self.h);
        v[8..].copy_from_slice(&BLAKE2B_IV);
        v[12] ^= self.t[0];
        v[13] ^= self.t[1];
        v[14] ^= self.f[0];
        v[15] ^= self.f[1];

        for sigma in BLAKE2B_SIGMA {
            g(&mut v, 0, 4, 8, 12, m[sigma[0]], m[sigma[1]]);
            g(&mut v, 1, 5, 9, 13, m[sigma[2]], m[sigma[3]]);
            g(&mut v, 2, 6, 10, 14, m[sigma[4]], m[sigma[5]]);
            g(&mut v, 3, 7, 11, 15, m[sigma[6]], m[sigma[7]]);
            g(&mut v, 0, 5, 10, 15, m[sigma[8]], m[sigma[9]]);
            g(&mut v, 1, 6, 11, 12, m[sigma[10]], m[sigma[11]]);
            g(&mut v, 2, 7, 8, 13, m[sigma[12]], m[sigma[13]]);
            g(&mut v, 3, 4, 9, 14, m[sigma[14]], m[sigma[15]]);
        }

        for index in 0..8 {
            self.h[index] ^= v[index] ^ v[index + 8];
        }
    }
}

fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blake2b_crypto_wrapper_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/blake2b.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/blake2b.h"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2b_kunit.c"
        ));
        let testvecs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2b-testvecs.h"
        ));
        assert!(source.contains("struct blake2b_tfm_ctx"));
        assert!(source.contains("if (keylen > BLAKE2B_KEY_SIZE)"));
        assert!(source.contains("blake2b_init_key(BLAKE2B_CTX(desc), digestsize"));
        assert!(source.contains("blake2b_update(BLAKE2B_CTX(desc), data, len);"));
        assert!(source.contains("blake2b_final(BLAKE2B_CTX(desc), out);"));
        assert!(source.contains("BLAKE2B_ALG(\"blake2b-512\", BLAKE2B_512_HASH_SIZE)"));
        assert!(source.contains("crypto_register_shashes(algs, ARRAY_SIZE(algs))"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"blake2b-512-lib\")"));
        assert!(header.contains("BLAKE2B_BLOCK_SIZE = 128"));
        assert!(header.contains("BLAKE2B_KEY_SIZE = 64"));
        assert!(kunit.contains("test_blake2b_all_key_and_hash_lens"));
        assert!(kunit.contains("KUNIT_CASE(test_blake2b_with_guarded_key_buf)"));
        assert!(kunit.contains("KUNIT_CASE(test_blake2b_with_guarded_out_buf)"));
        assert!(testvecs.contains(".data_len = 0"));
        assert!(testvecs.contains("blake2b_keyed_testvec_consolidated"));

        let mut ctx = Blake2bTfmCtx::default();
        assert_eq!(crypto_blake2b_setkey(&mut ctx, b"kernel"), Ok(()));
        assert_eq!(ctx.keylen, 6);
        assert_eq!(&ctx.key[..6], b"kernel");
        assert_eq!(
            crypto_blake2b_setkey(&mut ctx, &[0u8; BLAKE2B_KEY_SIZE + 1]),
            Err(-EINVAL)
        );
        assert_eq!(BLAKE2B_ALGS.len(), 4);
        assert_eq!(BLAKE2B_ALGS[0].digestsize, 20);
        assert_eq!(BLAKE2B_ALGS[3].cra_driver_name, "blake2b-512-lib");

        let mut digest = [0u8; BLAKE2B_HASH_SIZE];
        blake2b_digest(&[], &[], &mut digest).expect("digest");
        assert_eq!(
            digest,
            [
                0x78, 0x6a, 0x02, 0xf7, 0x42, 0x01, 0x59, 0x03, 0xc6, 0xc6, 0xfd, 0x85, 0x25, 0x52,
                0xd2, 0x72, 0x91, 0x2f, 0x47, 0x40, 0xe1, 0x58, 0x47, 0x61, 0x8a, 0x86, 0xe2, 0x17,
                0xf7, 0x1f, 0x54, 0x19, 0xd2, 0x5e, 0x10, 0x31, 0xaf, 0xee, 0x58, 0x53, 0x13, 0x89,
                0x64, 0x44, 0x93, 0x4e, 0xb0, 0x4b, 0x90, 0x3a, 0x68, 0x5b, 0x14, 0x48, 0xb7, 0x55,
                0xd5, 0x6f, 0x70, 0x1a, 0xfe, 0x9b, 0xe2, 0xce,
            ]
        );

        let mut data = [0u8; 1];
        rand_bytes_seeded_from_len(&mut data, 1);
        blake2b_digest(&[], &data, &mut digest).expect("digest");
        assert_eq!(
            digest,
            [
                0x6f, 0x2e, 0xcc, 0x83, 0x53, 0xa3, 0x20, 0x16, 0x5b, 0xda, 0xd0, 0x04, 0xd3, 0xcb,
                0xe4, 0x37, 0x5b, 0xf0, 0x84, 0x36, 0xe1, 0xad, 0x45, 0xcc, 0x4d, 0x7f, 0x09, 0x68,
                0xb2, 0x62, 0x93, 0x7f, 0x72, 0x32, 0xe8, 0xa7, 0x2f, 0x1f, 0x6f, 0xc6, 0x14, 0xd6,
                0x70, 0xae, 0x0c, 0xf0, 0xf3, 0xce, 0x64, 0x4d, 0x22, 0xdf, 0xc7, 0xa7, 0xf8, 0xa8,
                0x18, 0x23, 0xd8, 0x6c, 0xaf, 0x65, 0xa2, 0x54,
            ]
        );
    }

    fn rand_bytes_seeded_from_len(out: &mut [u8], len: u64) {
        let mut seed = len;
        for byte in out {
            seed = (seed.wrapping_mul(25_214_903_917).wrapping_add(11)) & ((1u64 << 48) - 1);
            *byte = (seed >> 16) as u8;
        }
    }
}
