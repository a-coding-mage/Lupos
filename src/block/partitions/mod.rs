//! linux-parity: partial
//! linux-source: vendor/linux/block/partitions
//! test-origin: linux:vendor/linux/block/partitions
//! Partition-table parsing — entry point.
//!
//! Mirrors `vendor/linux/block/partitions/core.c::check_partition`.  Reads
//! sector 0 from the disk; dispatches to MBR or GPT depending on the
//! `boot_signature` and the protective-MBR pattern.

extern crate alloc;

use alloc::format;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;

use crate::include::uapi::errno::{EBUSY, EINVAL, EIO};

use super::bio::{BIO_OP_READ, BIO_OP_WRITE, BioOp, BioRef, BioVec, bio_alloc, submit_bio};
use super::block_device::{
    BlockDevice, BlockDeviceOps, BlockDeviceRef, lookup_block_device, register_block_device,
};
use super::gendisk::register_gendisk;

pub mod gpt;
pub mod karma;
pub mod legacy;
pub mod mac;
pub mod mbr;
pub mod of;
pub mod osf;
pub mod sgi;
pub mod sun;
pub mod sysv68;
pub mod ultrix;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Partition {
    pub number: u32,
    pub start_sector: u64,
    pub nr_sectors: u64,
    pub type_guid: Option<[u8; 16]>, // GPT only
    pub type_byte: Option<u8>,       // MBR only
}

pub struct PartitionBlockDevice {
    pub parent: BlockDeviceRef,
    pub partition: Partition,
}

#[derive(Clone)]
pub struct RegisteredPartition {
    pub name: alloc::string::String,
    pub partition: Partition,
    pub bdev: BlockDeviceRef,
}

/// Read `nr_sectors` from `bdev` starting at `lba` into a Vec<u8>.
pub fn read_sectors(bdev: &BlockDeviceRef, lba: u64, nr_sectors: u64) -> Result<Vec<u8>, i32> {
    let bytes = (nr_sectors as usize) * 512;
    let buf = alloc::vec![0u8; bytes];
    let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_READ), lba);
    bio.add_vec(BioVec::new(buf));
    submit_bio(bio.clone())?;
    let v = bio.vecs.lock();
    let g = v[0].data.lock();
    Ok(g.clone())
}

/// Parse partitions on `bdev`.  Tries GPT first, falls back to MBR.
pub fn parse_partitions(bdev: &BlockDeviceRef) -> Result<Vec<Partition>, i32> {
    let s0 = read_sectors(bdev, 0, 1)?;
    if s0.len() < 512 {
        return Err(EINVAL);
    }
    if !mbr::has_valid_signature(&s0) {
        return Ok(Vec::new());
    }
    // Protective-MBR check for GPT.
    if mbr::is_protective_for_gpt(&s0) {
        return gpt::parse(bdev);
    }
    Ok(mbr::parse(&s0))
}

/// Convenience — wrap `Partition` in an Arc for sharing.
pub fn arc_partition(p: Partition) -> Arc<Partition> {
    Arc::new(p)
}

pub fn partition_device_name(disk_name: &str, number: u32) -> alloc::string::String {
    let base = disk_name.trim_start_matches("/dev/");
    if base.as_bytes().last().is_some_and(|b| b.is_ascii_digit()) {
        format!("{base}p{number}")
    } else {
        format!("{base}{number}")
    }
}

pub fn partition_block_device(parent: BlockDeviceRef, partition: Partition) -> BlockDeviceRef {
    let backing = Arc::new(PartitionBlockDevice { parent, partition });
    BlockDevice::wrap(backing, &PARTITION_OPS)
}

/// Linux `add_partition()` analogue for Lupos' flat block-device registry.
/// Parses `disk`, creates offset block devices, and registers `vda1`-style
/// names.  `/dev/vda1` works through `lookup_block_device()` normalization.
pub fn register_partition_devices(
    disk_name: &str,
    disk: &BlockDeviceRef,
) -> Result<Vec<RegisteredPartition>, i32> {
    let partitions = parse_partitions(disk)?;
    let mut out = Vec::new();
    for partition in partitions {
        let name = partition_device_name(disk_name, partition.number);
        let bdev = partition_block_device(disk.clone(), partition.clone());
        let registered = match register_block_device(&name, bdev.clone()) {
            Ok(()) => {
                register_gendisk(&name, bdev.clone());
                bdev
            }
            Err(EBUSY) => lookup_block_device(&name).unwrap_or(bdev),
            Err(e) => return Err(e),
        };
        out.push(RegisteredPartition {
            name,
            partition,
            bdev: registered,
        });
    }
    Ok(out)
}

fn partition_submit_bio(bdev: &BlockDeviceRef, bio: &BioRef) -> Result<(), i32> {
    let backing = partition_backing(bdev)?;
    let total_size = bio.total_size();
    let sectors = (total_size as u64).div_ceil(512);
    let end = bio.sector.checked_add(sectors).ok_or(EIO)?;
    if end > backing.partition.nr_sectors {
        return Err(EIO);
    }
    let translated_sector = backing
        .partition
        .start_sector
        .checked_add(bio.sector)
        .ok_or(EIO)?;

    match bio.op.0 {
        BIO_OP_READ => partition_read(&backing.parent, bio, translated_sector),
        BIO_OP_WRITE => partition_write(&backing.parent, bio, translated_sector),
        _ => {
            let parent_bio = bio_alloc(backing.parent.clone(), bio.op, translated_sector);
            submit_bio(parent_bio)
        }
    }
}

fn partition_read(parent: &BlockDeviceRef, bio: &BioRef, sector: u64) -> Result<(), i32> {
    let parent_bio = bio_alloc(parent.clone(), bio.op, sector);
    {
        let vecs = bio.vecs.lock();
        for vec in vecs.iter() {
            parent_bio.add_vec(BioVec::new(alloc::vec![0u8; vec.len]));
        }
    }
    submit_bio(parent_bio.clone())?;

    let child_vecs = bio.vecs.lock();
    let parent_vecs = parent_bio.vecs.lock();
    for (child, parent) in child_vecs.iter().zip(parent_vecs.iter()) {
        let mut child_data = child.data.lock();
        let parent_data = parent.data.lock();
        child_data[child.off..child.off + child.len]
            .copy_from_slice(&parent_data[parent.off..parent.off + parent.len]);
    }
    Ok(())
}

fn partition_write(parent: &BlockDeviceRef, bio: &BioRef, sector: u64) -> Result<(), i32> {
    let parent_bio = bio_alloc(parent.clone(), bio.op, sector);
    {
        let vecs = bio.vecs.lock();
        for vec in vecs.iter() {
            let data = vec.data.lock();
            parent_bio.add_vec(BioVec::new(data[vec.off..vec.off + vec.len].to_vec()));
        }
    }
    submit_bio(parent_bio)
}

fn partition_get_capacity(bdev: &BlockDeviceRef) -> u64 {
    partition_backing(bdev)
        .map(|backing| backing.partition.nr_sectors)
        .unwrap_or(0)
}

fn partition_block_size(bdev: &BlockDeviceRef) -> u32 {
    partition_backing(bdev)
        .map(|backing| (backing.parent.ops.block_size)(&backing.parent))
        .unwrap_or(512)
}

fn partition_backing(bdev: &BlockDeviceRef) -> Result<Arc<PartitionBlockDevice>, i32> {
    let backing = bdev.backing.lock().clone().ok_or(EIO)?;
    backing
        .downcast::<PartitionBlockDevice>()
        .map_err(|_: Arc<dyn Any + Send + Sync>| EIO)
}

static PARTITION_OPS: BlockDeviceOps = BlockDeviceOps {
    name: "partition",
    submit_bio: partition_submit_bio,
    get_capacity: partition_get_capacity,
    block_size: partition_block_size,
    ioctl: None,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::bio::{BIO_OP_READ, BIO_OP_WRITE};
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};

    #[test]
    fn partition_device_names_match_linux_suffix_rules() {
        assert_eq!(partition_device_name("vda", 1), "vda1");
        assert_eq!(partition_device_name("/dev/vda", 2), "vda2");
        assert_eq!(partition_device_name("loop0", 1), "loop0p1");
        assert_eq!(partition_device_name("nvme0n1", 3), "nvme0n1p3");
    }

    #[test]
    fn partition_block_device_translates_reads_and_writes() {
        let mem = MemBlockDevice::new("partxlate", 16 * 512);
        {
            let mut data = mem.data.lock();
            data[4 * 512..5 * 512].fill(0x5a);
        }
        let parent = BlockDevice::wrap(mem, mem_block_device_ops());
        let part = Partition {
            number: 1,
            start_sector: 4,
            nr_sectors: 4,
            type_guid: None,
            type_byte: Some(0x83),
        };
        let child = partition_block_device(parent.clone(), part);

        let read = bio_alloc(child.clone(), BioOp(BIO_OP_READ), 0);
        read.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(read.clone()).expect("partition read");
        assert!(read.vecs.lock()[0].data.lock().iter().all(|b| *b == 0x5a));

        let write = bio_alloc(child.clone(), BioOp(BIO_OP_WRITE), 1);
        write.add_vec(BioVec::new(alloc::vec![0xa5; 512]));
        submit_bio(write).expect("partition write");
        let reread = bio_alloc(parent.clone(), BioOp(BIO_OP_READ), 5);
        reread.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(reread.clone()).expect("parent read");
        assert!(reread.vecs.lock()[0].data.lock().iter().all(|b| *b == 0xa5));

        let out_of_range = bio_alloc(child, BioOp(BIO_OP_READ), 4);
        out_of_range.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        assert_eq!(submit_bio(out_of_range), Err(EIO));
    }

    #[test]
    fn register_partition_devices_exposes_dev_path_lookup() {
        struct RegistryCleanup(alloc::string::String);
        impl Drop for RegistryCleanup {
            fn drop(&mut self) {
                let _ = crate::block::block_device::unregister_block_device(&self.0);
                let _ = crate::block::gendisk::unregister_gendisk(&self.0);
            }
        }

        let mem = MemBlockDevice::new("partscan", 4096 * 512);
        {
            let mut data = mem.data.lock();
            mbr::build_mbr_with_one_partition(&mut data[..512], 0x0c, 8, 32);
            data[8 * 512..9 * 512].fill(0x7b);
        }
        let parent = BlockDevice::wrap(mem, mem_block_device_ops());
        let disk_name = format!("partscan{}", parent.id);
        let partition_name = partition_device_name(&disk_name, 1);
        let _cleanup = RegistryCleanup(partition_name.clone());
        let registered = register_partition_devices(&disk_name, &parent).expect("scan");

        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].name, partition_name);
        assert!(lookup_block_device(&partition_name).is_some());
        assert!(lookup_block_device(&format!("/dev/{partition_name}")).is_some());
        let read = bio_alloc(registered[0].bdev.clone(), BioOp(BIO_OP_READ), 0);
        read.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(read.clone()).expect("read registered partition");
        assert!(read.vecs.lock()[0].data.lock().iter().all(|b| *b == 0x7b));
    }

    #[test]
    fn malformed_gpt_entry_size_is_rejected_without_panic() {
        let nr_entries: u32 = 32;
        let entry_size: u32 = 16;
        let entries_bytes = (nr_entries as usize) * (entry_size as usize);
        let mut entries = alloc::vec![0u8; entries_bytes];
        entries[entries_bytes - 16..entries_bytes].copy_from_slice(&[0xa5; 16]);
        let entries_crc = gpt::entries_crc(&entries);

        let mut hdr_sector = alloc::vec![0u8; 512];
        let _ = gpt::build_header(
            &mut hdr_sector[..92],
            1,
            255,
            2,
            nr_entries,
            entry_size,
            entries_crc,
        );

        let mem = MemBlockDevice::new("badgpt", 4 * 512);
        {
            let mut data = mem.data.lock();
            mbr::build_mbr_with_one_partition(&mut data[..512], 0xee, 1, 255);
            data[512..1024].copy_from_slice(&hdr_sector);
            data[1024..1024 + entries_bytes].copy_from_slice(&entries);
        }
        let parent = BlockDevice::wrap(mem, mem_block_device_ops());

        assert_eq!(parse_partitions(&parent), Err(EINVAL));
    }
}
