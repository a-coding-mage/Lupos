//! linux-parity: partial
//! linux-source: vendor/linux/fs/fat
//! FAT32 / vfat — read+write (M46).
//!
//! Mirrors `vendor/linux/fs/fat/`.  Lupos M46 ships the FAT32 path only
//! (FAT12 / FAT16 are easy to add later); short and long filenames are
//! both honoured at lookup time, but the mkfs / write path generates only
//! short 8.3 names.  No xattrs, no nfs export, no Unicode normalization.

pub mod boot_sector;
pub mod dir;
pub mod fatent;
pub mod inode;
pub mod name_cache;
pub mod ops;

extern crate alloc;

use alloc::sync::Arc;

use crate::block::{block_device::BlockDeviceRef, lookup_block_device};
use crate::fs::dcache::d_alloc;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};
use crate::include::uapi::errno::ENODEV;

pub const FAT_SUPER_MAGIC: u64 = 0x4d44; // "MD" — Linux uses MSDOS_SUPER_MAGIC

pub struct FatSbi {
    pub bdev: BlockDeviceRef,
    pub bytes_per_sector: u32,
    pub sectors_per_cluster: u32,
    pub reserved_sectors: u32,
    pub num_fats: u32,
    pub fat_size_sectors: u32,
    pub root_cluster: u32,
    pub data_start_sector: u32,
    pub total_sectors: u32,
}

pub fn mount(source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let bdev = lookup_block_device(source).ok_or(ENODEV)?;
    let bpb = boot_sector::read(&bdev)?;
    let sbi = Arc::new(boot_sector::sbi_from_bpb(bdev.clone(), &bpb));
    let sb = SuperBlock::alloc("vfat", FAT_SUPER_MAGIC, &ops::FAT_SUPER_OPS);
    boot_sector::stash_sbi(&sb, sbi.clone())?;
    let root_inode = inode::root_inode(&sbi, &sb);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "vfat",
        mount,
        fs_flags: 0,
    });
}
