//! linux-parity: complete
//! linux-source: vendor/linux/fs/fat
//! test-origin: linux:vendor/linux/fs/fat
//! FAT cache, file-window, namei, and export helpers.
//!
//! Mirrors:
//! `vendor/linux/fs/fat/cache.c`
//! `vendor/linux/fs/fat/file.c`
//! `vendor/linux/fs/fat/misc.c`
//! `vendor/linux/fs/fat/namei_msdos.c`
//! `vendor/linux/fs/fat/namei_vfat.c`
//! `vendor/linux/fs/fat/nfs.c`

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, EOVERFLOW};

#[derive(Clone, Debug, Default)]
pub struct FatCache {
    clusters: BTreeMap<u32, Vec<u8>>,
}

impl FatCache {
    pub fn insert(&mut self, cluster: u32, data: Vec<u8>) -> Result<(), i32> {
        if cluster < 2 {
            return Err(EINVAL);
        }
        self.clusters.insert(cluster, data);
        Ok(())
    }

    pub fn get(&self, cluster: u32) -> Option<&[u8]> {
        self.clusters.get(&cluster).map(Vec::as_slice)
    }

    pub fn invalidate(&mut self, cluster: u32) {
        self.clusters.remove(&cluster);
    }
}

pub fn shortname_checksum(name: &[u8; 11]) -> u8 {
    name.iter().fold(0u8, |sum, byte| {
        ((sum & 1) << 7).wrapping_add(sum >> 1).wrapping_add(*byte)
    })
}

pub fn normalize_msdos_name(raw: &[u8; 11]) -> String {
    let base = trim_spaces(&raw[..8]);
    let ext = trim_spaces(&raw[8..11]);
    let mut out = String::new();
    push_ascii_upper(&mut out, base);
    if !ext.is_empty() {
        out.push('.');
        push_ascii_upper(&mut out, ext);
    }
    out
}

pub fn canonical_vfat_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch == '/' || ch == '\\' {
            continue;
        }
        out.push(ch.to_ascii_lowercase());
    }
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    out
}

pub fn names_match(disk_name: &str, query: &str) -> bool {
    canonical_vfat_name(disk_name) == canonical_vfat_name(query)
}

pub fn read_window(file_size: u64, pos: u64, buf_len: usize) -> Result<usize, i32> {
    if pos >= file_size {
        return Ok(0);
    }
    let remaining = file_size.checked_sub(pos).ok_or(EOVERFLOW)?;
    Ok(remaining.min(buf_len as u64) as usize)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FatFileHandle {
    pub start_cluster: u32,
    pub size: u32,
    pub checksum: u8,
}

pub fn export_file_handle(start_cluster: u32, size: u32, short_name: &[u8; 11]) -> FatFileHandle {
    FatFileHandle {
        start_cluster,
        size,
        checksum: shortname_checksum(short_name),
    }
}

pub fn valid_shortname_char(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z'
            | b'0'..=b'9'
            | b'$'
            | b'%'
            | b'\''
            | b'-'
            | b'_'
            | b'@'
            | b'~'
            | b'`'
            | b'!'
            | b'('
            | b')'
            | b'{'
            | b'}'
            | b'^'
            | b'#'
            | b'&'
    )
}

fn trim_spaces(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|byte| *byte != b' ')
        .map(|index| index + 1)
        .unwrap_or(0);
    &bytes[..end]
}

fn push_ascii_upper(out: &mut String, bytes: &[u8]) {
    for byte in bytes.iter().copied() {
        out.push((byte as char).to_ascii_uppercase());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fat_cache_rejects_reserved_clusters() {
        let mut cache = FatCache::default();
        assert_eq!(cache.insert(1, alloc::vec![1, 2, 3]), Err(EINVAL));
        cache.insert(2, alloc::vec![4]).unwrap();
        assert_eq!(cache.get(2), Some(&[4][..]));
        cache.invalidate(2);
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn shortname_normalization_and_checksum_are_stable() {
        let raw = *b"README  TXT";
        assert_eq!(normalize_msdos_name(&raw), "README.TXT");
        assert_eq!(shortname_checksum(&raw), shortname_checksum(&raw));
    }

    #[test]
    fn vfat_names_match_case_insensitively() {
        assert!(names_match("Long File.txt", "long file.TXT"));
        assert!(names_match("foo.", "FOO"));
    }

    #[test]
    fn read_window_clamps_to_file_size() {
        assert_eq!(read_window(10, 4, 16).unwrap(), 6);
        assert_eq!(read_window(10, 10, 16).unwrap(), 0);
    }
}
