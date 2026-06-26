//! linux-parity: complete
//! linux-source: vendor/linux/crypto/cast6_generic.c
//! test-origin: linux:vendor/linux/crypto/cast6_generic.c
//! Generic CAST6 block cipher translated from Linux.

use super::cast_common::{CAST_S1, CAST_S2, CAST_S3, CAST_S4};
use crate::include::uapi::errno::EINVAL;

pub const CAST6_BLOCK_SIZE: usize = 16;
pub const CAST6_MIN_KEY_SIZE: usize = 16;
pub const CAST6_MAX_KEY_SIZE: usize = 32;
pub const CRA_NAME: &str = "cast6";
pub const CRA_DRIVER_NAME: &str = "cast6-generic";
pub const CRA_PRIORITY: u32 = 100;
pub const MODULE_DESCRIPTION: &str = "Cast6 Cipher Algorithm";
pub const MODULE_ALIAS_CRYPTO: [&str; 2] = ["cast6", "cast6-generic"];

const TM: [[u32; 8]; 24] = [
    [
        0x5a827999, 0xc95c653a, 0x383650db, 0xa7103c7c, 0x15ea281d, 0x84c413be, 0xf39dff5f,
        0x6277eb00,
    ],
    [
        0xd151d6a1, 0x402bc242, 0xaf05ade3, 0x1ddf9984, 0x8cb98525, 0xfb9370c6, 0x6a6d5c67,
        0xd9474808,
    ],
    [
        0x482133a9, 0xb6fb1f4a, 0x25d50aeb, 0x94aef68c, 0x0388e22d, 0x7262cdce, 0xe13cb96f,
        0x5016a510,
    ],
    [
        0xbef090b1, 0x2dca7c52, 0x9ca467f3, 0x0b7e5394, 0x7a583f35, 0xe9322ad6, 0x580c1677,
        0xc6e60218,
    ],
    [
        0x35bfedb9, 0xa499d95a, 0x1373c4fb, 0x824db09c, 0xf1279c3d, 0x600187de, 0xcedb737f,
        0x3db55f20,
    ],
    [
        0xac8f4ac1, 0x1b693662, 0x8a432203, 0xf91d0da4, 0x67f6f945, 0xd6d0e4e6, 0x45aad087,
        0xb484bc28,
    ],
    [
        0x235ea7c9, 0x9238936a, 0x01127f0b, 0x6fec6aac, 0xdec6564d, 0x4da041ee, 0xbc7a2d8f,
        0x2b541930,
    ],
    [
        0x9a2e04d1, 0x0907f072, 0x77e1dc13, 0xe6bbc7b4, 0x5595b355, 0xc46f9ef6, 0x33498a97,
        0xa2237638,
    ],
    [
        0x10fd61d9, 0x7fd74d7a, 0xeeb1391b, 0x5d8b24bc, 0xcc65105d, 0x3b3efbfe, 0xaa18e79f,
        0x18f2d340,
    ],
    [
        0x87ccbee1, 0xf6a6aa82, 0x65809623, 0xd45a81c4, 0x43346d65, 0xb20e5906, 0x20e844a7,
        0x8fc23048,
    ],
    [
        0xfe9c1be9, 0x6d76078a, 0xdc4ff32b, 0x4b29decc, 0xba03ca6d, 0x28ddb60e, 0x97b7a1af,
        0x06918d50,
    ],
    [
        0x756b78f1, 0xe4456492, 0x531f5033, 0xc1f93bd4, 0x30d32775, 0x9fad1316, 0x0e86feb7,
        0x7d60ea58,
    ],
    [
        0xec3ad5f9, 0x5b14c19a, 0xc9eead3b, 0x38c898dc, 0xa7a2847d, 0x167c701e, 0x85565bbf,
        0xf4304760,
    ],
    [
        0x630a3301, 0xd1e41ea2, 0x40be0a43, 0xaf97f5e4, 0x1e71e185, 0x8d4bcd26, 0xfc25b8c7,
        0x6affa468,
    ],
    [
        0xd9d99009, 0x48b37baa, 0xb78d674b, 0x266752ec, 0x95413e8d, 0x041b2a2e, 0x72f515cf,
        0xe1cf0170,
    ],
    [
        0x50a8ed11, 0xbf82d8b2, 0x2e5cc453, 0x9d36aff4, 0x0c109b95, 0x7aea8736, 0xe9c472d7,
        0x589e5e78,
    ],
    [
        0xc7784a19, 0x365235ba, 0xa52c215b, 0x14060cfc, 0x82dff89d, 0xf1b9e43e, 0x6093cfdf,
        0xcf6dbb80,
    ],
    [
        0x3e47a721, 0xad2192c2, 0x1bfb7e63, 0x8ad56a04, 0xf9af55a5, 0x68894146, 0xd7632ce7,
        0x463d1888,
    ],
    [
        0xb5170429, 0x23f0efca, 0x92cadb6b, 0x01a4c70c, 0x707eb2ad, 0xdf589e4e, 0x4e3289ef,
        0xbd0c7590,
    ],
    [
        0x2be66131, 0x9ac04cd2, 0x099a3873, 0x78742414, 0xe74e0fb5, 0x5627fb56, 0xc501e6f7,
        0x33dbd298,
    ],
    [
        0xa2b5be39, 0x118fa9da, 0x8069957b, 0xef43811c, 0x5e1d6cbd, 0xccf7585e, 0x3bd143ff,
        0xaaab2fa0,
    ],
    [
        0x19851b41, 0x885f06e2, 0xf738f283, 0x6612de24, 0xd4ecc9c5, 0x43c6b566, 0xb2a0a107,
        0x217a8ca8,
    ],
    [
        0x90547849, 0xff2e63ea, 0x6e084f8b, 0xdce23b2c, 0x4bbc26cd, 0xba96126e, 0x296ffe0f,
        0x9849e9b0,
    ],
    [
        0x0723d551, 0x75fdc0f2, 0xe4d7ac93, 0x53b19834, 0xc28b83d5, 0x31656f76, 0xa03f5b17,
        0x0f1946b8,
    ],
];
const TR: [[u8; 8]; 4] = [
    [0x13, 0x04, 0x15, 0x06, 0x17, 0x08, 0x19, 0x0a],
    [0x1b, 0x0c, 0x1d, 0x0e, 0x1f, 0x10, 0x01, 0x12],
    [0x03, 0x14, 0x05, 0x16, 0x07, 0x18, 0x09, 0x1a],
    [0x0b, 0x1c, 0x0d, 0x1e, 0x0f, 0x00, 0x11, 0x02],
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cast6Ctx {
    pub km: [[u32; 4]; 12],
    pub kr: [[u8; 4]; 12],
}

impl Default for Cast6Ctx {
    fn default() -> Self {
        Self {
            km: [[0; 4]; 12],
            kr: [[0; 4]; 12],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CryptoCipherAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
    pub blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
}

pub const CAST6_ALG: CryptoCipherAlg = CryptoCipherAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    blocksize: CAST6_BLOCK_SIZE,
    min_keysize: CAST6_MIN_KEY_SIZE,
    max_keysize: CAST6_MAX_KEY_SIZE,
};

#[inline]
fn f1(d: u32, r: u8, m: u32) -> u32 {
    let i = m.wrapping_add(d).rotate_left(r as u32);
    (CAST_S1[(i >> 24) as usize] ^ CAST_S2[((i >> 16) & 0xff) as usize])
        .wrapping_sub(CAST_S3[((i >> 8) & 0xff) as usize])
        .wrapping_add(CAST_S4[(i & 0xff) as usize])
}
#[inline]
fn f2(d: u32, r: u8, m: u32) -> u32 {
    let i = (m ^ d).rotate_left(r as u32);
    CAST_S1[(i >> 24) as usize]
        .wrapping_sub(CAST_S2[((i >> 16) & 0xff) as usize])
        .wrapping_add(CAST_S3[((i >> 8) & 0xff) as usize])
        ^ CAST_S4[(i & 0xff) as usize]
}
#[inline]
fn f3(d: u32, r: u8, m: u32) -> u32 {
    let i = m.wrapping_sub(d).rotate_left(r as u32);
    (CAST_S1[(i >> 24) as usize].wrapping_add(CAST_S2[((i >> 16) & 0xff) as usize])
        ^ CAST_S3[((i >> 8) & 0xff) as usize])
        .wrapping_sub(CAST_S4[(i & 0xff) as usize])
}

fn w(key: &mut [u32; 8], i: usize) {
    key[6] ^= f1(key[7], TR[i % 4][0], TM[i][0]);
    key[5] ^= f2(key[6], TR[i % 4][1], TM[i][1]);
    key[4] ^= f3(key[5], TR[i % 4][2], TM[i][2]);
    key[3] ^= f1(key[4], TR[i % 4][3], TM[i][3]);
    key[2] ^= f2(key[3], TR[i % 4][4], TM[i][4]);
    key[1] ^= f3(key[2], TR[i % 4][5], TM[i][5]);
    key[0] ^= f1(key[1], TR[i % 4][6], TM[i][6]);
    key[7] ^= f2(key[0], TR[i % 4][7], TM[i][7]);
}

pub fn __cast6_setkey(c: &mut Cast6Ctx, in_key: &[u8]) -> Result<(), i32> {
    if in_key.len() < CAST6_MIN_KEY_SIZE
        || in_key.len() > CAST6_MAX_KEY_SIZE
        || in_key.len() % 4 != 0
    {
        return Err(-EINVAL);
    }
    let mut p_key = [0u8; 32];
    p_key[..in_key.len()].copy_from_slice(in_key);
    let mut key = [
        read_be32_32(&p_key, 0),
        read_be32_32(&p_key, 4),
        read_be32_32(&p_key, 8),
        read_be32_32(&p_key, 12),
        read_be32_32(&p_key, 16),
        read_be32_32(&p_key, 20),
        read_be32_32(&p_key, 24),
        read_be32_32(&p_key, 28),
    ];
    for i in 0..12 {
        w(&mut key, 2 * i);
        w(&mut key, 2 * i + 1);
        c.kr[i][0] = (key[0] & 0x1f) as u8;
        c.kr[i][1] = (key[2] & 0x1f) as u8;
        c.kr[i][2] = (key[4] & 0x1f) as u8;
        c.kr[i][3] = (key[6] & 0x1f) as u8;
        c.km[i][0] = key[7];
        c.km[i][1] = key[5];
        c.km[i][2] = key[3];
        c.km[i][3] = key[1];
    }
    Ok(())
}

pub fn cast6_setkey(c: &mut Cast6Ctx, key: &[u8]) -> Result<(), i32> {
    __cast6_setkey(c, key)
}

fn q(block: &mut [u32; 4], kr: &[u8; 4], km: &[u32; 4]) {
    block[2] ^= f1(block[3], kr[0], km[0]);
    block[1] ^= f2(block[2], kr[1], km[1]);
    block[0] ^= f3(block[1], kr[2], km[2]);
    block[3] ^= f1(block[0], kr[3], km[3]);
}
fn qbar(block: &mut [u32; 4], kr: &[u8; 4], km: &[u32; 4]) {
    block[3] ^= f1(block[0], kr[3], km[3]);
    block[0] ^= f3(block[1], kr[2], km[2]);
    block[1] ^= f2(block[2], kr[1], km[1]);
    block[2] ^= f1(block[3], kr[0], km[0]);
}

pub fn __cast6_encrypt(
    c: &Cast6Ctx,
    outbuf: &mut [u8; CAST6_BLOCK_SIZE],
    inbuf: &[u8; CAST6_BLOCK_SIZE],
) {
    let mut block = [
        read_be32(inbuf, 0),
        read_be32(inbuf, 4),
        read_be32(inbuf, 8),
        read_be32(inbuf, 12),
    ];
    for i in 0..6 {
        q(&mut block, &c.kr[i], &c.km[i]);
    }
    for i in 6..12 {
        qbar(&mut block, &c.kr[i], &c.km[i]);
    }
    for (i, word) in block.iter().copied().enumerate() {
        write_be32(outbuf, i * 4, word);
    }
}

pub fn cast6_encrypt(c: &Cast6Ctx, src: &[u8; CAST6_BLOCK_SIZE]) -> [u8; CAST6_BLOCK_SIZE] {
    let mut dst = [0u8; CAST6_BLOCK_SIZE];
    __cast6_encrypt(c, &mut dst, src);
    dst
}

pub fn __cast6_decrypt(
    c: &Cast6Ctx,
    outbuf: &mut [u8; CAST6_BLOCK_SIZE],
    inbuf: &[u8; CAST6_BLOCK_SIZE],
) {
    let mut block = [
        read_be32(inbuf, 0),
        read_be32(inbuf, 4),
        read_be32(inbuf, 8),
        read_be32(inbuf, 12),
    ];
    for i in (6..12).rev() {
        q(&mut block, &c.kr[i], &c.km[i]);
    }
    for i in (0..6).rev() {
        qbar(&mut block, &c.kr[i], &c.km[i]);
    }
    for (i, word) in block.iter().copied().enumerate() {
        write_be32(outbuf, i * 4, word);
    }
}

pub fn cast6_decrypt(c: &Cast6Ctx, src: &[u8; CAST6_BLOCK_SIZE]) -> [u8; CAST6_BLOCK_SIZE] {
    let mut dst = [0u8; CAST6_BLOCK_SIZE];
    __cast6_decrypt(c, &mut dst, src);
    dst
}

#[inline]
const fn read_be32(input: &[u8; CAST6_BLOCK_SIZE], offset: usize) -> u32 {
    u32::from_be_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}
#[inline]
const fn read_be32_32(input: &[u8; 32], offset: usize) -> u32 {
    u32::from_be_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}
#[inline]
fn write_be32(output: &mut [u8; CAST6_BLOCK_SIZE], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast6_generic_matches_linux_testmgr_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/cast6_generic.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/cast6.h"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        assert!(source.contains("int __cast6_setkey"));
        assert!(source.contains("void __cast6_encrypt"));
        assert!(source.contains("void __cast6_decrypt"));
        assert!(source.contains("if (key_len % 4 != 0)"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"cast6-generic\")"));
        assert!(header.contains("#define CAST6_BLOCK_SIZE 16"));
        assert!(testmgr.contains("static const struct cipher_testvec cast6_tv_template[]"));
        assert!(testmgr.contains("\\xc8\\x42\\xa0\\x89\\x72\\xb4\\x3d\\x20"));
        let vectors: &[(&[u8], [u8; CAST6_BLOCK_SIZE])] = &[
            (
                &[
                    0x23, 0x42, 0xbb, 0x9e, 0xfa, 0x38, 0x54, 0x2c, 0x0a, 0xf7, 0x56, 0x47, 0xf2,
                    0x9f, 0x61, 0x5d,
                ],
                [
                    0xc8, 0x42, 0xa0, 0x89, 0x72, 0xb4, 0x3d, 0x20, 0x83, 0x6c, 0x91, 0xd1, 0xb7,
                    0x53, 0x0f, 0x6b,
                ],
            ),
            (
                &[
                    0x23, 0x42, 0xbb, 0x9e, 0xfa, 0x38, 0x54, 0x2c, 0xbe, 0xd0, 0xac, 0x83, 0x94,
                    0x0a, 0xc2, 0x98, 0xba, 0xc7, 0x7a, 0x77, 0x17, 0x94, 0x28, 0x63,
                ],
                [
                    0x1b, 0x38, 0x6c, 0x02, 0x10, 0xdc, 0xad, 0xcb, 0xdd, 0x0e, 0x41, 0xaa, 0x08,
                    0xa7, 0xa7, 0xe8,
                ],
            ),
            (
                &[
                    0x23, 0x42, 0xbb, 0x9e, 0xfa, 0x38, 0x54, 0x2c, 0xbe, 0xd0, 0xac, 0x83, 0x94,
                    0x0a, 0xc2, 0x98, 0x8d, 0x7c, 0x47, 0xce, 0x26, 0x49, 0x08, 0x46, 0x1c, 0xc1,
                    0xb5, 0x13, 0x7a, 0xe6, 0xb6, 0x04,
                ],
                [
                    0x4f, 0x6a, 0x20, 0x38, 0x28, 0x68, 0x97, 0xb9, 0xc9, 0x87, 0x01, 0x36, 0x55,
                    0x33, 0x17, 0xfa,
                ],
            ),
        ];
        for (key, ciphertext) in vectors {
            let mut ctx = Cast6Ctx::default();
            cast6_setkey(&mut ctx, key).expect("setkey");
            let plaintext = [0u8; CAST6_BLOCK_SIZE];
            assert_eq!(cast6_encrypt(&ctx, &plaintext), *ciphertext);
            assert_eq!(cast6_decrypt(&ctx, ciphertext), plaintext);
        }
        let mut ctx = Cast6Ctx::default();
        assert_eq!(cast6_setkey(&mut ctx, &[0; 15]), Err(-EINVAL));
        assert_eq!(cast6_setkey(&mut ctx, &[0; 17]), Err(-EINVAL));
        assert_eq!(CAST6_ALG.min_keysize, 16);
        assert_eq!(CAST6_ALG.max_keysize, 32);
    }
}
