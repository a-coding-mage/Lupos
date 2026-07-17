//! linux-parity: partial
//! linux-source: vendor/linux/fs/fat
//! FAT32 BPB parsing.
//!
//! Mirrors `vendor/linux/fs/fat/inode.c::fat_read_bpb`.

extern crate alloc;

use alloc::sync::Arc;

use crate::block::block_device::BlockDeviceRef;
use crate::block::partitions::read_sectors;
use crate::fs::types::SuperBlockRef;
use crate::include::uapi::errno::EINVAL;

use super::FatSbi;

#[derive(Clone, Debug)]
pub struct Bpb {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entries: u16,
    pub total_sectors_16: u16,
    pub fat_size_16: u16,
    pub total_sectors_32: u32,
    // FAT32 specific
    pub fat_size_32: u32,
    pub root_cluster: u32,
    pub fs_info: u16,
}

pub fn read(bdev: &BlockDeviceRef) -> Result<Bpb, i32> {
    let s0 = read_sectors(bdev, 0, 1)?;
    if s0.len() < 512 {
        return Err(EINVAL);
    }
    let bytes_per_sector = u16::from_le_bytes([s0[11], s0[12]]);
    if bytes_per_sector != 512 {
        return Err(EINVAL);
    } // FAT32 standard
    let sectors_per_cluster = s0[13];
    let reserved_sectors = u16::from_le_bytes([s0[14], s0[15]]);
    let num_fats = s0[16];
    let root_entries = u16::from_le_bytes([s0[17], s0[18]]);
    let total_sectors_16 = u16::from_le_bytes([s0[19], s0[20]]);
    let fat_size_16 = u16::from_le_bytes([s0[22], s0[23]]);
    let total_sectors_32 = u32::from_le_bytes([s0[32], s0[33], s0[34], s0[35]]);
    let fat_size_32 = u32::from_le_bytes([s0[36], s0[37], s0[38], s0[39]]);
    let root_cluster = u32::from_le_bytes([s0[44], s0[45], s0[46], s0[47]]);
    let fs_info = u16::from_le_bytes([s0[48], s0[49]]);

    Ok(Bpb {
        bytes_per_sector,
        sectors_per_cluster,
        reserved_sectors,
        num_fats,
        root_entries,
        total_sectors_16,
        fat_size_16,
        total_sectors_32,
        fat_size_32,
        root_cluster,
        fs_info,
    })
}

pub fn sbi_from_bpb(bdev: BlockDeviceRef, bpb: &Bpb) -> FatSbi {
    let fat_size = if bpb.fat_size_16 != 0 {
        bpb.fat_size_16 as u32
    } else {
        bpb.fat_size_32
    };
    let total = if bpb.total_sectors_16 != 0 {
        bpb.total_sectors_16 as u32
    } else {
        bpb.total_sectors_32
    };
    let data_start = (bpb.reserved_sectors as u32) + (bpb.num_fats as u32) * fat_size;
    FatSbi {
        bdev,
        bytes_per_sector: bpb.bytes_per_sector as u32,
        sectors_per_cluster: bpb.sectors_per_cluster as u32,
        reserved_sectors: bpb.reserved_sectors as u32,
        num_fats: bpb.num_fats as u32,
        fat_size_sectors: fat_size,
        root_cluster: bpb.root_cluster,
        data_start_sector: data_start,
        total_sectors: total,
    }
}

pub fn stash_sbi(sb: &SuperBlockRef, sbi: Arc<FatSbi>) -> Result<(), i32> {
    sb.set_fs_private(sbi);
    Ok(())
}

pub fn get_sbi(sb: &SuperBlockRef) -> Option<Arc<FatSbi>> {
    sb.fs_private::<FatSbi>()
}
