//! linux-parity: complete
//! linux-source: vendor/linux/crypto/xxhash_generic.c
//! test-origin: linux:vendor/linux/crypto/xxhash_generic.c
//! Crypto API xxhash64 wrapper contract and portable XXH64 core.

use crate::include::uapi::errno::EINVAL;

pub const XXHASH64_BLOCK_SIZE: usize = 32;
pub const XXHASH64_DIGEST_SIZE: usize = 8;
pub const CRA_NAME: &str = "xxhash64";
pub const CRA_DRIVER_NAME: &str = "xxhash64-generic";
pub const CRA_PRIORITY: u32 = 100;
pub const MODULE_DESCRIPTION: &str = "xxhash calculations wrapper for lib/xxhash.c";
pub const MODULE_ALIAS_CRYPTO: [&str; 2] = ["xxhash64", "xxhash64-generic"];

const PRIME64_1: u64 = 11_400_714_785_074_694_791;
const PRIME64_2: u64 = 14_029_467_366_897_019_727;
const PRIME64_3: u64 = 1_609_587_929_392_839_161;
const PRIME64_4: u64 = 9_650_029_242_287_828_579;
const PRIME64_5: u64 = 2_870_177_450_012_600_261;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Xxhash64TfmCtx {
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShashAlg {
    pub digestsize: usize,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
    pub cra_blocksize: usize,
    pub cra_ctxsize: usize,
    pub optional_key: bool,
}

pub const XXHASH64_ALG: ShashAlg = ShashAlg {
    digestsize: XXHASH64_DIGEST_SIZE,
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    cra_blocksize: XXHASH64_BLOCK_SIZE,
    cra_ctxsize: core::mem::size_of::<Xxhash64TfmCtx>(),
    optional_key: true,
};

pub fn xxhash64_setkey(ctx: &mut Xxhash64TfmCtx, key: &[u8]) -> Result<(), i32> {
    if key.len() != core::mem::size_of::<u64>() {
        return Err(-EINVAL);
    }
    let mut seed = [0u8; 8];
    seed.copy_from_slice(key);
    ctx.seed = u64::from_le_bytes(seed);
    Ok(())
}

pub fn xxhash64_digest_bytes(data: &[u8], seed: u64) -> [u8; XXHASH64_DIGEST_SIZE] {
    xxh64(data, seed).to_le_bytes()
}

pub fn xxh64(data: &[u8], seed: u64) -> u64 {
    let mut index = 0usize;
    let mut hash;

    if data.len() >= XXHASH64_BLOCK_SIZE {
        let mut v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
        let mut v2 = seed.wrapping_add(PRIME64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(PRIME64_1);

        while index <= data.len() - XXHASH64_BLOCK_SIZE {
            v1 = round(v1, read_u64_le(data, index));
            v2 = round(v2, read_u64_le(data, index + 8));
            v3 = round(v3, read_u64_le(data, index + 16));
            v4 = round(v4, read_u64_le(data, index + 24));
            index += XXHASH64_BLOCK_SIZE;
        }

        hash = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        hash = merge_round(hash, v1);
        hash = merge_round(hash, v2);
        hash = merge_round(hash, v3);
        hash = merge_round(hash, v4);
    } else {
        hash = seed.wrapping_add(PRIME64_5);
    }

    hash = hash.wrapping_add(data.len() as u64);

    while index + 8 <= data.len() {
        let k1 = round(0, read_u64_le(data, index));
        hash ^= k1;
        hash = hash
            .rotate_left(27)
            .wrapping_mul(PRIME64_1)
            .wrapping_add(PRIME64_4);
        index += 8;
    }

    if index + 4 <= data.len() {
        hash ^= (read_u32_le(data, index) as u64).wrapping_mul(PRIME64_1);
        hash = hash
            .rotate_left(23)
            .wrapping_mul(PRIME64_2)
            .wrapping_add(PRIME64_3);
        index += 4;
    }

    while index < data.len() {
        hash ^= (data[index] as u64).wrapping_mul(PRIME64_5);
        hash = hash.rotate_left(11).wrapping_mul(PRIME64_1);
        index += 1;
    }

    avalanche(hash)
}

const fn read_u32_le(data: &[u8], index: usize) -> u32 {
    u32::from_le_bytes([
        data[index],
        data[index + 1],
        data[index + 2],
        data[index + 3],
    ])
}

const fn read_u64_le(data: &[u8], index: usize) -> u64 {
    u64::from_le_bytes([
        data[index],
        data[index + 1],
        data[index + 2],
        data[index + 3],
        data[index + 4],
        data[index + 5],
        data[index + 6],
        data[index + 7],
    ])
}

fn round(acc: u64, input: u64) -> u64 {
    acc.wrapping_add(input.wrapping_mul(PRIME64_2))
        .rotate_left(31)
        .wrapping_mul(PRIME64_1)
}

fn merge_round(acc: u64, val: u64) -> u64 {
    (acc ^ round(0, val))
        .wrapping_mul(PRIME64_1)
        .wrapping_add(PRIME64_4)
}

fn avalanche(mut hash: u64) -> u64 {
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(PRIME64_2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(PRIME64_3);
    hash ^ (hash >> 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xxhash_generic_matches_linux_crypto_wrapper() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/xxhash_generic.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        assert!(source.contains("#define XXHASH64_BLOCK_SIZE\t32"));
        assert!(source.contains("#define XXHASH64_DIGEST_SIZE\t8"));
        assert!(source.contains("if (keylen != sizeof(tctx->seed))"));
        assert!(source.contains("tctx->seed = get_unaligned_le64(key);"));
        assert!(source.contains("xxh64_reset(&dctx->xxhstate, tctx->seed);"));
        assert!(source.contains("put_unaligned_le64(xxh64_digest(&dctx->xxhstate), out);"));
        assert!(source.contains(".cra_driver_name = \"xxhash64-generic\""));
        assert!(source.contains(".cra_flags\t = CRYPTO_ALG_OPTIONAL_KEY"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"xxhash64-generic\")"));
        assert!(testmgr.contains("static const struct hash_testvec xxhash64_tv_template[]"));
        assert!(testmgr.contains(".digest = \"\\x99\\xe9\\xd8\\x51\\x37\\xdb\\x46\\xef\""));
        assert!(testmgr.contains(".key = \"\\xb1\\x79\\x37\\x9e\\x00\\x00\\x00\\x00\""));

        let mut ctx = Xxhash64TfmCtx::default();
        assert_eq!(
            xxhash64_setkey(&mut ctx, &0x0123_4567_89ab_cdefu64.to_le_bytes()),
            Ok(())
        );
        assert_eq!(ctx.seed, 0x0123_4567_89ab_cdef);
        assert_eq!(xxhash64_setkey(&mut ctx, &[1, 2, 3]), Err(-EINVAL));
        assert_eq!(xxh64(b"", 0), 0xef46_db37_51d8_e999);
        assert_eq!(
            xxhash64_digest_bytes(b"", 0),
            0xef46_db37_51d8_e999u64.to_le_bytes()
        );
        assert_eq!(
            xxhash64_digest_bytes(&[0x40], 0),
            [0x20, 0x5c, 0x91, 0xaa, 0x88, 0xeb, 0x59, 0xd0]
        );
        assert_eq!(
            xxhash64_digest_bytes(b"", 0x0000_0000_9e37_79b1),
            [0xef, 0x17, 0x9b, 0x92, 0xa2, 0xfd, 0x75, 0xac]
        );
        assert_eq!(XXHASH64_ALG.cra_priority, 100);
    }
}
