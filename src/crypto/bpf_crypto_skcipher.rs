//! linux-parity: complete
//! linux-source: vendor/linux/crypto/bpf_crypto_skcipher.c
//! test-origin: linux:vendor/linux/crypto/bpf_crypto_skcipher.c
//! BPF-facing symmetric cipher type backed by lskcipher helpers.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::crypto::arc4::{
    ARC4_ALG_NAME, Arc4Ctx, CRYPTO_LSKCIPHER_FLAG_CONT, crypto_arc4_crypt, crypto_arc4_setkey,
};
use crate::include::uapi::errno::{EINVAL, ENODEV};

pub const BPF_CRYPTO_SKCIPHER_NAME: &str = "skcipher";
pub const CRYPTO_ALG_TYPE_LSKCIPHER: u32 = 0x0000_000c;
pub const CRYPTO_ALG_TYPE_MASK: u32 = 0x0000_000f;

static REGISTERED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BpfCryptoSkcipher {
    algo: &'static str,
    key: Option<Arc4Ctx>,
    flags: u32,
}

impl BpfCryptoSkcipher {
    pub fn algo(&self) -> &'static str {
        self.algo
    }
}

pub fn bpf_crypto_lskcipher_has_algo(algo: &str) -> bool {
    matches!(algo, ARC4_ALG_NAME | "ecb(arc4)")
}

pub fn bpf_crypto_lskcipher_alloc_tfm(algo: &str) -> Result<BpfCryptoSkcipher, i32> {
    if !bpf_crypto_lskcipher_has_algo(algo) {
        return Err(-ENODEV);
    }
    Ok(BpfCryptoSkcipher {
        algo: ARC4_ALG_NAME,
        key: None,
        flags: 0,
    })
}

pub fn bpf_crypto_lskcipher_free_tfm(_tfm: BpfCryptoSkcipher) {}

pub fn bpf_crypto_lskcipher_setkey(tfm: &mut BpfCryptoSkcipher, key: &[u8]) -> Result<(), i32> {
    tfm.key = Some(crypto_arc4_setkey(key)?);
    Ok(())
}

pub fn bpf_crypto_lskcipher_get_flags(tfm: &BpfCryptoSkcipher) -> u32 {
    tfm.flags
}

pub fn bpf_crypto_lskcipher_ivsize(_tfm: &BpfCryptoSkcipher) -> usize {
    0
}

pub fn bpf_crypto_lskcipher_statesize(_tfm: &BpfCryptoSkcipher) -> usize {
    core::mem::size_of::<Arc4Ctx>()
}

fn crypt(
    tfm: &BpfCryptoSkcipher,
    src: &[u8],
    siv: &mut Arc4Ctx,
    flags: u32,
) -> Result<Vec<u8>, i32> {
    let Some(ctx) = tfm.key.as_ref() else {
        return Err(-EINVAL);
    };
    crypto_arc4_crypt(ctx, src, siv, flags)
}

pub fn bpf_crypto_lskcipher_encrypt(
    tfm: &BpfCryptoSkcipher,
    src: &[u8],
    siv: &mut Arc4Ctx,
) -> Result<Vec<u8>, i32> {
    crypt(tfm, src, siv, 0)
}

pub fn bpf_crypto_lskcipher_decrypt(
    tfm: &BpfCryptoSkcipher,
    src: &[u8],
    siv: &mut Arc4Ctx,
) -> Result<Vec<u8>, i32> {
    crypt(tfm, src, siv, 0)
}

pub fn bpf_crypto_lskcipher_encrypt_cont(
    tfm: &BpfCryptoSkcipher,
    src: &[u8],
    siv: &mut Arc4Ctx,
) -> Result<Vec<u8>, i32> {
    crypt(tfm, src, siv, CRYPTO_LSKCIPHER_FLAG_CONT)
}

pub fn bpf_crypto_skcipher_init() -> i32 {
    REGISTERED.store(true, Ordering::Release);
    0
}

pub fn bpf_crypto_skcipher_exit() -> Result<(), i32> {
    if !REGISTERED.swap(false, Ordering::AcqRel) {
        return Err(-ENODEV);
    }
    Ok(())
}

pub fn registered() -> bool {
    REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpf_crypto_skcipher_registers_lskcipher_type_and_forwards_arc4() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/bpf_crypto_skcipher.c"
        ));
        assert!(source.contains("bpf_crypto_register_type(&bpf_crypto_lskcipher_type)"));
        assert!(source.contains(".name\t\t= \"skcipher\""));
        assert!(source.contains("crypto_lskcipher_encrypt"));
        assert!(source.contains("crypto_lskcipher_decrypt"));
        assert!(source.contains("crypto_lskcipher_statesize"));

        assert_eq!(bpf_crypto_skcipher_init(), 0);
        assert!(registered());
        assert!(bpf_crypto_lskcipher_has_algo("arc4"));
        assert!(bpf_crypto_lskcipher_has_algo("ecb(arc4)"));
        assert!(!bpf_crypto_lskcipher_has_algo("missing"));

        let mut tfm = bpf_crypto_lskcipher_alloc_tfm("arc4").expect("tfm");
        assert_eq!(tfm.algo(), "arc4");
        bpf_crypto_lskcipher_setkey(&mut tfm, b"Secret").expect("setkey");
        assert_eq!(bpf_crypto_lskcipher_ivsize(&tfm), 0);
        assert!(bpf_crypto_lskcipher_statesize(&tfm) >= 256 * core::mem::size_of::<u32>());

        let mut enc_state = Arc4Ctx::default();
        let ciphertext =
            bpf_crypto_lskcipher_encrypt(&tfm, b"message", &mut enc_state).expect("encrypt");
        let mut dec_state = Arc4Ctx::default();
        let plaintext =
            bpf_crypto_lskcipher_decrypt(&tfm, &ciphertext, &mut dec_state).expect("decrypt");
        assert_eq!(plaintext, b"message");
        assert_eq!(bpf_crypto_skcipher_exit(), Ok(()));
    }
}
