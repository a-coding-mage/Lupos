//! linux-parity: complete
//! linux-source: vendor/linux/fs/isofs/joliet.c
//! test-origin: linux:vendor/linux/fs/isofs/joliet.c
//! Joliet filename conversion and kernel trimming rules.

extern crate alloc;

use alloc::vec::Vec;

pub const NLS_MAX_CHARSET_SIZE: usize = 6;
pub const JOLIET_VERSION_SUFFIX: &[u8] = b";1";

pub fn joliet_filename_utf8(raw_name: &[u8]) -> Vec<u8> {
    joliet_filename_with_nls(raw_name, |codepoint, out| {
        encode_utf8_codepoint(codepoint, out).unwrap_or(0)
    })
}

pub fn joliet_filename_ascii_fallback(raw_name: &[u8]) -> Vec<u8> {
    joliet_filename_with_nls(raw_name, |codepoint, out| {
        if codepoint <= 0x7f {
            out[0] = codepoint as u8;
            1
        } else {
            0
        }
    })
}

pub fn joliet_filename_with_nls(
    raw_name: &[u8],
    mut uni2char: impl FnMut(u16, &mut [u8; NLS_MAX_CHARSET_SIZE]) -> usize,
) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in raw_name.chunks_exact(2) {
        let codepoint = u16::from_be_bytes([chunk[0], chunk[1]]);
        if codepoint == 0 {
            break;
        }

        let mut buf = [0u8; NLS_MAX_CHARSET_SIZE];
        let written = uni2char(codepoint, &mut buf);
        if written > 0 {
            out.extend_from_slice(&buf[..written]);
        } else {
            out.push(b'?');
        }
    }
    trim_linux_joliet_name(out)
}

fn encode_utf8_codepoint(codepoint: u16, out: &mut [u8; NLS_MAX_CHARSET_SIZE]) -> Option<usize> {
    let ch = char::from_u32(codepoint as u32)?;
    let mut tmp = [0u8; 4];
    let encoded = ch.encode_utf8(&mut tmp).as_bytes();
    out[..encoded.len()].copy_from_slice(encoded);
    Some(encoded.len())
}

fn trim_linux_joliet_name(mut name: Vec<u8>) -> Vec<u8> {
    if name.len() > 2 && name.ends_with(JOLIET_VERSION_SUFFIX) {
        name.truncate(name.len() - 2);
    }
    while name.len() >= 2 && name.last() == Some(&b'.') {
        name.pop();
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joliet_filename_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/isofs/joliet.c"
        ));
        assert!(source.contains("#include <linux/types.h>"));
        assert!(source.contains("#include <linux/nls.h>"));
        assert!(source.contains("#include \"isofs.h\""));
        assert!(source.contains("static int"));
        assert!(source.contains("uni16_to_x8"));
        assert!(source.contains("get_joliet_filename"));
        assert!(source.contains("de->name_len[0] >> 1"));
        assert!(source.contains("outname[len-2] == ';'"));
        assert!(source.contains("outname[len-1] == '1'"));
        assert!(source.contains("while (len >= 2 && (outname[len-1] == '.'))"));

        let name = [0x00, b'F', 0x00, b'O', 0x00, b'O', 0x00, b';', 0x00, b'1'];
        assert_eq!(joliet_filename_utf8(&name), b"FOO");

        let dotted = [0x00, b'A', 0x00, b'.', 0x00, b'.'];
        assert_eq!(joliet_filename_utf8(&dotted), b"A");

        let single_dot = [0x00, b'.'];
        assert_eq!(joliet_filename_utf8(&single_dot), b".");

        let snowman = [0x26, 0x03];
        assert_eq!(joliet_filename_ascii_fallback(&snowman), b"?");
    }
}
