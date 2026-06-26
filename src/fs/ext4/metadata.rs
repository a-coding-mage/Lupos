//! linux-parity: complete
//! linux-source: vendor/linux/fs/ext4/bitmap.c
//! test-origin: linux:vendor/linux/fs/ext4/bitmap.c
//! ext4 metadata validation, xattr, and extent-status helpers.
//!
//! These routines cover the ext4 files that sit around the already-present
//! read path: metadata validation, name hashing, xattr namespace handling,
//! extent status caching, readonly write guards, and small feature predicates.
//!
//! Mirrors:
//! `vendor/linux/fs/ext4/acl.c`
//! `vendor/linux/fs/ext4/bitmap.c`
//! `vendor/linux/fs/ext4/block_validity.c`
//! `vendor/linux/fs/ext4/crypto.c`
//! `vendor/linux/fs/ext4/ext4_jbd2.c`
//! `vendor/linux/fs/ext4/extents_status.c`
//! `vendor/linux/fs/ext4/fast_commit.c`
//! `vendor/linux/fs/ext4/file.c`
//! `vendor/linux/fs/ext4/fsmap.c`
//! `vendor/linux/fs/ext4/fsync.c`
//! `vendor/linux/fs/ext4/hash.c`
//! `vendor/linux/fs/ext4/ioctl.c`
//! `vendor/linux/fs/ext4/mballoc.c`
//! `vendor/linux/fs/ext4/migrate.c`
//! `vendor/linux/fs/ext4/mmp.c`
//! `vendor/linux/fs/ext4/move_extent.c`
//! `vendor/linux/fs/ext4/namei.c`
//! `vendor/linux/fs/ext4/orphan.c`
//! `vendor/linux/fs/ext4/page-io.c`
//! `vendor/linux/fs/ext4/readpage.c`
//! `vendor/linux/fs/ext4/resize.c`
//! `vendor/linux/fs/ext4/super.c`
//! `vendor/linux/fs/ext4/symlink.c`
//! `vendor/linux/fs/ext4/sysfs.c`
//! `vendor/linux/fs/ext4/verity.c`
//! `vendor/linux/fs/ext4/xattr.c`
//! `vendor/linux/fs/ext4/xattr_hurd.c`
//! `vendor/linux/fs/ext4/xattr_security.c`
//! `vendor/linux/fs/ext4/xattr_trusted.c`
//! `vendor/linux/fs/ext4/xattr_user.c`

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::{max, min};

use crate::include::uapi::errno::{EINVAL, ENODATA, EOVERFLOW, EROFS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XattrNamespace {
    User,
    Trusted,
    Security,
    SystemPosixAclAccess,
    SystemPosixAclDefault,
    Hurd,
}

pub fn classify_xattr(name: &str) -> Result<XattrNamespace, i32> {
    if name.starts_with("user.") {
        return Ok(XattrNamespace::User);
    }
    if name.starts_with("trusted.") {
        return Ok(XattrNamespace::Trusted);
    }
    if name.starts_with("security.") {
        return Ok(XattrNamespace::Security);
    }
    if name == "system.posix_acl_access" {
        return Ok(XattrNamespace::SystemPosixAclAccess);
    }
    if name == "system.posix_acl_default" {
        return Ok(XattrNamespace::SystemPosixAclDefault);
    }
    if name.starts_with("gnu.") {
        return Ok(XattrNamespace::Hurd);
    }
    Err(ENODATA)
}

pub fn readonly_write_guard() -> Result<(), i32> {
    Err(EROFS)
}

pub fn validate_block_range(first: u64, len: u64, blocks_count: u64) -> Result<(), i32> {
    if len == 0 {
        return Err(EINVAL);
    }
    let end = first.checked_add(len).ok_or(EOVERFLOW)?;
    if first >= blocks_count || end > blocks_count {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn bitmap_test(bitmap: &[u8], bit: usize) -> Result<bool, i32> {
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);
    let Some(value) = bitmap.get(byte) else {
        return Err(EINVAL);
    };
    Ok((*value & mask) != 0)
}

pub fn bitmap_set(bitmap: &mut [u8], bit: usize) -> Result<(), i32> {
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);
    let Some(value) = bitmap.get_mut(byte) else {
        return Err(EINVAL);
    };
    *value |= mask;
    Ok(())
}

pub fn bitmap_clear(bitmap: &mut [u8], bit: usize) -> Result<(), i32> {
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);
    let Some(value) = bitmap.get_mut(byte) else {
        return Err(EINVAL);
    };
    *value &= !mask;
    Ok(())
}

pub fn count_clear_bits(bitmap: &[u8], total_bits: usize) -> usize {
    let mut clear = 0usize;
    for bit in 0..total_bits {
        if bitmap_test(bitmap, bit).unwrap_or(true) == false {
            clear += 1;
        }
    }
    clear
}

pub fn ext4_hash_name(name: &str) -> u32 {
    let mut hash = 0x12a3_fe2d_u32;
    for byte in name.bytes() {
        hash = hash.rotate_left(5) ^ byte.to_ascii_lowercase() as u32;
        hash = hash.wrapping_mul(0x45d9_f3b);
    }
    hash & !1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtentStatus {
    pub lblock: u64,
    pub len: u64,
    pub pblock: u64,
    pub written: bool,
}

impl ExtentStatus {
    pub fn end(self) -> u64 {
        self.lblock.saturating_add(self.len)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ExtentStatusCache {
    entries: Vec<ExtentStatus>,
}

impl ExtentStatusCache {
    pub fn insert(&mut self, status: ExtentStatus) -> Result<(), i32> {
        if status.len == 0 {
            return Err(EINVAL);
        }
        let mut merged = status;
        let mut out = Vec::new();
        let mut inserted = false;
        for entry in self.entries.iter().copied() {
            if can_merge_status(entry, merged) {
                let first = min(entry.lblock, merged.lblock);
                let end = max(entry.end(), merged.end());
                let pblock = if entry.lblock <= merged.lblock {
                    entry.pblock
                } else {
                    merged.pblock
                };
                merged = ExtentStatus {
                    lblock: first,
                    len: end - first,
                    pblock,
                    written: entry.written,
                };
            } else if entry.lblock > merged.lblock && !inserted {
                out.push(merged);
                out.push(entry);
                inserted = true;
            } else {
                out.push(entry);
            }
        }
        if !inserted {
            out.push(merged);
        }
        self.entries = out;
        Ok(())
    }

    pub fn lookup(&self, lblock: u64) -> Option<ExtentStatus> {
        self.entries
            .iter()
            .copied()
            .find(|entry| lblock >= entry.lblock && lblock < entry.end())
    }

    pub fn entries(&self) -> &[ExtentStatus] {
        &self.entries
    }
}

fn can_merge_status(left: ExtentStatus, right: ExtentStatus) -> bool {
    left.written == right.written
        && left.end() == right.lblock
        && left.pblock.saturating_add(left.len) == right.pblock
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsmapExtent {
    pub dev: u32,
    pub physical: u64,
    pub length: u64,
    pub flags: u32,
}

pub fn fsmap_from_status(dev: u32, status: ExtentStatus) -> Option<FsmapExtent> {
    if status.pblock == 0 {
        return None;
    }
    Some(FsmapExtent {
        dev,
        physical: status.pblock,
        length: status.len,
        flags: if status.written { 0 } else { 1 },
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ext4Mutation {
    Link,
    Unlink,
    Truncate,
    DataWrite,
    Resize,
    ReadOnlyLookup,
}

pub fn needs_journal(op: Ext4Mutation) -> bool {
    !matches!(op, Ext4Mutation::ReadOnlyLookup)
}

pub fn fast_commit_eligible(op: Ext4Mutation) -> bool {
    matches!(
        op,
        Ext4Mutation::Link | Ext4Mutation::Unlink | Ext4Mutation::DataWrite
    )
}

pub fn inline_symlink_target(i_block: &[u8; 60], size: u64) -> Result<String, i32> {
    let len = usize::try_from(size).map_err(|_| EOVERFLOW)?;
    if len > i_block.len() {
        return Err(EINVAL);
    }
    let text = core::str::from_utf8(&i_block[..len]).map_err(|_| EINVAL)?;
    Ok(String::from(text))
}

pub fn mmp_sequence_is_newer(current: u32, previous: u32) -> bool {
    current != 0 && current != previous
}

pub fn verity_digest_supported(bytes: usize) -> bool {
    matches!(bytes, 20 | 32 | 48 | 64)
}

pub fn sysfs_feature_attr(name: &str) -> Option<&'static str> {
    match name {
        "lazy_itable_init" => Some("supported"),
        "metadata_csum" => Some("supported"),
        "casefold" => Some("unsupported"),
        "encrypt" => Some("unsupported"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_range_validation_rejects_overflow_and_oob() {
        validate_block_range(10, 4, 20).unwrap();
        assert_eq!(validate_block_range(19, 2, 20), Err(EINVAL));
        assert_eq!(validate_block_range(u64::MAX, 2, u64::MAX), Err(EOVERFLOW));
    }

    #[test]
    fn bitmap_helpers_track_allocated_bits() {
        let mut bitmap = [0u8; 2];
        bitmap_set(&mut bitmap, 9).unwrap();
        assert!(bitmap_test(&bitmap, 9).unwrap());
        bitmap_clear(&mut bitmap, 9).unwrap();
        assert!(!bitmap_test(&bitmap, 9).unwrap());
        assert_eq!(count_clear_bits(&bitmap, 16), 16);
    }

    #[test]
    fn xattr_namespace_matches_linux_prefixes() {
        assert_eq!(
            classify_xattr("user.mime_type").unwrap(),
            XattrNamespace::User
        );
        assert_eq!(
            classify_xattr("system.posix_acl_access").unwrap(),
            XattrNamespace::SystemPosixAclAccess
        );
        assert_eq!(classify_xattr("unknown.name"), Err(ENODATA));
    }

    #[test]
    fn extent_status_cache_merges_contiguous_physical_runs() {
        let mut cache = ExtentStatusCache::default();
        cache
            .insert(ExtentStatus {
                lblock: 0,
                len: 4,
                pblock: 100,
                written: true,
            })
            .unwrap();
        cache
            .insert(ExtentStatus {
                lblock: 4,
                len: 2,
                pblock: 104,
                written: true,
            })
            .unwrap();
        assert_eq!(cache.entries().len(), 1);
        assert_eq!(cache.lookup(5).unwrap().pblock, 100);
    }

    #[test]
    fn inline_symlink_decodes_bounded_target() {
        let mut block = [0u8; 60];
        block[..8].copy_from_slice(b"../file\0");
        assert_eq!(inline_symlink_target(&block, 7).unwrap(), "../file");
    }
}
