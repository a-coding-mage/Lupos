//! linux-parity: complete
//! linux-source: vendor/linux/fs/nls/nls_utf8.c
//! test-origin: linux:vendor/linux/fs/nls/nls_utf8.c
//! UTF-8 NLS table conversion behavior.

use crate::include::uapi::errno::{EINVAL, ENAMETOOLONG};

pub const NLS_UTF8_CHARSET: &str = "utf8";
pub const MAX_WCHAR_T: u32 = 0xffff;
pub const UTF8_REPLACEMENT_FALLBACK: u16 = 0x003f;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Uni2CharOutcome {
    pub result: i32,
    pub bytes: [u8; 4],
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Char2UniOutcome {
    pub result: i32,
    pub uni: u16,
}

pub fn nls_utf8_uni2char(uni: u32, boundlen: usize) -> Uni2CharOutcome {
    if boundlen == 0 {
        return Uni2CharOutcome {
            result: -ENAMETOOLONG,
            bytes: [0; 4],
            len: 0,
        };
    }

    let Some(ch) = char::from_u32(uni) else {
        return Uni2CharOutcome {
            result: -EINVAL,
            bytes: [b'?', 0, 0, 0],
            len: 1,
        };
    };

    let mut bytes = [0u8; 4];
    let encoded = ch.encode_utf8(&mut bytes);
    let len = encoded.len();
    if len > boundlen {
        return Uni2CharOutcome {
            result: -EINVAL,
            bytes: [b'?', 0, 0, 0],
            len: 1,
        };
    }

    Uni2CharOutcome {
        result: len as i32,
        bytes,
        len,
    }
}

pub fn nls_utf8_char2uni(rawstring: &[u8], boundlen: usize) -> Char2UniOutcome {
    match decode_utf8_first(rawstring, boundlen) {
        Some((u, len)) if u <= MAX_WCHAR_T => Char2UniOutcome {
            result: len as i32,
            uni: u as u16,
        },
        _ => Char2UniOutcome {
            result: -EINVAL,
            uni: UTF8_REPLACEMENT_FALLBACK,
        },
    }
}

pub const fn nls_utf8_identity(byte: u8) -> u8 {
    byte
}

fn decode_utf8_first(rawstring: &[u8], boundlen: usize) -> Option<(u32, usize)> {
    let limit = core::cmp::min(rawstring.len(), boundlen);
    if limit == 0 {
        return None;
    }

    let first = rawstring[0];
    let (needed, mut value, min_value) = match first {
        0x00..=0x7f => return Some((first as u32, 1)),
        0xc2..=0xdf => (2, (first & 0x1f) as u32, 0x80),
        0xe0..=0xef => (3, (first & 0x0f) as u32, 0x800),
        0xf0..=0xf4 => (4, (first & 0x07) as u32, 0x10000),
        _ => return None,
    };
    if limit < needed {
        return None;
    }
    for &byte in &rawstring[1..needed] {
        if byte & 0xc0 != 0x80 {
            return None;
        }
        value = (value << 6) | (byte & 0x3f) as u32;
    }
    if value < min_value || value > 0x10ffff || (0xd800..=0xdfff).contains(&value) {
        return None;
    }
    Some((value, needed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nls_utf8_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nls/nls_utf8.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/nls.h>"));
        assert!(source.contains("static unsigned char identity[256];"));
        assert!(source.contains("static int uni2char"));
        assert!(source.contains("return -ENAMETOOLONG;"));
        assert!(source.contains("*out = '?';"));
        assert!(source.contains("static int char2uni"));
        assert!(source.contains("u > MAX_WCHAR_T"));
        assert!(source.contains(".charset\t= \"utf8\""));
        assert!(source.contains(".charset2lower\t= identity"));
        assert!(source.contains("MODULE_DESCRIPTION(\"NLS UTF-8\")"));

        let ascii = nls_utf8_uni2char(b'A' as u32, 4);
        assert_eq!(ascii.result, 1);
        assert_eq!(ascii.bytes[0], b'A');
        assert_eq!(nls_utf8_uni2char(b'A' as u32, 0).result, -ENAMETOOLONG);
        assert_eq!(nls_utf8_uni2char(0xd800, 4).result, -EINVAL);

        let pi = nls_utf8_char2uni("pi: π".as_bytes(), 8);
        assert_eq!(pi.result, 1);
        assert_eq!(pi.uni, b'p' as u16);
        assert_eq!(
            nls_utf8_char2uni("😀".as_bytes(), 4),
            Char2UniOutcome {
                result: -EINVAL,
                uni: UTF8_REPLACEMENT_FALLBACK,
            }
        );
        assert_eq!(nls_utf8_identity(0xa5), 0xa5);
    }
}
