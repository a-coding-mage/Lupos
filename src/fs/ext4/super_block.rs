//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4
//! test-origin: linux:vendor/linux/fs/ext4
//! ext4 superblock parsing.
//!
//! Mirrors `vendor/linux/fs/ext4/ext4.h::struct ext4_super_block`.  The
//! superblock lives at byte offset 1024 from the partition start.

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::block::block_device::BlockDeviceRef;
use crate::block::partitions::read_sectors;
use crate::fs::types::SuperBlockRef;
use crate::include::uapi::errno::EINVAL;

use super::balloc::{EXT4_MIN_DESC_SIZE, EXT4_MIN_DESC_SIZE_64BIT, Ext4GroupDesc};
use super::{EXT4_SUPER_MAGIC, Ext4Sbi};

const SB_OFF_BYTES: u64 = 1024;
const EXT4_MAX_DESC_SIZE: u32 = 1024;
const EXT4_FEATURE_INCOMPAT_64BIT: u32 = 0x0080;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ext4SuperIdentity {
    pub fs_uuid: [u8; 16],
    pub label: Option<String>,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct OnDiskSuperBlock {
    pub s_inodes_count: u32,
    pub s_blocks_count_lo: u32,
    pub s_r_blocks_count_lo: u32,
    pub s_free_blocks_count_lo: u32,
    pub s_free_inodes_count: u32,
    pub s_first_data_block: u32,
    pub s_log_block_size: u32,
    pub s_log_cluster_size: u32,
    pub s_blocks_per_group: u32,
    pub s_clusters_per_group: u32,
    pub s_inodes_per_group: u32,
    pub s_mtime: u32,
    pub s_wtime: u32,
    pub s_mnt_count: u16,
    pub s_max_mnt_count: u16,
    pub s_magic: u16,
    pub s_state: u16,
    pub s_errors: u16,
    pub s_minor_rev_level: u16,
    pub s_lastcheck: u32,
    pub s_checkinterval: u32,
    pub s_creator_os: u32,
    pub s_rev_level: u32,
    pub s_def_resuid: u16,
    pub s_def_resgid: u16,
    // Dynamic-rev fields.
    pub s_first_ino: u32,
    pub s_inode_size: u16,
    pub s_block_group_nr: u16,
    pub s_feature_compat: u32,
    pub s_feature_incompat: u32,
    pub s_feature_ro_compat: u32,
    pub s_uuid: [u8; 16],
    pub s_volume_name: [u8; 16],
    pub s_last_mounted: [u8; 64],
    pub s_algorithm_usage_bitmap: u32,
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_reserved_gdt_blocks: u16,
    pub s_journal_uuid: [u8; 16],
    pub s_journal_inum: u32,
    pub s_journal_dev: u32,
    pub s_last_orphan: u32,
    pub s_hash_seed: [u32; 4],
    pub s_def_hash_version: u8,
    pub s_jnl_backup_type: u8,
    pub s_desc_size: u16,
    pub s_default_mount_opts: u32,
    pub s_first_meta_bg: u32,
    pub s_mkfs_time: u32,
    pub s_jnl_blocks: [u32; 17],
    pub s_blocks_count_hi: u32,
    pub s_r_blocks_count_hi: u32,
    pub s_free_blocks_count_hi: u32,
    pub s_min_extra_isize: u16,
    pub s_want_extra_isize: u16,
    pub s_flags: u32,
    pub s_raid_stride: u16,
    pub s_mmp_update_interval: u16,
    pub s_mmp_block: u64,
    pub s_raid_stripe_width: u32,
    pub s_log_groups_per_flex: u8,
    pub s_checksum_type: u8,
    pub s_encryption_level: u8,
    pub s_reserved_pad: u8,
    pub s_kbytes_written: u64,
    // …trailing fields elided; we don't need them for the read path.
}

fn read_on_disk_super(bdev: &BlockDeviceRef) -> Result<OnDiskSuperBlock, i32> {
    // Read 4 KiB starting at sector 0 — gets us the boot block + first-block superblock.
    let bytes = read_sectors(bdev, 0, 8)?;
    if bytes.len() < (SB_OFF_BYTES as usize) + core::mem::size_of::<OnDiskSuperBlock>() {
        return Err(EINVAL);
    }
    let sb_ptr = unsafe { bytes.as_ptr().add(SB_OFF_BYTES as usize) as *const OnDiskSuperBlock };
    let sb = unsafe { core::ptr::read_unaligned(sb_ptr) };
    if u16::from_le(sb.s_magic) != EXT4_SUPER_MAGIC {
        return Err(EINVAL);
    }
    Ok(sb)
}

fn ext4_label_from_volume_name(volume_name: &[u8; 16]) -> Option<String> {
    let end = volume_name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(volume_name.len());
    if end == 0 {
        return None;
    }
    core::str::from_utf8(&volume_name[..end])
        .ok()
        .map(String::from)
}

pub fn read_super_identity(bdev: &BlockDeviceRef) -> Result<Ext4SuperIdentity, i32> {
    let sb = read_on_disk_super(bdev)?;
    let fs_uuid = sb.s_uuid;
    let volume_name = sb.s_volume_name;
    Ok(Ext4SuperIdentity {
        fs_uuid,
        label: ext4_label_from_volume_name(&volume_name),
    })
}

pub fn read_super(bdev: &BlockDeviceRef) -> Result<Ext4Sbi, i32> {
    let sb = read_on_disk_super(bdev)?;

    let log_block_size = u32::from_le(sb.s_log_block_size);
    let block_size = 1024u32 << log_block_size;
    let blocks_per_group = u32::from_le(sb.s_blocks_per_group);
    let inodes_per_group = u32::from_le(sb.s_inodes_per_group);
    let first_ino = u32::from_le(sb.s_first_ino).max(11);
    let inode_size = u16::from_le(sb.s_inode_size).max(128) as u32;
    let want_extra_isize = u16::from_le(sb.s_want_extra_isize);

    let inodes_count = u32::from_le(sb.s_inodes_count) as u64;
    let blocks_count_lo = u32::from_le(sb.s_blocks_count_lo) as u64;
    let blocks_count_hi = u32::from_le(sb.s_blocks_count_hi) as u64;
    let blocks_count = (blocks_count_hi << 32) | blocks_count_lo;

    let feature_compat = u32::from_le(sb.s_feature_compat);
    let feature_incompat = u32::from_le(sb.s_feature_incompat);
    let feature_ro_compat = u32::from_le(sb.s_feature_ro_compat);
    let raw_group_desc_size = u16::from_le(sb.s_desc_size) as u32;
    let has_64bit = feature_incompat & EXT4_FEATURE_INCOMPAT_64BIT != 0;
    let group_desc_size = if has_64bit {
        if raw_group_desc_size < EXT4_MIN_DESC_SIZE_64BIT
            || raw_group_desc_size > EXT4_MAX_DESC_SIZE
            || !raw_group_desc_size.is_power_of_two()
        {
            return Err(EINVAL);
        }
        raw_group_desc_size
    } else {
        EXT4_MIN_DESC_SIZE
    };

    // Compute number of block groups.
    let nr_groups = blocks_count.div_ceil(blocks_per_group as u64);

    // Read the block-group descriptor table.  On the standard layout it
    // lives in the block immediately following the superblock (block 1 for
    // 4 KiB block size) — when block_size == 1024 it's block 2.
    let gdt_start_block = if block_size == 1024 { 2 } else { 1 };
    let gdt_bytes = (nr_groups as usize) * (group_desc_size as usize);
    let gdt_sectors = gdt_bytes.div_ceil(512) as u64;
    let gdt_lba = (gdt_start_block as u64) * (block_size as u64) / 512;
    let gdt_buf = read_sectors(bdev, gdt_lba, gdt_sectors)?;

    let mut gds: Vec<Ext4GroupDesc> = Vec::with_capacity(nr_groups as usize);
    for i in 0..(nr_groups as usize) {
        let off = i * (group_desc_size as usize);
        let end = off + group_desc_size as usize;
        if end > gdt_buf.len() {
            break;
        }
        gds.push(Ext4GroupDesc::parse(&gdt_buf[off..end], group_desc_size));
    }

    Ok(Ext4Sbi {
        bdev: bdev.clone(),
        fs_uuid: sb.s_uuid,
        block_size,
        blocks_per_group,
        inodes_per_group,
        first_ino,
        inode_size,
        want_extra_isize,
        feature_compat,
        feature_incompat,
        feature_ro_compat,
        inodes_count,
        blocks_count,
        group_desc_size,
        group_descs: gds,
    })
}

pub fn stash_sbi(sb: &SuperBlockRef, sbi: Arc<Ext4Sbi>) -> Result<(), i32> {
    sb.set_fs_private(sbi);
    Ok(())
}

pub fn get_sbi(sb: &SuperBlockRef) -> Option<Arc<Ext4Sbi>> {
    sb.fs_private::<Ext4Sbi>()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::block::block_device::BlockDevice;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};

    fn put_u16(image: &mut [u8], offset: usize, value: u16) {
        image[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(image: &mut [u8], offset: usize, value: u32) {
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn ext4_read_super_preserves_linux_superblock_uuid() {
        let mut image = alloc::vec![0u8; 8192];
        let base = SB_OFF_BYTES as usize;
        let fsuuid = [
            0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ];
        put_u32(&mut image, base, 16);
        put_u32(&mut image, base + 4, 64);
        put_u32(&mut image, base + 24, 0);
        put_u32(&mut image, base + 32, 64);
        put_u32(&mut image, base + 40, 16);
        put_u16(&mut image, base + 56, EXT4_SUPER_MAGIC);
        put_u16(&mut image, base + 88, 128);
        image[base + 104..base + 120].copy_from_slice(&fsuuid);
        image[base + 120..base + 130].copy_from_slice(b"lupos-root");

        let mem = MemBlockDevice::new("ext4-uuid", image.len());
        mem.data.lock().copy_from_slice(&image);
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());

        let sbi = read_super(&bdev).expect("parse ext4 superblock");
        assert_eq!(sbi.fs_uuid, fsuuid);
        assert_eq!(
            read_super_identity(&bdev)
                .expect("parse ext4 identity")
                .label
                .as_deref(),
            Some("lupos-root")
        );
    }

    #[test]
    fn ext4_read_super_forces_legacy_desc_size_without_64bit_feature() {
        let mut image = alloc::vec![0u8; 8192];
        let base = SB_OFF_BYTES as usize;
        let gdt = 4096;

        put_u32(&mut image, base, 32768);
        put_u32(&mut image, base + 4, 65536);
        put_u32(&mut image, base + 24, 2);
        put_u32(&mut image, base + 32, 32768);
        put_u32(&mut image, base + 40, 32768);
        put_u16(&mut image, base + 56, EXT4_SUPER_MAGIC);
        put_u16(&mut image, base + 88, 256);
        put_u32(&mut image, base + 92, 0);
        put_u32(&mut image, base + 96, 0x40);
        put_u16(&mut image, base + 254, 64);

        put_u32(&mut image, gdt, 17);
        put_u32(&mut image, gdt + 4, 19);
        put_u32(&mut image, gdt + 8, 21);
        put_u32(&mut image, gdt + 32, 0x812);
        put_u32(&mut image, gdt + 36, 0x814);
        put_u32(&mut image, gdt + 40, 0x815);

        let mem = MemBlockDevice::new("ext4-legacy-gdt", image.len());
        mem.data.lock().copy_from_slice(&image);
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());

        let sbi = read_super(&bdev).expect("parse ext4 superblock");

        assert_eq!(sbi.group_desc_size, EXT4_MIN_DESC_SIZE);
        assert_eq!(sbi.group_descs[0].bg_inode_table, 21);
        assert_eq!(sbi.group_descs[1].bg_inode_table, 0x815);
    }
}
