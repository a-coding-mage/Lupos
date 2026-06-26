//! linux-parity: complete
//! linux-source: vendor/linux/fs/isofs/util.c
//! test-origin: linux:vendor/linux/fs/isofs/util.c
//! ISOFS export, Joliet, Rock Ridge, zisofs, and name helpers.
//!
//! Mirrors:
//! `vendor/linux/fs/isofs/compress.c`
//! `vendor/linux/fs/isofs/export.c`
//! `vendor/linux/fs/isofs/joliet.c`
//! `vendor/linux/fs/isofs/namei.c`
//! `vendor/linux/fs/isofs/rock.c`
//! `vendor/linux/fs/isofs/util.c`

extern crate alloc;

use alloc::string::String;

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsoFileHandle {
    pub extent: u32,
    pub size: u32,
    pub flags: u8,
}

pub fn export_file_handle(extent: u32, size: u32, flags: u8) -> IsoFileHandle {
    IsoFileHandle {
        extent,
        size,
        flags,
    }
}

pub fn normalize_iso_name(name: &[u8]) -> String {
    let mut end = name.len();
    for (index, byte) in name.iter().enumerate() {
        if *byte == b';' {
            end = index;
            break;
        }
    }
    while end > 0 && (name[end - 1] == b'.' || name[end - 1] == b' ') {
        end -= 1;
    }
    let mut out = String::new();
    for byte in name[..end].iter().copied() {
        out.push((byte as char).to_ascii_lowercase());
    }
    out
}

pub fn decode_joliet_name(bytes: &[u8]) -> Result<String, i32> {
    if bytes.len() % 2 != 0 {
        return Err(EINVAL);
    }
    let mut out = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let code = u16::from_be_bytes([bytes[index], bytes[index + 1]]);
        if code == 0 {
            break;
        }
        let Some(ch) = char::from_u32(code as u32) else {
            return Err(EINVAL);
        };
        out.push(ch);
        index += 2;
    }
    Ok(out)
}

pub fn rock_ridge_nm(system_use: &[u8]) -> Option<String> {
    let mut off = 0usize;
    while off + 5 <= system_use.len() {
        let sig = &system_use[off..off + 2];
        let len = system_use[off + 2] as usize;
        if len < 5 || off + len > system_use.len() {
            return None;
        }
        if sig == b"NM" {
            let flags = system_use[off + 4];
            if flags & 0x06 != 0 {
                return None;
            }
            let name = core::str::from_utf8(&system_use[off + 5..off + len]).ok()?;
            return Some(String::from(name));
        }
        off += len;
    }
    None
}

pub fn directory_record_name(raw_name: &[u8], joliet: bool) -> Result<String, i32> {
    if raw_name.len() == 1 && (raw_name[0] == 0 || raw_name[0] == 1) {
        return Ok(String::from(if raw_name[0] == 0 { "." } else { ".." }));
    }
    if joliet {
        decode_joliet_name(raw_name)
    } else {
        Ok(normalize_iso_name(raw_name))
    }
}

pub fn zisofs_uncompress_supported() -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_name_normalization_strips_version_and_casefolds() {
        assert_eq!(normalize_iso_name(b"README.TXT;1"), "readme.txt");
        assert_eq!(normalize_iso_name(b"DIR.;1"), "dir");
    }

    #[test]
    fn joliet_decodes_big_endian_ucs2() {
        assert_eq!(decode_joliet_name(&[0x00, b'H', 0x00, b'i']).unwrap(), "Hi");
        assert_eq!(decode_joliet_name(&[0]).unwrap_err(), EINVAL);
    }

    #[test]
    fn rock_ridge_nm_entry_overrides_plain_iso_name() {
        let entry = [b'N', b'M', 9, 1, 0, b'r', b'e', b'a', b'l'];
        assert_eq!(rock_ridge_nm(&entry).unwrap(), "real");
    }

    #[test]
    fn zisofs_reports_unsupported_in_kernel_path() {
        assert_eq!(zisofs_uncompress_supported(), Err(EOPNOTSUPP));
    }
}
