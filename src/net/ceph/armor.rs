//! linux-parity: complete
//! linux-source: vendor/linux/net/ceph/armor.c
//! test-origin: linux:vendor/linux/net/ceph/armor.c
//! Ceph base64 armor and unarmor helpers.

use crate::include::uapi::errno::EINVAL;

pub const PEM_KEY: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub const fn encode_bits(c: u8) -> u8 {
    PEM_KEY[c as usize]
}

pub const fn decode_bits(c: u8) -> Result<u8, i32> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        b'=' => Ok(0),
        _ => Err(-EINVAL),
    }
}

pub fn ceph_armor(src: &[u8]) -> alloc::vec::Vec<u8> {
    let mut dst = alloc::vec::Vec::new();
    let mut line = 0usize;
    let mut i = 0usize;
    while i < src.len() {
        let a = src[i];
        i += 1;
        dst.push(encode_bits(a >> 2));
        if i < src.len() {
            let b = src[i];
            i += 1;
            dst.push(encode_bits(((a & 3) << 4) | (b >> 4)));
            if i < src.len() {
                let c = src[i];
                i += 1;
                dst.push(encode_bits(((b & 15) << 2) | (c >> 6)));
                dst.push(encode_bits(c & 63));
            } else {
                dst.push(encode_bits((b & 15) << 2));
                dst.push(b'=');
            }
        } else {
            dst.push(encode_bits((a & 3) << 4));
            dst.push(b'=');
            dst.push(b'=');
        }
        line += 4;
        if line == 64 {
            line = 0;
            dst.push(b'\n');
        }
    }
    dst
}

extern crate alloc;

pub fn ceph_unarmor(src: &[u8]) -> Result<alloc::vec::Vec<u8>, i32> {
    let mut dst = alloc::vec::Vec::new();
    let mut i = 0usize;
    while i < src.len() {
        if src[i] == b'\n' {
            i += 1;
            continue;
        }
        if i + 4 > src.len() {
            return Err(-EINVAL);
        }
        let a = decode_bits(src[i])?;
        let b = decode_bits(src[i + 1])?;
        let c = decode_bits(src[i + 2])?;
        let d = decode_bits(src[i + 3])?;
        dst.push((a << 2) | (b >> 4));
        if src[i + 2] == b'=' {
            return Ok(dst);
        }
        dst.push(((b & 15) << 4) | (c >> 2));
        if src[i + 3] == b'=' {
            return Ok(dst);
        }
        dst.push(((c & 3) << 6) | d);
        i += 4;
    }
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_armor_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ceph/armor.c"
        ));
        assert!(source.contains("static const char *pem_key"));
        assert!(source.contains("static int encode_bits(int c)"));
        assert!(source.contains("static int decode_bits(char c)"));
        assert!(source.contains("if (c >= 'A' && c <= 'Z')"));
        assert!(source.contains("if (c == '=')"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("int ceph_armor(char *dst"));
        assert!(source.contains("if (line == 64)"));
        assert!(source.contains("*(dst++) = '\\n';"));
        assert!(source.contains("int ceph_unarmor(char *dst"));
        assert!(source.contains("if (src[0] == '\\n')"));
        assert!(source.contains("if (src + 4 > end)"));
        assert!(source.contains("if (src[2] == '=')"));
        assert!(source.contains("if (src[3] == '=')"));
    }

    #[test]
    fn armor_round_trips_and_rejects_bad_input() {
        assert_eq!(ceph_armor(b"f"), b"Zg==");
        assert_eq!(ceph_armor(b"fo"), b"Zm8=");
        assert_eq!(ceph_armor(b"foo"), b"Zm9v");
        let wrapped = ceph_armor(&[0u8; 48]);
        assert_eq!(wrapped[64], b'\n');
        assert_eq!(ceph_unarmor(&wrapped).unwrap(), alloc::vec![0u8; 48]);
        assert_eq!(ceph_unarmor(b"Zm9v").unwrap(), b"foo");
        assert_eq!(ceph_unarmor(b"\nZm9v").unwrap(), b"foo");
        assert_eq!(ceph_unarmor(b"abc").unwrap_err(), -EINVAL);
        assert_eq!(ceph_unarmor(b"ab?=").unwrap_err(), -EINVAL);
    }
}
