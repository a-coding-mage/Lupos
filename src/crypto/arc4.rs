//! linux-parity: complete
//! linux-source: vendor/linux/crypto/arc4.c
//! test-origin: linux:vendor/linux/crypto/arc4.c
//! ARC4 lskcipher wrapper backed by the Linux ARC4 state machine.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const ARC4_MIN_KEY_SIZE: usize = 1;
pub const ARC4_MAX_KEY_SIZE: usize = 256;
pub const ARC4_BLOCK_SIZE: usize = 1;
pub const CRYPTO_LSKCIPHER_FLAG_CONT: u32 = 1 << 0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arc4Ctx {
    pub s: [u32; 256],
    pub x: u32,
    pub y: u32,
}

impl Default for Arc4Ctx {
    fn default() -> Self {
        Self {
            s: [0; 256],
            x: 0,
            y: 0,
        }
    }
}

pub fn arc4_setkey(ctx: &mut Arc4Ctx, in_key: &[u8]) -> Result<(), i32> {
    if in_key.len() < ARC4_MIN_KEY_SIZE || in_key.len() > ARC4_MAX_KEY_SIZE {
        return Err(-EINVAL);
    }

    ctx.x = 1;
    ctx.y = 0;
    for i in 0..256 {
        ctx.s[i] = i as u32;
    }

    let mut j = 0usize;
    let mut k = 0usize;
    for i in 0..256 {
        let a = ctx.s[i];
        j = (j + in_key[k] as usize + a as usize) & 0xff;
        ctx.s[i] = ctx.s[j];
        ctx.s[j] = a;
        k += 1;
        if k >= in_key.len() {
            k = 0;
        }
    }
    Ok(())
}

pub fn arc4_crypt(ctx: &mut Arc4Ctx, input: &[u8], output: &mut [u8]) -> Result<(), i32> {
    if output.len() < input.len() {
        return Err(-EINVAL);
    }
    if input.is_empty() {
        return Ok(());
    }

    let mut x = ctx.x as usize;
    let mut y = ctx.y as usize;
    let s = &mut ctx.s;

    let mut a = s[x];
    y = (y + a as usize) & 0xff;
    let mut b = s[y];

    for (idx, byte) in input.iter().copied().enumerate() {
        s[y] = a;
        a = (a + b) & 0xff;
        s[x] = b;
        x = (x + 1) & 0xff;
        let ta = s[x];
        let ty = (y + ta as usize) & 0xff;
        let tb = s[ty];
        output[idx] = byte ^ s[a as usize] as u8;

        if idx + 1 == input.len() {
            break;
        }
        y = ty;
        a = ta;
        b = tb;
    }

    ctx.x = x as u32;
    ctx.y = y as u32;
    Ok(())
}

pub fn crypto_arc4_setkey(key: &[u8]) -> Result<Arc4Ctx, i32> {
    let mut ctx = Arc4Ctx::default();
    arc4_setkey(&mut ctx, key)?;
    Ok(ctx)
}

pub fn crypto_arc4_crypt(
    tfm_ctx: &Arc4Ctx,
    src: &[u8],
    siv: &mut Arc4Ctx,
    flags: u32,
) -> Result<Vec<u8>, i32> {
    if flags & CRYPTO_LSKCIPHER_FLAG_CONT == 0 {
        *siv = tfm_ctx.clone();
    }
    let mut dst = alloc::vec![0; src.len()];
    arc4_crypt(siv, src, &mut dst)?;
    Ok(dst)
}

pub const ARC4_ALG_NAME: &str = "arc4";
pub const ARC4_DRIVER_NAME: &str = "arc4-generic";
pub const ARC4_PRIORITY: u32 = 100;

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> Vec<u8> {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut out = Vec::new();
        for byte in bytes {
            out.push(DIGITS[(byte >> 4) as usize]);
            out.push(DIGITS[(byte & 0x0f) as usize]);
        }
        out
    }

    #[test]
    fn arc4_matches_linux_wrapper_and_known_stream_vector() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/arc4.c"
        ));
        assert!(source.contains("crypto_arc4_setkey"));
        assert!(source.contains("crypto_arc4_crypt"));
        assert!(source.contains("CRYPTO_LSKCIPHER_FLAG_CONT"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"ecb(arc4)\")"));

        let tfm = crypto_arc4_setkey(b"Key").expect("setkey");
        let mut state = Arc4Ctx::default();
        let ciphertext = crypto_arc4_crypt(&tfm, b"Plaintext", &mut state, 0).expect("encrypt");
        assert_eq!(hex(&ciphertext), b"bbf316e8d940af0ad3");

        let tfm = crypto_arc4_setkey(b"Key").expect("setkey");
        let mut decrypt_state = Arc4Ctx::default();
        let plaintext =
            crypto_arc4_crypt(&tfm, &ciphertext, &mut decrypt_state, 0).expect("decrypt");
        assert_eq!(plaintext, b"Plaintext");
    }

    #[test]
    fn arc4_continuation_reuses_siv_state_like_lskcipher() {
        let tfm = crypto_arc4_setkey(b"Wiki").expect("setkey");
        let mut whole_state = Arc4Ctx::default();
        let whole = crypto_arc4_crypt(&tfm, b"pedia", &mut whole_state, 0).expect("whole");

        let mut split_state = Arc4Ctx::default();
        let first = crypto_arc4_crypt(&tfm, b"pe", &mut split_state, 0).expect("first");
        let second = crypto_arc4_crypt(&tfm, b"dia", &mut split_state, CRYPTO_LSKCIPHER_FLAG_CONT)
            .expect("second");

        let mut split = first;
        split.extend_from_slice(&second);
        assert_eq!(split, whole);
        assert_eq!(crypto_arc4_setkey(&[]), Err(-EINVAL));
    }
}
