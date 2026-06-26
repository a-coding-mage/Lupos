//! linux-parity: complete
//! linux-source: vendor/linux/crypto/blowfish_generic.c
//! test-origin: linux:vendor/linux/crypto/blowfish_generic.c
//! Generic Blowfish block cipher translated from Linux.

use super::blowfish_common::{
    BF_BLOCK_SIZE, BF_MAX_KEY_SIZE, BF_MIN_KEY_SIZE, BfCtx, bf_round, blowfish_setkey,
};

pub const CRA_NAME: &str = "blowfish";
pub const CRA_DRIVER_NAME: &str = "blowfish-generic";
pub const CRA_PRIORITY: u32 = 100;
pub const MODULE_DESCRIPTION: &str = "Blowfish Cipher Algorithm";
pub const MODULE_ALIAS_CRYPTO: [&str; 2] = ["blowfish", "blowfish-generic"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CryptoCipherAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
    pub blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
}

pub const BLOWFISH_ALG: CryptoCipherAlg = CryptoCipherAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    blocksize: BF_BLOCK_SIZE,
    min_keysize: BF_MIN_KEY_SIZE,
    max_keysize: BF_MAX_KEY_SIZE,
};

pub fn bf_encrypt(ctx: &BfCtx, dst: &mut [u8; BF_BLOCK_SIZE], src: &[u8; BF_BLOCK_SIZE]) {
    let p = &ctx.p;
    let s = &ctx.s;
    let mut yl = read_be32(src, 0);
    let mut yr = read_be32(src, 4);

    for n in 0..16 {
        if n & 1 == 0 {
            bf_round(&mut yr, &mut yl, p, s, n);
        } else {
            bf_round(&mut yl, &mut yr, p, s, n);
        }
    }

    yl ^= p[16];
    yr ^= p[17];
    write_be32(dst, 0, yr);
    write_be32(dst, 4, yl);
}

pub fn bf_decrypt(ctx: &BfCtx, dst: &mut [u8; BF_BLOCK_SIZE], src: &[u8; BF_BLOCK_SIZE]) {
    let p = &ctx.p;
    let s = &ctx.s;
    let mut yl = read_be32(src, 0);
    let mut yr = read_be32(src, 4);

    let mut n = 17usize;
    while n >= 2 {
        if n & 1 == 1 {
            bf_round(&mut yr, &mut yl, p, s, n);
        } else {
            bf_round(&mut yl, &mut yr, p, s, n);
        }
        n -= 1;
    }

    yl ^= p[1];
    yr ^= p[0];
    write_be32(dst, 0, yr);
    write_be32(dst, 4, yl);
}

pub fn blowfish_encrypt_block(ctx: &BfCtx, src: &[u8; BF_BLOCK_SIZE]) -> [u8; BF_BLOCK_SIZE] {
    let mut dst = [0u8; BF_BLOCK_SIZE];
    bf_encrypt(ctx, &mut dst, src);
    dst
}

pub fn blowfish_decrypt_block(ctx: &BfCtx, src: &[u8; BF_BLOCK_SIZE]) -> [u8; BF_BLOCK_SIZE] {
    let mut dst = [0u8; BF_BLOCK_SIZE];
    bf_decrypt(ctx, &mut dst, src);
    dst
}

#[inline]
const fn read_be32(input: &[u8; BF_BLOCK_SIZE], offset: usize) -> u32 {
    u32::from_be_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

#[inline]
fn write_be32(output: &mut [u8; BF_BLOCK_SIZE], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blowfish_generic_matches_linux_testmgr_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/blowfish_generic.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        assert!(source.contains("static void bf_encrypt"));
        assert!(source.contains("static void bf_decrypt"));
        assert!(source.contains(".cra_driver_name\t=\t\"blowfish-generic\""));
        assert!(source.contains(".cia_setkey\t\t=\tblowfish_setkey"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"blowfish-generic\")"));
        assert!(testmgr.contains("static const struct cipher_testvec bf_tv_template[]"));
        assert!(testmgr.contains(".klen\t= 56"));

        let vectors: &[(&[u8], [u8; BF_BLOCK_SIZE], [u8; BF_BLOCK_SIZE])] = &[
            (
                &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x4e, 0xf9, 0x97, 0x45, 0x61, 0x98, 0xdd, 0x78],
            ),
            (
                &[0x1f, 0x1f, 0x1f, 0x1f, 0x0e, 0x0e, 0x0e, 0x0e],
                [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
                [0xa7, 0x90, 0x79, 0x51, 0x08, 0xea, 0x3c, 0xae],
            ),
            (
                &[0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87],
                [0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10],
                [0xe8, 0x7a, 0x24, 0x4e, 0x2c, 0xc8, 0x5e, 0x82],
            ),
            (
                &[
                    0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87, 0x78, 0x69, 0x5a, 0x4b, 0x3c,
                    0x2d, 0x1e, 0x0f,
                ],
                [0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10],
                [0x93, 0x14, 0x28, 0x87, 0xee, 0x3b, 0xe1, 0x5c],
            ),
            (
                &[
                    0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87, 0x78, 0x69, 0x5a, 0x4b, 0x3c,
                    0x2d, 0x1e, 0x0f, 0x00, 0x11, 0x22, 0x33, 0x44,
                ],
                [0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10],
                [0xe6, 0xf5, 0x1e, 0xd7, 0x9b, 0x9d, 0xb2, 0x1f],
            ),
        ];

        for (key, plaintext, ciphertext) in vectors {
            let mut ctx = BfCtx::default();
            blowfish_setkey(&mut ctx, key).expect("setkey");
            assert_eq!(blowfish_encrypt_block(&ctx, plaintext), *ciphertext);
            assert_eq!(blowfish_decrypt_block(&ctx, ciphertext), *plaintext);
        }

        assert_eq!(BLOWFISH_ALG.blocksize, 8);
        assert_eq!(BLOWFISH_ALG.min_keysize, 4);
        assert_eq!(BLOWFISH_ALG.max_keysize, 56);
        assert_eq!(MODULE_ALIAS_CRYPTO, ["blowfish", "blowfish-generic"]);
    }
}
