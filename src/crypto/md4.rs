//! linux-parity: complete
//! linux-source: vendor/linux/crypto/md4.c
//! test-origin: linux:vendor/linux/crypto/md4.c
//! MD4 shash implementation from the Linux Crypto API.

pub const MD4_DIGEST_SIZE: usize = 16;
pub const MD4_HMAC_BLOCK_SIZE: usize = 64;
pub const MD4_BLOCK_WORDS: usize = 16;
pub const MD4_HASH_WORDS: usize = 4;
pub const CRA_NAME: &str = "md4";
pub const CRA_DRIVER_NAME: &str = "md4-generic";
pub const MODULE_DESCRIPTION: &str = "MD4 Message Digest Algorithm";
pub const MODULE_ALIAS_CRYPTO: &str = "md4";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Md4Ctx {
    pub hash: [u32; MD4_HASH_WORDS],
    block: [u8; MD4_HMAC_BLOCK_SIZE],
    pub byte_count: u64,
}

impl Default for Md4Ctx {
    fn default() -> Self {
        Self {
            hash: [0; MD4_HASH_WORDS],
            block: [0; MD4_HMAC_BLOCK_SIZE],
            byte_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShashAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_blocksize: usize,
    pub descsize: usize,
    pub digestsize: usize,
}

pub const MD4_ALG: ShashAlg = ShashAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_blocksize: MD4_HMAC_BLOCK_SIZE,
    descsize: core::mem::size_of::<Md4Ctx>(),
    digestsize: MD4_DIGEST_SIZE,
};

#[inline]
const fn lshift(x: u32, s: u32) -> u32 {
    x.rotate_left(s)
}

#[inline]
const fn f(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (!x & z)
}

#[inline]
const fn g(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (x & z) | (y & z)
}

#[inline]
const fn h(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}

#[inline]
fn round1(a: &mut u32, b: u32, c: u32, d: u32, k: u32, s: u32) {
    *a = lshift(a.wrapping_add(f(b, c, d)).wrapping_add(k), s);
}

#[inline]
fn round2(a: &mut u32, b: u32, c: u32, d: u32, k: u32, s: u32) {
    *a = lshift(
        a.wrapping_add(g(b, c, d))
            .wrapping_add(k)
            .wrapping_add(0x5a82_7999),
        s,
    );
}

#[inline]
fn round3(a: &mut u32, b: u32, c: u32, d: u32, k: u32, s: u32) {
    *a = lshift(
        a.wrapping_add(h(b, c, d))
            .wrapping_add(k)
            .wrapping_add(0x6ed9_eba1),
        s,
    );
}

fn md4_transform(hash: &mut [u32; MD4_HASH_WORDS], block: &[u8; MD4_HMAC_BLOCK_SIZE]) {
    let mut input = [0u32; MD4_BLOCK_WORDS];
    for (index, chunk) in block.chunks_exact(4).enumerate() {
        input[index] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    let mut a = hash[0];
    let mut b = hash[1];
    let mut c = hash[2];
    let mut d = hash[3];

    round1(&mut a, b, c, d, input[0], 3);
    round1(&mut d, a, b, c, input[1], 7);
    round1(&mut c, d, a, b, input[2], 11);
    round1(&mut b, c, d, a, input[3], 19);
    round1(&mut a, b, c, d, input[4], 3);
    round1(&mut d, a, b, c, input[5], 7);
    round1(&mut c, d, a, b, input[6], 11);
    round1(&mut b, c, d, a, input[7], 19);
    round1(&mut a, b, c, d, input[8], 3);
    round1(&mut d, a, b, c, input[9], 7);
    round1(&mut c, d, a, b, input[10], 11);
    round1(&mut b, c, d, a, input[11], 19);
    round1(&mut a, b, c, d, input[12], 3);
    round1(&mut d, a, b, c, input[13], 7);
    round1(&mut c, d, a, b, input[14], 11);
    round1(&mut b, c, d, a, input[15], 19);

    round2(&mut a, b, c, d, input[0], 3);
    round2(&mut d, a, b, c, input[4], 5);
    round2(&mut c, d, a, b, input[8], 9);
    round2(&mut b, c, d, a, input[12], 13);
    round2(&mut a, b, c, d, input[1], 3);
    round2(&mut d, a, b, c, input[5], 5);
    round2(&mut c, d, a, b, input[9], 9);
    round2(&mut b, c, d, a, input[13], 13);
    round2(&mut a, b, c, d, input[2], 3);
    round2(&mut d, a, b, c, input[6], 5);
    round2(&mut c, d, a, b, input[10], 9);
    round2(&mut b, c, d, a, input[14], 13);
    round2(&mut a, b, c, d, input[3], 3);
    round2(&mut d, a, b, c, input[7], 5);
    round2(&mut c, d, a, b, input[11], 9);
    round2(&mut b, c, d, a, input[15], 13);

    round3(&mut a, b, c, d, input[0], 3);
    round3(&mut d, a, b, c, input[8], 9);
    round3(&mut c, d, a, b, input[4], 11);
    round3(&mut b, c, d, a, input[12], 15);
    round3(&mut a, b, c, d, input[2], 3);
    round3(&mut d, a, b, c, input[10], 9);
    round3(&mut c, d, a, b, input[6], 11);
    round3(&mut b, c, d, a, input[14], 15);
    round3(&mut a, b, c, d, input[1], 3);
    round3(&mut d, a, b, c, input[9], 9);
    round3(&mut c, d, a, b, input[5], 11);
    round3(&mut b, c, d, a, input[13], 15);
    round3(&mut a, b, c, d, input[3], 3);
    round3(&mut d, a, b, c, input[11], 9);
    round3(&mut c, d, a, b, input[7], 11);
    round3(&mut b, c, d, a, input[15], 15);

    hash[0] = hash[0].wrapping_add(a);
    hash[1] = hash[1].wrapping_add(b);
    hash[2] = hash[2].wrapping_add(c);
    hash[3] = hash[3].wrapping_add(d);
}

pub fn md4_init(ctx: &mut Md4Ctx) -> i32 {
    ctx.hash = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    ctx.block = [0; MD4_HMAC_BLOCK_SIZE];
    ctx.byte_count = 0;
    0
}

pub fn md4_update(ctx: &mut Md4Ctx, mut data: &[u8]) -> i32 {
    let used = (ctx.byte_count as usize) & 0x3f;
    ctx.byte_count = ctx.byte_count.wrapping_add(data.len() as u64);

    if used != 0 {
        let avail = MD4_HMAC_BLOCK_SIZE - used;
        if data.len() < avail {
            ctx.block[used..used + data.len()].copy_from_slice(data);
            return 0;
        }
        ctx.block[used..].copy_from_slice(&data[..avail]);
        let block = ctx.block;
        md4_transform(&mut ctx.hash, &block);
        data = &data[avail..];
    }

    while data.len() >= MD4_HMAC_BLOCK_SIZE {
        let mut block = [0u8; MD4_HMAC_BLOCK_SIZE];
        block.copy_from_slice(&data[..MD4_HMAC_BLOCK_SIZE]);
        md4_transform(&mut ctx.hash, &block);
        data = &data[MD4_HMAC_BLOCK_SIZE..];
    }

    ctx.block = [0; MD4_HMAC_BLOCK_SIZE];
    ctx.block[..data.len()].copy_from_slice(data);
    0
}

pub fn md4_final(ctx: &mut Md4Ctx, out: &mut [u8; MD4_DIGEST_SIZE]) -> i32 {
    let bit_count = ctx.byte_count << 3;
    let offset = (ctx.byte_count as usize) & 0x3f;

    ctx.block[offset] = 0x80;
    if offset >= 56 {
        for byte in &mut ctx.block[offset + 1..] {
            *byte = 0;
        }
        let block = ctx.block;
        md4_transform(&mut ctx.hash, &block);
        ctx.block = [0; MD4_HMAC_BLOCK_SIZE];
    } else {
        for byte in &mut ctx.block[offset + 1..56] {
            *byte = 0;
        }
    }

    ctx.block[56..64].copy_from_slice(&bit_count.to_le_bytes());
    let block = ctx.block;
    md4_transform(&mut ctx.hash, &block);

    for (index, word) in ctx.hash.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    *ctx = Md4Ctx::default();
    0
}

pub fn md4_digest(data: &[u8]) -> [u8; MD4_DIGEST_SIZE] {
    let mut ctx = Md4Ctx::default();
    let mut out = [0u8; MD4_DIGEST_SIZE];
    md4_init(&mut ctx);
    md4_update(&mut ctx, data);
    md4_final(&mut ctx, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md4_matches_linux_source_and_rfc1320_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/md4.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));

        assert!(source.contains("#define MD4_DIGEST_SIZE\t\t16"));
        assert!(source.contains("#define MD4_HMAC_BLOCK_SIZE\t64"));
        assert!(source.contains("struct md4_ctx"));
        assert!(source.contains("ROUND1(a, b, c, d, in[0], 3);"));
        assert!(source.contains("ROUND2(a, b, c, d,in[ 0], 3);"));
        assert!(source.contains("ROUND3(a, b, c, d,in[ 0], 3);"));
        assert!(source.contains("mctx->hash[0] = 0x67452301;"));
        assert!(source.contains("mctx->block[14] = mctx->byte_count << 3;"));
        assert!(source.contains(".cra_driver_name =\t\"md4-generic\""));
        assert!(source.contains("return crypto_register_shash(&alg);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"md4\")"));
        assert!(testmgr.contains("static const struct hash_testvec md4_tv_template[]"));
        assert!(testmgr.contains("MD4 test vectors from RFC1320"));

        let vectors: &[(&[u8], [u8; MD4_DIGEST_SIZE])] = &[
            (
                b"",
                [
                    0x31, 0xd6, 0xcf, 0xe0, 0xd1, 0x6a, 0xe9, 0x31, 0xb7, 0x3c, 0x59, 0xd7, 0xe0,
                    0xc0, 0x89, 0xc0,
                ],
            ),
            (
                b"a",
                [
                    0xbd, 0xe5, 0x2c, 0xb3, 0x1d, 0xe3, 0x3e, 0x46, 0x24, 0x5e, 0x05, 0xfb, 0xdb,
                    0xd6, 0xfb, 0x24,
                ],
            ),
            (
                b"abc",
                [
                    0xa4, 0x48, 0x01, 0x7a, 0xaf, 0x21, 0xd8, 0x52, 0x5f, 0xc1, 0x0a, 0xe8, 0x7a,
                    0xa6, 0x72, 0x9d,
                ],
            ),
            (
                b"message digest",
                [
                    0xd9, 0x13, 0x0a, 0x81, 0x64, 0x54, 0x9f, 0xe8, 0x18, 0x87, 0x48, 0x06, 0xe1,
                    0xc7, 0x01, 0x4b,
                ],
            ),
            (
                b"abcdefghijklmnopqrstuvwxyz",
                [
                    0xd7, 0x9e, 0x1c, 0x30, 0x8a, 0xa5, 0xbb, 0xcd, 0xee, 0xa8, 0xed, 0x63, 0xdf,
                    0x41, 0x2d, 0xa9,
                ],
            ),
            (
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                [
                    0x04, 0x3f, 0x85, 0x82, 0xf2, 0x41, 0xdb, 0x35, 0x1c, 0xe6, 0x27, 0xe1, 0x53,
                    0xe7, 0xf0, 0xe4,
                ],
            ),
            (
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                [
                    0xe3, 0x3b, 0x4d, 0xdc, 0x9c, 0x38, 0xf2, 0x19, 0x9c, 0x3e, 0x7b, 0x16, 0x4f,
                    0xcc, 0x05, 0x36,
                ],
            ),
        ];

        for (input, digest) in vectors {
            assert_eq!(&md4_digest(input), digest);
        }

        let mut split = Md4Ctx::default();
        let mut out = [0u8; MD4_DIGEST_SIZE];
        assert_eq!(md4_init(&mut split), 0);
        assert_eq!(md4_update(&mut split, b"abc"), 0);
        assert_eq!(md4_update(&mut split, b"defghijklmnopqrstuvwxyz"), 0);
        assert_eq!(md4_final(&mut split, &mut out), 0);
        assert_eq!(out, md4_digest(b"abcdefghijklmnopqrstuvwxyz"));
        assert_eq!(split, Md4Ctx::default());
        assert_eq!(MD4_ALG.descsize, core::mem::size_of::<Md4Ctx>());
    }
}
