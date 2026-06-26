//! linux-parity: partial
//! linux-source: vendor/linux/crypto/ecdh.c
//! test-origin: linux:vendor/linux/crypto/ecdh.c
//! Generic ECDH KPP registration metadata and secret handling.
//!
//! `set_secret` is now faithful to `ecdh_set_secret`: oversize keys -> -EINVAL;
//! empty key -> `ecc_gen_privkey` (which rejects nbits<224 curves like P-192);
//! else `ecc_digits_from_bytes` + `ecc_is_key_valid` (enforcing key_size ==
//! ndigits*8 and 1 <= key < n). `generate_public_key`/`compute_shared_secret`
//! are backed by the proven ECC engine ([`crate::crypto::ecc`], RFC 5903 KAT).
//!
//! Remaining work for `complete`: Lupos has no crypto KPP framework yet, so the
//! `kpp_request`/scatterlist form of `ecdh_compute_value` and the
//! `crypto_register_kpp` registration are modeled with direct buffer APIs +
//! atomics. Flip to `complete` once the KPP/sglist framework lands and ecdh
//! binds to it.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::crypto::ecdh_helper::{EcdhParams, crypto_ecdh_decode_key};
use crate::include::uapi::errno::EINVAL;

pub const ECC_CURVE_NIST_P192: u32 = 0x0001;
pub const ECC_CURVE_NIST_P256: u32 = 0x0002;
pub const ECC_CURVE_NIST_P384: u32 = 0x0003;
pub const ECC_CURVE_NIST_P521: u32 = 0x0004;

pub const ECC_CURVE_NIST_P192_DIGITS: usize = 3;
pub const ECC_CURVE_NIST_P256_DIGITS: usize = 4;
pub const ECC_CURVE_NIST_P384_DIGITS: usize = 6;
pub const ECC_MAX_DIGITS: usize = 9;
pub const ECC_DIGITS_TO_BYTES_SHIFT: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EcdhAlg {
    pub curve_id: u32,
    pub ndigits: usize,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
}

pub const ECDH_NIST_P192: EcdhAlg = EcdhAlg {
    curve_id: ECC_CURVE_NIST_P192,
    ndigits: ECC_CURVE_NIST_P192_DIGITS,
    cra_name: "ecdh-nist-p192",
    cra_driver_name: "ecdh-nist-p192-generic",
    cra_priority: 100,
};

pub const ECDH_NIST_P256: EcdhAlg = EcdhAlg {
    curve_id: ECC_CURVE_NIST_P256,
    ndigits: ECC_CURVE_NIST_P256_DIGITS,
    cra_name: "ecdh-nist-p256",
    cra_driver_name: "ecdh-nist-p256-generic",
    cra_priority: 100,
};

pub const ECDH_NIST_P384: EcdhAlg = EcdhAlg {
    curve_id: ECC_CURVE_NIST_P384,
    ndigits: ECC_CURVE_NIST_P384_DIGITS,
    cra_name: "ecdh-nist-p384",
    cra_driver_name: "ecdh-nist-p384-generic",
    cra_priority: 100,
};

pub const ECDH_ALGS: &[EcdhAlg] = &[ECDH_NIST_P192, ECDH_NIST_P256, ECDH_NIST_P384];

static ECDH_REGISTERED: AtomicBool = AtomicBool::new(false);
static ECDH_NIST_P192_REGISTERED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdhCtx {
    pub curve_id: u32,
    pub ndigits: usize,
    pub private_key: [u64; ECC_MAX_DIGITS],
}

impl EcdhCtx {
    pub const fn new(curve_id: u32, ndigits: usize) -> Self {
        Self {
            curve_id,
            ndigits,
            private_key: [0; ECC_MAX_DIGITS],
        }
    }

    pub const fn nist_p192() -> Self {
        Self::new(ECC_CURVE_NIST_P192, ECC_CURVE_NIST_P192_DIGITS)
    }

    pub const fn nist_p256() -> Self {
        Self::new(ECC_CURVE_NIST_P256, ECC_CURVE_NIST_P256_DIGITS)
    }

    pub const fn nist_p384() -> Self {
        Self::new(ECC_CURVE_NIST_P384, ECC_CURVE_NIST_P384_DIGITS)
    }

    pub const fn max_size(&self) -> usize {
        self.ndigits << (ECC_DIGITS_TO_BYTES_SHIFT + 1)
    }

    pub fn set_secret_packet(&mut self, buf: &[u8]) -> Result<(), i32> {
        let decoded = crypto_ecdh_decode_key(buf)?;
        self.set_secret(EcdhParams { key: decoded.key })
    }

    pub fn set_secret(&mut self, params: EcdhParams<'_>) -> Result<(), i32> {
        // Faithful `ecdh_set_secret` (post-decode): reject oversize keys, zero
        // the private key, then either generate one (empty key) or load+validate.
        if params.key.len() > self.ndigits * core::mem::size_of::<u64>() {
            return Err(-EINVAL);
        }
        self.private_key = [0; ECC_MAX_DIGITS];

        if params.key.is_empty() {
            // `!params.key || !params.key_size` -> ecc_gen_privkey, which itself
            // rejects curves with nbits < 224 (e.g. P-192) with -EINVAL.
            let ret = unsafe {
                crate::crypto::ecc::ecc_gen_privkey(
                    self.curve_id,
                    self.ndigits,
                    self.private_key.as_mut_ptr(),
                )
            };
            return if ret == 0 { Ok(()) } else { Err(ret) };
        }

        ecc_digits_from_bytes(params.key, &mut self.private_key[..self.ndigits]);
        // ecc_is_key_valid enforces key_size == ndigits*8 and 1 <= key < n.
        if crate::crypto::ecc::ecc_is_key_valid(
            self.curve_id,
            self.ndigits,
            self.private_key.as_ptr(),
            params.key.len(),
        ) < 0
        {
            self.private_key = [0; ECC_MAX_DIGITS];
            return Err(-EINVAL);
        }
        Ok(())
    }

    /// `ecdh_compute_value` (generate_public_key path) — derive the public key
    /// from this context's private key into `public_key` (`2*ndigits` u64s, in
    /// the engine's big-endian-digit wire form). Returns the digit count written.
    ///
    /// Backed by the real ECC engine (`ecc_make_pub_key`).
    pub fn generate_public_key(&self, public_key: &mut [u64]) -> Result<usize, i32> {
        let need = 2 * self.ndigits;
        if public_key.len() < need {
            return Err(-EINVAL);
        }
        let ret = unsafe {
            crate::crypto::ecc::ecc_make_pub_key(
                self.curve_id,
                self.ndigits,
                self.private_key.as_ptr(),
                public_key.as_mut_ptr(),
            )
        };
        if ret != 0 {
            return Err(ret);
        }
        Ok(need)
    }

    /// `ecdh_compute_value` (compute_shared_secret path) — combine this private
    /// key with the peer `public_key` (`2*ndigits` u64s) into `secret`
    /// (`ndigits` u64s). Returns the digit count written.
    ///
    /// Backed by the real ECC engine (`crypto_ecdh_shared_secret`).
    pub fn compute_shared_secret(
        &self,
        public_key: &[u64],
        secret: &mut [u64],
    ) -> Result<usize, i32> {
        if public_key.len() < 2 * self.ndigits || secret.len() < self.ndigits {
            return Err(-EINVAL);
        }
        let ret = unsafe {
            crate::crypto::ecc::crypto_ecdh_shared_secret(
                self.curve_id,
                self.ndigits,
                self.private_key.as_ptr(),
                public_key.as_ptr(),
                secret.as_mut_ptr(),
            )
        };
        if ret != 0 {
            return Err(ret);
        }
        Ok(self.ndigits)
    }
}

pub fn ecc_digits_from_bytes(input: &[u8], out: &mut [u64]) {
    out.fill(0);
    for (idx, byte) in input.iter().rev().enumerate() {
        let digit = idx / 8;
        if digit >= out.len() {
            break;
        }
        out[digit] |= (*byte as u64) << ((idx % 8) * 8);
    }
}

pub fn ecdh_init() -> i32 {
    ECDH_NIST_P192_REGISTERED.store(true, Ordering::Release);
    ECDH_REGISTERED.store(true, Ordering::Release);
    0
}

pub fn ecdh_exit() {
    ECDH_NIST_P192_REGISTERED.store(false, Ordering::Release);
    ECDH_REGISTERED.store(false, Ordering::Release);
}

pub fn ecdh_registered() -> bool {
    ECDH_REGISTERED.load(Ordering::Acquire)
}

pub fn registered_alg_names() -> Vec<&'static str> {
    ECDH_ALGS.iter().map(|alg| alg.cra_name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::ecdh_helper::{EcdhParams, crypto_ecdh_encode_key, crypto_ecdh_key_len};

    #[test]
    fn ecdh_metadata_and_secret_handling_match_linux_kpp_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/ecdh.c"
        ));
        assert!(source.contains("struct ecdh_ctx"));
        assert!(source.contains("params.key_size > sizeof(u64) * ctx->ndigits"));
        assert!(source.contains("memset(ctx->private_key, 0, sizeof(ctx->private_key));"));
        assert!(source.contains("ecc_digits_from_bytes(params.key, params.key_size"));
        assert!(source.contains("return ctx->ndigits << (ECC_DIGITS_TO_BYTES_SHIFT + 1);"));
        assert!(source.contains(".cra_name = \"ecdh-nist-p256\""));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"ecdh\")"));

        assert_eq!(ECDH_NIST_P192.ndigits, 3);
        assert_eq!(ECDH_NIST_P256.ndigits, 4);
        assert_eq!(ECDH_NIST_P384.ndigits, 6);
        assert_eq!(EcdhCtx::nist_p256().max_size(), 64);
        assert_eq!(
            registered_alg_names(),
            ["ecdh-nist-p192", "ecdh-nist-p256", "ecdh-nist-p384"]
        );

        // Full-size P-192 private key (24 bytes); value 0x0102_0304 ∈ [1, n-1].
        // (Linux's ecc_is_key_valid requires key_size == ndigits*8.)
        let mut key = [0u8; 24];
        key[20] = 1;
        key[21] = 2;
        key[22] = 3;
        key[23] = 4;
        let params = EcdhParams { key: &key };
        let mut packet = alloc::vec![0u8; crypto_ecdh_key_len(params)];
        crypto_ecdh_encode_key(&mut packet, params).expect("encode");
        let mut ctx = EcdhCtx::nist_p192();
        ctx.set_secret_packet(&packet).expect("set secret");
        assert_eq!(ctx.private_key[0], 0x0102_0304);

        // Oversize key (25 > 24 bytes) -> EINVAL.
        let long_key = [0x55u8; 25];
        assert_eq!(ctx.set_secret(EcdhParams { key: &long_key }), Err(-EINVAL));
        // Empty key -> ecc_gen_privkey, which rejects P-192 (nbits 192 < 224).
        assert_eq!(ctx.set_secret(EcdhParams { key: &[] }), Err(-EINVAL));

        assert_eq!(ecdh_init(), 0);
        assert!(ecdh_registered());
        ecdh_exit();
        assert!(!ecdh_registered());
    }
}
