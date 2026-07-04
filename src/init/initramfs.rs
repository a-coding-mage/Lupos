//! linux-parity: partial
//! linux-source: vendor/linux/init/initramfs.c
//! linux-source: vendor/linux/init/initramfs_internal.h
//! test-origin: linux:vendor/linux/init/initramfs.c
//! Minimal read-only initramfs support for `execve` path lookups.
//!
//! This parser indexes the `newc` CPIO format consumed by
//! `unpack_to_rootfs`; `rootfs::materialize_initramfs` mirrors the filesystem
//! creation side. Streaming unpack and decompression are intentionally deferred.

extern crate alloc;

use alloc::{string::String, vec::Vec};
use spin::Mutex;

const CPIO_BIN_MAGIC: &[u8; 6] = b"070707";
const CPIO_NEWC_MAGIC: &[u8; 6] = b"070701";
const CPIO_CRC_MAGIC: &[u8; 6] = b"070702";
// Linux `init/initramfs_internal.h`: CPIO_HDRLEN.
const CPIO_HEADER_LEN: usize = 110;
const CPIO_TRAILER: &str = "TRAILER!!!";
const PATH_MAX: usize = 4096;

use crate::include::uapi::stat::{
    S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitramfsParseError {
    IncorrectCpioMethod,
    NoCpioMagic,
    DamagedHeader,
    BadDataChecksum,
}

impl InitramfsParseError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IncorrectCpioMethod => "incorrect cpio method used: use -H newc option",
            Self::NoCpioMagic => "no cpio magic",
            Self::DamagedHeader => "damaged header",
            Self::BadDataChecksum => "bad data checksum",
        }
    }

    pub const fn errno(self) -> i32 {
        -8
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitramfsEntry {
    pub path: String,
    pub mode: u32,
    ino: u32,
    uid: u32,
    gid: u32,
    nlink: u32,
    mtime: u32,
    dev_major: u32,
    dev_minor: u32,
    rdev_major: u32,
    rdev_minor: u32,
    hdr_csum: u32,
    data_offset: usize,
    size: usize,
}

impl InitramfsEntry {
    #[inline]
    pub fn is_regular_file(&self) -> bool {
        self.mode & S_IFMT == S_IFREG
    }

    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.mode & S_IFMT == S_IFLNK
    }

    #[inline]
    pub fn is_dir(&self) -> bool {
        self.mode & S_IFMT == S_IFDIR
    }

    #[inline]
    pub fn is_chardev(&self) -> bool {
        self.mode & S_IFMT == S_IFCHR
    }

    #[inline]
    pub fn is_blockdev(&self) -> bool {
        self.mode & S_IFMT == S_IFBLK
    }

    #[inline]
    pub fn is_fifo(&self) -> bool {
        self.mode & S_IFMT == S_IFIFO
    }

    #[inline]
    pub fn is_socket(&self) -> bool {
        self.mode & S_IFMT == S_IFSOCK
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn uid(&self) -> u32 {
        self.uid
    }

    #[inline]
    pub fn gid(&self) -> u32 {
        self.gid
    }

    #[inline]
    pub fn nlink(&self) -> u32 {
        self.nlink
    }

    #[inline]
    pub fn mtime(&self) -> u32 {
        self.mtime
    }

    #[inline]
    pub fn ino(&self) -> u32 {
        self.ino
    }

    #[inline]
    pub fn dev_major(&self) -> u32 {
        self.dev_major
    }

    #[inline]
    pub fn dev_minor(&self) -> u32 {
        self.dev_minor
    }

    #[inline]
    pub fn rdev_major(&self) -> u32 {
        self.rdev_major
    }

    #[inline]
    pub fn rdev_minor(&self) -> u32 {
        self.rdev_minor
    }

    #[inline]
    pub fn hdr_csum(&self) -> u32 {
        self.hdr_csum
    }

    #[inline]
    pub fn link_key(&self) -> (u32, u32, u32) {
        (self.dev_major, self.dev_minor, self.ino)
    }
}

#[derive(Clone)]
pub struct InitramfsImage {
    bytes: &'static [u8],
    entries: Vec<InitramfsEntry>,
}

impl InitramfsImage {
    pub fn parse(bytes: &'static [u8]) -> Result<Self, InitramfsParseError> {
        let mut entries = Vec::new();
        let mut off = 0usize;

        while off + CPIO_HEADER_LEN <= bytes.len() {
            let hdr = &bytes[off..off + CPIO_HEADER_LEN];
            if &hdr[0..6] == CPIO_BIN_MAGIC {
                return Err(InitramfsParseError::IncorrectCpioMethod);
            }
            if &hdr[0..6] != CPIO_NEWC_MAGIC && &hdr[0..6] != CPIO_CRC_MAGIC {
                return Err(InitramfsParseError::NoCpioMagic);
            }
            let crc = &hdr[0..6] == CPIO_CRC_MAGIC;

            let ino = parse_hex_u32(&hdr[6..14]).ok_or(InitramfsParseError::DamagedHeader)?;
            let mode = parse_hex_u32(&hdr[14..22]).ok_or(InitramfsParseError::DamagedHeader)?;
            let uid = parse_hex_u32(&hdr[22..30]).ok_or(InitramfsParseError::DamagedHeader)?;
            let gid = parse_hex_u32(&hdr[30..38]).ok_or(InitramfsParseError::DamagedHeader)?;
            let nlink = parse_hex_u32(&hdr[38..46]).ok_or(InitramfsParseError::DamagedHeader)?;
            let mtime = parse_hex_u32(&hdr[46..54]).ok_or(InitramfsParseError::DamagedHeader)?;
            let filesize =
                parse_hex_u32(&hdr[54..62]).ok_or(InitramfsParseError::DamagedHeader)? as usize;
            let dev_major =
                parse_hex_u32(&hdr[62..70]).ok_or(InitramfsParseError::DamagedHeader)?;
            let dev_minor =
                parse_hex_u32(&hdr[70..78]).ok_or(InitramfsParseError::DamagedHeader)?;
            let rdev_major =
                parse_hex_u32(&hdr[78..86]).ok_or(InitramfsParseError::DamagedHeader)?;
            let rdev_minor =
                parse_hex_u32(&hdr[86..94]).ok_or(InitramfsParseError::DamagedHeader)?;
            let namesize =
                parse_hex_u32(&hdr[94..102]).ok_or(InitramfsParseError::DamagedHeader)? as usize;
            let hdr_csum =
                parse_hex_u32(&hdr[102..110]).ok_or(InitramfsParseError::DamagedHeader)?;
            if namesize == 0 || namesize > PATH_MAX {
                return Err(InitramfsParseError::DamagedHeader);
            }

            let name_start = off + CPIO_HEADER_LEN;
            let name_end = name_start
                .checked_add(namesize)
                .ok_or(InitramfsParseError::DamagedHeader)?;
            if name_end > bytes.len() {
                return Err(InitramfsParseError::DamagedHeader);
            }
            let raw_name = &bytes[name_start..name_end - 1]; // excludes trailing NUL
            let name =
                core::str::from_utf8(raw_name).map_err(|_| InitramfsParseError::DamagedHeader)?;
            let data_start = align_up(name_end, 4);
            let data_end = data_start
                .checked_add(filesize)
                .ok_or(InitramfsParseError::DamagedHeader)?;
            if data_end > bytes.len() {
                return Err(InitramfsParseError::DamagedHeader);
            }
            if crc && payload_csum(&bytes[data_start..data_end]) != hdr_csum {
                return Err(InitramfsParseError::BadDataChecksum);
            }

            if str_eq(name, CPIO_TRAILER) {
                break;
            }

            let norm = normalize_path(name);
            entries.push(InitramfsEntry {
                path: norm,
                mode,
                ino,
                uid,
                gid,
                nlink,
                mtime,
                dev_major,
                dev_minor,
                rdev_major,
                rdev_minor,
                hdr_csum,
                data_offset: data_start,
                size: filesize,
            });

            off = align_up(data_end, 4);
        }

        propagate_newc_hardlink_payloads(&mut entries);
        Ok(Self { bytes, entries })
    }

    pub fn entries(&self) -> &[InitramfsEntry] {
        &self.entries
    }

    pub fn find(&self, path: &str) -> Option<&InitramfsEntry> {
        let wanted = normalize_path(path);
        self.entries.iter().find(|e| str_eq(&e.path, &wanted))
    }

    pub fn read_file(&self, path: &str) -> Option<&'static [u8]> {
        self.read_file_inner(path, 0)
    }

    fn read_file_inner(&self, path: &str, depth: usize) -> Option<&'static [u8]> {
        if depth > 40 {
            return None;
        }
        let e = self.find(path)?;
        if e.is_symlink() {
            let target =
                core::str::from_utf8(&self.bytes[e.data_offset..e.data_offset + e.size]).ok()?;
            return self.read_file_inner(target, depth + 1);
        }
        if !e.is_regular_file() {
            return None;
        }
        Some(&self.bytes[e.data_offset..e.data_offset + e.size])
    }

    pub fn read_link(&self, path: &str) -> Option<&'static str> {
        let e = self.find(path)?;
        if !e.is_symlink() {
            return None;
        }
        core::str::from_utf8(&self.bytes[e.data_offset..e.data_offset + e.size]).ok()
    }
}

fn propagate_newc_hardlink_payloads(entries: &mut [InitramfsEntry]) {
    let mut i = 0usize;
    while i < entries.len() {
        if entries[i].nlink >= 2 && entries[i].is_regular_file() && entries[i].size != 0 {
            let key = entries[i].link_key();
            let mode_kind = entries[i].mode & S_IFMT;
            let data_offset = entries[i].data_offset;
            let size = entries[i].size;
            let mut j = 0usize;
            while j < entries.len() {
                if entries[j].nlink >= 2
                    && entries[j].is_regular_file()
                    && entries[j].link_key() == key
                    && entries[j].mode & S_IFMT == mode_kind
                    && entries[j].size == 0
                {
                    entries[j].data_offset = data_offset;
                    entries[j].size = size;
                }
                j += 1;
            }
        }
        i += 1;
    }
}

fn normalize_path(path: &str) -> String {
    let p = path.trim();
    if p.is_empty() {
        return "/".into();
    }
    if p.starts_with('/') {
        p.into()
    } else {
        let mut s = String::with_capacity(p.len() + 1);
        s.push('/');
        s.push_str(p);
        s
    }
}

fn str_eq(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    if ab.len() != bb.len() {
        return false;
    }
    let mut i = 0usize;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[inline]
const fn align_up(v: usize, align: usize) -> usize {
    (v + (align - 1)) & !(align - 1)
}

fn parse_hex_u32(buf: &[u8]) -> Option<u32> {
    let s = core::str::from_utf8(buf).ok()?;
    u32::from_str_radix(s, 16).ok()
}

fn payload_csum(data: &[u8]) -> u32 {
    data.iter()
        .fold(0u32, |sum, byte| sum.wrapping_add(*byte as u32))
}

static INITRAMFS: Mutex<Option<InitramfsImage>> = Mutex::new(None);

pub fn install(image: InitramfsImage) {
    *INITRAMFS.lock() = Some(image);
}

pub fn install_from_bytes(bytes: &'static [u8]) -> Result<(), i32> {
    let image = InitramfsImage::parse(bytes).map_err(InitramfsParseError::errno)?;
    install(image);
    Ok(())
}

pub fn is_installed() -> bool {
    INITRAMFS.lock().is_some()
}

#[cfg(test)]
pub fn reset_for_tests() {
    *INITRAMFS.lock() = None;
}

pub fn installed_entries() -> Result<Vec<InitramfsEntry>, i32> {
    let guard = INITRAMFS.lock();
    let image = guard.as_ref().ok_or(-2)?; // ENOENT
    Ok(image.entries.clone())
}

pub fn read_file(path: &str) -> Result<Vec<u8>, i32> {
    let guard = INITRAMFS.lock();
    let image = guard.as_ref().ok_or(-2)?; // ENOENT
    let data = image.read_file(path).ok_or(-2)?;
    Ok(data.to_vec())
}

pub fn read_file_slice(path: &str) -> Result<&'static [u8], i32> {
    let guard = INITRAMFS.lock();
    let image = guard.as_ref().ok_or(-2)?; // ENOENT
    image.read_file(path).ok_or(-2)
}

pub fn read_link(path: &str) -> Result<String, i32> {
    let guard = INITRAMFS.lock();
    let image = guard.as_ref().ok_or(-2)?; // ENOENT
    let target = image.read_link(path).ok_or(-22)?; // EINVAL
    Ok(target.into())
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::include::uapi::stat::{S_IFCHR, S_IFDIR, S_IFIFO};
    use alloc::boxed::Box;
    use alloc::vec;

    #[derive(Clone, Copy)]
    struct HeaderSpec<'a> {
        name: &'a str,
        mode: u32,
        ino: u32,
        uid: u32,
        gid: u32,
        nlink: u32,
        mtime: u32,
        dev_major: u32,
        dev_minor: u32,
        rdev_major: u32,
        rdev_minor: u32,
        csum: u32,
        payload: &'a [u8],
        crc: bool,
    }

    fn append_header_with_mtime(
        out: &mut Vec<u8>,
        name: &str,
        mode: u32,
        mtime: u32,
        payload: &[u8],
    ) {
        append_header_full(
            out,
            HeaderSpec {
                name,
                mode,
                ino: 0,
                uid: 0,
                gid: 0,
                nlink: 1,
                mtime,
                dev_major: 0,
                dev_minor: 0,
                rdev_major: 0,
                rdev_minor: 0,
                csum: 0,
                payload,
                crc: false,
            },
        );
    }

    fn append_header_full(out: &mut Vec<u8>, spec: HeaderSpec<'_>) {
        fn write_hex(out: &mut Vec<u8>, v: u32) {
            let s = std::format!("{v:08x}");
            out.extend_from_slice(s.as_bytes());
        }

        out.extend_from_slice(if spec.crc { b"070702" } else { b"070701" });
        write_hex(out, spec.ino);
        write_hex(out, spec.mode);
        write_hex(out, spec.uid);
        write_hex(out, spec.gid);
        write_hex(out, spec.nlink);
        write_hex(out, spec.mtime);
        write_hex(out, spec.payload.len() as u32);
        write_hex(out, spec.dev_major);
        write_hex(out, spec.dev_minor);
        write_hex(out, spec.rdev_major);
        write_hex(out, spec.rdev_minor);
        write_hex(out, (spec.name.len() + 1) as u32);
        write_hex(out, spec.csum);
        out.extend_from_slice(spec.name.as_bytes());
        out.push(0);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out.extend_from_slice(spec.payload);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }

    fn append_header(out: &mut Vec<u8>, name: &str, mode: u32, payload: &[u8]) {
        append_header_with_mtime(out, name, mode, 0, payload);
    }

    fn tiny_initramfs() -> Vec<u8> {
        let mut out = Vec::new();
        append_header(&mut out, "bin/hello", 0o100755, b"ELF...");
        append_header(&mut out, "etc/profile", 0o100644, b"export PATH=/bin");
        append_header(&mut out, CPIO_TRAILER, 0, &[]);
        out
    }

    fn symlink_initramfs() -> Vec<u8> {
        let mut out = Vec::new();
        append_header(
            &mut out,
            "usr/lib/systemd/systemd",
            0o100755,
            b"ELF-systemd",
        );
        append_header(&mut out, "sbin/init", 0o120777, b"/usr/lib/systemd/systemd");
        append_header(&mut out, CPIO_TRAILER, 0, &[]);
        out
    }

    #[test]
    fn parse_newc_and_lookup_regular_file() {
        let blob = tiny_initramfs();
        let leaked = Box::leak(blob.into_boxed_slice());
        let image = InitramfsImage::parse(leaked).expect("must parse");
        let e = image.find("/bin/hello").expect("entry");
        assert!(e.is_regular_file());
        assert_eq!(image.read_file("/bin/hello"), Some(&b"ELF..."[..]));
    }

    #[test]
    fn parse_newc_preserves_linux_inode_metadata_fields() {
        let mut blob = Vec::new();
        append_header_full(
            &mut blob,
            HeaderSpec {
                name: "bin/ls",
                mode: 0o100755,
                ino: 7,
                uid: 12,
                gid: 34,
                nlink: 1,
                mtime: 1_779_194_096,
                dev_major: 8,
                dev_minor: 1,
                rdev_major: 0,
                rdev_minor: 0,
                csum: 0,
                payload: b"ls",
                crc: false,
            },
        );
        append_header(&mut blob, CPIO_TRAILER, 0, &[]);
        let leaked = Box::leak(blob.into_boxed_slice());
        let image = InitramfsImage::parse(leaked).expect("must parse");
        let e = image.find("/bin/ls").expect("entry");
        assert_eq!(e.ino(), 7);
        assert_eq!(e.uid(), 12);
        assert_eq!(e.gid(), 34);
        assert_eq!(e.nlink(), 1);
        assert_eq!(e.mtime(), 1_779_194_096);
        assert_eq!(e.dev_major(), 8);
        assert_eq!(e.dev_minor(), 1);
    }

    #[test]
    fn symlink_entries_are_indexed_and_read_file_follows_target() {
        let blob = symlink_initramfs();
        let leaked = Box::leak(blob.into_boxed_slice());
        let image = InitramfsImage::parse(leaked).expect("must parse");
        let link = image.find("/sbin/init").expect("link entry");
        assert!(link.is_symlink());
        assert_eq!(
            image.read_file("/sbin/init"),
            Some(&b"ELF-systemd"[..]),
            "/sbin/init must resolve to the real systemd ELF"
        );
        assert_eq!(
            image.read_link("/sbin/init"),
            Some("/usr/lib/systemd/systemd")
        );
    }

    #[test]
    fn normalize_relative_paths() {
        assert_eq!(normalize_path("bin/sh"), "/bin/sh");
        assert_eq!(normalize_path("/bin/sh"), "/bin/sh");
    }

    #[test]
    fn initramfs_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/initramfs.c"
        ));
        let internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/initramfs_internal.h"
        ));
        assert!(internal.contains("#define CPIO_HDRLEN 110"));
        assert!(source.contains("unpack_to_rootfs"));
        assert!(source.contains("error(\"incorrect cpio method used: use -H newc option\")"));
        assert!(source.contains("error(\"no cpio magic\")"));
        assert!(source.contains("error(\"damaged header\")"));
        assert!(source.contains("error(\"bad data checksum\")"));
        assert!(source.contains("static char __init *find_link"));
        assert!(source.contains("static int __init maybe_link(void)"));
        assert!(source.contains("io_csum += p[i];"));
        assert!(source.contains("dir_utime();"));
    }

    #[test]
    fn parse_errors_use_linux_initramfs_diagnostics() {
        let mut binary = vec![b'0'; CPIO_HEADER_LEN];
        binary[..6].copy_from_slice(b"070707");
        assert!(matches!(
            InitramfsImage::parse(Box::leak(binary.into_boxed_slice())),
            Err(InitramfsParseError::IncorrectCpioMethod)
        ));

        let no_magic = vec![b'x'; CPIO_HEADER_LEN];
        assert!(matches!(
            InitramfsImage::parse(Box::leak(no_magic.into_boxed_slice())),
            Err(InitramfsParseError::NoCpioMagic)
        ));

        let mut damaged = vec![b'0'; CPIO_HEADER_LEN];
        damaged[..6].copy_from_slice(b"070701");
        assert!(matches!(
            InitramfsImage::parse(Box::leak(damaged.into_boxed_slice())),
            Err(InitramfsParseError::DamagedHeader)
        ));

        assert_eq!(
            InitramfsParseError::IncorrectCpioMethod.as_str(),
            "incorrect cpio method used: use -H newc option"
        );
        assert_eq!(InitramfsParseError::NoCpioMagic.as_str(), "no cpio magic");
        assert_eq!(
            InitramfsParseError::DamagedHeader.as_str(),
            "damaged header"
        );
        assert_eq!(
            InitramfsParseError::BadDataChecksum.as_str(),
            "bad data checksum"
        );
    }

    #[test]
    fn crc_newc_entries_verify_payload_sum() {
        let mut blob = Vec::new();
        append_header_full(
            &mut blob,
            HeaderSpec {
                name: "crc-file",
                mode: 0o100644,
                csum: payload_csum(b"ASDF"),
                payload: b"ASDF",
                crc: true,
                ino: 1,
                uid: 0,
                gid: 0,
                nlink: 1,
                mtime: 0,
                dev_major: 0,
                dev_minor: 0,
                rdev_major: 0,
                rdev_minor: 0,
            },
        );
        append_header(&mut blob, CPIO_TRAILER, 0, &[]);
        let leaked = Box::leak(blob.into_boxed_slice());
        assert!(InitramfsImage::parse(leaked).is_ok());

        let mut bad = Vec::new();
        append_header_full(
            &mut bad,
            HeaderSpec {
                name: "crc-file",
                mode: 0o100644,
                csum: 1,
                payload: b"ASDF",
                crc: true,
                ino: 1,
                uid: 0,
                gid: 0,
                nlink: 1,
                mtime: 0,
                dev_major: 0,
                dev_minor: 0,
                rdev_major: 0,
                rdev_minor: 0,
            },
        );
        assert!(matches!(
            InitramfsImage::parse(Box::leak(bad.into_boxed_slice())),
            Err(InitramfsParseError::BadDataChecksum)
        ));
    }

    #[test]
    fn directory_and_special_entries_are_indexed_with_device_numbers() {
        let mut blob = Vec::new();
        append_header(&mut blob, "etc", S_IFDIR | 0o755, &[]);
        append_header_full(
            &mut blob,
            HeaderSpec {
                name: "dev/ttyS0",
                mode: S_IFCHR | 0o600,
                rdev_major: 4,
                rdev_minor: 64,
                ino: 2,
                uid: 0,
                gid: 0,
                nlink: 1,
                mtime: 0,
                dev_major: 0,
                dev_minor: 1,
                csum: 0,
                payload: &[],
                crc: false,
            },
        );
        append_header(&mut blob, "run/fifo", S_IFIFO | 0o600, &[]);
        append_header(&mut blob, CPIO_TRAILER, 0, &[]);
        let image = InitramfsImage::parse(Box::leak(blob.into_boxed_slice())).expect("parse");

        assert!(image.find("/etc").expect("etc").is_dir());
        let tty = image.find("/dev/ttyS0").expect("ttyS0");
        assert!(tty.is_chardev());
        assert_eq!(tty.rdev_major(), 4);
        assert_eq!(tty.rdev_minor(), 64);
        assert!(image.find("/run/fifo").expect("fifo").is_fifo());
    }

    #[test]
    fn newc_hardlink_payload_on_last_entry_reads_from_all_names() {
        let mut blob = Vec::new();
        append_header_full(
            &mut blob,
            HeaderSpec {
                name: "bin/a",
                mode: 0o100755,
                ino: 9,
                nlink: 2,
                dev_major: 0,
                dev_minor: 1,
                payload: &[],
                uid: 0,
                gid: 0,
                mtime: 0,
                rdev_major: 0,
                rdev_minor: 0,
                csum: 0,
                crc: false,
            },
        );
        append_header_full(
            &mut blob,
            HeaderSpec {
                name: "bin/b",
                mode: 0o100755,
                ino: 9,
                nlink: 2,
                dev_major: 0,
                dev_minor: 1,
                payload: b"payload",
                uid: 0,
                gid: 0,
                mtime: 0,
                rdev_major: 0,
                rdev_minor: 0,
                csum: 0,
                crc: false,
            },
        );
        append_header(&mut blob, CPIO_TRAILER, 0, &[]);
        let image = InitramfsImage::parse(Box::leak(blob.into_boxed_slice())).expect("parse");
        assert_eq!(image.read_file("/bin/a"), Some(&b"payload"[..]));
        assert_eq!(image.read_file("/bin/b"), Some(&b"payload"[..]));
    }

    #[test]
    fn install_and_read_roundtrip() {
        let blob = tiny_initramfs();
        let leaked = Box::leak(blob.into_boxed_slice());
        install_from_bytes(leaked).expect("install");
        let bytes = read_file("/etc/profile").expect("lookup");
        assert_eq!(
            bytes,
            vec![
                101, 120, 112, 111, 114, 116, 32, 80, 65, 84, 72, 61, 47, 98, 105, 110
            ]
        );
    }
}
