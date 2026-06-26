//! linux-parity: complete
//! linux-source: vendor/linux/net/mptcp/crypto.c
//! test-origin: linux:vendor/linux/net/mptcp/crypto.c
//! Multipath TCP SHA-256 key derivation helpers.

extern crate alloc;

use alloc::vec::Vec;

const SHA256_BLOCK_SIZE: usize = 64;
const SHA256_DIGEST_SIZE: usize = 32;
const SHA256_K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

pub fn mptcp_crypto_key_sha(key: u64) -> (u32, u64) {
    let digest = sha256_digest(&key.to_be_bytes());
    let token = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    let idsn = u64::from_be_bytes([
        digest[24], digest[25], digest[26], digest[27], digest[28], digest[29], digest[30],
        digest[31],
    ]);
    (token, idsn)
}

pub fn mptcp_crypto_hmac_sha(key1: u64, key2: u64, msg: &[u8]) -> [u8; SHA256_DIGEST_SIZE] {
    hmac_sha256(&mptcp_hmac_key_bytes(key1, key2), msg)
}

pub fn mptcp_hmac_key_bytes(key1: u64, key2: u64) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[..8].copy_from_slice(&key1.to_be_bytes());
    key[8..].copy_from_slice(&key2.to_be_bytes());
    key
}

fn hmac_sha256(raw_key: &[u8], msg: &[u8]) -> [u8; SHA256_DIGEST_SIZE] {
    let mut key = [0u8; SHA256_BLOCK_SIZE];
    if raw_key.len() > SHA256_BLOCK_SIZE {
        key[..SHA256_DIGEST_SIZE].copy_from_slice(&sha256_digest(raw_key));
    } else {
        key[..raw_key.len()].copy_from_slice(raw_key);
    }

    let mut ipad = [0x36u8; SHA256_BLOCK_SIZE];
    let mut opad = [0x5cu8; SHA256_BLOCK_SIZE];
    for idx in 0..SHA256_BLOCK_SIZE {
        ipad[idx] ^= key[idx];
        opad[idx] ^= key[idx];
    }

    let mut inner = Vec::with_capacity(SHA256_BLOCK_SIZE + msg.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(msg);
    let inner_digest = sha256_digest(&inner);

    let mut outer = Vec::with_capacity(SHA256_BLOCK_SIZE + SHA256_DIGEST_SIZE);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha256_digest(&outer)
}

fn sha256_digest(data: &[u8]) -> [u8; SHA256_DIGEST_SIZE] {
    let mut h = [
        0x6a09_e667u32,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = Vec::with_capacity(data.len() + 72);
    msg.extend_from_slice(data);
    msg.push(0x80);
    while (msg.len() + 8) % SHA256_BLOCK_SIZE != 0 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for chunk in msg.chunks_exact(SHA256_BLOCK_SIZE) {
        for (idx, word) in chunk.chunks_exact(4).take(16).enumerate() {
            w[idx] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for idx in 16..64 {
            let s0 =
                w[idx - 15].rotate_right(7) ^ w[idx - 15].rotate_right(18) ^ (w[idx - 15] >> 3);
            let s1 = w[idx - 2].rotate_right(17) ^ w[idx - 2].rotate_right(19) ^ (w[idx - 2] >> 10);
            w[idx] = w[idx - 16]
                .wrapping_add(s0)
                .wrapping_add(w[idx - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for idx in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[idx])
                .wrapping_add(w[idx]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; SHA256_DIGEST_SIZE];
    for (idx, word) in h.iter().enumerate() {
        out[idx * 4..idx * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mptcp_crypto_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mptcp/crypto.c"
        ));
        assert!(source.contains("void mptcp_crypto_key_sha(u64 key, u32 *token, u64 *idsn)"));
        assert!(source.contains("__be64 input = cpu_to_be64(key);"));
        assert!(source.contains("sha256((__force u8 *)&input, sizeof(input)"));
        assert!(source.contains("*token = be32_to_cpu(mptcp_hashed_key[0]);"));
        assert!(source.contains("*idsn = be64_to_cpu(*((__be64 *)&mptcp_hashed_key[6]));"));
        assert!(source.contains("void mptcp_crypto_hmac_sha(u64 key1, u64 key2"));
        assert!(source.contains("__be64 key[2] = { cpu_to_be64(key1), cpu_to_be64(key2) };"));
        assert!(
            source
                .contains("hmac_sha256_usingrawkey((const u8 *)key, sizeof(key), msg, len, hmac);")
        );

        assert_eq!(
            sha256_digest(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
        assert_eq!(
            mptcp_crypto_key_sha(0),
            (0xaf55_70f5, 0xe5b2_328d_e0e8_3dfc)
        );
        assert_eq!(
            mptcp_hmac_key_bytes(0x0102_0304_0506_0708, 0x1112_1314_1516_1718),
            [
                1, 2, 3, 4, 5, 6, 7, 8, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            ]
        );
        assert_ne!(
            mptcp_crypto_hmac_sha(1, 2, b"msg"),
            mptcp_crypto_hmac_sha(2, 1, b"msg")
        );
    }
}
