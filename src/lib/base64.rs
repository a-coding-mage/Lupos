//! linux-parity: complete
//! linux-source: vendor/linux/lib/base64.c
//! test-origin: linux:vendor/linux/lib/base64.c
//! Base64 helpers with Linux's STD, URL-safe, and IMAP alphabets.

use crate::kernel::module::{export_symbol, find_symbol};

pub const BASE64_STD: i32 = 0;
pub const BASE64_URLSAFE: i32 = 1;
pub const BASE64_IMAP: i32 = 2;

const TABLES: [&[u8; 64]; 3] = [
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,",
];

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("base64_encode", base64_encode as usize, true);
    export_symbol_once("base64_decode", base64_decode as usize, true);
}

fn table_for(variant: i32) -> Option<&'static [u8; 64]> {
    TABLES.get(variant as usize).copied()
}

fn decode_byte(byte: u8, variant: i32) -> i32 {
    match byte {
        b'A'..=b'Z' => (byte - b'A') as i32,
        b'a'..=b'z' => (byte - b'a' + 26) as i32,
        b'0'..=b'9' => (byte - b'0' + 52) as i32,
        b'+' => {
            if variant == BASE64_STD || variant == BASE64_IMAP {
                62
            } else {
                -1
            }
        }
        b'/' => {
            if variant == BASE64_STD {
                63
            } else {
                -1
            }
        }
        b'-' => {
            if variant == BASE64_URLSAFE {
                62
            } else {
                -1
            }
        }
        b'_' => {
            if variant == BASE64_URLSAFE {
                63
            } else {
                -1
            }
        }
        b',' => {
            if variant == BASE64_IMAP {
                63
            } else {
                -1
            }
        }
        _ => -1,
    }
}

pub fn encode_slice(src: &[u8], dst: &mut [u8], padding: bool, variant: i32) -> Option<usize> {
    let table = table_for(variant)?;
    let mut sp = 0usize;
    let mut dp = 0usize;

    while src.len().saturating_sub(sp) >= 3 {
        if dst.len().saturating_sub(dp) < 4 {
            return None;
        }
        let ac = ((src[sp] as u32) << 16) | ((src[sp + 1] as u32) << 8) | src[sp + 2] as u32;
        dst[dp] = table[(ac >> 18) as usize];
        dst[dp + 1] = table[((ac >> 12) & 0x3f) as usize];
        dst[dp + 2] = table[((ac >> 6) & 0x3f) as usize];
        dst[dp + 3] = table[(ac & 0x3f) as usize];
        sp += 3;
        dp += 4;
    }

    match src.len() - sp {
        2 => {
            let need = if padding { 4 } else { 3 };
            if dst.len().saturating_sub(dp) < need {
                return None;
            }
            let ac = ((src[sp] as u32) << 16) | ((src[sp + 1] as u32) << 8);
            dst[dp] = table[(ac >> 18) as usize];
            dst[dp + 1] = table[((ac >> 12) & 0x3f) as usize];
            dst[dp + 2] = table[((ac >> 6) & 0x3f) as usize];
            dp += 3;
            if padding {
                dst[dp] = b'=';
                dp += 1;
            }
        }
        1 => {
            let need = if padding { 4 } else { 2 };
            if dst.len().saturating_sub(dp) < need {
                return None;
            }
            let ac = (src[sp] as u32) << 16;
            dst[dp] = table[(ac >> 18) as usize];
            dst[dp + 1] = table[((ac >> 12) & 0x3f) as usize];
            dp += 2;
            if padding {
                dst[dp] = b'=';
                dst[dp + 1] = b'=';
                dp += 2;
            }
        }
        _ => {}
    }

    Some(dp)
}

pub fn decode_slice(src: &[u8], dst: &mut [u8], mut padding: bool, variant: i32) -> Option<usize> {
    table_for(variant)?;
    let mut sp = 0usize;
    let mut srclen = src.len();
    let mut dp = 0usize;

    while srclen >= 4 {
        let input = [
            decode_byte(src[sp], variant),
            decode_byte(src[sp + 1], variant),
            decode_byte(src[sp + 2], variant),
            decode_byte(src[sp + 3], variant),
        ];
        let val = (input[0] << 18) | (input[1] << 12) | (input[2] << 6) | input[3];

        if val < 0 {
            if !padding || srclen != 4 || src[sp + 3] != b'=' {
                return None;
            }
            padding = false;
            srclen = if src[sp + 2] == b'=' { 2 } else { 3 };
            break;
        }

        if dst.len().saturating_sub(dp) < 3 {
            return None;
        }
        dst[dp] = (val >> 16) as u8;
        dst[dp + 1] = (val >> 8) as u8;
        dst[dp + 2] = val as u8;
        dp += 3;
        sp += 4;
        srclen -= 4;
    }

    if srclen == 0 {
        return Some(dp);
    }
    if padding || srclen == 1 || dst.len().saturating_sub(dp) < 1 {
        return None;
    }

    let mut val = (decode_byte(src[sp], variant) << 12) | (decode_byte(src[sp + 1], variant) << 6);
    dst[dp] = (val >> 10) as u8;
    dp += 1;

    if srclen == 2 {
        if (val & 0x8000_03ffu32 as i32) != 0 {
            return None;
        }
    } else {
        val |= decode_byte(src[sp + 2], variant);
        if (val & 0x8000_0003u32 as i32) != 0 {
            return None;
        }
        if dst.len().saturating_sub(dp) < 1 {
            return None;
        }
        dst[dp] = (val >> 2) as u8;
        dp += 1;
    }

    Some(dp)
}

pub unsafe extern "C" fn base64_encode(
    src: *const u8,
    srclen: i32,
    dst: *mut u8,
    padding: bool,
    variant: i32,
) -> i32 {
    if srclen < 0 || src.is_null() || dst.is_null() {
        return 0;
    }
    let src = unsafe { core::slice::from_raw_parts(src, srclen as usize) };
    let max_len = src.len().div_ceil(3) * 4;
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, max_len) };
    encode_slice(src, dst, padding, variant).unwrap_or(0) as i32
}

pub unsafe extern "C" fn base64_decode(
    src: *const u8,
    srclen: i32,
    dst: *mut u8,
    padding: bool,
    variant: i32,
) -> i32 {
    if srclen < 0 || src.is_null() || dst.is_null() {
        return -1;
    }
    let src = unsafe { core::slice::from_raw_parts(src, srclen as usize) };
    let max_len = (src.len() / 4) * 3 + 2;
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, max_len) };
    decode_slice(src, dst, padding, variant)
        .map(|len| len as i32)
        .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_ok(src: &[u8], expected: &str, padding: bool, variant: i32) {
        let mut out = [0u8; 128];
        let len = encode_slice(src, &mut out, padding, variant).expect("encode");
        assert_eq!(core::str::from_utf8(&out[..len]).unwrap(), expected);
    }

    fn decode_ok(src: &str, expected: &[u8], padding: bool, variant: i32) {
        let mut out = [0u8; 128];
        let len = decode_slice(src.as_bytes(), &mut out, padding, variant).expect("decode");
        assert_eq!(&out[..len], expected);
    }

    fn decode_err(src: &[u8], padding: bool, variant: i32) {
        let mut out = [0u8; 64];
        assert_eq!(decode_slice(src, &mut out, padding, variant), None);
    }

    #[test]
    fn linux_base64_kunit_vectors_pass() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/base64.c"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/base64_kunit.c"
        ));
        assert!(source.contains("BASE64_URLSAFE"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(base64_encode);"));
        assert!(kunit.contains("base64_std_encode_tests"));
        assert!(kunit.contains("base64_variant_tests"));

        encode_ok(b"", "", true, BASE64_STD);
        encode_ok(b"f", "Zg==", true, BASE64_STD);
        encode_ok(b"fo", "Zm8=", true, BASE64_STD);
        encode_ok(b"foo", "Zm9v", true, BASE64_STD);
        encode_ok(b"foob", "Zm9vYg==", true, BASE64_STD);
        encode_ok(b"fooba", "Zm9vYmE=", true, BASE64_STD);
        encode_ok(b"foobar", "Zm9vYmFy", true, BASE64_STD);
        encode_ok(b"Hello, world!", "SGVsbG8sIHdvcmxkIQ==", true, BASE64_STD);
        encode_ok(b"f", "Zg", false, BASE64_STD);
        encode_ok(b"fo", "Zm8", false, BASE64_STD);
        encode_ok(b"Hello, world!", "SGVsbG8sIHdvcmxkIQ", false, BASE64_STD);

        decode_ok("", b"", true, BASE64_STD);
        decode_ok("Zg==", b"f", true, BASE64_STD);
        decode_ok("Zm8=", b"fo", true, BASE64_STD);
        decode_ok("Zm9v", b"foo", true, BASE64_STD);
        decode_ok("Zm9vYmFy", b"foobar", false, BASE64_STD);
        decode_ok("SGVsbG8sIHdvcmxkIQ", b"Hello, world!", false, BASE64_STD);

        decode_err(b"Zg=!", true, BASE64_STD);
        decode_err(b"Zm$=", true, BASE64_STD);
        decode_err(b"Z===", true, BASE64_STD);
        decode_err(b"Zg", true, BASE64_STD);
        decode_err(b"Zg=", false, BASE64_STD);
        decode_err(&[b'Z', b'g', 0, b'='], false, BASE64_STD);
    }

    #[test]
    fn linux_base64_variants_match_alphabet_substitution() {
        let sample = [0x00, 0xfb, 0xff, 0x7f, 0x80];
        let mut std_buf = [0u8; 32];
        let mut url_buf = [0u8; 32];
        let mut imap_buf = [0u8; 32];
        let n_std = encode_slice(&sample, &mut std_buf, false, BASE64_STD).unwrap();
        let n_url = encode_slice(&sample, &mut url_buf, false, BASE64_URLSAFE).unwrap();
        let n_imap = encode_slice(&sample, &mut imap_buf, false, BASE64_IMAP).unwrap();

        for byte in &mut std_buf[..n_std] {
            if *byte == b'+' {
                *byte = b'-';
            } else if *byte == b'/' {
                *byte = b'_';
            }
        }
        assert_eq!(&std_buf[..n_std], &url_buf[..n_url]);

        let n_std = encode_slice(&sample, &mut std_buf, false, BASE64_STD).unwrap();
        for byte in &mut std_buf[..n_std] {
            if *byte == b'/' {
                *byte = b',';
            }
        }
        assert_eq!(&std_buf[..n_std], &imap_buf[..n_imap]);

        let mut back = [0u8; 16];
        assert_eq!(
            decode_slice(&url_buf[..n_url], &mut back, false, BASE64_URLSAFE),
            Some(sample.len())
        );
        assert_eq!(&back[..sample.len()], &sample);
        assert_eq!(
            decode_slice(&imap_buf[..n_imap], &mut back, false, BASE64_IMAP),
            Some(sample.len())
        );
        assert_eq!(&back[..sample.len()], &sample);
        decode_err(b"Zg==", false, BASE64_URLSAFE);

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("base64_decode"),
            Some(base64_decode as usize)
        );
    }
}
