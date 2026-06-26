//! linux-parity: partial
//! linux-source: vendor/linux/fs/libfs.c
//! Generic filesystem helpers — ports of `vendor/linux/fs/libfs.c`.
//!
//! `simple_*` routines that any in-memory filesystem (ramfs, tmpfs, debugfs,
//! kernfs) can wire into its op vtable.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Mutex;

use crate::include::uapi::errno::{
    EFBIG, EINVAL, EISDIR, ENOENT, ENOMEM, ENOSYS, ENOTEMPTY, EPERM, EROFS,
};

use super::types::{FileRef, Inode, InodeKind, InodePrivate, InodeRef, touch_inode_now};

/// `simple_lookup` — search the in-memory `RamDir` table.
pub fn simple_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let map = match &dir.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    map.lock()
        .iter()
        .find(|(child_name, _)| names_eq(child_name.as_str(), name))
        .map(|(_, inode)| inode.clone())
        .ok_or(ENOENT)
}

/// `simple_unlink` — remove a non-directory entry from a `RamDir`.
pub fn simple_unlink(dir: &InodeRef, name: &str) -> Result<(), i32> {
    let map = match &dir.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    let mut g = map.lock();
    let key = g
        .keys()
        .find(|child_name| names_eq(child_name.as_str(), name))
        .cloned()
        .ok_or(ENOENT)?;
    let child = g.get(&key).cloned().ok_or(ENOENT)?;
    if child.kind == InodeKind::Directory {
        return Err(EISDIR);
    }
    g.remove(&key);
    let nlink = child.nlink.fetch_sub(1, Ordering::AcqRel);
    drop(nlink);
    touch_inode_now(dir);
    touch_inode_now(&child);
    Ok(())
}

/// `simple_rmdir` — remove an empty directory entry from a `RamDir`.
pub fn simple_rmdir(dir: &InodeRef, name: &str) -> Result<(), i32> {
    let map = match &dir.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    let mut g = map.lock();
    let key = g
        .keys()
        .find(|child_name| names_eq(child_name.as_str(), name))
        .cloned()
        .ok_or(ENOENT)?;
    let child = g.get(&key).cloned().ok_or(ENOENT)?;
    if child.kind != InodeKind::Directory {
        return Err(ENOSYS);
    }
    if let InodePrivate::RamDir(cm) = &child.private {
        if !cm.lock().is_empty() {
            return Err(ENOTEMPTY);
        }
    }
    g.remove(&key);
    child.nlink.store(0, Ordering::Release);
    let parent_links = dir.nlink.load(Ordering::Acquire);
    if parent_links > 0 {
        dir.nlink.fetch_sub(1, Ordering::AcqRel);
    }
    touch_inode_now(dir);
    touch_inode_now(&child);
    Ok(())
}

fn names_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Emit Linux's synthetic `.` / `..` directory entries for in-memory
/// filesystem iterators.  `file.private` mirrors `dir_context::pos`.
pub fn synthetic_readdir_dot_entry(
    file: &FileRef,
) -> Result<Option<(String, u64, InodeKind)>, i32> {
    let mut pos = file.private.lock();
    let name = match *pos {
        0 => ".",
        1 => "..",
        _ => return Ok(None),
    };
    let ino = if *pos == 0 {
        file.inode().ok_or(EINVAL)?.ino
    } else {
        let parent = file
            .dentry
            .parent
            .lock()
            .clone()
            .unwrap_or_else(|| file.dentry.clone());
        parent.inode().or_else(|| file.inode()).ok_or(EINVAL)?.ino
    };
    *pos += 1;
    Ok(Some((String::from(name), ino, InodeKind::Directory)))
}

/// Generic readdir cursor — `file.private` holds a Linux-style directory
/// position: 0/1 for dot entries, 2+ for entries in the BTreeMap.
pub fn simple_readdir(file: &FileRef) -> Result<Option<(String, u64, InodeKind)>, i32> {
    if let Some(dot) = synthetic_readdir_dot_entry(file)? {
        return Ok(Some(dot));
    }
    let inode = file.inode().ok_or(EINVAL)?;
    let map = match &inode.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    let mut idx = file.private.lock();
    let g = map.lock();
    let child_idx = idx.saturating_sub(2);
    if child_idx >= g.len() {
        return Ok(None);
    }
    let (k, v) = g.iter().nth(child_idx).unwrap();
    let out = (k.clone(), v.ino, v.kind);
    *idx += 1;
    Ok(Some(out))
}

/// Generic ramfs-style read from `RamBytes`.
pub fn ram_file_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let n = match &inode.private {
        InodePrivate::RamBytes(m) => {
            let g = m.lock();
            let logical_len = inode.size.load(Ordering::Acquire) as usize;
            let start = (*pos as usize).min(logical_len);
            let n = (logical_len - start).min(buf.len());
            let materialized = if start < g.len() {
                let bytes = (g.len() - start).min(n);
                buf[..bytes].copy_from_slice(&g[start..start + bytes]);
                bytes
            } else {
                0
            };
            if materialized < n {
                buf[materialized..n].fill(0);
            }
            n
        }
        InodePrivate::StaticBytes(bytes) => {
            let start = (*pos as usize).min(bytes.len());
            let n = (bytes.len() - start).min(buf.len());
            buf[..n].copy_from_slice(&bytes[start..start + n]);
            n
        }
        InodePrivate::StaticCowBytes { base, overlay } => {
            let logical_len = inode.size.load(Ordering::Acquire) as usize;
            let start = (*pos as usize).min(logical_len);
            let n = (logical_len - start).min(buf.len());
            if let Some(bytes) = overlay.lock().as_ref() {
                let materialized = if start < bytes.len() {
                    let bytes_to_copy = (bytes.len() - start).min(n);
                    buf[..bytes_to_copy].copy_from_slice(&bytes[start..start + bytes_to_copy]);
                    bytes_to_copy
                } else {
                    0
                };
                if materialized < n {
                    buf[materialized..n].fill(0);
                }
            } else {
                let base_len = base.len().min(logical_len);
                let copied = if start < base_len {
                    let bytes_to_copy = (base_len - start).min(n);
                    buf[..bytes_to_copy].copy_from_slice(&base[start..start + bytes_to_copy]);
                    bytes_to_copy
                } else {
                    0
                };
                if copied < n {
                    buf[copied..n].fill(0);
                }
            }
            n
        }
        _ => return Err(EINVAL),
    };
    *pos += n as u64;
    Ok(n)
}

/// Generic ramfs-style write into `RamBytes`.
pub fn ram_file_write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    match &inode.private {
        InodePrivate::RamBytes(m) => {
            let mut g = m.lock();
            write_into_vec(&mut g, &inode, buf, pos)
        }
        InodePrivate::StaticCowBytes { base, overlay } => {
            let mut maybe_overlay = overlay.lock();
            if maybe_overlay.is_none() {
                let mut materialized = Vec::new();
                materialized.try_reserve(base.len()).map_err(|_| ENOMEM)?;
                materialized.extend_from_slice(base);
                *maybe_overlay = Some(materialized);
            }
            let bytes = maybe_overlay.as_mut().ok_or(EINVAL)?;
            write_into_vec(bytes, &inode, buf, pos)
        }
        InodePrivate::StaticBytes(_) => return Err(EROFS),
        _ => return Err(EINVAL),
    }
}

fn write_into_vec(
    g: &mut Vec<u8>,
    inode: &InodeRef,
    buf: &[u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    let p = *pos as usize;
    let end = p.checked_add(buf.len()).ok_or(EINVAL)?;
    if g.len() < end {
        let additional = end - g.len();
        g.try_reserve(additional).map_err(|_| ENOMEM)?;
        g.resize(end, 0);
    }
    g[p..end].copy_from_slice(buf);
    *pos += buf.len() as u64;
    let logical_len = inode.size.load(Ordering::Acquire).max(end as u64);
    inode.size.store(logical_len, Ordering::Release);
    touch_inode_now(inode);
    Ok(buf.len())
}

/// Update the logical size of a ramfs/tmpfs byte file without eagerly
/// materializing holes. Linux grows files through the page cache a page at a
/// time; a single contiguous `Vec` is only our compact representation for data
/// that has actually been written.
pub fn ram_file_set_size(inode: &InodeRef, size: u64) -> Result<(), i32> {
    if size > usize::MAX as u64 {
        return Err(EFBIG);
    }
    let new_len = size as usize;
    match &inode.private {
        InodePrivate::RamBytes(bytes) => {
            let mut g = bytes.lock();
            if new_len < g.len() {
                g.truncate(new_len);
            }
        }
        InodePrivate::StaticCowBytes { overlay, .. } => {
            if let Some(bytes) = overlay.lock().as_mut() {
                if new_len < bytes.len() {
                    bytes.truncate(new_len);
                }
            }
        }
        _ => {}
    }
    inode.size.store(size, Ordering::Release);
    touch_inode_now(inode);
    Ok(())
}

pub fn ram_file_zero_range(
    inode: &InodeRef,
    offset: u64,
    len: u64,
    keep_size: bool,
) -> Result<(), i32> {
    let end = offset.checked_add(len).ok_or(EINVAL)?;
    if !keep_size && end > inode.size.load(Ordering::Acquire) {
        ram_file_set_size(inode, end)?;
    }
    let start = offset.min(usize::MAX as u64) as usize;
    let end = end.min(usize::MAX as u64) as usize;
    if start >= end {
        return Ok(());
    }
    match &inode.private {
        InodePrivate::RamBytes(bytes) => {
            let mut g = bytes.lock();
            let zero_end = end.min(g.len());
            if start < zero_end {
                g[start..zero_end].fill(0);
            }
        }
        InodePrivate::StaticCowBytes { base, overlay } => {
            let mut maybe_overlay = overlay.lock();
            if maybe_overlay.is_none() {
                let mut materialized = Vec::new();
                materialized.try_reserve(base.len()).map_err(|_| ENOMEM)?;
                materialized.extend_from_slice(base);
                *maybe_overlay = Some(materialized);
            }
            if let Some(bytes) = maybe_overlay.as_mut() {
                let zero_end = end.min(bytes.len());
                if start < zero_end {
                    bytes[start..zero_end].fill(0);
                }
            }
        }
        _ => {}
    }
    touch_inode_now(inode);
    Ok(())
}

/// Build an empty `InodePrivate::RamDir`.
pub fn empty_ram_dir() -> InodePrivate {
    InodePrivate::RamDir(Mutex::new(BTreeMap::new()))
}

/// Build an empty `InodePrivate::RamBytes`.
pub fn empty_ram_bytes() -> InodePrivate {
    InodePrivate::RamBytes(Mutex::new(Vec::new()))
}

/// Build copy-on-write ramfs bytes backed by the installed initramfs image.
pub fn static_cow_bytes(base: &'static [u8]) -> InodePrivate {
    InodePrivate::StaticCowBytes {
        base,
        overlay: Mutex::new(None),
    }
}
