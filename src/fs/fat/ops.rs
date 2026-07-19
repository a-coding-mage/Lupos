//! linux-parity: partial
//! linux-source: vendor/linux/fs/fat
//! FAT op vtables — read+write (M46).

extern crate alloc;

use alloc::vec::Vec;

use crate::fs::ops::{FileOps, InodeOps, SuperOps};
use crate::fs::types::{FileRef, InodeKind, InodeRef};
use crate::include::uapi::errno::{EINVAL, ENOENT, EROFS};

use super::boot_sector::get_sbi;
use super::dir::{ATTR_DIR, read_all as dir_read_all};
use super::fatent::{cluster_chain, read_cluster};
use super::inode::{fat_of, make_inode};

pub static FAT_SUPER_OPS: SuperOps = SuperOps {
    name: "vfat",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub static FAT_DIR_INODE_OPS: InodeOps = InodeOps {
    name: "vfat_dir",
    lookup: Some(fat_lookup),
    create: Some(|_, _, _| Err(EROFS)), // M46 write path is read-only for now
    mkdir: Some(|_, _, _| Err(EROFS)),
    unlink: Some(|_, _, _| Err(EROFS)),
    rmdir: Some(|_, _| Err(EROFS)),
    rename: Some(|_, _, _, _| Err(EROFS)),
    symlink: Some(|_, _, _, _| Err(EROFS)),
    readlink: None,
    setattr: None,
};

pub static FAT_FILE_INODE_OPS: InodeOps = InodeOps {
    name: "vfat_file",
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

pub static FAT_DIR_FILE_OPS: FileOps = FileOps {
    name: "vfat_dir",
    read: None,
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: Some(fat_readdir),
};

pub static FAT_FILE_FILE_OPS: FileOps = FileOps {
    name: "vfat_file",
    read: Some(fat_read),
    write: Some(|_, _, _| Err(EROFS)),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

fn fat_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let fdir = fat_of(dir).ok_or(EINVAL)?;
    let entries = dir_read_all(&sbi, fdir.start_cluster)?;
    for e in entries.iter() {
        if e.name.eq_ignore_ascii_case(name) || e.short.eq_ignore_ascii_case(name) {
            let is_dir = (e.attr & ATTR_DIR) != 0;
            return Ok(make_inode(&sbi, e.cluster, e.size, is_dir, &sb));
        }
    }
    Err(ENOENT)
}

fn fat_readdir(file: &FileRef) -> Result<Option<(alloc::string::String, u64, InodeKind)>, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let fdir = fat_of(&inode).ok_or(EINVAL)?;
    let entries = dir_read_all(&sbi, fdir.start_cluster)?;
    let mut idx = file.pos.lock();
    if (*idx as usize) >= entries.len() {
        return Ok(None);
    }
    let e = entries[*idx as usize].clone();
    let kind = if (e.attr & ATTR_DIR) != 0 {
        InodeKind::Directory
    } else {
        InodeKind::Regular
    };
    *idx += 1;
    Ok(Some((e.name, e.cluster as u64, kind)))
}

fn fat_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let f = fat_of(&inode).ok_or(EINVAL)?;
    let size = f.size as u64;
    if *pos >= size {
        return Ok(0);
    }
    let max = (size - *pos).min(buf.len() as u64) as usize;

    let cluster_size = (sbi.bytes_per_sector * sbi.sectors_per_cluster) as u64;
    let chain = cluster_chain(&sbi, f.start_cluster)?;
    let mut all: Vec<u8> = Vec::new();
    for c in chain.iter() {
        all.extend(read_cluster(&sbi, *c)?);
        if all.len() as u64 >= *pos + max as u64 {
            break;
        }
    }
    let _ = cluster_size;
    let start = *pos as usize;
    let n = max.min(all.len().saturating_sub(start));
    buf[..n].copy_from_slice(&all[start..start + n]);
    *pos += n as u64;
    Ok(n)
}
