//! linux-parity: partial
//! linux-source: vendor/linux/fs/fat/fatent.c
//! FAT32 cluster-chain walking.
//!
//! Mirrors `vendor/linux/fs/fat/fatent.c::fat_ent_read`.

extern crate alloc;

use alloc::vec::Vec;

use crate::block::partitions::read_sectors;
use crate::include::uapi::errno::EINVAL;

use super::FatSbi;

pub const FAT32_EOC: u32 = 0x0FFFFFF8;
pub const FAT32_BAD: u32 = 0x0FFFFFF7;
pub const FAT32_MASK: u32 = 0x0FFFFFFF;

/// Read a single FAT32 entry for cluster `n`.
pub fn fat_get_next(sbi: &FatSbi, n: u32) -> Result<u32, i32> {
    let entry_byte = (n as u64) * 4;
    let sector = (sbi.reserved_sectors as u64) + entry_byte / 512;
    let off_in_sector = (entry_byte % 512) as usize;
    let buf = read_sectors(&sbi.bdev, sector, 1)?;
    if buf.len() < off_in_sector + 4 {
        return Err(EINVAL);
    }
    let v = u32::from_le_bytes([
        buf[off_in_sector],
        buf[off_in_sector + 1],
        buf[off_in_sector + 2],
        buf[off_in_sector + 3],
    ]);
    Ok(v & FAT32_MASK)
}

/// Walk a cluster chain starting at `start`.  Returns all clusters until EOC.
pub fn cluster_chain(sbi: &FatSbi, start: u32) -> Result<Vec<u32>, i32> {
    let mut out = Vec::new();
    let mut cur = start;
    let mut steps = 0u32;
    while cur >= 2
        && cur < FAT32_BAD
        && steps < (sbi.total_sectors / sbi.sectors_per_cluster.max(1)) + 16
    {
        out.push(cur);
        let n = fat_get_next(sbi, cur)?;
        if n >= FAT32_EOC || n < 2 {
            break;
        }
        cur = n;
        steps += 1;
    }
    Ok(out)
}

/// Convert a cluster number to its absolute (LBA) sector start.
pub fn cluster_to_sector(sbi: &FatSbi, cluster: u32) -> u64 {
    sbi.data_start_sector as u64 + ((cluster - 2) as u64) * sbi.sectors_per_cluster as u64
}

/// Read all bytes that make up a cluster.
pub fn read_cluster(sbi: &FatSbi, cluster: u32) -> Result<Vec<u8>, i32> {
    let lba = cluster_to_sector(sbi, cluster);
    read_sectors(&sbi.bdev, lba, sbi.sectors_per_cluster as u64)
}
