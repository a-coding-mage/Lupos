//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4/dir.c
//! test-origin: linux:vendor/linux/fs/ext4/dir.c
//! ext4 directory entry parser.
//!
//! Mirrors `vendor/linux/fs/ext4/dir.c` and `ext4.h::struct ext4_dir_entry_2`.
//! Each entry: `inode (4) | rec_len (2) | name_len (1) | file_type (1) | name`.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use super::Ext4Inode;
use super::Ext4Sbi;
use super::extents;
use super::indirect;
use super::inode as ext4_inode_mod;
use crate::block::partitions::read_sectors;
use crate::fs::types::InodeKind;
use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Debug)]
pub struct DirEntry {
    pub inode: u32,
    pub name: String,
    pub file_type: u8,
}

const FT_REG_FILE: u8 = 1;
const FT_DIR: u8 = 2;
const FT_CHRDEV: u8 = 3;
const FT_BLKDEV: u8 = 4;
const FT_FIFO: u8 = 5;
const FT_SOCK: u8 = 6;
const FT_SYMLINK: u8 = 7;

/// Read all directory entries from a directory inode (linear scan; htree
/// roots are still valid linear directories so this works for both layouts).
pub fn read_all(sbi: &Ext4Sbi, dir_inode: &Ext4Inode) -> Result<Vec<DirEntry>, i32> {
    if let Some(cached) = dir_inode.dir_cache.lock().clone() {
        return Ok(cached);
    }

    let dir_size = dir_inode.i_size.load(Ordering::Acquire) as usize;
    let block_size = sbi.block_size as usize;
    let mut out = Vec::new();
    let mut consumed = 0usize;
    let mut lblock: u64 = 0;
    let i_block_copy = { dir_inode.raw.lock().i_block };
    while consumed < dir_size {
        let phys = if ext4_inode_mod::uses_extents(dir_inode) {
            extents::map_block(sbi, i_block_copy, lblock)?
        } else {
            indirect::map_block(sbi, i_block_copy, lblock)?
        };
        let phys = phys.ok_or(EINVAL)?;
        let lba = phys * (block_size as u64) / 512;
        let nr = (block_size / 512) as u64;
        let buf = read_sectors(&sbi.bdev, lba, nr)?;
        parse_block(&buf, &mut out)?;
        consumed += block_size;
        lblock += 1;
    }
    *dir_inode.dir_cache.lock() = Some(out.clone());
    Ok(out)
}

pub fn invalidate_cache(dir_inode: &Ext4Inode) {
    *dir_inode.dir_cache.lock() = None;
}

fn parse_block(buf: &[u8], out: &mut Vec<DirEntry>) -> Result<(), i32> {
    let mut off = 0;
    while off + 8 <= buf.len() {
        let ino = u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        let rec_len = u16::from_le_bytes([buf[off + 4], buf[off + 5]]) as usize;
        let name_len = buf[off + 6] as usize;
        let file_type = buf[off + 7];
        if rec_len < 8 || off + rec_len > buf.len() {
            return Err(EINVAL);
        }
        if ino != 0 && name_len > 0 && off + 8 + name_len <= buf.len() {
            let name_bytes = &buf[off + 8..off + 8 + name_len];
            if let Ok(s) = core::str::from_utf8(name_bytes) {
                out.push(DirEntry {
                    inode: ino,
                    name: alloc::string::String::from(s),
                    file_type,
                });
            }
        }
        off += rec_len;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn is_dir_type(t: u8) -> bool {
    t == FT_DIR
}
#[allow(dead_code)]
pub fn is_reg_type(t: u8) -> bool {
    t == FT_REG_FILE
}

pub fn kind_for_type(t: u8) -> InodeKind {
    match t {
        FT_DIR => InodeKind::Directory,
        FT_SYMLINK => InodeKind::Symlink,
        FT_CHRDEV => InodeKind::Chardev,
        FT_BLKDEV => InodeKind::Blockdev,
        FT_FIFO => InodeKind::Fifo,
        FT_SOCK => InodeKind::Socket,
        _ => InodeKind::Regular,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext4_dirent_file_types_map_to_vfs_kinds() {
        assert_eq!(kind_for_type(FT_REG_FILE), InodeKind::Regular);
        assert_eq!(kind_for_type(FT_DIR), InodeKind::Directory);
        assert_eq!(kind_for_type(FT_CHRDEV), InodeKind::Chardev);
        assert_eq!(kind_for_type(FT_BLKDEV), InodeKind::Blockdev);
        assert_eq!(kind_for_type(FT_FIFO), InodeKind::Fifo);
        assert_eq!(kind_for_type(FT_SOCK), InodeKind::Socket);
        assert_eq!(kind_for_type(FT_SYMLINK), InodeKind::Symlink);
    }
}
