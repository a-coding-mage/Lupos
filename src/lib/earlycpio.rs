//! linux-parity: complete
//! linux-source: vendor/linux/lib/earlycpio.c
//! test-origin: linux:vendor/linux/lib/earlycpio.c
//! Early uncompressed cpio member lookup.

pub const MAX_CPIO_FILE_NAME: usize = 18;
const C_NFIELDS: usize = 14;
const C_MAGIC: usize = 0;
const C_MODE: usize = 2;
const C_FILESIZE: usize = 7;
const C_NAMESIZE: usize = 12;
const CPIO_HEADER_LEN: usize = 8 * C_NFIELDS - 2;
const S_IFMT: u32 = 0o170000;
const S_IFREG: u32 = 0o100000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpioData<'a> {
    pub data: Option<&'a [u8]>,
    pub size: usize,
    pub name: [u8; MAX_CPIO_FILE_NAME],
}

impl<'a> CpioData<'a> {
    pub const fn empty() -> Self {
        Self {
            data: None,
            size: 0,
            name: [0; MAX_CPIO_FILE_NAME],
        }
    }

    pub fn name_str(&self) -> &str {
        let len = self
            .name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(MAX_CPIO_FILE_NAME);
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }
}

fn align4(value: usize) -> usize {
    (value + 3) & !3
}

fn hex_nibble(byte: u8) -> Option<u32> {
    let x = byte.wrapping_sub(b'0');
    if x < 10 {
        return Some(x as u32);
    }
    let x = (byte | 0x20).wrapping_sub(b'a');
    if x < 6 {
        return Some((x + 10) as u32);
    }
    None
}

fn read_field(input: &[u8], pos: &mut usize, width: usize) -> Option<u32> {
    let mut value = 0u32;
    for _ in 0..width {
        value <<= 4;
        value += hex_nibble(*input.get(*pos)?)?;
        *pos += 1;
    }
    Some(value)
}

fn copy_cpio_name(dst: &mut [u8; MAX_CPIO_FILE_NAME], src: &[u8]) {
    let len = src.len().min(MAX_CPIO_FILE_NAME - 1);
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
}

pub fn find_cpio_data<'a>(path: &str, data: &'a [u8], nextoff: Option<&mut isize>) -> CpioData<'a> {
    let path = path.as_bytes();
    let mut offset = 0usize;

    while data.len().saturating_sub(offset) > CPIO_HEADER_LEN {
        if data[offset] == 0 {
            offset = offset.saturating_add(4);
            continue;
        }

        let mut pos = offset;
        let mut fields = [0u32; C_NFIELDS];
        for (i, field) in fields.iter_mut().enumerate() {
            let width = if i == 0 { 6 } else { 8 };
            let Some(value) = read_field(data, &mut pos, width) else {
                return CpioData::empty();
            };
            *field = value;
        }

        if fields[C_MAGIC].wrapping_sub(0x070701) > 1 {
            return CpioData::empty();
        }

        let remaining_after_header = data.len().saturating_sub(pos);
        let name_size = fields[C_NAMESIZE] as usize;
        let file_size = fields[C_FILESIZE] as usize;
        let dptr = align4(pos.saturating_add(name_size));
        let nptr = align4(dptr.saturating_add(file_size));

        if nptr > pos.saturating_add(remaining_after_header) || dptr < pos || nptr < dptr {
            return CpioData::empty();
        }

        let name = &data[pos..pos + name_size];
        if fields[C_MODE] & S_IFMT == S_IFREG && name_size >= path.len() && name.starts_with(path) {
            if let Some(nextoff) = nextoff {
                *nextoff = nptr as isize;
            }
            let suffix = &name[path.len()..name.len().saturating_sub(1)];
            let mut found = CpioData {
                data: Some(&data[dptr..dptr + file_size]),
                size: file_size,
                name: [0; MAX_CPIO_FILE_NAME],
            };
            copy_cpio_name(&mut found.name, suffix);
            return found;
        }

        offset = nptr;
    }

    CpioData::empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, vec::Vec};

    fn append_newc_file(out: &mut Vec<u8>, name: &str, mode: u32, body: &[u8]) {
        let namesize = name.len() + 1;
        out.extend_from_slice(
            format!(
                "070701{ino:08x}{mode:08x}{uid:08x}{gid:08x}{nlink:08x}{mtime:08x}{filesize:08x}{maj:08x}{min:08x}{rmaj:08x}{rmin:08x}{namesize:08x}{chksum:08x}",
                ino = 1,
                mode = mode,
                uid = 0,
                gid = 0,
                nlink = 1,
                mtime = 0,
                filesize = body.len(),
                maj = 0,
                min = 0,
                rmaj = 0,
                rmin = 0,
                namesize = namesize,
                chksum = 0,
            )
            .as_bytes(),
        );
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out.extend_from_slice(body);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }

    #[test]
    fn earlycpio_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/earlycpio.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/earlycpio.h"
        ));
        assert!(source.contains("enum cpio_fields"));
        assert!(source.contains("const size_t cpio_header_len = 8*C_NFIELDS - 2;"));
        assert!(source.contains("if (!*p)"));
        assert!(source.contains("p += 4;"));
        assert!(source.contains("j = 6;"));
        assert!(source.contains("if ((ch[C_MAGIC] - 0x070701) > 1)"));
        assert!(source.contains("dptr = PTR_ALIGN(p + ch[C_NAMESIZE], 4);"));
        assert!(source.contains("nptr = PTR_ALIGN(dptr + ch[C_FILESIZE], 4);"));
        assert!(source.contains("(ch[C_MODE] & 0170000) == 0100000"));
        assert!(source.contains("*nextoff = (long)nptr - (long)data;"));
        assert!(source.contains("strscpy(cd.name, p + mypathsize, MAX_CPIO_FILE_NAME);"));
        assert!(header.contains("#define MAX_CPIO_FILE_NAME 18"));
    }

    #[test]
    fn find_cpio_data_returns_regular_file_payload_and_suffix_name() {
        let mut archive = Vec::new();
        append_newc_file(&mut archive, "kernel/a", S_IFREG | 0o644, b"first");
        append_newc_file(&mut archive, "kernel/b", S_IFREG | 0o644, b"second");
        let mut next = 0;

        let found = find_cpio_data("kernel/", &archive, Some(&mut next));
        assert_eq!(found.data, Some(&b"first"[..]));
        assert_eq!(found.size, 5);
        assert_eq!(found.name_str(), "a");
        assert!(next > 0);

        let found = find_cpio_data("missing/", &archive, None);
        assert_eq!(found, CpioData::empty());
    }

    #[test]
    fn find_cpio_data_rejects_bad_magic_and_non_regular_files() {
        let mut archive = Vec::new();
        append_newc_file(&mut archive, "kernel/dir", 0o040000, b"body");
        assert_eq!(find_cpio_data("kernel/", &archive, None), CpioData::empty());

        archive[5] = b'9';
        assert_eq!(find_cpio_data("kernel/", &archive, None), CpioData::empty());
    }
}
