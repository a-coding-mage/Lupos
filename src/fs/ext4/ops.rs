//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4
//! test-origin: linux:vendor/linux/fs/ext4
//! ext4 op vtables.
//!
//! Read-only path:
//!   * `lookup`      — scans the parent dir's entries (read_all) for `name`.
//!   * `read`        — maps logical blocks via extents (or inline) and copies into buf.
//!   * `readdir`     — iterates the directory entry list.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::block::bio::{BIO_OP_WRITE, BioOp, BioVec, bio_alloc, submit_bio};
use crate::block::partitions::read_sectors;
use crate::fs::ops::{FileOps, InodeOps, SuperOps};
use crate::fs::types::{
    FileRef, Inode, InodeKind, InodePrivate, InodeRef, SuperBlockRef, touch_inode_now,
};
use crate::include::uapi::errno::{
    EEXIST, EINVAL, EIO, EISDIR, ENOENT, ENOSPC, ENOSYS, ENOTDIR, ENOTEMPTY, EROFS,
};
use crate::include::uapi::stat::{S_IFDIR, S_IFLNK, S_IFREG};

use super::dir::read_all as dir_read_all;
use super::extents;
use super::ialloc;
use super::indirect;
use super::inline as ext4_inline;
use super::inode::{OnDiskInode, ext4_inode_of, read_inode, uses_extents, uses_inline};
use super::metadata;
use super::super_block::get_sbi;

const EXT4_FT_REG_FILE: u8 = 1;
const EXT4_FT_DIR: u8 = 2;
const EXT4_FT_SYMLINK: u8 = 7;
const EXT4_NAME_LEN: usize = 255;
const EXT4_DIR_PAD: usize = 4;
const EXT4_APPEND_PREALLOC_THRESHOLD: u64 = 128 * 1024;
const EXT4_APPEND_PREALLOC_BLOCKS: usize = 32;
const EXT4_ALLOC_GROUP_TAIL_GUARD_BLOCKS: u64 = 256;
const EXT4_DIR_ROUND: usize = EXT4_DIR_PAD - 1;
const EXT4_GOOD_OLD_FIRST_INO: u32 = 11;
const EXT4_EXTENTS_FL: u32 = 0x80000;

pub static EXT4_SUPER_OPS: SuperOps = SuperOps {
    name: "ext4",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub static EXT4_DIR_INODE_OPS: InodeOps = InodeOps {
    name: "ext4_dir",
    lookup: Some(ext4_lookup),
    create: Some(ext4_create),
    mkdir: Some(ext4_mkdir),
    unlink: Some(ext4_unlink),
    rmdir: Some(ext4_rmdir),
    rename: Some(ext4_rename),
    symlink: Some(ext4_symlink),
    readlink: None,
};

pub static EXT4_FILE_INODE_OPS: InodeOps = InodeOps {
    name: "ext4_file",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: None,
};

pub static EXT4_SYMLINK_INODE_OPS: InodeOps = InodeOps {
    name: "ext4_symlink",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: Some(ext4_readlink),
};

pub static EXT4_DIR_FILE_OPS: FileOps = FileOps {
    name: "ext4_dir",
    read: None,
    write: None,
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: Some(ext4_readdir),
};

pub static EXT4_FILE_FILE_OPS: FileOps = FileOps {
    name: "ext4_file",
    read: Some(ext4_read),
    write: Some(ext4_write),
    llseek: None,
    fsync: Some(ext4_fsync),
    poll: None,
    ioctl: None,
    mmap: None,
    release: Some(ext4_release),
    readdir: None,
};

fn ext4_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let ext_dir = ext4_inode_of(dir).ok_or(EINVAL)?;
    let entries = dir_read_all(&sbi, &ext_dir)?;
    for e in entries.iter() {
        if e.name == name {
            return read_inode(&sbi, e.inode, &sb);
        }
    }
    Err(ENOENT)
}

fn ext4_create(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32> {
    if name.is_empty() || name.len() > EXT4_NAME_LEN {
        return Err(EINVAL);
    }
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let inode = ext4_new_inode(&sbi, &sb, S_IFREG | (mode & 0o7777), InodeKind::Regular)?;
    if let Err(err) = ext4_add_entry(dir, name, &inode, EXT4_FT_REG_FILE) {
        return Err(err);
    }
    Ok(inode)
}

fn ext4_mkdir(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32> {
    if name.is_empty() || name.len() > EXT4_NAME_LEN {
        return Err(EINVAL);
    }
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let inode = ext4_new_inode(&sbi, &sb, S_IFDIR | (mode & 0o7777), InodeKind::Directory)?;
    ext4_init_new_dir(&sbi, dir, &inode)?;
    if let Err(err) = ext4_add_entry(dir, name, &inode, EXT4_FT_DIR) {
        return Err(err);
    }
    ext4_inc_links(dir, 1)?;
    Ok(inode)
}

fn ext4_symlink(dir: &InodeRef, name: &str, target: &str, mode: u32) -> Result<InodeRef, i32> {
    if name.is_empty() || name.len() > EXT4_NAME_LEN || target.is_empty() {
        return Err(EINVAL);
    }
    if target.len() >= 60 {
        return Err(ENOSYS);
    }
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let inode = ext4_new_inode(&sbi, &sb, S_IFLNK | (mode & 0o7777), InodeKind::Symlink)?;
    let ext_inode = ext4_inode_of(&inode).ok_or(EINVAL)?;
    let mut raw = { *ext_inode.raw.lock() };
    raw.i_flags = 0;
    raw.i_size_lo = (target.len() as u32).to_le();
    raw.i_size_hi = 0;
    raw.i_blocks_lo = 0;
    let mut i_block = [0u32; 15];
    i_block_as_bytes_mut(&mut i_block)[..target.len()].copy_from_slice(target.as_bytes());
    raw.i_block = i_block;
    write_inode_metadata(&sbi, ext_inode.ino, &raw)?;
    *ext_inode.raw.lock() = raw;
    ext_inode
        .i_size
        .store(target.len() as u64, Ordering::Release);
    ext_inode.i_blocks.store(0, Ordering::Release);
    inode.size.store(target.len() as u64, Ordering::Release);
    if let Err(err) = ext4_add_entry(dir, name, &inode, EXT4_FT_SYMLINK) {
        return Err(err);
    }
    Ok(inode)
}

fn ext4_unlink(dir: &InodeRef, name: &str) -> Result<(), i32> {
    ext4_remove_entry(dir, name, false)
}

fn ext4_rmdir(dir: &InodeRef, name: &str) -> Result<(), i32> {
    ext4_remove_entry(dir, name, true)
}

fn ext4_rename(
    old_dir: &InodeRef,
    old_name: &str,
    new_dir: &InodeRef,
    new_name: &str,
) -> Result<(), i32> {
    ext4_validate_dirent_name(old_name)?;
    ext4_validate_dirent_name(new_name)?;

    let old_sb = old_dir.sb.lock().clone().ok_or(EINVAL)?;
    let new_sb = new_dir.sb.lock().clone().ok_or(EINVAL)?;
    if !alloc::sync::Arc::ptr_eq(&old_sb, &new_sb) {
        return Err(EINVAL);
    }
    let sbi = get_sbi(&old_sb).ok_or(EINVAL)?;

    let old_loc = ext4_find_dirent(old_dir, old_name)?.ok_or(ENOENT)?;
    if alloc::sync::Arc::ptr_eq(old_dir, new_dir) && old_name == new_name {
        return Ok(());
    }
    let moved = read_inode(&sbi, old_loc.ino, &old_sb)?;
    if moved.kind == InodeKind::Directory {
        return Err(ENOSYS);
    }

    let replaced = match ext4_find_dirent(new_dir, new_name)? {
        Some(loc) if loc.ino == old_loc.ino => return Ok(()),
        Some(loc) => Some(read_inode(&sbi, loc.ino, &old_sb)?),
        None => None,
    };
    if let Some(inode) = &replaced {
        if moved.kind == InodeKind::Directory && inode.kind != InodeKind::Directory {
            return Err(ENOTDIR);
        }
        if moved.kind != InodeKind::Directory && inode.kind == InodeKind::Directory {
            return Err(EISDIR);
        }
        if inode.kind == InodeKind::Directory {
            return Err(ENOSYS);
        }
    }

    if let Some(inode) = &replaced {
        ext4_remove_dirent_only(new_dir, new_name)?;
        ext4_drop_replaced_inode(&sbi, inode)?;
    }

    if alloc::sync::Arc::ptr_eq(old_dir, new_dir)
        && ext4_rename_dirent_in_place(old_dir, old_name, new_name)?
    {
        return Ok(());
    }

    ext4_add_entry(new_dir, new_name, &moved, old_loc.file_type)?;
    ext4_remove_dirent_only(old_dir, old_name)?;
    Ok(())
}

fn ext4_validate_dirent_name(name: &str) -> Result<(), i32> {
    if name.is_empty() || name == "." || name == ".." || name.len() > EXT4_NAME_LEN {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Ext4DirentLocation {
    lba: u64,
    off: usize,
    rec_len: usize,
    ino: u32,
    file_type: u8,
}

fn ext4_find_dirent(dir: &InodeRef, name: &str) -> Result<Option<Ext4DirentLocation>, i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let dir_inode = ext4_inode_of(dir).ok_or(EINVAL)?;
    let block_size = sbi.block_size as usize;
    let dir_size = dir_inode.i_size.load(Ordering::Acquire) as usize;
    let blocks = dir_size.div_ceil(block_size);
    let i_block_copy = { dir_inode.raw.lock().i_block };

    for lblock in 0..blocks {
        let phys = if uses_extents(&dir_inode) {
            extents::map_block(&sbi, i_block_copy, lblock as u64)?
        } else {
            indirect::map_block(&sbi, i_block_copy, lblock as u64)?
        };
        let Some(phys) = phys else {
            continue;
        };
        let lba = phys * sbi.block_size as u64 / 512;
        let block = read_sectors(&sbi.bdev, lba, sbi.block_size as u64 / 512)?;
        let mut off = 0usize;
        while off + 8 <= block.len() {
            let entry_ino = le_u32(&block, off)?;
            let rec_len = le_u16(&block, off + 4)? as usize;
            let name_len = block[off + 6] as usize;
            if rec_len < 8 || off + rec_len > block.len() {
                return Err(EINVAL);
            }
            if entry_ino != 0
                && name_len == name.len()
                && off + 8 + name_len <= block.len()
                && &block[off + 8..off + 8 + name_len] == name.as_bytes()
            {
                return Ok(Some(Ext4DirentLocation {
                    lba,
                    off,
                    rec_len,
                    ino: entry_ino,
                    file_type: block[off + 7],
                }));
            }
            off += rec_len;
        }
    }
    Ok(None)
}

fn ext4_rename_dirent_in_place(
    dir: &InodeRef,
    old_name: &str,
    new_name: &str,
) -> Result<bool, i32> {
    let loc = ext4_find_dirent(dir, old_name)?.ok_or(ENOENT)?;
    if ext4_dir_rec_len(new_name.len()) > loc.rec_len {
        return Ok(false);
    }

    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let dir_inode = ext4_inode_of(dir).ok_or(EINVAL)?;
    let mut block = read_sectors(&sbi.bdev, loc.lba, sbi.block_size as u64 / 512)?;
    put_le_u32(&mut block, loc.off, loc.ino)?;
    put_le_u16(&mut block, loc.off + 4, loc.rec_len as u16)?;
    block[loc.off + 6] = new_name.len() as u8;
    block[loc.off + 7] = loc.file_type;
    let name_start = loc.off + 8;
    block[name_start..loc.off + loc.rec_len].fill(0);
    block[name_start..name_start + new_name.len()].copy_from_slice(new_name.as_bytes());
    write_block(&sbi.bdev, loc.lba, &block)?;
    super::dir::invalidate_cache(&dir_inode);
    touch_inode_now(dir);
    Ok(true)
}

fn ext4_remove_dirent_only(dir: &InodeRef, name: &str) -> Result<u32, i32> {
    ext4_validate_dirent_name(name)?;
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let dir_inode = ext4_inode_of(dir).ok_or(EINVAL)?;
    let block_size = sbi.block_size as usize;
    let dir_size = dir_inode.i_size.load(Ordering::Acquire) as usize;
    let blocks = dir_size.div_ceil(block_size);
    let i_block_copy = { dir_inode.raw.lock().i_block };

    for lblock in 0..blocks {
        let phys = if uses_extents(&dir_inode) {
            extents::map_block(&sbi, i_block_copy, lblock as u64)?
        } else {
            indirect::map_block(&sbi, i_block_copy, lblock as u64)?
        };
        let Some(phys) = phys else {
            continue;
        };
        let lba = phys * sbi.block_size as u64 / 512;
        let mut block = read_sectors(&sbi.bdev, lba, sbi.block_size as u64 / 512)?;
        if let Some(removed_ino) = ext4_remove_dirent_from_block(&mut block, name)? {
            write_block(&sbi.bdev, lba, &block)?;
            super::dir::invalidate_cache(&dir_inode);
            touch_inode_now(dir);
            return Ok(removed_ino);
        }
    }
    Err(ENOENT)
}

fn ext4_drop_replaced_inode(sbi: &super::Ext4Sbi, inode: &InodeRef) -> Result<(), i32> {
    if inode.kind == InodeKind::Directory {
        return Err(ENOSYS);
    }
    ext4_dec_links(inode, 1)?;
    if let Some(ext_inode) = ext4_inode_of(inode) {
        super::dir::invalidate_cache(&ext_inode);
        touch_inode_now(inode);
    }
    if inode.nlink.load(Ordering::Acquire) == 0 {
        ext4_delete_inode(sbi, inode)?;
    }
    Ok(())
}

fn ext4_new_inode(
    sbi: &super::Ext4Sbi,
    sb: &SuperBlockRef,
    mode: u32,
    kind: InodeKind,
) -> Result<InodeRef, i32> {
    let ino = ext4_claim_free_inode(sbi, kind == InodeKind::Directory)?;
    let raw = ext4_new_raw_inode(sbi, mode, kind);
    write_inode_metadata(sbi, ino, &raw)?;

    let ext_inode = alloc::sync::Arc::new(super::Ext4Inode {
        ino,
        i_mode: mode as u16,
        i_size: core::sync::atomic::AtomicU64::new(0),
        i_blocks: core::sync::atomic::AtomicU64::new(0),
        raw: spin::Mutex::new(raw),
        dir_cache: spin::Mutex::new(None),
        append_reservation: spin::Mutex::new(None),
    });
    let inode = Inode::new(
        ino as u64,
        kind,
        mode,
        match kind {
            InodeKind::Directory => &EXT4_DIR_INODE_OPS,
            InodeKind::Symlink => &EXT4_SYMLINK_INODE_OPS,
            _ => &EXT4_FILE_INODE_OPS,
        },
        match kind {
            InodeKind::Directory => &EXT4_DIR_FILE_OPS,
            _ => &EXT4_FILE_FILE_OPS,
        },
        InodePrivate::Opaque(alloc::sync::Arc::into_raw(ext_inode) as usize),
    );
    *inode.sb.lock() = Some(sb.clone());
    Ok(inode)
}

fn ext4_new_raw_inode(sbi: &super::Ext4Sbi, mode: u32, kind: InodeKind) -> OnDiskInode {
    let nlink = if kind == InodeKind::Directory { 2 } else { 1 };
    let mut i_block = [0u32; 15];
    if matches!(
        kind,
        InodeKind::Directory | InodeKind::Regular | InodeKind::Symlink
    ) {
        let _ = init_extent_header(
            i_block_as_bytes_mut(&mut i_block),
            0,
            extent_node_max_entries(60) as u16,
            0,
        );
    }
    OnDiskInode {
        i_mode: (mode as u16).to_le(),
        i_uid: 0,
        i_size_lo: 0,
        i_atime: 0,
        i_ctime: 0,
        i_mtime: 0,
        i_dtime: 0,
        i_gid: 0,
        i_links_count: (nlink as u16).to_le(),
        i_blocks_lo: 0,
        i_flags: if matches!(
            kind,
            InodeKind::Directory | InodeKind::Regular | InodeKind::Symlink
        ) {
            EXT4_EXTENTS_FL.to_le()
        } else {
            0
        },
        _osd1: 0,
        i_block,
        i_generation: 0,
        i_file_acl_lo: 0,
        i_size_hi: 0,
        i_obso_faddr: 0,
        _osd2: [0; 12],
        i_extra_isize: sbi.want_extra_isize.to_le(),
        i_checksum_hi: 0,
        i_ctime_extra: 0,
        i_mtime_extra: 0,
        i_atime_extra: 0,
        i_crtime: 0,
        i_crtime_extra: 0,
        i_version_hi: 0,
        i_projid: 0,
    }
}

fn ext4_claim_free_inode(sbi: &super::Ext4Sbi, directory: bool) -> Result<u32, i32> {
    let mut group_index = 0usize;
    while group_index < sbi.group_descs.len() {
        let gd = &sbi.group_descs[group_index];
        let bitmap_lba = gd.bg_inode_bitmap * sbi.block_size as u64 / 512;
        let mut bitmap = read_sectors(&sbi.bdev, bitmap_lba, sbi.block_size as u64 / 512)?;
        let mut bit = 0usize;
        while bit < sbi.inodes_per_group as usize {
            let ino = (group_index as u32)
                .checked_mul(sbi.inodes_per_group)
                .and_then(|base| base.checked_add(bit as u32 + 1))
                .ok_or(EINVAL)?;
            if ino < sbi.first_ino.max(EXT4_GOOD_OLD_FIRST_INO) {
                bit += 1;
                continue;
            }
            if ino as u64 > sbi.inodes_count {
                break;
            }
            if !metadata::bitmap_test(&bitmap, bit)? {
                metadata::bitmap_set(&mut bitmap, bit)?;
                crate::fs::jbd2::transaction::jbd2_journal_write_metadata_block(
                    &sbi.bdev,
                    gd.bg_inode_bitmap,
                    sbi.block_size as u64,
                    &bitmap,
                )?;
                decrement_group_free_inodes(sbi, group_index)?;
                if directory {
                    increment_group_used_dirs(sbi, group_index)?;
                }
                decrement_super_free_inodes(sbi)?;
                return Ok(ino);
            }
            bit += 1;
        }
        group_index += 1;
    }
    Err(ENOSPC)
}

fn ext4_add_entry(dir: &InodeRef, name: &str, inode: &InodeRef, file_type: u8) -> Result<(), i32> {
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let dir_inode = ext4_inode_of(dir).ok_or(EINVAL)?;
    let mut raw_copy = { *dir_inode.raw.lock() };
    let mut i_block_copy = raw_copy.i_block;
    let block_size = sbi.block_size as usize;
    let dir_size = dir_inode.i_size.load(Ordering::Acquire) as usize;
    let blocks = dir_size.div_ceil(block_size);

    // Mirrors vendor/linux/fs/ext4/namei.c::__ext4_add_entry's linear
    // directory fallback: scan each leaf block for a matching/free dirent,
    // splitting the record in ext4_insert_dentry() form when needed.
    for lblock in 0..blocks {
        let phys = if uses_extents(&dir_inode) {
            extents::map_block(&sbi, i_block_copy, lblock as u64)?
        } else {
            indirect::map_block(&sbi, i_block_copy, lblock as u64)?
        };
        let Some(phys) = phys else {
            continue;
        };
        let lba = phys * sbi.block_size as u64 / 512;
        let mut block = read_sectors(&sbi.bdev, lba, sbi.block_size as u64 / 512)?;
        match ext4_insert_dirent_into_block(&mut block, name, inode.ino as u32, file_type)? {
            DirentInsert::Inserted => {
                write_block(&sbi.bdev, lba, &block)?;
                super::dir::invalidate_cache(&dir_inode);
                touch_inode_now(dir);
                return Ok(());
            }
            DirentInsert::NoSpace => {}
        }
    }

    let lblock = blocks as u64;
    let allocation =
        ext4_alloc_extent_append_block(&sbi, &mut raw_copy, &mut i_block_copy, lblock)?;
    let mut block = alloc::vec![0u8; block_size];
    put_le_u16(&mut block, 4, block_size as u16)?;
    match ext4_insert_dirent_into_block(&mut block, name, inode.ino as u32, file_type)? {
        DirentInsert::Inserted => {}
        DirentInsert::NoSpace => return Err(ENOSPC),
    }
    write_block(
        &sbi.bdev,
        allocation.data_block * sbi.block_size as u64 / 512,
        &block,
    )?;

    let new_size = (blocks + 1).checked_mul(block_size).ok_or(EINVAL)? as u64;
    raw_copy.i_block = i_block_copy;
    raw_copy.i_size_lo = (new_size as u32).to_le();
    raw_copy.i_size_hi = ((new_size >> 32) as u32).to_le();
    ext4_add_i_blocks(
        &mut raw_copy,
        allocation.allocated_blocks,
        sbi.block_size as u64,
    )?;
    write_inode_metadata(&sbi, dir_inode.ino, &raw_copy)?;
    *dir_inode.raw.lock() = raw_copy;
    dir_inode.i_size.store(new_size, Ordering::Release);
    dir_inode
        .i_blocks
        .store(u32::from_le(raw_copy.i_blocks_lo) as u64, Ordering::Release);
    dir.size.store(new_size, Ordering::Release);
    super::dir::invalidate_cache(&dir_inode);
    touch_inode_now(dir);
    Ok(())
}

fn ext4_remove_entry(dir: &InodeRef, name: &str, remove_dir: bool) -> Result<(), i32> {
    if name.is_empty() || name == "." || name == ".." || name.len() > EXT4_NAME_LEN {
        return Err(EINVAL);
    }
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let dir_inode = ext4_inode_of(dir).ok_or(EINVAL)?;
    let block_size = sbi.block_size as usize;
    let dir_size = dir_inode.i_size.load(Ordering::Acquire) as usize;
    let blocks = dir_size.div_ceil(block_size);
    let i_block_copy = { dir_inode.raw.lock().i_block };

    for lblock in 0..blocks {
        let phys = if uses_extents(&dir_inode) {
            extents::map_block(&sbi, i_block_copy, lblock as u64)?
        } else {
            indirect::map_block(&sbi, i_block_copy, lblock as u64)?
        };
        let Some(phys) = phys else {
            continue;
        };
        let lba = phys * sbi.block_size as u64 / 512;
        let mut block = read_sectors(&sbi.bdev, lba, sbi.block_size as u64 / 512)?;
        match ext4_remove_dirent_from_block(&mut block, name)? {
            Some(removed_ino) => {
                let removed = read_inode(&sbi, removed_ino, &sb)?;
                if remove_dir {
                    if removed.kind != InodeKind::Directory {
                        return Err(ENOTDIR);
                    }
                    ext4_ensure_empty_dir(&sbi, &removed)?;
                } else if removed.kind == InodeKind::Directory {
                    return Err(EISDIR);
                }

                write_block(&sbi.bdev, lba, &block)?;
                if remove_dir {
                    ext4_set_links(&removed, 0)?;
                    ext4_dec_links(dir, 1)?;
                } else {
                    ext4_dec_links(&removed, 1)?;
                }
                if let Some(removed_ext) = ext4_inode_of(&removed) {
                    super::dir::invalidate_cache(&removed_ext);
                    touch_inode_now(&removed);
                }
                if removed.nlink.load(Ordering::Acquire) == 0 {
                    ext4_delete_inode(&sbi, &removed)?;
                }
                super::dir::invalidate_cache(&dir_inode);
                touch_inode_now(dir);
                return Ok(());
            }
            None => {}
        }
    }
    Err(ENOENT)
}

fn ext4_remove_dirent_from_block(block: &mut [u8], name: &str) -> Result<Option<u32>, i32> {
    let mut off = 0usize;
    let mut prev_off: Option<usize> = None;
    while off + 8 <= block.len() {
        let entry_ino = le_u32(block, off)?;
        let rec_len = le_u16(block, off + 4)? as usize;
        let name_len = block[off + 6] as usize;
        if rec_len < 8 || off + rec_len > block.len() {
            return Err(EINVAL);
        }
        if entry_ino != 0
            && name_len == name.len()
            && off + 8 + name_len <= block.len()
            && &block[off + 8..off + 8 + name_len] == name.as_bytes()
        {
            if let Some(prev) = prev_off {
                let prev_len = le_u16(block, prev + 4)? as usize;
                let merged = prev_len.checked_add(rec_len).ok_or(EINVAL)?;
                put_le_u16(block, prev + 4, merged as u16)?;
            } else {
                put_le_u32(block, off, 0)?;
            }
            return Ok(Some(entry_ino));
        }
        prev_off = Some(off);
        off += rec_len;
    }
    Ok(None)
}

fn ext4_ensure_empty_dir(sbi: &super::Ext4Sbi, inode: &InodeRef) -> Result<(), i32> {
    let ext_inode = ext4_inode_of(inode).ok_or(EINVAL)?;
    let entries = dir_read_all(sbi, &ext_inode)?;
    if entries
        .iter()
        .any(|entry| entry.name != "." && entry.name != "..")
    {
        return Err(ENOTEMPTY);
    }
    Ok(())
}

fn ext4_delete_inode(sbi: &super::Ext4Sbi, inode: &InodeRef) -> Result<(), i32> {
    let ext_inode = ext4_inode_of(inode).ok_or(EINVAL)?;
    let mut raw = { *ext_inode.raw.lock() };
    let fully_freed = match ext4_free_inode_data_blocks(sbi, &raw)? {
        Some(_) => true,
        None => false,
    };

    if fully_freed {
        raw = ext4_empty_raw_inode();
    } else {
        raw.i_links_count = 0;
        raw.i_dtime = 1u32.to_le();
    }
    write_inode_metadata(sbi, ext_inode.ino, &raw)?;
    *ext_inode.raw.lock() = raw;
    inode.nlink.store(0, Ordering::Release);
    if fully_freed {
        ext_inode.i_size.store(0, Ordering::Release);
        ext_inode.i_blocks.store(0, Ordering::Release);
        inode.size.store(0, Ordering::Release);
        ext4_release_inode(sbi, ext_inode.ino, inode.kind == InodeKind::Directory)?;
    }
    Ok(())
}

fn ext4_free_inode_data_blocks(
    sbi: &super::Ext4Sbi,
    raw: &OnDiskInode,
) -> Result<Option<u64>, i32> {
    let i_blocks = u32::from_le(raw.i_blocks_lo) as u64;
    if i_blocks == 0 {
        return Ok(Some(0));
    }
    if u32::from_le(raw.i_file_acl_lo) != 0 {
        return Ok(None);
    }

    let i_flags = u32::from_le(raw.i_flags);
    if i_flags & EXT4_EXTENTS_FL == 0 {
        return Ok(None);
    }

    let i_block = raw.i_block;
    let bytes = i_block_as_bytes(&i_block);
    if le_u16(bytes, 0)? != extents::EXT4_EXT_MAGIC {
        return Err(EINVAL);
    }
    let entries = le_u16(bytes, 2)? as usize;
    let max_entries = le_u16(bytes, 4)? as usize;
    let depth = le_u16(bytes, 6)?;
    if depth != 0 {
        return Ok(None);
    }
    if entries > max_entries || 12 + entries * 12 > bytes.len() {
        return Err(EINVAL);
    }

    let mut ranges = Vec::new();
    let mut extent_blocks = 0u64;
    for index in 0..entries {
        let off = 12 + index * 12;
        let len = (le_u16(bytes, off + 4)? & 0x7fff) as u64;
        let phys = ((le_u16(bytes, off + 6)? as u64) << 32) | le_u32(bytes, off + 8)? as u64;
        if len == 0 {
            return Err(EINVAL);
        }
        let end = phys.checked_add(len).ok_or(EINVAL)?;
        extent_blocks = extent_blocks.checked_add(len).ok_or(EINVAL)?;
        ranges.push((phys, end));
    }
    let sectors = extent_blocks
        .checked_mul((sbi.block_size / 512) as u64)
        .ok_or(EINVAL)?;
    if sectors != i_blocks {
        return Ok(None);
    }

    let mut freed = 0u64;
    for (first, end) in ranges {
        for block in first..end {
            ext4_release_data_block(sbi, block)?;
            freed = freed.checked_add(1).ok_or(EINVAL)?;
        }
    }
    Ok(Some(freed))
}

fn ext4_empty_raw_inode() -> OnDiskInode {
    OnDiskInode {
        i_mode: 0,
        i_uid: 0,
        i_size_lo: 0,
        i_atime: 0,
        i_ctime: 0,
        i_mtime: 0,
        i_dtime: 0,
        i_gid: 0,
        i_links_count: 0,
        i_blocks_lo: 0,
        i_flags: 0,
        _osd1: 0,
        i_block: [0; 15],
        i_generation: 0,
        i_file_acl_lo: 0,
        i_size_hi: 0,
        i_obso_faddr: 0,
        _osd2: [0; 12],
        i_extra_isize: 0,
        i_checksum_hi: 0,
        i_ctime_extra: 0,
        i_mtime_extra: 0,
        i_atime_extra: 0,
        i_crtime: 0,
        i_crtime_extra: 0,
        i_version_hi: 0,
        i_projid: 0,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DirentInsert {
    Inserted,
    NoSpace,
}

fn ext4_insert_dirent_into_block(
    block: &mut [u8],
    name: &str,
    ino: u32,
    file_type: u8,
) -> Result<DirentInsert, i32> {
    let required = ext4_dir_rec_len(name.len());
    let mut off = 0usize;
    while off + 8 <= block.len() {
        let entry_ino = le_u32(block, off)?;
        let rec_len = le_u16(block, off + 4)? as usize;
        let name_len = block[off + 6] as usize;
        if rec_len < 8 || off + rec_len > block.len() {
            return Err(EINVAL);
        }
        if entry_ino != 0 && name_len == name.len() && off + 8 + name_len <= block.len() {
            if &block[off + 8..off + 8 + name_len] == name.as_bytes() {
                return Err(EEXIST);
            }
        }
        let used = if entry_ino == 0 {
            0
        } else {
            ext4_dir_rec_len(name_len)
        };
        let available = if entry_ino == 0 {
            rec_len
        } else {
            rec_len.saturating_sub(used)
        };
        if available >= required {
            let insert_off = if entry_ino == 0 { off } else { off + used };
            if entry_ino != 0 {
                put_le_u16(block, off + 4, used as u16)?;
                put_le_u16(block, insert_off + 4, (rec_len - used) as u16)?;
            }
            put_le_u32(block, insert_off, ino)?;
            block[insert_off + 6] = name.len() as u8;
            block[insert_off + 7] = file_type;
            let name_start = insert_off + 8;
            block[name_start..name_start + name.len()].copy_from_slice(name.as_bytes());
            return Ok(DirentInsert::Inserted);
        }
        off += rec_len;
    }
    Ok(DirentInsert::NoSpace)
}

fn ext4_init_new_dir(sbi: &super::Ext4Sbi, parent: &InodeRef, inode: &InodeRef) -> Result<(), i32> {
    let child_ext = ext4_inode_of(inode).ok_or(EINVAL)?;
    let parent_ino = parent.ino as u32;
    let mut raw_copy = { *child_ext.raw.lock() };
    let mut i_block_copy = raw_copy.i_block;
    let allocation = ext4_alloc_extent_append_block(sbi, &mut raw_copy, &mut i_block_copy, 0)?;
    let block_size = sbi.block_size as usize;
    let mut block = alloc::vec![0u8; block_size];

    // Mirrors vendor/linux/fs/ext4/namei.c::ext4_init_dirblock.
    let dot_len = ext4_dir_rec_len(1);
    put_le_u32(&mut block, 0, inode.ino as u32)?;
    put_le_u16(&mut block, 4, dot_len as u16)?;
    block[6] = 1;
    block[7] = EXT4_FT_DIR;
    block[8] = b'.';

    put_le_u32(&mut block, dot_len, parent_ino)?;
    put_le_u16(&mut block, dot_len + 4, (block_size - dot_len) as u16)?;
    block[dot_len + 6] = 2;
    block[dot_len + 7] = EXT4_FT_DIR;
    block[dot_len + 8] = b'.';
    block[dot_len + 9] = b'.';

    write_block(
        &sbi.bdev,
        allocation.data_block * sbi.block_size as u64 / 512,
        &block,
    )?;

    raw_copy.i_block = i_block_copy;
    raw_copy.i_size_lo = sbi.block_size.to_le();
    raw_copy.i_size_hi = 0;
    ext4_add_i_blocks(
        &mut raw_copy,
        allocation.allocated_blocks,
        sbi.block_size as u64,
    )?;
    write_inode_metadata(sbi, child_ext.ino, &raw_copy)?;
    *child_ext.raw.lock() = raw_copy;
    child_ext
        .i_size
        .store(sbi.block_size as u64, Ordering::Release);
    child_ext
        .i_blocks
        .store(u32::from_le(raw_copy.i_blocks_lo) as u64, Ordering::Release);
    inode.size.store(sbi.block_size as u64, Ordering::Release);
    super::dir::invalidate_cache(&child_ext);
    Ok(())
}

fn ext4_dir_rec_len(name_len: usize) -> usize {
    (name_len + 8 + EXT4_DIR_ROUND) & !EXT4_DIR_ROUND
}

fn ext4_add_i_blocks(raw: &mut OnDiskInode, blocks: u64, block_size: u64) -> Result<(), i32> {
    let sectors = blocks.checked_mul(block_size / 512).ok_or(EINVAL)?;
    let old = u32::from_le(raw.i_blocks_lo) as u64;
    raw.i_blocks_lo = old.checked_add(sectors).ok_or(EINVAL)?.min(u32::MAX as u64) as u32;
    raw.i_blocks_lo = raw.i_blocks_lo.to_le();
    Ok(())
}

fn ext4_inc_links(inode: &InodeRef, by: u16) -> Result<(), i32> {
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let ext_inode = ext4_inode_of(inode).ok_or(EINVAL)?;
    let mut raw = { *ext_inode.raw.lock() };
    let links = u16::from_le(raw.i_links_count)
        .checked_add(by)
        .ok_or(EINVAL)?;
    raw.i_links_count = links.to_le();
    write_inode_metadata(&sbi, ext_inode.ino, &raw)?;
    *ext_inode.raw.lock() = raw;
    inode.nlink.store(links as u32, Ordering::Release);
    Ok(())
}

fn ext4_dec_links(inode: &InodeRef, by: u16) -> Result<(), i32> {
    let current = inode.nlink.load(Ordering::Acquire) as u16;
    let links = current.checked_sub(by).ok_or(EINVAL)?;
    ext4_set_links(inode, links)
}

fn ext4_set_links(inode: &InodeRef, links: u16) -> Result<(), i32> {
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let ext_inode = ext4_inode_of(inode).ok_or(EINVAL)?;
    let mut raw = { *ext_inode.raw.lock() };
    raw.i_links_count = links.to_le();
    write_inode_metadata(&sbi, ext_inode.ino, &raw)?;
    *ext_inode.raw.lock() = raw;
    inode.nlink.store(links as u32, Ordering::Release);
    Ok(())
}

fn ext4_readdir(file: &FileRef) -> Result<Option<(alloc::string::String, u64, InodeKind)>, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let ext_dir = ext4_inode_of(&inode).ok_or(EINVAL)?;
    let entries = dir_read_all(&sbi, &ext_dir)?;
    let mut idx = file.private.lock();
    if (*idx as usize) >= entries.len() {
        return Ok(None);
    }
    let e = entries[*idx as usize].clone();
    let kind = super::dir::kind_for_type(e.file_type);
    *idx += 1;
    Ok(Some((e.name, e.inode as u64, kind)))
}

fn ext4_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let ext_inode = ext4_inode_of(&inode).ok_or(EINVAL)?;
    let isize_bytes = inode
        .size
        .load(Ordering::Acquire)
        .max(ext_inode.i_size.load(Ordering::Acquire));

    if *pos >= isize_bytes {
        return Ok(0);
    }
    let max = (isize_bytes - *pos).min(buf.len() as u64) as usize;

    // Fast path — inline_data stores bytes in i_block.
    if uses_inline(&ext_inode) {
        let inline = ext4_inline::inline_payload(&ext_inode, isize_bytes as usize);
        let start = *pos as usize;
        if start >= inline.len() {
            return Ok(0);
        }
        let end = (start + max).min(inline.len());
        let n = end - start;
        buf[..n].copy_from_slice(&inline[start..end]);
        *pos += n as u64;
        return Ok(n);
    }

    let block_size = sbi.block_size as u64;
    let i_block_copy = { ext_inode.raw.lock().i_block };
    let mut copied = 0usize;
    while copied < max {
        let abs = *pos + copied as u64;
        let lblock = abs / block_size;
        let in_block = (abs % block_size) as usize;
        let phys = if uses_extents(&ext_inode) {
            extents::map_block(&sbi, i_block_copy, lblock)?
        } else {
            indirect::map_block(&sbi, i_block_copy, lblock)?
        };
        let block_buf: Vec<u8> = match phys {
            Some(p) => {
                let lba = p * block_size / 512;
                read_sectors(&sbi.bdev, lba, block_size / 512)?
            }
            None => alloc::vec![0u8; block_size as usize],
        };
        let copy = (block_size as usize - in_block).min(max - copied);
        buf[copied..copied + copy].copy_from_slice(&block_buf[in_block..in_block + copy]);
        copied += copy;
    }
    *pos += copied as u64;
    Ok(copied)
}

fn ext4_readlink(inode: &InodeRef, buf: &mut [u8]) -> Result<usize, i32> {
    let ext_inode = ext4_inode_of(inode).ok_or(EINVAL)?;
    let isize_bytes = inode
        .size
        .load(Ordering::Acquire)
        .max(ext_inode.i_size.load(Ordering::Acquire));
    let max = (isize_bytes as usize).min(buf.len());
    if max == 0 {
        return Ok(0);
    }

    let raw_copy = { *ext_inode.raw.lock() };
    let sb = inode.sb.lock().clone();
    let block_size = sb
        .as_ref()
        .and_then(get_sbi)
        .map(|sbi| sbi.block_size)
        .unwrap_or(super::EXT4_BLOCK_SIZE_DEFAULT);

    // vendor/linux/fs/ext4/inode.c::ext4_inode_is_fast_symlink stores fast
    // symlink bytes directly in EXT4_I(inode)->i_data, backed here by i_block.
    if ext4_inode_is_fast_symlink(&ext_inode, &raw_copy, isize_bytes, block_size) {
        let i_block_copy = raw_copy.i_block;
        let target = i_block_as_bytes(&i_block_copy);
        let n = max.min(target.len());
        buf[..n].copy_from_slice(&target[..n]);
        return Ok(n);
    }

    // vendor/linux/fs/ext4/symlink.c::ext4_get_link reads inline-data
    // symlinks before falling back to logical block zero.
    if uses_inline(&ext_inode) {
        let inline = ext4_inline::inline_payload(&ext_inode, isize_bytes as usize);
        let n = max.min(inline.len());
        buf[..n].copy_from_slice(&inline[..n]);
        return Ok(n);
    }

    let sb = sb.ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let block_size = sbi.block_size as u64;
    let i_block_copy = raw_copy.i_block;
    let mut copied = 0usize;
    while copied < max {
        let abs = copied as u64;
        let lblock = abs / block_size;
        let in_block = (abs % block_size) as usize;
        let phys = if uses_extents(&ext_inode) {
            extents::map_block(&sbi, i_block_copy, lblock)?
        } else {
            indirect::map_block(&sbi, i_block_copy, lblock)?
        };
        let phys = phys.ok_or(EIO)?;
        let block_buf = read_sectors(&sbi.bdev, phys * block_size / 512, block_size / 512)?;
        let copy = (block_size as usize - in_block).min(max - copied);
        buf[copied..copied + copy].copy_from_slice(&block_buf[in_block..in_block + copy]);
        copied += copy;
    }
    Ok(copied)
}

fn ext4_inode_is_fast_symlink(
    ext_inode: &super::Ext4Inode,
    raw: &OnDiskInode,
    size: u64,
    block_size: u32,
) -> bool {
    const EXT4_N_BLOCKS: u64 = 15;
    const EXT4_FAST_SYMLINK_MAX: u64 = EXT4_N_BLOCKS * 4;

    if size == 0 || size >= EXT4_FAST_SYMLINK_MAX || uses_inline(ext_inode) {
        return false;
    }

    let ea_blocks = if u32::from_le(raw.i_file_acl_lo) != 0 {
        (block_size as u64) / 512
    } else {
        0
    };
    ext_inode
        .i_blocks
        .load(Ordering::Acquire)
        .saturating_sub(ea_blocks)
        == 0
}

fn ext4_write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    if buf.is_empty() {
        return Ok(0);
    }

    let inode = file.inode().ok_or(EINVAL)?;
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let ext_inode = ext4_inode_of(&inode).ok_or(EINVAL)?;
    if uses_inline(&ext_inode) {
        return Err(EROFS);
    }

    let file_size = inode
        .size
        .load(Ordering::Acquire)
        .max(ext_inode.i_size.load(Ordering::Acquire));
    let max = buf.len();
    let block_size = sbi.block_size as u64;
    let mut raw_copy = { *ext_inode.raw.lock() };
    let mut i_block_copy = raw_copy.i_block;
    let mut allocated_blocks = 0u64;
    let mut copied = 0usize;

    while copied < max {
        let abs = (*pos).checked_add(copied as u64).ok_or(EINVAL)?;
        let lblock = abs / block_size;
        let in_block = (abs % block_size) as usize;
        let mut newly_allocated_data_block = false;
        let mapped_phys = if uses_extents(&ext_inode) {
            extents::map_block(&sbi, i_block_copy, lblock)?
        } else {
            indirect::map_block(&sbi, i_block_copy, lblock)?
        };
        let phys = match mapped_phys {
            Some(phys) => phys,
            None if uses_extents(&ext_inode) && !sbi.group_descs.is_empty() => {
                let remaining = max - copied;
                let append_plan =
                    append_data_blocks_for_write(remaining, in_block, block_size, abs);
                let allocation = ext4_alloc_extent_append_blocks(
                    &sbi,
                    &mut raw_copy,
                    &mut i_block_copy,
                    lblock,
                    append_plan.map_blocks,
                    append_plan.reserve_blocks,
                    Some(&ext_inode.append_reservation),
                )?;
                allocated_blocks += allocation.allocated_blocks;
                if in_block == 0 && allocation.data_blocks > 1 {
                    let copy = (allocation.data_blocks as usize)
                        .saturating_mul(block_size as usize)
                        .min(remaining);
                    write_block(
                        &sbi.bdev,
                        allocation.data_block * block_size / 512,
                        &buf[copied..copied + copy],
                    )?;
                    copied += copy;
                    continue;
                }
                newly_allocated_data_block = true;
                allocation.data_block
            }
            None if copied == 0 => return Err(EINVAL),
            None => break,
        };

        let lba = phys * block_size / 512;
        let copy = (block_size as usize - in_block).min(max - copied);
        if in_block == 0 && copy == block_size as usize {
            write_block(&sbi.bdev, lba, &buf[copied..copied + copy])?;
        } else {
            let mut block_buf = if newly_allocated_data_block {
                alloc::vec![0u8; block_size as usize]
            } else {
                read_sectors(&sbi.bdev, lba, block_size / 512)?
            };
            block_buf[in_block..in_block + copy].copy_from_slice(&buf[copied..copied + copy]);
            write_block(&sbi.bdev, lba, &block_buf)?;
        }
        copied += copy;
    }

    let end = (*pos).checked_add(copied as u64).ok_or(EINVAL)?;
    *pos += copied as u64;
    if copied > 0 {
        let new_size = file_size.max(end);
        raw_copy.i_size_lo = (new_size as u32).to_le();
        raw_copy.i_size_hi = ((new_size >> 32) as u32).to_le();
        if allocated_blocks != 0 {
            let sectors = allocated_blocks
                .checked_mul(block_size / 512)
                .ok_or(EINVAL)?;
            let old_blocks = u32::from_le(raw_copy.i_blocks_lo) as u64;
            raw_copy.i_blocks_lo = old_blocks
                .checked_add(sectors)
                .ok_or(EINVAL)?
                .min(u32::MAX as u64) as u32;
            raw_copy.i_blocks_lo = raw_copy.i_blocks_lo.to_le();
        }
        write_inode_metadata(&sbi, ext_inode.ino, &raw_copy)?;
        *ext_inode.raw.lock() = raw_copy;
        ext_inode.i_size.store(new_size, Ordering::Release);
        if allocated_blocks != 0 {
            ext_inode
                .i_blocks
                .fetch_add(allocated_blocks * (block_size / 512), Ordering::AcqRel);
        }
        inode.size.fetch_max(new_size, Ordering::AcqRel);
        touch_inode_now(&inode);
    }
    Ok(copied)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Ext4AppendWritePlan {
    map_blocks: u16,
    reserve_blocks: u16,
}

fn append_data_blocks_for_write(
    remaining: usize,
    in_block: usize,
    block_size: u64,
    file_offset: u64,
) -> Ext4AppendWritePlan {
    if in_block != 0 {
        return Ext4AppendWritePlan {
            map_blocks: 1,
            reserve_blocks: 1,
        };
    }
    let block_size = block_size as usize;
    if block_size == 0 {
        return Ext4AppendWritePlan {
            map_blocks: 1,
            reserve_blocks: 1,
        };
    }
    let full_blocks = remaining / block_size;
    let map_blocks = if full_blocks == 0 { 1 } else { full_blocks };
    let reserve_blocks = if file_offset >= EXT4_APPEND_PREALLOC_THRESHOLD {
        map_blocks.max(EXT4_APPEND_PREALLOC_BLOCKS)
    } else {
        map_blocks
    };
    Ext4AppendWritePlan {
        map_blocks: map_blocks.min(0x7fff).max(1) as u16,
        reserve_blocks: reserve_blocks.min(0x7fff).max(1) as u16,
    }
}

fn ext4_fsync(file: &FileRef) -> Result<(), i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    ext4_release_inode_reservation(&inode)
}

fn ext4_release(file: FileRef) {
    if let Some(inode) = file.inode() {
        let _ = ext4_release_inode_reservation(&inode);
    }
}

fn ext4_release_inode_reservation(inode: &InodeRef) -> Result<(), i32> {
    let sb = inode.sb.lock().clone().ok_or(EINVAL)?;
    let sbi = get_sbi(&sb).ok_or(EINVAL)?;
    let ext_inode = ext4_inode_of(inode).ok_or(EINVAL)?;
    ext4_release_reserved_data_blocks(&sbi, &ext_inode.append_reservation)
}

fn normalized_append_counts(map_blocks: u16, reserve_blocks: u16) -> (u16, u16) {
    let map_blocks = map_blocks.max(1).min(0x7fff);
    let reserve_blocks = reserve_blocks.max(map_blocks).min(0x7fff);
    (map_blocks, reserve_blocks)
}

fn append_reservation_from_run(
    reservation: Option<&spin::Mutex<Option<super::Ext4BlockReservation>>>,
    run: Ext4BlockRun,
    map_blocks: u16,
) -> Ext4BlockRun {
    let map_count = run.count.min(map_blocks).max(1);
    if let Some(reservation) = reservation {
        if run.count > map_count {
            *reservation.lock() = Some(super::Ext4BlockReservation {
                start: run.start + map_count as u64,
                count: run.count - map_count,
            });
        }
    }
    Ext4BlockRun {
        start: run.start,
        count: map_count,
    }
}

fn take_append_reservation(
    reservation: Option<&spin::Mutex<Option<super::Ext4BlockReservation>>>,
    start: u64,
    map_blocks: u16,
) -> Option<Ext4BlockRun> {
    let reservation = reservation?;
    let mut guard = reservation.lock();
    let reserved = guard.as_mut()?;
    if reserved.start != start || reserved.count == 0 {
        return None;
    }
    let count = reserved.count.min(map_blocks).max(1);
    let run = Ext4BlockRun {
        start: reserved.start,
        count,
    };
    reserved.start += count as u64;
    reserved.count -= count;
    if reserved.count == 0 {
        *guard = None;
    }
    Some(run)
}

fn ext4_claim_append_data_blocks(
    sbi: &super::Ext4Sbi,
    reservation: Option<&spin::Mutex<Option<super::Ext4BlockReservation>>>,
    start: u64,
    map_blocks: u16,
    reserve_blocks: u16,
    require_start: bool,
) -> Result<Option<Ext4BlockRun>, i32> {
    let (map_blocks, reserve_blocks) = normalized_append_counts(map_blocks, reserve_blocks);
    if let Some(run) = take_append_reservation(reservation, start, map_blocks) {
        return Ok(Some(run));
    }
    if let Some(reservation) = reservation {
        let should_release = reservation
            .lock()
            .as_ref()
            .map(|reserved| reserved.start != start)
            .unwrap_or(false);
        if should_release {
            ext4_release_reserved_data_blocks(sbi, reservation)?;
        }
    }

    let run = if require_start {
        ext4_try_claim_contiguous_data_blocks(sbi, start, reserve_blocks)?
    } else {
        match ext4_claim_free_data_blocks_from(sbi, start, reserve_blocks) {
            Some(Ok(run)) => Some(run),
            Some(Err(err)) => return Err(err),
            None => None,
        }
    };
    Ok(run.map(|run| append_reservation_from_run(reservation, run, map_blocks)))
}

fn ext4_release_reserved_data_blocks(
    sbi: &super::Ext4Sbi,
    reservation: &spin::Mutex<Option<super::Ext4BlockReservation>>,
) -> Result<(), i32> {
    let reserved = reservation.lock().take();
    let Some(reserved) = reserved else {
        return Ok(());
    };
    ext4_release_data_blocks(sbi, reserved.start, reserved.count)
}

fn write_block(
    bdev: &crate::block::block_device::BlockDeviceRef,
    lba: u64,
    bytes: &[u8],
) -> Result<(), i32> {
    if bytes.is_empty() || bytes.len() % 512 != 0 {
        return Err(EINVAL);
    }
    const MAX_BIO_BYTES: usize = 512;
    if bytes.len() > MAX_BIO_BYTES {
        let mut sector = lba;
        for chunk in bytes.chunks(MAX_BIO_BYTES) {
            if chunk.len() % 512 != 0 {
                return Err(EINVAL);
            }
            let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_WRITE), sector);
            bio.add_vec(BioVec::new(chunk.to_vec()));
            submit_bio(bio)?;
            sector = sector
                .checked_add((chunk.len() / 512) as u64)
                .ok_or(EINVAL)?;
        }
        return Ok(());
    }
    let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_WRITE), lba);
    bio.add_vec(BioVec::new(bytes.to_vec()));
    submit_bio(bio)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Ext4AppendAllocation {
    data_block: u64,
    data_blocks: u16,
    allocated_blocks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Ext4BlockRun {
    start: u64,
    count: u16,
}

fn ext4_alloc_extent_append_block(
    sbi: &super::Ext4Sbi,
    raw: &mut OnDiskInode,
    i_block: &mut [u32; 15],
    lblock: u64,
) -> Result<Ext4AppendAllocation, i32> {
    ext4_alloc_extent_append_blocks(sbi, raw, i_block, lblock, 1, 1, None)
}

fn ext4_alloc_extent_append_blocks(
    sbi: &super::Ext4Sbi,
    raw: &mut OnDiskInode,
    i_block: &mut [u32; 15],
    lblock: u64,
    map_data_blocks: u16,
    reserve_data_blocks: u16,
    reservation: Option<&spin::Mutex<Option<super::Ext4BlockReservation>>>,
) -> Result<Ext4AppendAllocation, i32> {
    let (map_data_blocks, reserve_data_blocks) =
        normalized_append_counts(map_data_blocks, reserve_data_blocks);
    let bytes = i_block_as_bytes_mut(i_block);
    if le_u16(bytes, 0)? != extents::EXT4_EXT_MAGIC {
        return Err(EINVAL);
    }
    match le_u16(bytes, 6)? {
        0 => {
            if le_u16(bytes, 2)? == 0 {
                let allocation = ext4_claim_append_data_blocks(
                    sbi,
                    reservation,
                    lblock,
                    map_data_blocks,
                    reserve_data_blocks,
                    false,
                )?
                .ok_or(EIO)?;
                put_extent_entry(bytes, 0, lblock, allocation.count, allocation.start)?;
                put_le_u16(bytes, 2, 1)?;
                raw.i_block = *i_block;
                return Ok(Ext4AppendAllocation {
                    data_block: allocation.start,
                    data_blocks: allocation.count,
                    allocated_blocks: allocation.count as u64,
                });
            }
            let allocation = match ext4_try_append_leaf_extent(
                sbi,
                bytes,
                lblock,
                map_data_blocks,
                reserve_data_blocks,
                reservation,
            )? {
                Some(allocation) => allocation,
                None => ext4_grow_leaf_root_for_append(sbi, bytes, lblock)?,
            };
            raw.i_block = *i_block;
            Ok(allocation)
        }
        1 | 2 => {
            let allocation = ext4_alloc_indexed_extent_append_block(
                sbi,
                bytes,
                lblock,
                map_data_blocks,
                reserve_data_blocks,
                reservation,
            )?;
            raw.i_block = *i_block;
            Ok(allocation)
        }
        _ => Err(EINVAL),
    }
}

fn ext4_try_append_leaf_extent(
    sbi: &super::Ext4Sbi,
    bytes: &mut [u8],
    lblock: u64,
    map_data_blocks: u16,
    reserve_data_blocks: u16,
    reservation: Option<&spin::Mutex<Option<super::Ext4BlockReservation>>>,
) -> Result<Option<Ext4AppendAllocation>, i32> {
    if le_u16(bytes, 0)? != extents::EXT4_EXT_MAGIC || le_u16(bytes, 6)? != 0 {
        return Err(EINVAL);
    }
    let entries = le_u16(bytes, 2)? as usize;
    let max_entries = le_u16(bytes, 4)? as usize;
    if entries == 0 || entries > max_entries || 12 + entries * 12 > bytes.len() {
        return Err(EINVAL);
    }

    let last_off = 12 + (entries - 1) * 12;
    let last_lblock = le_u32(bytes, last_off)? as u64;
    let last_len = (le_u16(bytes, last_off + 4)? & 0x7fff) as u64;
    let last_phys =
        ((le_u16(bytes, last_off + 6)? as u64) << 32) | le_u32(bytes, last_off + 8)? as u64;
    if last_len == 0 || lblock != last_lblock + last_len {
        return Err(EINVAL);
    }

    let contiguous_phys = last_phys.checked_add(last_len).ok_or(EINVAL)?;
    let extension_cap = (0x7fff - last_len).min(map_data_blocks as u64) as u16;
    if extension_cap != 0 {
        let reserve_cap = (0x7fff - last_len).min(reserve_data_blocks as u64) as u16;
        if let Some(allocation) = ext4_claim_append_data_blocks(
            sbi,
            reservation,
            contiguous_phys,
            extension_cap,
            reserve_cap,
            true,
        )? {
            put_le_u16(bytes, last_off + 4, (last_len as u16) + allocation.count)?;
            return Ok(Some(Ext4AppendAllocation {
                data_block: allocation.start,
                data_blocks: allocation.count,
                allocated_blocks: allocation.count as u64,
            }));
        }
    }

    if entries >= max_entries || 12 + (entries + 1) * 12 > bytes.len() {
        return Ok(None);
    }

    let allocation = ext4_claim_append_data_blocks(
        sbi,
        reservation,
        contiguous_phys,
        map_data_blocks,
        reserve_data_blocks,
        false,
    )?
    .ok_or(EIO)?;
    put_extent_entry(bytes, entries, lblock, allocation.count, allocation.start)?;
    put_le_u16(bytes, 2, (entries + 1) as u16)?;
    Ok(Some(Ext4AppendAllocation {
        data_block: allocation.start,
        data_blocks: allocation.count,
        allocated_blocks: allocation.count as u64,
    }))
}

fn ext4_alloc_indexed_extent_append_block(
    sbi: &super::Ext4Sbi,
    root_bytes: &mut [u8],
    lblock: u64,
    map_data_blocks: u16,
    reserve_data_blocks: u16,
    reservation: Option<&spin::Mutex<Option<super::Ext4BlockReservation>>>,
) -> Result<Ext4AppendAllocation, i32> {
    let depth = le_u16(root_bytes, 6)?;
    if depth == 1 {
        match ext4_append_into_index_node(
            sbi,
            root_bytes,
            1,
            lblock,
            map_data_blocks,
            reserve_data_blocks,
            reservation,
        )? {
            Some(allocation) => return Ok(allocation),
            None => return ext4_grow_index_root_for_append(sbi, root_bytes, lblock),
        }
    }
    if depth == 2 {
        return ext4_append_into_index_node(
            sbi,
            root_bytes,
            2,
            lblock,
            map_data_blocks,
            reserve_data_blocks,
            reservation,
        )?
        .ok_or(-ENOSPC);
    }
    Err(EINVAL)
}

fn ext4_append_into_index_node(
    sbi: &super::Ext4Sbi,
    node_bytes: &mut [u8],
    depth: u16,
    lblock: u64,
    map_data_blocks: u16,
    reserve_data_blocks: u16,
    reservation: Option<&spin::Mutex<Option<super::Ext4BlockReservation>>>,
) -> Result<Option<Ext4AppendAllocation>, i32> {
    let entries = le_u16(node_bytes, 2)? as usize;
    let max_entries = le_u16(node_bytes, 4)? as usize;
    if entries == 0 || entries > max_entries || 12 + entries * 12 > node_bytes.len() {
        return Err(EINVAL);
    }

    let idx_off = 12 + (entries - 1) * 12;
    let idx_lblock = le_u32(node_bytes, idx_off)? as u64;
    if lblock < idx_lblock {
        return Err(EINVAL);
    }
    let leaf_block =
        ((le_u16(node_bytes, idx_off + 8)? as u64) << 32) | le_u32(node_bytes, idx_off + 4)? as u64;
    if leaf_block == 0 || leaf_block >= sbi.blocks_count {
        return Err(EINVAL);
    }

    let child_lba = leaf_block * sbi.block_size as u64 / 512;
    let mut child = read_sectors(&sbi.bdev, child_lba, sbi.block_size as u64 / 512)?;
    let allocation = if depth == 1 {
        match ext4_try_append_leaf_extent(
            sbi,
            &mut child,
            lblock,
            map_data_blocks,
            reserve_data_blocks,
            reservation,
        )? {
            Some(allocation) => Some(allocation),
            None => {
                if entries >= max_entries || 12 + (entries + 1) * 12 > node_bytes.len() {
                    None
                } else {
                    let allocation =
                        ext4_create_new_leaf_for_index_node(sbi, node_bytes, entries, lblock)?;
                    Some(allocation)
                }
            }
        }
    } else {
        ext4_append_into_index_node(
            sbi,
            &mut child,
            depth - 1,
            lblock,
            map_data_blocks,
            reserve_data_blocks,
            reservation,
        )?
    };
    if allocation.is_some() {
        write_block(&sbi.bdev, child_lba, &child)?;
    }
    Ok(allocation)
}

fn ext4_grow_leaf_root_for_append(
    sbi: &super::Ext4Sbi,
    root_bytes: &mut [u8],
    lblock: u64,
) -> Result<Ext4AppendAllocation, i32> {
    let old_first_lblock = first_lblock_in_extent_node(root_bytes)?;
    let search = leaf_append_search_start(root_bytes)?;
    let old_leaf_block = ext4_claim_free_data_block_from(sbi, search).ok_or(EIO)??;
    let data_block = ext4_claim_free_data_block_from(sbi, old_leaf_block + 1).ok_or(EIO)??;
    let new_leaf_block = ext4_claim_free_data_block_from(sbi, data_block + 1).ok_or(EIO)??;

    let mut old_leaf = alloc::vec![0u8; sbi.block_size as usize];
    old_leaf[..root_bytes.len()].copy_from_slice(root_bytes);
    let old_leaf_max = extent_node_max_entries(old_leaf.len()) as u16;
    put_le_u16(&mut old_leaf, 4, old_leaf_max)?;
    write_extent_node_block(sbi, old_leaf_block, &old_leaf)?;

    let new_leaf = new_extent_leaf_block(sbi.block_size as usize, lblock, data_block)?;
    write_extent_node_block(sbi, new_leaf_block, &new_leaf)?;

    init_extent_header(
        root_bytes,
        2,
        extent_node_max_entries(root_bytes.len()) as u16,
        1,
    )?;
    put_idx_entry(root_bytes, 0, old_first_lblock, old_leaf_block)?;
    put_idx_entry(root_bytes, 1, lblock, new_leaf_block)?;

    Ok(Ext4AppendAllocation {
        data_block,
        data_blocks: 1,
        allocated_blocks: 3,
    })
}

fn ext4_create_new_leaf_for_index_node(
    sbi: &super::Ext4Sbi,
    node_bytes: &mut [u8],
    insert_at: usize,
    lblock: u64,
) -> Result<Ext4AppendAllocation, i32> {
    let data_block = ext4_claim_free_data_block_from(sbi, lblock).ok_or(EIO)??;
    let new_leaf_block = ext4_claim_free_data_block_from(sbi, data_block + 1).ok_or(EIO)??;
    let new_leaf = new_extent_leaf_block(sbi.block_size as usize, lblock, data_block)?;
    write_extent_node_block(sbi, new_leaf_block, &new_leaf)?;
    put_idx_entry(node_bytes, insert_at, lblock, new_leaf_block)?;
    put_le_u16(node_bytes, 2, (insert_at + 1) as u16)?;
    Ok(Ext4AppendAllocation {
        data_block,
        data_blocks: 1,
        allocated_blocks: 2,
    })
}

fn ext4_grow_index_root_for_append(
    sbi: &super::Ext4Sbi,
    root_bytes: &mut [u8],
    lblock: u64,
) -> Result<Ext4AppendAllocation, i32> {
    let old_first_lblock = first_lblock_in_extent_node(root_bytes)?;
    let old_index_block = ext4_claim_free_data_block_from(sbi, lblock).ok_or(EIO)??;
    let data_block = ext4_claim_free_data_block_from(sbi, old_index_block + 1).ok_or(EIO)??;
    let new_leaf_block = ext4_claim_free_data_block_from(sbi, data_block + 1).ok_or(EIO)??;
    let new_index_block = ext4_claim_free_data_block_from(sbi, new_leaf_block + 1).ok_or(EIO)??;

    let mut old_index = alloc::vec![0u8; sbi.block_size as usize];
    old_index[..root_bytes.len()].copy_from_slice(root_bytes);
    let old_index_max = extent_node_max_entries(old_index.len()) as u16;
    put_le_u16(&mut old_index, 4, old_index_max)?;
    write_extent_node_block(sbi, old_index_block, &old_index)?;

    let new_leaf = new_extent_leaf_block(sbi.block_size as usize, lblock, data_block)?;
    write_extent_node_block(sbi, new_leaf_block, &new_leaf)?;

    let mut new_index = alloc::vec![0u8; sbi.block_size as usize];
    init_extent_header(
        &mut new_index,
        1,
        extent_node_max_entries(sbi.block_size as usize) as u16,
        1,
    )?;
    put_idx_entry(&mut new_index, 0, lblock, new_leaf_block)?;
    write_extent_node_block(sbi, new_index_block, &new_index)?;

    init_extent_header(
        root_bytes,
        2,
        extent_node_max_entries(root_bytes.len()) as u16,
        2,
    )?;
    put_idx_entry(root_bytes, 0, old_first_lblock, old_index_block)?;
    put_idx_entry(root_bytes, 1, lblock, new_index_block)?;

    Ok(Ext4AppendAllocation {
        data_block,
        data_blocks: 1,
        allocated_blocks: 4,
    })
}

fn first_lblock_in_extent_node(bytes: &[u8]) -> Result<u64, i32> {
    if le_u16(bytes, 0)? != extents::EXT4_EXT_MAGIC || le_u16(bytes, 2)? == 0 {
        return Err(EINVAL);
    }
    Ok(le_u32(bytes, 12)? as u64)
}

fn leaf_append_search_start(bytes: &[u8]) -> Result<u64, i32> {
    let entries = le_u16(bytes, 2)? as usize;
    if entries == 0 || 12 + entries * 12 > bytes.len() {
        return Err(EINVAL);
    }
    let last_off = 12 + (entries - 1) * 12;
    let last_len = (le_u16(bytes, last_off + 4)? & 0x7fff) as u64;
    let last_phys =
        ((le_u16(bytes, last_off + 6)? as u64) << 32) | le_u32(bytes, last_off + 8)? as u64;
    if last_len == 0 {
        return Err(EINVAL);
    }
    last_phys.checked_add(last_len).ok_or(EINVAL)
}

fn extent_node_max_entries(len: usize) -> usize {
    len.saturating_sub(12) / 12
}

fn new_extent_leaf_block(block_size: usize, lblock: u64, data_block: u64) -> Result<Vec<u8>, i32> {
    let mut leaf = alloc::vec![0u8; block_size];
    init_extent_header(&mut leaf, 1, extent_node_max_entries(block_size) as u16, 0)?;
    put_extent_entry(&mut leaf, 0, lblock, 1, data_block)?;
    Ok(leaf)
}

fn init_extent_header(
    bytes: &mut [u8],
    entries: u16,
    max_entries: u16,
    depth: u16,
) -> Result<(), i32> {
    if bytes.len() < 12 {
        return Err(EINVAL);
    }
    bytes.fill(0);
    put_le_u16(bytes, 0, extents::EXT4_EXT_MAGIC)?;
    put_le_u16(bytes, 2, entries)?;
    put_le_u16(bytes, 4, max_entries)?;
    put_le_u16(bytes, 6, depth)
}

fn put_extent_entry(
    bytes: &mut [u8],
    index: usize,
    lblock: u64,
    len: u16,
    phys: u64,
) -> Result<(), i32> {
    let off = 12 + index * 12;
    put_le_u32(bytes, off, lblock as u32)?;
    put_le_u16(bytes, off + 4, len)?;
    put_le_u16(bytes, off + 6, (phys >> 32) as u16)?;
    put_le_u32(bytes, off + 8, phys as u32)
}

fn put_idx_entry(bytes: &mut [u8], index: usize, lblock: u64, block: u64) -> Result<(), i32> {
    let off = 12 + index * 12;
    put_le_u32(bytes, off, lblock as u32)?;
    put_le_u32(bytes, off + 4, block as u32)?;
    put_le_u16(bytes, off + 8, (block >> 32) as u16)?;
    put_le_u16(bytes, off + 10, 0)
}

fn write_extent_node_block(sbi: &super::Ext4Sbi, block: u64, bytes: &[u8]) -> Result<(), i32> {
    crate::fs::jbd2::transaction::jbd2_journal_write_metadata_block(
        &sbi.bdev,
        block,
        sbi.block_size as u64,
        bytes,
    )
}

fn ext4_try_claim_block(sbi: &super::Ext4Sbi, block: u64) -> Result<bool, i32> {
    Ok(ext4_try_claim_contiguous_data_blocks(sbi, block, 1)?.is_some())
}

fn ext4_claim_free_data_block_from(sbi: &super::Ext4Sbi, start: u64) -> Option<Result<u64, i32>> {
    ext4_claim_free_data_blocks_from(sbi, start, 1).map(|result| result.map(|run| run.start))
}

fn ext4_claim_free_data_blocks_from(
    sbi: &super::Ext4Sbi,
    start: u64,
    max_blocks: u16,
) -> Option<Result<Ext4BlockRun, i32>> {
    let first = first_allocatable_block(sbi);
    let start = start.max(first).min(sbi.blocks_count);
    ext4_claim_free_data_blocks_in_range(sbi, start, sbi.blocks_count, max_blocks)
        .or_else(|| ext4_claim_free_data_blocks_in_range(sbi, first, start, max_blocks))
}

fn ext4_claim_free_data_blocks_in_range(
    sbi: &super::Ext4Sbi,
    start: u64,
    end: u64,
    max_blocks: u16,
) -> Option<Result<Ext4BlockRun, i32>> {
    let end = end.min(sbi.blocks_count);
    if start >= end || max_blocks == 0 {
        return None;
    }

    // Linux's ext4 allocator loads the block-group bitmap once and scans it
    // with ext4_find_next_zero_bit()/mb_find_next_zero_bit; doing one disk
    // read per candidate block makes full distro roots crawl under TCG.
    let blocks_per_group = sbi.blocks_per_group as u64;
    let bitmap_bits = (sbi.block_size as usize).saturating_mul(8);
    let Some((first_group, _)) = ext4_block_group_and_bit(sbi, start) else {
        return None;
    };
    let Some((last_group, _)) = ext4_block_group_and_bit(sbi, end - 1) else {
        return None;
    };
    let first_group = first_group as u64;
    let last_group = last_group as u64;
    let mut group = first_group;
    while group <= last_group {
        let group_start = ext4_group_first_block(sbi, group);
        let group_start = match group_start {
            Ok(group_start) => group_start,
            Err(err) => return Some(Err(err)),
        };
        let bit_start = start.saturating_sub(group_start).min(blocks_per_group) as usize;
        let bit_end = end.saturating_sub(group_start).min(blocks_per_group) as usize;
        let bit_end = bit_end.min(bitmap_bits);
        if bit_start >= bit_end {
            group += 1;
            continue;
        }

        match claim_data_block_run_in_group_bitmap(
            sbi,
            group as usize,
            bit_start,
            bit_end,
            max_blocks,
            false,
        ) {
            Ok(Some(run)) => return Some(Ok(run)),
            Ok(None) => {}
            Err(err) => return Some(Err(err)),
        }
        group += 1;
    }
    None
}

fn ext4_try_claim_contiguous_data_blocks(
    sbi: &super::Ext4Sbi,
    start: u64,
    max_blocks: u16,
) -> Result<Option<Ext4BlockRun>, i32> {
    if start >= sbi.blocks_count || max_blocks == 0 || !ext4_allocatable_data_block(sbi, start) {
        return Ok(None);
    }
    let blocks_per_group = sbi.blocks_per_group as u64;
    let Some((group, bit_start)) = ext4_block_group_and_bit(sbi, start) else {
        return Ok(None);
    };
    let bit_end = bit_start
        .saturating_add(max_blocks as usize)
        .min(blocks_per_group as usize)
        .min((sbi.block_size as usize).saturating_mul(8));
    claim_data_block_run_in_group_bitmap(sbi, group, bit_start, bit_end, max_blocks, true)
}

fn first_allocatable_block(sbi: &super::Ext4Sbi) -> u64 {
    first_data_block(sbi)
}

fn first_data_block(sbi: &super::Ext4Sbi) -> u64 {
    if sbi.block_size == 1024 { 1 } else { 0 }
}

fn ext4_group_first_block(sbi: &super::Ext4Sbi, group: u64) -> Result<u64, i32> {
    group
        .checked_mul(sbi.blocks_per_group as u64)
        .and_then(|base| base.checked_add(first_data_block(sbi)))
        .ok_or(EINVAL)
}

fn ext4_block_group_and_bit(sbi: &super::Ext4Sbi, block: u64) -> Option<(usize, usize)> {
    let first = first_data_block(sbi);
    if block < first || block >= sbi.blocks_count {
        return None;
    }
    let rel = block - first;
    let group = rel / sbi.blocks_per_group as u64;
    let bit = (rel % sbi.blocks_per_group as u64) as usize;
    Some((group as usize, bit))
}

fn ext4_allocatable_data_block(sbi: &super::Ext4Sbi, block: u64) -> bool {
    if block >= sbi.blocks_count || block < first_allocatable_block(sbi) {
        return false;
    }

    let Some((group, _)) = ext4_block_group_and_bit(sbi, block) else {
        return false;
    };
    let Some(gd) = sbi.group_descs.get(group) else {
        return false;
    };
    let Ok(group_start) = ext4_group_first_block(sbi, group as u64) else {
        return false;
    };
    let next_group_start = group_start.saturating_add(sbi.blocks_per_group as u64);
    if next_group_start < sbi.blocks_count {
        let tail_guard_start = next_group_start
            .saturating_sub(EXT4_ALLOC_GROUP_TAIL_GUARD_BLOCKS)
            .max(group_start);
        if block >= tail_guard_start {
            return false;
        }
    }

    if group == 0 && primary_metadata_block(sbi, block) {
        return false;
    }
    if block == gd.bg_block_bitmap || block == gd.bg_inode_bitmap {
        return false;
    }

    let inode_table_blocks = ext4_inode_table_blocks(sbi);
    block < gd.bg_inode_table || block >= gd.bg_inode_table.saturating_add(inode_table_blocks)
}

fn primary_metadata_block(sbi: &super::Ext4Sbi, block: u64) -> bool {
    if sbi.block_size == 1024 {
        block <= 2
    } else {
        block <= 1
    }
}

fn ext4_inode_table_blocks(sbi: &super::Ext4Sbi) -> u64 {
    let bytes = sbi.inodes_per_group as u64 * sbi.inode_size as u64;
    bytes.div_ceil(sbi.block_size as u64)
}

fn claim_data_block_run_in_group_bitmap(
    sbi: &super::Ext4Sbi,
    group: usize,
    bit_start: usize,
    bit_end: usize,
    max_blocks: u16,
    require_start: bool,
) -> Result<Option<Ext4BlockRun>, i32> {
    let gd = sbi.group_descs.get(group).ok_or(EINVAL)?;
    let bitmap_lba = gd.bg_block_bitmap * sbi.block_size as u64 / 512;
    let mut bitmap = read_sectors(&sbi.bdev, bitmap_lba, sbi.block_size as u64 / 512)?;
    let bit = match find_next_free_allocatable_bit(sbi, &bitmap, group, bit_start, bit_end)? {
        Some(bit) => bit,
        None => return Ok(None),
    };
    if require_start && bit != bit_start {
        return Ok(None);
    }

    let group_start = ext4_group_first_block(sbi, group as u64)?;
    let mut count = 0u16;
    while count < max_blocks {
        let candidate_bit = bit + count as usize;
        if candidate_bit >= bit_end {
            break;
        }
        let block = group_start
            .checked_add(candidate_bit as u64)
            .ok_or(EINVAL)?;
        if !ext4_allocatable_data_block(sbi, block)
            || metadata::bitmap_test(&bitmap, candidate_bit)?
        {
            break;
        }
        count += 1;
    }
    if count == 0 {
        return Ok(None);
    }
    let run_start = group_start + bit as u64;

    for bit in bit..bit + count as usize {
        metadata::bitmap_set(&mut bitmap, bit)?;
    }
    crate::fs::jbd2::transaction::jbd2_journal_write_metadata_block(
        &sbi.bdev,
        gd.bg_block_bitmap,
        sbi.block_size as u64,
        &bitmap,
    )?;
    decrement_group_free_blocks_by(sbi, group, count as u32)?;
    decrement_super_free_blocks_by(sbi, count as u32)?;
    Ok(Some(Ext4BlockRun {
        start: run_start,
        count,
    }))
}

fn find_next_free_allocatable_bit(
    sbi: &super::Ext4Sbi,
    bitmap: &[u8],
    group: usize,
    bit_start: usize,
    bit_end: usize,
) -> Result<Option<usize>, i32> {
    let group_start = ext4_group_first_block(sbi, group as u64)?;
    let mut bit = bit_start;
    while bit < bit_end {
        if bit % 8 == 0 && bit + 8 <= bit_end {
            let byte = *bitmap.get(bit / 8).ok_or(EINVAL)?;
            if byte == 0xff {
                bit += 8;
                continue;
            }
        }
        if !metadata::bitmap_test(bitmap, bit)? {
            let block = group_start.checked_add(bit as u64).ok_or(EINVAL)?;
            if ext4_allocatable_data_block(sbi, block) {
                return Ok(Some(bit));
            }
        }
        bit += 1;
    }
    Ok(None)
}

fn ext4_release_inode(sbi: &super::Ext4Sbi, ino: u32, directory: bool) -> Result<(), i32> {
    if ino == 0 || ino as u64 > sbi.inodes_count {
        return Err(EINVAL);
    }
    let group = (ino - 1) / sbi.inodes_per_group;
    let bit = ((ino - 1) % sbi.inodes_per_group) as usize;
    let gd = sbi.group_descs.get(group as usize).ok_or(EINVAL)?;
    let bitmap_lba = gd.bg_inode_bitmap * sbi.block_size as u64 / 512;
    let mut bitmap = read_sectors(&sbi.bdev, bitmap_lba, sbi.block_size as u64 / 512)?;
    if !metadata::bitmap_test(&bitmap, bit)? {
        return Err(EINVAL);
    }
    metadata::bitmap_clear(&mut bitmap, bit)?;
    crate::fs::jbd2::transaction::jbd2_journal_write_metadata_block(
        &sbi.bdev,
        gd.bg_inode_bitmap,
        sbi.block_size as u64,
        &bitmap,
    )?;
    increment_group_free_inodes(sbi, group as usize)?;
    if directory {
        decrement_group_used_dirs(sbi, group as usize)?;
    }
    increment_super_free_inodes(sbi)
}

fn ext4_release_data_block(sbi: &super::Ext4Sbi, block: u64) -> Result<(), i32> {
    ext4_release_data_blocks(sbi, block, 1)
}

fn ext4_release_data_blocks(sbi: &super::Ext4Sbi, start: u64, count: u16) -> Result<(), i32> {
    if count == 0 {
        return Ok(());
    }
    let mut block = start;
    let end = start.checked_add(count as u64).ok_or(EINVAL)?;
    while block < end {
        if !ext4_allocatable_data_block(sbi, block) {
            return Err(EINVAL);
        }
        let Some((group, first_bit)) = ext4_block_group_and_bit(sbi, block) else {
            return Err(EINVAL);
        };
        let group_end = ext4_group_first_block(sbi, group as u64 + 1)?
            .min(end)
            .min(sbi.blocks_count);
        let group_count = group_end.checked_sub(block).ok_or(EINVAL)? as u32;
        let gd = sbi.group_descs.get(group).ok_or(EINVAL)?;
        let bitmap_lba = gd.bg_block_bitmap * sbi.block_size as u64 / 512;
        let mut bitmap = read_sectors(&sbi.bdev, bitmap_lba, sbi.block_size as u64 / 512)?;
        for bit in first_bit..first_bit + group_count as usize {
            if !metadata::bitmap_test(&bitmap, bit)? {
                return Err(EINVAL);
            }
            metadata::bitmap_clear(&mut bitmap, bit)?;
        }
        crate::fs::jbd2::transaction::jbd2_journal_write_metadata_block(
            &sbi.bdev,
            gd.bg_block_bitmap,
            sbi.block_size as u64,
            &bitmap,
        )?;
        increment_group_free_blocks_by(sbi, group, group_count)?;
        increment_super_free_blocks_by(sbi, group_count)?;
        block = group_end;
    }
    Ok(())
}

fn decrement_group_free_blocks(sbi: &super::Ext4Sbi, group: usize) -> Result<(), i32> {
    decrement_group_free_blocks_by(sbi, group, 1)
}

fn decrement_group_free_blocks_by(
    sbi: &super::Ext4Sbi,
    group: usize,
    count: u32,
) -> Result<(), i32> {
    if count == 0 {
        return Ok(());
    }
    let gdt_start_block = if sbi.block_size == 1024 { 2 } else { 1 };
    let desc_off =
        gdt_start_block as u64 * sbi.block_size as u64 + group as u64 * sbi.group_desc_size as u64;
    let len = sbi.group_desc_size.max(32) as usize;
    let mut desc = read_disk_bytes(sbi, desc_off, len)?;
    let lo = le_u16(&desc, 12)?;
    let total = if desc.len() >= 46 {
        lo as u32 | ((le_u16(&desc, 44)? as u32) << 16)
    } else {
        lo as u32
    };
    if total < count {
        return Err(EIO);
    }
    let remaining = total - count;
    put_le_u16(&mut desc, 12, remaining as u16)?;
    if desc.len() >= 46 {
        put_le_u16(&mut desc, 44, (remaining >> 16) as u16)?;
    }
    write_disk_bytes(sbi, desc_off, &desc)
}

fn increment_group_free_blocks(sbi: &super::Ext4Sbi, group: usize) -> Result<(), i32> {
    increment_group_free_blocks_by(sbi, group, 1)
}

fn increment_group_free_blocks_by(
    sbi: &super::Ext4Sbi,
    group: usize,
    count: u32,
) -> Result<(), i32> {
    if count == 0 {
        return Ok(());
    }
    let gdt_start_block = if sbi.block_size == 1024 { 2 } else { 1 };
    let desc_off =
        gdt_start_block as u64 * sbi.block_size as u64 + group as u64 * sbi.group_desc_size as u64;
    let len = sbi.group_desc_size.max(32) as usize;
    let mut desc = read_disk_bytes(sbi, desc_off, len)?;
    let lo = le_u16(&desc, 12)?;
    let total = if desc.len() >= 46 {
        lo as u32 | ((le_u16(&desc, 44)? as u32) << 16)
    } else {
        lo as u32
    };
    let total = total.checked_add(count).ok_or(EIO)?;
    put_le_u16(&mut desc, 12, total as u16)?;
    if desc.len() >= 46 {
        put_le_u16(&mut desc, 44, (total >> 16) as u16)?;
    } else if total > u16::MAX as u32 {
        return Err(EIO);
    }
    write_disk_bytes(sbi, desc_off, &desc)
}

fn decrement_group_free_inodes(sbi: &super::Ext4Sbi, group: usize) -> Result<(), i32> {
    let gdt_start_block = if sbi.block_size == 1024 { 2 } else { 1 };
    let desc_off =
        gdt_start_block as u64 * sbi.block_size as u64 + group as u64 * sbi.group_desc_size as u64;
    let len = sbi.group_desc_size.max(32) as usize;
    let mut desc = read_disk_bytes(sbi, desc_off, len)?;
    let lo = le_u16(&desc, 14)?;
    if lo > 0 {
        put_le_u16(&mut desc, 14, lo - 1)?;
    } else if desc.len() >= 48 {
        let hi = le_u16(&desc, 46)?;
        if hi == 0 {
            return Err(EIO);
        }
        put_le_u16(&mut desc, 14, u16::MAX)?;
        put_le_u16(&mut desc, 46, hi - 1)?;
    } else {
        return Err(EIO);
    }
    write_disk_bytes(sbi, desc_off, &desc)
}

fn increment_group_free_inodes(sbi: &super::Ext4Sbi, group: usize) -> Result<(), i32> {
    let gdt_start_block = if sbi.block_size == 1024 { 2 } else { 1 };
    let desc_off =
        gdt_start_block as u64 * sbi.block_size as u64 + group as u64 * sbi.group_desc_size as u64;
    let len = sbi.group_desc_size.max(32) as usize;
    let mut desc = read_disk_bytes(sbi, desc_off, len)?;
    let lo = le_u16(&desc, 14)?;
    if lo < u16::MAX {
        put_le_u16(&mut desc, 14, lo + 1)?;
    } else if desc.len() >= 48 {
        let hi = le_u16(&desc, 46)?;
        put_le_u16(&mut desc, 14, 0)?;
        put_le_u16(&mut desc, 46, hi.checked_add(1).ok_or(EIO)?)?;
    } else {
        return Err(EIO);
    }
    write_disk_bytes(sbi, desc_off, &desc)
}

fn increment_group_used_dirs(sbi: &super::Ext4Sbi, group: usize) -> Result<(), i32> {
    let gdt_start_block = if sbi.block_size == 1024 { 2 } else { 1 };
    let desc_off =
        gdt_start_block as u64 * sbi.block_size as u64 + group as u64 * sbi.group_desc_size as u64;
    let len = sbi.group_desc_size.max(32) as usize;
    let mut desc = read_disk_bytes(sbi, desc_off, len)?;
    let lo = le_u16(&desc, 16)?;
    if lo < u16::MAX {
        put_le_u16(&mut desc, 16, lo + 1)?;
    } else if desc.len() >= 50 {
        let hi = le_u16(&desc, 48)?;
        put_le_u16(&mut desc, 16, 0)?;
        put_le_u16(&mut desc, 48, hi.checked_add(1).ok_or(EIO)?)?;
    } else {
        return Err(EIO);
    }
    write_disk_bytes(sbi, desc_off, &desc)
}

fn decrement_group_used_dirs(sbi: &super::Ext4Sbi, group: usize) -> Result<(), i32> {
    let gdt_start_block = if sbi.block_size == 1024 { 2 } else { 1 };
    let desc_off =
        gdt_start_block as u64 * sbi.block_size as u64 + group as u64 * sbi.group_desc_size as u64;
    let len = sbi.group_desc_size.max(32) as usize;
    let mut desc = read_disk_bytes(sbi, desc_off, len)?;
    let lo = le_u16(&desc, 16)?;
    if lo > 0 {
        put_le_u16(&mut desc, 16, lo - 1)?;
    } else if desc.len() >= 50 {
        let hi = le_u16(&desc, 48)?;
        if hi == 0 {
            return Err(EIO);
        }
        put_le_u16(&mut desc, 16, u16::MAX)?;
        put_le_u16(&mut desc, 48, hi - 1)?;
    } else {
        return Err(EIO);
    }
    write_disk_bytes(sbi, desc_off, &desc)
}

fn decrement_super_free_blocks(sbi: &super::Ext4Sbi) -> Result<(), i32> {
    decrement_super_free_blocks_by(sbi, 1)
}

fn decrement_super_free_blocks_by(sbi: &super::Ext4Sbi, blocks: u32) -> Result<(), i32> {
    if blocks == 0 {
        return Ok(());
    }
    const SB_OFF_BYTES: u64 = 1024;
    let mut bytes = read_disk_bytes(sbi, SB_OFF_BYTES + 12, 4)?;
    let free = le_u32(&bytes, 0)?;
    if free < blocks {
        return Err(EIO);
    }
    put_le_u32(&mut bytes, 0, free - blocks)?;
    write_disk_bytes(sbi, SB_OFF_BYTES + 12, &bytes)
}

fn increment_super_free_blocks(sbi: &super::Ext4Sbi) -> Result<(), i32> {
    increment_super_free_blocks_by(sbi, 1)
}

fn increment_super_free_blocks_by(sbi: &super::Ext4Sbi, blocks: u32) -> Result<(), i32> {
    if blocks == 0 {
        return Ok(());
    }
    const SB_OFF_BYTES: u64 = 1024;
    let mut count = read_disk_bytes(sbi, SB_OFF_BYTES + 12, 4)?;
    let free = le_u32(&count, 0)?;
    put_le_u32(&mut count, 0, free.checked_add(blocks).ok_or(EIO)?)?;
    write_disk_bytes(sbi, SB_OFF_BYTES + 12, &count)
}

fn decrement_super_free_inodes(sbi: &super::Ext4Sbi) -> Result<(), i32> {
    const SB_OFF_BYTES: u64 = 1024;
    let mut count = read_disk_bytes(sbi, SB_OFF_BYTES + 16, 4)?;
    let free = le_u32(&count, 0)?;
    if free == 0 {
        return Err(EIO);
    }
    put_le_u32(&mut count, 0, free - 1)?;
    write_disk_bytes(sbi, SB_OFF_BYTES + 16, &count)
}

fn increment_super_free_inodes(sbi: &super::Ext4Sbi) -> Result<(), i32> {
    const SB_OFF_BYTES: u64 = 1024;
    let mut count = read_disk_bytes(sbi, SB_OFF_BYTES + 16, 4)?;
    let free = le_u32(&count, 0)?;
    put_le_u32(&mut count, 0, free.checked_add(1).ok_or(EIO)?)?;
    write_disk_bytes(sbi, SB_OFF_BYTES + 16, &count)
}

fn write_inode_metadata(sbi: &super::Ext4Sbi, ino: u32, raw: &OnDiskInode) -> Result<(), i32> {
    if sbi.group_descs.is_empty() {
        return Ok(());
    }
    let off = ialloc::inode_disk_offset(sbi, ino)?;
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (raw as *const OnDiskInode).cast::<u8>(),
            core::mem::size_of::<OnDiskInode>(),
        )
    };
    write_disk_bytes(sbi, off, bytes)
}

fn read_disk_bytes(sbi: &super::Ext4Sbi, byte_off: u64, len: usize) -> Result<Vec<u8>, i32> {
    if len == 0 {
        return Err(EINVAL);
    }
    let lba = byte_off / 512;
    let within = (byte_off % 512) as usize;
    let total = within.checked_add(len).ok_or(EINVAL)?;
    let sectors = total.div_ceil(512) as u64;
    let buf = read_sectors(&sbi.bdev, lba, sectors)?;
    Ok(buf[within..within + len].to_vec())
}

fn write_disk_bytes(sbi: &super::Ext4Sbi, byte_off: u64, bytes: &[u8]) -> Result<(), i32> {
    if bytes.is_empty() {
        return Err(EINVAL);
    }
    let block_size = sbi.block_size as u64;
    let block = byte_off / block_size;
    let within = (byte_off % block_size) as usize;
    let total = within.checked_add(bytes.len()).ok_or(EINVAL)?;
    if total > block_size as usize {
        return Err(EINVAL);
    }
    let mut buf = read_sectors(&sbi.bdev, block * block_size / 512, block_size / 512)?;
    buf[within..within + bytes.len()].copy_from_slice(bytes);
    crate::fs::jbd2::transaction::jbd2_journal_write_metadata_block(
        &sbi.bdev, block, block_size, &buf,
    )
}

fn i_block_as_bytes_mut(i_block: &mut [u32; 15]) -> &mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(i_block.as_mut_ptr().cast::<u8>(), 60) }
}

fn i_block_as_bytes(i_block: &[u32; 15]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(i_block.as_ptr().cast::<u8>(), 60) }
}

fn le_u16(bytes: &[u8], off: usize) -> Result<u16, i32> {
    let raw = bytes.get(off..off + 2).ok_or(EINVAL)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn le_u32(bytes: &[u8], off: usize) -> Result<u32, i32> {
    let raw = bytes.get(off..off + 4).ok_or(EINVAL)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn put_le_u16(bytes: &mut [u8], off: usize, value: u16) -> Result<(), i32> {
    let dst = bytes.get_mut(off..off + 2).ok_or(EINVAL)?;
    dst.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_le_u32(bytes: &mut [u8], off: usize, value: u32) -> Result<(), i32> {
    let dst = bytes.get_mut(off..off + 4).ok_or(EINVAL)?;
    dst.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use core::sync::atomic::AtomicU64;
    use spin::Mutex;

    use crate::block::block_device::{BlockDevice, BlockDeviceRef};
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};
    use crate::fs::dcache::d_alloc;
    use crate::fs::ext4::extents::EXT4_EXT_MAGIC;
    use crate::fs::ext4::inode::OnDiskInode;
    use crate::fs::ext4::super_block::stash_sbi;
    use crate::fs::ext4::{EXT4_SUPER_MAGIC, Ext4Inode, Ext4Sbi};
    use crate::fs::types::{File, Inode, InodePrivate, SuperBlock};

    #[test]
    fn ext4_block_bitmap_mapping_accounts_for_first_data_block() {
        let mem = MemBlockDevice::new("ext4-bitmap-map", 128 * 1024);
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let sbi = Ext4Sbi {
            bdev,
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 8192,
            inodes_per_group: 2048,
            first_ino: 11,
            inode_size: 256,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16_384,
            blocks_count: 65_536,
            group_desc_size: 32,
            group_descs: Vec::new(),
        };

        assert_eq!(first_data_block(&sbi), 1);
        assert_eq!(ext4_block_group_and_bit(&sbi, 1), Some((0, 0)));
        assert_eq!(ext4_block_group_and_bit(&sbi, 4422), Some((0, 4421)));
        assert_eq!(ext4_block_group_and_bit(&sbi, 8192), Some((0, 8191)));
        assert_eq!(ext4_block_group_and_bit(&sbi, 8193), Some((1, 0)));
    }

    #[test]
    fn ext4_fast_symlink_readlink_uses_i_block() {
        let target = b"/usr/lib/systemd/system/multi-user.target";
        let raw = raw_fast_symlink_inode(target);
        let ext_inode = Arc::new(Ext4Inode {
            ino: 15,
            i_mode: 0o120777,
            i_size: AtomicU64::new(target.len() as u64),
            i_blocks: AtomicU64::new(0),
            raw: Mutex::new(raw),
            dir_cache: Mutex::new(None),
            append_reservation: Mutex::new(None),
        });
        let inode = Inode::new(
            15,
            InodeKind::Symlink,
            0o777,
            &EXT4_SYMLINK_INODE_OPS,
            &EXT4_FILE_FILE_OPS,
            InodePrivate::Opaque(Arc::into_raw(ext_inode) as usize),
        );
        inode.size.store(target.len() as u64, Ordering::Release);

        let mut buf = [0u8; 128];
        let n = ext4_readlink(&inode, &mut buf).expect("fast symlink readlink");
        assert_eq!(&buf[..n], target);

        let mut short = [0u8; 8];
        let n = ext4_readlink(&inode, &mut short).expect("truncated fast symlink readlink");
        assert_eq!(n, short.len());
        assert_eq!(&short, &target[..short.len()]);
    }

    #[test]
    fn ext4_write_overwrites_existing_extent_block() {
        let mem = MemBlockDevice::new("ext4-write0", 8192);
        {
            let mut data = mem.data.lock();
            data[2048..3072].fill(b'A');
        }
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let sb = SuperBlock::alloc("ext4", EXT4_SUPER_MAGIC as u64, &EXT4_SUPER_OPS);
        let sbi = Arc::new(Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 64,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 128,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 64,
            group_desc_size: 32,
            group_descs: Vec::new(),
        });
        stash_sbi(&sb, sbi).unwrap();

        let raw = raw_inode_with_extent(2, 1024);
        let ext_inode = Arc::new(Ext4Inode {
            ino: 12,
            i_mode: 0o100644,
            i_size: AtomicU64::new(1024),
            i_blocks: AtomicU64::new(2),
            raw: Mutex::new(raw),
            dir_cache: Mutex::new(None),
            append_reservation: Mutex::new(None),
        });
        let inode = Inode::new(
            12,
            InodeKind::Regular,
            0o644,
            &EXT4_FILE_INODE_OPS,
            &EXT4_FILE_FILE_OPS,
            InodePrivate::Opaque(Arc::into_raw(ext_inode) as usize),
        );
        *inode.sb.lock() = Some(sb);
        let dentry = d_alloc("file");
        dentry.instantiate(inode);
        let file = File::new(dentry, 0, 0, &EXT4_FILE_FILE_OPS);

        let mut pos = 7;
        assert_eq!(ext4_write(&file, b"lupos", &mut pos), Ok(5));
        assert_eq!(pos, 12);

        let block = read_sectors(&bdev, 4, 2).unwrap();
        assert_eq!(&block[0..7], b"AAAAAAA");
        assert_eq!(&block[7..12], b"lupos");
        assert_eq!(&block[12..16], b"AAAA");

        let mut read_back = [0u8; 5];
        let mut read_pos = 7;
        assert_eq!(ext4_read(&file, &mut read_back, &mut read_pos), Ok(5));
        assert_eq!(&read_back, b"lupos");
    }

    #[test]
    fn ext4_write_appends_into_preallocated_extent_block() {
        let (_bdev, file, inode) = extent_file("ext4-append0", 2, 2, 1024);

        let mut pos = 1024;
        assert_eq!(ext4_write(&file, b"second", &mut pos), Ok(6));
        assert_eq!(pos, 1030);
        assert_eq!(inode.size.load(Ordering::Acquire), 1030);

        let mut read_back = [0u8; 6];
        let mut read_pos = 1024;
        assert_eq!(ext4_read(&file, &mut read_back, &mut read_pos), Ok(6));
        assert_eq!(&read_back, b"second");
        assert_eq!(read_pos, 1030);
    }

    #[test]
    fn ext4_write_unmapped_append_block_fails_without_growing_size() {
        let (_bdev, file, inode) = extent_file("ext4-append-hole0", 2, 1, 1024);

        let mut pos = 1024;
        assert_eq!(ext4_write(&file, b"hole", &mut pos), Err(EINVAL));
        assert_eq!(pos, 1024);
        assert_eq!(inode.size.load(Ordering::Acquire), 1024);
    }

    #[test]
    fn ext4_write_allocates_contiguous_extent_block_and_commits_metadata() {
        let mem = MemBlockDevice::new("ext4-alloc0", 128 * 1024);
        let raw = raw_inode_with_extent_len(8, 1, 1024);
        {
            let mut data = mem.data.lock();
            data[1024 + 12..1024 + 16].copy_from_slice(&10u32.to_le_bytes());
            data[2048 + 12..2048 + 14].copy_from_slice(&10u16.to_le_bytes());
            write_raw_inode_to_image(&mut data, 4 * 1024 + 11 * 256, &raw);
        }
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let sb = SuperBlock::alloc("ext4", EXT4_SUPER_MAGIC as u64, &EXT4_SUPER_OPS);
        let sbi = Arc::new(Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 64,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 256,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 64,
            group_desc_size: 32,
            group_descs: alloc::vec![super::super::balloc::Ext4GroupDesc {
                bg_block_bitmap: 3,
                bg_inode_bitmap: 0,
                bg_inode_table: 4,
                bg_free_blocks_count: 10,
                bg_free_inodes_count: 0,
                bg_used_dirs_count: 0,
            }],
        });
        stash_sbi(&sb, sbi).unwrap();

        let ext_inode = Arc::new(Ext4Inode {
            ino: 12,
            i_mode: 0o100644,
            i_size: AtomicU64::new(1024),
            i_blocks: AtomicU64::new(2),
            raw: Mutex::new(raw),
            dir_cache: Mutex::new(None),
            append_reservation: Mutex::new(None),
        });
        let inode = Inode::new(
            12,
            InodeKind::Regular,
            0o644,
            &EXT4_FILE_INODE_OPS,
            &EXT4_FILE_FILE_OPS,
            InodePrivate::Opaque(Arc::into_raw(ext_inode) as usize),
        );
        inode.size.store(1024, Ordering::Release);
        *inode.sb.lock() = Some(sb);
        let dentry = d_alloc("file");
        dentry.instantiate(inode.clone());
        let file = File::new(dentry, 0, 0, &EXT4_FILE_FILE_OPS);

        let mut pos = 1024;
        assert_eq!(ext4_write(&file, b"second", &mut pos), Ok(6));
        assert_eq!(pos, 1030);
        assert_eq!(inode.size.load(Ordering::Acquire), 1030);

        let mut read_back = [0u8; 6];
        let mut read_pos = 1024;
        assert_eq!(ext4_read(&file, &mut read_back, &mut read_pos), Ok(6));
        assert_eq!(&read_back, b"second");

        let bitmap = read_sectors(&bdev, 6, 2).unwrap();
        assert!(metadata::bitmap_test(&bitmap, 9).unwrap());
        let gdt = read_sectors(&bdev, 4, 2).unwrap();
        assert_eq!(u16::from_le_bytes([gdt[12], gdt[13]]), 9);
        let sb_bytes = read_sectors(&bdev, 2, 2).unwrap();
        assert_eq!(
            u32::from_le_bytes([sb_bytes[12], sb_bytes[13], sb_bytes[14], sb_bytes[15]]),
            9
        );

        let inode_lba = (4 * 1024 + 11 * 256) as u64 / 512;
        let inode_bytes = read_sectors(&bdev, inode_lba, 1).unwrap();
        let within = (11 * 256) % 512;
        assert_eq!(
            u32::from_le_bytes([
                inode_bytes[within + 4],
                inode_bytes[within + 5],
                inode_bytes[within + 6],
                inode_bytes[within + 7],
            ]),
            1030
        );
        assert_eq!(
            u32::from_le_bytes([
                inode_bytes[within + 28],
                inode_bytes[within + 29],
                inode_bytes[within + 30],
                inode_bytes[within + 31],
            ]),
            4
        );
        let i_block = &inode_bytes[within + 40..within + 100];
        assert_eq!(u16::from_le_bytes([i_block[16], i_block[17]]), 2);
        assert!(
            crate::fs::jbd2::transaction::committed_metadata_count_for_block_device(&bdev) >= 4
        );
    }

    #[test]
    fn ext4_allocator_claims_first_clear_bit_from_loaded_group_bitmap() {
        let mem = MemBlockDevice::new("ext4-alloc-bitmap-scan", 128 * 1024);
        {
            let mut data = mem.data.lock();
            data[1024 + 12..1024 + 16].copy_from_slice(&90u32.to_le_bytes());
            data[2048 + 12..2048 + 14].copy_from_slice(&90u16.to_le_bytes());
            for bit in 0..=40 {
                metadata::bitmap_set(&mut data[3 * 1024..4 * 1024], bit).unwrap();
            }
        }
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let sbi = Arc::new(Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 128,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 256,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 128,
            group_desc_size: 32,
            group_descs: alloc::vec![super::super::balloc::Ext4GroupDesc {
                bg_block_bitmap: 3,
                bg_inode_bitmap: 0,
                bg_inode_table: 4,
                bg_free_blocks_count: 90,
                bg_free_inodes_count: 0,
                bg_used_dirs_count: 0,
            }],
        });

        let claimed = ext4_claim_free_data_block_from(&sbi, 0)
            .expect("free block")
            .expect("claim block");
        assert_eq!(claimed, 41);

        let bitmap = read_sectors(&bdev, 6, 2).unwrap();
        assert!(metadata::bitmap_test(&bitmap, 41).unwrap());
        let gdt = read_sectors(&bdev, 4, 2).unwrap();
        assert_eq!(u16::from_le_bytes([gdt[12], gdt[13]]), 89);
    }

    #[test]
    fn ext4_write_allocates_noncontiguous_leaf_extent_when_next_block_is_busy() {
        let mem = MemBlockDevice::new("ext4-alloc-noncontig0", 128 * 1024);
        let raw = raw_inode_with_extent_len(8, 1, 1024);
        {
            let mut data = mem.data.lock();
            data[1024 + 12..1024 + 16].copy_from_slice(&10u32.to_le_bytes());
            data[2048 + 12..2048 + 14].copy_from_slice(&10u16.to_le_bytes());
            for bit in 0..=9 {
                metadata::bitmap_set(&mut data[3 * 1024..4 * 1024], bit).unwrap();
            }
            write_raw_inode_to_image(&mut data, 4 * 1024 + 11 * 256, &raw);
        }
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let sb = SuperBlock::alloc("ext4", EXT4_SUPER_MAGIC as u64, &EXT4_SUPER_OPS);
        let sbi = Arc::new(Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 64,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 256,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 64,
            group_desc_size: 32,
            group_descs: alloc::vec![super::super::balloc::Ext4GroupDesc {
                bg_block_bitmap: 3,
                bg_inode_bitmap: 0,
                bg_inode_table: 4,
                bg_free_blocks_count: 10,
                bg_free_inodes_count: 0,
                bg_used_dirs_count: 0,
            }],
        });
        stash_sbi(&sb, sbi).unwrap();

        let ext_inode = Arc::new(Ext4Inode {
            ino: 12,
            i_mode: 0o100644,
            i_size: AtomicU64::new(1024),
            i_blocks: AtomicU64::new(2),
            raw: Mutex::new(raw),
            dir_cache: Mutex::new(None),
            append_reservation: Mutex::new(None),
        });
        let inode = Inode::new(
            12,
            InodeKind::Regular,
            0o644,
            &EXT4_FILE_INODE_OPS,
            &EXT4_FILE_FILE_OPS,
            InodePrivate::Opaque(Arc::into_raw(ext_inode) as usize),
        );
        inode.size.store(1024, Ordering::Release);
        *inode.sb.lock() = Some(sb);
        let dentry = d_alloc("file");
        dentry.instantiate(inode.clone());
        let file = File::new(dentry, 0, 0, &EXT4_FILE_FILE_OPS);

        let mut pos = 1024;
        assert_eq!(ext4_write(&file, b"second", &mut pos), Ok(6));
        assert_eq!(pos, 1030);

        let mut read_back = [0u8; 6];
        let mut read_pos = 1024;
        assert_eq!(ext4_read(&file, &mut read_back, &mut read_pos), Ok(6));
        assert_eq!(&read_back, b"second");

        let bitmap = read_sectors(&bdev, 6, 2).unwrap();
        assert!(metadata::bitmap_test(&bitmap, 9).unwrap());
        assert!(metadata::bitmap_test(&bitmap, 10).unwrap());

        let inode_lba = (4 * 1024 + 11 * 256) as u64 / 512;
        let inode_bytes = read_sectors(&bdev, inode_lba, 1).unwrap();
        let within = (11 * 256) % 512;
        let i_block = &inode_bytes[within + 40..within + 100];
        assert_eq!(u16::from_le_bytes([i_block[2], i_block[3]]), 2);
        assert_eq!(
            u32::from_le_bytes([i_block[24], i_block[25], i_block[26], i_block[27]]),
            1
        );
        assert_eq!(u16::from_le_bytes([i_block[28], i_block[29]]), 1);
        assert_eq!(
            u32::from_le_bytes([i_block[32], i_block[33], i_block[34], i_block[35]]),
            10
        );
    }

    #[test]
    fn ext4_write_extends_indexed_extent_leaf_and_commits_metadata() {
        let mem = MemBlockDevice::new("ext4-alloc-indexed0", 128 * 1024);
        let raw = raw_inode_with_indexed_extent(12, 1024);
        {
            let mut data = mem.data.lock();
            data[1024 + 12..1024 + 16].copy_from_slice(&10u32.to_le_bytes());
            data[2048 + 12..2048 + 14].copy_from_slice(&10u16.to_le_bytes());
            for bit in 0..=8 {
                metadata::bitmap_set(&mut data[3 * 1024..4 * 1024], bit).unwrap();
            }
            metadata::bitmap_set(&mut data[3 * 1024..4 * 1024], 12).unwrap();
            write_indexed_leaf_block(&mut data, 12 * 1024, 8, 1);
            write_raw_inode_to_image(&mut data, 4 * 1024 + 11 * 256, &raw);
        }
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let sb = SuperBlock::alloc("ext4", EXT4_SUPER_MAGIC as u64, &EXT4_SUPER_OPS);
        let sbi = Arc::new(Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 64,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 256,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 64,
            group_desc_size: 32,
            group_descs: alloc::vec![super::super::balloc::Ext4GroupDesc {
                bg_block_bitmap: 3,
                bg_inode_bitmap: 0,
                bg_inode_table: 4,
                bg_free_blocks_count: 10,
                bg_free_inodes_count: 0,
                bg_used_dirs_count: 0,
            }],
        });
        stash_sbi(&sb, sbi).unwrap();

        let ext_inode = Arc::new(Ext4Inode {
            ino: 12,
            i_mode: 0o100644,
            i_size: AtomicU64::new(1024),
            i_blocks: AtomicU64::new(4),
            raw: Mutex::new(raw),
            dir_cache: Mutex::new(None),
            append_reservation: Mutex::new(None),
        });
        let inode = Inode::new(
            12,
            InodeKind::Regular,
            0o644,
            &EXT4_FILE_INODE_OPS,
            &EXT4_FILE_FILE_OPS,
            InodePrivate::Opaque(Arc::into_raw(ext_inode) as usize),
        );
        inode.size.store(1024, Ordering::Release);
        *inode.sb.lock() = Some(sb);
        let dentry = d_alloc("file");
        dentry.instantiate(inode.clone());
        let file = File::new(dentry, 0, 0, &EXT4_FILE_FILE_OPS);

        let mut pos = 1024;
        assert_eq!(ext4_write(&file, b"indexed", &mut pos), Ok(7));
        assert_eq!(pos, 1031);
        assert_eq!(inode.size.load(Ordering::Acquire), 1031);

        let mut read_back = [0u8; 7];
        let mut read_pos = 1024;
        assert_eq!(ext4_read(&file, &mut read_back, &mut read_pos), Ok(7));
        assert_eq!(&read_back, b"indexed");

        let leaf = read_sectors(&bdev, 24, 2).unwrap();
        assert_eq!(u16::from_le_bytes([leaf[16], leaf[17]]), 2);
        let bitmap = read_sectors(&bdev, 6, 2).unwrap();
        assert!(metadata::bitmap_test(&bitmap, 9).unwrap());
        assert!(metadata::bitmap_test(&bitmap, 12).unwrap());

        let inode_lba = (4 * 1024 + 11 * 256) as u64 / 512;
        let inode_bytes = read_sectors(&bdev, inode_lba, 1).unwrap();
        let within = (11 * 256) % 512;
        assert_eq!(
            u32::from_le_bytes([
                inode_bytes[within + 4],
                inode_bytes[within + 5],
                inode_bytes[within + 6],
                inode_bytes[within + 7],
            ]),
            1031
        );
    }

    #[test]
    fn ext4_allocator_splits_full_root_leaf_and_grows_depth() {
        let raw = raw_inode_with_full_leaf_extents(&[(0, 8), (1, 10), (2, 12), (3, 14)], 4096);
        let (_mem, bdev, sbi) = allocator_sbi("ext4-split-root-leaf", 15);
        let mut raw_copy = raw;
        let mut i_block = raw.i_block;

        let allocation =
            ext4_alloc_extent_append_block(&sbi, &mut raw_copy, &mut i_block, 4).unwrap();

        assert_eq!(
            allocation,
            Ext4AppendAllocation {
                data_block: 17,
                data_blocks: 1,
                allocated_blocks: 3,
            }
        );
        let root = i_block_as_bytes_mut(&mut i_block);
        assert_eq!(u16::from_le_bytes([root[2], root[3]]), 2);
        assert_eq!(u16::from_le_bytes([root[6], root[7]]), 1);
        assert_eq!(
            u32::from_le_bytes([root[12], root[13], root[14], root[15]]),
            0
        );
        assert_eq!(
            u32::from_le_bytes([root[16], root[17], root[18], root[19]]),
            16
        );
        assert_eq!(
            u32::from_le_bytes([root[24], root[25], root[26], root[27]]),
            4
        );
        assert_eq!(
            u32::from_le_bytes([root[28], root[29], root[30], root[31]]),
            18
        );

        let old_leaf = read_sectors(&bdev, 32, 2).unwrap();
        assert_eq!(u16::from_le_bytes([old_leaf[2], old_leaf[3]]), 4);
        assert_eq!(u16::from_le_bytes([old_leaf[4], old_leaf[5]]), 84);
        let new_leaf = read_sectors(&bdev, 36, 2).unwrap();
        assert_eq!(
            u32::from_le_bytes([new_leaf[12], new_leaf[13], new_leaf[14], new_leaf[15]]),
            4
        );
        assert_eq!(
            u32::from_le_bytes([new_leaf[20], new_leaf[21], new_leaf[22], new_leaf[23]]),
            17
        );
        let bitmap = read_sectors(&bdev, 6, 2).unwrap();
        assert!(metadata::bitmap_test(&bitmap, 16).unwrap());
        assert!(metadata::bitmap_test(&bitmap, 17).unwrap());
        assert!(metadata::bitmap_test(&bitmap, 18).unwrap());
    }

    #[test]
    fn ext4_allocator_creates_new_leaf_for_full_indexed_leaf() {
        let raw = raw_inode_with_index_entries(&[(0, 12)], 4096);
        let (mem, bdev, sbi) = allocator_sbi("ext4-new-leaf", 24);
        {
            let mut image = mem.data.lock();
            write_leaf_block(
                &mut image,
                12 * 1024,
                &[(0, 20), (1, 21), (2, 22), (3, 23)],
                4,
            );
        }
        let mut raw_copy = raw;
        let mut i_block = raw.i_block;

        let allocation =
            ext4_alloc_extent_append_block(&sbi, &mut raw_copy, &mut i_block, 4).unwrap();

        assert_eq!(
            allocation,
            Ext4AppendAllocation {
                data_block: 25,
                data_blocks: 1,
                allocated_blocks: 2,
            }
        );
        let root = i_block_as_bytes_mut(&mut i_block);
        assert_eq!(u16::from_le_bytes([root[2], root[3]]), 2);
        assert_eq!(
            u32::from_le_bytes([root[24], root[25], root[26], root[27]]),
            4
        );
        assert_eq!(
            u32::from_le_bytes([root[28], root[29], root[30], root[31]]),
            26
        );
        let new_leaf = read_sectors(&bdev, 52, 2).unwrap();
        assert_eq!(u16::from_le_bytes([new_leaf[2], new_leaf[3]]), 1);
        assert_eq!(
            u32::from_le_bytes([new_leaf[12], new_leaf[13], new_leaf[14], new_leaf[15]]),
            4
        );
        assert_eq!(
            u32::from_le_bytes([new_leaf[20], new_leaf[21], new_leaf[22], new_leaf[23]]),
            25
        );
    }

    #[test]
    fn ext4_allocator_grows_full_index_root_to_depth_two() {
        let raw = raw_inode_with_index_entries(&[(0, 12), (10, 13), (20, 14), (30, 15)], 34 * 1024);
        let (mem, bdev, sbi) = allocator_sbi("ext4-grow-depth2", 54);
        {
            let mut image = mem.data.lock();
            write_leaf_block(&mut image, 12 * 1024, &[(0, 20)], 4);
            write_leaf_block(&mut image, 13 * 1024, &[(10, 30)], 4);
            write_leaf_block(&mut image, 14 * 1024, &[(20, 40)], 4);
            write_leaf_block(
                &mut image,
                15 * 1024,
                &[(30, 50), (31, 51), (32, 52), (33, 53)],
                4,
            );
        }
        let mut raw_copy = raw;
        let mut i_block = raw.i_block;

        let allocation =
            ext4_alloc_extent_append_block(&sbi, &mut raw_copy, &mut i_block, 34).unwrap();

        assert_eq!(
            allocation,
            Ext4AppendAllocation {
                data_block: 56,
                data_blocks: 1,
                allocated_blocks: 4,
            }
        );
        let root = i_block_as_bytes_mut(&mut i_block);
        assert_eq!(u16::from_le_bytes([root[2], root[3]]), 2);
        assert_eq!(u16::from_le_bytes([root[6], root[7]]), 2);
        assert_eq!(
            u32::from_le_bytes([root[16], root[17], root[18], root[19]]),
            55
        );
        assert_eq!(
            u32::from_le_bytes([root[24], root[25], root[26], root[27]]),
            34
        );
        assert_eq!(
            u32::from_le_bytes([root[28], root[29], root[30], root[31]]),
            58
        );

        let old_index = read_sectors(&bdev, 110, 2).unwrap();
        assert_eq!(u16::from_le_bytes([old_index[2], old_index[3]]), 4);
        assert_eq!(u16::from_le_bytes([old_index[4], old_index[5]]), 84);
        assert_eq!(u16::from_le_bytes([old_index[6], old_index[7]]), 1);
        let new_index = read_sectors(&bdev, 116, 2).unwrap();
        assert_eq!(u16::from_le_bytes([new_index[2], new_index[3]]), 1);
        assert_eq!(
            u32::from_le_bytes([new_index[12], new_index[13], new_index[14], new_index[15]]),
            34
        );
        assert_eq!(
            u32::from_le_bytes([new_index[16], new_index[17], new_index[18], new_index[19]]),
            57
        );
        let new_leaf = read_sectors(&bdev, 114, 2).unwrap();
        assert_eq!(
            u32::from_le_bytes([new_leaf[12], new_leaf[13], new_leaf[14], new_leaf[15]]),
            34
        );
        assert_eq!(
            u32::from_le_bytes([new_leaf[20], new_leaf[21], new_leaf[22], new_leaf[23]]),
            56
        );
    }

    #[test]
    fn ext4_mkdir_and_create_follow_linux_namei_linear_dirent_path() {
        let mem = MemBlockDevice::new("ext4-namei-create", 256 * 1024);
        {
            let mut data = mem.data.lock();
            data[1024 + 12..1024 + 16].copy_from_slice(&90u32.to_le_bytes());
            data[1024 + 16..1024 + 20].copy_from_slice(&4u32.to_le_bytes());
            data[2048..2052].copy_from_slice(&3u32.to_le_bytes());
            data[2052..2056].copy_from_slice(&4u32.to_le_bytes());
            data[2056..2060].copy_from_slice(&5u32.to_le_bytes());
            data[2060..2062].copy_from_slice(&90u16.to_le_bytes());
            data[2062..2064].copy_from_slice(&4u16.to_le_bytes());
            data[2064..2066].copy_from_slice(&1u16.to_le_bytes());
            for bit in 0..=9 {
                metadata::bitmap_set(&mut data[3 * 1024..4 * 1024], bit).unwrap();
            }
            for bit in 0..=10 {
                metadata::bitmap_set(&mut data[4 * 1024..5 * 1024], bit).unwrap();
            }
            write_dot_dir_block(&mut data[9 * 1024..10 * 1024], 2, 2);
            let raw_parent = raw_dir_inode_with_extent(9, 1024, 2);
            write_raw_inode_to_image(&mut data, 5 * 1024 + 256, &raw_parent);
        }
        let bdev = BlockDevice::wrap(mem.clone(), mem_block_device_ops());
        let sb = SuperBlock::alloc("ext4", EXT4_SUPER_MAGIC as u64, &EXT4_SUPER_OPS);
        let sbi = Arc::new(Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 128,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 256,
            want_extra_isize: 32,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 128,
            group_desc_size: 32,
            group_descs: alloc::vec![super::super::balloc::Ext4GroupDesc {
                bg_block_bitmap: 3,
                bg_inode_bitmap: 4,
                bg_inode_table: 5,
                bg_free_blocks_count: 90,
                bg_free_inodes_count: 4,
                bg_used_dirs_count: 1,
            }],
        });
        stash_sbi(&sb, sbi.clone()).unwrap();

        let raw_parent = raw_dir_inode_with_extent(9, 1024, 2);
        let parent_ext = Arc::new(Ext4Inode {
            ino: 2,
            i_mode: 0o40755,
            i_size: AtomicU64::new(1024),
            i_blocks: AtomicU64::new(2),
            raw: Mutex::new(raw_parent),
            dir_cache: Mutex::new(None),
            append_reservation: Mutex::new(None),
        });
        let parent = Inode::new(
            2,
            InodeKind::Directory,
            0o755,
            &EXT4_DIR_INODE_OPS,
            &EXT4_DIR_FILE_OPS,
            InodePrivate::Opaque(Arc::into_raw(parent_ext) as usize),
        );
        parent.size.store(1024, Ordering::Release);
        parent.nlink.store(2, Ordering::Release);
        *parent.sb.lock() = Some(sb);

        let parent_ext = ext4_inode_of(&parent).unwrap();
        let initial_entries = dir_read_all(&sbi, &parent_ext).unwrap();
        assert!(!initial_entries.iter().any(|entry| entry.name == "tmp"));
        assert!(!initial_entries.iter().any(|entry| entry.name == "state"));

        let tmp = ext4_mkdir(&parent, "tmp", 0o1777).expect("mkdir tmp");
        assert_eq!(tmp.kind, InodeKind::Directory);
        let state = ext4_create(&parent, "state", 0o644).expect("create regular file");
        assert_eq!(state.kind, InodeKind::Regular);

        let parent_ext = ext4_inode_of(&parent).unwrap();
        let entries = dir_read_all(&sbi, &parent_ext).unwrap();
        assert!(entries.iter().any(|entry| entry.name == "tmp"));
        assert!(entries.iter().any(|entry| entry.name == "state"));
        assert_eq!(parent.nlink.load(Ordering::Acquire), 3);

        let child_ext = ext4_inode_of(&tmp).unwrap();
        let child_entries = dir_read_all(&sbi, &child_ext).unwrap();
        assert!(child_entries.iter().any(|entry| entry.name == "."));
        assert!(child_entries.iter().any(|entry| entry.name == ".."));

        let dentry = d_alloc("state");
        dentry.instantiate(state.clone());
        let file = File::new(dentry, 0, 0, &EXT4_FILE_FILE_OPS);
        let mut pos = 0;
        assert_eq!(ext4_write(&file, b"ok", &mut pos), Ok(2));
        let mut read_back = [0u8; 2];
        let mut read_pos = 0;
        assert_eq!(ext4_read(&file, &mut read_back, &mut read_pos), Ok(2));
        assert_eq!(&read_back, b"ok");

        assert_eq!(ext4_rename(&parent, "state", &parent, "state.db"), Ok(()));
        assert!(matches!(ext4_lookup(&parent, "state"), Err(ENOENT)));
        let renamed_state = ext4_lookup(&parent, "state.db").expect("renamed state");
        assert_eq!(renamed_state.ino, state.ino);
        let dentry = d_alloc("state.db");
        dentry.instantiate(renamed_state.clone());
        let file = File::new(dentry, 0, 0, &EXT4_FILE_FILE_OPS);
        let mut read_back = [0u8; 2];
        let mut read_pos = 0;
        assert_eq!(ext4_read(&file, &mut read_back, &mut read_pos), Ok(2));
        assert_eq!(&read_back, b"ok");

        let stale = ext4_create(&parent, "core.db", 0o644).expect("create stale db");
        assert_eq!(ext4_rename(&parent, "state.db", &parent, "core.db"), Ok(()));
        assert!(matches!(ext4_lookup(&parent, "state.db"), Err(ENOENT)));
        let current = ext4_lookup(&parent, "core.db").expect("replacement target");
        assert_eq!(current.ino, state.ino);
        let stale_raw = ialloc::read_raw_inode(&sbi, stale.ino as u32).unwrap();
        assert_eq!(le_u16(&stale_raw, 0).unwrap(), 0);

        assert_eq!(ext4_unlink(&parent, "core.db"), Ok(()));
        let entries_after_unlink = dir_read_all(&sbi, &parent_ext).unwrap();
        assert!(
            !entries_after_unlink
                .iter()
                .any(|entry| entry.name == "core.db" || entry.name == "state.db")
        );
        assert!(matches!(ext4_lookup(&parent, "core.db"), Err(ENOENT)));
        let sb_ref = parent.sb.lock().clone().unwrap();
        let removed_state = read_inode(&sbi, state.ino as u32, &sb_ref).unwrap();
        assert_eq!(removed_state.nlink.load(Ordering::Acquire), 0);
        let removed_state_raw = ialloc::read_raw_inode(&sbi, state.ino as u32).unwrap();
        assert_eq!(le_u16(&removed_state_raw, 0).unwrap(), 0);
        assert_eq!(le_u32(&removed_state_raw, 20).unwrap(), 0);

        assert_eq!(ext4_rmdir(&parent, "tmp"), Ok(()));
        let entries_after_rmdir = dir_read_all(&sbi, &parent_ext).unwrap();
        assert!(!entries_after_rmdir.iter().any(|entry| entry.name == "tmp"));
        assert!(matches!(ext4_lookup(&parent, "tmp"), Err(ENOENT)));
        assert_eq!(parent.nlink.load(Ordering::Acquire), 2);

        let inode_bitmap = read_sectors(&bdev, 8, 2).unwrap();
        assert!(!metadata::bitmap_test(&inode_bitmap, 11).unwrap());
        assert!(!metadata::bitmap_test(&inode_bitmap, 12).unwrap());
        assert!(!metadata::bitmap_test(&inode_bitmap, 13).unwrap());
        let block_bitmap = read_sectors(&bdev, 6, 2).unwrap();
        assert!(!metadata::bitmap_test(&block_bitmap, 10).unwrap());
        assert!(!metadata::bitmap_test(&block_bitmap, 11).unwrap());
        let gdt = read_sectors(&bdev, 4, 2).unwrap();
        assert_eq!(u16::from_le_bytes([gdt[12], gdt[13]]), 90);
        assert_eq!(u16::from_le_bytes([gdt[14], gdt[15]]), 4);
        assert_eq!(u16::from_le_bytes([gdt[16], gdt[17]]), 1);
        let super_bytes = read_sectors(&bdev, 2, 2).unwrap();
        assert_eq!(
            u32::from_le_bytes([
                super_bytes[12],
                super_bytes[13],
                super_bytes[14],
                super_bytes[15]
            ]),
            90
        );
        assert_eq!(
            u32::from_le_bytes([
                super_bytes[16],
                super_bytes[17],
                super_bytes[18],
                super_bytes[19]
            ]),
            2
        );
    }

    fn extent_file(
        name: &str,
        phys_block: u64,
        extent_len: u16,
        size: u64,
    ) -> (BlockDeviceRef, FileRef, InodeRef) {
        let mem = MemBlockDevice::new(name, 8192);
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let sb = SuperBlock::alloc("ext4", EXT4_SUPER_MAGIC as u64, &EXT4_SUPER_OPS);
        let sbi = Arc::new(Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 64,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 128,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 64,
            group_desc_size: 32,
            group_descs: Vec::new(),
        });
        stash_sbi(&sb, sbi).unwrap();

        let raw = raw_inode_with_extent_len(phys_block, extent_len, size);
        let ext_inode = Arc::new(Ext4Inode {
            ino: 12,
            i_mode: 0o100644,
            i_size: AtomicU64::new(size),
            i_blocks: AtomicU64::new(extent_len as u64 * 2),
            raw: Mutex::new(raw),
            dir_cache: Mutex::new(None),
            append_reservation: Mutex::new(None),
        });
        let inode = Inode::new(
            12,
            InodeKind::Regular,
            0o644,
            &EXT4_FILE_INODE_OPS,
            &EXT4_FILE_FILE_OPS,
            InodePrivate::Opaque(Arc::into_raw(ext_inode) as usize),
        );
        inode.size.store(size, Ordering::Release);
        *inode.sb.lock() = Some(sb);
        let dentry = d_alloc("file");
        dentry.instantiate(inode.clone());
        let file = File::new(dentry, 0, 0, &EXT4_FILE_FILE_OPS);
        (bdev, file, inode)
    }

    fn raw_inode_with_extent(phys_block: u64, size: u64) -> OnDiskInode {
        raw_inode_with_extent_len(phys_block, 1, size)
    }

    fn raw_fast_symlink_inode(target: &[u8]) -> OnDiskInode {
        let mut raw = raw_inode_with_extent_len(0, 0, target.len() as u64);
        let mut i_block = [0; 15];
        i_block_as_bytes_mut(&mut i_block)[..target.len()].copy_from_slice(target);
        raw.i_mode = 0o120777u16.to_le();
        raw.i_blocks_lo = 0;
        raw.i_flags = 0;
        raw.i_block = i_block;
        raw
    }

    fn raw_inode_with_extent_len(phys_block: u64, len: u16, size: u64) -> OnDiskInode {
        OnDiskInode {
            i_mode: 0o100644u16.to_le(),
            i_uid: 0,
            i_size_lo: (size as u32).to_le(),
            i_atime: 0,
            i_ctime: 0,
            i_mtime: 0,
            i_dtime: 0,
            i_gid: 0,
            i_links_count: 1u16.to_le(),
            i_blocks_lo: (len as u32 * 2).to_le(),
            i_flags: 0x80000u32.to_le(),
            _osd1: 0,
            i_block: extent_i_block(phys_block, len),
            i_generation: 0,
            i_file_acl_lo: 0,
            i_size_hi: ((size >> 32) as u32).to_le(),
            i_obso_faddr: 0,
            _osd2: [0; 12],
            i_extra_isize: 0,
            i_checksum_hi: 0,
            i_ctime_extra: 0,
            i_mtime_extra: 0,
            i_atime_extra: 0,
            i_crtime: 0,
            i_crtime_extra: 0,
            i_version_hi: 0,
            i_projid: 0,
        }
    }

    fn raw_inode_with_indexed_extent(leaf_block: u64, size: u64) -> OnDiskInode {
        OnDiskInode {
            i_mode: 0o100644u16.to_le(),
            i_uid: 0,
            i_size_lo: (size as u32).to_le(),
            i_atime: 0,
            i_ctime: 0,
            i_mtime: 0,
            i_dtime: 0,
            i_gid: 0,
            i_links_count: 1u16.to_le(),
            i_blocks_lo: 4u32.to_le(),
            i_flags: 0x80000u32.to_le(),
            _osd1: 0,
            i_block: indexed_i_block(leaf_block),
            i_generation: 0,
            i_file_acl_lo: 0,
            i_size_hi: ((size >> 32) as u32).to_le(),
            i_obso_faddr: 0,
            _osd2: [0; 12],
            i_extra_isize: 0,
            i_checksum_hi: 0,
            i_ctime_extra: 0,
            i_mtime_extra: 0,
            i_atime_extra: 0,
            i_crtime: 0,
            i_crtime_extra: 0,
            i_version_hi: 0,
            i_projid: 0,
        }
    }

    fn allocator_sbi(
        name: &str,
        used_bits_through: usize,
    ) -> (Arc<MemBlockDevice>, BlockDeviceRef, super::super::Ext4Sbi) {
        let mem = MemBlockDevice::new(name, 256 * 1024);
        {
            let mut data = mem.data.lock();
            data[1024 + 12..1024 + 16].copy_from_slice(&90u32.to_le_bytes());
            data[2048 + 12..2048 + 14].copy_from_slice(&90u16.to_le_bytes());
            for bit in 0..=used_bits_through {
                metadata::bitmap_set(&mut data[3 * 1024..4 * 1024], bit).unwrap();
            }
        }
        let bdev = BlockDevice::wrap(mem.clone(), mem_block_device_ops());
        let sbi = super::super::Ext4Sbi {
            bdev: bdev.clone(),
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 128,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size: 256,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 128,
            group_desc_size: 32,
            group_descs: alloc::vec![super::super::balloc::Ext4GroupDesc {
                bg_block_bitmap: 3,
                bg_inode_bitmap: 0,
                bg_inode_table: 4,
                bg_free_blocks_count: 90,
                bg_free_inodes_count: 0,
                bg_used_dirs_count: 0,
            }],
        };
        (mem, bdev, sbi)
    }

    fn raw_inode_with_full_leaf_extents(extents: &[(u32, u64)], size: u64) -> OnDiskInode {
        let mut raw = raw_inode_with_extent_len(extents[0].1, 1, size);
        raw.i_block = leaf_i_block_with_extents(extents, 4);
        raw.i_blocks_lo = (extents.len() as u32 * 2).to_le();
        raw
    }

    fn raw_inode_with_index_entries(entries: &[(u32, u64)], size: u64) -> OnDiskInode {
        let mut raw = raw_inode_with_indexed_extent(entries[0].1, size);
        raw.i_block = index_i_block_with_entries(entries);
        raw.i_blocks_lo = (entries.len() as u32 * 2).to_le();
        raw
    }

    fn raw_dir_inode_with_extent(phys_block: u64, size: u64, links: u16) -> OnDiskInode {
        let mut raw = raw_inode_with_extent_len(phys_block, 1, size);
        raw.i_mode = 0o40755u16.to_le();
        raw.i_links_count = links.to_le();
        raw
    }

    fn write_dot_dir_block(block: &mut [u8], self_ino: u32, parent_ino: u32) {
        let block_len = block.len();
        block.fill(0);
        block[0..4].copy_from_slice(&self_ino.to_le_bytes());
        block[4..6].copy_from_slice(&12u16.to_le_bytes());
        block[6] = 1;
        block[7] = EXT4_FT_DIR;
        block[8] = b'.';
        block[12..16].copy_from_slice(&parent_ino.to_le_bytes());
        block[16..18].copy_from_slice(&((block_len - 12) as u16).to_le_bytes());
        block[18] = 2;
        block[19] = EXT4_FT_DIR;
        block[20] = b'.';
        block[21] = b'.';
    }

    fn extent_i_block(phys_block: u64, len: u16) -> [u32; 15] {
        let mut bytes = [0u8; 60];
        bytes[0..2].copy_from_slice(&EXT4_EXT_MAGIC.to_le_bytes());
        bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&4u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&0u16.to_le_bytes());
        bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
        bytes[16..18].copy_from_slice(&len.to_le_bytes());
        bytes[18..20].copy_from_slice(&((phys_block >> 32) as u16).to_le_bytes());
        bytes[20..24].copy_from_slice(&(phys_block as u32).to_le_bytes());

        let mut out = [0u32; 15];
        for (slot, chunk) in out.iter_mut().zip(bytes.chunks_exact(4)) {
            *slot = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        out
    }

    fn leaf_i_block_with_extents(extents: &[(u32, u64)], max_entries: u16) -> [u32; 15] {
        let mut bytes = [0u8; 60];
        write_extent_header(&mut bytes, extents.len() as u16, max_entries, 0);
        for (idx, (lblock, phys)) in extents.iter().copied().enumerate() {
            write_extent_entry_bytes(&mut bytes, idx, lblock, 1, phys);
        }
        bytes_to_i_block(&bytes)
    }

    fn indexed_i_block(leaf_block: u64) -> [u32; 15] {
        let mut bytes = [0u8; 60];
        bytes[0..2].copy_from_slice(&EXT4_EXT_MAGIC.to_le_bytes());
        bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&4u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&1u16.to_le_bytes());
        bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&(leaf_block as u32).to_le_bytes());
        bytes[20..22].copy_from_slice(&((leaf_block >> 32) as u16).to_le_bytes());

        let mut out = [0u32; 15];
        for (idx, chunk) in bytes.chunks_exact(4).enumerate() {
            out[idx] = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        out
    }

    fn index_i_block_with_entries(entries: &[(u32, u64)]) -> [u32; 15] {
        let mut bytes = [0u8; 60];
        write_extent_header(&mut bytes, entries.len() as u16, 4, 1);
        for (idx, (lblock, leaf_block)) in entries.iter().copied().enumerate() {
            write_idx_entry_bytes(&mut bytes, idx, lblock, leaf_block);
        }
        bytes_to_i_block(&bytes)
    }

    fn write_indexed_leaf_block(image: &mut [u8], off: usize, phys_block: u64, len: u16) {
        let leaf = &mut image[off..off + 1024];
        leaf[0..2].copy_from_slice(&EXT4_EXT_MAGIC.to_le_bytes());
        leaf[2..4].copy_from_slice(&1u16.to_le_bytes());
        leaf[4..6].copy_from_slice(&4u16.to_le_bytes());
        leaf[6..8].copy_from_slice(&0u16.to_le_bytes());
        leaf[12..16].copy_from_slice(&0u32.to_le_bytes());
        leaf[16..18].copy_from_slice(&len.to_le_bytes());
        leaf[18..20].copy_from_slice(&((phys_block >> 32) as u16).to_le_bytes());
        leaf[20..24].copy_from_slice(&(phys_block as u32).to_le_bytes());
    }

    fn write_leaf_block(image: &mut [u8], off: usize, extents: &[(u32, u64)], max_entries: u16) {
        let leaf = &mut image[off..off + 1024];
        leaf.fill(0);
        write_extent_header(leaf, extents.len() as u16, max_entries, 0);
        for (idx, (lblock, phys)) in extents.iter().copied().enumerate() {
            write_extent_entry_bytes(leaf, idx, lblock, 1, phys);
        }
    }

    fn write_extent_header(bytes: &mut [u8], entries: u16, max_entries: u16, depth: u16) {
        bytes[0..2].copy_from_slice(&EXT4_EXT_MAGIC.to_le_bytes());
        bytes[2..4].copy_from_slice(&entries.to_le_bytes());
        bytes[4..6].copy_from_slice(&max_entries.to_le_bytes());
        bytes[6..8].copy_from_slice(&depth.to_le_bytes());
    }

    fn write_extent_entry_bytes(bytes: &mut [u8], idx: usize, lblock: u32, len: u16, phys: u64) {
        let off = 12 + idx * 12;
        bytes[off..off + 4].copy_from_slice(&lblock.to_le_bytes());
        bytes[off + 4..off + 6].copy_from_slice(&len.to_le_bytes());
        bytes[off + 6..off + 8].copy_from_slice(&((phys >> 32) as u16).to_le_bytes());
        bytes[off + 8..off + 12].copy_from_slice(&(phys as u32).to_le_bytes());
    }

    fn write_idx_entry_bytes(bytes: &mut [u8], idx: usize, lblock: u32, leaf_block: u64) {
        let off = 12 + idx * 12;
        bytes[off..off + 4].copy_from_slice(&lblock.to_le_bytes());
        bytes[off + 4..off + 8].copy_from_slice(&(leaf_block as u32).to_le_bytes());
        bytes[off + 8..off + 10].copy_from_slice(&((leaf_block >> 32) as u16).to_le_bytes());
    }

    fn bytes_to_i_block(bytes: &[u8; 60]) -> [u32; 15] {
        let mut out = [0u32; 15];
        for (idx, chunk) in bytes.chunks_exact(4).enumerate() {
            out[idx] = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        out
    }

    fn write_raw_inode_to_image(image: &mut [u8], off: usize, raw: &OnDiskInode) {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (raw as *const OnDiskInode).cast::<u8>(),
                core::mem::size_of::<OnDiskInode>(),
            )
        };
        image[off..off + bytes.len()].copy_from_slice(bytes);
    }
}
