//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4/ialloc.c
//! ext4 inode-table read helpers.

extern crate alloc;

use alloc::vec::Vec;

use crate::block::partitions::read_sectors;
use crate::include::uapi::errno::EINVAL;

use super::Ext4Sbi;

/// Compute the byte offset of inode `ino` (1-based) on disk.
pub fn inode_disk_offset(sbi: &Ext4Sbi, ino: u32) -> Result<u64, i32> {
    if ino == 0 {
        return Err(EINVAL);
    }
    let group = (ino - 1) / sbi.inodes_per_group;
    let index_in_group = (ino - 1) % sbi.inodes_per_group;
    let gd = sbi.group_descs.get(group as usize).ok_or(EINVAL)?;
    let table_block = gd.bg_inode_table;
    let off =
        table_block * (sbi.block_size as u64) + (index_in_group as u64) * (sbi.inode_size as u64);
    Ok(off)
}

/// Read raw inode bytes (sized to `s_inode_size`).
pub fn read_raw_inode(sbi: &Ext4Sbi, ino: u32) -> Result<Vec<u8>, i32> {
    let off = inode_disk_offset(sbi, ino)?;
    let lba = off / 512;
    let within = (off % 512) as usize;
    let total_bytes = within + sbi.inode_size as usize;
    let nr_sectors = total_bytes.div_ceil(512) as u64;
    let buf = read_sectors(&sbi.bdev, lba, nr_sectors)?;
    Ok(buf[within..within + sbi.inode_size as usize].to_vec())
}
