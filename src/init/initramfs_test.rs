//! linux-parity: partial
//! linux-source: vendor/linux/init/initramfs_test.c
//! test-origin: linux:vendor/linux/init/initramfs_test.c
//! Source-backed Rust coverage for Linux initramfs KUnit CPIO fixtures.

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

pub const CPIO_NEWC_MAGIC: &str = "070701";
pub const CPIO_CRC_MAGIC: &str = "070702";
pub const CPIO_HDR_OX_INJECT: &str =
    "%s%08x%08x0x%06x0X%06x%08x%08x%08x%08x%08x%08x%08x0x%06x%08x%s";
pub const CPIO_HDRLEN: usize = 110;
pub const PATH_MAX: usize = 4096;
pub const INITRAMFS_TEST_MANY_LIMIT: usize = 1000;
pub const INITRAMFS_TEST_MANY_PATH_MAX: usize = 26;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitramfsTestCpio<'a> {
    pub magic: &'a str,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u32,
    pub mtime: u32,
    pub filesize: u32,
    pub devmajor: u32,
    pub devminor: u32,
    pub rdevmajor: u32,
    pub rdevminor: u32,
    pub namesize: u32,
    pub csum: u32,
    pub fname: &'a str,
    pub data: &'a [u8],
}

impl<'a> InitramfsTestCpio<'a> {
    pub fn regular(fname: &'a str, data: &'a [u8]) -> Self {
        Self {
            magic: CPIO_NEWC_MAGIC,
            ino: 1,
            mode: 0o100777,
            uid: 0,
            gid: 0,
            nlink: 1,
            mtime: 1,
            filesize: data.len() as u32,
            devmajor: 0,
            devminor: 1,
            rdevmajor: 0,
            rdevminor: 0,
            namesize: fname.len() as u32 + 1,
            csum: 0,
            fname,
            data,
        }
    }
}

pub fn fill_cpio(entries: &[InitramfsTestCpio<'_>], inject_ox: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        let start = out.len();
        let header = if inject_ox {
            format!(
                "{}{:08x}{:08x}0x{:06x}0X{:06x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}0x{:06x}{:08x}{}",
                entry.magic,
                entry.ino,
                entry.mode,
                entry.uid,
                entry.gid,
                entry.nlink,
                entry.mtime,
                entry.filesize,
                entry.devmajor,
                entry.devminor,
                entry.rdevmajor,
                entry.rdevminor,
                entry.namesize,
                entry.csum,
                entry.fname
            )
        } else {
            format!(
                "{}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{}",
                entry.magic,
                entry.ino,
                entry.mode,
                entry.uid,
                entry.gid,
                entry.nlink,
                entry.mtime,
                entry.filesize,
                entry.devmajor,
                entry.devminor,
                entry.rdevmajor,
                entry.rdevminor,
                entry.namesize,
                entry.csum,
                entry.fname
            )
        };
        out.extend_from_slice(header.as_bytes());
        out.push(0);

        let name_end = start + CPIO_HDRLEN + entry.namesize as usize;
        if out.len() < name_end {
            out.resize(name_end, 0);
        }
        align_4(&mut out);

        out.extend_from_slice(entry.data);
        align_4(&mut out);
    }
    out
}

pub fn cpio_payload_csum(data: &[u8]) -> u32 {
    data.iter().map(|byte| *byte as u32).sum()
}

pub fn many_path_len(index: usize) -> usize {
    format!("initramfs_test_many-{index}").len() + 1
}

pub fn fname_pad_namesize() -> usize {
    4096 - CPIO_HDRLEN
}

fn align_4(out: &mut Vec<u8>) {
    while out.len() & 3 != 0 {
        out.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::initramfs::InitramfsImage;

    #[test]
    fn initramfs_test_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/initramfs_test.c"
        ));
        assert!(source.contains("struct initramfs_test_cpio"));
        assert!(source.contains("static size_t fill_cpio(struct initramfs_test_cpio *cs"));
        assert!(source.contains("#define CPIO_HDR_OX_INJECT"));
        assert!(source.contains("inject_ox ? CPIO_HDR_OX_INJECT : CPIO_HDR_FMT"));
        assert!(source.contains("off += CPIO_HDRLEN + c->namesize;"));
        assert!(source.contains("while (off & 3)"));
        assert!(source.contains("memcpy(&out[off], c->data, c->filesize);"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_extract)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_fname_overrun)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_data)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_csum)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_hardlink)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_many)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_fname_pad)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_fname_path_max)"));
        assert!(source.contains("KUNIT_CASE(initramfs_test_hdr_hex)"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Initramfs KUnit test suite\")"));
    }

    #[test]
    fn fill_cpio_builds_newc_archive_parseable_by_initramfs() {
        let file = InitramfsTestCpio::regular("initramfs_test_data", b"ASDF");
        let trailer = InitramfsTestCpio {
            magic: CPIO_NEWC_MAGIC,
            ino: 0,
            mode: 0,
            uid: 0,
            gid: 0,
            nlink: 0,
            mtime: 0,
            filesize: 0,
            devmajor: 0,
            devminor: 0,
            rdevmajor: 0,
            rdevminor: 0,
            namesize: "TRAILER!!!".len() as u32 + 1,
            csum: 0,
            fname: "TRAILER!!!",
            data: &[],
        };
        let image = fill_cpio(&[file, trailer], false);
        assert_eq!(&image[..6], b"070701");
        assert_eq!(image.len() & 3, 0);

        let leaked = alloc::boxed::Box::leak(image.into_boxed_slice());
        let parsed = InitramfsImage::parse(leaked).unwrap();
        assert_eq!(parsed.read_file("/initramfs_test_data"), Some(&b"ASDF"[..]));
    }

    #[test]
    fn crc_fixture_uses_payload_byte_sum_for_070702_entries() {
        let mut file = InitramfsTestCpio::regular("initramfs_test_csum", b"ASDF");
        file.magic = CPIO_CRC_MAGIC;
        file.csum = cpio_payload_csum(file.data);
        let image = fill_cpio(&[file], false);
        assert_eq!(&image[..6], b"070702");
        assert!(image.windows(8).any(|window| window == b"0000011e"));
        assert_eq!(
            file.csum,
            ('A' as u32) + ('S' as u32) + ('D' as u32) + ('F' as u32)
        );
    }

    #[test]
    fn padded_fname_places_file_data_at_4k_archive_offset() {
        let mut padded_name = [0u8; 4096 - CPIO_HDRLEN];
        padded_name[..13].copy_from_slice(b"padded_fname\0");
        let fname = core::str::from_utf8(&padded_name[..12]).unwrap();
        let file = InitramfsTestCpio {
            namesize: fname_pad_namesize() as u32,
            fname,
            data: b"this file data is aligned at 4K in the archive",
            filesize: b"this file data is aligned at 4K in the archive".len() as u32,
            ..InitramfsTestCpio::regular(fname, b"")
        };
        let image = fill_cpio(&[file], false);
        assert_eq!(CPIO_HDRLEN + file.namesize as usize, 4096);
        assert_eq!(
            &image[4096..4096 + file.filesize as usize],
            b"this file data is aligned at 4K in the archive"
        );
    }

    #[test]
    fn many_limit_path_budget_matches_source_formula() {
        assert_eq!(INITRAMFS_TEST_MANY_LIMIT, 1000);
        assert_eq!(INITRAMFS_TEST_MANY_PATH_MAX, 26);
        assert_eq!(many_path_len(999), "initramfs_test_many-999".len() + 1);
        assert!(many_path_len(INITRAMFS_TEST_MANY_LIMIT) <= PATH_MAX);
    }

    #[test]
    fn hdr_hex_fixture_rejects_ox_prefixed_header_fields() {
        let file = InitramfsTestCpio::regular("initramfs_test_hdr_hex", b"ASDF");
        let image = fill_cpio(&[file], true);
        let leaked = alloc::boxed::Box::leak(image.into_boxed_slice());
        assert!(matches!(
            InitramfsImage::parse(leaked),
            Err(crate::init::initramfs::InitramfsParseError::DamagedHeader)
        ));
    }
}
