//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/arc4.c
//! test-origin: linux:vendor/linux/lib/crypto/arc4.c
//! ARC4 state-machine helpers.

use crate::kernel::module::{export_symbol, find_symbol};

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

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("arc4_setkey", arc4_setkey_raw as usize, false);
    export_symbol_once("arc4_crypt", arc4_crypt_raw as usize, false);
}

pub fn arc4_setkey(ctx: &mut Arc4Ctx, in_key: &[u8]) {
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
}

pub fn arc4_crypt(ctx: &mut Arc4Ctx, out: &mut [u8], input: &[u8]) {
    if input.is_empty() {
        return;
    }
    assert!(out.len() >= input.len());

    let s = &mut ctx.s;
    let mut x = ctx.x as usize;
    let mut y = ctx.y as usize;
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
        out[idx] = byte ^ s[a as usize] as u8;
        if idx + 1 == input.len() {
            break;
        }
        y = ty;
        a = ta;
        b = tb;
    }

    ctx.x = x as u32;
    ctx.y = y as u32;
}

pub unsafe extern "C" fn arc4_setkey_raw(
    ctx: *mut Arc4Ctx,
    in_key: *const u8,
    key_len: u32,
) -> i32 {
    if ctx.is_null() || in_key.is_null() || key_len == 0 {
        return 0;
    }
    let key = unsafe { core::slice::from_raw_parts(in_key, key_len as usize) };
    unsafe { arc4_setkey(&mut *ctx, key) };
    0
}

pub unsafe extern "C" fn arc4_crypt_raw(
    ctx: *mut Arc4Ctx,
    out: *mut u8,
    input: *const u8,
    len: u32,
) {
    if ctx.is_null() || out.is_null() || input.is_null() {
        return;
    }
    let out = unsafe { core::slice::from_raw_parts_mut(out, len as usize) };
    let input = unsafe { core::slice::from_raw_parts(input, len as usize) };
    unsafe { arc4_crypt(&mut *ctx, out, input) };
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;

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
    fn arc4_matches_linux_state_machine_and_known_stream_vector() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/arc4.c"
        ));
        assert!(source.contains("ctx->x = 1;"));
        assert!(source.contains("ctx->S[i] = i;"));
        assert!(source.contains("j = (j + in_key[k] + a) & 0xff;"));
        assert!(source.contains("*out++ = *in++ ^ S[a];"));
        assert!(source.contains("EXPORT_SYMBOL(arc4_crypt);"));

        let mut ctx = Arc4Ctx::default();
        arc4_setkey(&mut ctx, b"Key");
        let mut out = [0u8; 9];
        arc4_crypt(&mut ctx, &mut out, b"Plaintext");
        assert_eq!(hex(&out), b"bbf316e8d940af0ad3");

        let mut decrypt = Arc4Ctx::default();
        arc4_setkey(&mut decrypt, b"Key");
        let mut plain = [0u8; 9];
        arc4_crypt(&mut decrypt, &mut plain, &out);
        assert_eq!(&plain, b"Plaintext");
    }
}
