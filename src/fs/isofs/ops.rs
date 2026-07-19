//! linux-parity: partial
//! linux-source: vendor/linux/fs/isofs
//! ISO9660 op vtables.

extern crate alloc;

use alloc::vec::Vec;

use crate::block::partitions::read_sectors;
use crate::fs::ops::{FileOps, InodeOps, SuperOps};
use crate::fs::types::{FileRef, InodeKind, InodeRef};
use crate::include::uapi::errno::{EINVAL, ENOENT, EROFS};

use super::dir::{is_dir as iso_is_dir, read_all as dir_read_all};
use super::inode::{iso_of, make_inode};

pub static ISO_SUPER_OPS: SuperOps = SuperOps {
    name: "iso9660",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub static ISO_DIR_INODE_OPS: InodeOps = InodeOps {
    name: "iso_dir",
    lookup: Some(iso_lookup),
    create: Some(|_, _, _| Err(EROFS)),
    mkdir: Some(|_, _, _| Err(EROFS)),
    unlink: Some(|_, _, _| Err(EROFS)),
    rmdir: Some(|_, _| Err(EROFS)),
    rename: Some(|_, _, _, _| Err(EROFS)),
    symlink: Some(|_, _, _, _| Err(EROFS)),
    readlink: None,
    setattr: None,
};
pub static ISO_FILE_INODE_OPS: InodeOps = InodeOps {
    name: "iso_file",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: None,
    setattr: None,
};
pub static ISO_DIR_FILE_OPS: FileOps = FileOps {
    name: "iso_dir",
    read: None,
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: Some(iso_readdir),
};
pub static ISO_FILE_FILE_OPS: FileOps = FileOps {
    name: "iso_file",
    read: Some(iso_read),
    write: Some(|_, _, _| Err(EROFS)),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

fn iso_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = super::get_sbi(&sb).ok_or(EINVAL)?;
    let ino = iso_of(dir).ok_or(EINVAL)?;
    let entries = dir_read_all(&sbi, ino.extent, ino.size)?;
    for e in entries.iter() {
        if e.name.eq_ignore_ascii_case(name) {
            return Ok(make_inode(e.extent, e.size, iso_is_dir(e.flags), &sb));
        }
    }
    Err(ENOENT)
}

fn iso_readdir(file: &FileRef) -> Result<Option<(alloc::string::String, u64, InodeKind)>, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = super::get_sbi(&sb).ok_or(EINVAL)?;
    let ino = iso_of(&inode).ok_or(EINVAL)?;
    let entries = dir_read_all(&sbi, ino.extent, ino.size)?;
    let mut idx = file.pos.lock();
    if (*idx as usize) >= entries.len() {
        return Ok(None);
    }
    let e = entries[*idx as usize].clone();
    let kind = if iso_is_dir(e.flags) {
        InodeKind::Directory
    } else {
        InodeKind::Regular
    };
    *idx += 1;
    Ok(Some((e.name, e.extent as u64, kind)))
}

fn iso_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = super::get_sbi(&sb).ok_or(EINVAL)?;
    let ino = iso_of(&inode).ok_or(EINVAL)?;
    let size = ino.size as u64;
    if *pos >= size {
        return Ok(0);
    }
    let max = (size - *pos).min(buf.len() as u64) as usize;

    // Read entire extent (small fixtures); slice from pos.
    let lba = ino.extent as u64 * 4;
    let nr_sectors = ((ino.size as u64).div_ceil(512)) as u64;
    let bytes: Vec<u8> = read_sectors(&sbi.bdev, lba, nr_sectors)?;
    let start = *pos as usize;
    let n = max.min(bytes.len().saturating_sub(start));
    buf[..n].copy_from_slice(&bytes[start..start + n]);
    *pos += n as u64;
    Ok(n)
}
